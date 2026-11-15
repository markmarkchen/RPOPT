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
//! are [constant | θ coefficients] (the perturbation block β is implied by T, see `pivot_facet`). Variables: w_i = i, z_i = n + i, the facet parameter α = 2n.

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
    /// position in `ineq` of each original constraint (usize::MAX for equalities)
    pos: Vec<usize>,
    /// X = M_EE^{-1} [M_EI | q_E | R_E], ne x (n + 1 + nt) (empty without equalities)
    xe: Mat,
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
        let mut xe = Mat::zeros(0, 0);
        let mut pos = vec![usize::MAX; mfull];
        for (a, &i) in ineq.iter().enumerate() {
            pos[i] = a;
        }
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
            xe = x;
        }
        Some(Plcp { n, nt, m, q, r, ineq, eq, pos, xe })
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

    /// The tableau after the principal pivot (r, c), written into new buffers in one pass (the
    /// same operations as `clone` followed by `pivot`, without the copy of the untouched rows).
    /// `self` must have no pending pivot.
    fn pivoted(&self, r: usize, c: usize) -> Tab {
        debug_assert!(self.pend.is_none());
        let (n, nc, w) = (self.n, self.nc, self.w);
        let p = self.t[r * nc + c];
        let inv = 1.0 / p;
        let fcol: Vec<f64> = (0..n).map(|i| self.t[i * nc + c]).collect();
        let trow: Vec<f64> = (0..nc).map(|k| if k == c { inv } else { -self.t[r * nc + k] * inv }).collect();
        let rrow: Vec<f64> = (0..w).map(|k| self.rho[r * w + k] * -inv).collect();
        let tc = trow[c];
        let mut t = Vec::with_capacity(n * nc);
        let mut rho = Vec::with_capacity(n * w);
        for i in 0..n {
            let si = &self.t[i * nc..(i + 1) * nc];
            let sr = &self.rho[i * w..(i + 1) * w];
            if i == r {
                t.extend_from_slice(&trow);
                rho.extend_from_slice(&rrow);
                continue;
            }
            let f = fcol[i];
            if f == 0.0 {
                t.extend_from_slice(si);
                rho.extend_from_slice(sr);
                continue;
            }
            let start = t.len();
            t.extend(si.iter().zip(trow.iter()).map(|(o, s)| o + f * s));
            t[start + c] = f * tc;
            rho.extend(sr.iter().zip(rrow.iter()).map(|(o, s)| o + f * s));
        }
        let mut row_var = self.row_var.clone();
        let mut col_var = self.col_var.clone();
        std::mem::swap(&mut row_var[r], &mut col_var[c]);
        Tab { n, nc, w, t, rho, row_var, col_var, pend: None }
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
    let w = 1 + nt;
    let mut rho = vec![0.0; n * w];
    for i in 0..n {
        rho[i * w] = p.q[i];
        rho[i * w + 1..i * w + 1 + nt].copy_from_slice(&p.r[i * nt..(i + 1) * nt]);
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
/// Pivot tolerance relative to the largest entry of the entering column (see `pivot_facet`).
fn prel() -> f64 {
    static V: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *V.get_or_init(|| std::env::var("PREL").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0))
}

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

/// Is `v` lexico-nonnegative when entries are judged relative to its largest entry? Used where
/// `v` is a difference of rows scaled by 1/a (amplified by up to 1/PTOL), so an absolute `ZTOL`
/// would call a genuine sign noise or a genuine value zero.
fn lex_nonneg_rel(v: &[f64]) -> bool {
    let vm = v.iter().fold(0.0f64, |m, x| m.max(x.abs()));
    v.iter().find(|x| x.abs() > ZTOL * vm).map_or(true, |x| *x > 0.0)
}

struct Facet<'a> {
    /// facet-space rows of the polytope F (E_f θ_f <= f_f)
    ef: &'a Mat,
    ff: &'a [f64],
    /// per-variable magnitude of the row (set in `adjacent`, where the rows identically zero on
    /// the facet are found with a tolerance relative to it); `in_z` uses the same tolerance
    sc: &'a [f64],
}

/// Algorithm 2 (`pivot`): from the ACLFB `tab` (α basic or entering as column `e`) returns the
/// z-row sets of the adjacent bases.
fn pivot_facet(p: &Plcp, tab: &mut Tab, e: usize, fpoly: Facet, depth: usize, out: &mut Vec<Vec<usize>>) {
    let n = p.n;
    let d1 = p.nt - 1;
    let alpha = 2 * n;
    let mut e = e;
    crate::census_bump!(PfCalls, 1);
    match depth {
        0 => {
            crate::census_bump!(PfDepth0, 1);
        }
        1 => {
            crate::census_bump!(PfDepth1, 1);
        }
        2 => {
            crate::census_bump!(PfDepth2, 1);
        }
        3 => {
            crate::census_bump!(PfDepth3, 1);
        }
        4 => {
            crate::census_bump!(PfDepth4, 1);
        }
        5 => {
            crate::census_bump!(PfDepth5, 1);
        }
        6 => {
            crate::census_bump!(PfDepth6, 1);
        }
        _ => {
            crate::census_bump!(PfDepth7, 1);
        }
    }
    for _ in 0..(8 * n + 50) {
        // P: rows decreasing in the entering variable
        let a: Vec<f64> = (0..n).map(|i| -tab.tv(i, e)).collect();
        let w = tab.w;
        // the perturbation block β = ∂x_B/∂ε of the tableau is not stored: column j of β is
        // -T[:, c] when w_j is nonbasic in column c (its column of the pivoted system is e_j), and
        // the unit vector of its row when w_j is basic (bitwise what the stored block would hold)
        let mut wcol = vec![usize::MAX; n];
        for (c, &v) in tab.col_var.iter().enumerate() {
            if v < n {
                wcol[v] = c;
            }
        }
        let mut wrow = vec![usize::MAX; n];
        for (r, &v) in tab.row_var.iter().enumerate() {
            if v < n {
                wrow[v] = r;
            }
        }
        let rfull = |i: usize, k: usize| -> f64 {
            if k < w {
                tab.rv(i, k)
            } else {
                let j = k - w;
                if wcol[j] != usize::MAX {
                    -tab.tv(i, wcol[j])
                } else if wrow[j] == i {
                    1.0
                } else {
                    0.0
                }
            }
        };
        let rowmax = |i: usize| (0..1 + d1).fold(0.0f64, |m, k| m.max(tab.rv(i, k).abs()));
        // a candidate leaving row must decrease by more than rounding noise relative to the column:
        // with entries of 1e2 to 1e3 the noise is far above the absolute PTOL, and pivoting on it
        // (pop_011: a = 1.6e-10 in a column of 1.5e2) gave bases with multipliers of 1e13 whose regions
        // overlapped the true ones
        let amax = (0..n).fold(0.0f64, |m, i| m.max(a[i].abs()));
        let pthr = PTOL.max(prel() * amax);
        let pset: Vec<usize> = (0..n).filter(|&i| a[i] > pthr).collect();
        let in_z = |i: usize| rowmax(i) <= ZTOL * fpoly.sc[tab.row_var[i]];
        let pz: Vec<usize> = pset.iter().cloned().filter(|&i| in_z(i)).collect();
        let alpha_row = tab.row_var.iter().position(|&v| v == alpha);
        // No row that vanishes on the facet blocks the entering variable and α rises: the entering
        // variable replaces α, which is the neighbour. This holds also when P is empty (nothing
        // decreases at all). Returning there, as before, lost the neighbour whenever the previous
        // pivot had exchanged a dependent pair (pop_081: parallel rows of a one-variable program).
        if pz.is_empty() {
            if let Some(ra) = alpha_row {
                if tab.tv(ra, e) > pthr {
                    out.push(z_after(tab, n, ra, e));
                    return;
                }
            }
        }
        if pset.is_empty() {
            return;
        }
        if !pz.is_empty() {
            // Case 1: leaving variable independent of θ_f
            crate::census_bump!(PfCase1, 1);
            let beta = |i: usize| -> Vec<f64> { (w..w + n).map(|k| rfull(i, k)).collect() };
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
        crate::census_bump!(PfCase2, 1);
        if depth > 6 {
            crate::census_bump!(PfDepthLimit, 1);
            return;
        }
        for &l in &pset {
            crate::census_bump!(PfBranches, 1);
            let leaving = tab.row_var[l];
            // lexico-inequalities (16): Γ_j = a_l row_j - a_j row_l  ⪰ 0 for all j in P. Γ_j scales
            // with a_l a_j (1e-20 for rows with a near PTOL), so the tolerances are applied to the
            // scale-free Γ_j / (a_l a_j) = row_j / a_j - row_l / a_l instead.
            let mut ok = true;
            let mut ef = fpoly.ef.clone();
            let mut ff = fpoly.ff.to_vec();
            for &j in &pset {
                if j == l {
                    continue;
                }
                let g: Vec<f64> = (0..w + n).map(|k| rfull(j, k) / a[j] - rfull(l, k) / a[l]).collect();
                let gsc = 1.0 + (0..1 + d1).fold(0.0f64, |m, k| m.max((rfull(j, k) / a[j]).abs()).max((rfull(l, k) / a[l]).abs()));
                let gq_zero = g[1..1 + d1].iter().all(|v| v.abs() <= ZTOL * gsc);
                if gq_zero {
                    let mut v = vec![g[0]];
                    v.extend_from_slice(&g[1 + d1..]);
                    if !lex_nonneg_rel(&v) {
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
                crate::census_bump!(PfLexCut, 1);
                continue;
            }
            if d1 > 0 {
                crate::census_bump!(PfCheb, 1);
                let ball = {
                    crate::census_purpose!(Neighbour);
                    chebyshev_ball(&ef, &ff, &[])
                };
                match ball {
                    Some(s) if s.x[d1] > 1e-9 => {}
                    _ => {
                        crate::census_bump!(PfChebEmpty, 1);
                        continue;
                    }
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
            #[cfg(feature = "lp_census")]
            {
                let mut key = next.row_var.clone();
                key.sort_unstable();
                if crate::census::basis_seen(key) {
                    crate::census_bump!(PfRepeatBasis, 1);
                }
            }
            pivot_facet(p, &mut next, ne, Facet { ef: &ef, ff: &ff, sc: fpoly.sc }, depth + 1, out);
        }
        return;
    }
    crate::census_bump!(PfIterLimit, 1);
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
///
/// `cert` is asked only when the fast path has failed, that is, when the slow path (Case 2) would
/// run: if it names a known region R that is the only one across the facet, the search is skipped
/// (unless `audit`, which runs it anyway). The third result is that region.
fn adjacent(p: &Plcp, tab: &Tab, erow: &[f64], frhs: f64, e_all: &Mat, f_all: &[f64], ws: &mut Option<Tab>, cert: Option<&dyn Fn() -> Option<usize>>, audit: bool) -> (Vec<Nbr>, Option<usize>) {
    #[cfg(feature = "lp_census")]
    let _cen_t = std::time::Instant::now();
    crate::census_bump!(AdjCalls, 1);
    let (n, nt) = (p.n, p.nt);
    let nrm = erow.iter().map(|v| v * v).sum::<f64>().sqrt();
    let g: Vec<f64> = erow.iter().map(|v| v / nrm).collect();
    let b = -frhs / nrm; // γ'θ + b = 0
    let d1 = nt - 1;
    // rows identically zero on the facet (none: boundary of Θ), and the α column γ_i = ρθ_i · g
    let mut gam = vec![0.0; n];
    let mut zrows: Vec<usize> = vec![];
    let mut rowsc = vec![1.0f64; 2 * n + 1];
    for i in 0..n {
        let rr = &tab.rho[i * tab.w..(i + 1) * tab.w];
        let gi: f64 = (0..nt).map(|k| rr[1 + k] * g[k]).sum();
        gam[i] = gi;
        // relative to the row's magnitude: rows with gradients of 1e6 carry rounding errors above
        // the absolute tolerance, and an identically-zero row would then be missed (no neighbour)
        let sc = 1.0f64.max(rr[0].abs()).max((b * gi).abs()).max((0..nt).map(|k| rr[1 + k] * rr[1 + k]).sum::<f64>().sqrt());
        rowsc[tab.row_var[i]] = sc;
        if (rr[0] - b * gi).abs() <= ZTOL * sc && (0..nt).map(|k| (rr[1 + k] - gi * g[k]).powi(2)).sum::<f64>().sqrt() <= ZTOL * sc {
            zrows.push(i);
        }
    }
    if zrows.is_empty() {
        crate::census_bump!(AdjNone, 1);
        crate::census_bump!(AdjNoneNanos, _cen_t.elapsed().as_nanos());
        return (vec![], None);
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
                crate::census_bump!(AdjFast, 1);
                crate::census_bump!(AdjFastNanos, _cen_t.elapsed().as_nanos());
                return (vec![Nbr::Toggle(v, kk)], None);
            }
        }
    }
    let certified = cert.and_then(|f| f());
    if certified.is_some() && !audit {
        crate::census_bump!(AdjCertSkip, 1);
        crate::census_bump!(AdjCertSkipNanos, _cen_t.elapsed().as_nanos());
        return (vec![], certified);
    }
    let nb = null_basis(&g);
    let w = 1 + d1;
    // work tableau: n x (n+1) with the α column last
    let nc = n + 1;
    // the buffers of the previous work tableau are reused (every entry is overwritten)
    let mut work = ws.take().filter(|t| t.n == n && t.nc == nc && t.w == w).unwrap_or_else(|| Tab { n, nc, w, t: vec![0.0; n * nc], rho: vec![0.0; n * w], row_var: vec![0; n], col_var: vec![0; nc], pend: None });
    for i in 0..n {
        work.t[i * nc..i * nc + n].copy_from_slice(&tab.t[i * n..(i + 1) * n]);
        let rr = &tab.rho[i * tab.w..(i + 1) * tab.w];
        work.t[i * nc + n] = gam[i];
        work.rho[i * w] = rr[0] - b * gam[i];
        for (c, v) in nb.iter().enumerate() {
            work.rho[i * w + 1 + c] = (0..nt).map(|k| rr[1 + k] * v[k]).sum();
        }
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
    #[cfg(feature = "lp_census")]
    crate::census::basis_reset();
    pivot_facet(p, &mut work, n, Facet { ef: &ef, ff: &ff, sc: &rowsc }, 0, &mut out);
    *ws = Some(work);
    crate::census_bump!(AdjCase2, 1);
    crate::census_bump!(AdjCase2Nanos, _cen_t.elapsed().as_nanos());
    (out.into_iter().map(Nbr::Z).collect(), certified)
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
            let regions = if method == Method::PivotParallel { pivot_solve_par(program, &plcp, seed) } else { pivot_solve(program, &plcp, seed) };
            Ok(regions)
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

/// A child reached by Case-1 pivots has its tableau rebuilt from scratch every REFACTOR levels. The
/// tableau reached by pivots equals the rebuilt one in exact arithmetic (Lean `tableau_unique`); in
/// floating point the difference does not grow with the depth (bench/results/mplcp_tableau_handling.md:
/// at most 4e-12 relative to depth 28 on the 3 and 4 parameter problems, the same size as at depth
/// 0 to 7), so the period only trades a rebuild (about |z| pivots) against nothing. 8 -> 32.
const REFACTOR: usize = 32;

/// Where a queued region's tableau comes from. `Lazy` is the parent's tableau shared with the
/// siblings plus the principal pivot (row, column) that turns it into this region's tableau; the
/// pivot is done when the region is popped, and the last holder of the parent pivots it in place.
enum TabSrc {
    None,
    Ready(Tab),
    Lazy(std::sync::Arc<Tab>, usize, usize),
}

impl From<Option<Tab>> for TabSrc {
    fn from(t: Option<Tab>) -> Self {
        t.map_or(TabSrc::None, TabSrc::Ready)
    }
}

impl TabSrc {
    /// The tableau, rebuilt from the active set of `cr` when there is none.
    fn resolve(self, plcp: &Plcp, cr: &CriticalRegion) -> Option<Tab> {
        match self {
            TabSrc::Ready(t) => Some(t),
            TabSrc::Lazy(a, v, kk) => Some(match std::sync::Arc::try_unwrap(a) {
                Ok(mut t) => {
                    t.pivot(v, kk);
                    t
                }
                Err(a) => a.pivoted(v, kk),
            }),
            TabSrc::None => tableau_of(plcp, cr),
        }
    }
}

/// What is known about an active set that `adjacent` has returned: the index of its region in the
/// result list, or that its region was rejected (empty, lower dimensional, or not yet built).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    Built(usize),
    Rejected,
}

/// Exact map from an active set (as returned by `Plcp::active_set`) to its `Slot`; unlike the
/// rounded hyperplane keys of `Known` it has no collisions.
type Seen = crate::fxhash::FxHashMap<Vec<usize>, Slot>;

/// Claims `aset` as seen; true when it was new (it is then marked `Rejected` until its region is built).
fn seen_claim(seen: &mut Seen, aset: &[usize]) -> bool {
    if seen.contains_key(aset) {
        false
    } else {
        seen.insert(aset.to_vec(), Slot::Rejected);
        true
    }
}

/// A region whose neighbours are still to be explored: its index in the result list, its Chebyshev
/// center, and its tableau (rebuilt from the active set when `None`).
struct Item {
    idx: usize,
    center: Option<Vec<f64>>,
    tab: TabSrc,
    depth: usize,
    /// hyperplane (unit normal, offset) of the parent's facet this region was entered through,
    /// only when it was reached by a Case-1 Toggle pivot (see `expand_candidates`)
    entry: Option<(Vec<f64>, f64)>,
    /// vertices of the region polytope (flattened) when the fast clip builder produced them
    verts: Option<Vec<f64>>,
}

/// The initial region of the exploration (and its center) or `None` when the program has none.
fn initial_region(program: &MpProgram, seed: u64) -> Option<(CriticalRegion, Option<Vec<f64>>)> {
    let mut rng = StdRng::seed_from_u64(seed);
    let active_set = program.gen_optimal_active_set(&mut rng).or_else(|| super::degenerate_start(program, &mut rng))?;
    let mut initial = crate::mpqp_utils::gen_cr_from_active_set(program, &active_set, false)?;
    if dependent_rows(program, &initial) {
        if let Some(better) = lex_feasible_start(program, &initial) {
            initial = better;
        }
    }
    let nt = program.num_t();
    let init_center = if nt > 1 { chebyshev_ball(&initial.e, &initial.f, &[]).map(|s| s.x[..nt].to_vec()) } else { None };
    Some((initial, init_center))
}

/// The tableau of region `cr`, rebuilt from its active set.
fn tableau_of(plcp: &Plcp, cr: &CriticalRegion) -> Option<Tab> {
    let neq = plcp.eq.len();
    let z: Vec<usize> = cr.active_set[neq..].iter().map(|c| plcp.ineq.iter().position(|x| x == c).unwrap()).collect();
    tableau_for(plcp, &z)
}

/// A neighbour found by the facet pivoting, before its region is built.
struct Cand {
    aset: Vec<usize>,
    hint: Option<Vec<f64>>,
    /// principal pivot of the parent tableau that gives the neighbour's tableau
    piv: Option<(usize, usize)>,
    /// the parent's facet hyperplane (unit normal, offset), set only for a Toggle neighbour
    entry: Option<(Vec<f64>, f64)>,
    /// audit mode only: the known region that the certificate said is the only one across the facet
    cert: Option<usize>,
}

// ---------------------------------------------------------------------------- facet certificate

/// Certificate mode: 0 off, 1 on (default), 2 audit (certify, but still search, and count the new
/// regions that the search reaches across certified facets; for the census and the tests). Modes
/// 1 and 2 apply to 3 to 5 parameters, where the check measured faster or neutral; modes 3 (on) and
/// 4 (audit) apply to every count the vertex clip covers (2 to 6). At 2 parameters the problems are
/// too small to repay the bookkeeping, and at 6 the check (many vertices per facet) can cost more
/// than the search it saves (rand_t6_x5_m20_s0: 0.74x).
static CERT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Sets the facet-certificate mode of `Method::Pivot` and `Method::PivotParallel`: 0 off (the
/// default: at 6 parameters it lost a real region on masses3_N4, and the stray overlapping regions of
/// pop_005, pop_007 and pop_011 could pass as the known neighbour; it stays off until that is
/// understood), 1 on at 3 to 5 parameters, 2 audit, 3 and 4 the same at 2 to 6.
pub fn set_certificate(mode: u8) {
    CERT.store(mode, std::sync::atomic::Ordering::Relaxed);
}

static AUDIT_CERTIFIED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static AUDIT_EXTRA: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Audit mode counters: candidates returned across certified facets, and those of them that built
/// a new region (see `pivot_solve`; the regions are found anyway, the order can change).
pub fn certificate_audit() -> (usize, usize) {
    (AUDIT_CERTIFIED.load(std::sync::atomic::Ordering::Relaxed), AUDIT_EXTRA.load(std::sync::atomic::Ordering::Relaxed))
}

/// Does mode `mode` apply the certificate to a program with `nt` parameters?
fn cert_active(mode: u8, nt: usize) -> bool {
    match mode {
        1 | 2 => (3..=5).contains(&nt),
        3 | 4 => (2..=6).contains(&nt),
        _ => false,
    }
}

type HKey = [i64; 7];

/// Hash key of the unit-normalised hyperplane (a·θ <= b), rounded to 1e-5. Used only to find
/// candidate regions; a missed rounding boundary loses a skip, never soundness.
fn hkey(a: &[f64], b: f64) -> HKey {
    let nrm = a.iter().map(|v| v * v).sum::<f64>().sqrt();
    let mut k = [0i64; 7];
    for (o, v) in k.iter_mut().zip(a) {
        *o = (v / nrm * 1e5).round() as i64;
    }
    k[6] = (b / nrm * 1e5).round() as i64;
    k
}

/// The facet hyperplanes of the known regions, for 2 to 6 parameters (the range of the vertex clip).
#[derive(Default)]
struct Known {
    map: crate::fxhash::FxHashMap<HKey, Vec<u32>>,
}

impl Known {
    fn add(&mut self, idx: usize, cr: &CriticalRegion) {
        if cr.e.cols < 2 || cr.e.cols > 6 {
            return;
        }
        for k in 0..cr.e.rows {
            self.map.entry(hkey(cr.e.row(k), cr.f[k])).or_default().push(idx as u32);
        }
    }
}

struct CertCtx<'a> {
    verts: &'a [f64],
    known: &'a Known,
    regions: &'a [CriticalRegion],
    nt: usize,
    audit: bool,
    /// only regions with an index below this count as known: the regions that exist when the
    /// current breadth-first wave starts, the same set in the serial and the wave-parallel method
    limit: usize,
}

impl<'a> CertCtx<'a> {
    fn new(mode: u8, verts: Option<&'a [f64]>, known: &'a Known, regions: &'a [CriticalRegion], nt: usize, limit: usize) -> Option<CertCtx<'a>> {
        match (mode, verts) {
            (_, Some(v)) if cert_active(mode, nt) => Some(CertCtx { verts: v, known, regions, nt, audit: mode == 2 || mode == 4, limit }),
            _ => None,
        }
    }
}

/// Is the only region across facet k of `cur` already known? Returns its index.
///
/// Certificate. Let F be facet k (row k of the region, hyperplane g·θ = b). If a known region R
/// has a row on the same hyperplane with the opposite normal, and R contains F, then R is the only
/// region across F: F is a facet (its vertices span the hyperplane), R is full dimensional and has
/// F inside its facet on that hyperplane, so R fills the far side of every relative interior
/// point of F, and lexico-feasible regions do not overlap (Lean `lex_unique`), so no other region
/// reaches across F and `adjacent` could only return R. "R contains F" is checked on the vertices
/// of F (the region's vertices tight on row k) against R's rows: F is their convex hull. Without
/// vertices (more than 6 parameters, or the clip builder gave up) there is no certificate.
fn certify(cx: &CertCtx, cur: &CriticalRegion, k: usize) -> Option<usize> {
    let nt = cx.nt;
    let row = cur.e.row(k);
    let nrm = row.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !(nrm > 0.0) {
        return None;
    }
    let g: Vec<f64> = row.iter().map(|v| v / nrm).collect();
    let b = cur.f[k] / nrm;
    let ng: Vec<f64> = g.iter().map(|v| -v).collect();
    let cand = cx.known.map.get(&hkey(&ng, -b))?;
    // vertices of F
    let tol = 1e-9 * (1.0 + b.abs());
    let tight: Vec<&[f64]> = cx.verts.chunks_exact(nt).filter(|v| (v.iter().zip(&g).map(|(x, y)| x * y).sum::<f64>() - b).abs() <= 10.0 * tol).collect();
    if tight.len() < nt {
        return None;
    }
    // F has dimension nt - 1: the differences to the first vertex span nt - 1 directions
    let mut basis: Vec<Vec<f64>> = vec![];
    for v in &tight[1..] {
        let mut d: Vec<f64> = v.iter().zip(tight[0]).map(|(x, y)| x - y).collect();
        let n0 = d.iter().map(|x| x * x).sum::<f64>().sqrt();
        for u in &basis {
            let pr: f64 = d.iter().zip(u).map(|(x, y)| x * y).sum();
            for (x, y) in d.iter_mut().zip(u) {
                *x -= pr * y;
            }
        }
        let n1 = d.iter().map(|x| x * x).sum::<f64>().sqrt();
        if n1 > 1e-7 && n1 > 1e-3 * n0 {
            basis.push(d.iter().map(|x| x / n1).collect());
            if basis.len() == nt - 1 {
                break;
            }
        }
    }
    if basis.len() != nt - 1 {
        return None;
    }
    'regions: for &j in cand {
        if j as usize >= cx.limit {
            continue;
        }
        let r = &cx.regions[j as usize];
        // a row of R on the same hyperplane with the opposite normal
        let has_row = (0..r.e.rows).any(|i| {
            let ei = r.e.row(i);
            let n = ei.iter().map(|v| v * v).sum::<f64>().sqrt();
            n > 0.0 && ei.iter().zip(&g).all(|(x, y)| (x / n + y).abs() <= 1e-7) && (r.f[i] / n + b).abs() <= 1e-7 * (1.0 + b.abs())
        });
        if !has_row {
            continue;
        }
        // every vertex of F satisfies every row of R
        for i in 0..r.e.rows {
            let ei = r.e.row(i);
            let n = ei.iter().map(|v| v * v).sum::<f64>().sqrt();
            let fi = r.f[i] / n;
            for v in &tight {
                let s: f64 = ei.iter().zip(*v).map(|(x, y)| x * y).sum::<f64>() / n;
                if s - fi > 1e-9 * (1.0 + fi.abs()) {
                    continue 'regions;
                }
            }
        }
        return Some(j as usize);
    }
    None
}

/// Neighbour candidates across every facet of region `cur` (tableau `tab`).
fn expand_candidates(plcp: &Plcp, cur: &CriticalRegion, center: Option<&Vec<f64>>, tab: &Tab, depth: usize, entry: Option<&(Vec<f64>, f64)>, cert: Option<&CertCtx>, ws: &mut Option<Tab>) -> Vec<Cand> {
    let mut cands = vec![];
    // Entry facet skip. A child reached by a Case-1 Toggle pivot (principal pivot of one z row v with
    // its complement) shares with its parent the facet {parent e_k θ = f_k}. In Case 1 the neighbour
    // across the facet is the toggle for every point of the facet (it does not depend on the point),
    // and on that hyperplane the two bases coincide except for the swapped pair, so the facet is the
    // same set seen from both sides and the only region across it from the child is the parent, whose
    // active set is already in `seen` (its candidate would be dropped anyway). After a Case-2 (Z)
    // entry the child's side of the facet may border several regions, so nothing is skipped then.
    // The child's row is the parent's negated (unit normal and offset), within a tolerance; only the
    // first matching row is skipped, so a duplicated row still gets its search.
    let skip = entry.and_then(|(g, b)| {
        (0..cur.e.rows).find(|&k| {
            let r = cur.e.row(k);
            let nrm = r.iter().map(|v| v * v).sum::<f64>().sqrt();
            nrm > 0.0 && r.iter().zip(g).all(|(x, y)| (x / nrm + y).abs() <= 1e-9) && (cur.f[k] / nrm + b).abs() <= 1e-9 * (1.0 + b.abs())
        })
    });
    #[cfg(feature = "lp_census")]
    let mut cens: Vec<(u32, bool)> = vec![];
    for k in 0..cur.e.rows {
        if skip == Some(k) {
            continue;
        }
        // Certificate skip (see `certify`), asked inside `adjacent` once the fast path has failed.
        let cfun = || -> Option<usize> {
            let cx = cert?;
            #[cfg(feature = "lp_census")]
            let _ct = std::time::Instant::now();
            crate::census_bump!(CertFacets, 1);
            let r = certify(cx, cur, k);
            crate::census_bump!(CertNanos, _ct.elapsed().as_nanos());
            if r.is_some() {
                crate::census_bump!(CertHit, 1);
            }
            r
        };
        let (nbrs, certified) = adjacent(plcp, tab, cur.e.row(k), cur.f[k], &cur.e, &cur.f, ws, if cert.is_some() { Some(&cfun as &dyn Fn() -> Option<usize>) } else { None }, cert.map_or(false, |c| c.audit));
        if nbrs.is_empty() {
            continue;
        }
        #[cfg(feature = "lp_census")]
        {
            let gid = crate::census::next_group();
            cens.extend(nbrs.iter().map(|nb| (gid, matches!(nb, Nbr::Toggle(..)))));
            for nb in &nbrs {
                if matches!(nb, Nbr::Toggle(..)) {
                    crate::census_bump!(NbrFastRet, 1);
                } else {
                    crate::census_bump!(NbrSlowRet, 1);
                }
            }
        }
        // a point on the facet: the region's Chebyshev center projected onto the row
        let hint: Option<Vec<f64>> = center.map(|c| {
            let a = cur.e.row(k);
            let sft = cur.f[k] - a.iter().zip(c).map(|(x, y)| x * y).sum::<f64>();
            c.iter().zip(a).map(|(ci, ai)| ci + sft * ai).collect()
        });
        for nb in nbrs {
            let z = match &nb {
                Nbr::Toggle(v, _) => z_toggle(tab, plcp.n, *v),
                Nbr::Z(z) => z.clone(),
            };
            let entry = match &nb {
                Nbr::Toggle(..) => {
                    let a = cur.e.row(k);
                    let nrm = a.iter().map(|v| v * v).sum::<f64>().sqrt();
                    Some((a.iter().map(|v| v / nrm).collect::<Vec<f64>>(), cur.f[k] / nrm))
                }
                _ => None,
            };
            let piv = match nb {
                Nbr::Toggle(v, kk) if depth + 1 < REFACTOR => Some((v, kk)),
                _ => None,
            };
            cands.push(Cand { aset: plcp.active_set(z.into_iter()), hint: hint.clone(), piv, entry, cert: certified });
        }
    }
    #[cfg(feature = "lp_census")]
    crate::census::batch(&cens);
    cands
}

/// Switch between the multiplier law read from the tableau (the default) and the Cholesky KKT solve
/// `optimal_control_law_dual` (opt-out, and always the fallback for a doubtful basis: a tiny pivot
/// element or a non-finite row). The tableau law gives identical regions and saves the KKT solve of
/// every new region; its coefficients agree with the KKT solve to about 6e-9 relative on
/// ill-conditioned problems, so they are not bit-identical.
static TABLEAU_LAW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Selects the tableau law (`true`, default) or the KKT-solve law (`false`) for the regions built by
/// `Method::Pivot` and `Method::PivotParallel`.
pub fn set_tableau_law(on: bool) {
    TABLEAU_LAW.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Multiplier law (λ_AS = A_l θ + b_l, `aset` = equalities then sorted inequalities) of a region
/// read from its tableau, without a KKT solve.
///
/// Derivation. With z = λ, w = slack, the program's KKT system after eliminating x is
///   w - M z = q + R θ,   M = A Q^-1 A', q = b + A Q^-1 c, R = F + A Q^-1 H  (the KKT cache),
/// and x(θ) = -Q^-1 (c + H θ + A' λ) (done by `complete_primal_law` from the cached Q^-1 H, Q^-1 c
/// and A Q^-1). Split the rows into equalities E (w_E = 0, z_E free) and inequalities I.
///   Equality block: 0 = q_E + R_E θ + M_EE z_E + M_EI z_I, so with X = M_EE^-1 [M_EI | q_E | R_E]
///     z_E = -(X_q + X_R θ + X_I z_I).                                              (1)
///   Substituting (1) into the I rows gives the reduced pLCP of `Plcp::new`.
///   Region with basis B (z_B basic, all other z and every w_N zero): the tableau reads
///     z_B(θ) = ρ_B,0 + ρ_B,θ θ   (rows of ρ are [constant | θ coefficients]),
///   so the inequality multipliers are the tableau rows themselves (zero for the nonbasic z),
///   and the equality multipliers follow from (1) with z_I = z_B(θ):
///     A_l,E = -(X_R + X_I[:, B] ρ_B,θ),   b_l,E = -(X_q + X_I[:, B] ρ_B,0).
/// The cost is O(|B| nt + ne |B| nt) instead of a Cholesky of S = A_AS Q^-1 A_AS' and |B| nt solves.
/// `row(k, out)` writes the ρ row (width 1 + nt) of the basic z_k, returning false if it is not basic
/// or not finite.
fn tableau_law(plcp: &Plcp, aset: &[usize], mut row: impl FnMut(usize, &mut [f64]) -> bool) -> Option<crate::program::ControlLaw> {
    let (nt, neq) = (plcp.nt, plcp.eq.len());
    let k = aset.len();
    let mut a_l = Mat::zeros(k, nt);
    let mut b_l = vec![0.0; k];
    let mut buf = vec![0.0; 1 + nt];
    for (ii, &orig) in aset[neq..].iter().enumerate() {
        let pos = *plcp.pos.get(orig)?;
        if pos == usize::MAX || !row(pos, &mut buf) {
            return None;
        }
        b_l[neq + ii] = buf[0];
        a_l.row_mut(neq + ii).copy_from_slice(&buf[1..]);
    }
    for e in 0..neq {
        let xr = plcp.xe.row(e);
        let mut bq = -xr[plcp.n];
        let mut al: Vec<f64> = xr[plcp.n + 1..].iter().map(|v| -v).collect();
        for (ii, &orig) in aset[neq..].iter().enumerate() {
            let c = xr[plcp.pos[orig]];
            if c == 0.0 {
                continue;
            }
            bq -= c * b_l[neq + ii];
            for (o, l) in al.iter_mut().zip(a_l.row(neq + ii)) {
                *o -= c * l;
            }
        }
        b_l[e] = bq;
        a_l.row_mut(e).copy_from_slice(&al);
    }
    Some(crate::program::ControlLaw { a_x: Mat::zeros(0, 0), b_x: vec![], a_l, b_l })
}

/// The tableau law of candidate `c` whose parent basis has tableau `parent`: one pivot away when
/// `c.piv` is set (only the needed rows are formed, as in `Tab::pivoted`), otherwise from a fresh
/// tableau (returned so the caller can reuse it as the region's own tableau).
fn candidate_law(plcp: &Plcp, parent: &Tab, c: &Cand) -> Option<(crate::program::ControlLaw, Option<Tab>)> {
    let n = plcp.n;
    let w = 1 + plcp.nt;
    match c.piv {
        Some((r, col)) => {
            let p = parent.tv(r, col);
            if !(p.abs() > 1e-9) {
                return None;
            }
            let inv = 1.0 / p;
            let rrow: Vec<f64> = (0..w).map(|k| parent.rv(r, k) * -inv).collect();
            let entering = parent.col_var[col];
            let law = tableau_law(plcp, &c.aset, |k, out| {
                let var = n + k;
                if var == entering {
                    out.copy_from_slice(&rrow);
                } else {
                    let Some(i) = parent.row_var.iter().position(|&v| v == var) else { return false };
                    if i == r {
                        return false;
                    }
                    let f = parent.tv(i, col);
                    for (j, o) in out.iter_mut().enumerate() {
                        *o = parent.rv(i, j) + f * rrow[j];
                    }
                }
                out.iter().all(|v| v.is_finite())
            })?;
            Some((law, None))
        }
        None => {
            let neq = plcp.eq.len();
            let z: Vec<usize> = c.aset[neq..].iter().map(|&o| plcp.pos[o]).collect();
            let tab = tableau_for(plcp, &z)?;
            let law = tableau_law(plcp, &c.aset, |k, out| {
                let Some(i) = tab.row_var.iter().position(|&v| v == n + k) else { return false };
                for (j, o) in out.iter_mut().enumerate() {
                    *o = tab.rv(i, j);
                }
                out.iter().all(|v| v.is_finite())
            })?;
            Some((law, Some(tab)))
        }
    }
}

/// Region of a candidate (`None` when it is empty or lower dimensional), plus the candidate's own
/// tableau when it had to be formed from scratch for the law.
fn build_cand(program: &MpProgram, plcp: &Plcp, parent: &Tab, c: &Cand, want_verts: bool) -> Option<(CriticalRegion, Option<Vec<f64>>, Option<Tab>, Option<Vec<f64>>)> {
    if program.num_t() > 1 {
        let (law, own) = if TABLEAU_LAW.load(std::sync::atomic::Ordering::Relaxed) {
            match candidate_law(plcp, parent, c) {
                Some((l, t)) => (Some(l), t),
                None => (None, None),
            }
        } else {
            (None, None)
        };
        let (cr, ce, verts) = build::build_region(program, &c.aset, c.hint.as_deref(), law, want_verts)?;
        Some((cr, Some(ce), own, verts))
    } else {
        Some((gen_cr_from_active_set_lambda_first(program, &c.aset, true)?, None, None, None))
    }
}

fn pivot_solve(program: &MpProgram, plcp: &Plcp, seed: u64) -> Vec<CriticalRegion> {
    let Some((initial, init_center)) = initial_region(program, seed) else { return vec![] };
    let mut seen: Seen = Seen::default();
    seen.insert(initial.active_set.clone(), Slot::Built(0));
    let mut regions = vec![initial];
    let cmode = CERT.load(std::sync::atomic::Ordering::Relaxed);
    let mut known = Known::default();
    if cert_active(cmode, plcp.nt) {
        known.add(0, &regions[0]);
    }
    let mut queue: VecDeque<Item> = VecDeque::new();
    queue.push_back(Item { idx: 0, center: init_center, tab: TabSrc::None, depth: 0, entry: None, verts: None });
    let mut ws: Option<Tab> = None;
    let mut pending: Vec<(CriticalRegion, Option<Vec<f64>>, Option<(usize, usize)>, Option<Tab>, Option<(Vec<f64>, f64)>, Option<Vec<f64>>)> = vec![];
    let mut wave_limit = 1usize;
    while let Some(Item { idx, center, tab, depth, entry, verts }) = queue.pop_front() {
        if idx >= wave_limit {
            wave_limit = regions.len();
        }
        let cur = &regions[idx];
        let Some(tab) = crate::census_timed!(MplcpTabNanos, tab.resolve(plcp, cur)) else { continue };
        let cx = CertCtx::new(cmode, verts.as_deref(), &known, &regions, plcp.nt, wave_limit);
        let cands = crate::census_timed!(MplcpNbrNanos, expand_candidates(plcp, cur, center.as_ref(), &tab, depth, entry.as_ref(), cx.as_ref(), &mut ws));
        for c in cands {
            if c.cert.is_some() {
                AUDIT_CERTIFIED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            let fresh = seen_claim(&mut seen, &c.aset);
            #[cfg(feature = "lp_census")]
            crate::census::seen(fresh);
            if !fresh {
                crate::census_bump!(Dup, 1);
                continue;
            }
            crate::census_bump!(CandGen, 1);
            let built = crate::census_timed!(MplcpBuildNanos, build_cand(program, plcp, &tab, &c, cert_active(cmode, plcp.nt)));
            if let Some((cr, ce, own, vt)) = built {
                if c.cert.is_some() {
                    // audit: the search across a certified facet also reached a new region (it
                    // touches the facet only in a lower dimension; it is found through another
                    // facet when the certificate skips this one)
                    AUDIT_EXTRA.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    crate::census_bump!(CertExtra, 1);
                }
                pending.push((cr, ce, c.piv, own, c.entry, vt));
            }
        }
        // the neighbours' tableaux are one pivot away (rebuilt from scratch every REFACTOR levels);
        // each pivot is done when its region is popped, and the last one popped takes over this tableau
        let tab = std::sync::Arc::new(tab);
        for (cr, ce, piv, own, ent, vt) in pending.drain(..) {
            let (src, nd) = match piv {
                Some((v, kk)) => (TabSrc::Lazy(tab.clone(), v, kk), depth + 1),
                None => (TabSrc::from(own), 0),
            };
            seen.insert(cr.active_set.clone(), Slot::Built(regions.len()));
            regions.push(cr);
            if cert_active(cmode, plcp.nt) {
                known.add(regions.len() - 1, regions.last().unwrap());
            }
            queue.push_back(Item { idx: regions.len() - 1, center: ce, tab: src, depth: nd, entry: ent, verts: vt });
        }
    }
    regions
}

/// A wave (or batch of new regions) goes to rayon only when its estimated serial time exceeds this.
const PAR_NS: f64 = 20_000.0;

thread_local! {
    static WS: std::cell::RefCell<Option<Tab>> = const { std::cell::RefCell::new(None) };
}

/// `pivot_solve` with the regions of each breadth-first wave expanded and built in parallel. The
/// neighbour sets are merged in the order of the sequential method, so the same regions are found
/// in the same order. A wave runs in parallel only when the measured cost per region of the
/// earlier waves predicts more than `PAR_NS` of serial work; with one thread, or for small
/// problems, it is the serial method.
fn pivot_solve_par(program: &MpProgram, plcp: &Plcp, seed: u64) -> Vec<CriticalRegion> {
    use rayon::prelude::*;
    use std::sync::Arc;
    use std::time::Instant;
    if rayon::current_num_threads() <= 1 {
        return pivot_solve(program, plcp, seed);
    }
    let Some((initial, init_center)) = initial_region(program, seed) else { return vec![] };
    let mut seen: Seen = Seen::default();
    seen.insert(initial.active_set.clone(), Slot::Built(0));
    let mut regions = vec![initial];
    let cmode = CERT.load(std::sync::atomic::Ordering::Relaxed);
    let mut known = Known::default();
    if cert_active(cmode, plcp.nt) {
        known.add(0, &regions[0]);
    }
    let mut wave: Vec<Item> = vec![Item { idx: 0, center: init_center, tab: TabSrc::None, depth: 0, entry: None, verts: None }];
    // measured serial cost (ns) of expanding one region and of building one candidate
    let mut ns_expand: f64 = 0.0;
    let mut ns_build: f64 = 0.0;
    while !wave.is_empty() {
        let wave_limit = regions.len();
        let expand = |ws: &mut Option<Tab>, item: Item| -> Option<(Arc<Tab>, usize, Vec<Cand>)> {
            let Item { idx, center, tab, depth, entry, verts } = item;
            let cur = &regions[idx];
            let tab = tab.resolve(plcp, cur)?;
            let cx = CertCtx::new(cmode, verts.as_deref(), &known, &regions, plcp.nt, wave_limit);
            let cands = expand_candidates(plcp, cur, center.as_ref(), &tab, depth, entry.as_ref(), cx.as_ref(), ws);
            Some((Arc::new(tab), depth, cands))
        };
        let nw = wave.len();
        let go_par = nw >= 2 && ns_expand * nw as f64 >= PAR_NS;
        let t0 = Instant::now();
        let expanded: Vec<Option<(Arc<Tab>, usize, Vec<Cand>)>> = if go_par {
            wave.into_par_iter().map(|item| WS.with(|w| expand(&mut w.borrow_mut(), item))).collect()
        } else {
            let mut ws = None;
            wave.into_iter().map(|item| expand(&mut ws, item)).collect()
        };
        if !go_par {
            let per = t0.elapsed().as_nanos() as f64 / nw as f64;
            ns_expand = if ns_expand == 0.0 { per } else { 0.5 * (ns_expand + per) };
        }
        // new active sets, in sequential order
        let mut jobs: Vec<(Arc<Tab>, usize, Cand)> = vec![];
        for (tab, depth, cands) in expanded.into_iter().flatten() {
            for c in cands {
                if seen_claim(&mut seen, &c.aset) {
                    jobs.push((tab.clone(), depth, c));
                }
            }
        }
        let build = |(tab, depth, c): &(Arc<Tab>, usize, Cand)| -> Option<(CriticalRegion, Option<Vec<f64>>, Option<Tab>, usize, Option<(Vec<f64>, f64)>, Option<Vec<f64>>)> {
            let (cr, center, own, vt) = build_cand(program, plcp, tab, c, cert_active(cmode, plcp.nt))?;
            let (ntab, nd) = match c.piv {
                Some((v, kk)) => {
                    (Some(tab.pivoted(v, kk)), depth + 1)
                }
                None => (own, 0),
            };
            Some((cr, center, ntab, nd, c.entry.clone(), vt))
        };
        let nj = jobs.len();
        let go_par = nj >= 2 && ns_build * nj as f64 >= PAR_NS;
        let t0 = Instant::now();
        let built: Vec<Option<(CriticalRegion, Option<Vec<f64>>, Option<Tab>, usize, Option<(Vec<f64>, f64)>, Option<Vec<f64>>)>> = if go_par { jobs.par_iter().map(build).collect() } else { jobs.iter().map(build).collect() };
        if !go_par && nj > 0 {
            let per = t0.elapsed().as_nanos() as f64 / nj as f64;
            ns_build = if ns_build == 0.0 { per } else { 0.5 * (ns_build + per) };
        }
        wave = vec![];
        for (cr, center, tab, depth, entry, verts) in built.into_iter().flatten() {
            seen.insert(cr.active_set.clone(), Slot::Built(regions.len()));
            regions.push(cr);
            if cert_active(cmode, plcp.nt) {
                known.add(regions.len() - 1, regions.last().unwrap());
            }
            wave.push(Item { idx: regions.len() - 1, center, tab: tab.into(), depth, entry, verts });
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
    Plcp { n, nt, m, q, r, ineq: vec![], eq: vec![], pos: vec![], xe: Mat::zeros(0, 0) }
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
        crate::census_purpose!(Feasibility);
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
    Plcp { n, nt, m, q, r, ineq: p.ineq.clone(), eq: p.eq.clone(), pos: p.pos.clone(), xe: p.xe.clone() }
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

/// Is the complementary basis of `tab` lexico-feasible? A basic row whose parameter part vanishes
/// (a multiplier or slack that is identically zero: dependent active rows) is decided by the
/// perturbation, so [constant | β row] must be lexico-nonnegative; every other row is positive on
/// the interior of its region.
fn lex_feasible(tab: &Tab) -> bool {
    let (n, w) = (tab.n, tab.w);
    let mut wcol = vec![usize::MAX; n];
    for (c, &v) in tab.col_var.iter().enumerate() {
        if v < n {
            wcol[v] = c;
        }
    }
    let mut wrow = vec![usize::MAX; n];
    for (r, &v) in tab.row_var.iter().enumerate() {
        if v < n {
            wrow[v] = r;
        }
    }
    let mut v = vec![0.0; 1 + n];
    for i in 0..n {
        if (1..w).any(|k| tab.rv(i, k).abs() > ZTOL) {
            continue;
        }
        v[0] = tab.rv(i, 0);
        for j in 0..n {
            v[1 + j] = if wcol[j] != usize::MAX {
                -tab.tv(i, wcol[j])
            } else if wrow[j] == i {
                1.0
            } else {
                0.0
            };
        }
        if !lex_nonneg(&v) {
            return false;
        }
    }
    true
}

/// Cheap screen on the region's laws: does a multiplier (active row) or a slack (inactive row)
/// vanish identically? Only such a region can have a lexico-infeasible basis.
fn dependent_rows(program: &MpProgram, cr: &CriticalRegion) -> bool {
    let nt = program.num_t();
    let tiny = |c: &[f64], d: f64| d.abs() <= 1e-9 && c.iter().all(|v| v.abs() <= 1e-9);
    for (k, &i) in cr.active_set.iter().enumerate() {
        if !program.equality_indices.contains(&i) && k < cr.c.rows && tiny(cr.c.row(k), cr.d[k]) {
            return true;
        }
    }
    let nx = program.num_x();
    for i in 0..program.a.rows {
        if cr.active_set.contains(&i) {
            continue;
        }
        let ai = program.a.row(i);
        let d = program.b[i] - (0..nx).map(|j| ai[j] * cr.b[j]).sum::<f64>();
        let c: Vec<f64> = (0..nt).map(|t| program.f[(i, t)] - (0..nx).map(|j| ai[j] * cr.a[(j, t)]).sum::<f64>()).collect();
        if tiny(&c, d) {
            return true;
        }
    }
    false
}

/// Lexicographic Lemke at the parameter `th`: a complementary basis (z rows) that is
/// lexico-feasible for  w - M z = q + R th + ε,  or `None` (iteration limit or a ray).
/// M is positive semidefinite, so Lemke's method with the lexicographic ratio test terminates with
/// a solution whenever the LCP is feasible; the perturbation block β starts as the identity.
fn lex_lemke(p: &Plcp, th: &[f64]) -> Option<Vec<usize>> {
    let n = p.n;
    // columns: w (0..n), z (n..2n), z0 (2n); then rhs; then β (n)
    let nc = 2 * n + 1;
    let wd = nc + 1 + n;
    let mut a = vec![0.0; n * wd];
    for i in 0..n {
        a[i * wd + i] = 1.0;
        for j in 0..n {
            a[i * wd + n + j] = -p.m[i * n + j];
        }
        a[i * wd + 2 * n] = -1.0;
        a[i * wd + nc] = p.q[i] + (0..p.nt).map(|t| p.r[i * p.nt + t] * th[t]).sum::<f64>();
        a[i * wd + nc + 1 + i] = 1.0;
    }
    let mut basis: Vec<usize> = (0..n).collect();
    let rhs_beta = |a: &[f64], i: usize| -> Vec<f64> { a[i * wd + nc..(i + 1) * wd].to_vec() };
    // z0 enters at the lexico-minimal row, if any row is lexico-negative
    let mut leave = None;
    for i in 0..n {
        let v = rhs_beta(&a, i);
        if !lex_nonneg(&v) && leave.map_or(true, |l| lex_cmp(&v, &rhs_beta(&a, l)) == Ordering::Less) {
            leave = Some(i);
        }
    }
    let Some(mut r) = leave else { return Some(vec![]) };
    let mut enter = 2 * n;
    for _ in 0..(50 * n + 200) {
        // pivot (r, enter)
        let piv = a[r * wd + enter];
        if piv.abs() < 1e-11 {
            return None;
        }
        for k in 0..wd {
            a[r * wd + k] /= piv;
        }
        let prow: Vec<f64> = a[r * wd..(r + 1) * wd].to_vec();
        for i in 0..n {
            if i != r {
                let f = a[i * wd + enter];
                if f != 0.0 {
                    for k in 0..wd {
                        a[i * wd + k] -= f * prow[k];
                    }
                    a[i * wd + enter] = 0.0;
                }
            }
        }
        let out = basis[r];
        basis[r] = enter;
        if out == 2 * n {
            return Some(basis.iter().filter(|&&v| v >= n && v < 2 * n).map(|&v| v - n).collect::<Vec<_>>()).map(|mut z| {
                z.sort_unstable();
                z
            });
        }
        enter = comp(out, n);
        // lexicographic ratio test
        let mut best: Option<(usize, Vec<f64>)> = None;
        for i in 0..n {
            let d = a[i * wd + enter];
            if d > 1e-11 {
                let v: Vec<f64> = a[i * wd + nc..(i + 1) * wd].iter().map(|x| x / d).collect();
                if best.as_ref().map_or(true, |(_, b)| lex_cmp(&v, b) == Ordering::Less) {
                    best = Some((i, v));
                }
            }
        }
        r = best?.0;
    }
    None
}

/// A lexico-feasible start for a start region `init` with identically vanishing rows (dependent
/// active rows): its basis may be lexico-infeasible, and the walk from such a basis visits both it
/// and its lexico-feasible twin, which describe the same parameter set. The twin is found by the
/// lexicographic Lemke method at the Chebyshev center of `init`. `None` keeps `init`.
fn lex_feasible_start(program: &MpProgram, init: &CriticalRegion) -> Option<CriticalRegion> {
    let plcp = Plcp::new(program)?;
    if tableau_of(&plcp, init).map_or(true, |t| lex_feasible(&t)) {
        return None;
    }
    let nt = program.num_t();
    let th = chebyshev_ball(&init.e, &init.f, &[]).map(|s| s.x[..nt].to_vec())?;
    let z = lex_lemke(&plcp, &th)?;
    let aset = plcp.active_set(z.into_iter());
    let cr = crate::mpqp_utils::gen_cr_from_active_set(program, &aset, false)?;
    if !tableau_of(&plcp, &cr).map_or(false, |t| lex_feasible(&t)) || !cr.is_inside(&th, 1e-7) {
        return None;
    }
    Some(cr)
}

/// Diagnostic: number of `regions` whose basis is lexico-infeasible (0 for the output of `solve`).
pub fn count_lex_infeasible(program: &MpProgram, regions: &[CriticalRegion]) -> usize {
    match Plcp::new(program) {
        Some(plcp) => regions.iter().filter(|cr| tableau_of(&plcp, cr).map_or(false, |t| !lex_feasible(&t))).count(),
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Case-2 vectors row_j/a_j - row_l/a_l carry a factor 1/a with a down to PTOL, so their
    /// sign is read relative to the largest entry, not against the absolute ZTOL.
    #[test]
    fn lex_nonneg_rel_is_scale_free() {
        // 1e-10-sized entries: the absolute test calls the vector zero, the relative one reads its sign
        assert!(lex_nonneg(&[-1e-10, 1e-10]));
        assert!(!lex_nonneg_rel(&[-1e-10, 1e-10]));
        assert!(lex_nonneg_rel(&[1e-10, -1e-10]));
        // an entry below ZTOL relative to the largest is noise
        assert!(lex_nonneg_rel(&[-1e-12, 1.0]));
        assert!(!lex_nonneg_rel(&[0.0, -1.0, 5.0]));
        assert!(lex_nonneg_rel(&[0.0, 0.0]));
    }
}
