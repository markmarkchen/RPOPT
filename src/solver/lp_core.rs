//! Allocation-free core of the dense dual simplex (see `lp.rs` for the method).
//!
//! All working storage lives in a thread-local `Scratch` that is reused across calls, so after the
//! first call on a thread an LP solve performs no heap allocation unless the caller asks for the
//! full solution (`Want::Full`).
//!
//! LPs with at most 16 variables first go to the revised form of the same method (`lp_rev.rs`);
//! the tableau runs when that path cannot verify its answer, and for larger LPs.

use crate::linalg::{dot, Lu, Mat};
use std::cell::RefCell;

#[path = "lp_rev.rs"]
mod rev;

use std::sync::atomic::{AtomicBool, Ordering};

/// Benchmarking switch (`lp::bench::set_tableau_only`): skip the revised path and run the
/// tableau, the solver before `lp_rev` existed.
pub(crate) static TABLEAU_ONLY: AtomicBool = AtomicBool::new(false);

pub const PIVOT_TOL: f64 = 1e-9;

const MAX_DEGENERATE_BEFORE_BLAND: usize = 30;

/// Run of degenerate pivots after which pricing switches to Bland's rule (anti-cycling). In a
/// feasibility LP every pivot of the dual is degenerate, so a fixed 30 means that every LP that
/// needs more than 30 pivots finishes under Bland's rule, which is very slow on large LPs and
/// ran into the iteration limit (a false "infeasible") on n >= 48 test LPs. Up to n = 16 the
/// threshold is unchanged; above, Bland's rule is a last resort after 10 (n + m) such pivots.
#[inline]
pub(crate) fn bland_after(n: usize, m: usize) -> usize {
    if n <= 16 { MAX_DEGENERATE_BEFORE_BLAND } else { MAX_DEGENERATE_BEFORE_BLAND.max(10 * (n + m)) }
}
const REFACTOR_ROUNDS: usize = 4;
const DRIVE_OUT: bool = false;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Want {
    Feasible,
    Point,
    Full,
}

pub(crate) struct Core {
    pub x: Vec<f64>,
    pub dual: Vec<f64>,
}

#[derive(Clone, Copy)]
struct ColInfo {
    con: u32,
    sign: f64,
}

enum Outcome {
    Optimal,
    Unbounded(usize),
    IterLimit,
}

#[derive(Default)]
struct Scratch {
    t: Vec<f64>,
    basis: Vec<usize>,
    prow: Vec<f64>,
    cols: Vec<ColInfo>,
    sigma: Vec<f64>,
    is_eq: Vec<bool>,
    inv1: Vec<f64>,
    inv2: Vec<f64>,
    x: Vec<f64>,
    y: Vec<f64>,
    bshift: Vec<f64>,
    rev: rev::RevScratch,
}

thread_local! {
    static SCRATCH: RefCell<Scratch> = RefCell::new(Scratch::default());
}

struct Tab<'s> {
    nrows: usize,
    ncols: usize,
    width: usize,
    nstruct: usize,
    /// rows updated by a pivot: n + 2 in phase 1, n + 1 afterwards (phase-1 row dropped)
    live: usize,
    t: &'s mut Vec<f64>,
    basis: &'s mut Vec<usize>,
    prow: &'s mut Vec<f64>,
}

impl Tab<'_> {
    #[inline(always)]
    fn at(&self, r: usize, c: usize) -> f64 {
        self.t[r * self.width + c]
    }

    #[inline]
    fn pivot(&mut self, pr: usize, pc: usize) {
        #[cfg(feature = "lp_census")]
        crate::census::piv_bump();
        let w = self.width;
        {
            let row = &mut self.t[pr * w..(pr + 1) * w];
            let inv = 1.0 / row[pc];
            for v in row.iter_mut() {
                *v *= inv;
            }
            row[pc] = 1.0;
            self.prow.clear();
            self.prow.extend_from_slice(row);
        }
        let prow = &self.prow[..w];
        for r in 0..self.live {
            if r == pr {
                continue;
            }
            let row = &mut self.t[r * w..(r + 1) * w];
            let f = row[pc];
            if f == 0.0 {
                continue;
            }
            for (v, p) in row.iter_mut().zip(prow) {
                *v -= f * p;
            }
            row[pc] = 0.0;
        }
        self.basis[pr] = pc;
    }

    /// Simplex iterations on the given cost row. A reduced cost counts as negative when
    /// d_j * inv[j] < -1e-10 (inv[j] = 1 / magnitude of column j's cost); Dantzig pricing on the
    /// scaled values, Bland's rule after a run of degenerate pivots. With `block_artificials`
    /// basic artificials are kept at zero.
    /// `sc[j]` is the magnitude of column j's cost: d_j counts as negative when d_j < -1e-10 sc[j],
    /// and Dantzig pricing minimises d_j / sc[j] (compared by cross-multiplication, no division).
    fn iterate(&mut self, cost_row: usize, sc: &[f64], max_iter: usize, block_artificials: bool) -> Outcome {
        let w = self.width;
        let rhs = self.ncols;
        let ns = self.nstruct;
        let phase1 = !block_artificials;
        let mut degenerate = 0usize;
        for _ in 0..max_iter {
            // phase 1 is done once the artificials sum to zero (further pivots would be degenerate)
            if phase1 && -self.t[cost_row * w + rhs] <= 1e-13 {
                return Outcome::Optimal;
            }
            let bland = degenerate > bland_after(self.nrows, ns);
            let crow = &self.t[cost_row * w..cost_row * w + ns];
            let sc = &sc[..ns];
            let mut enter = usize::MAX;
            if bland {
                for (j, (&d, &s)) in crow.iter().zip(sc).enumerate() {
                    if d < -1e-10 * s {
                        enter = j;
                        break;
                    }
                }
            } else {
                let (mut best_d, mut best_s) = (0.0f64, 1.0f64);
                for (j, (&d, &s)) in crow.iter().zip(sc).enumerate() {
                    if d < -1e-10 * s && (enter == usize::MAX || d * best_s < best_d * s) {
                        best_d = d;
                        best_s = s;
                        enter = j;
                    }
                }
            }
            if enter == usize::MAX {
                return Outcome::Optimal;
            }
            let mut leave = usize::MAX;
            let mut best_ratio = f64::INFINITY;
            let mut best_piv = 0.0;
            for r in 0..self.nrows {
                let a = self.t[r * w + enter];
                let artificial = block_artificials && self.basis[r] >= ns;
                let (eligible, ratio) = if artificial {
                    (a.abs() > PIVOT_TOL, 0.0)
                } else if a > PIVOT_TOL {
                    (true, self.t[r * w + rhs].max(0.0) / a)
                } else {
                    (false, 0.0)
                };
                if !eligible {
                    continue;
                }
                let better = if leave == usize::MAX || ratio < best_ratio - 1e-12 {
                    true
                } else if ratio <= best_ratio + 1e-12 {
                    if bland { self.basis[r] < self.basis[leave] } else { a.abs() > best_piv }
                } else {
                    false
                };
                if better {
                    best_ratio = ratio;
                    best_piv = a.abs();
                    leave = r;
                }
            }
            if leave == usize::MAX {
                return Outcome::Unbounded(enter);
            }
            if best_ratio <= 1e-12 {
                degenerate += 1;
            } else {
                degenerate = 0;
            }
            self.pivot(leave, enter);
        }
        Outcome::IterLimit
    }
}

struct Data<'a> {
    a: &'a Mat,
    b: &'a [f64],
    c: &'a [f64],
    n: usize,
    cols: &'a [ColInfo],
    sigma: &'a [f64],
    is_eq: &'a [bool],
}

impl Data<'_> {
    #[inline]
    fn m_entry(&self, k: usize, j: usize) -> f64 {
        let ns = self.cols.len();
        if j < ns {
            let ci = self.cols[j];
            self.sigma[k] * ci.sign * self.a[(ci.con as usize, k)]
        } else if j - ns == k {
            1.0
        } else {
            0.0
        }
    }

    #[inline]
    fn cost(&self, j: usize) -> f64 {
        if j < self.cols.len() {
            let ci = self.cols[j];
            ci.sign * self.b[ci.con as usize]
        } else {
            0.0
        }
    }

    /// Rebuilds the rows and the phase-2 cost row from the basis and the original data (rare path).
    #[cold]
    fn refactor(&self, tab: &mut Tab) -> bool {
        let n = self.n;
        let w = tab.width;
        let ncols = tab.ncols;
        let mut bm = Mat::zeros(n, n);
        for r in 0..n {
            for k in 0..n {
                bm[(k, r)] = self.m_entry(k, tab.basis[r]);
            }
        }
        let Some(lu) = Lu::new(&bm) else { return false };
        let mut col = vec![0.0; n];
        for j in 0..ncols {
            for k in 0..n {
                col[k] = self.m_entry(k, j);
            }
            let x = lu.solve(&col);
            for r in 0..n {
                tab.t[r * w + j] = x[r];
            }
        }
        let rvec: Vec<f64> = (0..n).map(|k| self.sigma[k] * (-self.c[k])).collect();
        let xr = lu.solve(&rvec);
        for r in 0..n {
            tab.t[r * w + ncols] = xr[r];
        }
        for r in 0..n {
            tab.t[r * w + tab.basis[r]] = 1.0;
        }
        let cb: Vec<f64> = tab.basis.iter().map(|&j| self.cost(j)).collect();
        for j in 0..ncols {
            let mut s = self.cost(j);
            for r in 0..n {
                s -= cb[r] * tab.t[r * w + j];
            }
            tab.t[n * w + j] = s;
        }
        let mut z = 0.0;
        for r in 0..n {
            z += cb[r] * tab.t[r * w + ncols];
        }
        tab.t[n * w + ncols] = -z;
        let basis = tab.basis.clone();
        for bj in basis {
            tab.t[n * w + bj] = 0.0;
        }
        true
    }

    /// Is the ray along entering column `e` a valid Farkas certificate on the original data?
    /// The ray has at most n + 1 nonzero multipliers (entering column plus basic structurals),
    /// so the check is O(n^2) on those rows only.
    fn certificate_holds(&self, tab: &Tab, e: usize, _y: &mut Vec<f64>) -> bool {
        let n = self.n;
        let ns = self.cols.len();
        // (constraint, multiplier) pairs, merging the two columns of a split equality
        let mut idx = [0u32; 65];
        let mut val = [0.0f64; 65];
        let mut cnt = 0usize;
        let push = |con: u32, v: f64, idx: &mut [u32; 65], val: &mut [f64; 65], cnt: &mut usize| -> bool {
            for q in 0..*cnt {
                if idx[q] == con {
                    val[q] += v;
                    return true;
                }
            }
            if *cnt == 65 {
                return false;
            }
            idx[*cnt] = con;
            val[*cnt] = v;
            *cnt += 1;
            true
        };
        let ce = self.cols[e];
        if !push(ce.con, ce.sign, &mut idx, &mut val, &mut cnt) {
            return false;
        }
        for r in 0..n {
            let v = -tab.at(r, e);
            let bj = tab.basis[r];
            if bj >= ns {
                if v.abs() > 1e-9 {
                    return false;
                }
                continue;
            }
            if v == 0.0 {
                continue;
            }
            let ci = self.cols[bj];
            if !push(ci.con, ci.sign * v, &mut idx, &mut val, &mut cnt) {
                return self.certificate_dense(tab, e);
            }
        }
        let ysum: f64 = val[..cnt].iter().map(|v| v.abs()).sum::<f64>().max(1e-300);
        let mut amax: f64 = 0.0;
        for q in 0..cnt {
            let i = idx[q] as usize;
            if !self.is_eq[i] && val[q] < -1e-9 * ysum {
                return false;
            }
            amax = amax.max(self.a.row(i).iter().fold(0.0f64, |s, v| s.max(v.abs())));
        }
        let amax = amax.max(1e-300);
        let mut by = 0.0;
        for q in 0..cnt {
            by += self.b[idx[q] as usize] * val[q];
        }
        for k in 0..n {
            let mut s = 0.0;
            for q in 0..cnt {
                s += self.a[(idx[q] as usize, k)] * val[q];
            }
            if s.abs() > 1e-9 * ysum * amax {
                return false;
            }
        }
        by < -1e-12 * ysum
    }

    /// Dense fallback of `certificate_holds` (n > 64).
    #[cold]
    fn certificate_dense(&self, tab: &Tab, e: usize) -> bool {
        let n = self.n;
        let m = self.a.rows;
        let ns = self.cols.len();
        let mut y = vec![0.0; m];
        let ce = self.cols[e];
        y[ce.con as usize] += ce.sign;
        for r in 0..n {
            let v = -tab.at(r, e);
            let bj = tab.basis[r];
            if bj >= ns {
                if v.abs() > 1e-9 {
                    return false;
                }
                continue;
            }
            let ci = self.cols[bj];
            y[ci.con as usize] += ci.sign * v;
        }
        let ysum: f64 = y.iter().map(|v| v.abs()).sum::<f64>().max(1e-300);
        let mut amax: f64 = 0.0;
        for i in 0..m {
            if y[i] != 0.0 {
                if !self.is_eq[i] && y[i] < -1e-9 * ysum {
                    return false;
                }
                amax = amax.max(self.a.row(i).iter().fold(0.0f64, |s, v| s.max(v.abs())));
            }
        }
        let amax = amax.max(1e-300);
        for k in 0..n {
            let mut s = 0.0;
            for i in 0..m {
                if y[i] != 0.0 {
                    s += self.a[(i, k)] * y[i];
                }
            }
            if s.abs() > 1e-9 * ysum * amax {
                return false;
            }
        }
        dot(self.b, &y) < -1e-12 * ysum
    }

    /// x = pi read from the tableau satisfies every original row (= dual reduced costs >= 0).
    fn primal_ok(&self, tab: &Tab, x: &mut Vec<f64>) -> bool {
        let n = self.n;
        let ns = self.cols.len();
        x.clear();
        x.extend((0..n).map(|k| -self.sigma[k] * tab.at(n, ns + k)));
        for i in 0..self.a.rows {
            let row = self.a.row(i);
            let mut ax = 0.0;
            let mut mag = 0.0;
            for (u, v) in row.iter().zip(x.iter()) {
                let p = u * v;
                ax += p;
                mag += p.abs();
            }
            let s = self.b[i] - ax;
            let viol = if self.is_eq[i] { s.abs() } else { -s };
            if viol > 1e-9 * (1.0 + self.b[i].abs() + mag) {
                return false;
            }
        }
        true
    }
}

/// Solves min c'x s.t. A x <= b (rows in `eq` at equality), x free. Returns None if infeasible
/// or unbounded. With `Want::Feasible` the returned `Core` has empty vectors.
pub(crate) fn solve(c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize], want: Want) -> Option<Core> {
    solve_from(c, a, b, eq, None, want)
}

/// Same LP, with the origin shifted to `x0` (x = x0 + z): an exact change of variables that only
/// changes the simplex starting point. A feasible or nearly feasible `x0` leaves few violated
/// rows and so few pivots.
pub(crate) fn solve_from(c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize], x0: Option<&[f64]>, want: Want) -> Option<Core> {
    SCRATCH.with(|cell| {
        let mut guard = cell.borrow_mut();
        let s = &mut *guard;
        let rhs = match x0 {
            None => rev::Rhs::Plain,
            Some(x0) => rev::Rhs::Shift(x0),
        };
        if !TABLEAU_ONLY.load(Ordering::Relaxed) {
            if let rev::Rev::Done(out) = rev::solve(&mut s.rev, c, a, b, eq, rhs, want) {
                return out;
            }
        }
        match x0 {
            None => solve_in(s, c, a, b, eq, want),
            Some(x0) => {
                let mut bs = std::mem::take(&mut s.bshift);
                bs.clear();
                bs.extend((0..a.rows).map(|i| b[i] - dot(a.row(i), x0)));
                let out = solve_in(s, c, a, &bs, eq, want).map(|mut core| {
                    if !core.x.is_empty() {
                        for (v, o) in core.x.iter_mut().zip(x0) {
                            *v += o;
                        }
                    }
                    core
                });
                s.bshift = bs;
                out
            }
        }
    })
}

/// `solve_from` with the shifted right hand side `bs = b - A x0` supplied by the caller.
pub(crate) fn solve_shifted(a: &Mat, bs: &[f64], eq: &[usize], x0: &[f64], want: Want) -> Option<Core> {
    SCRATCH.with(|cell| {
        let mut guard = cell.borrow_mut();
        if !TABLEAU_ONLY.load(Ordering::Relaxed) {
            if let rev::Rev::Done(out) = rev::solve(&mut guard.rev, None, a, bs, eq, rev::Rhs::Shifted(x0), want) {
                return out;
            }
        }
        solve_in(&mut guard, None, a, bs, eq, want).map(|mut core| {
            if !core.x.is_empty() {
                for (v, o) in core.x.iter_mut().zip(x0) {
                    *v += o;
                }
            }
            core
        })
    })
}

fn solve_in(s: &mut Scratch, c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize], want: Want) -> Option<Core> {
    let n = a.cols;
    let m = a.rows;
    debug_assert_eq!(b.len(), m);
    s.is_eq.clear();
    s.is_eq.resize(m, false);
    for &e in eq {
        s.is_eq[e] = true;
    }
    if n == 0 {
        for i in 0..m {
            if b[i] < -1e-9 || (s.is_eq[i] && b[i].abs() > 1e-9) {
                return None;
            }
        }
        return Some(Core { x: vec![], dual: if want == Want::Full { vec![0.0; m] } else { vec![] } });
    }
    let zero = [0.0f64; 64];
    let zv;
    let c: &[f64] = match c {
        Some(c) => c,
        None if n <= 64 => &zero[..n],
        None => {
            zv = vec![0.0; n];
            &zv
        }
    };

    s.cols.clear();
    for i in 0..m {
        s.cols.push(ColInfo { con: i as u32, sign: 1.0 });
        if s.is_eq[i] {
            s.cols.push(ColInfo { con: i as u32, sign: -1.0 });
        }
    }
    let nstruct = s.cols.len();
    let ncols = nstruct + n;
    let width = ncols + 1;
    let rhs = ncols;
    s.sigma.clear();
    s.sigma.extend(c.iter().map(|&ck| if -ck < 0.0 { -1.0 } else { 1.0 }));

    // tableau: rows 0..n, phase-2 cost row n, phase-1 cost row n+1
    s.t.clear();
    s.t.resize((n + 2) * width, 0.0);
    {
        let t = &mut s.t;
        for (j, ci) in s.cols.iter().enumerate() {
            let arow = a.row(ci.con as usize);
            let mut p1 = 0.0;
            for k in 0..n {
                let v = s.sigma[k] * ci.sign * arow[k];
                t[k * width + j] = v;
                p1 += v;
            }
            t[n * width + j] = ci.sign * b[ci.con as usize];
            t[(n + 1) * width + j] = -p1;
        }
        let mut p1r = 0.0;
        for k in 0..n {
            t[k * width + nstruct + k] = 1.0;
            let r = s.sigma[k] * (-c[k]);
            t[k * width + rhs] = r;
            p1r += r;
        }
        t[(n + 1) * width + rhs] = -p1r;
    }
    s.basis.clear();
    s.basis.extend(nstruct..nstruct + n);
    let c_scale = c.iter().fold(1.0f64, |m, v| m.max(v.abs()));
    s.inv1.clear();
    s.inv1.resize(ncols, c_scale);
    s.inv2.clear();
    s.inv2.extend(s.cols.iter().map(|ci| 1.0 + b[ci.con as usize].abs()));
    s.inv2.resize(ncols, 1.0);

    let Scratch { t, basis, prow, cols, sigma, is_eq, inv1, inv2, x, y, .. } = s;
    let data = Data { a, b, c, n, cols, sigma, is_eq };
    let mut tab = Tab { nrows: n, ncols, width, nstruct, live: n + 2, t, basis, prow };
    let max_iter = 50 * (n + ncols) + 1000;

    if let Outcome::IterLimit = tab.iterate(n + 1, inv1, max_iter, false) {
        return None;
    }
    let infeas: f64 = (0..n).filter(|&r| tab.basis[r] >= nstruct).map(|r| tab.at(r, rhs).abs()).sum();
    if infeas > 1e-9 * c_scale {
        return None; // dual infeasible -> primal unbounded (or infeasible)
    }
    tab.live = n + 1;
    // artificials still basic (at zero) are not pivoted out here: phase 2 blocks them and removes
    // one only when an entering column actually needs its row
    for r in 0..if DRIVE_OUT { n } else { 0 } {
        if tab.basis[r] >= nstruct {
            let mut best = 0.0;
            let mut col = usize::MAX;
            for j in 0..nstruct {
                let v = tab.at(r, j).abs();
                if v > best {
                    best = v;
                    col = j;
                }
            }
            if best > 1e-9 {
                tab.pivot(r, col);
            }
        }
    }

    let mut verified = false;
    for round in 0..REFACTOR_ROUNDS {
        match tab.iterate(n, inv2, max_iter, true) {
            Outcome::IterLimit => return None,
            Outcome::Unbounded(e) => {
                if data.certificate_holds(&tab, e, y) || round + 1 == REFACTOR_ROUNDS {
                    return None; // primal infeasible
                }
                if !data.refactor(&mut tab) {
                    return None;
                }
            }
            Outcome::Optimal => {
                if data.primal_ok(&tab, x) {
                    verified = true;
                    break;
                }
                if !data.refactor(&mut tab) {
                    break;
                }
                if data.primal_ok(&tab, x) {
                    verified = true;
                    break;
                }
            }
        }
    }
    if !verified {
        return None;
    }
    match want {
        Want::Feasible => Some(Core { x: vec![], dual: vec![] }),
        Want::Point => Some(Core { x: x.clone(), dual: vec![] }),
        Want::Full => {
            let mut dual = vec![0.0; m];
            for r in 0..n {
                let bj = tab.basis[r];
                if bj < nstruct {
                    let ci = data.cols[bj];
                    let v = tab.at(r, rhs);
                    let v = if v < 0.0 && v > -1e-9 { 0.0 } else { v };
                    dual[ci.con as usize] += ci.sign * v;
                }
            }
            Some(Core { x: x.clone(), dual })
        }
    }
}
