//! Arnström and Axehill (IEEE CDC 2024, DOI 10.1109/CDC56724.2024.10886132): combinatorial
//! connected-graph mpQP solver (Algorithm 1), with the feasibility problems solved by the dual
//! active-set method DAQP (Arnström, Bemporad, Axehill, IEEE TAC 2022, Algorithm 1).
//!
//! * Section II-A: the mpQP is transformed to the multi-parametric least-distance problem (mpLDP)
//!   `min ½|u|² s.t. M u <= d(θ)` with `M = A R^{-1}`, `d(θ) = b + Fθ + M R^{-T} (Hθ + c)` (eqs. 2
//!   to 4, `Q = R'R`). Only `G = M M'` and the affine offset `d(θ) = Dθ + d0` are used afterwards.
//! * Section II-B: for an active set A with LICQ (`G_AA` nonsingular, checked by an LDL' factorization),
//!   `λ_A(θ) = -G_AA^{-1} d_A(θ)` (7) and `μ_Ā(θ) = d_Ā(θ) + G_ĀA λ_A(θ)` (8, 9); the critical region
//!   is `{θ in Θ0 : μ >= 0, λ >= 0}` (10).
//! * Section III-B, Algorithm 1: explore the graph of combinatorially adjacent active sets. A set
//!   with LICQ and a nonempty region adds its supersets and subsets; a set with LICQ and an empty
//!   region adds nothing; a set violating LICQ adds its subsets only. Emptiness is decided by the
//!   LDP `min |θ|²` over the region rows (eq. 13), solved with DAQP (`daqp` below).
//! * Remark 6: A0 is the final working set of DAQP on the mpLDP at a feasible parameter.
//!
//! Library conventions (as the other algorithms here): the output keeps the full-dimensional
//! regions, built (redundant rows removed, law in the original variables) by `gen_cr_from_kkt` for the
//! sets that DAQP found nonempty. mpLPs, problems with Q not positive definite and m > 128 are
//! delegated to `combinatorial_graph`.
//!
//! Implementation notes (deviations from the paper's pseudo-code, none changes the explored graph):
//! * the region test is two DAQP solves, first on the rows λ >= 0 and θ in Θ0 (cheap, most empty
//!   regions are detected here), then, warm started from that working set, with the μ >= 0 rows;
//! * rows are stored by columns and normalized to unit length, the μ rows of all constraints are
//!   formed by vectorized axpy sweeps over the columns of G;
//! * after a removal from the working set the LDL' factors are rebuilt (the working set has at most
//!   n_θ + 1 elements) instead of the rank-one downdate of Lemma 4.

use crate::critical_region::CriticalRegion;
use crate::fxhash::FxHashSet as HashSet;
use crate::linalg::{dot, norm, Cholesky, Mat};
use crate::mpqp_utils::{gen_cr_from_kkt, kkt_region_from_law};
use crate::program::{ControlLaw, MpProgram};
use std::collections::VecDeque;

/// Compact dense DAQP for `min ½|x|² s.t. a_i x <= b_i` (the least-distance form, H = I, f = 0,
/// R = I), with the first `n_eq` rows as equality constraints (always in the working set).
/// The rows are stored by columns (`cols[t * stride + i]` = entry t of row i) so that the primal
/// slacks of all rows are computed by vectorized column sweeps.
pub mod daqp {
    /// Primal feasibility tolerance (rows are expected to have unit norm).
    pub const PRIMAL_TOL: f64 = 1e-7;
    const DUAL_TOL: f64 = 1e-12;
    const ZERO_TOL: f64 = 1e-11;

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub enum Status {
        /// optimal: the problem is feasible
        Feasible,
        Infeasible,
        /// iteration limit or numerical breakdown
        Unknown,
    }

    /// Constraint rows `a_i x <= b_i`, i < n, stored by columns with row stride `stride`.
    #[derive(Clone, Copy)]
    pub struct Rows<'a> {
        pub cols: &'a [f64],
        pub stride: usize,
        pub b: &'a [f64],
        pub n: usize,
        pub dim: usize,
    }

    impl Rows<'_> {
        #[inline]
        fn at(&self, i: usize, t: usize) -> f64 {
            self.cols[t * self.stride + i]
        }
        #[inline]
        fn dot_rows(&self, i: usize, j: usize) -> f64 {
            (0..self.dim).map(|t| self.at(i, t) * self.at(j, t)).sum()
        }
    }

    /// Workspace (reused between solves).
    #[derive(Default)]
    pub struct Daqp {
        /// working set (indices of rows, in insertion order)
        pub w: Vec<usize>,
        in_w: Vec<bool>,
        /// dual variable of every row
        pub lam: Vec<f64>,
        /// L (unit lower triangular) and D of M_W M_W' = L D L', row stride `cap`
        l: Vec<f64>,
        dg: Vec<f64>,
        cap: usize,
        y: Vec<f64>,
        p: Vec<f64>,
        ls: Vec<f64>,
        mu: Vec<f64>,
        tail: Vec<usize>,
        /// the primal point u = -M_W' λ at the end of a feasible solve
        pub x: Vec<f64>,
        pub iters: usize,
    }

    impl Daqp {
        pub fn new() -> Daqp {
            Daqp::default()
        }

        /// Adds row `r` to the factorization of `M_W M_W'` (Lemma 3 of the DAQP paper); returns
        /// delta (the new diagonal entry of D).
        fn ldl_add(&mut self, a: &Rows, r: usize) -> f64 {
            let k = self.w.len(); // before the push
            let cap = self.cap;
            let mut delta = a.dot_rows(r, r);
            for i in 0..k {
                let wi = self.w[i];
                let mut s = a.dot_rows(wi, r);
                for j in 0..i {
                    s -= self.l[i * cap + j] * self.dg[j] * self.l[k * cap + j];
                }
                let lki = s / self.dg[i];
                self.l[k * cap + i] = lki;
                delta -= lki * lki * self.dg[i];
            }
            self.l[k * cap + k] = 1.0;
            self.dg[k] = delta;
            self.w.push(r);
            delta
        }

        /// Rebuilds the factorization for the current working set (after a removal).
        /// Removes working-set position `j`: the factors of the elements before `j` are unchanged
        /// (L and D of a leading block do not depend on later rows), the later ones are re-added.
        fn remove_at(&mut self, a: &Rows, j: usize) {
            self.tail.clear();
            self.tail.extend_from_slice(&self.w[j + 1..]);
            self.w.truncate(j);
            for t in 0..self.tail.len() {
                let r = self.tail[t];
                self.ldl_add(a, r);
            }
        }

        /// After `Status::Infeasible`: the working set and the non-negative multipliers `p` over it of the
        /// infeasibility certificate (`sum p_i a_i` is zero and `sum p_i b_i` is negative in exact arithmetic;
        /// the caller can check both in floating point).
        pub fn certificate(&self) -> (&[usize], &[f64]) {
            (&self.w, &self.p)
        }

        /// Dual active-set method (Algorithm 1 of the DAQP paper) for the feasibility of
        /// `a x <= b`, started from W = {equality rows}, λ = 0.
        pub fn solve(&mut self, a: &Rows, n_eq: usize) -> Status {
            let n = a.n;
            self.cap = a.dim + n_eq + 2;
            let cap = self.cap;
            self.l.clear();
            self.l.resize(cap * cap, 0.0);
            self.dg.clear();
            self.dg.resize(cap, 0.0);
            self.w.clear();
            self.in_w.clear();
            self.in_w.resize(n, false);
            self.lam.clear();
            self.lam.resize(n, 0.0);
            self.iters = 0;
            for r in 0..n_eq {
                let dl = self.ldl_add(a, r);
                self.in_w[r] = true;
                if dl <= ZERO_TOL {
                    return Status::Unknown;
                }
            }
            self.run(a, n_eq)
        }

        /// Continues from the state of the last feasible `solve`/`resume` after rows were appended
        /// (`a.n` larger, the first rows unchanged): the old working set and λ are a valid warm start
        /// (λ >= 0 supported on W).
        pub fn resume(&mut self, a: &Rows, n_eq: usize) -> Status {
            self.in_w.resize(a.n, false);
            self.lam.resize(a.n, 0.0);
            self.iters = 0;
            self.run(a, n_eq)
        }

        fn run(&mut self, a: &Rows, n_eq: usize) -> Status {
            let n = a.n;
            let dim = a.dim;
            let cap = self.cap;
            let b = a.b;
            let max_iter = 100 + 20 * (n + dim);
            loop {
                self.iters += 1;
                if self.iters > max_iter {
                    return Status::Unknown;
                }
                let k = self.w.len();
                let singular = k > 0 && (k > dim || self.dg[k - 1] <= ZERO_TOL);
                if !singular {
                    // lambda* from L D L' lam = -d_W
                    self.y.clear();
                    for i in 0..k {
                        let mut s = -b[self.w[i]];
                        for j in 0..i {
                            s -= self.l[i * cap + j] * self.y[j];
                        }
                        self.y.push(s);
                    }
                    for i in 0..k {
                        self.y[i] /= self.dg[i];
                    }
                    for i in (0..k).rev() {
                        let mut s = self.y[i];
                        for j in i + 1..k {
                            s -= self.l[j * cap + i] * self.y[j];
                        }
                        self.y[i] = s;
                    }
                    if self.y.iter().any(|v| !v.is_finite()) {
                        return Status::Unknown;
                    }
                    let nonneg = (n_eq..k).all(|i| self.y[i] >= -DUAL_TOL);
                    if nonneg {
                        for i in 0..k {
                            self.lam[self.w[i]] = self.y[i];
                        }
                        // u = -M_W' lam, mu_j = b_j - a_j u for every row (rows of W have mu = 0)
                        self.x.clear();
                        self.x.resize(dim, 0.0);
                        for i in 0..k {
                            let wi = self.w[i];
                            let li = self.y[i];
                            for t in 0..dim {
                                self.x[t] -= li * a.at(wi, t);
                            }
                        }
                        self.mu.clear();
                        self.mu.extend_from_slice(&b[..n]);
                        for t in 0..dim {
                            let xt = self.x[t];
                            for (m, c) in self.mu.iter_mut().zip(&a.cols[t * a.stride..t * a.stride + n]) {
                                *m -= xt * c;
                            }
                        }
                        let worst = self.mu.iter().fold(f64::INFINITY, |m, &v| m.min(v));
                        if worst >= -PRIMAL_TOL {
                            return Status::Feasible;
                        }
                        let jw = self.mu.iter().position(|&v| v == worst).unwrap();
                        if k >= cap - 1 || self.in_w[jw] {
                            return Status::Unknown;
                        }
                        self.ldl_add(a, jw);
                        self.in_w[jw] = true;
                        self.lam[jw] = 0.0;
                        continue;
                    }
                    // fix a component: p = lam* - lam, B = {i : lam*_i < 0}
                    let mut best = f64::INFINITY;
                    let mut jb = usize::MAX;
                    for i in n_eq..k {
                        if self.y[i] < -DUAL_TOL {
                            let p = self.y[i] - self.lam[self.w[i]];
                            if p < 0.0 {
                                let ratio = -self.lam[self.w[i]] / p;
                                if ratio < best {
                                    best = ratio;
                                    jb = i;
                                }
                            }
                        }
                    }
                    if jb == usize::MAX {
                        return Status::Unknown;
                    }
                    for i in 0..k {
                        let wi = self.w[i];
                        self.lam[wi] += best * (self.y[i] - self.lam[wi]);
                    }
                    let wj = self.w[jb];
                    self.lam[wj] = 0.0;
                    self.in_w[wj] = false;
                    self.remove_at(a, jb);
                } else {
                    // singular: null vector p of M_W M_W' from the last row of L (Lemma 2)
                    let last = k - 1;
                    self.p.clear();
                    self.p.resize(k, 0.0);
                    self.p[last] = 1.0;
                    self.ls.clear();
                    self.ls.extend((0..last).map(|j| self.l[last * cap + j]));
                    for i in (0..last).rev() {
                        let mut s = -self.ls[i];
                        for j in i + 1..last {
                            s -= self.l[j * cap + i] * self.p[j];
                        }
                        self.p[i] = s;
                    }
                    let mut dp = 0.0;
                    for i in 0..k {
                        dp += b[self.w[i]] * self.p[i];
                    }
                    if dp.abs() < 1e-13 {
                        return Status::Unknown;
                    }
                    if dp > 0.0 {
                        for v in self.p.iter_mut() {
                            *v = -*v;
                        }
                    }
                    let mut best = f64::INFINITY;
                    let mut jb = usize::MAX;
                    for i in n_eq..k {
                        let p = self.p[i];
                        if p < -DUAL_TOL {
                            let ratio = -self.lam[self.w[i]] / p;
                            if ratio < best {
                                best = ratio;
                                jb = i;
                            }
                        }
                    }
                    if jb == usize::MAX {
                        return Status::Infeasible;
                    }
                    for i in 0..k {
                        let wi = self.w[i];
                        self.lam[wi] += best * self.p[i];
                    }
                    let wj = self.w[jb];
                    self.lam[wj] = 0.0;
                    self.in_w[wj] = false;
                    self.remove_at(a, jb);
                }
            }
        }
    }
}

use daqp::{Daqp, Rows, Status};

/// The mpLDP data (eqs. 2 to 4).
struct Ldp {
    m: usize,
    nx: usize,
    nt: usize,
    neq: usize,
    /// M = A R^{-1}, m x nx
    mm: Vec<f64>,
    /// G = M M', m x m (symmetric)
    g: Vec<f64>,
    /// [D | d0] with d(θ) = Dθ + d0, by columns: dd_cols[t * m + i], t = 0..=nt
    dd_cols: Vec<f64>,
    /// the same by rows, m x (nt + 1)
    dd: Vec<f64>,
    /// parameter-space rows by columns (stride n_th), normalized, with rhs
    th_cols: Vec<f64>,
    th_rhs: Vec<f64>,
    /// rows (numerically) parallel to row i, as a bit mask: a set containing both is LICQ-violating
    parallel: Vec<u128>,
    /// bounding box of Θ0
    bbox: Option<(Vec<f64>, Vec<f64>)>,
}

impl Ldp {
    fn new(p: &MpProgram) -> Option<Ldp> {
        let q = p.q.as_ref()?;
        let m = p.num_constraints();
        let nx = p.num_x();
        let nt = p.num_t();
        let ch = Cholesky::new(q)?; // Q = L L' = R'R with R = L'
        let l = &ch.l;
        // M = A R^{-1}: each row solves L m' = a'
        let mut mm = vec![0.0; m * nx];
        for i in 0..m {
            let ai = p.a.row(i);
            let row = &mut mm[i * nx..(i + 1) * nx];
            for r in 0..nx {
                let mut s = ai[r];
                for c in 0..r {
                    s -= l[r * nx + c] * row[c];
                }
                row[r] = s / l[r * nx + r];
            }
        }
        // v = R^{-T} f(θ) = L^{-1} (Hθ + c): columns [H | c]
        let mut v = vec![0.0; nx * (nt + 1)];
        for r in 0..nx {
            for t in 0..=nt {
                let mut s = if t < nt { p.h[(r, t)] } else { p.c[r] };
                for c in 0..r {
                    s -= l[r * nx + c] * v[c * (nt + 1) + t];
                }
                v[r * (nt + 1) + t] = s / l[r * nx + r];
            }
        }
        let mut g = vec![0.0; m * m];
        for i in 0..m {
            for j in 0..=i {
                let s = dot(&mm[i * nx..(i + 1) * nx], &mm[j * nx..(j + 1) * nx]);
                g[i * m + j] = s;
                g[j * m + i] = s;
            }
        }
        let mut dd = vec![0.0; m * (nt + 1)];
        let mut dd_cols = vec![0.0; m * (nt + 1)];
        for i in 0..m {
            let mi = &mm[i * nx..(i + 1) * nx];
            for t in 0..=nt {
                let mut s = if t < nt { p.f[(i, t)] } else { p.b[i] };
                for r in 0..nx {
                    s += mi[r] * v[r * (nt + 1) + t];
                }
                dd[i * (nt + 1) + t] = s;
                dd_cols[t * m + i] = s;
            }
        }
        let mut th_rows: Vec<Vec<f64>> = vec![];
        let mut th_rhs = vec![];
        for i in 0..p.a_t.rows {
            let r = p.a_t.row(i);
            let n = norm(r);
            if n > 1e-12 {
                th_rows.push(r.iter().map(|x| x / n).collect());
                th_rhs.push(p.b_t[i] / n);
            }
        }
        let nth = th_rhs.len();
        let mut th_cols = vec![0.0; nth * nt];
        for (i, r) in th_rows.iter().enumerate() {
            for t in 0..nt {
                th_cols[t * nth + i] = r[t];
            }
        }
        // a pair of parallel rows makes G_AA singular: the pivot of the second row of the pair is
        // G_jj - G_ij^2 / G_ii, below the tolerance of `ldl_gram` whatever else is in the set
        let mut parallel = vec![0u128; m];
        if m <= 128 {
            for i in 0..m {
                for j in 0..m {
                    if i != j {
                        let (gii, gjj, gij) = (g[i * m + i], g[j * m + j], g[i * m + j]);
                        if gjj - gij * gij / gii <= 1e-11 * gii.max(gjj) && gii - gij * gij / gjj <= 1e-11 * gii.max(gjj) {
                            parallel[i] |= 1u128 << j;
                        }
                    }
                }
            }
        }
        Some(Ldp { m, nx, nt, neq: p.num_equality_constraints(), mm, g, dd_cols, dd, th_cols, th_rhs, parallel, bbox: p.theta_bbox().cloned() })
    }
}

/// Outcome of the test of one active set.
enum Test {
    /// LICQ violated
    Licq,
    /// LICQ holds, region empty
    Empty,
    /// LICQ holds, region nonempty: the multiplier law [Λ | λ0] (k x (nt + 1))
    Region,
}

/// Scratch buffers of the test (reused between candidates).
struct Work {
    daqp: Daqp,
    /// LDL' of G_AA, stride k
    l: Vec<f64>,
    dg: Vec<f64>,
    dgi: Vec<f64>,
    /// [Λ | λ0], k x (nt + 1)
    x: Vec<f64>,
    /// rows by columns with stride `n_total`: λ rows, θ rows, then the m μ rows
    cols: Vec<f64>,
    rhs: Vec<f64>,
    inv: Vec<f64>,
}

impl Work {
    fn new() -> Work {
        Work { daqp: Daqp::new(), l: vec![], dg: vec![], dgi: vec![], x: vec![], cols: vec![], rhs: vec![], inv: vec![] }
    }
}

/// LDL' of G_AA; false if singular (relative pivot tolerance as the Cholesky rank certificate of
/// the other algorithms).
fn ldl_gram(ldp: &Ldp, a: &[usize], w: &mut Work) -> bool {
    let k = a.len();
    w.l.clear();
    w.l.resize(k * k, 0.0);
    w.dg.clear();
    w.dg.resize(k, 0.0);
    w.dgi.resize(k, 0.0);
    let mut smax: f64 = 1e-300;
    for &i in a {
        smax = smax.max(ldp.g[i * ldp.m + i]);
    }
    for i in 0..k {
        let mut d = ldp.g[a[i] * ldp.m + a[i]];
        for j in 0..i {
            let mut s = ldp.g[a[i] * ldp.m + a[j]];
            for c in 0..j {
                s -= w.l[i * k + c] * w.dg[c] * w.l[j * k + c];
            }
            let lij = s * w.dgi[j];
            w.l[i * k + j] = lij;
            d -= lij * lij * w.dg[j];
        }
        if !(d > 1e-10 * smax) {
            return false;
        }
        w.l[i * k + i] = 1.0;
        w.dg[i] = d;
        w.dgi[i] = 1.0 / d;
    }
    true
}

/// Section II-B and Algorithm 1 steps 4 to 8 for the active set `a` (sorted). On `Test::Region`
/// the law [Λ | λ0] is left in `w.x`.
fn test_set(ldp: &Ldp, key: u128, a: &[usize], w: &mut Work) -> Test {
    let k = a.len();
    let nt = ldp.nt;
    let nc = nt + 1;
    let neq = ldp.neq;
    let m = ldp.m;
    #[cfg(feature = "lp_census")]
    let t_ldl = std::time::Instant::now();
    if k > ldp.nx || a.iter().any(|&i| ldp.parallel[i] & key != 0) || !ldl_gram(ldp, a, w) {
        return Test::Licq;
    }
    #[cfg(feature = "lp_census")]
    crate::census::bump(crate::census::Counter::PdLdlNanos, t_ldl.elapsed().as_nanos() as u64);
    // [Λ | λ0] = -G_AA^{-1} [D_A | d0_A]
    w.x.clear();
    w.x.resize(k * nc, 0.0);
    for (ii, &i) in a.iter().enumerate() {
        for t in 0..nc {
            w.x[ii * nc + t] = -ldp.dd[i * nc + t];
        }
    }
    for i in 0..k {
        for j in 0..i {
            let lij = w.l[i * k + j];
            if lij != 0.0 {
                let (head, tail) = w.x.split_at_mut(i * nc);
                for (o, s) in tail[..nc].iter_mut().zip(&head[j * nc..(j + 1) * nc]) {
                    *o -= lij * s;
                }
            }
        }
    }
    for i in 0..k {
        let inv = w.dgi[i];
        for v in &mut w.x[i * nc..(i + 1) * nc] {
            *v *= inv;
        }
    }
    for i in (0..k).rev() {
        for j in i + 1..k {
            let lji = w.l[j * k + i];
            if lji != 0.0 {
                let (head, tail) = w.x.split_at_mut(j * nc);
                for (o, s) in head[i * nc..(i + 1) * nc].iter_mut().zip(&tail[..nc]) {
                    *o -= lji * s;
                }
            }
        }
    }
    // rows: [λ >= 0 (non-equality multipliers) | θ in Θ0 | μ >= 0 for the m constraints]
    let nl = k - neq;
    let nth = ldp.th_rhs.len();
    let n1 = nl + nth;
    let n = n1 + m;
    w.cols.clear();
    w.cols.resize(nt * n, 0.0);
    w.rhs.clear();
    w.rhs.resize(n, 0.0);
    for ii in 0..nl {
        let row = &w.x[(neq + ii) * nc..(neq + ii + 1) * nc];
        let nrm = norm(&row[..nt]);
        if nrm > 1e-12 {
            // λ_i = row[..nt] θ + row[nt] over the box of Θ0: a maximum below zero proves emptiness
            // (the same conclusion DAQP reaches, found without iterating)
            if let Some((lo, hi)) = &ldp.bbox {
                let mut mx = row[nt];
                for t in 0..nt {
                    mx += (row[t] * lo[t]).max(row[t] * hi[t]);
                }
                if mx < -10.0 * daqp::PRIMAL_TOL * nrm {
                    crate::census_bump!(PdEarlyEmpty, 1);
                    return Test::Empty;
                }
            }
            for t in 0..nt {
                w.cols[t * n + ii] = -row[t] / nrm;
            }
            w.rhs[ii] = row[nt] / nrm;
        } else if row[nt] < -daqp::PRIMAL_TOL {
            return Test::Empty;
        } else {
            w.rhs[ii] = 1.0;
        }
    }
    for t in 0..nt {
        w.cols[t * n + nl..t * n + n1].copy_from_slice(&ldp.th_cols[t * nth..(t + 1) * nth]);
    }
    w.rhs[nl..n1].copy_from_slice(&ldp.th_rhs);
    // stage 1: λ and θ rows only
    let rows1 = Rows { cols: &w.cols, stride: n, b: &w.rhs, n: n1, dim: nt };
    #[cfg(feature = "lp_census")]
    let t_d1 = std::time::Instant::now();
    let st1 = w.daqp.solve(&rows1, 0);
    crate::census_bump!(PdDaqp1, 1);
    #[cfg(feature = "lp_census")]
    crate::census::bump(crate::census::Counter::PdDaqpNanos, t_d1.elapsed().as_nanos() as u64);
    if st1 == Status::Infeasible {
        return Test::Empty;
    }
    // stage 2: μ_j = d_j + G_jA λ_A for all j (rows of A are made trivial), normalized
    {
        let (cols, rhs) = (&mut w.cols, &mut w.rhs);
        for t in 0..nt {
            let dst = &mut cols[t * n + n1..(t + 1) * n];
            for (o, s) in dst.iter_mut().zip(&ldp.dd_cols[t * m..(t + 1) * m]) {
                *o = -s;
            }
        }
        for (o, s) in rhs[n1..].iter_mut().zip(&ldp.dd_cols[nt * m..(nt + 1) * m]) {
            *o = *s;
        }
        for (ii, &i) in a.iter().enumerate() {
            let grow = &ldp.g[i * m..(i + 1) * m];
            let xr = &w.x[ii * nc..(ii + 1) * nc];
            for t in 0..nt {
                let c = -xr[t];
                for (o, g) in cols[t * n + n1..(t + 1) * n].iter_mut().zip(grow) {
                    *o += c * g;
                }
            }
            let c = xr[nt];
            for (o, g) in rhs[n1..].iter_mut().zip(grow) {
                *o += c * g;
            }
        }
        // normalize; rows with a zero normal are dropped (rhs >= 0) or prove emptiness
        w.inv.clear();
        w.inv.resize(m, 0.0);
        for t in 0..nt {
            for (s, c) in w.inv.iter_mut().zip(&cols[t * n + n1..(t + 1) * n]) {
                *s += c * c;
            }
        }
        let mut bad = false;
        for (s, r) in w.inv.iter_mut().zip(&rhs[n1..]) {
            let ok = *s > 1e-24;
            bad |= !ok & (*r < -daqp::PRIMAL_TOL);
            *s = if ok { 1.0 / s.sqrt() } else { 0.0 };
        }
        if bad {
            return Test::Empty;
        }
        for t in 0..nt {
            for (c, s) in cols[t * n + n1..(t + 1) * n].iter_mut().zip(&w.inv) {
                *c *= s;
            }
        }
        for (r, s) in rhs[n1..].iter_mut().zip(&w.inv) {
            *r *= s;
        }
        for &i in a {
            for t in 0..nt {
                cols[t * n + n1 + i] = 0.0;
            }
            rhs[n1 + i] = 0.0;
        }
    }
    let rows = Rows { cols: &w.cols, stride: n, b: &w.rhs, n, dim: nt };
    #[cfg(feature = "lp_census")]
    let t_d2 = std::time::Instant::now();
    let st = if st1 == Status::Feasible { w.daqp.resume(&rows, 0) } else { w.daqp.solve(&rows, 0) };
    crate::census_bump!(PdDaqp2, 1);
    #[cfg(feature = "lp_census")]
    crate::census::bump(crate::census::Counter::PdDaqpNanos, t_d2.elapsed().as_nanos() as u64);
    match st {
        Status::Feasible => Test::Region,
        Status::Infeasible => Test::Empty,
        Status::Unknown => {
            // numerical breakdown of DAQP: decide with the general LP
            let mut e = Mat::zeros(n, nt);
            for i in 0..n {
                for t in 0..nt {
                    e[(i, t)] = w.cols[t * n + i];
                }
            }
            crate::census_bump!(PdLpFallback, 1);
            if crate::solver::lp::lp_feasible(&e, &w.rhs, &[]) { Test::Region } else { Test::Empty }
        }
    }
}

/// Remark 6: A0 from DAQP on the mpLDP at a feasible parameter.
fn initial_set(program: &MpProgram, ldp: &Ldp) -> Option<Vec<usize>> {
    let theta = program.feasible_theta_point()?;
    let (m, nx, nt, neq) = (ldp.m, ldp.nx, ldp.nt, ldp.neq);
    // the equality constraints are the first rows
    let mut cols = vec![0.0; m * nx];
    let mut rhs = vec![0.0; m];
    for i in 0..m {
        let mi = &ldp.mm[i * nx..(i + 1) * nx];
        let n = norm(mi).max(1e-300);
        for t in 0..nx {
            cols[t * m + i] = mi[t] / n;
        }
        let d = dot(&ldp.dd[i * (nt + 1)..i * (nt + 1) + nt], &theta) + ldp.dd[i * (nt + 1) + nt];
        rhs[i] = d / n;
    }
    let mut d = Daqp::new();
    let rows = Rows { cols: &cols, stride: m, b: &rhs, n: m, dim: nx };
    if d.solve(&rows, neq) != Status::Feasible {
        return None;
    }
    let mut a = d.w.clone();
    a.sort_unstable();
    Some(a)
}

pub fn pdaqp(program: &MpProgram) -> Vec<CriticalRegion> {
    let opts = super::SolveOptions::default();
    let m = program.num_constraints();
    if !program.is_qp() || m > 128 {
        return super::combinatorial_graph(program, &opts, false);
    }
    let Some(ldp) = Ldp::new(program) else {
        return super::combinatorial_graph(program, &opts, false);
    };
    let Some(a0) = initial_set(program, &ldp) else {
        return super::combinatorial_graph(program, &opts, false);
    };
    let to_bits = |a: &[usize]| a.iter().fold(0u128, |k, &i| k | (1u128 << i));
    let eq_mask = to_bits(&program.equality_indices);
    let full: u128 = if m == 128 { u128::MAX } else { (1u128 << m) - 1 };
    let mut explored: HashSet<u128> = HashSet::default();
    let mut queue: VecDeque<u128> = VecDeque::new();
    let k0 = to_bits(&a0);
    explored.insert(k0);
    queue.push_back(k0);
    let mut regions = vec![];
    let mut work = Work::new();
    let mut a: Vec<usize> = Vec::with_capacity(m);
    let push = |k: u128, queue: &mut VecDeque<u128>, explored: &mut HashSet<u128>| {
        if explored.insert(k) {
            queue.push_back(k);
        }
    };
    while let Some(k) = queue.pop_front() {
        a.clear();
        let mut r = k;
        while r != 0 {
            a.push(r.trailing_zeros() as usize);
            r &= r - 1;
        }
        #[cfg(feature = "lp_census")]
        let t_test = std::time::Instant::now();
        let outcome = test_set(&ldp, k, &a, &mut work);
        #[cfg(feature = "lp_census")]
        crate::census::bump(crate::census::Counter::PdTestNanos, t_test.elapsed().as_nanos() as u64);
        crate::census_bump!(PdTests, 1);
        let (sup, sub) = match outcome {
            Test::Licq => {
                crate::census_bump!(PdLicq, 1);
                (false, true)
            }
            Test::Empty => {
                crate::census_bump!(PdEmpty, 1);
                (false, false)
            }
            Test::Region => {
                crate::census_bump!(PdRegion, 1);
                #[cfg(feature = "lp_census")]
                let t_build = std::time::Instant::now();
                let nc = ldp.nt + 1;
                let x = &work.x;
                let law = ControlLaw {
                    a_x: Mat::zeros(0, 0),
                    b_x: vec![],
                    a_l: Mat::from_vec(a.len(), ldp.nt, (0..a.len()).flat_map(|i| x[i * nc..i * nc + ldp.nt].to_vec()).collect()),
                    b_l: (0..a.len()).map(|i| x[i * nc + ldp.nt]).collect(),
                };
                let kkt = kkt_region_from_law(program, &a, law);
                if let Some(cr) = gen_cr_from_kkt(program, &a, kkt, true) {
                    regions.push(cr);
                }
                #[cfg(feature = "lp_census")]
                crate::census::bump(crate::census::Counter::PdBuildNanos, t_build.elapsed().as_nanos() as u64);
                (true, true)
            }
        };
        if sub {
            let mut r = k & !eq_mask;
            while r != 0 {
                let i = r.trailing_zeros();
                push(k & !(1u128 << i), &mut queue, &mut explored);
                r &= r - 1;
            }
        }
        if sup {
            let mut r = full & !k;
            while r != 0 {
                let i = r.trailing_zeros();
                push(k | (1u128 << i), &mut queue, &mut explored);
                r &= r - 1;
            }
        }
    }
    regions
}
