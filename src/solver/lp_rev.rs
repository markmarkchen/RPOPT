//! Revised form of the `lp_core` simplex for n <= 16 variables (const generic, stack arrays).
//!
//! The tableau of `lp_core` stores B^-1 M for every column and updates all of it at each pivot.
//! Here only the n x n basis inverse, the basic values and the two multiplier vectors are kept;
//! the reduced costs are recomputed from the contiguous rows of A in each pricing pass (no
//! tableau to build, no stores to column data). Pricing (Dantzig on the scaled reduced costs,
//! first index among ties; Bland after a run of degenerate pivots) and the ratio test are those
//! of `Tab::iterate`, with two changes for the equality rows:
//!
//! * an equality row is one free column that enters in whichever orientation has the negative
//!   reduced cost (the +/- column pair of the tableau, which are exact negatives); once basic it
//!   is a free variable and never leaves (it does not take part in the ratio test);
//! * in a feasibility LP (zero objective, so every pivot is degenerate and any basis is a valid
//!   start) the equality columns are pivoted into the basis first, one O(n^2) pivot each instead
//!   of an O(mn) pricing pass that finds the row violated (`crash_eq`), from a per-thread cache
//!   of the state after the shared prefix of the equality rows (`CrashCache`).
//!
//! Pricing keeps four independent running minima instead of a data-dependent branch per row
//! (the reduced cost signs of these LPs are close to random, so the branch mispredicts); the
//! short loops on the critical path of dependent pivots keep their (predictable) branches.
//! B^-1 is stored transposed so that B^-1 a and the pivot update are contiguous axpys. Measured
//! on synthetic LPs with m = 4n (`examples/lp_replay.rs scale`) this beats the tableau up to
//! n = 16, is level with it for n = 17..32 and about 2x slower above, hence n <= 16 only.
//!
//! The last pricing pass of an optimal run has computed b - A x for every nonbasic row and found
//! no violation beyond 1e-10 (1 + |b|), which implies the `primal_ok` tolerance (1e-9 (1 + |b| +
//! sum |a_k x_k|)); only the basic rows are checked again. Infeasibility needs the Farkas
//! certificate on the original data. Anything unverified returns `Undecided` and the caller runs
//! the tableau solver, so every answer is verified on the original rows whatever the path.

use super::{bland_after, Core, Want, PIVOT_TOL};
use crate::linalg::Mat;

const MAX_N: usize = 16;
/// column flags
const F_EQ: u8 = 1;
const F_BASIC: u8 = 2;
const F_NEG: u8 = 4;
/// kind of the basic variable of a row
const K_ART: u8 = 0;
const K_INEQ: u8 = 1;
const K_FREE: u8 = 2;

#[derive(Default)]
pub(crate) struct RevScratch {
    flags: Vec<u8>,
    bs: Vec<f64>,
    inv: Vec<f64>,
    crash: CrashCache,
}

/// Right hand side: `b` itself, `b - A x0` (computed here), or already shifted by `x0`.
#[derive(Clone, Copy)]
pub(crate) enum Rhs<'a> {
    Plain,
    Shift(&'a [f64]),
    Shifted(&'a [f64]),
}

pub(crate) enum Rev {
    Done(Option<Core>),
    Undecided,
}

pub(crate) fn solve(rs: &mut RevScratch, c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize], rhs: Rhs, want: Want) -> Rev {
    macro_rules! dispatch {
        ($($k:literal)*) => {
            match a.cols {
                $($k => run::<$k>(rs, c, a, b, eq, rhs, want),)*
                _ => Rev::Undecided,
            }
        };
    }
    dispatch!(1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16)
}

struct St<const N: usize> {
    basis: [usize; N],
    kind: [u8; N],
    /// B^-1 transposed: bt[k][r] = (B^-1)_{rk}
    bt: [[f64; N]; N],
    beta: [f64; N],
    /// phase-2 multipliers (x = sigma o pi)
    pi: [f64; N],
    /// phase-1 multipliers
    w: [f64; N],
    sigma: [f64; N],
}

enum Out<const N: usize> {
    Optimal,
    Unbounded { e: usize, oe: f64, u: [f64; N] },
    IterLimit,
}

/// a . x with four partial sums (short dependency chains).
#[inline(always)]
fn dotn<const N: usize>(a: &[f64], x: &[f64; N]) -> f64 {
    let a: &[f64; N] = a.try_into().unwrap();
    let mut s = [0.0f64; 4];
    for k in 0..N {
        s[k % 4] += a[k] * x[k];
    }
    (s[0] + s[1]) + (s[2] + s[3])
}

/// a . x summed in order (bitwise `linalg::dot`).
#[inline(always)]
fn dot_seq<const N: usize>(a: &[f64], x: &[f64]) -> f64 {
    let a: &[f64; N] = a.try_into().unwrap();
    let x: &[f64; N] = x.try_into().unwrap();
    let mut s = 0.0;
    for k in 0..N {
        s += a[k] * x[k];
    }
    s
}

/// Scaled reduced cost of a structural column (0 for basic columns): phase 1 -w'M_j (the caller
/// passes v = w o sigma), phase 2 (b_j - a_j x) / (1 + |b_j|) (v = x); an equality column in its
/// better orientation, -|.|.
#[inline(always)]
fn score<const N: usize, const P1: bool, const ZERO: bool>(f: u8, row: &[f64], v: &[f64; N], bsj: f64, invj: f64) -> f64 {
    let q = if P1 {
        let h = dotn::<N>(row, v);
        if f & F_EQ != 0 { -h.abs() } else { -h }
    } else {
        let g = if ZERO { bsj } else { bsj - dotn::<N>(row, v) };
        (if f & F_EQ != 0 { -g.abs() } else { g }) * invj
    };
    if f & F_BASIC != 0 { 0.0 } else { q }
}

/// Entering column: index of the smallest score below `thr`, the first one among equal values
/// (a sequential scan with strict improvement), or usize::MAX; with `bland` the first score
/// below `thr`. Four independent select chains instead of a data-dependent branch per column.
#[inline(always)]
fn price<const N: usize, const P1: bool, const ZERO: bool>(bland: bool, rows: &[f64], v: &[f64; N], bs: &[f64], inv: &[f64], flags: &[u8], thr: f64) -> usize {
    let m = bs.len();
    let (rows, inv, flags) = (&rows[..m * N], &inv[..m], &flags[..m]);
    if bland {
        return (0..m).find(|&j| score::<N, P1, ZERO>(flags[j], &rows[j * N..(j + 1) * N], v, bs[j], inv[j]) < thr).unwrap_or(usize::MAX);
    }
    let mut bv = [thr; 4];
    let mut bi = [usize::MAX; 4];
    let mut j = 0;
    while j + 4 <= m {
        for l in 0..4 {
            let i = j + l;
            let q = score::<N, P1, ZERO>(flags[i], &rows[i * N..(i + 1) * N], v, bs[i], inv[i]);
            let t = q < bv[l];
            bv[l] = if t { q } else { bv[l] };
            bi[l] = if t { i } else { bi[l] };
        }
        j += 4;
    }
    while j < m {
        let q = score::<N, P1, ZERO>(flags[j], &rows[j * N..(j + 1) * N], v, bs[j], inv[j]);
        let t = q < bv[0];
        bv[0] = if t { q } else { bv[0] };
        bi[0] = if t { j } else { bi[0] };
        j += 1;
    }
    merge_lanes(&bv, &bi)
}

/// Merges four running minima (lane l holds the first index of its minimum among j = l mod 4):
/// smallest value, then smallest index, i.e. the result of one sequential strict-improvement scan.
#[inline(always)]
fn merge_lanes(bv: &[f64; 4], bi: &[usize; 4]) -> usize {
    let (mut best, mut idx) = (bv[0], bi[0]);
    for l in 1..4 {
        let t = (bv[l] < best) | ((bv[l] == best) & (bi[l] < idx));
        best = if t { bv[l] } else { best };
        idx = if t { bi[l] } else { idx };
    }
    idx
}

/// u = B^-1 M_e for column e in orientation `oe`.
#[inline(always)]
fn ftran<const N: usize>(st: &St<N>, ae: &[f64], oe: f64) -> [f64; N] {
    let mut me = [0.0f64; N];
    for k in 0..N {
        me[k] = st.sigma[k] * oe * ae[k];
    }
    // u = sum_k me_k (column k of B^-1), contiguous in `bt`
    let mut u = [0.0f64; N];
    for k in 0..N {
        let mk = me[k];
        for r in 0..N {
            u[r] += st.bt[k][r] * mk;
        }
    }
    u
}

/// Simplex iterations: phase 1 (`P1`, artificials not blocked) or phase 2 (artificials blocked
/// at zero). `DEGEN`: zero objective, all basic values stay zero, every ratio is 0, so the ratio
/// test only picks the eligible row with the largest pivot (Bland: smallest basis index).
#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn iterate<const N: usize, const P1: bool, const DEGEN: bool>(st: &mut St<N>, rows: &[f64], bs: &[f64], inv: &[f64], flags: &mut [u8], c_scale: f64, max_iter: usize, x_zero: &mut bool) -> Out<N> {
    let m = bs.len();
    let mut degenerate = 0usize;
    let bland_limit = bland_after(N, m);
    for _ in 0..max_iter {
        if P1 {
            let mut obj = 0.0;
            for r in 0..N {
                if st.kind[r] == K_ART {
                    obj += st.beta[r];
                }
            }
            if obj <= 1e-13 {
                return Out::Optimal;
            }
        }
        let bland = degenerate > bland_limit;
        let mut v = [0.0f64; N];
        for k in 0..N {
            v[k] = if P1 { st.w[k] * st.sigma[k] } else { st.sigma[k] * st.pi[k] };
        }
        let skip_dot = !P1 && *x_zero;
        let thr = if P1 { -1e-10 * c_scale } else { -1e-10 };
        let enter = if P1 {
            price::<N, true, false>(bland, rows, &v, bs, inv, flags, thr)
        } else if skip_dot {
            price::<N, false, true>(bland, rows, &v, bs, inv, flags, thr)
        } else {
            price::<N, false, false>(bland, rows, &v, bs, inv, flags, thr)
        };
        if enter == usize::MAX {
            return Out::Optimal;
        }
        let ae = &rows[enter * N..(enter + 1) * N];
        let (enter_d, enter_neg) = if P1 {
            let h = dotn::<N>(ae, &v);
            if flags[enter] & F_EQ != 0 && h < 0.0 { (h, true) } else { (-h, false) }
        } else {
            let g = if skip_dot { bs[enter] } else { bs[enter] - dotn::<N>(ae, &v) };
            if flags[enter] & F_EQ != 0 && g > 0.0 { (-g, true) } else { (g, false) }
        };
        let oe = if enter_neg { -1.0 } else { 1.0 };
        let u = ftran::<N>(st, ae, oe);
        let mut leave = usize::MAX;
        if DEGEN {
            let mut bestp = 0.0;
            for r in 0..N {
                let a = u[r];
                let k = st.kind[r];
                let eligible = if k == K_ART { a.abs() > PIVOT_TOL } else { k == K_INEQ && a > PIVOT_TOL };
                if !eligible {
                    continue;
                }
                let better = if leave == usize::MAX {
                    true
                } else if bland {
                    st.basis[r] < st.basis[leave]
                } else {
                    a.abs() > bestp
                };
                if better {
                    bestp = a.abs();
                    leave = r;
                }
            }
            if leave == usize::MAX {
                return Out::Unbounded { e: enter, oe, u };
            }
            degenerate += 1;
        } else {
            let mut best_ratio = f64::INFINITY;
            let mut best_piv = 0.0;
            for r in 0..N {
                let a = u[r];
                let k = st.kind[r];
                let (eligible, ratio) = if k == K_FREE {
                    (false, 0.0) // free variable: never blocks
                } else if !P1 && k == K_ART {
                    (a.abs() > PIVOT_TOL, 0.0)
                } else if a > PIVOT_TOL {
                    (true, st.beta[r].max(0.0) / a)
                } else {
                    (false, 0.0)
                };
                if !eligible {
                    continue;
                }
                let better = if leave == usize::MAX || ratio < best_ratio - 1e-12 {
                    true
                } else if ratio <= best_ratio + 1e-12 {
                    if bland { st.basis[r] < st.basis[leave] } else { a.abs() > best_piv }
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
                return Out::Unbounded { e: enter, oe, u };
            }
            if best_ratio <= 1e-12 {
                degenerate += 1;
            } else {
                degenerate = 0;
            }
        }
        // reduced costs of the entering column: phase 2 (moves pi), phase 1 (moves w)
        let (d2, d1) = if P1 {
            let mut x = [0.0f64; N];
            for k in 0..N {
                x[k] = st.sigma[k] * st.pi[k];
            }
            (oe * (bs[enter] - dotn::<N>(ae, &x)), Some(enter_d))
        } else {
            (enter_d, None)
        };
        pivot::<N>(st, flags, m, leave, enter, enter_neg, &u, d2, d1);
        *x_zero = false;
    }
    Out::IterLimit
}

/// Basis change: column `enter` (orientation `neg`, B^-1 column `u`) replaces the basic variable
/// of row `leave`; the multipliers move by the entering reduced costs (`d1` in phase 1 only).
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn pivot<const N: usize>(st: &mut St<N>, flags: &mut [u8], m: usize, leave: usize, enter: usize, neg: bool, u: &[f64; N], d2: f64, d1: Option<f64>) {
    #[cfg(feature = "lp_census")]
    crate::census::piv_bump();
    let ip = 1.0 / u[leave];
    // row `leave` of B^-1 is divided by the pivot, the others lose u_r times it (column by
    // column of the transposed inverse: contiguous axpys). Each column is written once, with a
    // lane select for the pivot row: a scalar store into a freshly vector-stored column would
    // stall the vector loads of the next ftran (store forwarding).
    for k in 0..N {
        let col = &mut st.bt[k];
        let p = col[leave] * ip;
        for r in 0..N {
            let nv = col[r] - u[r] * p;
            col[r] = if r == leave { p } else { nv };
        }
        st.pi[k] += d2 * p;
        if let Some(d1) = d1 {
            st.w[k] += d1 * p;
        }
    }
    let theta = st.beta[leave] * ip;
    for r in 0..N {
        let nv = st.beta[r] - u[r] * theta;
        st.beta[r] = if r == leave { theta } else { nv };
    }
    let old = st.basis[leave];
    if old < m {
        flags[old] &= !(F_BASIC | F_NEG);
    }
    let fe = flags[enter] & F_EQ;
    flags[enter] = fe | F_BASIC | if neg { F_NEG } else { 0 };
    st.basis[leave] = enter;
    st.kind[leave] = if fe != 0 { K_FREE } else { K_INEQ };
}

/// Basis after crashing a prefix of the equality rows (one entry per thread). It depends only
/// on m, n, the prefix and the coefficients of its rows (the crash pivots on B^-1 alone, in +
/// orientation, and the multipliers are computed afterwards), so a hit restores exactly the
/// state the crash would compute: the results do not depend on what the cache held.
#[derive(Default)]
pub(crate) struct CrashCache {
    n: usize,
    m: usize,
    eq: Vec<usize>,
    rows: Vec<u64>,
    basis: Vec<usize>,
    kind: Vec<u8>,
    bt: Vec<f64>,
}

impl CrashCache {
    fn matches<const N: usize>(&self, m: usize, pre: &[usize], rows: &[f64]) -> bool {
        self.n == N
            && self.m == m
            && self.eq == pre
            && pre.iter().enumerate().all(|(i, &e)| rows[e * N..(e + 1) * N].iter().zip(&self.rows[i * N..(i + 1) * N]).all(|(a, b)| a.to_bits() == *b))
    }

    fn save<const N: usize>(&mut self, st: &St<N>, m: usize, pre: &[usize], rows: &[f64]) {
        self.n = N;
        self.m = m;
        self.eq.clear();
        self.eq.extend_from_slice(pre);
        self.rows.clear();
        for &e in pre {
            self.rows.extend(rows[e * N..(e + 1) * N].iter().map(|v| v.to_bits()));
        }
        self.basis.clear();
        self.basis.extend_from_slice(&st.basis);
        self.kind.clear();
        self.kind.extend_from_slice(&st.kind);
        self.bt.clear();
        for col in &st.bt {
            self.bt.extend_from_slice(col);
        }
    }

    fn restore<const N: usize>(&self, st: &mut St<N>, flags: &mut [u8]) {
        st.basis.copy_from_slice(&self.basis);
        st.kind.copy_from_slice(&self.kind);
        for (k, col) in st.bt.iter_mut().enumerate() {
            col.copy_from_slice(&self.bt[k * N..(k + 1) * N]);
        }
        for r in 0..N {
            if st.kind[r] != K_ART {
                flags[st.basis[r]] |= F_BASIC;
            }
        }
    }
}

/// Feasibility LPs: pivots each equality column into the basis in place of an artificial (the
/// largest |B^-1 a| among the artificial rows) before any pricing pass; rows dependent on
/// earlier ones are skipped (pricing handles them). The pivots only change B^-1; the
/// multipliers pi = c_B' B^-1 (c_j = b_j for a basic equality column) are computed at the end.
/// The state after all but the last equality row is cached: the LPs of one combinatorial group
/// share that prefix (the parent's active set).
#[inline(never)]
fn crash_eq<const N: usize>(st: &mut St<N>, rows: &[f64], bs: &[f64], flags: &mut [u8], eq: &[usize], cache: &mut CrashCache, x_zero: &mut bool) {
    let m = bs.len();
    let q = eq.len();
    let mut start = 0;
    if q >= 2 && cache.matches::<N>(m, &eq[..q - 1], rows) {
        cache.restore::<N>(st, flags);
        start = q - 1;
    }
    for (i, &e) in eq.iter().enumerate().skip(start) {
        if q >= 2 && i == q - 1 && start == 0 {
            cache.save::<N>(st, m, &eq[..q - 1], rows);
        }
        if flags[e] & F_BASIC != 0 {
            continue;
        }
        let ae = &rows[e * N..(e + 1) * N];
        let u = ftran::<N>(st, ae, 1.0);
        let mut leave = usize::MAX;
        let mut best = PIVOT_TOL;
        for r in 0..N {
            // (branches, not selects: they are mostly predictable and a select chain would sit
            // on the critical path of the dependent crash pivots)
            if st.kind[r] == K_ART && u[r].abs() > best {
                best = u[r].abs();
                leave = r;
            }
        }
        if leave == usize::MAX {
            continue;
        }
        pivot::<N>(st, flags, m, leave, e, false, &u, 0.0, None);
    }
    let mut cb = [0.0f64; N];
    for r in 0..N {
        if st.kind[r] != K_ART {
            cb[r] = bs[st.basis[r]];
        }
    }
    let mut zero = true;
    for k in 0..N {
        let mut s = 0.0;
        for r in 0..N {
            s += cb[r] * st.bt[k][r];
        }
        st.pi[k] = s;
        zero &= s == 0.0;
    }
    *x_zero = zero;
}

/// Farkas ray along entering column `e` (orientation `oe`, B^-1 column `u`) checked on the
/// original rows, as `Data::certificate_holds`.
#[allow(clippy::too_many_arguments)]
fn certificate<const N: usize>(st: &St<N>, rows: &[f64], bs: &[f64], flags: &[u8], e: usize, oe: f64, u: &[f64; N]) -> bool {
    let mut idx = [0usize; MAX_N + 1];
    let mut val = [0.0f64; MAX_N + 1];
    idx[0] = e;
    val[0] = oe;
    let mut cnt = 1;
    for r in 0..N {
        let v = -u[r];
        let bj = st.basis[r];
        if st.kind[r] == K_ART {
            if v.abs() > 1e-9 {
                return false;
            }
            continue;
        }
        if v == 0.0 {
            continue;
        }
        let o = if flags[bj] & F_NEG != 0 { -1.0 } else { 1.0 };
        idx[cnt] = bj;
        val[cnt] = o * v;
        cnt += 1;
    }
    let ysum: f64 = val[..cnt].iter().map(|v| v.abs()).sum::<f64>().max(1e-300);
    let mut amax: f64 = 0.0;
    for q in 0..cnt {
        let i = idx[q];
        if flags[i] & F_EQ == 0 && val[q] < -1e-9 * ysum {
            return false;
        }
        amax = amax.max(rows[i * N..(i + 1) * N].iter().fold(0.0f64, |s, v| s.max(v.abs())));
    }
    let amax = amax.max(1e-300);
    let mut by = 0.0;
    for q in 0..cnt {
        by += bs[idx[q]] * val[q];
    }
    for k in 0..N {
        let mut s = 0.0;
        for q in 0..cnt {
            s += rows[idx[q] * N + k] * val[q];
        }
        if s.abs() > 1e-9 * ysum * amax {
            return false;
        }
    }
    by < -1e-12 * ysum
}

#[inline(never)]
fn run<const N: usize>(rs: &mut RevScratch, c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize], rhs: Rhs, want: Want) -> Rev {
    let m = a.rows;
    let rows = &a.data[..m * N];
    let RevScratch { flags, bs: bsv, inv, crash, .. } = rs;
    flags.clear();
    flags.resize(m, 0);
    for &e in eq {
        flags[e] = F_EQ;
    }
    let (bs, x0): (&[f64], Option<&[f64]>) = match rhs {
        Rhs::Plain => (&b[..m], None),
        Rhs::Shift(x0) => {
            bsv.resize(m, 0.0);
            for (i, (s, row)) in bsv.iter_mut().zip(rows.chunks_exact(N)).enumerate() {
                *s = b[i] - dot_seq::<N>(row, x0);
            }
            (&bsv[..], Some(x0))
        }
        Rhs::Shifted(x0) => (&b[..m], Some(x0)),
    };
    inv.resize(m, 0.0);
    for (v, s) in inv.iter_mut().zip(bs) {
        *v = 1.0 / (1.0 + s.abs());
    }
    let inv = &inv[..];
    let mut st = St::<N> { basis: [0; N], kind: [K_ART; N], bt: [[0.0; N]; N], beta: [0.0; N], pi: [0.0; N], w: [1.0; N], sigma: [1.0; N] };
    for k in 0..N {
        st.basis[k] = m + k;
        st.bt[k][k] = 1.0;
    }
    let mut c_scale = 1.0f64;
    if let Some(c) = c {
        for k in 0..N {
            st.sigma[k] = if -c[k] < 0.0 { -1.0 } else { 1.0 };
            st.beta[k] = st.sigma[k] * (-c[k]);
            c_scale = c_scale.max(c[k].abs());
        }
    }
    let max_iter = 50 * (2 * N + m + eq.len()) + 1000;
    let mut x_zero = true;
    let out = if c.is_some() {
        if let Out::IterLimit = iterate::<N, true, false>(&mut st, rows, bs, inv, flags, c_scale, max_iter, &mut x_zero) {
            return Rev::Undecided;
        }
        let infeas: f64 = (0..N).filter(|&r| st.kind[r] == K_ART).map(|r| st.beta[r].abs()).sum();
        if infeas > 1e-9 * c_scale {
            super::set_status(super::LpStatus::Undecided);
            return Rev::Done(None); // dual infeasible -> primal unbounded (or infeasible)
        }
        iterate::<N, false, false>(&mut st, rows, bs, inv, flags, c_scale, max_iter, &mut x_zero)
    } else {
        crash_eq::<N>(&mut st, rows, bs, flags, eq, crash, &mut x_zero);
        iterate::<N, false, true>(&mut st, rows, bs, inv, flags, c_scale, max_iter, &mut x_zero)
    };
    match out {
        Out::IterLimit => Rev::Undecided,
        Out::Unbounded { e, oe, u } => {
            if certificate::<N>(&st, rows, bs, flags, e, oe, &u) {
                super::set_status(super::LpStatus::Infeasible);
                Rev::Done(None)
            } else {
                Rev::Undecided
            }
        }
        Out::Optimal => {
            let mut x = [0.0f64; N];
            for k in 0..N {
                x[k] = st.sigma[k] * st.pi[k];
                if !x[k].is_finite() {
                    return Rev::Undecided;
                }
            }
            // nonbasic rows passed the last pricing pass at this x; check the basic rows
            for r in 0..N {
                if st.kind[r] == K_ART {
                    continue;
                }
                let j = st.basis[r];
                let row = &rows[j * N..(j + 1) * N];
                let mut ax = 0.0;
                let mut mag = 0.0;
                for k in 0..N {
                    let p = row[k] * x[k];
                    ax += p;
                    mag += p.abs();
                }
                let s = bs[j] - ax;
                let viol = if flags[j] & F_EQ != 0 { s.abs() } else { -s };
                if viol > 1e-9 * (1.0 + bs[j].abs() + mag) {
                    return Rev::Undecided;
                }
            }
            let xv = match want {
                Want::Feasible => vec![],
                _ => {
                    let mut v = x.to_vec();
                    if let Some(x0) = x0 {
                        for (vi, o) in v.iter_mut().zip(x0) {
                            *vi += o;
                        }
                    }
                    v
                }
            };
            let dual = if want == Want::Full {
                let mut dual = vec![0.0; m];
                for r in 0..N {
                    if st.kind[r] == K_ART {
                        continue;
                    }
                    let j = st.basis[r];
                    let v = st.beta[r];
                    let v = if v < 0.0 && v > -1e-9 { 0.0 } else { v };
                    dual[j] += if flags[j] & F_NEG != 0 { -v } else { v };
                }
                dual
            } else {
                vec![]
            };
            Rev::Done(Some(Core { x: xv, dual }))
        }
    }
}
