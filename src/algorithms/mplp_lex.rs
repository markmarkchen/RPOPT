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
use crate::linalg::{dot, norm, rank, Lu, Mat};
use crate::mpqp_utils::gen_cr_from_active_set_lambda_first;
use crate::program::MpProgram;
use crate::utils::chebyshev_ball;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::cmp::Ordering;
use std::collections::VecDeque;

const TOL: f64 = 1e-9;

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
    /// width of the primal lex vectors (value, γ, ε_1..ε_m)
    w1: usize,
    /// width of the dual lex vectors (value, γ, δ_1..δ_n)
    w2: usize,
}

/// Quantities of a basis: β = A_B^{-1}, the slack lex rows of the nonbasic constraints and the
/// multiplier lex rows of the basis (rows follow the order of the basis list).
struct State {
    binv: Mat,
    slack: Vec<f64>,
    lam: Vec<f64>,
}

impl<'a> Lex<'a> {
    fn new(a: &'a Mat, neq: usize) -> Self {
        Lex { a, neq, m: a.rows, nx: a.cols, w1: 2 + a.rows, w2: 2 + a.cols }
    }

    /// Lex right hand side [q0 | q1 | I] (m x w1) and lex cost [d0 | d1 | I] (nx x w2).
    fn lex_data(&self, q0: &[f64], q1: &[f64], d0: &[f64], d1: &[f64]) -> (Mat, Mat) {
        let mut q = Mat::zeros(self.m, self.w1);
        for j in 0..self.m {
            q[(j, 0)] = q0[j];
            q[(j, 1)] = q1[j];
            q[(j, 2 + j)] = 1.0;
        }
        let mut d = Mat::zeros(self.nx, self.w2);
        for k in 0..self.nx {
            d[(k, 0)] = d0[k];
            d[(k, 1)] = d1[k];
            d[(k, 2 + k)] = 1.0;
        }
        (q, d)
    }

    fn state(&self, b: &[usize], q: &Mat, d: &Mat) -> Result<State, Fail> {
        let (nx, m, w1, w2) = (self.nx, self.m, self.w1, self.w2);
        let ab = self.a.select_rows(b);
        let lu = Lu::new(&ab).ok_or(Fail::Singular)?;
        let binv = lu.solve_mat(&Mat::eye(nx));
        let x = binv.matmul(&q.select_rows(b));
        let mut in_b = vec![false; m];
        for &i in b {
            in_b[i] = true;
        }
        let mut slack = vec![0.0; m * w1];
        for j in 0..m {
            if in_b[j] {
                continue;
            }
            let aj = self.a.row(j);
            let out = &mut slack[j * w1..(j + 1) * w1];
            for c in 0..w1 {
                let mut s = q[(j, c)];
                let mut mag = s.abs();
                for k in 0..nx {
                    let t = aj[k] * x[(k, c)];
                    s -= t;
                    mag += t.abs();
                }
                // cancellation noise of the numeric columns counts as an exact tie
                if c < 2 && s.abs() < 1e-12 * mag {
                    s = 0.0;
                }
                out[c] = s;
            }
        }
        // multipliers  λ_B = -β' D
        let mut lam = vec![0.0; nx * w2];
        for p in 0..nx {
            for c in 0..w2 {
                let mut s = 0.0;
                let mut mag = 0.0;
                for k in 0..nx {
                    let t = binv[(k, p)] * d[(k, c)];
                    s -= t;
                    mag += t.abs();
                }
                if c < 2 && s.abs() < 1e-12 * mag {
                    s = 0.0;
                }
                lam[p * w2 + c] = s;
            }
        }
        Ok(State { binv, slack, lam })
    }

    /// Lexicographic simplex from the basis `b` (equality rows in the first `neq` positions) for
    /// the lex data (q, d). Phase 1 (dual simplex with the artificial multipliers `cp`, which
    /// keeps the dual feasible and removes lex-infeasible slack rows) gives a lex-feasible basis,
    /// phase 2 (primal simplex with the lexicographic ratio test (7)) the lex-optimal one.
    fn solve(&self, q: &Mat, d: &Mat, mut b: Vec<usize>, cp: &[f64]) -> Result<Vec<usize>, Fail> {
        let (nx, m, w1, w2) = (self.nx, self.m, self.w1, self.w2);
        let cap = 200 + 20 * (m + nx);
        for _ in 0..cap {
            let st = self.state(&b, q, d)?;
            let mut in_b = vec![false; m];
            for &i in &b {
                in_b[i] = true;
            }
            // phase 1: a lex-negative slack row
            let mut worst: Option<usize> = None;
            for j in self.neq..m {
                if in_b[j] {
                    continue;
                }
                let row = &st.slack[j * w1..(j + 1) * w1];
                if lex_sign(row) < 0 {
                    match worst {
                        Some(w) if lex_cmp(row, &st.slack[w * w1..(w + 1) * w1]) != Ordering::Less => {}
                        _ => worst = Some(j),
                    }
                }
            }
            if let Some(j) = worst {
                let aj = self.a.row(j);
                let mut pick: Option<(usize, f64)> = None;
                for p in self.neq..nx {
                    let mut y = 0.0;
                    let mut l = 0.0;
                    for k in 0..nx {
                        y += aj[k] * st.binv[(k, p)];
                        l -= st.binv[(k, p)] * cp[k];
                    }
                    if y > 1e-9 {
                        let r = l / y;
                        if pick.map_or(true, |(_, br)| r < br) {
                            pick = Some((p, r));
                        }
                    }
                }
                let Some((p, _)) = pick else { return Err(Fail::Infeasible) };
                b[p] = j;
                continue;
            }
            // phase 2: a lex-negative multiplier row leaves
            let mut leave: Option<usize> = None;
            for p in self.neq..nx {
                let row = &st.lam[p * w2..(p + 1) * w2];
                if lex_sign(row) < 0 {
                    match leave {
                        Some(l) if lex_cmp(row, &st.lam[l * w2..(l + 1) * w2]) != Ordering::Less => {}
                        _ => leave = Some(p),
                    }
                }
            }
            let Some(p) = leave else { return Ok(b) };
            // edge direction  dir = -β e_p ; lexicographic ratio test over A_j dir > 0
            let dir: Vec<f64> = (0..nx).map(|k| -st.binv[(k, p)]).collect();
            let mut best: Option<(usize, Vec<f64>)> = None;
            for j in self.neq..m {
                if in_b[j] {
                    continue;
                }
                let aj = dot(self.a.row(j), &dir);
                if aj > 1e-9 {
                    let ratio: Vec<f64> = st.slack[j * w1..(j + 1) * w1].iter().map(|v| v / aj).collect();
                    if best.as_ref().map_or(true, |(_, br)| lex_cmp(&ratio, br) == Ordering::Less) {
                        best = Some((j, ratio));
                    }
                }
            }
            let Some((j, _)) = best else { return Err(Fail::Unbounded) };
            b[p] = j;
        }
        Err(Fail::Stalled)
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
        let (q, d) = lex.lex_data(&q0, &vec![0.0; m], &d0, &vec![0.0; nx]);
        let cp = artificial(&mut rng, &b0);
        match lex.solve(&q, &d, b0.clone(), &cp) {
            Ok(mut basis) => {
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
    let mut queue: VecDeque<(CriticalRegion, Vec<usize>)> = VecDeque::new();
    regions.push(cr0.clone());
    queue.push_back((cr0, basis0));
    while let Some((cr, basis)) = queue.pop_front() {
        let ne = cr.e.rows;
        let facets: Vec<(Vec<f64>, f64)> = (0..ne).filter_map(|k| unit(cr.e.row(k), cr.f[k])).collect();
        for k in 0..facets.len() {
            let (nk, fk) = (&facets[k].0, facets[k].1);
            // facet on the boundary of Θ
            if theta_rows.iter().any(|(t, ft)| t.iter().zip(nk).all(|(a, b)| (a - b).abs() < 1e-7) && (ft - fk).abs() < 1e-6 * (1.0 + fk.abs())) {
                continue;
            }
            // relative interior point θ0 of the facet (rows on the same plane are dropped)
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
            let Some(ball) = chebyshev_ball(&Mat::from_rows(&rows), &rhs, &[kk]) else { continue };
            if ball.x[nt] <= 1e-9 {
                continue;
            }
            let th0 = &ball.x[..nt];
            // lex data: value at θ0 and coefficient of γ along the outward normal
            let q0 = rhs_at(program, th0);
            let q1 = program.f.matvec(nk);
            let d0 = cost_at(program, th0);
            let d1 = program.h.matvec(nk);
            let (q, d) = lex.lex_data(&q0, &q1, &d0, &d1);
            let start: Vec<usize> = {
                // basis in the order used by the lex machinery: equality rows first
                let mut s = basis.clone();
                s.sort_unstable();
                s
            };
            let cp = artificial(&mut rng, &start);
            let Ok(mut nb) = lex.solve(&q, &d, start, &cp) else { continue };
            nb.sort_unstable();
            if !seen.insert(nb.clone()) {
                continue;
            }
            if let Some(ncr) = gen_cr_from_active_set_lambda_first(program, &nb, true) {
                regions.push(ncr.clone());
                queue.push_back((ncr, nb));
            }
        }
    }
    Ok(regions)
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
