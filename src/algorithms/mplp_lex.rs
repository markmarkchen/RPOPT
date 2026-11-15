//! Lexicographic perturbation for multiparametric linear programs.
//!
//! C. N. Jones, E. C. Kerrigan, J. M. Maciejowski, "Lexicographic perturbation for
//! multiparametric linear programming with applications to control", Automatica 43(10), 2007.
//!
//! The mpLP  min (c + Hθ)'x  s.t.  A x <= b + Fθ  (rows in `equality_indices` hold with equality,
//! they are always in the basis) is replaced by the lexicographically (symbolically) perturbed
//! program with  b -> b + ε,  ε = (ε, ε², .., ε^m)  and  c -> c + δ,  δ = (δ, δ², .., δ^n)
//! (Sections 3.1 and 3.2). Every optimal basis of the perturbed program is optimal for the
//! original; the perturbed program is primal and dual nondegenerate, so for every θ exactly one
//! basis is optimal. A basis B is a set of n_x linearly independent constraint rows (the active
//! set); with β = A_B^{-1} and κ = -A_B^{-T}:
//!   * lex-feasible:  every nonbasic slack row  [ (b+Fθ)_j - A_j β (b+Fθ)_B | e_j - A_j β E_B ]  is
//!     lexico-positive (Definition 6 / Theorem 8),
//!   * dual lex-feasible: every inequality multiplier row  [ κ (c+Hθ) | κ ]  is lexico-positive
//!     (eq. (9) and Lemma 13).
//! The regions R_B (Definition 11, Lemma 13, Lemma 15) are disjoint and cover the feasible Θ
//! (Proposition 14), the optimizer is unique and continuous (Theorem 16), the regions meet
//! facet to facet (Corollary 20), and the graph of bases is connected (Theorem 22).
//!
//! Section 4.1: the neighbour across a facet F of R_B is the optimal basis of the perturbed LP at
//! θ0 + γ n (θ0 in the relative interior of F, n the outward normal, γ an infinitesimal larger
//! than ε and δ); it is found by a few lexicographic simplex pivots started from B (a single
//! pivot when nondegenerate, Corollary 18). Nothing is evaluated at a small numeric step: the
//! lexicographic vectors carry the columns [value at θ0 | γ coefficient | ε (or δ) identity], and
//! ties are broken by the first nonzero entry. Section 4.2: depth-first (here breadth-first)
//! enumeration of the solution graph.
//!
//! Generalization: the paper takes the parameter in the cost or in the right hand side but not
//! both. Here both may be present (H and F); the perturbations ε and δ do not depend on θ, so
//! the two lexicographic conditions are independent and all the statements above carry over
//! (continuity of x(θ) is only guaranteed when H = 0, as in the paper, where the parameter
//! enters one side).
//!
//! Initial basis (Remark 9): a dual simplex from an arbitrary basis with an artificial positive
//! multiplier vector (lexicographic primal ratio comes for free because the dual is nondegenerate),
//! then primal lex-pivots with the true cost.

use crate::critical_region::CriticalRegion;
use crate::fxhash::FxHashSet as HashSet;
use crate::linalg::{norm, rank, Lu, Mat};
use crate::mpqp_utils::gen_cr_from_active_set_lambda_first;
use crate::program::MpProgram;
use crate::utils::chebyshev_ball;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::cmp::Ordering;
use std::collections::VecDeque;

const TOL: f64 = 1e-9;
/// The direct facet crossing is trusted only between regions with a Chebyshev radius above this:
/// thinner regions are numerically marginal (ties inside the tolerance) and take the LP path.
const FAT: f64 = 1e-4;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fail {
    Infeasible,
    Unbounded,
    Singular,
    Stalled,
}

/// First entry above the tolerance decides the sign (lexicographic sign).
#[inline]
fn lex_sign(v: &[f64]) -> i32 {
    for &x in v {
        if x > TOL {
            return 1;
        }
        if x < -TOL {
            return -1;
        }
    }
    0
}

#[inline]
fn lex_cmp(a: &[f64], b: &[f64]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        let d = x - y;
        if d > TOL {
            return Ordering::Greater;
        }
        if d < -TOL {
            return Ordering::Less;
        }
    }
    Ordering::Equal
}

/// Static data of the LP: constraint matrix and the number of leading equality rows.
struct Lex<'a> {
    a: &'a Mat,
    neq: usize,
    m: usize,
    nx: usize,
}

/// Working basis: the basis list (equality rows first), β = A_B^{-1} (nx x nx) and the tableau
/// T = A β (m x nx, row j = A_j β). The lexicographic slack row of a nonbasic constraint j is
/// [ q0_j - A_j x0 | q1_j - A_j x1 | e_j - T_j E_B ]: its identity block is nonzero only in the
/// columns {j} ∪ B, so it is never stored; T_j gives it.
#[derive(Clone)]
struct Work {
    b: Vec<usize>,
    binv: Vec<f64>,
    t: Vec<f64>,
}

/// Scratch buffers of one `solve` call, reused across pivots.
struct Scratch {
    x0: Vec<f64>,
    x1: Vec<f64>,
    s0: Vec<f64>,
    s1: Vec<f64>,
    in_b: Vec<bool>,
    bs: Vec<(usize, usize)>,
    cols: Vec<(usize, Option<usize>)>,
    lam: Vec<f64>,
    lam2: Vec<f64>,
}

impl Scratch {
    fn new(m: usize, nx: usize) -> Self {
        Scratch {
            x0: vec![0.0; nx],
            x1: vec![0.0; nx],
            s0: vec![0.0; m],
            s1: vec![0.0; m],
            in_b: vec![false; m],
            bs: Vec::with_capacity(nx),
            cols: Vec::with_capacity(nx + 2),
            lam: vec![0.0; nx + 2],
            lam2: vec![0.0; nx + 2],
        }
    }
}

/// Fresh factorizations are taken after this many pivots (drift control of the rank-1 updates).
const REFRESH: usize = 25;

impl<'a> Lex<'a> {
    fn new(a: &'a Mat, neq: usize) -> Self {
        Lex { a, neq, m: a.rows, nx: a.cols }
    }

    /// β and T from scratch for the basis `b`.
    fn fresh(&self, b: &[usize], w: &mut Work) -> Result<(), Fail> {
        let ab = self.a.select_rows(b);
        let lu = Lu::new(&ab).ok_or(Fail::Singular)?;
        let binv = lu.solve_mat(&Mat::eye(self.nx));
        let t = self.a.matmul(&binv);
        w.b.clear();
        w.b.extend_from_slice(b);
        w.binv = binv.data;
        w.t = t.data;
        Ok(())
    }

    /// Row j enters the basis at position p (rank-1 update of β and T).
    fn pivot(&self, w: &mut Work, p: usize, j: usize) {
        let nx = self.nx;
        let u: Vec<f64> = w.t[j * nx..(j + 1) * nx].to_vec();
        let up = u[p];
        let ratio: Vec<f64> = u.iter().map(|v| v / up).collect();
        for k in 0..nx {
            let row = &mut w.binv[k * nx..(k + 1) * nx];
            let cp = row[p];
            for q in 0..nx {
                row[q] -= cp * ratio[q];
            }
            row[p] = cp / up;
        }
        for i in 0..self.m {
            let row = &mut w.t[i * nx..(i + 1) * nx];
            let cp = row[p];
            for q in 0..nx {
                row[q] -= cp * ratio[q];
            }
            row[p] = cp / up;
        }
        w.b[p] = j;
    }

    /// Value of the lexicographic slack row j (divided by `div`) in the identity column c, where
    /// `pos` is the basis position of c if c is basic.
    #[inline]
    fn eps_val(&self, w: &Work, j: usize, div: f64, c: usize, pos: Option<usize>) -> f64 {
        let mut v = if c == j { 1.0 } else { 0.0 };
        if let Some(p) = pos {
            v -= w.t[j * self.nx + p];
        }
        v / div
    }

    /// Lexicographic sign of the slack row j.
    #[inline]
    fn slack_sign(&self, w: &Work, sc: &Scratch, j: usize) -> i32 {
        let (a, b) = (sc.s0[j], sc.s1[j]);
        if a > TOL {
            return 1;
        }
        if a < -TOL {
            return -1;
        }
        if b > TOL {
            return 1;
        }
        if b < -TOL {
            return -1;
        }
        for &(c, p) in &sc.bs {
            if c > j {
                break;
            }
            let v = -w.t[j * self.nx + p];
            if v > TOL {
                return 1;
            }
            if v < -TOL {
                return -1;
            }
        }
        1
    }

    /// Lexicographic comparison of the slack rows j / dj and k / dk.
    fn slack_cmp(&self, w: &Work, sc: &mut Scratch, j: usize, dj: f64, k: usize, dk: f64) -> Ordering {
        let d = sc.s0[j] / dj - sc.s0[k] / dk;
        if d > TOL {
            return Ordering::Greater;
        }
        if d < -TOL {
            return Ordering::Less;
        }
        let d = sc.s1[j] / dj - sc.s1[k] / dk;
        if d > TOL {
            return Ordering::Greater;
        }
        if d < -TOL {
            return Ordering::Less;
        }
        // identity block over the columns B ∪ {j, k} in increasing order
        sc.cols.clear();
        sc.cols.extend(sc.bs.iter().map(|&(c, p)| (c, Some(p))));
        for x in [j, k] {
            let at = sc.cols.partition_point(|&(c, _)| c < x);
            sc.cols.insert(at, (x, None));
        }
        for &(c, p) in &sc.cols {
            let d = self.eps_val(w, j, dj, c, p) - self.eps_val(w, k, dk, c, p);
            if d > TOL {
                return Ordering::Greater;
            }
            if d < -TOL {
                return Ordering::Less;
            }
        }
        Ordering::Equal
    }

    /// Lexicographic simplex from the basis in `w` (equality rows in the first `neq` positions) for
    /// the lex data (q0, q1 | d0, d1). Phase 1 (dual simplex with the artificial multipliers `cp`,
    /// which keeps the dual feasible and removes lex-infeasible slack rows) gives a lex-feasible
    /// basis, phase 2 (primal simplex with the lexicographic ratio test (7)) the lex-optimal one.
    fn solve(&self, w: &mut Work, sc: &mut Scratch, q0: &[f64], q1: &[f64], d0: &[f64], d1: &[f64], cp: &[f64]) -> Result<(), Fail> {
        let (nx, m) = (self.nx, self.m);
        let cap = 200 + 20 * (m + nx);
        let mut since = 0usize;
        for _ in 0..cap {
            if since >= REFRESH {
                let b = w.b.clone();
                self.fresh(&b, w)?;
                since = 0;
            }
            // x = β q_B and the numeric slack columns
            for k in 0..nx {
                let row = &w.binv[k * nx..(k + 1) * nx];
                let (mut a0, mut a1) = (0.0, 0.0);
                for (p, &bp) in w.b.iter().enumerate() {
                    a0 += row[p] * q0[bp];
                    a1 += row[p] * q1[bp];
                }
                sc.x0[k] = a0;
                sc.x1[k] = a1;
            }
            sc.in_b.iter_mut().for_each(|v| *v = false);
            for &i in &w.b {
                sc.in_b[i] = true;
            }
            for j in 0..m {
                if sc.in_b[j] {
                    continue;
                }
                let aj = self.a.row(j);
                let (mut s0, mut m0, mut s1, mut m1) = (q0[j], q0[j].abs(), q1[j], q1[j].abs());
                for k in 0..nx {
                    let t = aj[k] * sc.x0[k];
                    s0 -= t;
                    m0 += t.abs();
                    let t = aj[k] * sc.x1[k];
                    s1 -= t;
                    m1 += t.abs();
                }
                // cancellation noise of the numeric columns counts as an exact tie
                sc.s0[j] = if s0.abs() < 1e-12 * m0 { 0.0 } else { s0 };
                sc.s1[j] = if s1.abs() < 1e-12 * m1 { 0.0 } else { s1 };
            }
            sc.bs.clear();
            sc.bs.extend(w.b.iter().enumerate().map(|(p, &c)| (c, p)));
            sc.bs.sort_unstable();
            // phase 1: a lex-negative slack row
            let mut worst: Option<usize> = None;
            for j in self.neq..m {
                if sc.in_b[j] {
                    continue;
                }
                if self.slack_sign(w, sc, j) < 0 {
                    match worst {
                        Some(wj) if self.slack_cmp(w, sc, j, 1.0, wj, 1.0) != Ordering::Less => {}
                        _ => worst = Some(j),
                    }
                }
            }
            if let Some(j) = worst {
                let mut pick: Option<(usize, f64)> = None;
                for p in self.neq..nx {
                    let y = w.t[j * nx + p];
                    if y > 1e-9 {
                        let mut l = 0.0;
                        for k in 0..nx {
                            l -= w.binv[k * nx + p] * cp[k];
                        }
                        let r = l / y;
                        if pick.map_or(true, |(_, br)| r < br) {
                            pick = Some((p, r));
                        }
                    }
                }
                let Some((p, _)) = pick else { return Err(Fail::Infeasible) };
                self.pivot(w, p, j);
                since += 1;
                continue;
            }
            // phase 2: a lex-negative multiplier row leaves
            let mut leave: Option<usize> = None;
            for p in self.neq..nx {
                self.lam_row(w, p, d0, d1, &mut sc.lam2);
                if lex_sign(&sc.lam2) < 0 {
                    let better = match leave {
                        None => true,
                        Some(_) => lex_cmp(&sc.lam2, &sc.lam) == Ordering::Less,
                    };
                    if better {
                        leave = Some(p);
                        std::mem::swap(&mut sc.lam, &mut sc.lam2);
                    }
                }
            }
            let Some(p) = leave else { return Ok(()) };
            // edge direction  dir = -β e_p ; A_j dir = -T_jp ; lexicographic ratio test over A_j dir > 0
            let mut best: Option<(usize, f64)> = None;
            for j in self.neq..m {
                if sc.in_b[j] {
                    continue;
                }
                let aj = -w.t[j * nx + p];
                if aj > 1e-9 {
                    if best.map_or(true, |(bj, bd)| self.slack_cmp(w, sc, j, aj, bj, bd) == Ordering::Less) {
                        best = Some((j, aj));
                    }
                }
            }
            let Some((j, _)) = best else { return Err(Fail::Unbounded) };
            self.pivot(w, p, j);
            since += 1;
        }
        Err(Fail::Stalled)
    }

    /// Multiplier row  [ -β_p' d0 | -β_p' d1 | -β_{.,p} ]  of basis position p.
    fn lam_row(&self, w: &Work, p: usize, d0: &[f64], d1: &[f64], out: &mut [f64]) {
        let nx = self.nx;
        let (mut s0, mut m0, mut s1, mut m1) = (0.0, 0.0, 0.0, 0.0);
        for k in 0..nx {
            let bk = w.binv[k * nx + p];
            let t = bk * d0[k];
            s0 -= t;
            m0 += t.abs();
            let t = bk * d1[k];
            s1 -= t;
            m1 += t.abs();
            out[2 + k] = -bk;
        }
        out[0] = if s0.abs() < 1e-12 * m0 { 0.0 } else { s0 };
        out[1] = if s1.abs() < 1e-12 * m1 { 0.0 } else { s1 };
    }
}

/// Unit-norm form of a row (a'θ <= f).
fn unit(row: &[f64], f: f64) -> Option<(Vec<f64>, f64)> {
    let n = norm(row);
    if n < 1e-12 {
        return None;
    }
    Some((row.iter().map(|v| v / n).collect(), f / n))
}

/// A basis from the first `neq` equality rows plus a greedy independent completion.
fn initial_basis(a: &Mat, neq: usize) -> Option<Vec<usize>> {
    let nx = a.cols;
    let mut b: Vec<usize> = (0..neq).collect();
    let mut r = if neq > 0 { rank(&a.select_rows(&b)) } else { 0 };
    if r < neq {
        return None;
    }
    for j in neq..a.rows {
        if b.len() == nx {
            break;
        }
        b.push(j);
        let rk = rank(&a.select_rows(&b));
        if rk > r {
            r = rk;
        } else {
            b.pop();
        }
    }
    (b.len() == nx).then_some(b)
}

/// Facet-to-facet enumeration of the lex-optimal bases (Section 4.2).
pub fn solve(program: &MpProgram, seed: u64) -> Result<Vec<CriticalRegion>, String> {
    if program.q.is_some() {
        return Err("mplp_lex: not an mpLP".into());
    }
    let nx = program.num_x();
    let nt = program.num_t();
    let m = program.num_constraints();
    let neq = program.num_equality_constraints();
    if nx == 0 || m < nx {
        return Err("mplp_lex: fewer constraints than variables".into());
    }
    if rank(&program.a) < nx {
        return Err("mplp_lex: constraint matrix A is rank deficient (no vertices)".into());
    }
    let b0 = initial_basis(&program.a, neq).ok_or("mplp_lex: dependent equality rows")?;
    let lex = Lex::new(&program.a, neq);
    let mut sc = Scratch::new(m, nx);
    let zm = vec![0.0; m];
    let zx = vec![0.0; nx];
    let mut rng = StdRng::seed_from_u64(seed);
    let artificial =|rng: &mut StdRng, b: &[usize]| -> Vec<f64> {
        // c' = -A_B' λ' with λ' > 0 random: makes the current basis dual feasible
        let lam: Vec<f64> = (0..b.len()).map(|_| rng.gen_range(0.5..1.5)).collect();
        let mut cp = vec![0.0; nx];
        for (p, &row) in b.iter().enumerate() {
            for k in 0..nx {
                cp[k] -= program.a[(row, k)] * lam[p];
            }
        }
        cp
    };

    // ---- initial full-dimensional region at a generic feasible θ0
    let ball = program.feasible_space_chebychev_ball().ok_or("mplp_lex: infeasible program")?;
    let center: Vec<f64> = ball.x[nx..nx + nt].to_vec();
    let radius = ball.x[ball.x.len() - 1];
    let mut initial: Option<(CriticalRegion, Vec<usize>)> = None;
    for attempt in 0..300 {
        let th: Vec<f64> = if attempt == 0 && radius <= 0.0 {
            center.clone()
        } else {
            center.iter().map(|c| c + rng.gen_range(-1.0..1.0) * 0.9 * radius.max(1e-6)).collect()
        };
        let q0 = rhs_at(program, &th);
        let d0 = cost_at(program, &th);
        let cp = artificial(&mut rng, &b0);
        let mut w = Work { b: vec![], binv: vec![], t: vec![] };
        let res = lex.fresh(&b0, &mut w).and_then(|_| lex.solve(&mut w, &mut sc, &q0, &zm, &d0, &zx, &cp));
        match res {
            Ok(()) => {
                let mut basis = w.b.clone();
                basis.sort_unstable();
                if let Some(cr) = gen_cr_from_active_set_lambda_first(program, &basis, true) {
                    initial = Some((cr, basis));
                    break;
                }
            }
            Err(Fail::Unbounded) if attempt > 20 => return Err("mplp_lex: unbounded program".into()),
            Err(_) => {}
        }
    }
    let (cr0, basis0) = initial.ok_or("mplp_lex: no full-dimensional initial region found")?;

    // normalized parameter-space rows, to recognise facets on the boundary of Θ
    let theta_rows: Vec<(Vec<f64>, f64)> = (0..program.a_t.rows).filter_map(|i| unit(program.a_t.row(i), program.b_t[i])).collect();

    let mut seen: HashSet<Vec<usize>> = HashSet::default();
    seen.insert(basis0.clone());
    let mut regions: Vec<CriticalRegion> = vec![];
    // facets already crossed from the other side: basis -> (unit normal, offset) of its facets whose
    // neighbour is known (each facet is shared by exactly two regions, Corollary 20)
    let mut crossed: crate::fxhash::FxHashMap<Vec<usize>, Vec<(Vec<f64>, f64)>> = Default::default();
    let mut queue: VecDeque<(usize, Vec<usize>)> = VecDeque::new();
    let mut index: crate::fxhash::FxHashMap<Vec<usize>, usize> = Default::default();
    index.insert(basis0.clone(), 0);
    regions.push(cr0);
    queue.push_back((0, basis0));
    let h_zero = program.h.data.iter().all(|v| *v == 0.0);
    let mut parent = Work { b: vec![], binv: vec![], t: vec![] };
    let mut wk = parent.clone();
    while let Some((ri, basis)) = queue.pop_front() {
        let known = crossed.remove(&basis).unwrap_or_default();
        // the basis is sorted: equality rows first
        if lex.fresh(&basis, &mut parent).is_err() {
            continue;
        }
        let facets: Vec<(Vec<f64>, f64)> = {
            let cr = &regions[ri];
            (0..cr.e.rows).filter_map(|k| unit(cr.e.row(k), cr.f[k])).collect()
        };
        let mut centre: Option<Option<Vec<f64>>> = None;
        let (dctx, dctx_rows, zset) = if h_zero && regions[ri].cheb_radius.is_some_and(|r| r > FAT) { region_context(&lex, program, &regions[ri], &basis) } else { (false, vec![], vec![]) };
        for k in 0..facets.len() {
            let (nk, fk) = (&facets[k].0, facets[k].1);
            // facet on the boundary of Θ
            if theta_rows.iter().any(|(t, ft)| t.iter().zip(nk).all(|(a, b)| (a - b).abs() < 1e-7) && (ft - fk).abs() < 1e-6 * (1.0 + fk.abs())) {
                continue;
            }
            if known.iter().any(|(t, ft)| t.iter().zip(nk).all(|(a, b)| (a - b).abs() < 1e-7) && (ft - fk).abs() < 1e-6 * (1.0 + fk.abs())) {
                continue;
            }
            // direct crossing (no LP): H = 0 and the facet comes from exactly one inactive constraint
            let mut direct = match (&dctx, direct_row(&dctx_rows, nk, fk)) {
                (true, Some(j)) => direct_cross(&lex, &parent, &mut wk, &mut sc, program, &zset, &basis, j, nk),
                _ => None,
            };
            // the neighbour must have the reversed facet as one of its facets; otherwise (a numerically
            // marginal region) the LP path decides
            let mut prebuilt: Option<CriticalRegion> = None;
            let mut cached: Option<(Vec<usize>, Option<CriticalRegion>)> = None;
            if let Some(nb) = &direct {
                let has_reverse = |cr: &CriticalRegion| {
                    cr.cheb_radius.is_some_and(|r| r > FAT) && (0..cr.e.rows).filter_map(|r| unit(cr.e.row(r), cr.f[r])).any(|(n, f)| n.iter().zip(nk).all(|(a, b)| (a + b).abs() < 1e-7) && (f + fk).abs() < 1e-6 * (1.0 + fk.abs()))
                };
                let ok = if let Some(&r2) = index.get(nb) {
                    has_reverse(&regions[r2])
                } else if seen.contains(nb) {
                    false
                } else {
                    let built = gen_cr_from_active_set_lambda_first(program, nb, true);
                    if built.as_ref().is_some_and(|cr| has_reverse(cr)) {
                        prebuilt = built;
                        true
                    } else {
                        cached = Some((nb.clone(), built));
                        false
                    }
                };
                if !ok {
                    direct = None;
                }
            }
            let nb: Vec<usize> = if let Some(nb) = &direct {
                nb.clone()
            } else {
                // relative interior point θ0 of the facet. First try the foot of the region's Chebyshev
                // center on the facet plane: it is in the relative interior when it satisfies every other
                // facet row with a margin (no LP). Otherwise the facet's own Chebyshev center (rows on the
                // same plane are dropped).
                if centre.is_none() {
                    let rows: Vec<Vec<f64>> = facets.iter().map(|(n, _)| n.clone()).collect();
                    let rhs: Vec<f64> = facets.iter().map(|(_, f)| *f).collect();
                    centre = Some({
                        crate::census_purpose!(Neighbour);
                        chebyshev_ball(&Mat::from_rows(&rows), &rhs, &[]).map(|b| b.x[..nt].to_vec())
                    });
                }
                let mut foot: Option<Vec<f64>> = None;
                if let Some(Some(c)) = &centre {
                    let d = fk - nk.iter().zip(c).map(|(a, b)| a * b).sum::<f64>();
                    let p: Vec<f64> = c.iter().zip(nk).map(|(c, n)| c + d * n).collect();
                    let ok = facets.iter().enumerate().all(|(i, (ni, fi))| {
                        if i == k || (ni.iter().zip(nk).all(|(a, b)| (a - b).abs() < 1e-7) && (fi - fk).abs() < 1e-7 * (1.0 + fk.abs())) {
                            return true;
                        }
                        fi - ni.iter().zip(&p).map(|(a, b)| a * b).sum::<f64>() > 1e-6 * (1.0 + fi.abs())
                    });
                    if ok {
                        foot = Some(p);
                    }
                }
                let ball_x: Vec<f64>;
                if let Some(p) = foot {
                    ball_x = p;
                } else {
                    let mut rows: Vec<Vec<f64>> = vec![];
                    let mut rhs: Vec<f64> = vec![];
                    for (i, (ni, fi)) in facets.iter().enumerate() {
                        if i != k && ni.iter().zip(nk).all(|(a, b)| (a - b).abs() < 1e-7) && (fi - fk).abs() < 1e-7 * (1.0 + fk.abs()) {
                            continue;
                        }
                        rows.push(ni.clone());
                        rhs.push(*fi);
                    }
                    let kk = rows.iter().position(|r| r == nk).unwrap_or(0);
                    let ball = {
                        crate::census_purpose!(Neighbour);
                        chebyshev_ball(&Mat::from_rows(&rows), &rhs, &[kk])
                    };
                    let Some(ball) = ball else { continue };
                    if ball.x[nt] <= 1e-9 {
                        continue;
                    }
                    ball_x = ball.x;
                }
                let th0 = &ball_x[..nt];
                // lex data: value at θ0 and coefficient of γ along the outward normal
                let q0 = rhs_at(program, th0);
                let q1 = program.f.matvec(nk);
                let d0 = cost_at(program, th0);
                let d1 = program.h.matvec(nk);
                let cp = artificial(&mut rng, &basis);
                wk.clone_from(&parent);
                let r = lex.solve(&mut wk, &mut sc, &q0, &q1, &d0, &d1, &cp);
                if r.is_err() {
                    continue;
                }
                let mut nb = wk.b.clone();
                nb.sort_unstable();
                nb
            };
            crossed.entry(nb.clone()).or_default().push((nk.iter().map(|v| -v).collect(), -fk));
            if let Some(ncr) = prebuilt {
                seen.insert(nb.clone());
                index.insert(nb.clone(), regions.len());
                regions.push(ncr);
                queue.push_back((regions.len() - 1, nb));
                continue;
            }
            if !seen.insert(nb.clone()) {
                continue;
            }
            let built = match cached {
                Some((cb, cr)) if cb == nb => cr,
                _ => gen_cr_from_active_set_lambda_first(program, &nb, true),
            };
            if let Some(ncr) = built {
                index.insert(nb.clone(), regions.len());
                regions.push(ncr);
                queue.push_back((regions.len() - 1, nb));
            }
        }
    }
    Ok(regions)
}

/// Per region: whether the direct crossing applies (no multiplier facets), the unit rows of the
/// region's inactive-constraint facets with their constraint index, and the nonbasic constraints
/// whose slack is identically zero in θ (their lexicographic sign is decided by the ε columns).
fn region_context(lex: &Lex, program: &MpProgram, cr: &CriticalRegion, basis: &[usize]) -> (bool, Vec<(usize, Vec<f64>, f64)>, Vec<bool>) {
    if !cr.lambda_set.is_empty() {
        return (false, vec![], vec![]);
    }
    let nt = program.num_t();
    let slack_fn = |k: usize| -> (Vec<f64>, f64) {
        // A_k x(θ) - b_k - F_k θ  =  e θ - f  with  e = A_k a - F_k,  f = b_k - A_k b
        let ak = program.a.row(k);
        let mut e = vec![0.0; nt];
        let mut f = program.b[k];
        for i in 0..lex.nx {
            for t in 0..nt {
                e[t] += ak[i] * cr.a[(i, t)];
            }
            f -= ak[i] * cr.b[i];
        }
        for t in 0..nt {
            e[t] -= program.f[(k, t)];
        }
        (e, f)
    };
    let rows: Vec<(usize, Vec<f64>, f64)> = cr
        .regular_set
        .1
        .iter()
        .filter_map(|&j| {
            let (e, f) = slack_fn(j);
            unit(&e, f).map(|(n, f)| (j, n, f))
        })
        .collect();
    let mut inb = vec![false; lex.m];
    for &i in basis {
        inb[i] = true;
    }
    let mut z = vec![false; lex.m];
    for k in 0..lex.m {
        if inb[k] {
            continue;
        }
        let (e, f) = slack_fn(k);
        z[k] = e.iter().all(|v| v.abs() <= 1e-8) && f.abs() <= 1e-8;
    }
    (true, rows, z)
}

/// The inactive constraint whose facet is (nk, fk), if exactly one row of the region matches.
fn direct_row(rows: &[(usize, Vec<f64>, f64)], nk: &[f64], fk: f64) -> Option<usize> {
    let mut found = None;
    for (j, n, f) in rows {
        if n.iter().zip(nk).all(|(a, b)| (a - b).abs() < 1e-7) && (f - fk).abs() < 1e-6 * (1.0 + fk.abs()) {
            if found.is_some() {
                return None;
            }
            found = Some(*j);
        }
    }
    found
}

/// Crosses the facet where the inactive constraint j becomes active (H = 0): one dual
/// lexicographic ratio test with the true cost picks the constraint that leaves. At a relative
/// interior point θ0 of the facet every other nonbasic slack is positive except the identically
/// zero ones, so the pivoted basis is primal feasible at θ0 + γ n and dual feasible (dual simplex),
/// hence the neighbour. The check runs the lexicographic simplex on the pivoted basis with
/// synthetic slack values (1 for ordinary nonbasic rows, 0 for basic rows, j and the zero rows) and
/// accepts only if it does not pivot. A tie in the ratio test returns None (LP path).
#[allow(clippy::too_many_arguments)]
fn direct_cross(lex: &Lex, parent: &Work, wk: &mut Work, sc: &mut Scratch, program: &MpProgram, zset: &[bool], basis: &[usize], j: usize, nk: &[f64]) -> Option<Vec<usize>> {
    let nx = lex.nx;
    let d0 = &program.c;
    let d1 = vec![0.0; nx];
    let mut best: Option<(usize, Vec<f64>)> = None;
    let mut tmp = vec![0.0; nx + 2];
    for p in lex.neq..nx {
        let y = parent.t[j * nx + p];
        if y > 1e-9 {
            lex.lam_row(parent, p, d0, &d1, &mut tmp);
            let r: Vec<f64> = tmp.iter().map(|v| v / y).collect();
            match &best {
                None => best = Some((p, r)),
                Some((_, br)) => match lex_cmp(&r, br) {
                    Ordering::Less => best = Some((p, r)),
                    Ordering::Equal => return None,
                    Ordering::Greater => {}
                },
            }
        }
    }
    let (p, _) = best?;
    wk.clone_from(parent);
    lex.pivot(wk, p, j);
    let mut pivoted = wk.b.clone();
    pivoted.sort_unstable();
    let mut q0 = vec![0.0; lex.m];
    for k in 0..lex.m {
        if !(basis.contains(&k) || k == j || zset[k]) {
            q0[k] = 1.0;
        }
    }
    let q1 = program.f.matvec(nk);
    let cp = vec![0.0; nx];
    lex.solve(wk, sc, &q0, &q1, d0, &d1, &cp).ok()?;
    let mut nb = wk.b.clone();
    nb.sort_unstable();
    (nb == pivoted).then_some(nb)
}

fn rhs_at(p: &MpProgram, th: &[f64]) -> Vec<f64> {
    let mut v = p.f.matvec(th);
    for (x, b) in v.iter_mut().zip(&p.b) {
        *x += b;
    }
    v
}

fn cost_at(p: &MpProgram, th: &[f64]) -> Vec<f64> {
    let mut v = p.h.matvec(th);
    for (x, c) in v.iter_mut().zip(&p.c) {
        *x += c;
    }
    v
}

/// `solve` for the dispatcher: an unsupported program gives an empty list and a warning.
pub fn regions(program: &MpProgram, seed: u64) -> Vec<CriticalRegion> {
    match solve(program, seed) {
        Ok(r) => r,
        Err(msg) => {
            eprintln!("warning: {msg}");
            vec![]
        }
    }
}
