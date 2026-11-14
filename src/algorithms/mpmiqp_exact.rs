//! Exact solution of mp-MIQP: Oberdieck and Pistikopoulos, "Explicit hybrid model-predictive
//! control: the exact solution", Automatica 58 (2015) 152-159.
//!
//! Regions that come from different binary combinations overlap. On an overlap the set where one
//! quadratic value function is lower is bounded by a quadric, so the optimal partition has
//! (possibly) non-convex pieces. Following the paper, a piece is the intersection of a polytope
//! with a list of quadratic constraints `q(theta) <= 0` (Eq. 11), and the comparison proceeds as
//! in Steps 4 to 6 of Algorithm 1:
//!
//! 1. `Delta z = z_new - z_old` (Eq. 16) on the overlap `CR`;
//! 2. classification (Eq. 17, 18): if `Delta z <= 0` everywhere on `CR` the new law wins the whole
//!    overlap, if `Delta z >= 0` everywhere the old law does (the delta_max / delta_min tests are
//!    posed as emptiness tests of `CR ∩ {Delta z > tol}` and `CR ∩ {Delta z < -tol}`);
//! 3. otherwise `CR` is split by `Delta z <= 0` / `Delta z >= 0` (Eq. 19), the quadric being added
//!    to the description of both parts;
//! 4. pieces that are empty, which with quadratic constraints is a non-convex feasibility problem
//!    (Eq. 21, 22), are removed with a spatial branch and bound (Appendix B).
//!
//! Deviations from the paper: the comparison is applied to the enumeration output (one mp-QP per
//! feasible binary combination, as in PPOPT) instead of inside a decomposition or branch and
//! bound loop, so no parametric upper bound with suboptimality epsilon is used. The affine outer
//! approximation of Step 2/3 (McCormick) is not needed because every mp-QP is solved over a
//! polytope: the polytope of a piece is kept exactly and the quadrics are carried alongside. In the
//! spatial branch and bound the relaxation of each quadratic constraint is an affine underestimator
//! built on the spectral decomposition of its Hessian (tangent for convex directions, secant for
//! concave ones) instead of McCormick relaxations of bilinear terms. Both are valid underestimators
//! that become exact as the box shrinks.

use super::mpmiqp::{solve_mpmiqp_enumeration, MpmiProgram};
use super::{MpqpAlgorithm, SolveOptions};
use crate::critical_region::CriticalRegion;
use crate::linalg::{dot, Mat};
use crate::solution::Solution;
use crate::utils::{cheap_remove_redundant_constraints, chebyshev_ball};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

const EPS_R: f64 = 1e-8;
/// A point is accepted as a witness for a quadric when it is this far inside.
const TOL_Q: f64 = 1e-9;
/// Spatial branch and bound: boxes with all edges below this are discarded.
const MIN_BOX: f64 = 1e-6;
/// Pieces cut by quadrics must contain a ball of at least this radius. Slivers thinner than this
/// (typically the neighbourhood of an equal-objective curve) would otherwise need on the order of
/// 1/sqrt(EPS) boxes along the curve to be rejected.
const EPS_Q: f64 = 1e-6;
/// Spatial branch and bound node limit; above it the answer is "not empty" (safe: it only
/// causes an extra split, never a gap).
const MAX_NODES: usize = 1500;
/// Tangent-cut refinements per spatial branch and bound node before bisecting.
const KELLEY_ITERS: usize = 6;
/// Relax / tighten rounds per node.
const OBBT_ROUNDS: usize = 3;

static N_CHECKS: AtomicUsize = AtomicUsize::new(0);
static N_BB_NODES: AtomicUsize = AtomicUsize::new(0);
static N_CAPPED: AtomicUsize = AtomicUsize::new(0);

// ------------------------------------------------------------------------------------ quadrics

/// `q(theta) = 1/2 theta' M theta + l' theta + c` with `M` symmetric.
#[derive(Clone, Debug)]
pub struct Quad {
    pub m: Mat,
    pub l: Vec<f64>,
    pub c: f64,
    lam: Vec<f64>,
    vecs: Mat, // columns are eigenvectors of M
}

impl Quad {
    pub fn new(m: Mat, l: Vec<f64>, c: f64) -> Quad {
        let n = m.rows;
        let mut sym = Mat::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                sym[(i, j)] = 0.5 * (m[(i, j)] + m[(j, i)]);
            }
        }
        let (lam, vecs) = if n > 0 && sym.max_abs() > 0.0 {
            let e = sym.to_nalgebra().symmetric_eigen();
            (e.eigenvalues.iter().cloned().collect(), Mat::from_nalgebra(&e.eigenvectors))
        } else {
            (vec![0.0; n], Mat::eye(n))
        };
        Quad { m: sym, l, c, lam, vecs }
    }

    #[inline]
    pub fn value(&self, th: &[f64]) -> f64 {
        0.5 * dot(th, &self.m.matvec(th)) + dot(&self.l, th) + self.c
    }

    pub fn is_convex(&self) -> bool {
        self.lam.iter().all(|&v| v >= -1e-12)
    }

    /// `self ≈ sign * other` coefficient-wise (both normalized to unit largest coefficient).
    fn close(&self, other: &Quad, sign: f64) -> bool {
        const T: f64 = 1e-7;
        (self.c - sign * other.c).abs() <= T
            && self.l.iter().zip(&other.l).all(|(a, b)| (a - sign * b).abs() <= T)
            && (0..self.m.rows).all(|i| (0..self.m.cols).all(|j| (self.m[(i, j)] - sign * other.m[(i, j)]).abs() <= T))
    }

    fn neg(&self) -> Quad {
        Quad::new(self.m.neg(), self.l.iter().map(|v| -v).collect(), -self.c)
    }

    /// Affine underestimator `a' theta + b <= q(theta)` on the box.
    /// Convex directions are linearized at `at` (clamped to the box; the box midpoint when
    /// `None`), concave directions use the secant.
    fn under(&self, lo: &[f64], hi: &[f64], at: Option<&[f64]>) -> (Vec<f64>, f64) {
        let n = lo.len();
        let mut a = self.l.clone();
        let mut b = self.c;
        for k in 0..n {
            let lk = self.lam[k];
            if lk == 0.0 {
                continue;
            }
            let (mut slo, mut shi) = (0.0, 0.0);
            for j in 0..n {
                let v = self.vecs[(j, k)];
                if v >= 0.0 {
                    slo += v * lo[j];
                    shi += v * hi[j];
                } else {
                    slo += v * hi[j];
                    shi += v * lo[j];
                }
            }
            // s = v_k' theta; term 1/2 lam s^2 >= alpha s + beta
            let (alpha, beta) = if lk > 0.0 {
                let s0 = match at {
                    Some(c) => (0..n).map(|j| self.vecs[(j, k)] * c[j]).sum::<f64>().clamp(slo, shi),
                    None => 0.5 * (slo + shi),
                };
                (lk * s0, -0.5 * lk * s0 * s0)
            } else {
                (0.5 * lk * (slo + shi), -0.5 * lk * slo * shi)
            };
            for j in 0..n {
                a[j] += alpha * self.vecs[(j, k)];
            }
            b += beta;
        }
        (a, b)
    }
}

/// Polytope `e theta <= f` intersected with the quadratic constraints `q <= 0`.
#[derive(Clone, Debug)]
pub struct GRegion {
    pub e: Mat,
    pub f: Vec<f64>,
    pub quads: Vec<Quad>,
}

impl GRegion {
    pub fn poly(e: Mat, f: Vec<f64>) -> GRegion {
        GRegion { e, f, quads: vec![] }
    }

    fn with_row(&self, row: &[f64], rhs: f64) -> GRegion {
        let (e, f) = with_row(&self.e, &self.f, row, rhs);
        GRegion { e, f, quads: self.quads.clone() }
    }

    /// Adds `q <= 0`. A quadric with zero Hessian becomes a row of the polytope; constraints are
    /// normalized to unit largest coefficient. `None` when `q` is a positive constant.
    fn with_quad(&self, q: &Quad) -> Option<GRegion> {
        let sm = q.m.max_abs();
        let sl = q.l.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        if sm == 0.0 && sl <= 1e-13 {
            return if q.c <= 1e-12 { Some(self.clone()) } else { None };
        }
        if sm == 0.0 {
            return Some(self.with_row(&q.l, -q.c));
        }
        let s = sm.max(sl);
        let mut qq = Quad::new(q.m.scale(1.0 / s), q.l.iter().map(|v| v / s).collect(), q.c / s);
        qq.c = q.c / s;
        // a quadric already present (same boundary): redundant if on the same side, and the
        // region degenerates to the boundary itself (a sliver) if on the opposite side
        for old in &self.quads {
            if old.close(&qq, 1.0) {
                return Some(self.clone());
            }
            if old.close(&qq, -1.0) {
                return None;
            }
        }
        let mut out = self.clone();
        out.quads.push(qq);
        Some(out)
    }

    fn merge(&self, o: &GRegion) -> GRegion {
        let (e, f) = stack(&self.e, &self.f, &o.e, &o.f);
        let mut quads = self.quads.clone();
        quads.extend(o.quads.iter().cloned());
        GRegion { e, f, quads }
    }

    pub fn contains(&self, th: &[f64], tol: f64) -> bool {
        (0..self.e.rows).all(|i| dot(self.e.row(i), th) - self.f[i] <= tol) && self.quads.iter().all(|q| q.value(th) <= tol)
    }
}

// ------------------------------------------------------------------------------------ geometry

fn radius(e: &Mat, f: &[f64]) -> f64 {
    match chebyshev_ball(e, f, &[]) {
        Some(s) => s.x[e.cols],
        None => -1.0,
    }
}

fn stack(e1: &Mat, f1: &[f64], e2: &Mat, f2: &[f64]) -> (Mat, Vec<f64>) {
    let e = Mat::vstack(&[e1, e2]);
    let mut f = f1.to_vec();
    f.extend_from_slice(f2);
    (e, f)
}

fn with_row(e: &Mat, f: &[f64], row: &[f64], rhs: f64) -> (Mat, Vec<f64>) {
    let mut e = e.clone();
    e.push_row(row);
    let mut f = f.to_vec();
    f.push(rhs);
    (e, f)
}

fn exact_reduce(e: &Mat, f: &[f64]) -> (Mat, Vec<f64>) {
    let (e, f) = cheap_remove_redundant_constraints(e, f);
    let mut keep: Vec<usize> = (0..e.rows).collect();
    let mut i = 0;
    while i < keep.len() {
        let row = e.row(keep[i]);
        let others: Vec<usize> = keep.iter().cloned().filter(|&j| j != keep[i]).collect();
        let sub = e.select_rows(&others);
        let subf: Vec<f64> = others.iter().map(|&j| f[j]).collect();
        let neg: Vec<f64> = row.iter().map(|v| -v).collect();
        let redundant = match crate::solver::lp::solve_lp(Some(&neg), &sub, &subf, &[]) {
            Some(s) => -s.obj <= f[keep[i]] + 1e-9,
            None => false,
        };
        if redundant {
            keep.remove(i);
        } else {
            i += 1;
        }
    }
    (e.select_rows(&keep), keep.iter().map(|&j| f[j]).collect())
}

fn tidy(g: GRegion) -> GRegion {
    let (e, f) = cheap_remove_redundant_constraints(&g.e, &g.f);
    let (e, f) = if e.rows > 12 { exact_reduce(&e, &f) } else { (e, f) };
    GRegion { e, f, quads: g.quads }
}

/// Is the region full dimensional? With quadrics this is the non-convex feasibility problem of
/// Eq. (22), solved by spatial branch and bound (Appendix B of the paper): bound the region by a
/// box, relax every quadric to an affine underestimator on the box, discard the box when the
/// relaxed polytope is empty or thin, accept it when the center of the relaxed polytope satisfies
/// the true quadrics, otherwise bisect the longest edge.
pub fn nonempty(g: &GRegion) -> bool {
    let nc = N_CHECKS.fetch_add(1, Ordering::Relaxed);
    if nc % 5000 == 0 && std::env::var("EXACT_TRACE").is_ok() {
        eprintln!("  check {} nodes {} capped {}", nc, N_BB_NODES.load(Ordering::Relaxed), N_CAPPED.load(Ordering::Relaxed));
    }
    if radius(&g.e, &g.f) <= EPS_R {
        return false;
    }
    if g.quads.is_empty() {
        return true;
    }
    let n = g.e.cols;
    let mut lo0 = vec![0.0; n];
    let mut hi0 = vec![0.0; n];
    for j in 0..n {
        let mut c = vec![0.0; n];
        c[j] = 1.0;
        match crate::solver::lp::solve_lp(Some(&c), &g.e, &g.f, &[]) {
            Some(s) => lo0[j] = s.x[j],
            None => return true,
        }
        c[j] = -1.0;
        match crate::solver::lp::solve_lp(Some(&c), &g.e, &g.f, &[]) {
            Some(s) => hi0[j] = s.x[j],
            None => return true,
        }
    }
    let mut stk = vec![(lo0, hi0)];
    let mut nodes = 0usize;
    while let Some((mut lo, mut hi)) = stk.pop() {
        nodes += 1;
        N_BB_NODES.fetch_add(1, Ordering::Relaxed);
        if nodes > MAX_NODES {
            N_CAPPED.fetch_add(1, Ordering::Relaxed);
            return true;
        }
        let mut pruned = false;
        // rounds of: relax on the box, Kelley tangent cuts, then bound tightening of the box
        // (optimality-based bound tightening: min / max of every coordinate over the relaxation)
        for _round in 0..OBBT_ROUNDS {
            let mut e = g.e.clone();
            let mut f = g.f.clone();
            for j in 0..n {
                let mut r = vec![0.0; n];
                r[j] = 1.0;
                e.push_row(&r);
                f.push(hi[j]);
                r[j] = -1.0;
                e.push_row(&r);
                f.push(-lo[j]);
            }
            for q in &g.quads {
                let (a, b) = q.under(&lo, &hi, None);
                e.push_row(&a);
                f.push(-b);
            }
            for it in 0..KELLEY_ITERS {
                let Some(ball) = chebyshev_ball(&e, &f, &[]) else {
                    pruned = true;
                    break;
                };
                if ball.x[n] <= EPS_Q {
                    pruned = true;
                    break;
                }
                let center = ball.x[..n].to_vec();
                if g.quads.iter().all(|q| q.value(&center) <= -TOL_Q) {
                    return true;
                }
                if it + 1 == KELLEY_ITERS {
                    break;
                }
                for q in &g.quads {
                    if q.value(&center) > -TOL_Q {
                        let (a, b) = q.under(&lo, &hi, Some(&center));
                        e.push_row(&a);
                        f.push(-b);
                    }
                }
            }
            if pruned {
                break;
            }
            let (mut nlo, mut nhi) = (lo.clone(), hi.clone());
            let mut shrink = 0.0f64;
            for j in 0..n {
                let mut c = vec![0.0; n];
                c[j] = 1.0;
                let Some(sl) = crate::solver::lp::solve_lp(Some(&c), &e, &f, &[]) else {
                    pruned = true;
                    break;
                };
                c[j] = -1.0;
                let Some(sh) = crate::solver::lp::solve_lp(Some(&c), &e, &f, &[]) else {
                    pruned = true;
                    break;
                };
                let w0 = hi[j] - lo[j];
                nlo[j] = sl.x[j].max(lo[j]);
                nhi[j] = sh.x[j].min(hi[j]).max(nlo[j]);
                if w0 > 0.0 {
                    shrink = shrink.max(1.0 - (nhi[j] - nlo[j]) / w0);
                }
            }
            if pruned {
                break;
            }
            lo = nlo;
            hi = nhi;
            if shrink < 0.2 {
                break;
            }
        }
        if pruned {
            continue;
        }
        let (mut d, mut w) = (0, 0.0);
        for j in 0..n {
            if hi[j] - lo[j] > w {
                w = hi[j] - lo[j];
                d = j;
            }
        }
        if w < MIN_BOX {
            continue;
        }
        let mid = 0.5 * (lo[d] + hi[d]);
        let mut hi1 = hi.clone();
        hi1[d] = mid;
        let mut lo2 = lo.clone();
        lo2[d] = mid;
        stk.push((lo, hi1));
        stk.push((lo2, hi));
    }
    false
}

/// `a` minus `b` as a list of disjoint full dimensional pieces: one piece per constraint of `b`
/// (polytope rows first, then quadrics), the j-th violating constraint j and satisfying 1..j-1.
fn gdiff(a: &GRegion, b: &GRegion) -> Vec<GRegion> {
    let mut out = vec![];
    let mut acc = a.clone();
    for j in 0..b.e.rows {
        let row = b.e.row(j);
        if row.iter().all(|v| v.abs() < 1e-14) {
            continue;
        }
        let neg: Vec<f64> = row.iter().map(|v| -v).collect();
        let cut = acc.with_row(&neg, -b.f[j]);
        if nonempty(&cut) {
            out.push(tidy(cut));
        }
        acc = acc.with_row(row, b.f[j]);
        if !nonempty(&acc) {
            return out;
        }
    }
    for q in &b.quads {
        if let Some(cut) = acc.with_quad(&q.neg()) {
            if nonempty(&cut) {
                out.push(cut);
            }
        }
        match acc.with_quad(q) {
            Some(a2) => acc = a2,
            None => return out,
        }
        if !nonempty(&acc) {
            return out;
        }
    }
    out
}

// ------------------------------------------------------------------------------------ pieces

#[derive(Clone)]
struct Piece {
    reg: GRegion,
    z: Quad,
    src: usize,
}

impl Piece {
    fn with_reg(&self, reg: GRegion) -> Piece {
        Piece { reg, z: self.z.clone(), src: self.src }
    }
}

fn clean(v: f64, scale: f64) -> f64 {
    if v.abs() <= 1e-9 * scale {
        0.0
    } else {
        v
    }
}

/// Exact comparison of an existing piece `s` and a new piece `n` (Steps 4 and 5): returns the
/// replacements of `s` and of `n`; on the overlap the lower value function wins and `s` keeps ties.
fn compare_pair(s: &Piece, n: &Piece) -> (Vec<Piece>, Vec<Piece>) {
    let nt = s.z.l.len();
    // Delta z = z_n - z_s
    let sm = s.z.m.max_abs().max(n.z.m.max_abs());
    let sl = s.z.l.iter().chain(&n.z.l).fold(0.0f64, |a, v| a.max(v.abs()));
    let sc = 1.0f64.max(s.z.c.abs()).max(n.z.c.abs());
    let mut dm = n.z.m.sub(&s.z.m);
    for i in 0..nt {
        for j in 0..nt {
            dm[(i, j)] = clean(dm[(i, j)], sm);
        }
    }
    let dl: Vec<f64> = n.z.l.iter().zip(&s.z.l).map(|(a, b)| clean(a - b, sl.max(1e-12))).collect();
    let dc = clean(n.z.c - s.z.c, sc);
    let delta = Quad::new(dm, dl, dc);
    let diff_s = |r: &GRegion| gdiff(&s.reg, r).into_iter().map(|g| s.with_reg(g)).collect::<Vec<_>>();
    let diff_n = |r: &GRegion| gdiff(&n.reg, r).into_iter().map(|g| n.with_reg(g)).collect::<Vec<_>>();
    let tol = 1e-9 * sc;
    if delta.m.max_abs() == 0.0 && delta.l.iter().all(|&v| v == 0.0) && delta.c.abs() <= tol {
        return (vec![s.clone()], diff_n(&s.reg));
    }
    let ov = s.reg.merge(&n.reg);
    // n loses everywhere on the overlap iff {Delta z + tol <= 0} ∩ overlap is empty
    let mut shifted = delta.clone();
    shifted.c += tol;
    let n_better_somewhere = ov.with_quad(&shifted).map_or(false, |g| nonempty(&g));
    if !n_better_somewhere {
        return (vec![s.clone()], diff_n(&s.reg));
    }
    let mut shifted_neg = delta.neg();
    shifted_neg.c += tol;
    let s_better_somewhere = ov.with_quad(&shifted_neg).map_or(false, |g| nonempty(&g));
    if !s_better_somewhere {
        return (diff_s(&n.reg), vec![n.clone()]);
    }
    // split by Delta z <= 0 (n wins) and Delta z >= 0 (s wins)
    let r = n.reg.with_quad(&delta);
    let t = s.reg.with_quad(&delta.neg());
    let s_rem = match r {
        Some(r) => diff_s(&r),
        None => vec![s.clone()],
    };
    let n_rem = match t {
        Some(t) => diff_n(&t),
        None => vec![n.clone()],
    };
    (s_rem, n_rem)
}

fn overlaps(s: &Piece, n: &Piece) -> bool {
    nonempty(&s.reg.merge(&n.reg))
}

fn resolve(sp: Piece, ns: Vec<Piece>) -> (Vec<Piece>, Vec<Piece>) {
    let mut n_final = vec![];
    for idx in 0..ns.len() {
        if !overlaps(&sp, &ns[idx]) {
            n_final.push(ns[idx].clone());
            continue;
        }
        let (sp_rem, n_rem) = compare_pair(&sp, &ns[idx]);
        n_final.extend(n_rem);
        let mut rest: Vec<Piece> = ns[idx + 1..].to_vec();
        let mut s_final = vec![];
        for s2 in sp_rem {
            let (sf, r2) = resolve(s2, rest);
            s_final.extend(sf);
            rest = r2;
        }
        n_final.extend(rest);
        return (s_final, n_final);
    }
    (vec![sp], n_final)
}

fn insert_piece(pieces: &mut Vec<Piece>, new: Piece) {
    let mut n_parts = vec![new];
    let mut out = Vec::with_capacity(pieces.len() + 1);
    for s in pieces.drain(..) {
        if n_parts.is_empty() {
            out.push(s);
            continue;
        }
        let (sf, nf) = resolve(s, std::mem::take(&mut n_parts));
        out.extend(sf);
        n_parts = nf;
    }
    out.extend(n_parts);
    *pieces = out;
}

// ------------------------------------------------------------------------------------ result

/// One piece of the exact partition: law of a binary combination on a polytope cut by quadrics.
#[derive(Clone)]
pub struct ExactPiece {
    /// `x*(theta)`, binaries and `e theta <= f` (the polytope part of the piece)
    pub region: CriticalRegion,
    /// quadratic constraints `q(theta) <= 0` that cut the polytope (empty for convex pieces)
    pub quads: Vec<Quad>,
    /// optimal value function on the piece
    pub z: Quad,
}

impl ExactPiece {
    pub fn contains(&self, th: &[f64], tol: f64) -> bool {
        (0..self.region.e.rows).all(|i| dot(self.region.e.row(i), th) - self.region.f[i] <= tol) && self.quads.iter().all(|q| q.value(th) <= tol)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ExactStats {
    pub enumeration_regions: usize,
    pub pieces: usize,
    pub nonconvex_pieces: usize,
    pub max_quads: usize,
    pub emptiness_checks: usize,
    pub bb_nodes: usize,
    pub bb_capped: usize,
    pub enum_secs: f64,
    pub compare_secs: f64,
}

pub struct ExactSolution {
    pub program: Arc<MpmiProgram>,
    pub pieces: Vec<ExactPiece>,
}

impl ExactSolution {
    pub fn len(&self) -> usize {
        self.pieces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    /// First piece containing theta (tolerance `tol` on every constraint; boundaries are shared
    /// between pieces and the value function is continuous there).
    pub fn locate(&self, th: &[f64], tol: f64) -> Option<usize> {
        self.pieces.iter().position(|p| p.contains(th, tol))
    }

    /// [`locate`](Self::locate) that falls back to a looser tolerance (slivers thinner than the
    /// emptiness threshold are dropped from the partition, so a point on one is off by <= 1e-6).
    pub fn locate_robust(&self, th: &[f64]) -> Option<usize> {
        self.locate(th, 1e-9).or_else(|| self.locate(th, 2e-6))
    }

    /// Number of pieces containing theta; a negative `tol` counts only pieces that contain it
    /// with at least that slack (used to test for ambiguity away from boundaries).
    pub fn count_containing(&self, th: &[f64], tol: f64) -> usize {
        self.pieces.iter().filter(|p| p.contains(th, tol)).count()
    }

    pub fn evaluate(&self, th: &[f64]) -> Option<Vec<f64>> {
        self.locate_robust(th).map(|i| self.pieces[i].region.evaluate(th))
    }

    pub fn evaluate_objective(&self, th: &[f64]) -> Option<f64> {
        let i = self.locate_robust(th)?;
        Some(self.pieces[i].z.value(th))
    }
}

/// Value function `z(theta)` of a region as an exact quadratic (polarization of the objective).
fn region_value_function(p: &MpmiProgram, cr: &CriticalRegion) -> Quad {
    let nt = p.num_t();
    let z = |th: &[f64]| p.base.evaluate_objective(&cr.evaluate(th), th);
    let zero = vec![0.0; nt];
    let c = z(&zero);
    let mut zp = vec![0.0; nt];
    let mut zm = vec![0.0; nt];
    let mut m = Mat::zeros(nt, nt);
    let mut l = vec![0.0; nt];
    for i in 0..nt {
        let mut t = zero.clone();
        t[i] = 1.0;
        zp[i] = z(&t);
        t[i] = -1.0;
        zm[i] = z(&t);
        m[(i, i)] = zp[i] + zm[i] - 2.0 * c;
        l[i] = 0.5 * (zp[i] - zm[i]);
    }
    for i in 0..nt {
        for j in i + 1..nt {
            let mut t = zero.clone();
            t[i] = 1.0;
            t[j] = 1.0;
            let v = z(&t) - zp[i] - zp[j] + c;
            m[(i, j)] = v;
            m[(j, i)] = v;
        }
    }
    Quad::new(m, l, c)
}

/// Exact comparison (Oberdieck and Pistikopoulos 2015) applied to an overlapping enumeration
/// solution. The result is a partition of the feasible set into polytope-with-quadrics pieces,
/// each holding the optimal binary combination, law and value function.
pub fn compare_regions_exact(program: &Arc<MpmiProgram>, enumeration: &Solution) -> (ExactSolution, ExactStats) {
    N_CHECKS.store(0, Ordering::Relaxed);
    N_BB_NODES.store(0, Ordering::Relaxed);
    N_CAPPED.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let templates = &enumeration.critical_regions;
    let mut pieces: Vec<Piece> = vec![];
    for (i, cr) in templates.iter().enumerate() {
        let z = region_value_function(program, cr);
        insert_piece(&mut pieces, Piece { reg: GRegion::poly(cr.e.clone(), cr.f.clone()), z, src: i });
        if std::env::var("EXACT_TRACE").is_ok() {
            eprintln!("inserted {}/{} -> {} pieces, {} checks, {} nodes, {:.2}s", i + 1, templates.len(), pieces.len(), N_CHECKS.load(Ordering::Relaxed), N_BB_NODES.load(Ordering::Relaxed), t0.elapsed().as_secs_f64());
        }
    }
    let mut out = Vec::with_capacity(pieces.len());
    let mut nonconvex = 0;
    let mut max_quads = 0;
    for pc in pieces {
        let mut cr = templates[pc.src].clone();
        let (e, f) = exact_reduce(&pc.reg.e, &pc.reg.f);
        cr.e = e;
        cr.f = f;
        cr.cheb_radius = None;
        if !pc.reg.quads.is_empty() {
            nonconvex += 1;
        }
        max_quads = max_quads.max(pc.reg.quads.len());
        out.push(ExactPiece { region: cr, quads: pc.reg.quads, z: pc.z });
    }
    let stats = ExactStats {
        enumeration_regions: templates.len(),
        pieces: out.len(),
        nonconvex_pieces: nonconvex,
        max_quads,
        emptiness_checks: N_CHECKS.load(Ordering::Relaxed),
        bb_nodes: N_BB_NODES.load(Ordering::Relaxed),
        bb_capped: N_CAPPED.load(Ordering::Relaxed),
        enum_secs: 0.0,
        compare_secs: t0.elapsed().as_secs_f64(),
    };
    (ExactSolution { program: program.clone(), pieces: out }, stats)
}

/// Enumeration (PPOPT) followed by the exact comparison.
pub fn solve_mpmiqp_exact(program: &Arc<MpmiProgram>, alg: MpqpAlgorithm, opts: &SolveOptions) -> (ExactSolution, ExactStats, Solution) {
    let t0 = Instant::now();
    let en = solve_mpmiqp_enumeration(program, alg, opts);
    let enum_secs = t0.elapsed().as_secs_f64();
    let (sol, mut st) = compare_regions_exact(program, &en);
    st.enum_secs = enum_secs;
    (sol, st, en)
}
