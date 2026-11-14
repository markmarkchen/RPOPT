//! Multiparametric linear complementarity (mp-LCP) solver for strictly convex mpQPs.
//!
//! Jones and Morari, "Multiparametric linear complementarity problems", CDC 2006 (Algorithms 1
//! and 2): the KKT system of the mpQP is the pLCP  w - M z = q + R θ  (z = multipliers,
//! w = slacks, M = A Q^{-1} A' positive semidefinite). Critical regions are enumerated
//! facet to facet. A region is a complementary basis B; its neighbour across a facet is found by
//! pivoting, with the lexicographic perturbation  q + R θ + ε,  ε = (ε0, ε0², ..., ε0^n),  of the
//! right hand side kept in the tableau (columns β = A_B^{-1}), so that every θ lies in exactly one
//! region and degenerate pivots are resolved consistently.
//!
//! Equality rows have free multipliers; they are eliminated before the pLCP is formed (Schur
//! complement of M on the equality block) and are always in the active set.
//!
//! Tableau form (compact, one column per nonbasic variable): x_B = ρ + T y_N, where the rows of ρ
//! are [constant | θ coefficients | β]. Variables: w_i = i, z_i = n + i, the facet parameter α = 2n.

use crate::critical_region::CriticalRegion;
use crate::fxhash::FxHashSet as HashSet;
use crate::linalg::{Lu, Mat};
use crate::mpqp_utils::gen_cr_from_active_set_lambda_first;
use crate::program::MpProgram;
use crate::utils::chebyshev_ball;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::cmp::Ordering;
use std::collections::VecDeque;

#[path = "mplcp_build.rs"]
mod build;

/// The reduced pLCP (equalities eliminated).
struct Plcp {
    n: usize,
    nt: usize,
    /// n x n
    m: Vec<f64>,
    q: Vec<f64>,
    /// n x nt
    r: Vec<f64>,
    /// original constraint index of each pLCP row
    ineq: Vec<usize>,
    eq: Vec<usize>,
}

impl Plcp {
    fn new(program: &MpProgram) -> Option<Plcp> {
        let kc = program.kkt_cache_pub()?;
        let nt = program.num_t();
        let mfull = kc.m.rows;
        let eq = program.equality_indices.clone();
        let ineq: Vec<usize> = (0..mfull).filter(|i| !eq.contains(i)).collect();
        let n = ineq.len();
        let mut m = vec![0.0; n * n];
        let mut q = vec![0.0; n];
        let mut r = vec![0.0; n * nt];
        for (a, &i) in ineq.iter().enumerate() {
            for (b, &j) in ineq.iter().enumerate() {
                m[a * n + b] = kc.m[(i, j)];
            }
            q[a] = kc.base_f[i];
            for t in 0..nt {
                r[a * nt + t] = -kc.base_e[(i, t)];
            }
        }
        if !eq.is_empty() {
            let ne = eq.len();
            let mee = Mat::from_vec(ne, ne, eq.iter().flat_map(|&i| eq.iter().map(move |&j| (i, j))).map(|(i, j)| kc.m[(i, j)]).collect());
            let lu = Lu::new(&mee)?;
            // X = M_EE^{-1} [M_EI | q_E | R_E]
            let w = n + 1 + nt;
            let mut rhs = Mat::zeros(ne, w);
            for (a, &i) in eq.iter().enumerate() {
                for (b, &j) in ineq.iter().enumerate() {
                    rhs[(a, b)] = kc.m[(i, j)];
                }
                rhs[(a, n)] = kc.base_f[i];
                for t in 0..nt {
                    rhs[(a, n + 1 + t)] = -kc.base_e[(i, t)];
                }
            }
            let x = lu.solve_mat(&rhs);
            for (a, &i) in ineq.iter().enumerate() {
                // M_IE X
                let mie: Vec<f64> = eq.iter().map(|&e| kc.m[(i, e)]).collect();
                for b in 0..n {
                    let mut s = 0.0;
                    for (k, mk) in mie.iter().enumerate() {
                        s += mk * x[(k, b)];
                    }
                    m[a * n + b] -= s;
                }
                let mut s = 0.0;
                for (k, mk) in mie.iter().enumerate() {
                    s += mk * x[(k, n)];
                }
                q[a] -= s;
                for t in 0..nt {
                    let mut s = 0.0;
                    for (k, mk) in mie.iter().enumerate() {
                        s += mk * x[(k, n + 1 + t)];
                    }
                    r[a * nt + t] -= s;
                }
            }
        }
        Some(Plcp { n, nt, m, q, r, ineq, eq })
    }

    /// Full active set (equalities first) of the z-basic rows.
    fn active_set(&self, z_rows: impl Iterator<Item = usize>) -> Vec<usize> {
        let mut v: Vec<usize> = z_rows.map(|k| self.ineq[k]).collect();
        v.sort_unstable();
        let mut out = self.eq.clone();
        out.extend(v);
        out
    }
}

/// A pivot recorded but not yet applied to the tableau values (the variable names are already
/// swapped). Entries of the pivoted tableau are computed from it on demand with exactly the
/// operations `Tab::flush` applies, so the results are bit for bit the same.
#[derive(Clone)]
struct Pend {
    r: usize,
    c: usize,
    tc: f64,
    /// [column c of T before the pivot (n) | pivot row of T after (nc) | pivot row of ρ after (w)]
    buf: Vec<f64>,
}

/// Compact tableau x_B = ρ + T y_N with `nc` nonbasic columns and rows ρ of width `w`.
#[derive(Clone)]
struct Tab {
    n: usize,
    nc: usize,
    w: usize,
    t: Vec<f64>,
    rho: Vec<f64>,
    row_var: Vec<usize>,
    col_var: Vec<usize>,
    pend: Option<Pend>,
}

impl Tab {
    /// Entry (i, k) of T (with the deferred pivot, if any, applied).
    #[inline]
    fn tv(&self, i: usize, k: usize) -> f64 {
        match &self.pend {
            None => self.t[i * self.nc + k],
            Some(p) => {
                if i == p.r {
                    return p.buf[self.n + k];
                }
                let f = p.buf[i];
                if f == 0.0 {
                    self.t[i * self.nc + k]
                } else if k == p.c {
                    f * p.tc
                } else {
                    self.t[i * self.nc + k] + f * p.buf[self.n + k]
                }
            }
        }
    }

    /// Entry (i, k) of ρ (with the deferred pivot, if any, applied).
    #[inline]
    fn rv(&self, i: usize, k: usize) -> f64 {
        match &self.pend {
            None => self.rho[i * self.w + k],
            Some(p) => {
                if i == p.r {
                    return p.buf[self.n + self.nc + k];
                }
                let f = p.buf[i];
                if f == 0.0 {
                    self.rho[i * self.w + k]
                } else {
                    self.rho[i * self.w + k] + f * p.buf[self.n + self.nc + k]
                }
            }
        }
    }

    /// Principal pivot (row r, column c).
    fn pivot(&mut self, r: usize, c: usize) {
        self.defer_pivot(r, c);
        self.flush();
    }

    /// Records the principal pivot (row r, column c); the tableau must have no pending pivot.
    fn defer_pivot(&mut self, r: usize, c: usize) {
        debug_assert!(self.pend.is_none());
        let (n, nc, w) = (self.n, self.nc, self.w);
        let p = self.t[r * nc + c];
        let inv = 1.0 / p;
        let mut buf = vec![0.0; n + nc + w];
        for i in 0..n {
            buf[i] = self.t[i * nc + c];
        }
        for k in 0..nc {
            let v = self.t[r * nc + k];
            buf[n + k] = if k == c { inv } else { -v * inv };
        }
        for k in 0..w {
            buf[n + nc + k] = self.rho[r * w + k] * -inv;
        }
        let tc = buf[n + c];
        self.pend = Some(Pend { r, c, tc, buf });
        std::mem::swap(&mut self.row_var[r], &mut self.col_var[c]);
    }

    /// Applies the pending pivot to the values.
    fn flush(&mut self) {
        let Some(p) = self.pend.take() else { return };
        let (n, nc, w) = (self.n, self.nc, self.w);
        let (fcol, rest) = p.buf.split_at(n);
        let (trow, rrow) = rest.split_at(nc);
        for i in 0..n {
            if i == p.r {
                continue;
            }
            let f = fcol[i];
            if f == 0.0 {
                continue;
            }
            let ti = &mut self.t[i * nc..(i + 1) * nc];
            for (o, s) in ti.iter_mut().zip(trow.iter()) {
                *o += f * s;
            }
            ti[p.c] = f * p.tc;
            let ri = &mut self.rho[i * w..(i + 1) * w];
            for (o, s) in ri.iter_mut().zip(rrow.iter()) {
                *o += f * s;
            }
        }
        self.t[p.r * nc..(p.r + 1) * nc].copy_from_slice(trow);
        self.rho[p.r * w..(p.r + 1) * w].copy_from_slice(rrow);
    }

    fn col_of(&self, var: usize) -> usize {
        self.col_var.iter().position(|&v| v == var).unwrap()
    }
}

/// Persistent tableau of the basis with z-basic rows `z`: rows [q | R | I], T = M, then one
/// principal pivot per z row (nonsingular when M_zz is positive definite).
fn tableau_for(p: &Plcp, z: &[usize]) -> Option<Tab> {
    let (n, nt) = (p.n, p.nt);
    let w = 1 + nt + n;
    let mut rho = vec![0.0; n * w];
    for i in 0..n {
        rho[i * w] = p.q[i];
        rho[i * w + 1..i * w + 1 + nt].copy_from_slice(&p.r[i * nt..(i + 1) * nt]);
        rho[i * w + 1 + nt + i] = 1.0;
    }
    let mut tab = Tab { n, nc: n, w, t: p.m.clone(), rho, row_var: (0..n).collect(), col_var: (n..2 * n).collect(), pend: None };
    for &i in z {
        let piv = tab.t[i * n + i];
        if piv.abs() < 1e-11 {
            return None;
        }
        tab.pivot(i, i);
    }
    Some(tab)
}

const ZTOL: f64 = 1e-9;
const PTOL: f64 = 1e-10;

fn comp(var: usize, n: usize) -> usize {
    if var < n {
        var + n
    } else {
        var - n
    }
}

fn lex_cmp(a: &[f64], b: &[f64]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        let tol = 1e-12 * (1.0 + x.abs().max(y.abs()));
        if x - y > tol {
            return Ordering::Greater;
        }
        if y - x > tol {
            return Ordering::Less;
        }
    }
    Ordering::Equal
}

/// Is `v` (first entry the scalar part, then the perturbation columns) lexico-nonnegative?
fn lex_nonneg(v: &[f64]) -> bool {
    for x in v {
        if x.abs() > ZTOL {
            return *x > 0.0;
        }
    }
    true
}

struct Facet<'a> {
    /// facet-space rows of the polytope F (E_f θ_f <= f_f)
    ef: &'a Mat,
    ff: &'a [f64],
}

/// Algorithm 2 (`pivot`): from the ACLFB `tab` (α basic or entering as column `e`) returns the
/// z-row sets of the adjacent bases.
fn pivot_facet(p: &Plcp, tab: &mut Tab, e: usize, fpoly: Facet, depth: usize, out: &mut Vec<Vec<usize>>) {
    let n = p.n;
    PROF[9].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d1 = p.nt - 1;
    let alpha = 2 * n;
    let mut e = e;
    for _ in 0..(8 * n + 50) {
        // P: rows decreasing in the entering variable
        let a: Vec<f64> = (0..n).map(|i| -tab.tv(i, e)).collect();
        let w = tab.w;
        let rowmax = |i: usize| (0..1 + d1).fold(0.0f64, |m, k| m.max(tab.rv(i, k).abs()));
        let pset: Vec<usize> = (0..n).filter(|&i| a[i] > PTOL).collect();
        if pset.is_empty() {
            return;
        }
        let in_z = |i: usize| rowmax(i) <= ZTOL;
        let pz: Vec<usize> = pset.iter().cloned().filter(|&i| in_z(i)).collect();
        let alpha_row = tab.row_var.iter().position(|&v| v == alpha);
        if pz.is_empty() {
            if let Some(ra) = alpha_row {
                if tab.tv(ra, e) > PTOL {
                    out.push(z_after(tab, n, ra, e));
                    return;
                }
            }
        }
        if !pz.is_empty() {
            // Case 1: leaving variable independent of θ_f
            let beta = |i: usize| -> Vec<f64> { (1 + d1..w).map(|k| tab.rv(i, k)).collect() };
            let mut l = pz[0];
            for &i in &pz[1..] {
                let u: Vec<f64> = beta(i).iter().map(|v| v / a[i]).collect();
                let v: Vec<f64> = beta(l).iter().map(|v| v / a[l]).collect();
                if lex_cmp(&u, &v) == Ordering::Less {
                    l = i;
                }
            }
            let leaving = tab.row_var[l];
            if leaving == alpha {
                return;
            }
            tab.flush();
            tab.defer_pivot(l, e);
            e = tab.col_of(comp(leaving, n));
            continue;
        }
        // Case 2: the leaving variable depends on θ_f
        PROF[12].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        PROF[13].fetch_add(pset.len() as u64, std::sync::atomic::Ordering::Relaxed);
        if depth > 6 {
            return;
        }
        for &l in &pset {
            let leaving = tab.row_var[l];
            // lexico-inequalities (16): Γ_j = a_l row_j - a_j row_l  ⪰ 0 for all j in P
            let mut ok = true;
            let mut ef = fpoly.ef.clone();
            let mut ff = fpoly.ff.to_vec();
            for &j in &pset {
                if j == l {
                    continue;
                }
                let g: Vec<f64> = (0..w).map(|k| a[l] * tab.rv(j, k) - a[j] * tab.rv(l, k)).collect();
                let gq_zero = g[1..1 + d1].iter().all(|v| v.abs() <= ZTOL);
                if gq_zero {
                    let mut v = vec![g[0]];
                    v.extend_from_slice(&g[1 + d1..]);
                    if !lex_nonneg(&v) {
                        ok = false;
                        break;
                    }
                } else {
                    // g0 + g_q θ_f >= 0  ->  -g_q θ_f <= g0
                    let nrm = g[1..1 + d1].iter().map(|v| v * v).sum::<f64>().sqrt();
                    let row: Vec<f64> = g[1..1 + d1].iter().map(|v| -v / nrm).collect();
                    ef.push_row(&row);
                    ff.push(g[0] / nrm);
                }
            }
            if !ok {
                continue;
            }
            if d1 > 0 {
                PROF[10].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                match chebyshev_ball(&ef, &ff, &[]) {
                    Some(s) if s.x[d1] > 1e-9 => {
                        PROF[14].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    _ => continue,
                }
            }
            if leaving == alpha {
                out.push(z_after(tab, n, l, e));
                continue;
            }
            let mut next = tab.clone();
            next.flush();
            next.defer_pivot(l, e);
            let ne = next.col_of(comp(leaving, n));
            pivot_facet(p, &mut next, ne, Facet { ef: &ef, ff: &ff }, depth + 1, out);
        }
        return;
    }
}

/// z rows of the basis `tab` with the pair of row v toggled.
fn z_toggle(tab: &Tab, n: usize, v: usize) -> Vec<usize> {
    let mut z = z_rows(tab, n);
    let var = tab.row_var[v];
    if var < n {
        z.push(var);
        z.sort_unstable();
    } else {
        z.retain(|&x| x != var - n);
    }
    z
}

/// z rows of the basis reached from `tab` by the pivot (row r, column c), without doing the pivot.
fn z_after(tab: &Tab, n: usize, r: usize, c: usize) -> Vec<usize> {
    let mut v: Vec<usize> = tab
        .row_var
        .iter()
        .enumerate()
        .map(|(i, &x)| if i == r { tab.col_var[c] } else { x })
        .filter(|&x| x >= n && x < 2 * n)
        .map(|x| x - n)
        .collect();
    v.sort_unstable();
    v
}

fn z_rows(tab: &Tab, n: usize) -> Vec<usize> {
    let mut v: Vec<usize> = tab.row_var.iter().filter(|&&x| x >= n && x < 2 * n).map(|&x| x - n).collect();
    v.sort_unstable();
    v
}

/// Orthonormal basis (columns, nt x (nt-1)) of the orthogonal complement of the unit vector g.
fn null_basis(g: &[f64]) -> Vec<Vec<f64>> {
    let nt = g.len();
    let mut basis: Vec<Vec<f64>> = vec![];
    let skip = (0..nt).max_by(|&a, &b| g[a].abs().partial_cmp(&g[b].abs()).unwrap()).unwrap();
    for k in 0..nt {
        if k == skip {
            continue;
        }
        let mut v = vec![0.0; nt];
        v[k] = 1.0;
        for u in std::iter::once(g).chain(basis.iter().map(|b| b.as_slice())) {
            let dp: f64 = v.iter().zip(u).map(|(a, b)| a * b).sum();
            for (x, y) in v.iter_mut().zip(u) {
                *x -= dp * y;
            }
        }
        let nrm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        for x in v.iter_mut() {
            *x /= nrm;
        }
        basis.push(v);
    }
    basis
}

/// A neighbouring basis: the principal pivot (row, column) of the current tableau, or its z rows.
enum Nbr {
    Toggle(usize, usize),
    Z(Vec<usize>),
}

/// Neighbouring z-row sets across the facet {e_k θ = f_k} of the region with tableau `tab`.
fn adjacent(p: &Plcp, tab: &Tab, erow: &[f64], frhs: f64, e_all: &Mat, f_all: &[f64], ws: &mut Option<Tab>) -> Vec<Nbr> {
    let (n, nt) = (p.n, p.nt);
    PROF[8].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nrm = erow.iter().map(|v| v * v).sum::<f64>().sqrt();
    let g: Vec<f64> = erow.iter().map(|v| v / nrm).collect();
    let b = -frhs / nrm; // γ'θ + b = 0
    let d1 = nt - 1;
    // rows identically zero on the facet (none: boundary of Θ), and the α column γ_i = ρθ_i · g
    let mut gam = vec![0.0; n];
    let mut zrows: Vec<usize> = vec![];
    for i in 0..n {
        let rr = &tab.rho[i * tab.w..(i + 1) * tab.w];
        let gi: f64 = (0..nt).map(|k| rr[1 + k] * g[k]).sum();
        gam[i] = gi;
        if (rr[0] - b * gi).abs() <= ZTOL && (0..nt).map(|k| (rr[1 + k] - gi * g[k]).powi(2)).sum::<f64>().sqrt() <= ZTOL {
            zrows.push(i);
        }
    }
    if zrows.is_empty() {
        return vec![];
    }
    // Fast path of Algorithm 2: a single leaving candidate v in P ∩ Z (Case 1 without ties), after
    // which α is basic and the entering variable comp(v) makes P' ∩ Z' empty with α rising:
    // the neighbour is the principal pivot (toggle) of v. Only column entries are needed.
    let pv: Vec<usize> = zrows.iter().cloned().filter(|&i| gam[i] < -PTOL).collect();
    if pv.len() == 1 {
        let v = pv[0];
        let var = tab.row_var[v];
        let kk = tab.col_of(comp(var, n));
        let pp = gam[v];
        let tve = -tab.t[v * n + kk] / pp;
        if tve > PTOL {
            let blocked = zrows.iter().any(|&i| i != v && -(tab.t[i * n + kk] + gam[i] * tve) > PTOL);
            if !blocked {
                return vec![Nbr::Toggle(v, kk)];
            }
        }
    }
    let nb = null_basis(&g);
    let w = 1 + d1 + n;
    // work tableau: n x (n+1) with the α column last
    let nc = n + 1;
    // the buffers of the previous work tableau are reused (every entry is overwritten)
    let mut work = ws.take().unwrap_or_else(|| Tab { n, nc, w, t: vec![0.0; n * nc], rho: vec![0.0; n * w], row_var: vec![0; n], col_var: vec![0; nc], pend: None });
    for i in 0..n {
        work.t[i * nc..i * nc + n].copy_from_slice(&tab.t[i * n..(i + 1) * n]);
        let rr = &tab.rho[i * tab.w..(i + 1) * tab.w];
        work.t[i * nc + n] = gam[i];
        work.rho[i * w] = rr[0] - b * gam[i];
        for (c, v) in nb.iter().enumerate() {
            work.rho[i * w + 1 + c] = (0..nt).map(|k| rr[1 + k] * v[k]).sum();
        }
        work.rho[i * w + 1 + d1..(i + 1) * w].copy_from_slice(&rr[1 + nt..]);
    }
    work.row_var.copy_from_slice(&tab.row_var);
    work.col_var[..n].copy_from_slice(&tab.col_var);
    work.col_var[n] = 2 * n;
    work.pend = None;
    // F: region ∩ Θ restricted to the facet hyperplane, θ = N θ_f - γ b
    let mut ef = Mat::zeros(0, d1);
    let mut ff = vec![];
    if d1 > 0 {
        for i in 0..e_all.rows {
            let r = e_all.row(i);
            let row: Vec<f64> = nb.iter().map(|v| r.iter().zip(v).map(|(a, b)| a * b).sum()).collect();
            let nr = row.iter().map(|v| v * v).sum::<f64>().sqrt();
            if nr < 1e-12 {
                continue;
            }
            let eg: f64 = r.iter().zip(&g).map(|(a, b)| a * b).sum();
            ef.push_row(&row.iter().map(|v| v / nr).collect::<Vec<_>>());
            ff.push((f_all[i] + b * eg) / nr);
        }
    }
    let mut out = vec![];
    let t0 = std::time::Instant::now();
    pivot_facet(p, &mut work, n, Facet { ef: &ef, ff: &ff }, 0, &mut out);
    pt(2, t0);
    *ws = Some(work);
    out.into_iter().map(Nbr::Z).collect()
}

/// Which pLCP method `solve` runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// Jones-Morari 2006 facet pivoting with lexicographic perturbation (strictly convex mpQP)
    Pivot,
    /// `Pivot` with the facets of each wave of regions explored in parallel (rayon)
    PivotParallel,
    /// Herceg et al. 2015 enumeration of complementary bases (mpQP and mpLP)
    Enumeration,
}

/// Solves the program through its pLCP. Programs the method cannot handle give an `Err` with the
/// reason (the dispatcher turns it into an empty solution and a warning); nothing panics.
///
/// Degenerate problems: the pivoting method follows a global lexicographic perturbation, so at a
/// degenerate vertex (dependent active constraints) it splits the parameter space into the
/// lexico-feasible regions, which can be more and smaller regions than the single region
/// `geometric` finds there. The regions still partition the feasible set (no overlap, no gap, see
/// tests/mplcp.rs), but the region signature can differ from `geometric`'s (and `ppopt_bench`'s
/// same-as-first check can fail there).
/// The enumeration method has no perturbation: where several bases describe the same
/// parameter set (degenerate mpLPs) it may return overlapping regions.
pub fn solve(program: &MpProgram, seed: u64, method: Method) -> Result<Vec<CriticalRegion>, String> {
    match method {
        Method::Pivot | Method::PivotParallel => {
            if program.q.is_none() {
                return Err("mplcp: mpLP not supported by the pivoting method (use mplcp_enum)".into());
            }
            let plcp = Plcp::new(program).ok_or("mplcp: no pLCP form (Q not positive definite or singular equality block)")?;
            Ok(if method == Method::PivotParallel { pivot_solve_par(program, &plcp, seed) } else { pivot_solve(program, &plcp, seed) })
        }
        Method::Enumeration => enum_solve(program),
    }
}

/// `solve` for the dispatcher: an unsupported program gives an empty list and a warning on stderr.
pub fn regions(program: &MpProgram, seed: u64, method: Method) -> Vec<CriticalRegion> {
    match solve(program, seed, method) {
        Ok(r) => r,
        Err(msg) => {
            eprintln!("warning: {msg}");
            vec![]
        }
    }
}

pub static PROF: [std::sync::atomic::AtomicU64; 16] = [const { std::sync::atomic::AtomicU64::new(0) }; 16];
pub const PROF_NAMES: [&str; 15] = ["tableau_for", "adjacent_total", "pivot_facet", "seen/aset", "law_dual", "gen_cr", "tab_clone_pivot", "region_clone", "cnt:facets", "cnt:adjacent", "cnt:cheb_lp", "total", "c2 entries", "c2 sum pset", "lp feasible"];
#[inline]
fn pt(i: usize, t0: std::time::Instant) {
    PROF[i].fetch_add(t0.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
}
const REFACTOR: usize = 8;
/// smallest wave (or batch of new regions) that `pivot_solve_par` hands to rayon
const PAR_MIN: usize = 4;

struct Item {
    cr: CriticalRegion,
    center: Option<Vec<f64>>,
    tab: Option<Tab>,
    depth: usize,
}

fn pivot_solve(program: &MpProgram, plcp: &Plcp, seed: u64) -> Vec<CriticalRegion> {
    let tt = std::time::Instant::now();
    let r = pivot_solve_inner(program, plcp, seed);
    pt(11, tt);
    r
}

fn pivot_solve_inner(program: &MpProgram, plcp: &Plcp, seed: u64) -> Vec<CriticalRegion> {
    let mut rng = StdRng::seed_from_u64(seed);
    let Some(active_set) = program.gen_optimal_active_set(&mut rng) else { return vec![] };
    let Some(initial) = crate::mpqp_utils::gen_cr_from_active_set(program, &active_set, false) else { return vec![] };
    let nt = program.num_t();
    let init_center = if nt > 1 { chebyshev_ball(&initial.e, &initial.f, &[]).map(|s| s.x[..nt].to_vec()) } else { None };
    let mut regions = vec![initial.clone()];
    let mut seen: HashSet<Vec<usize>> = HashSet::default();
    seen.insert(initial.active_set.clone());
    let mut queue: VecDeque<Item> = VecDeque::new();
    queue.push_back(Item { cr: initial, center: init_center, tab: None, depth: 0 });
    let neq = plcp.eq.len();
    let mut ws: Option<Tab> = None;
    let mut pending: Vec<(CriticalRegion, Option<Vec<f64>>, Option<(usize, usize)>)> = vec![];
    while let Some(Item { cr: cur, center, tab, depth }) = queue.pop_front() {
        let tab = match tab {
            Some(t) => t,
            None => {
                // z rows of the region (position in the reduced LCP)
                let z: Vec<usize> = cur.active_set[neq..].iter().map(|c| plcp.ineq.iter().position(|x| x == c).unwrap()).collect();
                let t0 = std::time::Instant::now();
                let Some(t) = tableau_for(plcp, &z) else { continue };
                pt(0, t0);
                t
            }
        };
        for k in 0..cur.e.rows {
            let t0 = std::time::Instant::now();
            let nbrs = adjacent(plcp, &tab, cur.e.row(k), cur.f[k], &cur.e, &cur.f, &mut ws);
            pt(1, t0);
            if nbrs.is_empty() {
                continue;
            }
            // a point on the facet: the region's Chebyshev center projected onto the row
            let hint: Option<Vec<f64>> = center.as_ref().map(|c| {
                let a = cur.e.row(k);
                let s = cur.f[k] - a.iter().zip(c).map(|(x, y)| x * y).sum::<f64>();
                c.iter().zip(a).map(|(ci, ai)| ci + s * ai).collect()
            });
            for nb in nbrs {
                let t0 = std::time::Instant::now();
                let z = match &nb {
                    Nbr::Toggle(v, _) => z_toggle(&tab, plcp.n, *v),
                    Nbr::Z(z) => z.clone(),
                };
                let aset = plcp.active_set(z.into_iter());
                if !seen.insert(aset.clone()) {
                    pt(3, t0);
                    continue;
                }
                pt(3, t0);
                let built = if nt > 1 {
                    let t0 = std::time::Instant::now();
                    let r = build::build_region(program, &aset, hint.as_deref()).map(|(cr, c)| (cr, Some(c)));
                    pt(5, t0);
                    r
                } else {
                    gen_cr_from_active_set_lambda_first(program, &aset, true).map(|cr| (cr, None))
                };
                if let Some((cr, c)) = built {
                    let t0 = std::time::Instant::now();
                    regions.push(cr.clone());
                    pt(7, t0);
                    let piv = match nb {
                        Nbr::Toggle(v, kk) if depth + 1 < REFACTOR => Some((v, kk)),
                        _ => None,
                    };
                    pending.push((cr, c, piv));
                }
            }
        }
        // the neighbours' tableaux are one pivot away (rebuilt from scratch every REFACTOR levels);
        // the last one takes over this region's tableau
        let np = pending.len();
        let mut tab = Some(tab);
        for (idx, (cr, c, piv)) in pending.drain(..).enumerate() {
            let (ntab, nd) = match piv {
                Some((v, kk)) => {
                    let t0 = std::time::Instant::now();
                    let mut t = if idx + 1 == np { tab.take().unwrap() } else { tab.as_ref().unwrap().clone() };
                    t.pivot(v, kk);
                    pt(6, t0);
                    (Some(t), depth + 1)
                }
                None => (None, 0),
            };
            queue.push_back(Item { cr, center: c, tab: ntab, depth: nd });
        }
    }
    regions
}

/// A neighbour found by the facet pivoting, before its region is built.
struct Cand {
    aset: Vec<usize>,
    hint: Option<Vec<f64>>,
    /// principal pivot of the parent tableau that gives the neighbour's tableau
    piv: Option<(usize, usize)>,
}

/// `pivot_solve` with the regions of each breadth-first wave expanded and built in parallel. The
/// neighbour sets are merged in the order of the sequential method, so the same regions are found.
fn pivot_solve_par(program: &MpProgram, plcp: &Plcp, seed: u64) -> Vec<CriticalRegion> {
    use rayon::prelude::*;
    use std::sync::Arc;
    let mut rng = StdRng::seed_from_u64(seed);
    let Some(active_set) = program.gen_optimal_active_set(&mut rng) else { return vec![] };
    let Some(initial) = crate::mpqp_utils::gen_cr_from_active_set(program, &active_set, false) else { return vec![] };
    let nt = program.num_t();
    let init_center = if nt > 1 { chebyshev_ball(&initial.e, &initial.f, &[]).map(|s| s.x[..nt].to_vec()) } else { None };
    let mut regions = vec![initial.clone()];
    let mut seen: HashSet<Vec<usize>> = HashSet::default();
    seen.insert(initial.active_set.clone());
    let neq = plcp.eq.len();
    let mut wave: Vec<Item> = vec![Item { cr: initial, center: init_center, tab: None, depth: 0 }];
    while !wave.is_empty() {
        // facets of every region of the wave
        let expand = |ws: &mut Option<Tab>, Item { cr: cur, center, tab, depth }: Item| -> Option<(Arc<Tab>, usize, Vec<Cand>)> {
            let tab = match tab {
                Some(t) => t,
                None => {
                    let z: Vec<usize> = cur.active_set[neq..].iter().map(|c| plcp.ineq.iter().position(|x| x == c).unwrap()).collect();
                    tableau_for(plcp, &z)?
                }
            };
            let mut cands = vec![];
            for k in 0..cur.e.rows {
                let nbrs = adjacent(plcp, &tab, cur.e.row(k), cur.f[k], &cur.e, &cur.f, ws);
                if nbrs.is_empty() {
                    continue;
                }
                let hint: Option<Vec<f64>> = center.as_ref().map(|c| {
                    let a = cur.e.row(k);
                    let sft = cur.f[k] - a.iter().zip(c).map(|(x, y)| x * y).sum::<f64>();
                    c.iter().zip(a).map(|(ci, ai)| ci + sft * ai).collect()
                });
                for nb in nbrs {
                    let z = match &nb {
                        Nbr::Toggle(v, _) => z_toggle(&tab, plcp.n, *v),
                        Nbr::Z(z) => z.clone(),
                    };
                    let piv = match nb {
                        Nbr::Toggle(v, kk) if depth + 1 < REFACTOR => Some((v, kk)),
                        _ => None,
                    };
                    cands.push(Cand { aset: plcp.active_set(z.into_iter()), hint: hint.clone(), piv });
                }
            }
            Some((Arc::new(tab), depth, cands))
        };
        let expanded: Vec<Option<(Arc<Tab>, usize, Vec<Cand>)>> = if wave.len() >= PAR_MIN {
            wave.into_par_iter().map_init(|| None::<Tab>, |ws, item| expand(ws, item)).collect()
        } else {
            let mut ws = None;
            wave.into_iter().map(|item| expand(&mut ws, item)).collect()
        };
        // new active sets, in sequential order
        let mut jobs: Vec<(Arc<Tab>, usize, Cand)> = vec![];
        for (tab, depth, cands) in expanded.into_iter().flatten() {
            for c in cands {
                if seen.insert(c.aset.clone()) {
                    jobs.push((tab.clone(), depth, c));
                }
            }
        }
        let build = |(tab, depth, c): (Arc<Tab>, usize, Cand)| -> Option<Item> {
            let (cr, center) = if nt > 1 {
                let (cr, ce) = build::build_region(program, &c.aset, c.hint.as_deref())?;
                (cr, Some(ce))
            } else {
                (gen_cr_from_active_set_lambda_first(program, &c.aset, true)?, None)
                };
                let (ntab, nd) = match c.piv {
                    Some((v, kk)) => {
                        let mut t = (*tab).clone();
                        t.pivot(v, kk);
                        (Some(t), depth + 1)
                    }
                    None => (None, 0),
                };
                Some(Item { cr, center, tab: ntab, depth: nd })
        };
        let built: Vec<Option<Item>> = if jobs.len() >= PAR_MIN { jobs.into_par_iter().map(build).collect() } else { jobs.into_iter().map(build).collect() };
        wave = built.into_iter().flatten().collect();
        for it in &wave {
            regions.push(it.cr.clone());
        }
    }
    regions
}

// ---------------------------------------------------------------------------- enumeration

/// pLCP of an mpLP in the general-M form of Herceg et al. 2015 (eq. 9 with H = 0, x = x+ - x-):
/// pairs (v+_i, u+_i), (v-_i, u-_i) for the split free variables, then (s_j, λ_j).
fn lp_plcp(program: &MpProgram) -> Plcp {
    let nx = program.num_x();
    let nt = program.num_t();
    let mc = program.num_constraints();
    let n = 2 * nx + mc;
    let mut m = vec![0.0; n * n];
    let mut q = vec![0.0; n];
    let mut r = vec![0.0; n * nt];
    for i in 0..nx {
        q[i] = program.c[i];
        q[nx + i] = -program.c[i];
        for t in 0..nt {
            r[i * nt + t] = program.h[(i, t)];
            r[(nx + i) * nt + t] = -program.h[(i, t)];
        }
        for j in 0..mc {
            let a = program.a[(j, i)];
            m[i * n + 2 * nx + j] = a; // v+ = c + Hθ + A'λ
            m[(nx + i) * n + 2 * nx + j] = -a;
            m[(2 * nx + j) * n + i] = -a; // s = b + Fθ - A (u+ - u-)
            m[(2 * nx + j) * n + nx + i] = a;
        }
    }
    for j in 0..mc {
        q[2 * nx + j] = program.b[j];
        for t in 0..nt {
            r[(2 * nx + j) * nt + t] = program.f[(j, t)];
        }
    }
    Plcp { n, nt, m, q, r, ineq: vec![], eq: vec![] }
}

/// State of the depth-first form of Algorithm 1 (pairs are decided one by one; `choice[j]` is
/// true when z_j is basic and false when w_j is).
struct Enumerator<'a> {
    p: &'a Plcp,
    a_t: &'a Mat,
    b_t: &'a [f64],
    choice: Vec<bool>,
    /// orthonormal basis of the columns chosen so far (rank test, line 7)
    ortho: Vec<Vec<f64>>,
    leaves: Vec<Vec<bool>>,
    /// scratch for the feasibility LP
    zsel: Vec<usize>,
}

impl<'a> Enumerator<'a> {
    /// w_j (j < n) at the point (z, θ).
    fn w_value(&self, j: usize, z: &[f64], th: &[f64]) -> f64 {
        let p = self.p;
        let mut v = p.q[j];
        for (t, x) in th.iter().enumerate() {
            v += p.r[j * p.nt + t] * x;
        }
        for (k, zk) in z.iter().enumerate() {
            if *zk != 0.0 {
                v += p.m[j * p.n + k] * zk;
            }
        }
        v
    }

    /// F(B') of eq. (10) for the partial basis of the first `i` pairs: is there (z, θ) with
    /// θ in Θ, nonnegative w and z, z_k = 0 for k < i with w_k basic, and w_k = 0 for k < i with
    /// z_k basic? (The undecided pairs are only required to be nonnegative.) Returns a point (z, θ).
    fn feasible(&mut self, i: usize) -> Option<(Vec<f64>, Vec<f64>)> {
        let p = self.p;
        let (n, nt) = (p.n, p.nt);
        self.zsel.clear();
        self.zsel.extend((0..i).filter(|&k| self.choice[k]));
        self.zsel.extend(i..n);
        let nz = self.zsel.len();
        let nv = nz + nt;
        let neq = (0..i).filter(|&j| self.choice[j]).count();
        let rows = n + nz + self.a_t.rows;
        let mut a = Mat::zeros(rows, nv);
        let mut b = vec![0.0; rows];
        let (mut e_row, mut i_row) = (0, neq);
        for j in 0..n {
            // w_j = q_j + R_j θ + M_j z
            let is_eq = j < i && self.choice[j];
            let (row, sign) = if is_eq {
                e_row += 1;
                (e_row - 1, 1.0)
            } else {
                i_row += 1;
                (i_row - 1, -1.0)
            };
            let ar = a.row_mut(row);
            for (c, &k) in self.zsel.iter().enumerate() {
                ar[c] = sign * p.m[j * n + k];
            }
            for t in 0..nt {
                ar[nz + t] = sign * p.r[j * nt + t];
            }
            b[row] = -sign * p.q[j];
        }
        for c in 0..nz {
            a[(n + c, c)] = -1.0;
        }
        for k in 0..self.a_t.rows {
            let ar = a.row_mut(n + nz + k);
            ar[nz..].copy_from_slice(self.a_t.row(k));
            b[n + nz + k] = self.b_t[k];
        }
        let eq: Vec<usize> = (0..neq).collect();
        let y = crate::solver::lp::lp_feasible_point(&a, &b, &eq, None)?;
        let mut z = vec![0.0; n];
        for (c, &k) in self.zsel.iter().enumerate() {
            z[k] = y[c];
        }
        Some((z, y[nz..].to_vec()))
    }

    /// Residual of the column of pair `i` (w_i = e_i or z_i = -M e_i) against the chosen columns;
    /// pushes the normalized residual when the rank grows.
    fn push_column(&mut self, i: usize, z_basic: bool) -> bool {
        let n = self.p.n;
        let mut col = vec![0.0; n];
        if z_basic {
            for j in 0..n {
                col[j] = -self.p.m[j * n + i];
            }
        } else {
            col[i] = 1.0;
        }
        let nrm0 = col.iter().map(|v| v * v).sum::<f64>().sqrt();
        for u in &self.ortho {
            let d: f64 = col.iter().zip(u).map(|(a, b)| a * b).sum();
            for (c, x) in col.iter_mut().zip(u) {
                *c -= d * x;
            }
        }
        let nrm = col.iter().map(|v| v * v).sum::<f64>().sqrt();
        if nrm <= 1e-9 * nrm0.max(1e-300) || nrm0 == 0.0 {
            return false;
        }
        for c in col.iter_mut() {
            *c /= nrm;
        }
        self.ortho.push(col);
        true
    }

    fn dfs(&mut self, i: usize, witness: Option<(Vec<f64>, Vec<f64>)>) {
        let n = self.p.n;
        if i == n {
            self.leaves.push(self.choice.clone());
            return;
        }
        for z_basic in [false, true] {
            if !self.push_column(i, z_basic) {
                continue;
            }
            self.choice[i] = z_basic;
            // the parent's point is a witness when it already satisfies the new restriction
            let mut wit = None;
            if let Some((z, th)) = &witness {
                let tol = 1e-10;
                let ok = if z_basic { self.w_value(i, z, th).abs() <= tol * (1.0 + self.p.q[i].abs()) } else { z[i].abs() <= tol };
                if ok {
                    wit = Some((z.clone(), th.clone()));
                }
            }
            if wit.is_none() {
                wit = self.feasible(i + 1);
            }
            if let Some(w) = wit {
                self.dfs(i + 1, Some(w));
            }
            self.ortho.pop();
        }
    }
}

/// How often each pLCP row is active at random parameters (a cheap probe for the pair order).
fn activity(program: &MpProgram, p: &Plcp) -> Vec<usize> {
    let mut act = vec![0usize; p.n];
    let Some((lo, hi)) = program.theta_bbox() else { return act };
    let mut rng = StdRng::seed_from_u64(1);
    for _ in 0..64 {
        let th: Vec<f64> = lo.iter().zip(hi).map(|(l, h)| rand::Rng::gen_range(&mut rng, *l..=*h)).collect();
        if let Some(sol) = program.solve_theta_uncounted(&th) {
            for c in sol.active_set {
                if let Some(k) = p.ineq.iter().position(|&x| x == c) {
                    act[k] += 1;
                }
            }
        }
    }
    act
}

/// The pLCP with pair `perm[k]` moved to position k (any pair order is valid for Algorithm 1).
fn permuted(p: &Plcp, perm: &[usize]) -> Plcp {
    let n = p.n;
    let nt = p.nt;
    let mut m = vec![0.0; n * n];
    let mut q = vec![0.0; n];
    let mut r = vec![0.0; n * nt];
    for (a, &i) in perm.iter().enumerate() {
        q[a] = p.q[i];
        r[a * nt..(a + 1) * nt].copy_from_slice(&p.r[i * nt..(i + 1) * nt]);
        for (b, &j) in perm.iter().enumerate() {
            m[a * n + b] = p.m[i * n + j];
        }
    }
    Plcp { n, nt, m, q, r, ineq: p.ineq.clone(), eq: p.eq.clone() }
}

/// Herceg et al. (2015) Algorithm 1: enumerates the feasible complementary bases of the pLCP and
/// builds the critical region of each (full-dimensional ones only).
fn enum_solve(program: &MpProgram) -> Result<Vec<CriticalRegion>, String> {
    let is_lp = program.q.is_none();
    let plcp = if is_lp {
        if !program.equality_indices.is_empty() {
            return Err("mplcp_enum: mpLP with equality constraints is not supported".into());
        }
        lp_plcp(program)
    } else {
        Plcp::new(program).ok_or("mplcp_enum: no pLCP form (Q not positive definite or singular equality block)")?
    };
    let n = plcp.n;
    // Any pair order is valid for Algorithm 1; deciding the rows that are rarely active first
    // prunes far more (2.5x to 12x fewer LPs than the natural order on the bench problems).
    let perm: Vec<usize> = if is_lp {
        (0..n).collect()
    } else {
        let act = activity(program, &plcp);
        let mut v: Vec<usize> = (0..n).collect();
        v.sort_by_key(|&k| (act[k], std::cmp::Reverse(k)));
        v
    };
    let pp = permuted(&plcp, &perm);
    let mut en = Enumerator { p: &pp, a_t: &program.a_t, b_t: &program.b_t, choice: vec![false; n], ortho: vec![], leaves: vec![], zsel: vec![] };
    en.dfs(0, None);
    let nx = program.num_x();
    let mut seen: HashSet<Vec<usize>> = HashSet::default();
    let mut regions = vec![];
    for leaf_p in std::mem::take(&mut en.leaves) {
        let mut leaf = vec![false; n];
        for (pos, &orig) in perm.iter().enumerate() {
            leaf[orig] = leaf_p[pos];
        }
        let aset = if is_lp {
            let mut a: Vec<usize> = (0..program.num_constraints()).filter(|&j| leaf[2 * nx + j]).collect();
            a.sort_unstable();
            if a.len() != nx || !program.check_active_set_rank(&a) {
                continue;
            }
            a
        } else {
            plcp.active_set((0..n).filter(|&k| leaf[k]))
        };
        if !seen.insert(aset.clone()) {
            continue;
        }
        if let Some(cr) = gen_cr_from_active_set_lambda_first(program, &aset, true) {
            regions.push(cr);
        }
    }
    Ok(regions)
}
