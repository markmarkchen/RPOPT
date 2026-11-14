//! Approximate mpQP with an a priori error bound.
//!
//! Bemporad and Filippi, "Suboptimal explicit receding horizon control via approximate
//! multiparametric quadratic programming", JOTA 117(1):9-38, 2003, Case C of Section 4
//! (relaxation of the multiplier nonnegativity only) with the a priori bound of Section 4.6.
//!
//! For an optimal active set N (multipliers lambda_N, Hessian Q, rows A_N) the exact critical region
//! is {primal feasibility of the inactive rows, lambda_N(theta) >= 0}. The approximate critical region
//! relaxes the second condition to lambda_N(theta) >= -delta3 (paper (16)):
//!   * the affine optimizer of the exact active set stays primal feasible on the whole relaxed region
//!     (the primal rows are not relaxed), so every returned optimizer is feasible;
//!   * the value function error is bounded by Lemma 4.3 (paper (18)):
//!       phi_hat(theta) - phi*(theta) <= 1/2 delta3' (A_N Q^-1 A_N') delta3,
//!     so delta3 = beta * 1 with beta = sqrt(2 eps / 1'(A_N Q^-1 A_N')1) guarantees error <= eps
//!     on the region (equality multipliers are free and are not relaxed);
//!   * optionally the optimizer error bound of Theorem 4.2 (paper (24)):
//!       beta <= rho / sqrt(1'(A_N Q^-1 A_N')1) * min_i 1/sqrt([Q^-1]_ii).
//!
//! The rest of the parameter space is explored recursively with the partition of Theorem 4.1: the
//! remaining polyhedron R is cut by the non-redundant facets of the approximate region into
//! R_i = R and {facet i reversed} and {facets j < i kept}; a new parameter vector (a Chebyshev
//! center of the feasible part of R_i) starts the next region. Pieces without a full dimensional
//! feasible part are dropped.
//!
//! Two ways of exploring the remaining space are provided (`Exploration`). `Clipped` is the paper's
//! recursion; its region count is larger than the exact solution's at small epsilon because the
//! slabs of Theorem 4.1 cut the exact regions (about 30x on the 4-parameter problems, and it
//! only drops below the exact count for large epsilon). `Subtract` (opt-in, uses the overlap
//! allowed by Remark 4.2) keeps the unclipped regions and removes each from all uncovered
//! polyhedra; it gives fewer regions than the exact solution, but the list of uncovered
//! polyhedra grows to tens of thousands, so it is only practical for small problems.
//!
//! Deviations from the paper: only Case C (no projection) is implemented, not the Case A/B
//! least-squares fit (17); the relative error mode uses a global lower bound of the value function
//! (eps = eps_abs + eps_rel * max(min phi*, 0)) because the paper gives absolute bounds only; the
//! optimizer error bound weights all x components (W = I). The approximate regions overlap on
//! shared facets only (closed pieces, remark 4.2).

use crate::critical_region::CriticalRegion;
use crate::linalg::{dot, is_full_rank_rows, norm, Cholesky, Mat};
use crate::program::MpProgram;
use crate::solution::Solution;
use crate::utils::{chebyshev_ball, find_redundant_constraints, remove_duplicate_rows};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ApproxOptions {
    /// absolute bound on phi_hat - phi*
    pub eps_abs: f64,
    /// relative part, see the module docs (0 disables it)
    pub eps_rel: f64,
    /// optional bound rho on max |x_hat(theta) - x*(theta)| (paper Theorem 4.2)
    pub rho_optimizer: Option<f64>,
    pub seed: u64,
    pub exploration: Exploration,
}

/// How the part of the parameter space not yet covered is explored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exploration {
    /// Paper section 4 literally: each approximate region is clipped to the polyhedron R_i it was
    /// started in, and R_i minus the region is split by Theorem 4.1 (regions never overlap, but the
    /// slabs of the recursion fragment the partition: many more regions than the exact solution).
    Clipped,
    /// Remark 4.2: regions are the unclipped approximate critical regions (they may overlap). The
    /// uncovered set is kept as a list of polyhedra; each new region is subtracted from every
    /// polyhedron of the list with the construction of Theorem 4.1. At eps = 0 this reproduces the
    /// exact regions, and the count only drops as eps grows.
    Subtract,
}

impl ApproxOptions {
    pub fn abs(eps: f64) -> Self {
        ApproxOptions { eps_abs: eps, eps_rel: 0.0, rho_optimizer: None, seed: 0, exploration: Exploration::Clipped }
    }
}

/// Uncovered polyhedron {E theta <= f}; `lo`/`hi` is an outer bounding box (inherited from the
/// parent piece, so it is valid but not tight) used to discard empty cuts without an LP.
struct Ctx<'a> {
    xt_a: &'a Mat,
    xt_b: &'a [f64],
    m: usize,
    nx: usize,
    eq: &'a [usize],
}

struct Piece {
    e: Mat,
    f: Vec<f64>,
    lo: Vec<f64>,
    hi: Vec<f64>,
    /// (center, radius) of the Chebyshev ball of the feasible part, when already known
    ball: Option<(Vec<f64>, f64)>,
}

/// piece minus {g_rows theta <= g_rhs}: for each cut facet i in `cuts`, the part with facet i
/// reversed and the facets before it kept (Theorem 4.1). Cuts without a full dimensional part are dropped.
fn subtract(q: &Piece, g_rows: &[Vec<f64>], g_rhs: &[f64], cuts: &[usize], ctx: &Ctx, want_bbox: bool) -> Vec<Piece> {
    let nt = q.e.cols;
    let mut out = vec![];
    for (pos, &gi) in cuts.iter().enumerate() {
        let a = &g_rows[gi];
        // the reversed facet needs a'theta >= g somewhere in the box
        let ub: f64 = a.iter().zip(q.lo.iter().zip(&q.hi)).map(|(ak, (l, h))| (ak * l).max(ak * h)).sum();
        if ub < g_rhs[gi] - 1e-9 {
            continue;
        }
        let mut pe = Mat::zeros(q.e.rows + 1 + pos, nt);
        let mut pf = Vec::with_capacity(q.e.rows + 1 + pos);
        for i in 0..q.e.rows {
            pe.row_mut(i).copy_from_slice(q.e.row(i));
            pf.push(q.f[i]);
        }
        for (k, v) in a.iter().enumerate() {
            pe[(q.e.rows, k)] = -v;
        }
        pf.push(-g_rhs[gi]);
        for (j, &gj) in cuts[..pos].iter().enumerate() {
            pe.row_mut(q.e.rows + 1 + j).copy_from_slice(&g_rows[gj]);
            pf.push(g_rhs[gj]);
        }
        // the piece must contain a full dimensional set of feasible parameters: Chebyshev ball of the
        // joint (x, theta) polytope restricted to the piece (pieces outside the feasible set are dropped
        // here instead of being cut up by every later region)
        let mut joint = Mat::zeros(ctx.m + pe.rows, ctx.nx + nt);
        for i in 0..ctx.m {
            joint.row_mut(i).copy_from_slice(&ctx.xt_a.row(i)[..ctx.nx + nt]);
        }
        for i in 0..pe.rows {
            joint.row_mut(ctx.m + i)[ctx.nx..].copy_from_slice(pe.row(i));
        }
        let mut jb = ctx.xt_b[..ctx.m].to_vec();
        jb.extend_from_slice(&pf);
        match chebyshev_ball(&joint, &jb, ctx.eq) {
            Some(s) if s.x[ctx.nx + nt] > RADIUS_TOL => {
                // exact bounding box (2 n_t LPs from the Chebyshev center): later cuts of this piece
                // are mostly discarded by the box test alone
                let c = s.x[ctx.nx..ctx.nx + nt].to_vec();
                let mut lo = q.lo.clone();
                let mut hi = q.hi.clone();
                let mut obj = vec![0.0; nt];
                for k in 0..if want_bbox { nt } else { 0 } {
                    obj[k] = 1.0;
                    if let Some(x) = crate::solver::lp::lp_point_from(Some(&obj), &pe, &pf, &[], &c) {
                        lo[k] = x[k];
                    }
                    obj[k] = -1.0;
                    if let Some(x) = crate::solver::lp::lp_point_from(Some(&obj), &pe, &pf, &[], &c) {
                        hi[k] = x[k];
                    }
                    obj[k] = 0.0;
                }
                out.push(Piece { e: pe, f: pf, lo, hi, ball: Some((c, s.x[ctx.nx + nt])) });
            }
            _ => {}
        }
    }
    out
}

#[derive(Clone, Debug, Default)]
pub struct ApproxStats {
    pub regions: usize,
    pub qp_solves: usize,
    /// pieces of the partition discarded because no region could be started (degenerate points)
    pub dropped_pieces: usize,
    /// effective absolute bound used (after the relative part)
    pub eps_effective: f64,
}

/// Approximate mpQP with absolute value function error bound `eps` (see module docs).
/// Falls back to the exact geometric algorithm for programs without a strictly convex Q.
pub fn solve_mpqp_approx(program: &Arc<MpProgram>, eps: f64) -> Solution {
    solve_mpqp_approx_with(program, &ApproxOptions::abs(eps)).0
}

const RADIUS_TOL: f64 = 1e-7;

pub fn solve_mpqp_approx_with(program: &Arc<MpProgram>, opts: &ApproxOptions) -> (Solution, ApproxStats) {
    let mut stats = ApproxStats::default();
    let Some(q) = program.q.as_ref() else {
        let sol = crate::algorithms::solve_mpqp(program, crate::algorithms::MpqpAlgorithm::Geometric);
        stats.regions = sol.len();
        return (sol, stats);
    };
    let nx = program.num_x();
    let nt = program.num_t();
    let neq = program.num_equality_constraints();
    let kc = program.kkt_cache_pub().expect("approximate mpQP needs a strictly convex Q");
    let qinv = Cholesky::new(q).expect("Q must be positive definite").solve_mat(&Mat::eye(nx));
    let min_inv_sqrt_diag = (0..nx).map(|i| 1.0 / qinv[(i, i)].sqrt()).fold(f64::INFINITY, f64::min);

    let eps_eff = opts.eps_abs + if opts.eps_rel > 0.0 { opts.eps_rel * value_lower_bound(program).max(0.0) } else { 0.0 };
    stats.eps_effective = eps_eff;

    let (xt_a, xt_b) = program.xtheta_polytope();
    let m = program.num_constraints();
    let mut rng = StdRng::seed_from_u64(opts.seed);

    // R starts as the parameter set A_t theta <= b_t (normalised rows)
    let (e0, f0) = normalize_rows(&program.a_t, &program.b_t);
    let ctx = Ctx { xt_a: &xt_a, xt_b: &xt_b, m, nx, eq: &program.equality_indices };
    let subtract_mode = opts.exploration == Exploration::Subtract;
    let (blo, bhi) = program.theta_bbox().cloned().expect("bounded parameter set");
    let mut stack: Vec<Piece> = vec![Piece { e: e0.clone(), f: f0.clone(), lo: blo, hi: bhi, ball: None }];
    let mut regions: Vec<CriticalRegion> = vec![];

    while let Some(piece) = stack.pop() {
        let (er, fr) = (&piece.e, &piece.f);
        // rows bounding the region itself: the whole parameter set (Subtract) or the current piece (Clipped)
        let (cr_e, cr_f): (&Mat, &Vec<f64>) = if subtract_mode { (&e0, &f0) } else { (er, fr) };
        // joint (x, theta) polytope restricted to R; its Chebyshev ball gives a point strictly inside
        // the feasible part of R (radius > 0 iff that part is full dimensional)
        let (center, radius): (Vec<f64>, f64) = match &piece.ball {
            Some((c, r)) => (c.clone(), *r),
            None => {
                let mut joint = Mat::zeros(m + er.rows, nx + nt);
                for i in 0..m {
                    joint.row_mut(i).copy_from_slice(&xt_a.row(i)[..nx + nt]);
                }
                for i in 0..er.rows {
                    joint.row_mut(m + i)[nx..].copy_from_slice(er.row(i));
                }
                let mut jb = xt_b[..m].to_vec();
                jb.extend_from_slice(fr);
                let Some(ball) = chebyshev_ball(&joint, &jb, &program.equality_indices) else { continue };
                (ball.x[nx..nx + nt].to_vec(), ball.x[nx + nt])
            }
        };
        if radius <= RADIUS_TOL {
            continue;
        }

        let mut started = None;
        for attempt in 0..10 {
            let th: Vec<f64> = if attempt == 0 {
                center.clone()
            } else {
                // stay inside the projected ball (radius >= r in theta space)
                let dir: Vec<f64> = (0..nt).map(|_| crate::program::standard_normal(&mut rng)).collect();
                let n = norm(&dir).max(1e-300);
                center.iter().zip(&dir).map(|(c, d)| c + 0.5 * radius * rng.gen_range(0.0..1.0) * d / n).collect()
            };
            stats.qp_solves += 1;
            let sol = program.solve_theta_uncounted(&th);
            let Some(sol) = sol else { continue };
            // maximal linearly independent subset of the active set (paper section 3.1)
            let mut n_set: Vec<usize> = sol.active_set.clone();
            if !is_full_rank_rows(&program.a, &n_set) {
                let mut keep: Vec<usize> = n_set.iter().cloned().filter(|i| *i < neq).collect();
                for &i in &sol.active_set {
                    if i >= neq {
                        keep.push(i);
                        if !is_full_rank_rows(&program.a, &keep) {
                            keep.pop();
                        }
                    }
                }
                n_set = keep;
            }
            let Some(law) = program.optimal_control_law(&n_set) else { continue };
            // x0 must lie in the (exact) region of N, otherwise the relaxation is not centred on it
            let lam = |t: &[f64]| -> Vec<f64> { (0..n_set.len()).map(|k| dot(law.a_l.row(k), t) + law.b_l[k]).collect() };
            if lam(&th).iter().skip(neq).any(|&l| l < -1e-9) {
                continue;
            }
            started = Some((th, n_set, law));
            break;
        }
        let Some((_th, n_set, law)) = started else {
            stats.dropped_pieces += 1;
            continue;
        };

        // relaxation delta3 = beta * 1 on the inequality multipliers (equalities are free)
        let ineq_pos: Vec<usize> = (neq..n_set.len()).collect();
        let mut s_sum = 0.0;
        for &a in &ineq_pos {
            for &b in &ineq_pos {
                s_sum += kc.m[(n_set[a], n_set[b])];
            }
        }
        let beta = if s_sum > 1e-14 {
            let mut b = (2.0 * eps_eff / s_sum).sqrt();
            if let Some(rho) = opts.rho_optimizer {
                b = b.min(rho / s_sum.sqrt() * min_inv_sqrt_diag);
            }
            b
        } else {
            0.0
        };

        let (e_all, f_all, n_lambda, inactive) = program.region_rows(&n_set, &law);
        let n_g = n_lambda + inactive.len();
        let mut g_rows: Vec<Vec<f64>> = Vec::new();
        let mut g_rhs: Vec<f64> = Vec::new();
        let mut g_src: Vec<(bool, usize)> = Vec::new(); // (is_lambda, index in lambda list / inactive list)
        for r in 0..n_g {
            let mut row = e_all.row(r).to_vec();
            let mut rhs = f_all[r];
            if r < n_lambda {
                rhs += beta;
            }
            let nrm = norm(&row);
            if nrm <= 1e-8 {
                continue;
            }
            for v in row.iter_mut() {
                *v /= nrm;
            }
            rhs /= nrm;
            g_rows.push(row);
            g_rhs.push(rhs);
            g_src.push(if r < n_lambda { (true, r) } else { (false, r - n_lambda) });
        }
        let ng = g_rows.len();
        let mut all = Mat::zeros(ng + cr_e.rows, nt);
        let mut all_b = Vec::with_capacity(ng + cr_e.rows);
        for (i, r) in g_rows.iter().enumerate() {
            all.row_mut(i).copy_from_slice(r);
            all_b.push(g_rhs[i]);
        }
        for i in 0..cr_e.rows {
            all.row_mut(ng + i).copy_from_slice(cr_e.row(i));
            all_b.push(cr_f[i]);
        }
        // drop rows that cannot bind inside the parameter box, then the redundant ones
        let kept = non_redundant_rows(program, &all, &all_b);
        let kept_g: Vec<usize> = kept.iter().cloned().filter(|&i| i < ng).collect();
        let e_red = all.select_rows(&kept);
        let f_red: Vec<f64> = kept.iter().map(|&i| all_b[i]).collect();
        let (e_red, f_red) = remove_duplicate_rows(&e_red, &f_red);

        if subtract_mode {
            // subtract the new region from the current piece and from every other uncovered piece
            let old = std::mem::take(&mut stack);
            for q in std::iter::once(piece).chain(old) {
                stack.extend(subtract(&q, &g_rows, &g_rhs, &kept_g, &ctx, true));
            }
        } else {
            // Theorem 4.1: pieces R_i = R, facet i reversed, facets j < i kept
            stack.extend(subtract(&piece, &g_rows, &g_rhs, &kept_g, &ctx, false));
        }

        let lambda_set: Vec<usize> = kept_g.iter().filter(|&&i| g_src[i].0).map(|&i| n_set[neq + g_src[i].1]).collect();
        let reg_pos: Vec<usize> = kept_g.iter().filter(|&&i| !g_src[i].0).map(|&i| g_src[i].1).collect();
        let reg_idx: Vec<usize> = reg_pos.iter().map(|&p| inactive[p]).collect();
        regions.push(CriticalRegion {
            a: law.a_x,
            b: law.b_x,
            c: law.a_l,
            d: law.b_l,
            e: e_red,
            f: f_red,
            active_set: n_set,
            omega_set: vec![],
            lambda_set,
            regular_set: (reg_pos, reg_idx),
            y_fixation: None,
            y_indices: None,
            x_indices: None,
            cheb_radius: None,
        });
    }
    stats.regions = regions.len();
    let mut sol = Solution::new(program.clone(), regions);
    sol.is_overlapping = false;
    (sol, stats)
}

fn normalize_rows(a: &Mat, b: &[f64]) -> (Mat, Vec<f64>) {
    let mut e = a.clone();
    let mut f = b.to_vec();
    for i in 0..e.rows {
        let n = norm(e.row(i));
        if n > 0.0 {
            for v in e.row_mut(i) {
                *v /= n;
            }
            f[i] /= n;
        }
    }
    (e, f)
}

/// Indices of rows of {E theta <= f} that touch the polytope (PPOPT redundancy test), after the
/// cheap bounding-box reduction.
fn non_redundant_rows(program: &MpProgram, e: &Mat, f: &[f64]) -> Vec<usize> {
    // rows implied by the parameter box are dropped without an LP (map back to original indices)
    let mut cand: Vec<usize> = (0..e.rows).collect();
    if let Some((lo, hi)) = program.theta_bbox() {
        cand.retain(|&i| {
            let ub: f64 = e.row(i).iter().zip(lo.iter().zip(hi)).map(|(a, (l, h))| (a * l).max(a * h)).sum();
            !(ub < f[i] - 1e-9 * (1.0 + f[i].abs()))
        });
    }
    let sub = e.select_rows(&cand);
    let subf: Vec<f64> = cand.iter().map(|&i| f[i]).collect();
    // rows are unit norm. A row is kept iff its hyperplane touches the polytope. As in
    // `gen_cr_from_kkt`: a row whose maximum over the polytope's bounding box is below its rhs cannot
    // be reached; one whose hyperplane contains the projection of the Chebyshev center onto it
    // (inside the polytope) is reached; only the rest need an LP, started at the center.
    let nt = sub.cols;
    let center = match chebyshev_ball(&sub, &subf, &[]) {
        Some(s) if s.x[nt] > 1e-9 => s.x[..nt].to_vec(),
        _ => return find_redundant_constraints(&sub, &subf, &[]).into_iter().map(|k| cand[k]).collect(),
    };
    let mut bbox: Option<(Vec<f64>, Vec<f64>)> = Some((vec![0.0; nt], vec![0.0; nt]));
    let mut obj = vec![0.0; nt];
    'bb: for k in 0..nt {
        obj[k] = 1.0;
        let lo = crate::solver::lp::lp_point_from(Some(&obj), &sub, &subf, &[], &center);
        obj[k] = -1.0;
        let hi = crate::solver::lp::lp_point_from(Some(&obj), &sub, &subf, &[], &center);
        obj[k] = 0.0;
        match (lo, hi, bbox.as_mut()) {
            (Some(l), Some(h), Some(b)) => {
                b.0[k] = l[k];
                b.1[k] = h[k];
            }
            _ => {
                bbox = None;
                break 'bb;
            }
        }
    }
    let mut kept = vec![];
    for p in 0..sub.rows {
        let a = sub.row(p);
        let fp = subf[p];
        let tol = 1e-9 * (1.0 + fp.abs());
        if let Some((lo, hi)) = &bbox {
            let ub: f64 = a.iter().zip(lo.iter().zip(hi)).map(|(ak, (l, h))| (ak * l).max(ak * h)).sum();
            if ub < fp - 1e-7 * (1.0 + fp.abs()) {
                continue;
            }
        }
        let sft = fp - dot(a, &center);
        let q: Vec<f64> = center.iter().zip(a).map(|(c, ai)| c + sft * ai).collect();
        let inside = (0..sub.rows).all(|i| i == p || dot(sub.row(i), &q) <= subf[i] + 1e-9 * (1.0 + subf[i].abs()));
        if (inside && (dot(a, &q) - fp).abs() <= tol) || crate::solver::lp::lp_feasible_from(&sub, &subf, &[p], &center) {
            kept.push(cand[p]);
        }
    }
    kept
}

/// min over the feasible (x, theta) of the joint objective (a lower bound of phi* on the feasible
/// set), 0 if it cannot be computed (non-convex or semidefinite joint Hessian).
fn value_lower_bound(program: &MpProgram) -> f64 {
    let Some(q) = program.q.as_ref() else { return 0.0 };
    let nx = program.num_x();
    let nt = program.num_t();
    let n = nx + nt;
    let mut hess = Mat::zeros(n, n);
    for i in 0..nx {
        for j in 0..nx {
            hess[(i, j)] = q[(i, j)];
        }
        for t in 0..nt {
            hess[(i, nx + t)] = program.h[(i, t)];
            hess[(nx + t, i)] = program.h[(i, t)];
        }
    }
    for s in 0..nt {
        for t in 0..nt {
            hess[(nx + s, nx + t)] = program.q_t[(s, t)];
        }
    }
    // tiny ridge so the joint QP solver accepts a semidefinite Hessian (lowers the bound only)
    let ridge = 1e-9;
    for i in 0..n {
        hess[(i, i)] += ridge;
    }
    let mut c = program.c.clone();
    c.extend_from_slice(&program.c_t);
    let (pa, pb) = program.xtheta_polytope();
    match crate::solver::qp::solve_qp(&hess, Some(&c), &pa, &pb, &program.equality_indices) {
        Some(s) => {
            let v = s.obj + program.c_c;
            v - 1e-6 * (1.0 + v.abs())
        }
        None => 0.0,
    }
}
