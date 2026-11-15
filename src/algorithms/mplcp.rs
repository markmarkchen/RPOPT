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
    /// Euclidean radius of the parameter box (every region lies within it), or -1 when the box is unbounded or unknown
    coord_bound: f64,
    /// rows a_i of the constraint matrix and a_i Q^-1 of the inequality rows (n x n_x), empty with equality rows: the original data
    /// that `refined_multipliers` recomputes a dependency from
    arows: Mat,
    aqi: Mat,
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
        let coord_bound = match program.theta_bbox() {
            Some((lo, hi)) if lo.iter().chain(hi.iter()).all(|v| v.is_finite()) => lo.iter().zip(hi).map(|(l, h)| (l * l).max(h * h)).sum::<f64>().sqrt(),
            _ => -1.0,
        };
        let (mut arows, mut aqi) = (Mat::zeros(0, 0), Mat::zeros(0, 0));
        if eq.is_empty() {
            let nx = program.a.cols;
            arows = Mat::zeros(n, nx);
            aqi = Mat::zeros(n, nx);
            for (a, &i) in ineq.iter().enumerate() {
                arows.row_mut(a).copy_from_slice(program.a.row(i));
                aqi.row_mut(a).copy_from_slice(kc.a_qi.row(i));
            }
        }
        Some(Plcp { n, nt, m, q, r, ineq, eq, pos, xe, coord_bound, arows, aqi })
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

/// Smallest pivot of row `i` that `tableau_for` accepts. Relative to the diagonal of M = A Q^-1 A' when that is below 1, so
/// that a constraint of small scale (the diagonal of M runs from 5e-13 to 5e2 on the masses family) is not refused for
/// having a pivot below an absolute 1e-11; unchanged (1e-11) for rows of scale 1 and above.
fn pivot_floor(p: &Plcp, i: usize) -> f64 {
    (1e-11 * p.m[i * p.n + i].abs().min(1.0)).max(1e-300)
}

/// Persistent tableau of the basis with z-basic rows `z`: rows [q | R | I], T = M, then one
/// principal pivot per z row (nonsingular when M_zz is positive definite). The pivots go in the order of `z`; if one falls
/// below its floor the basis is tried again with the largest remaining pivot (relative to the diagonal of M) first, which
/// accepts every basis that has a pivot order above the floors; only then is it refused.
fn tableau_for(p: &Plcp, z: &[usize]) -> Option<Tab> {
    let (n, nt) = (p.n, p.nt);
    let w = 1 + nt;
    let fresh = || {
        let mut rho = vec![0.0; n * w];
        for i in 0..n {
            rho[i * w] = p.q[i];
            rho[i * w + 1..i * w + 1 + nt].copy_from_slice(&p.r[i * nt..(i + 1) * nt]);
        }
        Tab { n, nc: n, w, t: p.m.clone(), rho, row_var: (0..n).collect(), col_var: (n..2 * n).collect(), pend: None }
    };
    let mut tab = fresh();
    let mut ok = true;
    for &i in z {
        let piv = tab.t[i * n + i];
        if !(piv.abs() >= pivot_floor(p, i)) {
            ok = false;
            break;
        }
        tab.pivot(i, i);
    }
    if ok {
        return Some(tab);
    }
    // second attempt: largest relative pivot first
    let mut tab = fresh();
    let mut left: Vec<usize> = z.to_vec();
    while !left.is_empty() {
        let (k, best) = left
            .iter()
            .enumerate()
            .map(|(k, &i)| (k, tab.t[i * n + i].abs() / p.m[i * n + i].abs().max(1e-300)))
            .fold((0, -1.0f64), |a, b| if b.1 > a.1 { b } else { a });
        let i = left.swap_remove(k);
        let piv = tab.t[i * n + i];
        if !(piv.abs() >= pivot_floor(p, i)) || !best.is_finite() {
            crate::census_bump!(TabRefuse, 1);
            if piv.abs() > 1e-6 * p.m[i * n + i].abs() {
                crate::census_bump!(TabRefuseRel, 1);
            }
            return None;
        }
        tab.pivot(i, i);
    }
    crate::census_bump!(TabRescued, 1);
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
    /// active sets already known (built or rejected): a leaf branch that ends in one of them is
    /// skipped without its Chebyshev LP, since `expand_candidates` would drop it anyway
    seen: Option<&'a Seen>,
    /// the old hint (parent centre projected onto the row) in facet coordinates, for the census only
    hint_f: Option<&'a [f64]>,
}

/// Algorithm 2 (`pivot`): from the ACLFB `tab` (α basic or entering as column `e`) returns the
/// z-row sets of the adjacent bases.
#[inline(never)]
fn walk_one(p: &Plcp, tab: &mut Tab, e: usize, fpoly: Facet, depth: usize, centre: Option<&[f64]>, witness: Option<&[f64]>, first: Option<WalkKey>, left: &mut usize, stack: &mut Vec<Frame>, out: &mut Vec<(Vec<usize>, Option<Vec<f64>>)>) -> WalkEnd {
    let n = p.n;
    let d1 = p.nt - 1;
    let alpha = 2 * n;
    let mut e = e;
    let mut first = first;
    #[cfg(feature = "lp_census")]
    crate::census::depth_note(depth);
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
    loop {
        crate::census_t0!(cm);
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
        let pset: Vec<usize> = (0..n).filter(|&i| a[i] > PTOL).collect();
        let in_z = |i: usize| rowmax(i) <= ZTOL * fpoly.sc[tab.row_var[i]];
        let pz: Vec<usize> = pset.iter().cloned().filter(|&i| in_z(i)).collect();
        let alpha_row = tab.row_var.iter().position(|&v| v == alpha);
        crate::census_lap!(cm, Prelude);
        // No row that vanishes on the facet blocks the entering variable and α rises: the entering
        // variable replaces α, which is the neighbour. This holds also when P is empty (nothing
        // decreases at all). Returning there, as before, lost the neighbour whenever the previous
        // pivot had exchanged a dependent pair (pop_081: parallel rows of a one-variable program).
        if pz.is_empty() {
            if let Some(ra) = alpha_row {
                if tab.tv(ra, e) > PTOL {
                    // the hint of the returned neighbour is the Chebyshev centre of the sub-facet, or the witness
                    // that certified it nonempty (a sub-facet accepted without its LP has no centre)
                    let z = z_after(tab, n, ra, e);
                    push_leaf(&fpoly, z, centre.or(witness), out);
                    crate::census_lap!(cm, Leaf);
                    return WalkEnd::Done;
                }
            }
        }
        if pset.is_empty() {
            return WalkEnd::Done;
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
            crate::census_lap!(cm, Case1);
            if leaving == alpha {
                return WalkEnd::Done;
            }
            if *left == 0 {
                return WalkEnd::Budget;
            }
            *left -= 1;
            tab.flush();
            tab.defer_pivot(l, e);
            e = tab.col_of(comp(leaving, n));
            crate::census_lap!(cm, Pivot);
            // exact cycle check: any repeated state of the walk is a return to the state right after
            // its first pivot (Lean walk_repeat_returns)
            match &first {
                None => first = Some(walk_key(tab, e)),
                Some(k) if walk_key_is(k, tab, e) => {
                    crate::census_bump!(WalkCycle, 1);
                    return WalkEnd::Cycle;
                }
                _ => {}
            }
            crate::census_lap!(cm, Key);
            continue;
        }
        // Case 2: the leaving variable depends on θ_f
        crate::census_bump!(PfCase2, 1);
        let mut children: Vec<Frame> = vec![];
        let mut cycled = false;
        crate::census_skip!(cm);
        // the rows of P scaled by 1/a, once for all pairs: R[pi][k] = rfull(j, k) / a[j] (the same division the
        // pairwise differences below used), and the largest |R| over the first 1 + d1 entries
        let ww = w + n;
        let mut rs: Vec<f64> = Vec::with_capacity(pset.len() * ww);
        let mut rmax: Vec<f64> = Vec::with_capacity(pset.len());
        for &j in &pset {
            let start = rs.len();
            rs.extend((0..ww).map(|k| rfull(j, k) / a[j]));
            rmax.push(rs[start..start + 1 + d1].iter().fold(0.0f64, |m, v| m.max(v.abs())));
        }
        // cut rows of the pairs, staged until the branch survives its lexicographic tests
        let mut cut_rows: Vec<f64> = vec![];
        let mut cut_rhs: Vec<f64> = vec![];
        let mut lexv: Vec<f64> = Vec::with_capacity(1 + n);
        crate::census_lap!(cm, Gamma);
        for (pl, &l) in pset.iter().enumerate() {
            crate::census_bump!(PfBranches, 1);
            let leaving = tab.row_var[l];
            // lexico-inequalities (16): Γ_j = a_l row_j - a_j row_l  ⪰ 0 for all j in P. Γ_j scales
            // with a_l a_j (1e-20 for rows with a near PTOL), so the tolerances are applied to the
            // scale-free Γ_j / (a_l a_j) = row_j / a_j - row_l / a_l instead.
            let mut ok = true;
            cut_rows.clear();
            cut_rhs.clear();
            let rl = &rs[pl * ww..(pl + 1) * ww];
            for (pj, &j) in pset.iter().enumerate() {
                if j == l {
                    continue;
                }
                let rj = &rs[pj * ww..(pj + 1) * ww];
                let gsc = 1.0 + rmax[pj].max(rmax[pl]);
                let gq_zero = (1..1 + d1).all(|k| (rj[k] - rl[k]).abs() <= ZTOL * gsc);
                if gq_zero {
                    lexv.clear();
                    lexv.push(rj[0] - rl[0]);
                    lexv.extend((1 + d1..ww).map(|k| rj[k] - rl[k]));
                    if !lex_nonneg_rel(&lexv) {
                        ok = false;
                        break;
                    }
                } else {
                    // g0 + g_q θ_f >= 0  ->  -g_q θ_f <= g0
                    let nrm = (1..1 + d1).map(|k| (rj[k] - rl[k]) * (rj[k] - rl[k])).sum::<f64>().sqrt();
                    cut_rows.extend((1..1 + d1).map(|k| -(rj[k] - rl[k]) / nrm));
                    cut_rhs.push((rj[0] - rl[0]) / nrm);
                }
            }
            crate::census_lap!(cm, Gamma);
            if !ok {
                crate::census_bump!(PfLexCut, 1);
                continue;
            }
            let (mut ef, mut ff) = (fpoly.ef.clone(), fpoly.ff.to_vec());
            for (r, &f) in cut_rows.chunks(d1.max(1)).zip(&cut_rhs) {
                ef.push_row(r);
                ff.push(f);
            }
            // a leaf branch (alpha leaves: the next basis is complementary and the walk returns it)
            // whose active set is known is dropped by the caller anyway: skip its Chebyshev LP
            let leaf_z = if leaving == alpha { Some(z_after(tab, n, l, e)) } else { None };
            let leaf_known = match (&leaf_z, fpoly.seen) {
                (Some(z), Some(seen)) => seen.contains_key(&p.active_set(z.iter().copied())),
                _ => false,
            };
            crate::census_lap!(cm, Seen);
            if leaf_known {
                crate::census_bump!(LeafSkip, 1);
                continue;
            }
            if leaf_z.is_some() {
                crate::census_bump!(LeafKept, 1);
            }
            let mut bcentre: Option<Vec<f64>> = None;
            let mut bwit: Option<Vec<f64>> = None;
            if d1 > 0 {
                // Nonemptiness needs the Chebyshev LP only when no point is known to clear every row of
                // the piece: a non-leaf branch is accepted on a witness (its centre is computed later,
                // if a leaf below it needs it as a hint). A leaf branch always gets its centre now.
                if leaf_z.is_none() {
                    if ef.rows == fpoly.ef.rows && witness.is_some() {
                        // no theta-dependent cut: the piece is the parent polytope
                        crate::census_bump!(WitnessAccept, 1);
                        crate::census_bump!(WitnessNoCuts, 1);
                        bwit = witness.map(|w| w.to_vec());
                        bcentre = centre.map(|c| c.to_vec());
                    } else {
                        let cands = [witness, fpoly.hint_f];
                        let clears = |pt: &[f64]| (0..ef.rows).all(|i| ff[i] - ef.row(i).iter().zip(pt).map(|(a, b)| a * b).sum::<f64>() >= WITNESS_MARGIN);
                        if let Some(pt) = cands.iter().flatten().find(|pt| clears(pt)) {
                            crate::census_bump!(WitnessAccept, 1);
                            bwit = Some(pt.to_vec());
                        } else if cands.iter().any(|c| c.is_some()) {
                            crate::census_bump!(WitnessFail, 1);
                        }
                        crate::census_lap!(cm, Wit);
                    }
                } else if ef.rows == fpoly.ef.rows && centre.is_some() {
                    bcentre = centre.map(|c| c.to_vec());
                    bwit = witness.map(|w| w.to_vec());
                }
                #[cfg(feature = "lp_census")]
                let boxe = if bwit.is_none() && bcentre.is_none() { box_probe(&ef, &ff, fpoly.ef.rows) } else { false };
                crate::census_lap!(cm, Census);
                let dq = bwit.is_none() && bcentre.is_none() && daqp_certainly_empty(&ef, &ff, d1, p.coord_bound);
                if bwit.is_none() && bcentre.is_none() {
                    crate::census_lap!(cm, Daqp);
                }
                if dq {
                    #[cfg(feature = "lp_census")]
                    if boxe {
                        crate::census_bump!(BoxEmptyDaqp, 1);
                    }
                    crate::census_bump!(PfChebEmpty, 1);
                    continue;
                }
                if bwit.is_none() && bcentre.is_none() {
                    crate::census_bump!(PfCheb, 1);
                    #[cfg(feature = "lp_census")]
                    let piv0 = crate::census::piv_get();
                    #[cfg(feature = "lp_census")]
                    let lp_t = std::time::Instant::now();
                    let ball = {
                        crate::census_purpose!(Neighbour);
                        chebyshev_ball(&ef, &ff, &[])
                    };
                    crate::census_lap!(cm, Lp);
                    #[cfg(feature = "lp_census")]
                    let lp_ns = lp_t.elapsed().as_nanos() as u64;
                    #[cfg(feature = "lp_census")]
                    {
                        let lp_ok = matches!(&ball, Some(s) if s.x[d1] > 1e-9);
                        if boxe {
                            crate::census_bump!(BoxEmpty, 1);
                            if lp_ok {
                                crate::census_bump!(BoxEmptyNonempty, 1);
                            } else {
                                crate::census_bump!(BoxEmptyLp, 1);
                            }
                        }
                        cheb_probe(&ef, &ff, fpoly.ef.rows, centre.or(witness).or(fpoly.hint_f), d1, crate::census::piv_get() - piv0, ball.as_ref().map(|s| s.x.clone()), leaf_z.is_some(), lp_ns);
                    }
                    crate::census_lap!(cm, Census);
                    match ball {
                        Some(s) if s.x[d1] > 1e-9 => {
                            bcentre = Some(s.x[..d1].to_vec());
                            bwit = bcentre.clone();
                        }
                        _ => {
                            crate::census_bump!(PfChebEmpty, 1);
                            continue;
                        }
                    }
                }
            }
            if let Some(z) = leaf_z {
                crate::census_skip!(cm);
                push_leaf(&Facet { ef: &ef, ff: &ff, sc: fpoly.sc, seen: fpoly.seen, hint_f: fpoly.hint_f }, z, bcentre.as_deref(), out);
                crate::census_lap!(cm, Leaf);
                continue;
            }
            if *left == 0 {
                return WalkEnd::Budget;
            }
            *left -= 1;
            crate::census_skip!(cm);
            let mut next = tab.clone();
            crate::census_lap!(cm, Clone);
            next.flush();
            next.defer_pivot(l, e);
            let ne = next.col_of(comp(leaving, n));
            crate::census_lap!(cm, Pivot);
            let cfirst = match &first {
                None => Some(walk_key(&next, ne)),
                Some(k) if walk_key_is(k, &next, ne) => {
                    crate::census_bump!(WalkCycle, 1);
                    cycled = true;
                    continue;
                }
                Some(k) => Some(k.clone()),
            };
            crate::census_lap!(cm, Key);
            #[cfg(feature = "lp_census")]
            {
                let mut key = next.row_var.clone();
                key.sort_unstable();
                if crate::census::basis_seen(key) {
                    crate::census_bump!(PfRepeatBasis, 1);
                }
            }
            crate::census_lap!(cm, Census);
            children.push(Frame { tab: next, e: ne, ef, ff, centre: bcentre, witness: bwit, depth: depth + 1, first: cfirst });
            crate::census_lap!(cm, Stack);
        }
        // the branches are explored in order, depth first: the first one is on top of the stack
        stack.extend(children.into_iter().rev());
        crate::census_lap!(cm, Stack);
        return if cycled { WalkEnd::Cycle } else { WalkEnd::Done };
    }
}

/// How one walk of `pivot_facet` ended.
enum WalkEnd {
    /// every branch of this walk ended (a returned basis or a dead end)
    Done,
    /// a state repeated: some branch is lost
    Cycle,
    /// the pivot budget of the facet ran out
    Budget,
}

/// State of a walk: basis labels and entering column.
type WalkKey = (Vec<usize>, Vec<usize>, usize);

fn walk_key(tab: &Tab, e: usize) -> WalkKey {
    (tab.row_var.clone(), tab.col_var.clone(), e)
}

fn walk_key_is(k: &WalkKey, tab: &Tab, e: usize) -> bool {
    k.2 == e && k.0 == tab.row_var && k.1 == tab.col_var
}

/// A Case-2 branch waiting to be walked: the tableau after the branch pivot, its entering column,
/// the sub-facet polytope and its Chebyshev centre, and the state after the first pivot of the walk.
struct Frame {
    tab: Tab,
    e: usize,
    ef: Mat,
    ff: Vec<f64>,
    centre: Option<Vec<f64>>,
    /// a point at least `WITNESS_MARGIN` inside the sub-facet polytope (so it is nonempty with Chebyshev
    /// radius above the LP's 1e-9): the Chebyshev centre when the LP ran, else the accepted witness
    witness: Option<Vec<f64>>,
    depth: usize,
    first: Option<WalkKey>,
}

/// Algorithm 2 on an explicit work stack (no depth limit, no iteration cap). Returns false when a
/// repeated state or the pivot `budget` stopped some branch, so a neighbour may be missing.
#[inline(never)]
fn pivot_facet(p: &Plcp, tab: &mut Tab, e: usize, fpoly: Facet, budget: usize, out: &mut Vec<(Vec<usize>, Option<Vec<f64>>)>, lost: &mut Vec<Vec<f64>>) -> bool {
    let mut left = budget;
    let mut stack: Vec<Frame> = vec![];
    let mut complete = true;
    match walk_one(p, tab, e, Facet { ef: fpoly.ef, ff: fpoly.ff, sc: fpoly.sc, seen: fpoly.seen, hint_f: fpoly.hint_f }, 0, None, None, None, &mut left, &mut stack, out) {
        WalkEnd::Done => {}
        WalkEnd::Cycle => complete = false,
        WalkEnd::Budget => {
            crate::census_bump!(WalkBudget, 1);
            // the frames not yet walked are lost: their sub-facets are probed by the caller
            lost.extend(stack.drain(..).filter_map(|f| f.centre.or(f.witness)));
            return false;
        }
    }
    loop {
        crate::census_t0!(cm);
        let Some(fr) = stack.pop() else { break };
        let Frame { mut tab, e, ef, ff, centre, witness, depth, first } = fr;
        let facet = Facet { ef: &ef, ff: &ff, sc: fpoly.sc, seen: fpoly.seen, hint_f: fpoly.hint_f };
        crate::census_lap!(cm, Stack);
        let end = walk_one(p, &mut tab, e, facet, depth, centre.as_deref(), witness.as_deref(), first, &mut left, &mut stack, out);
        crate::census_skip!(cm);
        match end {
            WalkEnd::Done => {}
            WalkEnd::Cycle => {
                complete = false;
                lost.extend(centre.or(witness));
            }
            WalkEnd::Budget => {
                crate::census_bump!(WalkBudget, 1);
                lost.extend(centre.or(witness));
                lost.extend(stack.drain(..).filter_map(|f| f.centre.or(f.witness)));
                return false;
            }
        }
        drop(tab);
        drop(ef);
        drop(ff);
        crate::census_lap!(cm, Stack);
    }
    complete
}

/// Census only: what a walk Chebyshev LP looked like against its parent (rows added, parent point violations, cold pivots), and
/// an extra primal-shifted solve from the parent point to see how many pivots a warm start takes.
#[cfg(feature = "lp_census")]
fn cheb_probe(ef: &Mat, ff: &[f64], parent_rows: usize, pt: Option<&[f64]>, d1: usize, cold_piv: u64, cold_x: Option<Vec<f64>>, leaf: bool, lp_ns: u64) {
    use crate::census::Counter as C;
    if !probes() {
        return;
    }
    if leaf {
        crate::census::bump(C::LpLeafCalls, 1);
        crate::census::bump(C::LpLeafNs, lp_ns);
        crate::census::bump(C::LpLeafPiv, cold_piv);
    } else {
        crate::census::bump(C::LpNlCalls, 1);
        crate::census::bump(C::LpNlNs, lp_ns);
        crate::census::bump(C::LpNlPiv, cold_piv);
    }
    let ne = cold_x.as_ref().map_or(false, |x| x[d1] > 1e-9) as usize;
    let eo = crate::census::EO_BASE + ne * 8;
    crate::census::bump_slot(eo, 1);
    crate::census::bump_slot(eo + 1, cold_piv);
    crate::census::bump_slot(eo + 2, lp_ns);
    // phase 1 only: feasibility of the rows shifted by 1e-9 ||a|| (radius above 1e-9)
    let ffs: Vec<f64> = (0..ef.rows).map(|i| ff[i] - 1e-9 * ef.row(i).iter().map(|v| v * v).sum::<f64>().sqrt()).collect();
    let t = std::time::Instant::now();
    let p0 = crate::census::piv_get();
    let feas = {
        crate::census_purpose!(Other);
        crate::solver::lp::lp_feasible_point(ef, &ffs, &[], None).is_some()
    };
    crate::census::bump(C::P1Calls, 1);
    crate::census::bump(C::P1Pivots, crate::census::piv_get() - p0);
    let p1_ns = t.elapsed().as_nanos() as u64;
    crate::census::bump(C::P1Ns, p1_ns);
    let p1_piv = crate::census::piv_get() - p0;
    if cold_x.as_ref().map_or(false, |x| x[d1] > 1e-9) == feas {
        crate::census::bump(C::P1Same, 1);
    }
    crate::census::bump_slot(eo + 6, p1_piv);
    crate::census::bump_slot(eo + 7, p1_ns);
    // exact redundancy of the rows of the piece, on every 8th LP
    thread_local! {
        static RED_N: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }
    let k = RED_N.with(|c| {
        c.set(c.get() + 1);
        c.get()
    });
    if k % 8 == 0 && ef.rows > 0 {
        let mut nonred = 0u64;
        for i in 0..ef.rows {
            let keep: Vec<usize> = (0..ef.rows).filter(|&j| j != i).collect();
            let (a2, b2) = (ef.select_rows(&keep), keep.iter().map(|&j| ff[j]).collect::<Vec<f64>>());
            let obj: Vec<f64> = ef.row(i).iter().map(|v| -v).collect();
            let red = {
                crate::census_purpose!(Other);
                match crate::solver::lp::lp_point(Some(&obj), &a2, &b2, &[]) {
                    Some(x) => ef.row(i).iter().zip(&x).map(|(a, b)| a * b).sum::<f64>() <= ff[i] + 1e-9 * (1.0 + ff[i].abs()),
                    None => false,
                }
            };
            if !red {
                nonred += 1;
            }
        }
        crate::census::bump(C::RedSamples, 1);
        crate::census::bump(C::RedRows, ef.rows as u64);
        crate::census::bump(C::RedNonRed, nonred);
    }
    let added = ef.rows - parent_rows;
    crate::census::bump(C::ChebRows, ef.rows as u64);
    crate::census::bump(C::ChebDims, d1 as u64);
    crate::census::bump(C::ChebAdded, added as u64);
    crate::census::bump([C::ChebAdd0, C::ChebAdd1, C::ChebAdd2, C::ChebAdd3p][added.min(3)], 1);
    crate::census::bump(C::ChebPivots, cold_piv);
    let Some(pt) = pt else { return };
    crate::census::bump(C::ChebHasPt, 1);
    let nrm = |i: usize| ef.row(i).iter().map(|v| v * v).sum::<f64>().sqrt();
    let slack = |i: usize| ff[i] - ef.row(i).iter().zip(pt).map(|(a, b)| a * b).sum::<f64>();
    let (mut va, mut vn) = (0u64, 0u64);
    let mut r0 = f64::INFINITY;
    for i in 0..ef.rows {
        if slack(i) < 0.0 {
            va += 1;
            if i >= parent_rows {
                vn += 1;
            }
        }
        r0 = r0.min(slack(i) / nrm(i));
    }
    crate::census::bump(C::ChebViolAll, va);
    crate::census::bump(C::ChebViolNew, vn);
    crate::census::bump(C::ChebPivotsPt, cold_piv);
    if va == 0 {
        crate::census::bump(C::ChebPtInside, 1);
    }
    let mut x0 = pt.to_vec();
    x0.push(r0.max(0.0));
    let t = std::time::Instant::now();
    let p0 = crate::census::piv_get();
    let sol = {
        crate::census_purpose!(Other);
        crate::mpqp_utils::chebyshev_ball_from(ef, ff, &x0)
    };
    let np = crate::census::piv_get() - p0;
    crate::census::bump(C::ChebShiftCalls, 1);
    crate::census::bump(C::ChebShiftPivots, np);
    let sh_ns = t.elapsed().as_nanos() as u64;
    crate::census::bump(C::ChebShiftNs, sh_ns);
    {
        let eo = crate::census::EO_BASE + (cold_x.as_ref().map_or(false, |x| x[d1] > 1e-9) as usize) * 8;
        crate::census::bump_slot(eo + 3, 1);
        crate::census::bump_slot(eo + 4, np);
        crate::census::bump_slot(eo + 5, sh_ns);
    }
    // same decision: both nonempty at the 1e-9 radius or both not, and for nonempty pieces the radius within 1e-6 relative
    let dc = cold_x.as_ref().map_or(false, |x| x[d1] > 1e-9);
    let ds = sol.as_ref().map_or(false, |x| x.x[d1] > 1e-9);
    if dc == ds && (!dc || (cold_x.as_ref().unwrap()[d1] - sol.as_ref().unwrap().x[d1]).abs() <= 1e-6 * (1.0 + cold_x.as_ref().unwrap()[d1].abs())) {
        crate::census::bump(C::ChebShiftSame, 1);
    } else if std::env::var("CHEB_DBG").is_ok() {
        eprintln!("cheb mismatch cold {:?} shifted {:?} rows {} d1 {}", cold_x.as_ref().map(|x| x[d1]), sol.as_ref().map(|x| x.x[d1]), ef.rows, d1);
    }
}

/// Census only: forgets the hyperplanes walked so far (call between repeated solves).
#[cfg(feature = "lp_census")]
pub fn census_reset_local() {
    HYPERS.with(|h| h.borrow_mut().clear());
}

// Census only: the box of the region (in theta) and the facet basis, set once per walk call, for `box_probe`.
#[cfg(feature = "lp_census")]
thread_local! {
    static BOXCTX: std::cell::RefCell<Option<(Vec<f64>, Vec<f64>, Vec<Vec<f64>>)>> = const { std::cell::RefCell::new(None) };
    static HYPERS: std::cell::RefCell<std::collections::HashMap<Vec<i64>, u8>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Census only: the extra probes (Chebyshev warm start, phase-1, row redundancy, region box) run only with CHEB_PROBE=1, so a plain
/// census run keeps clean timings.
#[cfg(feature = "lp_census")]
fn probes() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("CHEB_PROBE").is_ok())
}

#[cfg(feature = "lp_census")]
fn box_set(e_all: &Mat, f_all: &[f64], centre: Option<&[f64]>, nb: &[Vec<f64>]) {
    if !probes() {
        return;
    }
    let bx = centre.and_then(|c| crate::mpqp_utils::region_bbox(e_all, f_all, c));
    BOXCTX.with(|b| *b.borrow_mut() = bx.map(|(lo, hi)| (lo, hi, nb.to_vec())));
}

/// Census only: records the facet hyperplane (sign-canonical g, b); true when the hyperplane was walked before.
#[cfg(feature = "lp_census")]
fn census_hyper(g: &[f64], b: f64) -> bool {
    use crate::census::Counter as C;
    let s = g.iter().find(|v| v.abs() > 1e-9).map_or(1.0, |v| v.signum());
    let side: u8 = if s > 0.0 { 1 } else { 2 };
    let mut key: Vec<i64> = g.iter().map(|v| (s * v * 1e6).round() as i64).collect();
    key.push((s * b * 1e6).round() as i64);
    crate::census::bump(C::HyperWalks, 1);
    let prev = HYPERS.with(|h| {
        let mut h = h.borrow_mut();
        let e = h.entry(key).or_insert(0);
        let p = *e;
        *e |= side;
        p
    });
    if prev != 0 {
        crate::census::bump(C::HyperRepeat, 1);
        if prev & side == 0 {
            crate::census::bump(C::HyperOther, 1);
        }
    }
    prev != 0
}

/// Census only: would the region box decide this piece? Counts the test, the polytope rows, the rows redundant over the box, the rows
/// added to the parent piece (and the redundant ones among them); true when a row's minimum over the box exceeds its rhs (piece empty).
#[cfg(feature = "lp_census")]
fn box_probe(ef: &Mat, ff: &[f64], parent_rows: usize) -> bool {
    use crate::census::Counter as C;
    if !probes() {
        return false;
    }
    crate::census::bump(C::BoxTests, 1);
    crate::census::bump(C::BoxRows, ef.rows as u64);
    crate::census::bump(C::BoxCutRows, (ef.rows - parent_rows) as u64);
    BOXCTX.with(|b| {
        let b = b.borrow();
        let Some((lo, hi, nb)) = b.as_ref() else { return false };
        let (mut empty, mut red, mut redc) = (false, 0u64, 0u64);
        for i in 0..ef.rows {
            let row = ef.row(i);
            // row . theta_f = c . theta on the facet, c = sum_j row[j] nb[j]
            let (mut mn, mut mx) = (0.0, 0.0);
            for k in 0..lo.len() {
                let ck: f64 = row.iter().zip(nb).map(|(r, v)| r * v[k]).sum();
                mn += (ck * lo[k]).min(ck * hi[k]);
                mx += (ck * lo[k]).max(ck * hi[k]);
            }
            if mn > ff[i] + 1e-9 {
                empty = true;
            }
            if mx <= ff[i] {
                red += 1;
                if i >= parent_rows {
                    redc += 1;
                }
            }
        }
        crate::census::bump(C::BoxRedRows, red);
        crate::census::bump(C::BoxRedCut, redc);
        empty
    })
}

/// Records a returned basis with the Chebyshev centre (facet coordinates) of the sub-facet it was
/// reached on, and counts whether the old hint lies in that sub-facet (census).
fn push_leaf(fpoly: &Facet, z: Vec<usize>, centre: Option<&[f64]>, out: &mut Vec<(Vec<usize>, Option<Vec<f64>>)>) {
    #[cfg(feature = "lp_census")]
    {
        crate::census_t0!(cm);
        if centre.is_some() {
            crate::census_bump!(HintCentre, 1);
        }
        if let Some(h) = fpoly.hint_f {
            let inside = (0..fpoly.ef.rows).all(|i| fpoly.ef.row(i).iter().zip(h).map(|(a, b)| a * b).sum::<f64>() <= fpoly.ff[i] + 1e-9);
            if inside {
                crate::census_bump!(HintInside, 1);
            } else {
                crate::census_bump!(HintOutside, 1);
            }
        }
        crate::census_lap!(cm, HintCensus);
    }
    let _ = fpoly;
    out.push((z, centre.map(|c| c.to_vec())));
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
    /// z rows, and the Chebyshev centre (in parameter space) of the sub-facet the basis was reached on
    Z(Vec<usize>, Option<Vec<f64>>),
    /// the walk did not finish: a point just across the facet, to solve the QP at
    Across(Vec<f64>),
}

/// The rank-1 enumeration of `adjacent` (on by default; `set_rank1(false)` restores the walk).
static RANK1: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

pub fn set_rank1(on: bool) {
    RANK1.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// The facet polytope F = region ∩ Θ restricted to the hyperplane, in the coordinates θ = N θ_f - g b.
fn facet_polytope(nb: &[Vec<f64>], g: &[f64], b: f64, e_all: &Mat, f_all: &[f64], d1: usize) -> (Mat, Vec<f64>) {
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
            let eg: f64 = r.iter().zip(g).map(|(a, b)| a * b).sum();
            ef.push_row(&row.iter().map(|v| v / nr).collect::<Vec<_>>());
            ff.push((f_all[i] + b * eg) / nr);
        }
    }
    (ef, ff)
}

/// Case 2 with exactly one row c vanishing on the facet, a slack row (w_c = 0 on the facet) whose pivot
/// T[c,kk] (kk the column of its complement z_c) is zero. Then a_c = sum_j mu_j a_j over the active rows,
/// column kk holds -mu_j on the z rows and zero on every slack row, and the active set T = A + {c} has rank
/// deficiency one. Across the facet z_c rises and the multipliers move as lambda_j - mu_j z_c; the basis that
/// is lexicographically feasible drops j* = argmin lambda_j / mu_j over P- = {j : mu_j > 0}, unique at a generic
/// facet point. So the neighbours are A + {c} - {j}, one per j in P- whose piece F ∩ {lambda_j / mu_j <=
/// lambda_k / mu_k, k in P-} of the facet is nonempty (a witness, or the Chebyshev LP); P- empty means no
/// neighbour (boundary of the feasible set). A known set is dropped. A multiplier is zero, nonzero or undecided in the scale of the
/// dependency (`MU_ZERO`, `MU_NONZERO`, as the pivot measure); the undecided ones go to the walk (on the benchmark problems the band holds a few hundred to a few thousand multipliers per solve;
/// a gap-band multiplier is recomputed from the original data first, see `refined_multipliers`). Returns None when the case does not apply
/// (several vanishing rows, a multiplier row, a nonzero pivot or slack column, ties between two ratios as
/// affine functions on F, nt = 1), and the walk runs instead.
#[allow(clippy::too_many_arguments)]
fn rank1_case2(p: &Plcp, tab: &Tab, g: &[f64], b: f64, gam: &[f64], zrows: &[usize], pv: &[usize], e_all: &Mat, f_all: &[f64], seen: Option<&Seen>, region_centre: Option<&[f64]>, refine: bool) -> Option<Vec<Nbr>> {
    let (n, nt) = (p.n, p.nt);
    let d1 = nt - 1;
    crate::census_t0!(cm);
    if d1 == 0 || zrows.len() != 1 || pv.len() != 1 || pv[0] != zrows[0] {
        { crate::census_reason!(1); return None; }
    }
    let c = zrows[0];
    let var = tab.row_var[c];
    if var >= n {
        { crate::census_reason!(2); return None; }
    }
    let kk = tab.col_of(comp(var, n));
    // zero pivot and zero slack columns, in the scale-free measure (see `pivot_measure`)
    if pivot_part(p, tab, c, kk) > MEASURE_ZERO || pivot_measure(p, tab, c, kk) > MEASURE_ZERO {
        { crate::census_reason!(3); return None; }
    }
    // Hypotheses of the rank-1 lemma (Lean DependentFacet): row c of the tableau is w_c = rho_c + sum_j mu_j w_j with
    // no z column, and column kk is -mu_j on the active rows and 0 elsewhere (checked above for the slack rows).
    // Row c: zero on every nonbasic z column, and mu_j on the nonbasic w_j column of every active row j.
    for (col, &cv) in tab.col_var.iter().enumerate().take(n) {
        if cv >= n && cv < 2 * n {
            if tab.t[c * n + col].abs() > 1e-8 {
                { crate::census_reason!(4); return None; }
            }
        }
    }
    let zrows_all: Vec<usize> = (0..n).filter(|&i| tab.row_var[i] >= n && tab.row_var[i] < 2 * n).collect();
    #[cfg(feature = "lp_census")]
    let mu_max = zrows_all.iter().fold(0.0f64, |m, &i| m.max(tab.t[i * n + kk].abs()));
    let mcc = p.m[var * n + var].max(1e-300);
    // a multiplier in the undecided band of the tableau is recomputed from the original data (`refined_multipliers`); the call goes on only
    // when every such entry comes out as zero (noise) or clearly nonzero
    let mut mu_ref: Option<Vec<f64>> = None;
    if refine {
        let in_band = |i: usize| {
            let j = tab.row_var[i] - n;
            let nu = tab.t[i * n + kk].abs() / (mcc / p.m[j * n + j].max(1e-300)).sqrt();
            nu > MU_ZERO && nu <= MU_NONZERO
        };
        if zrows_all.iter().any(|&i| in_band(i)) {
            let zj: Vec<usize> = zrows_all.iter().map(|&i| tab.row_var[i] - n).collect();
            mu_ref = refined_multipliers(p, &zj, var);
        }
    }
    let mut pm: Vec<usize> = vec![];
    let mut pm_mu: Vec<f64> = vec![];
    let mut zeroed_pos: Vec<usize> = vec![];
    for (ai, &i) in zrows_all.iter().enumerate() {
        let mut mi = -tab.t[i * n + kk];
        let colw = tab.col_of(tab.row_var[i] - n);
        let j = tab.row_var[i] - n;
        // scale of a multiplier: a_c = sum mu_j a_j, so mu_j is in units of ||a_c|| / ||a_j|| = sqrt(M_cc / M_jj)
        let ujj = (mcc / p.m[j * n + j].max(1e-300)).sqrt();
        let m = mi.abs();
        let nu = m / ujj;
        // the rule before the scale-free tests, for the census
        #[cfg(feature = "lp_census")]
        {
            let old_mismatch = (tab.t[c * n + colw] - mi).abs() > 1e-8 * (1.0 + mi.abs());
            let old_small = m > PTOL && m <= 1e-7 * mu_max.max(1.0);
            let new_mismatch = (tab.t[c * n + colw] - mi).abs() > 1e-8 * (ujj + mi.abs());
            if old_mismatch && !new_mismatch {
                crate::census_bump!(MuMismatchFreed, 1);
            }
            if old_small && !new_mismatch && nu > MU_NONZERO && mi > 0.0 {
                crate::census_bump!(MuAccepted, 1);
            }
            if old_small && !new_mismatch && nu <= MU_ZERO {
                crate::census_bump!(MuNoiseZero, 1);
            }
            if !(old_small || old_mismatch) && mi > 0.0 && mi <= PTOL && nu > MU_NONZERO {
                crate::census_bump!(MuNewPositive, 1);
            }
            if !new_mismatch && nu > MU_ZERO && nu <= MU_NONZERO {
                crate::census_bump!(MuGap, 1);
            }
        }
        if (tab.t[c * n + colw] - mi).abs() > 1e-8 * (ujj + mi.abs()) {
            { crate::census_reason!(5); return None; }
        }
        let nu_tab = nu;
        let mut nu = nu;
        if nu > MU_ZERO && nu <= MU_NONZERO {
            let Some(v) = &mu_ref else {
                // neither zero nor safely nonzero, in the scale of the dependency: the walk decides
                #[cfg(feature = "lp_census")]
                refine_probe(p, tab, &zrows_all, var, kk, mcc);
                { crate::census_reason!(6); return None; }
            };
            mi = v[ai];
            nu = mi.abs() / ujj;
            if nu > REFINED_ZERO && nu < MU_NONZERO {
                crate::census_bump!(MuRefinedBand, 1);
                { crate::census_reason!(6); return None; }
            }
            if nu <= REFINED_ZERO {
                crate::census_bump!(MuRefinedZero, 1);
            } else {
                crate::census_bump!(MuRefinedReal, 1);
            }
        }
        if mi > 0.0 && nu > MU_NONZERO {
            pm.push(i);
            pm_mu.push(mi);
        } else if mi > 0.0 && nu_tab > 0.0 && nu_tab <= MU_ZERO {
            // a positive tableau multiplier decided zero without a recomputation (a rounded zero, or a tiny true positive)
            zeroed_pos.push(ai);
        }
    }
    // Safeguard: when no multiplier is positive but some positive one was decided zero, "no neighbour across the facet" would be wrong if one of them is a
    // true tiny multiplier (it would be the only member). The recomputation from the original data decides: all exactly zero (rounding level) means a
    // boundary facet, anything else goes to the walk.
    if pm.is_empty() && !zeroed_pos.is_empty() {
        crate::census_bump!(MuEmptyGuard, 1);
        let zj: Vec<usize> = zrows_all.iter().map(|&i| tab.row_var[i] - n).collect();
        let exact_zero = refined_multipliers(p, &zj, var).map_or(false, |v| {
                zeroed_pos.iter().all(|&ai| {
                    let j = zj[ai];
                    v[ai].abs() / (mcc / p.m[j * n + j].max(1e-300)).sqrt() <= REFINED_ZERO
                })
            });
        if !exact_zero {
            crate::census_bump!(MuEmptyToWalk, 1);
            { crate::census_reason!(9); return None; }
        }
    }
    let mu: Vec<f64> = pm_mu;
    if pm.is_empty() {
        crate::census_bump!(R1Handled, 1);
        return Some(vec![]);
    }
    // the members A + {c} - {j}, minus the known ones
    let zc = z_rows(tab, n);
    let mut members: Vec<(usize, Vec<usize>)> = vec![];
    for (a, &ia) in pm.iter().enumerate() {
        let zj = tab.row_var[ia] - n;
        let mut z: Vec<usize> = zc.iter().cloned().filter(|&x| x != zj).collect();
        z.push(var);
        z.sort_unstable();
        z.dedup();
        let known = seen.map_or(false, |s| s.contains_key(&p.active_set(z.iter().copied())));
        crate::census_lap!(cm, Seen);
        if known {
            crate::census_bump!(R1SkipSeen, 1);
            continue;
        }
        members.push((a, z));
    }
    crate::census_skip!(cm);
    // lambda_i on the facet, in facet coordinates: (rr0 - b gam_i, N' rr)
    let nb = null_basis(g);
    let w = 1 + d1;
    let lam: Vec<Vec<f64>> = pm
        .iter()
        .map(|&i| {
            let rr = &tab.rho[i * tab.w..(i + 1) * tab.w];
            let mut v = vec![rr[0] - b * gam[i]];
            for vv in nb.iter() {
                v.push((0..nt).map(|k| rr[1 + k] * vv[k]).sum());
            }
            v
        })
        .collect();
    // mu_k lambda_j - mu_j lambda_k as (g0, g_q) for the pair (a, b)
    let cut = |a: usize, bb: usize| -> Vec<f64> { (0..w).map(|k| mu[bb] * lam[a][k] - mu[a] * lam[bb][k]).collect() };
    let scale = |a: usize, bb: usize| 1.0 + (0..w).fold(0.0f64, |m, k| m.max((mu[bb] * lam[a][k]).abs()).max((mu[a] * lam[bb][k]).abs()));
    for a in 0..pm.len() {
        for bb in a + 1..pm.len() {
            let d = cut(a, bb);
            if d.iter().all(|v| v.abs() <= ZTOL * scale(a, bb)) {
                { crate::census_reason!(7); return None; } // two ratios identical on F: the lexicographic tie-break decides, the walk handles it
            }
        }
    }
    crate::census_bump!(R1Handled, 1);
    crate::census_lap!(cm, R1Entry);
    if members.is_empty() {
        return Some(vec![]);
    }
    let (ef, ff) = facet_polytope(&nb, g, b, e_all, f_all, d1);
    let to_theta = |cf: &[f64]| -> Vec<f64> { (0..nt).map(|k| nb.iter().zip(cf).map(|(v, x)| v[k] * x).sum::<f64>() - g[k] * b).collect() };
    // the region centre projected onto the facet, in facet coordinates, as a first witness candidate
    let hint_f: Option<Vec<f64>> = region_centre.map(|c| {
        // g is the unit normal of the row: the projection onto the hyperplane g.theta + b = 0
        let sft = -(g.iter().zip(c).map(|(x, y)| x * y).sum::<f64>() + b);
        let h: Vec<f64> = c.iter().zip(g).map(|(ci, gi)| ci + sft * gi).collect();
        nb.iter().map(|v| h.iter().zip(v).map(|(a, b)| a * b).sum()).collect()
    });
    let mut witnesses: Vec<Vec<f64>> = hint_f.into_iter().collect();
    let mut out = vec![];
    crate::census_lap!(cm, R1Poly);
    for (a, z) in members {
        crate::census_skip!(cm);
        let mut e2 = ef.clone();
        let mut f2 = ff.clone();
        let mut empty = false;
        for bb in 0..pm.len() {
            if bb == a {
                continue;
            }
            // lambda_j / mu_j <= lambda_k / mu_k  <=>  mu_k lambda_j - mu_j lambda_k <= 0
            let d = cut(a, bb);
            let nrm = d[1..].iter().map(|v| v * v).sum::<f64>().sqrt();
            let sc = scale(a, bb);
            if nrm <= ZTOL * sc {
                if d[0] > ZTOL * sc {
                    empty = true;
                    break;
                } else if d[0] >= -ZTOL * sc {
                    { crate::census_reason!(8); return None; } // a tie as a constant: the walk decides
                }
                continue;
            }
            e2.push_row(&d[1..].iter().map(|v| v / nrm).collect::<Vec<_>>());
            f2.push(-d[0] / nrm);
        }
        crate::census_lap!(cm, R1Cuts);
        if empty {
            crate::census_bump!(R1EmptyMember, 1);
            #[cfg(feature = "lp_census")]
            crate::census::tested(&z, true);
            crate::census_lap!(cm, Census);
            continue;
        }
        let clears = |pt: &[f64]| (0..e2.rows).all(|i| f2[i] - e2.row(i).iter().zip(pt).map(|(x, y)| x * y).sum::<f64>() >= WITNESS_MARGIN);
        let wfound = witnesses.iter().find(|pt| clears(pt)).cloned();
        crate::census_lap!(cm, Wit);
        if let Some(pt) = wfound {
            crate::census_bump!(R1WitnessAcc, 1);
            crate::census_bump!(R1Emitted, 1);
            #[cfg(feature = "lp_census")]
            crate::census::tested(&z, false);
            crate::census_lap!(cm, Census);
            out.push(Nbr::Z(z, Some(to_theta(&pt))));
            crate::census_lap!(cm, R1Out);
            continue;
        }
        let dq = daqp_certainly_empty(&e2, &f2, d1, p.coord_bound);
        crate::census_lap!(cm, Daqp);
        if dq {
            crate::census_bump!(R1EmptyMember, 1);
            crate::census_bump!(PfChebEmpty, 1);
            #[cfg(feature = "lp_census")]
            crate::census::tested(&z, true);
            crate::census_lap!(cm, Census);
            continue;
        }
        crate::census_bump!(R1LpRun, 1);
        crate::census_bump!(PfCheb, 1);
        let ball = {
            crate::census_purpose!(Neighbour);
            chebyshev_ball(&e2, &f2, &[])
        };
        crate::census_lap!(cm, Lp);
        match ball {
            Some(s) if s.x[d1] > 1e-9 => {
                let cf = s.x[..d1].to_vec();
                crate::census_bump!(R1Emitted, 1);
                #[cfg(feature = "lp_census")]
                crate::census::tested(&z, false);
                crate::census_lap!(cm, Census);
                out.push(Nbr::Z(z, Some(to_theta(&cf))));
                witnesses.push(cf);
                crate::census_lap!(cm, R1Out);
            }
            _ => {
                crate::census_bump!(R1EmptyMember, 1);
                crate::census_bump!(PfChebEmpty, 1);
                #[cfg(feature = "lp_census")]
                crate::census::tested(&z, true);
                crate::census_lap!(cm, Census);
            }
        }
    }
    Some(out)
}

/// Scale-free size of the pivot of exchanging the vanishing slack row `c` with its complement (column `kk`):
/// the larger of the pivot |T[c,kk]| / M_cc and the slack-column entries |T[i,kk]| / sqrt(M_ii M_cc) over the other
/// slack rows (M the pLCP matrix, so the diagonal is the scale of each constraint). The pivot of an exact linear
/// dependence of constraint c on the active rows has measure 0 up to rounding (observed below 4e-10), and any other
/// pivot has measure of at least 1.7e-5 on the masses, POP and random problems measured; M entries range from 5e-13
/// to 5e2, so absolute thresholds (PTOL on the pivot, 1e-8 on the column) called a tiny but nonzero pivot zero.
fn pivot_measure(p: &Plcp, tab: &Tab, c: usize, kk: usize) -> f64 {
    let n = p.n;
    let vc = tab.row_var[c];
    let mcc = p.m[vc * n + vc].max(1e-300);
    let mut m = pivot_part(p, tab, c, kk);
    for i in 0..n {
        let vi = tab.row_var[i];
        if i != c && vi < n {
            m = m.max(tab.t[i * n + kk].abs() / (p.m[vi * n + vi].max(1e-300) * mcc).sqrt());
        }
    }
    m
}

/// The pivot part |T[c,kk]| / M_cc of `pivot_measure`, O(1). The measure is the maximum of this and the column part, so
/// a pivot part above a threshold already puts the measure above it and the O(n) column loop can be skipped.
fn pivot_part(p: &Plcp, tab: &Tab, c: usize, kk: usize) -> f64 {
    let n = p.n;
    let vc = tab.row_var[c];
    tab.t[c * n + kk].abs() / p.m[vc * n + vc].max(1e-300)
}

/// Census only: recomputes the multipliers of a gap-band call from the original data and records how the refined values fall against the
/// tableau's (entries in the band, the decade of the refined nu, calls fully decided, time, and agreement on the large entries).
#[cfg(feature = "lp_census")]
fn refine_probe(p: &Plcp, tab: &Tab, zrows_all: &[usize], var: usize, kk: usize, mcc: f64) {
    use crate::census::{bump_slot, MU2_BASE};
    let n = p.n;
    let zj: Vec<usize> = zrows_all.iter().map(|&i| tab.row_var[i] - n).collect();
    let t0 = std::time::Instant::now();
    let Some(mu) = refined_multipliers(p, &zj, var) else { return };
    bump_slot(MU2_BASE + 6, t0.elapsed().as_nanos() as u64);
    bump_slot(MU2_BASE, 1);
    let mut all_decided = true;
    for (a, &i) in zrows_all.iter().enumerate() {
        let j = zj[a];
        let u = (mcc / p.m[j * n + j].max(1e-300)).sqrt();
        let nu_t = (-tab.t[i * n + kk]).abs() / u;
        let nu_r = mu[a].abs() / u;
        let dec = if nu_r > 0.0 { ((-nu_r.log10()).floor().max(0.0) as usize).min(15) } else { 15 };
        if nu_t > MU_ZERO && nu_t <= MU_NONZERO {
            bump_slot(MU2_BASE + 1, 1);
            bump_slot(MU2_BASE + 9 + dec, 1);
            if nu_r <= 1e-12 {
                bump_slot(MU2_BASE + 2, 1);
            } else if nu_r >= MU_NONZERO {
                bump_slot(MU2_BASE + 3, 1);
            } else {
                bump_slot(MU2_BASE + 4, 1);
                all_decided = false;
            }
        } else if nu_t >= 1e-4 {
            // agreement on the large entries: sign and relative value of mu = -T[i,kk]
            bump_slot(MU2_BASE + 7, 1);
            let mt = -tab.t[i * n + kk];
            if mt * mu[a] <= 0.0 || (mt - mu[a]).abs() > 1e-6 * mt.abs() {
                bump_slot(MU2_BASE + 8, 1);
            }
        } else if nu_t <= MU_ZERO && nu_r > 1e-12 {
            // a tableau zero that the refined value does not call zero
            bump_slot(MU2_BASE + 25, 1);
        }
    }
    if all_decided {
        bump_slot(MU2_BASE + 5, 1);
    }
}

/// The multipliers of the dependency a_c = sum_j mu_j a_j over the rows `zj` (constraint indices of the active z rows), recomputed from
/// the original data instead of read from the tableau: the Gram system M_ZZ mu = M_Zc (M = A Q^-1 A'), then `REFINE_STEPS` steps of
/// iterative refinement whose residual r = a_c - sum_j mu_j a_j is formed from the rows of A themselves and whose correction solves
/// M_ZZ delta = (A_Z Q^-1) r. `None` with equality rows (the data is then reduced) or a singular Gram matrix.
fn refined_multipliers(p: &Plcp, zj: &[usize], c: usize) -> Option<Vec<f64>> {
    if p.arows.rows == 0 || zj.is_empty() {
        return None;
    }
    let (n, k) = (p.n, zj.len());
    // the Gram matrix is equilibrated to a unit diagonal: the rows of A differ in scale by 1e3 and more, and the library LU judges singularity
    // against the largest entry of the whole matrix
    let sc: Vec<f64> = zj.iter().map(|&j| p.m[j * n + j].max(1e-300).sqrt()).collect();
    let mut g = Mat::zeros(k, k);
    for (a, &ja) in zj.iter().enumerate() {
        for (b, &jb) in zj.iter().enumerate() {
            g[(a, b)] = p.m[ja * n + jb] / (sc[a] * sc[b]);
        }
    }
    let lu = crate::linalg::Lu::new(&g)?;
    let rhs: Vec<f64> = zj.iter().enumerate().map(|(a, &j)| p.m[j * n + c] / sc[a]).collect();
    let mut mu: Vec<f64> = lu.solve(&rhs).iter().zip(&sc).map(|(y, s)| y / s).collect();
    let nx = p.arows.cols;
    let mut r = vec![0.0; nx];
    for _ in 0..REFINE_STEPS {
        r.copy_from_slice(p.arows.row(c));
        for (a, &ja) in zj.iter().enumerate() {
            let row = p.arows.row(ja);
            for (rv, x) in r.iter_mut().zip(row) {
                *rv -= mu[a] * x;
            }
        }
        let corr: Vec<f64> = zj.iter().enumerate().map(|(a, &j)| p.aqi.row(j).iter().zip(&r).map(|(x, y)| x * y).sum::<f64>() / sc[a]).collect();
        let delta = lu.solve(&corr);
        for ((m, d), s) in mu.iter_mut().zip(&delta).zip(&sc) {
            *m += d / s;
        }
    }
    mu.iter().all(|v| v.is_finite()).then_some(mu)
}

/// Is the set of constraints (original indices, no equality rows) linearly dependent, judged on the equilibrated Gram matrix of their
/// rows? Such an active set violates LICQ and cannot be the active set of a region.
#[cfg(feature = "lp_census")]
fn dependent_set(p: &Plcp, aset: &[usize]) -> bool {
    let rows: Vec<usize> = aset.iter().map(|&i| p.pos[i]).collect();
    let n = p.n;
    let sc: Vec<f64> = rows.iter().map(|&j| p.m[j * n + j].max(1e-300).sqrt()).collect();
    let mut g = Mat::zeros(rows.len(), rows.len());
    for (a, &ja) in rows.iter().enumerate() {
        for (b, &jb) in rows.iter().enumerate() {
            g[(a, b)] = p.m[ja * n + jb] / (sc[a] * sc[b]);
        }
    }
    crate::linalg::Lu::new(&g).is_none()
}

const REFINE_STEPS: usize = 2;
/// A refined multiplier of this size or below (in the scale of the dependency) is zero: an exact dependency comes out at rounding level 1e-15.
const REFINED_ZERO: f64 = 1e-12;
/// Switch for the refined classification of gap-band multipliers (on by default).
/// Census audit (`MPLCP_MU_AUDIT=1`): gap-band calls are answered by the refined enumeration and by the walk, and the neighbour sets compared.
#[cfg(feature = "lp_census")]
static MU_AUDIT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(feature = "lp_census")]
pub fn set_mu_audit(on: bool) {
    MU_AUDIT.store(on, std::sync::atomic::Ordering::Relaxed);
}
static REFINE_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
pub fn set_refine_multipliers(on: bool) {
    REFINE_ON.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Measure at or below which the pivot is zero (rank-1 case), and above which it is nonzero (Case 1). Between them
/// the walk decides; `PivotGap` counts those calls.
/// The same thresholds for a multiplier mu_j of the dependency a_c = sum mu_j a_j, in the scale sqrt(M_cc / M_jj): zero below
/// `MU_ZERO`, nonzero above `MU_NONZERO`, left to the walk between them.
const MU_ZERO: f64 = 1e-9;
const MU_NONZERO: f64 = 1e-6;
const MEASURE_ZERO: f64 = 1e-9;
const MEASURE_NONZERO: f64 = 1e-6;

/// Neighbouring z-row sets across the facet {e_k θ = f_k} of the region with tableau `tab`.
///
/// `cert` is asked only when the fast path has failed, that is, when the slow path (Case 2) would
/// run: if it names a known region R that is the only one across the facet, the search is skipped
/// (unless `audit`, which runs it anyway). The third result is that region.
fn adjacent(p: &Plcp, tab: &Tab, erow: &[f64], frhs: f64, e_all: &Mat, f_all: &[f64], ws: &mut Option<Tab>, cert: Option<&dyn Fn() -> Option<usize>>, audit: bool, seen: Option<&Seen>, region_centre: Option<&[f64]>) -> (Vec<Nbr>, Option<usize>) {
    #[cfg(feature = "lp_census")]
    let _cen_t = std::time::Instant::now();
    #[cfg(feature = "lp_census")]
    let cm0 = crate::census::Mark::now();
    crate::census_t0!(cm);
    #[cfg(feature = "lp_census")]
    crate::census::c2_reset();
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
    crate::census_lap!(cm, Pre);
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
        // Single vanishing slack row: the scale-free measure decides in both directions. A pivot below the
        // absolute PTOL is still nonzero when it is not small against the scale of M (`nonzero_rel`), and a
        // pivot above PTOL is rounding of an exact dependence when the measure is at most MEASURE_ZERO: that
        // facet is a rank-1 Case 2 and not a Case-1 toggle (`demoted`).
        let single = zrows.len() == 1 && var < n && tve > 0.0;
        // the O(n) column loop is needed only when the decision is not already fixed by the pivot part: above
        // MEASURE_ZERO a call with tve > PTOL cannot be demoted, above MEASURE_NONZERO a call with tve <= PTOL is
        // nonzero; the census build always computes the full measure so that `PivotGap` counts exactly
        let m = if !single {
            f64::INFINITY
        } else {
            let pp = pivot_part(p, tab, v, kk);
            let decided = if tve > PTOL { pp > MEASURE_ZERO } else { pp > MEASURE_NONZERO };
            if decided && !cfg!(feature = "lp_census") {
                pp
            } else {
                pivot_measure(p, tab, v, kk)
            }
        };
        if single && m > MEASURE_ZERO && m <= MEASURE_NONZERO {
            crate::census_bump!(PivotGap, 1);
        }
        let nonzero_rel = single && tve <= PTOL && m > MEASURE_NONZERO;
        let demoted = single && tve > PTOL && m <= MEASURE_ZERO;
        if nonzero_rel {
            crate::census_bump!(FastRel, 1);
        }
        if demoted {
            crate::census_bump!(FastDemoted, 1);
        }
        if (tve > PTOL && !demoted) || nonzero_rel {
            let blocked = zrows.iter().any(|&i| i != v && -(tab.t[i * n + kk] + gam[i] * tve) > PTOL);
            if !blocked {
                crate::census_bump!(AdjFast, 1);
                crate::census_bump!(AdjFastNanos, _cen_t.elapsed().as_nanos());
                if nonzero_rel {
                    // a pivot below PTOL leaves a badly scaled child tableau and an entry hyperplane that
                    // the pivot step cannot be trusted with: hand over the basis only, so the child is
                    // factored from its active set (measured on masses8_N2: keeping the pivoted tableau
                    // cost 1.5x main, this costs 0.86x)
                    return (vec![Nbr::Z(z_toggle(tab, n, v), None)], None);
                }
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
    crate::census_lap!(cm, Gate);
    // Rank-1 Case 2: one vanishing slack row c with a zero pivot, a_c = sum mu_j a_j over the active rows.
    // The neighbours are enumerated directly, no walk (see `rank1_case2`).
    #[cfg(feature = "lp_census")]
    let mut audit_alt: Option<Option<Vec<Nbr>>> = None;
    if RANK1.load(std::sync::atomic::Ordering::Relaxed) {
        #[cfg(feature = "lp_census")]
        let snap = crate::census::c2_snapshot();
        let refine = REFINE_ON.load(std::sync::atomic::Ordering::Relaxed);
        #[cfg(feature = "lp_census")]
        let audit = MU_AUDIT.load(std::sync::atomic::Ordering::Relaxed);
        #[cfg(not(feature = "lp_census"))]
        let audit = false;
        if let Some(nbrs) = rank1_case2(p, tab, g.as_slice(), b, &gam, &zrows, &pv, e_all, f_all, seen, region_centre, refine && !audit) {
            crate::census_bump!(AdjCase2, 1);
            crate::census_bump!(AdjCase2Nanos, _cen_t.elapsed().as_nanos());
            #[cfg(feature = "lp_census")]
            crate::census::c2_commit(2, &cm0, 0);
            return (nbrs, certified);
        }
        crate::census_bump!(R1Fallback, 1);
        // audit: a call refused for a gap-band multiplier is also answered by the refined enumeration, and compared with the walk below
        #[cfg(feature = "lp_census")]
        if audit {
            let reason = crate::census::reason_take();
            if reason == 6 {
                audit_alt = Some(rank1_case2(p, tab, g.as_slice(), b, &gam, &zrows, &pv, e_all, f_all, seen, region_centre, true));
                if audit_alt.as_ref().map_or(false, |x| x.is_none()) && std::env::var("AUDIT_DBG").is_ok() {
                    eprintln!("audit alt none reason {}", crate::census::reason_take());
                }
            }
            crate::census::reason_set(reason);
        }
        // the failed attempt is walk overhead: its inner laps are dropped, its whole time is one lap
        #[cfg(feature = "lp_census")]
        crate::census::c2_restore(snap);
        crate::census_lap!(cm, R1Try);
    }
    #[cfg(feature = "lp_census")]
    let (r1_class, r1_kk, r1_pminus, r1_mu, r1_c, r1_overhead) = {
        let t0 = std::time::Instant::now();
        let (cl, kk, pm, mu, cc) = rank1_probe(p, tab, &zrows, &pv, &gam);
        (cl, kk, pm, mu, cc, t0.elapsed().as_nanos())
    };
    crate::census_lap!(cm, Census);
    #[cfg(feature = "lp_census")]
    crate::census::depth_reset();
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
    crate::census_lap!(cm, Setup);
    #[cfg(feature = "lp_census")]
    crate::census::basis_reset();
    #[cfg(feature = "lp_census")]
    let r1_overhead = if r1_class == 0 { r1_overhead + rank1_census(p, tab, &work, &ef, &ff, d1, seen, &r1_pminus, &r1_mu, r1_c, r1_kk) } else { r1_overhead };
    crate::census_lap!(cm, Census);
    // the old hint (parent centre on the row) in facet coordinates: N' hint (N is orthogonal to g)
    let hint_f: Option<Vec<f64>> = region_centre.map(|c| {
        let sft = frhs - erow.iter().zip(c).map(|(x, y)| x * y).sum::<f64>();
        let h: Vec<f64> = c.iter().zip(erow).map(|(ci, ai)| ci + sft * ai).collect();
        nb.iter().map(|v| h.iter().zip(v).map(|(a, b)| a * b).sum()).collect()
    });
    // Pivot budget of one facet walk. Finiteness: from a lexico-feasible start no state repeats (Lean
    // walk_no_repeat); in general a repeated state must be the state right after the first pivot
    // (walk_repeat_returns), which the cycle check catches. The number of states is the number of reachable
    // label configurations, about n 2^n (chain_length_bound), which is no use as a cost limit: walks of
    // thousands of pivots exist and never find a region the shorter ones miss (measured on masses3_N3 to
    // masses8_N2, pop_005, rand_t12 and rand_t14: 30 to 20,000 pivots give the same regions apart from 3 on
    // pop_005, while the time grows 10-fold at 300 and 300-fold at 3,000). 2n pivots (at least 64) keeps the
    // cost of the old caps, and a spent budget is not silent: the facet also gets the QP just across it.
    let budget = (WALK_BUDGET_PER_ROW * n).max(WALK_BUDGET_MIN);
    crate::census_lap!(cm, Setup);
    #[cfg(feature = "lp_census")]
    let r1_reason = crate::census::reason_take();
    #[cfg(feature = "lp_census")]
    let wk_class = {
        let k = zrows.len();
        let nsl = zrows.iter().filter(|&&i| tab.row_var[i] < n).count();
        match (k, nsl) {
            (1, 1) => {
                let c = zrows[0];
                let kk = tab.col_of(comp(tab.row_var[c], n));
                let m = pivot_measure(p, tab, c, kk);
                if m <= MEASURE_ZERO {
                    // the rank-1 enumeration refused a zero-pivot call: 8 a nonzero z column of row c, 9 symmetry mismatch, 10 multiplier in the gap
                    // band, 11 a tie between two ratios, 0 anything else (rank-1 not tried)
                    match r1_reason {
                        4 => 8,
                        5 => 9,
                        6 => 10,
                        7 | 8 => 11,
                        _ => 0,
                    }
                } else if m <= MEASURE_NONZERO {
                    1
                } else {
                    2
                }
            }
            (1, _) => 3,
            (2, 2) => 4,
            (2, 1) => 5,
            (2, _) => 6,
            _ => 7,
        }
    };
    #[cfg(feature = "lp_census")]
    let hyper_rep = census_hyper(&g, b);
    #[cfg(feature = "lp_census")]
    box_set(e_all, f_all, region_centre, &nb);
    crate::census_lap!(cm, Census);
    let mut lost: Vec<Vec<f64>> = vec![];
    let complete = pivot_facet(p, &mut work, n, Facet { ef: &ef, ff: &ff, sc: &rowsc, seen, hint_f: hint_f.as_deref() }, budget, &mut out, &mut lost);
    *ws = Some(work);
    crate::census_skip!(cm);
    crate::census_bump!(AdjCase2, 1);
    crate::census_bump!(AdjCase2Nanos, _cen_t.elapsed().as_nanos());
    #[cfg(feature = "lp_census")]
    {
        use crate::census::Counter as C;
        let calls = [C::R1Calls0, C::R1Calls1, C::R1Calls2, C::R1Calls3, C::R1Calls4, C::R1Calls5];
        let nanos = [C::R1Nanos0, C::R1Nanos1, C::R1Nanos2, C::R1Nanos3, C::R1Nanos4, C::R1Nanos5];
        crate::census::bump(calls[r1_class], 1);
        crate::census::bump(nanos[r1_class], (_cen_t.elapsed().as_nanos().saturating_sub(r1_overhead)) as u64);
        if r1_class == 0 {
            let d = crate::census::depth_take();
            let dc = match d {
                0 => C::R1D0,
                1 => C::R1D1,
                2 => C::R1D2,
                3..=5 => C::R1D3to5,
                _ => C::R1D6p,
            };
            crate::census::bump(dc, 1);
        }
    }
    // theta = N theta_f - g b
    let nbrs = out
        .into_iter()
        .map(|(z, c)| {
            let c = c.map(|cf| (0..nt).map(|k| nb.iter().zip(&cf).map(|(v, x)| v[k] * x).sum::<f64>() - g[k] * b).collect::<Vec<f64>>());
            Nbr::Z(z, c)
        })
        .collect();
    let mut nbrs: Vec<Nbr> = nbrs;
    #[cfg(feature = "lp_census")]
    if let Some(alt) = audit_alt {
        use std::collections::BTreeSet;
        crate::census_bump!(AuditCalls, 1);
        match alt {
            Some(alt) if complete => {
                let set = |v: &[Nbr]| -> BTreeSet<Vec<usize>> { v.iter().filter_map(|nb| if let Nbr::Z(z, _) = nb { Some(p.active_set(z.iter().copied())) } else { None }).collect() };
                // the walk does not drop every known set (a Case-2 branch ending at once returns it), the enumeration does; the caller
                // drops known sets either way, so they are removed from the walk's side before comparing
                let (a, w) = (set(&alt), set(&nbrs).into_iter().filter(|z| !seen.map_or(false, |sn| sn.contains_key(z))).collect::<BTreeSet<_>>());
                // the sets the walk may add: A + {c} - {j} for a row j whose refined multiplier is zero (the set is then linearly dependent)
                let c = zrows[0];
                let var = tab.row_var[c];
                let zall: Vec<usize> = (0..p.n).filter(|&i| tab.row_var[i] >= p.n && tab.row_var[i] < 2 * p.n).collect();
                let zjs: Vec<usize> = zall.iter().map(|&i| tab.row_var[i] - p.n).collect();
                let mut p_real = 0usize;
                if let Some(mu_r) = refined_multipliers(p, &zjs, var) {
                    let mcc = p.m[var * p.n + var].max(1e-300);
                    for (k, &zj) in zjs.iter().enumerate() {
                        let u = (mcc / p.m[zj * p.n + zj].max(1e-300)).sqrt();
                        if mu_r[k] > 0.0 && mu_r[k].abs() / u >= MU_NONZERO {
                            p_real += 1;
                        }
                    }
                }
                if a == w {
                    crate::census_bump!(AuditSame, 1);
                } else if a.is_subset(&w) && w.difference(&a).all(|z| dependent_set(p, z)) {
                    // every extra set of the walk has linearly dependent rows (it removes a row whose multiplier is zero, or descends from such a set):
                    // not an active set of any region
                    crate::census_bump!(AuditExplained, 1);
                    let _ = p_real;
                } else if a.is_subset(&w) && p_real == 0 {
                    // no multiplier is positive in exact arithmetic (no neighbour across the facet), yet the walk, driven by a positive noise multiplier,
                    // wandered through further bases
                    crate::census_bump!(AuditNoPositive, 1);
                } else if a.is_subset(&w) {
                    // the walk has sets the enumeration lacks, though a real positive multiplier exists
                    crate::census_bump!(AuditExtraWalk, 1);
                    if std::env::var("AUDIT_DBG").is_ok() {
                        eprintln!("audit EXTRA-WALK: refined {:?} walk-only {:?}", a, w.difference(&a).collect::<Vec<_>>());
                        if let Some(mu_r) = refined_multipliers(p, &zjs, var) {
                            let mcc = p.m[var * p.n + var].max(1e-300);
                            for (k, &zj) in zjs.iter().enumerate() {
                                let u = (mcc / p.m[zj * p.n + zj].max(1e-300)).sqrt();
                                eprintln!("   j={} refined mu {:.3e} nu {:.2e}", zj, mu_r[k], mu_r[k].abs() / u);
                            }
                        }
                    }
                } else {
                    // the enumeration has a set the walk lacks
                    crate::census_bump!(AuditDiff, 1);
                    if std::env::var("AUDIT_DBG").is_ok() {
                        eprintln!("audit diff: refined-only {:?} walk-only {:?}", a.difference(&w).collect::<Vec<_>>(), w.difference(&a).collect::<Vec<_>>());
                        if let Some(mu_r) = refined_multipliers(p, &zjs, var) {
                            let mcc = p.m[var * p.n + var].max(1e-300);
                            for (k, &zj) in zjs.iter().enumerate() {
                                let u = (mcc / p.m[zj * p.n + zj].max(1e-300)).sqrt();
                                let mut z: Vec<usize> = zjs.iter().cloned().filter(|&x| x != zj).collect();
                                z.push(var);
                                let aset = p.active_set(z.into_iter());
                                let tag = if w.contains(&aset) && !a.contains(&aset) { "WALK-ONLY" } else if a.contains(&aset) { "both" } else { "-" };
                                eprintln!("   j={} (constraint {}) refined mu {:.3e} nu {:.2e} tableau nu {:.2e} member-set {}", zj, p.ineq[zj], mu_r[k], mu_r[k].abs() / u, tab.t[zall[k] * p.n + tab.col_of(comp(var, p.n))].abs() / u, tag);
                            }
                        }
                    }
                }
            }
            None => crate::census_bump!(AuditAltNone, 1),
            Some(_) => crate::census_bump!(AuditSkip, 1),
        }
    }
    crate::census_lap!(cm, Post);
    if !complete {
        // a repeated state or the budget stopped the walk: also look across the facet by a QP at a
        // point just beyond its Chebyshev centre (resolved by the caller, which has the program)
        // The probes: the centre of the whole facet, and the centre (or witness) of every sub-facet whose
        // walk was stopped or never started, so that the neighbours across those are not lost either
        // (at most 16 points, so the cost stays negligible).
        let sol = if d1 > 0 { chebyshev_ball(&ef, &ff, &[]) } else { None };
        let cf: Vec<f64> = sol.map_or(vec![0.0; d1], |s| s.x[..d1].to_vec());
        let mut probes: Vec<Vec<f64>> = vec![cf];
        for pt in lost {
            if probes.len() < 16 && !probes.iter().any(|q| q.iter().zip(&pt).all(|(a, b)| (a - b).abs() <= 1e-9)) {
                probes.push(pt);
            }
        }
        for cf in probes {
            let on: Vec<f64> = (0..nt).map(|k| nb.iter().zip(&cf).map(|(v, x)| v[k] * x).sum::<f64>() - g[k] * b).collect();
            let delta = 1e-6 * (1.0 + on.iter().fold(0.0f64, |m, v| m.max(v.abs())));
            nbrs.push(Nbr::Across(on.iter().zip(&g).map(|(x, gk)| x + delta * gk).collect()));
        }
    }
    crate::census_lap!(cm, Post);
    #[cfg(feature = "lp_census")]
    crate::census::c2_commit(3, &cm0, wk_class);
    #[cfg(feature = "lp_census")]
    {
        crate::census::bump(crate::census::Counter::HyperNs, _cen_t.elapsed().as_nanos() as u64);
        if hyper_rep {
            crate::census::bump(crate::census::Counter::HyperRepeatNs, _cen_t.elapsed().as_nanos() as u64);
        }
    }
    (nbrs, certified)
}

/// Census only: classifies a Case-2 call by the number and type of the rows that vanish on the facet and
/// returns, for the rank-1 class, the column kk of the complement of the vanishing row, the rows P- (z rows with
/// -T[i,kk] > PTOL), their multipliers mu = -T[i,kk] and the vanishing row. Classes: 0 one slack row with a zero
/// pivot and zero slack columns, 1 the same with a nonzero slack column, 2 one slack row otherwise, 3 one
/// multiplier row, 4 two rows, 5 three or more.
#[cfg(feature = "lp_census")]
fn rank1_probe(p: &Plcp, tab: &Tab, zrows: &[usize], pv: &[usize], gam: &[f64]) -> (usize, usize, Vec<usize>, Vec<f64>, usize) {
    let n = p.n;
    match zrows.len() {
        0 => (5, 0, vec![], vec![], 0),
        1 => {
            let c = zrows[0];
            let var = tab.row_var[c];
            if var >= n {
                return (3, 0, vec![], vec![], c);
            }
            let kk = tab.col_of(comp(var, n));
            let zero_pivot = pv.len() == 1 && pv[0] == c && tab.t[c * n + kk].abs() <= PTOL * gam[c].abs().max(1.0);
            if !zero_pivot {
                return (2, kk, vec![], vec![], c);
            }
            let slack_zero = (0..n).all(|i| i == c || tab.row_var[i] >= n || tab.t[i * n + kk].abs() <= 1e-8);
            if !slack_zero {
                return (1, kk, vec![], vec![], c);
            }
            let pm: Vec<usize> = (0..n).filter(|&i| tab.row_var[i] >= n && tab.row_var[i] < 2 * n && -tab.t[i * n + kk] > PTOL).collect();
            let mu: Vec<f64> = pm.iter().map(|&i| -tab.t[i * n + kk]).collect();
            (0, kk, pm, mu, c)
        }
        2 => (4, 0, vec![], vec![], 0),
        _ => (5, 0, vec![], vec![], 0),
    }
}

/// Census only, for the rank-1 class: the |P-| histogram, the members of S = A + {c} - {j} (j in P-), how many are in
/// `Seen`, for the unseen ones whether their sub-facet F and {lambda_j / mu_j <= lambda_k / mu_k for all k in P-} is
/// empty (Chebyshev LP), and whether two ratios are identical as affine functions on F. Returns the time spent.
#[cfg(feature = "lp_census")]
#[allow(clippy::too_many_arguments)]
fn rank1_census(p: &Plcp, tab: &Tab, work: &Tab, ef: &Mat, ff: &[f64], d1: usize, seen: Option<&Seen>, pm: &[usize], mu: &[f64], c: usize, _kk: usize) -> u128 {
    use crate::census::{bump, Counter as C};
    let t0 = std::time::Instant::now();
    let n = p.n;
    let hist = [C::R1P0, C::R1P1, C::R1P2, C::R1P3, C::R1P4, C::R1P5, C::R1P6, C::R1P7];
    bump(hist[pm.len().min(7)], 1);
    if pm.is_empty() {
        return t0.elapsed().as_nanos();
    }
    let w = 1 + d1;
    let zc = z_rows(tab, n);
    let cvar = tab.row_var[c] as usize; // slack w_c: the constraint index is cvar
    let mut all_seen = true;
    // lambda_i(theta_f) = rho row i of the work tableau
    let lam = |i: usize| -> &[f64] { &work.rho[i * w..(i + 1) * w] };
    let mut tie = false;
    for a in 0..pm.len() {
        for b in a + 1..pm.len() {
            let (ia, ib) = (pm[a], pm[b]);
            let diff: Vec<f64> = (0..w).map(|k| mu[b] * lam(ia)[k] - mu[a] * lam(ib)[k]).collect();
            let sc = 1.0 + (0..w).fold(0.0f64, |m, k| m.max((mu[b] * lam(ia)[k]).abs()).max((mu[a] * lam(ib)[k]).abs()));
            if diff.iter().all(|v| v.abs() <= 1e-9 * sc) {
                tie = true;
            }
        }
    }
    if tie {
        bump(C::R1Ties, 1);
    }
    for (a, &ia) in pm.iter().enumerate() {
        let zj = tab.row_var[ia] - n;
        let mut z: Vec<usize> = zc.iter().cloned().filter(|&x| x != zj).collect();
        let cz = cvar; // the slack index equals the z index of its complement
        z.push(cz);
        z.sort_unstable();
        z.dedup();
        bump(C::R1Members, 1);
        let known = seen.map_or(false, |s| s.contains_key(&p.active_set(z.iter().copied())));
        if known {
            continue;
        }
        all_seen = false;
        bump(C::R1Unseen, 1);
        // sub-facet: F and mu_k lambda_j - mu_j lambda_k <= 0 for k in P-, k != j
        let mut e2 = ef.clone();
        let mut f2 = ff.to_vec();
        if d1 > 0 {
            for (b, &ib) in pm.iter().enumerate() {
                if b == a {
                    continue;
                }
                let g: Vec<f64> = (0..w).map(|k| mu[b] * lam(ia)[k] - mu[a] * lam(ib)[k]).collect();
                let nrm = g[1..].iter().map(|v| v * v).sum::<f64>().sqrt();
                if nrm <= 1e-12 {
                    continue;
                }
                e2.push_row(&g[1..].iter().map(|v| v / nrm).collect::<Vec<_>>());
                f2.push(-g[0] / nrm);
            }
            let empty = match chebyshev_ball(&e2, &f2, &[]) {
                Some(s) => s.x[d1] <= 1e-9,
                None => true,
            };
            bump(if empty { C::R1UnseenEmpty } else { C::R1UnseenNonEmpty }, 1);
        }
    }
    if all_seen {
        bump(C::R1AllSeen, 1);
    }
    t0.elapsed().as_nanos()
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

static DAQP_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Switches the DAQP emptiness filter of the sub-facet tests (default on); off sends every test to the LP.
pub fn set_daqp_tests(on: bool) {
    DAQP_ON.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// The filter pays only when most of its calls end in a certified empty (a call that does not costs a DAQP solve and
/// then the LP as well). Per thread, after each window of `DQ_WINDOW` calls with fewer than `DQ_MIN_SHARE` percent
/// certified, the next `DQ_PAUSE` tests skip DAQP and go straight to the LP, then it is tried again.
const DQ_WINDOW: u32 = 512;
const DQ_MIN_SHARE: u32 = 60;
const DQ_PAUSE: u32 = 8192;

#[derive(Default)]
struct DqState {
    dq: crate::algorithms::pdaqp::daqp::Daqp,
    cols: Vec<f64>,
    rhs: Vec<f64>,
    calls: u32,
    empties: u32,
    pause: u32,
}

thread_local! {
    static DQ_WS: std::cell::RefCell<DqState> = std::cell::RefCell::new(DqState::default());
}

/// True when `{x in R^d1 : e x <= f}` is certainly empty: DAQP reports it infeasible and its certificate checks.
/// DAQP's infeasibility answer alone is not enough, because on nearly parallel rows its null-vector step can claim
/// infeasibility for a polytope with a ball of radius 1e-6 inside (seen on masses4_N3). The multipliers `p` of its
/// certificate are clamped to `max(p, 0)`, and from the clamped vector the residual `r = sum p_i a_i` and the gap
/// `g = sum p_i b_i` are recomputed. Every `x` of the polytope satisfies `0 <= p.(b - A x) = g - r.x`, so the polytope is empty
/// when `g + |r|_1 R < 0`, where `R` bounds `|x|_inf`: the facet coordinates are the parameters minus the component along
/// the facet normal, so `|x|_inf <= |x|_2 <= |theta|_2 <= bound`, the Euclidean radius of the parameter box
/// (`coord_bound`; negative when the box is unbounded or unknown, and then no certificate is accepted).
/// Most sub-facet tests of the neighbour search are empty, and one DAQP solve costs a fraction of the LP; everything not
/// certified goes to the LP unchanged.
fn daqp_certainly_empty(e: &Mat, f: &[f64], d1: usize, bound: f64) -> bool {
    use crate::algorithms::pdaqp::daqp::{Rows, Status};
    if d1 == 0 || e.rows == 0 || !(bound >= 0.0) || !DAQP_ON.load(std::sync::atomic::Ordering::Relaxed) {
        return false;
    }
    #[cfg(feature = "lp_census")]
    let t0 = std::time::Instant::now();
    let n = e.rows;
    crate::census_bump!(DqTests, 1);
    let empty = DQ_WS.with(|ws| {
        let st = &mut *ws.borrow_mut();
        if st.pause > 0 {
            st.pause -= 1;
            return None;
        }
        let DqState { dq, cols, rhs, .. } = &mut *st;
        #[cfg(feature = "lp_census")]
        let tb = std::time::Instant::now();
        cols.clear();
        cols.resize(n * d1, 0.0);
        rhs.clear();
        rhs.resize(n, 0.0);
        for i in 0..n {
            let row = e.row(i);
            let nrm = row.iter().map(|v| v * v).sum::<f64>().sqrt();
            if !(nrm > 1e-12) {
                return Some(false);
            }
            for t in 0..d1 {
                cols[t * n + i] = row[t] / nrm;
            }
            rhs[i] = f[i] / nrm;
        }
        let rows = Rows { cols: &cols[..], stride: n, b: &rhs[..], n, dim: d1 };
        #[cfg(feature = "lp_census")]
        crate::census::bump(crate::census::Counter::DqBuildNs, tb.elapsed().as_nanos() as u64);
        #[cfg(feature = "lp_census")]
        let ts = std::time::Instant::now();
        let st_inf = dq.solve(&rows, 0) == Status::Infeasible;
        #[cfg(feature = "lp_census")]
        crate::census::bump(crate::census::Counter::DqSolveNs, ts.elapsed().as_nanos() as u64);
        #[cfg(feature = "lp_census")]
        let tc = std::time::Instant::now();
        let certified = st_inf && {
            let (w, p) = dq.certificate();
            if p.iter().any(|v| !v.is_finite()) {
                false
            } else {
                let sp: f64 = p.iter().map(|&v| v.max(0.0)).sum();
                let gap: f64 = w.iter().zip(p).map(|(&i, &v)| v.max(0.0) * rhs[i]).sum();
                let res: f64 = (0..d1).map(|t| w.iter().zip(p).map(|(&i, &v)| v.max(0.0) * cols[t * n + i]).sum::<f64>().abs()).sum();
                sp > 0.0 && gap + res * bound < -1e-10 * sp
            }
        };
        #[cfg(feature = "lp_census")]
        crate::census::bump(crate::census::Counter::DqCertNs, tc.elapsed().as_nanos() as u64);
        st.calls += 1;
        st.empties += certified as u32;
        if st.calls == DQ_WINDOW {
            if st.empties * 100 < DQ_MIN_SHARE * DQ_WINDOW {
                st.pause = DQ_PAUSE;
            }
            st.calls = 0;
            st.empties = 0;
        }
        Some(certified)
    });
    #[cfg(feature = "lp_census")]
    crate::census::bump(crate::census::Counter::DqNanos, t0.elapsed().as_nanos() as u64);
    match empty {
        Some(true) => {
            crate::census_bump!(DqEmpty, 1);
            true
        }
        Some(false) => {
            crate::census_bump!(DqFallback, 1);
            false
        }
        None => false,
    }
}

const WITNESS_MARGIN: f64 = 1e-8;

/// Pivot budget of one facet walk in `pivot_facet`: `WALK_BUDGET_PER_ROW * n` pivots (n = rows of the
/// pLCP), at least `WALK_BUDGET_MIN`. A walk that spends it is stopped and the facet also gets the QP
/// just across it. Smaller is faster and finds the same regions on everything measured except
/// pop_005 (30 or 60 pivots: 10,294 regions, 100 or more: 10,297); see
/// `bench/results/mplcp_case2_phase3.md`.
const WALK_BUDGET_PER_ROW: usize = 2;
const WALK_BUDGET_MIN: usize = 64;

/// Memory statistics and knobs of the serial exploration: parents whose tableau is shared by queued lazy children,
/// the peak of that count, and the size of one tableau; `TAB_BUDGET` (bytes, 0 = no limit, default 512 MiB, shared by the live parents and the tableaux owned by queued items) is the limit on the tableaux of live parents beyond which a parent's children are queued without a
/// tableau and rebuild it from their active set when popped.
static LIVE_PARENTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static PEAK_PARENTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static TAB_BYTES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static TAB_BUDGET: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(512 << 20);

static SEEN_LEN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static SEEN_BYTES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static PEAK_QUEUE_LEN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static PEAK_QUEUE_BYTES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Memory held by the search structures of the last serial exploration: (entries of the `Seen` map, its bytes by
/// capacity, peak queued items, peak bytes of the queued items without their shared tableaux).
pub fn search_stats() -> (usize, usize, usize, usize) {
    let g = |a: &std::sync::atomic::AtomicUsize| a.load(std::sync::atomic::Ordering::Relaxed);
    (g(&SEEN_LEN), g(&SEEN_BYTES), g(&PEAK_QUEUE_LEN), g(&PEAK_QUEUE_BYTES))
}

/// Bytes of a tableau's numbers and indices (what the budget counts).
fn tab_len_bytes(t: &Tab) -> usize {
    (t.t.len() + t.rho.len()) * 8 + (t.row_var.len() + t.col_var.len()) * 8
}

fn item_bytes(it: &Item) -> usize {
    std::mem::size_of::<Item>()
        + it.center.as_ref().map_or(0, |c| c.capacity() * 8)
        + it.entry.as_ref().map_or(0, |e| e.0.capacity() * 8)
        + it.verts.as_ref().map_or(0, |v| v.capacity() * 8)
        + match &it.tab {
            TabSrc::Ready(t) => (t.t.capacity() + t.rho.capacity() + t.row_var.capacity() + t.col_var.capacity()) * 8,
            _ => 0,
        }
}

/// (peak number of live parent tableaux, bytes of one tableau) of the last serial exploration.
pub fn tab_stats() -> (usize, usize) {
    (PEAK_PARENTS.load(std::sync::atomic::Ordering::Relaxed), TAB_BYTES.load(std::sync::atomic::Ordering::Relaxed))
}

/// The byte budget for live parent tableaux (0 = none) of the serial exploration.
pub fn set_tab_budget(tab_budget: usize) {
    TAB_BUDGET.store(tab_budget, std::sync::atomic::Ordering::Relaxed);
}

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
                    LIVE_PARENTS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                    t.pivot(v, kk);
                    t
                }
                Err(a) => a.pivoted(v, kk),
            }),
            TabSrc::None => {
                let t = tableau_of(plcp, cr);
                if t.is_none() {
                    crate::census_bump!(ResolveNone, 1);
                }
                t
            }
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

/// Modes 5 and 6 are cost diagnostics: 5 collects vertices and the hyperplane index as mode 1 but looks nothing up; 6 collects the vertices only.
/// Certificate mode: 0 off, 1 on (default), 2 audit (certify, but still search, and count the new
/// regions that the search reaches across certified facets; for the census and the tests). Modes
/// 1 and 2 apply to 3 to 5 parameters, where the check measured faster or neutral; modes 3 (on) and
/// 4 (audit) apply to every count the vertex clip covers (2 to 6). At 2 parameters the problems are
/// too small to repay the bookkeeping, and at 6 the check (many vertices per facet) can cost more
/// than the search it saves (rand_t6_x5_m20_s0: 0.74x).
static CERT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);

/// Sets the facet-certificate mode of `Method::Pivot` and `Method::PivotParallel`: 0 off (the
/// default: at 6 parameters it lost a real region on masses3_N4, and the stray overlapping regions of
/// pop_005, pop_007 and pop_011 could pass as the known neighbour; it stays off until that is
/// understood), 1 on at 3 to 5 parameters, 2 audit, 3 and 4 the same at 2 to 6.
pub fn set_certificate(mode: u8) {
    CERT.store(mode, std::sync::atomic::Ordering::Relaxed);
}

static AUDIT_CERTIFIED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static AUDIT_EXTRA: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Audit mode counters: certified facets whose search ran (whether or not it returned a candidate; a rank-1 search of a facet
/// whose neighbours are all known returns none), and the candidates across them that built a new region (see `pivot_solve`; the
/// regions are found anyway, the order can change).
pub fn certificate_audit() -> (usize, usize) {
    (AUDIT_CERTIFIED.load(std::sync::atomic::Ordering::Relaxed), AUDIT_EXTRA.load(std::sync::atomic::Ordering::Relaxed))
}

/// Does mode `mode` apply the certificate to a program with `nt` parameters?
fn cert_active(mode: u8, nt: usize) -> bool {
    match mode {
        1 | 2 | 5 | 6 => (3..=5).contains(&nt),
        3 | 4 => (2..=6).contains(&nt),
        _ => false,
    }
}

type HKey = u64;

/// Hash key of the unit-normalised hyperplane (a·θ <= b), rounded to 1e-5. Used only to find
/// candidate regions (collisions and a missed rounding boundary lose or add a candidate, never
/// soundness: every candidate is checked exactly).
fn hkey(a: &[f64], b: f64) -> HKey {
    let nrm = a.iter().map(|v| v * v).sum::<f64>().sqrt();
    let inv = 1e5 / nrm;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for v in a {
        h = (h ^ ((v * inv).round() as i64 as u64)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    (h ^ ((b * inv).round() as i64 as u64)).wrapping_mul(0x0000_0100_0000_01b3)
}

/// The facet hyperplanes of the known regions, for 2 to 6 parameters (the range of the vertex clip).
/// `heads` maps a hyperplane key to the newest entry of a chain in `entries` (region, next entry);
/// no allocation per row.
#[derive(Default)]
struct Known {
    heads: crate::fxhash::FxHashMap<HKey, u32>,
    entries: Vec<(u32, u32)>,
}

impl Known {
    fn add(&mut self, idx: usize, cr: &CriticalRegion) {
        if cr.e.cols < 2 || cr.e.cols > 6 {
            return;
        }
        for k in 0..cr.e.rows {
            let at = self.entries.len() as u32;
            let prev = self.heads.insert(hkey(cr.e.row(k), cr.f[k]), at).unwrap_or(u32::MAX);
            self.entries.push((idx as u32, prev));
        }
    }

    /// Regions that have a row with this key, newest first.
    fn candidates(&self, key: HKey) -> impl Iterator<Item = u32> + '_ {
        let mut at = self.heads.get(&key).copied().unwrap_or(u32::MAX);
        std::iter::from_fn(move || {
            if at == u32::MAX {
                return None;
            }
            let (region, next) = self.entries[at as usize];
            at = next;
            Some(region)
        })
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
            (_, Some(v)) if mode != 5 && mode != 6 && cert_active(mode, nt) => Some(CertCtx { verts: v, known, regions, nt, audit: mode == 2 || mode == 4, limit }),
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
    let mut cand = cx.known.candidates(hkey(&ng, -b)).peekable();
    cand.peek()?;
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
    'regions: for j in cand {
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
fn expand_candidates(plcp: &Plcp, cur: &CriticalRegion, center: Option<&Vec<f64>>, tab: &Tab, depth: usize, entry: Option<&(Vec<f64>, f64)>, cert: Option<&CertCtx>, seen: Option<&Seen>, program: &MpProgram, ws: &mut Option<Tab>) -> Vec<Cand> {
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
        // a point on the facet: the region's Chebyshev center projected onto the row (computed up front
        // only for the census, which counts how often it lies in a leaf's sub-facet)
        let mk_hint = || -> Option<Vec<f64>> {
            center.map(|c| {
                let a = cur.e.row(k);
                let sft = cur.f[k] - a.iter().zip(c).map(|(x, y)| x * y).sum::<f64>();
                c.iter().zip(a).map(|(ci, ai)| ci + sft * ai).collect()
            })
        };
        let (nbrs, certified) = adjacent(plcp, tab, cur.e.row(k), cur.f[k], &cur.e, &cur.f, ws, if cert.is_some() { Some(&cfun as &dyn Fn() -> Option<usize>) } else { None }, cert.map_or(false, |c| c.audit), seen, center.map(|c| c.as_slice()));
        // audit mode: every certified facet is counted, whether or not its search returned a candidate (a rank-1 search of a
        // facet whose neighbours are all known returns none)
        if certified.is_some() && cert.map_or(false, |c| c.audit) {
            AUDIT_CERTIFIED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
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
        let hint = mk_hint();
        for nb in nbrs {
            if let Nbr::Across(pt) = &nb {
                // fallback: the lexicographic Lemke basis at the probe point just across the facet (the QP's
                // optimal active set can be a lexico-infeasible twin); a basis whose tableau does not exist
                // (rank-deficient active set) or is not lexico-feasible is rejected
                let basis = lex_lemke(plcp, pt).filter(|z| tableau_for(plcp, z).map_or(false, |t| lex_feasible(&t)));
                let _ = program;
                match basis {
                    Some(z) => {
                        crate::census_bump!(FallbackAcross, 1);
                        cands.push(Cand { aset: plcp.active_set(z.into_iter()), hint: Some(pt.clone()), piv: None, entry: None, cert: None });
                    }
                    None => {
                        crate::census_bump!(FallbackFail, 1);
                    }
                }
                continue;
            }
            let z = match &nb {
                Nbr::Toggle(v, _) => z_toggle(tab, plcp.n, *v),
                Nbr::Z(z, _) => z.clone(),
                Nbr::Across(_) => unreachable!(),
            };
            // a Case-2 basis starts from the centre of its own sub-facet when the walk found one
            let hint = match &nb {
                Nbr::Z(_, Some(c)) => Some(c.clone()),
                _ => hint.clone(),
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
            cands.push(Cand { aset: plcp.active_set(z.into_iter()), hint, piv, entry, cert: certified });
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
            // the step is refused only when the pivot is small against the scale of its row (M_vv below 1), as in `tableau_for`
            let vr = parent.row_var[r];
            let floor = if vr < n { 1e-9 * plcp.m[vr * n + vr].abs().min(1.0) } else { 1e-9 };
            if !(p.abs() > floor.max(1e-300)) {
                crate::census_bump!(CandLawSmallPiv, 1);
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
            let Some(tab) = tableau_for(plcp, &z) else {
                crate::census_bump!(CandLawNoTab, 1);
                return None;
            };
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
/// Largest relative residual of the active constraints under a region's law, above which the
/// candidate basis is rejected (see `build_cand`).
const TIGHT_TOL: f64 = 1e-3;

/// Diagnostic: the tightness residual of `cr` at `centre` and whether the QP agrees with it.
pub fn tight_diagnostic(program: &MpProgram, cr: &CriticalRegion, centre: &[f64]) -> (f64, bool) {
    (tight_residual(program, cr, centre), qp_agrees(program, cr, centre))
}

static TIGHT_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Switches the tightness flag of `build_cand` (default on); off reproduces the walk without it,
/// for the comparison tools.
pub fn set_tight_check(on: bool) {
    TIGHT_ON.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Does the direct QP at the region's centre agree with the region: same active set, or the same
/// primal solution as the region's law (to 1e-6 relative)?
fn qp_agrees(program: &MpProgram, cr: &CriticalRegion, centre: &[f64]) -> bool {
    let Some(out) = program.solve_theta(centre) else { return false };
    let mut qa = out.active_set.clone();
    qa.sort_unstable();
    let mut ca = cr.active_set.clone();
    ca.sort_unstable();
    if qa == ca {
        return true;
    }
    let x = cr.evaluate(centre);
    x.len() == out.sol.len() && x.iter().zip(&out.sol).all(|(a, b)| (a - b).abs() <= 1e-6 * (1.0 + b.abs()))
}

/// Largest relative violation, at the region's centre, of "the active constraints are tight under
/// the region's primal law". A basis that is singular in exact arithmetic (two parallel rows both
/// active) gets a garbage law that cannot satisfy both, whatever size its coefficients have.
fn tight_residual(program: &MpProgram, cr: &CriticalRegion, centre: &[f64]) -> f64 {
    let x = cr.evaluate(centre);
    let nx = program.num_x();
    let nt = program.num_t();
    let mut worst = 0.0f64;
    for &i in &cr.active_set {
        let ai = program.a.row(i);
        let ax: f64 = (0..nx).map(|j| ai[j] * x[j]).sum();
        let rhs: f64 = program.b[i] + (0..nt).map(|t| program.f[(i, t)] * centre[t]).sum::<f64>();
        let sc = 1.0 + rhs.abs() + (0..nx).map(|j| (ai[j] * x[j]).abs()).sum::<f64>();
        worst = worst.max((ax - rhs).abs() / sc);
    }
    worst
}


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
        // A basis that is singular in exact arithmetic (parallel rows both active) can pass every
        // other test with a garbage law; its region then overlaps the true ones (pop_005, pop_007,
        // pop_011). Legitimate regions have residuals up to 1e-5 (masses5_N3), garbage ones 1e-1. The
        // residual only flags a candidate; a direct QP at the region's centre decides, so a genuine
        // sliver in an ill-conditioned problem is kept.
        if TIGHT_ON.load(std::sync::atomic::Ordering::Relaxed) && tight_residual(program, &cr, &ce) > TIGHT_TOL {
            crate::census_bump!(TightFlagged, 1);
            // the builder's centre comes from an LP started at a hint; a region of radius 1e-8 can have
            // the QP flip at that point, so the centre of the final rows is tried as well
            let c2: Option<Vec<f64>> = crate::utils::chebyshev_ball(&cr.e, &cr.f, &[]).map(|sol| sol.x[..ce.len()].to_vec());
            if !qp_agrees(program, &cr, &ce) && !c2.map_or(false, |c| qp_agrees(program, &cr, &c)) {
                crate::census_bump!(TightRejected, 1);
                return None;
            }
            crate::census_bump!(TightKeptByQp, 1);
        }
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
    if cmode != 6 && cert_active(cmode, plcp.nt) {
        known.add(0, &regions[0]);
    }
    let mut queue: VecDeque<Item> = VecDeque::new();
    queue.push_back(Item { idx: 0, center: init_center, tab: TabSrc::None, depth: 0, entry: None, verts: None });
    let mut ws: Option<Tab> = None;
    let mut pending: Vec<(CriticalRegion, Option<Vec<f64>>, Option<(usize, usize)>, Option<Tab>, Option<(Vec<f64>, f64)>, Option<Vec<f64>>)> = vec![];
    let mut wave_limit = 1usize;
    LIVE_PARENTS.store(0, std::sync::atomic::Ordering::Relaxed);
    PEAK_PARENTS.store(0, std::sync::atomic::Ordering::Relaxed);
    let budget = TAB_BUDGET.load(std::sync::atomic::Ordering::Relaxed);
    PEAK_QUEUE_LEN.store(0, std::sync::atomic::Ordering::Relaxed);
    PEAK_QUEUE_BYTES.store(0, std::sync::atomic::Ordering::Relaxed);
    let mut queue_bytes = item_bytes(queue.front().unwrap());
    // bytes of the tableaux that queued items own (a Case-2 child arrives with its own), counted against the same budget
    let mut ready_bytes = 0usize;
    loop {
        let Some(it) = queue.pop_front() else { break };
        queue_bytes -= item_bytes(&it);
        if let TabSrc::Ready(t) = &it.tab {
            ready_bytes -= tab_len_bytes(t);
        }
        let Item { idx, center, tab, depth, entry, verts } = it;
        if idx >= wave_limit {
            wave_limit = regions.len();
        }
        let cur = &regions[idx];
        let Some(tab) = crate::census_timed!(MplcpTabNanos, tab.resolve(plcp, cur)) else { continue };
        let cx = CertCtx::new(cmode, verts.as_deref(), &known, &regions, plcp.nt, wave_limit);
        let cands = crate::census_timed!(MplcpNbrNanos, expand_candidates(plcp, cur, center.as_ref(), &tab, depth, entry.as_ref(), cx.as_ref(), Some(&seen), program, &mut ws));
        for c in cands {
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
        let tbytes = tab_len_bytes(&tab);
        TAB_BYTES.store(tbytes, std::sync::atomic::Ordering::Relaxed);
        // beyond the budget for live parent tableaux the children are queued without one and rebuild it when popped
        let over = budget > 0 && (LIVE_PARENTS.load(std::sync::atomic::Ordering::Relaxed) + 1) * tbytes + ready_bytes > budget;
        let tab = std::sync::Arc::new(tab);
        let mut lazy_children = 0usize;
        for (cr, ce, piv, own, ent, vt) in pending.drain(..) {
            let (src, nd) = match piv {
                Some((v, kk)) if !over => {
                    lazy_children += 1;
                    (TabSrc::Lazy(tab.clone(), v, kk), depth + 1)
                }
                Some(_) => (TabSrc::None, 0),
                None => match own {
                    // an owned tableau is kept while the live parents and the owned tableaux fit in the budget
                    Some(t) if budget == 0 || (LIVE_PARENTS.load(std::sync::atomic::Ordering::Relaxed) + 1) * tbytes + ready_bytes + tab_len_bytes(&t) <= budget => {
                        ready_bytes += tab_len_bytes(&t);
                        (TabSrc::Ready(t), 0)
                    }
                    _ => (TabSrc::None, 0),
                },
            };
            seen.insert(cr.active_set.clone(), Slot::Built(regions.len()));
            regions.push(cr);
            if cmode != 6 && cert_active(cmode, plcp.nt) {
                known.add(regions.len() - 1, regions.last().unwrap());
            }
            let it = Item { idx: regions.len() - 1, center: ce, tab: src, depth: nd, entry: ent, verts: vt };
            queue_bytes += item_bytes(&it);
            queue.push_back(it);
            PEAK_QUEUE_LEN.fetch_max(queue.len(), std::sync::atomic::Ordering::Relaxed);
            PEAK_QUEUE_BYTES.fetch_max(queue_bytes, std::sync::atomic::Ordering::Relaxed);
        }
        if lazy_children > 0 {
            let live = LIVE_PARENTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            PEAK_PARENTS.fetch_max(live, std::sync::atomic::Ordering::Relaxed);
        }
    }
    SEEN_LEN.store(seen.len(), std::sync::atomic::Ordering::Relaxed);
    // hashbrown: one control byte per bucket and (key header + slot) per bucket, at the load factor 7/8
    SEEN_BYTES.store(seen.capacity() * (1 + std::mem::size_of::<(Vec<usize>, Slot)>()) + seen.keys().map(|k| k.capacity() * 8).sum::<usize>(), std::sync::atomic::Ordering::Relaxed);
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
    if cmode != 6 && cert_active(cmode, plcp.nt) {
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
            let cands = expand_candidates(plcp, cur, center.as_ref(), &tab, depth, entry.as_ref(), cx.as_ref(), Some(&seen), program, ws);
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
            if cmode != 6 && cert_active(cmode, plcp.nt) {
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
    Plcp { n, nt, m, q, r, ineq: vec![], eq: vec![], pos: vec![], xe: Mat::zeros(0, 0), coord_bound: -1.0, arows: Mat::zeros(0, 0), aqi: Mat::zeros(0, 0) }
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
    Plcp { n, nt, m, q, r, ineq: p.ineq.clone(), eq: p.eq.clone(), pos: p.pos.clone(), xe: p.xe.clone(), coord_bound: p.coord_bound, arows: Mat::zeros(0, 0), aqi: Mat::zeros(0, 0) }
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
