//! mp-MILP beyond enumeration.
//!
//! * [`compare_regions`]: the comparison step of Dua and Pistikopoulos (Ann. Oper. Res. 99, 2000).
//!   Regions that come from different binary combinations overlap; on every overlap an LP decides
//!   which objective is lower and, when neither dominates, the overlap is cut along the hyperplane
//!   where the two affine objectives are equal. The result is a non-overlapping partition holding
//!   the optimal binary combination and law of each piece.
//! * [`solve_mpmilp_bb`]: branch and bound in the parameter space (Oberdieck, Wittmann-Hohlbein,
//!   Pistikopoulos, J. Global Optim. 59, 2014). The mpLP relaxation is solved at each node, the
//!   parts of its regions whose relaxed objective is not below the incumbent's parametric upper
//!   bound are pruned, relaxed regions with integral binaries update the incumbent through the
//!   comparison step, and the remaining regions are branched on a fractional binary.
//! * General integers: [`binary_expansion`] (baseline) and native branching `y <= k` / `y >= k+1`
//!   in [`solve_mpmilp_bb_general`].
//!
//! Only programs whose optimal objective is affine in theta are supported (mpMILP with `H = 0`),
//! since the comparison hyperplane is the zero set of the difference of two affine functions.
//! The `c`, `d`, `active_set` fields of regions produced by B&B refer to the relaxed node problem.

use super::mpmiqp::MpmiProgram;
use super::{solve_mpqp_with, MpqpAlgorithm, SolveOptions};
use crate::critical_region::CriticalRegion;
use crate::linalg::{dot, Mat};
use crate::program::MpProgram;
use crate::solution::Solution;
use crate::utils::{chebyshev_ball, cheap_remove_redundant_constraints};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;
use std::time::Instant;

/// Minimum Chebyshev radius for a piece to count as full dimensional.
const EPS_R: f64 = 1e-8;

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

/// Drops every row implied by the others (one LP per row, sequentially), keeping the polytope exact.
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

fn tidy(e: Mat, f: Vec<f64>) -> (Mat, Vec<f64>) {
    let (e, f) = cheap_remove_redundant_constraints(&e, &f);
    if e.rows > 12 {
        return exact_reduce(&e, &f);
    }
    (e, f)
}

/// P minus Q as a list of disjoint full-dimensional convex pieces.
fn diff(pe: &Mat, pf: &[f64], qe: &Mat, qf: &[f64]) -> Vec<(Mat, Vec<f64>)> {
    let mut out = vec![];
    let (mut ae, mut af) = (pe.clone(), pf.to_vec());
    for j in 0..qe.rows {
        let row = qe.row(j);
        if row.iter().all(|v| v.abs() < 1e-14) {
            continue;
        }
        let neg: Vec<f64> = row.iter().map(|v| -v).collect();
        let (ce, cf) = with_row(&ae, &af, &neg, -qf[j]);
        if radius(&ce, &cf) > EPS_R {
            out.push(tidy(ce, cf));
        }
        let (ne, nf) = with_row(&ae, &af, row, qf[j]);
        ae = ne;
        af = nf;
        if radius(&ae, &af) <= EPS_R {
            break;
        }
    }
    out
}

fn range_of(g: &[f64], k: f64, e: &Mat, f: &[f64]) -> Option<(f64, f64)> {
    let lo = crate::solver::lp::solve_lp(Some(g), e, f, &[])?.obj + k;
    let ng: Vec<f64> = g.iter().map(|v| -v).collect();
    let hi = -crate::solver::lp::solve_lp(Some(&ng), e, f, &[])?.obj + k;
    Some((lo, hi))
}

// ------------------------------------------------------------------------------------ pieces

/// A convex piece of the partition with the affine objective of the law attached to it.
#[derive(Clone)]
struct Piece {
    e: Mat,
    f: Vec<f64>,
    g: Vec<f64>,
    k: f64,
    src: usize,
    /// binary vectors already analysed on this piece (integer cuts of the decomposition algorithm)
    tried: Vec<Vec<i8>>,
    /// no feasible solution known yet (upper bound +infinity)
    inf: bool,
}

impl Piece {
    fn with_poly(&self, (e, f): (Mat, Vec<f64>)) -> Piece {
        Piece { e, f, g: self.g.clone(), k: self.k, src: self.src, tried: self.tried.clone(), inf: self.inf }
    }
}

fn tol_for(a: f64, b: f64) -> f64 {
    1e-9 * 1.0f64.max(a.abs()).max(b.abs())
}

/// Dua-Pistikopoulos comparison of an existing piece `s` and a new piece `n`. Returns the
/// replacements of `s` and of `n`; on the overlap the lower objective wins (the existing piece
/// keeps ties).
fn compare_pair(s: &Piece, n: &Piece) -> (Vec<Piece>, Vec<Piece>) {
    let (ie, i_f) = stack(&s.e, &s.f, &n.e, &n.f);
    let d: Vec<f64> = s.g.iter().zip(&n.g).map(|(a, b)| a - b).collect(); // f_s - f_n
    let kd = s.k - n.k;
    let tol = tol_for(s.k, n.k);
    let (lo, hi) = range_of(&d, kd, &ie, &i_f).unwrap_or((0.0, 0.0));
    let diff_s = |q: (&Mat, &[f64])| diff(&s.e, &s.f, q.0, q.1).into_iter().map(|p| s.with_poly(p)).collect::<Vec<_>>();
    let diff_n = |q: (&Mat, &[f64])| diff(&n.e, &n.f, q.0, q.1).into_iter().map(|p| n.with_poly(p)).collect::<Vec<_>>();
    if hi <= tol {
        // f_s <= f_n + tol on the overlap: s is never worse, n loses the whole overlap
        (vec![s.clone()], diff_n((&s.e, &s.f)))
    } else if lo >= -tol {
        // f_n <= f_s + tol: n is never worse
        (diff_s((&n.e, &n.f)), vec![n.clone()])
    } else {
        // n better where d >= 0 (f_s >= f_n), s better where d <= 0
        let neg: Vec<f64> = d.iter().map(|v| -v).collect();
        let (re, rf) = with_row(&n.e, &n.f, &neg, kd); // n ∩ {d' th + kd >= 0}
        let (se, sf) = with_row(&s.e, &s.f, &d, -kd); // s ∩ {d' th + kd <= 0}
        (diff_s((&re, &rf)), diff_n((&se, &sf)))
    }
}

fn overlaps(s: &Piece, n: &Piece) -> bool {
    let (e, f) = stack(&s.e, &s.f, &n.e, &n.f);
    radius(&e, &f) > EPS_R
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

/// Inserts `new` into the non-overlapping partition `pieces`, keeping the lower objective on every
/// overlap.
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

/// Parts of the polytope (e, f) where the affine objective (g, k) is below the incumbent's upper
/// bound, i.e. what survives bound-based pruning.
fn prune_against(e: &Mat, f: &[f64], g: &[f64], k: f64, inc: &[Piece]) -> Vec<(Mat, Vec<f64>)> {
    let mut parts = vec![(e.clone(), f.to_vec())];
    for s in inc {
        let mut next = vec![];
        for (pe, pf) in parts {
            let (ie, i_f) = stack(&pe, &pf, &s.e, &s.f);
            if radius(&ie, &i_f) <= EPS_R {
                next.push((pe, pf));
                continue;
            }
            let d: Vec<f64> = s.g.iter().zip(g).map(|(a, b)| a - b).collect(); // UB - relaxed
            let kd = s.k - k;
            let tol = tol_for(s.k, k);
            let (lo, hi) = range_of(&d, kd, &ie, &i_f).unwrap_or((0.0, 0.0));
            if lo > tol {
                next.push((pe, pf));
            } else if hi <= tol {
                next.extend(diff(&pe, &pf, &s.e, &s.f));
            } else {
                // remove s ∩ {d' th + kd <= tol}
                let (re, rf) = with_row(&s.e, &s.f, &d, tol - kd);
                next.extend(diff(&pe, &pf, &re, &rf));
            }
        }
        parts = next;
        if parts.is_empty() {
            break;
        }
    }
    parts
}

// ------------------------------------------------------------------------------------ helpers

fn check_affine(p: &MpmiProgram) -> Result<(), String> {
    let b = &p.base;
    if b.q.is_some() {
        return Err("mpMILP comparison needs a linear objective (q = None)".into());
    }
    if b.h.max_abs() > 1e-12 || b.q_t.max_abs() > 1e-12 {
        return Err("objective is not affine in theta (H != 0)".into());
    }
    Ok(())
}

/// Affine objective (g, k) of a region of the mixed-integer program (x(theta) = A theta + b with
/// binaries fixed).
fn region_affine(p: &MpmiProgram, cr: &CriticalRegion) -> (Vec<f64>, f64) {
    let b = &p.base;
    let nt = b.num_t();
    let mut g = b.c_t.clone();
    let mut k = b.c_c;
    for (i, &xi) in p.cont_indices.iter().enumerate() {
        let c = b.c[xi];
        for j in 0..nt {
            g[j] += c * cr.a[(i, j)];
        }
        k += c * cr.b[i];
    }
    if let Some(y) = &cr.y_fixation {
        for (kk, &yi) in p.binary_indices.iter().enumerate() {
            k += b.c[yi] * y[kk];
        }
    }
    (g, k)
}

fn pieces_to_solution(p: &Arc<MpmiProgram>, templates: &[CriticalRegion], pieces: Vec<Piece>, reduce: bool) -> Solution {
    let mut regions = Vec::with_capacity(pieces.len());
    for pc in pieces {
        let mut cr = templates[pc.src].clone();
        let (e, f) = if reduce { exact_reduce(&pc.e, &pc.f) } else { (pc.e.clone(), pc.f.clone()) };
        cr.e = e;
        cr.f = f;
        cr.cheb_radius = None;
        regions.push(cr);
    }
    Solution { program: p.clone(), continuous: None, critical_regions: regions, is_overlapping: false, point_location_tolerance: 1e-7 }
}

// ------------------------------------------------------------------------------------ comparison

/// Dua-Pistikopoulos comparison applied to an overlapping enumeration solution. The returned
/// solution is a non-overlapping partition of the same feasible set.
pub fn compare_regions(program: &Arc<MpmiProgram>, enumeration: &Solution) -> Result<Solution, String> {
    check_affine(program)?;
    let templates: Vec<CriticalRegion> = enumeration.critical_regions.clone();
    let mut pieces: Vec<Piece> = vec![];
    for (i, cr) in templates.iter().enumerate() {
        if cr.e.rows == 0 {
            return Err("region without constraints".into());
        }
        let (g, k) = region_affine(program, cr);
        insert_piece(&mut pieces, Piece { e: cr.e.clone(), f: cr.f.clone(), g, k, src: i, tried: vec![], inf: false });
    }
    Ok(pieces_to_solution(program, &templates, pieces, true))
}

/// Enumeration (PPOPT) followed by the comparison step.
pub fn solve_mpmilp_enumeration_compared(program: &Arc<MpmiProgram>, alg: MpqpAlgorithm, opts: &SolveOptions) -> Result<Solution, String> {
    let en = super::mpmiqp::solve_mpmiqp_enumeration(program, alg, opts);
    compare_regions(program, &en)
}

// ------------------------------------------------------------------------------------ B&B

#[derive(Clone, Debug, Default)]
pub struct BbStats {
    /// mpLP relaxations solved (one per tree node)
    pub nodes: usize,
    /// relaxed regions generated over all nodes
    pub relaxed_regions: usize,
    /// fractional relaxed regions removed entirely by the parametric upper bound
    pub pruned_regions: usize,
    /// integral regions offered to the incumbent
    pub incumbent_updates: usize,
    /// distinct binary fixations visited (tree nodes in the sense of the paper)
    pub distinct_fixations: usize,
    /// regions of the final partition
    pub regions: usize,
    pub seconds: f64,
}

struct NodeRegion {
    cr: CriticalRegion,
    g: Vec<f64>,
    k: f64,
    /// binary position (into binary_indices) to branch on; None when integral
    branch: Option<usize>,
    /// complete binary vector for integral regions
    y: Vec<f64>,
}

/// Builds and solves the mpLP relaxation with `fixed[j] = Some(v)` binaries substituted and the
/// remaining binaries relaxed to [0, 1], over the polytope (te, tf) of theta.
fn solve_node(p: &MpmiProgram, fixed: &[Option<f64>], te: &Mat, tf: &[f64], alg: MpqpAlgorithm, opts: &SolveOptions) -> Vec<NodeRegion> {
    let b = &p.base;
    let nt = b.num_t();
    let nb = p.binary_indices.len();
    let nc = p.cont_indices.len();
    let mut free: Vec<usize> = p.cont_indices.clone();
    let mut free_bins = vec![];
    for j in 0..nb {
        if fixed[j].is_none() {
            free.push(p.binary_indices[j]);
            free_bins.push(j);
        }
    }
    let fix_cols: Vec<usize> = (0..nb).filter(|&j| fixed[j].is_some()).map(|j| p.binary_indices[j]).collect();
    let fix_vals: Vec<f64> = (0..nb).filter_map(|j| fixed[j]).collect();
    let a_free = b.a.select_cols(&free);
    let a_fix = b.a.select_cols(&fix_cols);
    let afix_y = if fix_cols.is_empty() { vec![0.0; b.num_constraints()] } else { a_fix.matvec(&fix_vals) };
    let mut kept_eq = vec![];
    let mut kept_in = vec![];
    for i in 0..b.num_constraints() {
        let no_free = a_free.row(i).iter().all(|v| v.abs() <= 1e-8);
        let no_par = b.f.row(i).iter().all(|v| v.abs() <= 1e-8);
        let is_eq = b.equality_indices.contains(&i);
        if no_free && no_par {
            let slack = b.b[i] - afix_y[i];
            if (is_eq && slack.abs() > 1e-8) || (!is_eq && slack < -1e-8) {
                return vec![]; // fixation infeasible
            }
            continue;
        }
        if is_eq {
            kept_eq.push(i)
        } else {
            kept_in.push(i)
        }
    }
    let mut kept = kept_eq.clone();
    kept.extend(kept_in);
    let new_eq: Vec<usize> = (0..kept_eq.len()).collect();
    let mut a = a_free.select_rows(&kept);
    let mut rhs: Vec<f64> = kept.iter().map(|&i| b.b[i] - afix_y[i]).collect();
    let f = b.f.select_rows(&kept);
    for pos in 0..free_bins.len() {
        let mut up = vec![0.0; free.len()];
        up[nc + pos] = 1.0;
        a.push_row(&up);
        rhs.push(1.0);
        let mut lo = vec![0.0; free.len()];
        lo[nc + pos] = -1.0;
        a.push_row(&lo);
        rhs.push(0.0);
    }
    let f = Mat::vstack(&[&f, &Mat::zeros(2 * free_bins.len(), nt)]);
    // the program usually already holds 0 <= y <= 1; exact duplicate rows make Graph/Geometric fail
    let (a, rhs, f) = {
        let comb = Mat::hstack(&[&a, &f]);
        let (c2, r2) = crate::utils::remove_duplicate_rows(&comb, &rhs);
        let cols: Vec<usize> = (0..a.cols).collect();
        let fcols: Vec<usize> = (a.cols..a.cols + nt).collect();
        (c2.select_cols(&cols), r2, c2.select_cols(&fcols))
    };
    let c_free: Vec<f64> = free.iter().map(|&i| b.c[i]).collect();
    let c_c = b.c_c + fix_cols.iter().zip(&fix_vals).map(|(&i, v)| b.c[i] * v).sum::<f64>();
    let h_free = b.h.select_rows(&free);
    let mut sub = MpProgram::new(a, rhs, c_free, h_free, None, te.clone(), tf.to_vec(), f, new_eq, true);
    sub.c_c = c_c;
    sub.c_t = b.c_t.clone();
    let sol = solve_mpqp_with(&Arc::new(sub), alg, opts);
    let mut out = vec![];
    for cr in sol.critical_regions {
        let mut g = b.c_t.clone();
        let mut k = c_c;
        for (r, &vi) in free.iter().enumerate() {
            let c = b.c[vi];
            for j in 0..nt {
                g[j] += c * cr.a[(r, j)];
            }
            k += c * cr.b[r];
        }
        let mut branch = None;
        let mut y = vec![0.0; nb];
        for j in 0..nb {
            if let Some(v) = fixed[j] {
                y[j] = v;
            }
        }
        for (pos, &j) in free_bins.iter().enumerate() {
            let row = cr.a.row(nc + pos);
            let bv = cr.b[nc + pos];
            let const_row = row.iter().all(|v| v.abs() <= 1e-7);
            let near = (bv - bv.round()).abs() <= 1e-7 && bv.round() >= -1e-7 && bv.round() <= 1.0 + 1e-7;
            if const_row && near {
                y[j] = bv.round();
            } else if branch.is_none() {
                branch = Some(j);
            }
        }
        out.push(NodeRegion { cr, g, k, branch, y });
    }
    out
}

/// Options of [`solve_mpmilp_bb_with`].
#[derive(Clone, Copy, Debug)]
pub struct BbOptions {
    /// true: children are solved only over the surviving part of the parent region (one mpLP per
    /// surviving region, as in Oberdieck et al.); false: every binary fixation is solved once over
    /// the whole parameter set and a fixation is expanded when any of its regions survives.
    pub restrict_domain: bool,
    /// remove redundant facets of the final regions (exact LP test); false keeps the raw cuts
    pub reduce_final: bool,
}

/// Branch and bound for mpMILP with binary variables (restricted-domain variant).
pub fn solve_mpmilp_bb(program: &Arc<MpmiProgram>, alg: MpqpAlgorithm, opts: &SolveOptions) -> Result<(Solution, BbStats), String> {
    solve_mpmilp_bb_with(program, alg, opts, BbOptions { restrict_domain: true, reduce_final: true })
}

pub fn solve_mpmilp_bb_with(program: &Arc<MpmiProgram>, alg: MpqpAlgorithm, opts: &SolveOptions, bbo: BbOptions) -> Result<(Solution, BbStats), String> {
    check_affine(program)?;
    let t0 = Instant::now();
    let p = program.as_ref();
    let nb = p.binary_indices.len();
    let nc = p.cont_indices.len();
    let mut stats = BbStats::default();
    let mut templates: Vec<CriticalRegion> = vec![];
    let mut inc: Vec<Piece> = vec![];
    let mut stack: Vec<(Vec<Option<f64>>, Mat, Vec<f64>)> = vec![(vec![None; nb], p.base.a_t.clone(), p.base.b_t.clone())];
    let mut seen: std::collections::HashSet<Vec<i8>> = Default::default();
    let key = |f: &[Option<f64>]| f.iter().map(|v| v.map(|x| x as i8).unwrap_or(-1)).collect::<Vec<i8>>();
    seen.insert(key(&stack[0].0));
    let mut distinct = std::collections::HashSet::new();
    while let Some((fixed, te, tf)) = stack.pop() {
        stats.nodes += 1;
        distinct.insert(key(&fixed));
        let regions = solve_node(p, &fixed, &te, &tf, alg, opts);
        stats.relaxed_regions += regions.len();
        // integral regions first: they tighten the upper bound before pruning
        for r in regions.iter().filter(|r| r.branch.is_none()) {
            let mut cr = r.cr.clone();
            cr.a = cr.a.select_rows(&(0..nc).collect::<Vec<_>>());
            cr.b.truncate(nc);
            cr.y_fixation = Some(r.y.clone());
            cr.y_indices = Some(p.binary_indices.clone());
            cr.x_indices = Some(p.cont_indices.clone());
            templates.push(cr);
            stats.incumbent_updates += 1;
            insert_piece(&mut inc, Piece { e: r.cr.e.clone(), f: r.cr.f.clone(), g: r.g.clone(), k: r.k, src: templates.len() - 1, tried: vec![], inf: false });
        }
        let mut full_branch: Option<usize> = None;
        for r in regions.iter().filter(|r| r.branch.is_some()) {
            let surv = prune_against(&r.cr.e, &r.cr.f, &r.g, r.k, &inc);
            if surv.is_empty() {
                stats.pruned_regions += 1;
                continue;
            }
            let j = r.branch.unwrap();
            if !bbo.restrict_domain {
                full_branch = Some(full_branch.map_or(j, |b| b.min(j)));
                continue;
            }
            // a single surviving fragment is a tighter domain; several are replaced by the whole region
            let (ce, cf) = if surv.len() == 1 { surv.into_iter().next().unwrap() } else { (r.cr.e.clone(), r.cr.f.clone()) };
            for v in [1.0, 0.0] {
                let mut fx = fixed.clone();
                fx[j] = Some(v);
                stack.push((fx, ce.clone(), cf.clone()));
            }
        }
        if let Some(j) = full_branch {
            for v in [1.0, 0.0] {
                let mut fx = fixed.clone();
                fx[j] = Some(v);
                if seen.insert(key(&fx)) {
                    stack.push((fx, p.base.a_t.clone(), p.base.b_t.clone()));
                }
            }
        }
    }
    stats.distinct_fixations = distinct.len();
    let sol = pieces_to_solution(program, &templates, inc, bbo.reduce_final);
    stats.regions = sol.critical_regions.len();
    stats.seconds = t0.elapsed().as_secs_f64();
    Ok((sol, stats))
}

// ------------------------------------------------------------------------------------ decomposition

#[derive(Clone, Debug, Default)]
pub struct DecompStats {
    /// mpLPs solved (one per new integer solution on a region)
    pub mplps: usize,
    /// MILP master problems solved
    pub milps: usize,
    pub regions: usize,
    pub seconds: f64,
}

/// MILP master problem (4) of Dua and Pistikopoulos on one piece: theta is a variable restricted to
/// the piece, the parametric cut `c'x + d'y <= zhat(theta) - eps` removes solutions that are not
/// better than the current upper bound (absent while the bound is infinite), and one integer cut
/// per binary vector already analysed on the piece removes repeats.
fn master_milp(p: &MpmiProgram, piece: &Piece) -> Option<Vec<f64>> {
    let b = &p.base;
    let (nx, nt) = (b.num_x(), b.num_t());
    let (pa, pb) = b.xtheta_polytope();
    let mut a = pa;
    let mut rhs = pb;
    for i in 0..piece.e.rows {
        let mut row = vec![0.0; nx + nt];
        row[nx..].copy_from_slice(piece.e.row(i));
        a.push_row(&row);
        rhs.push(piece.f[i]);
    }
    if !piece.inf {
        let mut row = vec![0.0; nx + nt];
        row[..nx].copy_from_slice(&b.c);
        for j in 0..nt {
            row[nx + j] = b.c_t[j] - piece.g[j];
        }
        a.push_row(&row);
        rhs.push(piece.k - b.c_c - 1e-7 * piece.k.abs().max(1.0));
    }
    for t in &piece.tried {
        let mut row = vec![0.0; nx + nt];
        let mut ones = 0.0;
        for (j, &yi) in p.binary_indices.iter().enumerate() {
            if t[j] == 1 {
                row[yi] = 1.0;
                ones += 1.0;
            } else {
                row[yi] = -1.0;
            }
        }
        a.push_row(&row);
        rhs.push(ones - 1.0);
    }
    let mut c = b.c.clone();
    c.extend_from_slice(&b.c_t);
    let s = b.solver.solve_milp(Some(&c), &a, &rhs, &b.equality_indices, &p.binary_indices)?;
    Some(p.binary_indices.iter().map(|&i| s.sol[i].round()).collect())
}

/// The decomposition algorithm of Dua and Pistikopoulos (2000, Sec. 2): an MILP with theta free
/// gives a first integer vector; the mpLP for that vector gives a parametric upper bound on each
/// of its critical regions; on every region an MILP (integer cuts plus the parametric cut) looks
/// for a better vector, whose mpLP is compared with the current bound (Appendix A); regions where
/// the MILP is infeasible are final. Regions where no vector is feasible are left uncovered.
pub fn solve_mpmilp_decomposition(program: &Arc<MpmiProgram>, alg: MpqpAlgorithm, opts: &SolveOptions) -> Result<(Solution, DecompStats), String> {
    check_affine(program)?;
    let t0 = Instant::now();
    let p = program.as_ref();
    let nt = p.base.num_t();
    let mut st = DecompStats::default();
    let mut templates: Vec<CriticalRegion> = vec![];
    let mut done: Vec<Piece> = vec![];
    let mut work = vec![Piece { e: p.base.a_t.clone(), f: p.base.b_t.clone(), g: vec![0.0; nt], k: 0.0, src: usize::MAX, tried: vec![], inf: true }];
    let nc = p.cont_indices.len();
    while let Some(s) = work.pop() {
        st.milps += 1;
        let Some(y) = master_milp(p, &s) else {
            if !s.inf {
                done.push(s);
            }
            continue;
        };
        let mut tried = s.tried.clone();
        tried.push(y.iter().map(|&v| v as i8).collect());
        st.mplps += 1;
        let fixed: Vec<Option<f64>> = y.iter().map(|&v| Some(v)).collect();
        let mut news = vec![];
        for r in solve_node(p, &fixed, &s.e, &s.f, alg, opts) {
            let mut cr = r.cr.clone();
            cr.a = cr.a.select_rows(&(0..nc).collect::<Vec<_>>());
            cr.b.truncate(nc);
            cr.y_fixation = Some(r.y.clone());
            cr.y_indices = Some(p.binary_indices.clone());
            cr.x_indices = Some(p.cont_indices.clone());
            templates.push(cr);
            news.push(Piece { e: r.cr.e.clone(), f: r.cr.f.clone(), g: r.g, k: r.k, src: templates.len() - 1, tried: tried.clone(), inf: false });
        }
        let mut local: Vec<Piece>;
        if s.inf {
            // parts of the region where this vector is infeasible stay at infinity (Appendix B)
            let mut rem = vec![(s.e.clone(), s.f.clone())];
            for n in &news {
                rem = rem.into_iter().flat_map(|(e, f)| diff(&e, &f, &n.e, &n.f)).collect();
            }
            local = news;
            for (e, f) in rem {
                local.push(Piece { e, f, g: vec![0.0; nt], k: 0.0, src: usize::MAX, tried: tried.clone(), inf: true });
            }
        } else {
            let mut base = s.clone();
            base.tried = tried;
            local = vec![base];
            for n in news {
                insert_piece(&mut local, n);
            }
        }
        work.extend(local);
    }
    let sol = pieces_to_solution(program, &templates, done, true);
    st.regions = sol.critical_regions.len();
    st.seconds = t0.elapsed().as_secs_f64();
    Ok((sol, st))
}

// ------------------------------------------------------------------------------------ general integers

/// Binary expansion baseline: every integer `0 <= y <= ub` becomes `sum_b 2^b z_b` with binary
/// `z_b`, plus a row `sum_b 2^b z_b <= ub`. Returns the all-binary program (continuous variables
/// first, then the bits) and, per integer variable, the positions of its bits among the new
/// variables.
#[allow(clippy::too_many_arguments)]
pub fn binary_expansion(
    a: &Mat,
    b: &[f64],
    c: &[f64],
    f: &Mat,
    a_t: &Mat,
    b_t: &[f64],
    int_indices: &[usize],
    upper: &[u32],
    equality_indices: &[usize],
) -> (MpmiProgram, Vec<Vec<usize>>) {
    let nx = a.cols;
    let keep: Vec<usize> = (0..nx).filter(|j| !int_indices.contains(j)).collect();
    let mut new_cols: Vec<Vec<f64>> = keep.iter().map(|&j| a.col(j)).collect();
    let mut new_c: Vec<f64> = keep.iter().map(|&j| c[j]).collect();
    let mut bound_rows: Vec<(Vec<(usize, f64)>, f64)> = vec![];
    let mut groups = vec![];
    for (v, &j) in int_indices.iter().enumerate() {
        let nbits = (32 - upper[v].leading_zeros()) as usize;
        let mut g = vec![];
        let mut terms = vec![];
        for bit in 0..nbits {
            let w = (1u64 << bit) as f64;
            new_cols.push(a.col(j).iter().map(|x| x * w).collect());
            new_c.push(c[j] * w);
            g.push(new_cols.len() - 1);
            terms.push((new_cols.len() - 1, w));
        }
        bound_rows.push((terms, upper[v] as f64));
        groups.push(g);
    }
    let m = a.rows;
    let nn = new_cols.len();
    let mut an = Mat::zeros(m + bound_rows.len(), nn);
    for (jn, col) in new_cols.iter().enumerate() {
        for i in 0..m {
            an[(i, jn)] = col[i];
        }
    }
    let mut bn = b.to_vec();
    let mut fx = Mat::zeros(m + bound_rows.len(), f.cols);
    for i in 0..m {
        fx.row_mut(i).copy_from_slice(f.row(i));
    }
    for (r, (terms, ub)) in bound_rows.iter().enumerate() {
        for &(jn, w) in terms {
            an[(m + r, jn)] = w;
        }
        bn.push(*ub);
    }
    let bin_idx: Vec<usize> = groups.iter().flatten().cloned().collect();
    let prog = MpmiProgram::new(an, bn, new_c, Mat::zeros(nn, f.cols), None, a_t.clone(), b_t.to_vec(), fx, bin_idx, equality_indices.to_vec());
    (prog, groups)
}

/// Native general-integer branch and bound in the parameter space: branching `y <= floor(v)` and
/// `y >= floor(v) + 1` on a fractional integer variable, with the same pruning and comparison
/// step as [`solve_mpmilp_bb`]. The program's `binary_indices` are read as general integer
/// variables with bounds `0 <= y <= upper`.
pub fn solve_mpmilp_bb_general(program: &Arc<MpmiProgram>, upper: &[u32], alg: MpqpAlgorithm, opts: &SolveOptions) -> Result<(Solution, BbStats), String> {
    check_affine(program)?;
    let t0 = Instant::now();
    let p = program.as_ref();
    let ni = p.binary_indices.len();
    assert_eq!(ni, upper.len());
    let mut stats = BbStats::default();
    let mut templates: Vec<CriticalRegion> = vec![];
    let mut inc: Vec<Piece> = vec![];
    type Node = (Vec<(f64, f64)>, Mat, Vec<f64>);
    let mut stack: Vec<Node> = vec![(upper.iter().map(|&u| (0.0, u as f64)).collect(), p.base.a_t.clone(), p.base.b_t.clone())];
    let b = &p.base;
    let nt = b.num_t();
    let nc = p.cont_indices.len();
    let mut x_idx: Vec<usize> = p.cont_indices.clone();
    x_idx.extend(&p.binary_indices);
    let c_x: Vec<f64> = x_idx.iter().map(|&i| b.c[i]).collect();
    let a_all = b.a.select_cols(&x_idx);
    let eqs = b.equality_indices.clone();
    while let Some((bounds, te, tf)) = stack.pop() {
        stats.nodes += 1;
        let mut a = a_all.clone();
        let mut rhs = b.b.clone();
        for (pos, &(lo, hi)) in bounds.iter().enumerate() {
            let mut up = vec![0.0; x_idx.len()];
            up[nc + pos] = 1.0;
            a.push_row(&up);
            rhs.push(hi);
            let mut dn = vec![0.0; x_idx.len()];
            dn[nc + pos] = -1.0;
            a.push_row(&dn);
            rhs.push(-lo);
        }
        let f = Mat::vstack(&[&b.f, &Mat::zeros(2 * ni, nt)]);
        let mut sub = MpProgram::new(a, rhs, c_x.clone(), Mat::zeros(x_idx.len(), nt), None, te.clone(), tf.clone(), f, eqs.clone(), true);
        sub.c_c = b.c_c;
        sub.c_t = b.c_t.clone();
        let sol = solve_mpqp_with(&Arc::new(sub), alg, opts);
        stats.relaxed_regions += sol.critical_regions.len();
        let mut frac = vec![];
        for cr in sol.critical_regions {
            let mut g = b.c_t.clone();
            let mut k = b.c_c;
            for (r, c) in c_x.iter().enumerate() {
                for j in 0..nt {
                    g[j] += c * cr.a[(r, j)];
                }
                k += c * cr.b[r];
            }
            let mut branch = None;
            let mut y = vec![0.0; ni];
            for pos in 0..ni {
                let row = cr.a.row(nc + pos);
                let bv = cr.b[nc + pos];
                if row.iter().all(|v| v.abs() <= 1e-7) && (bv - bv.round()).abs() <= 1e-7 {
                    y[pos] = bv.round();
                } else if branch.is_none() {
                    let (th, _) = cr.chebyshev().unwrap_or((vec![0.0; nt], 0.0));
                    branch = Some((pos, (dot(row, &th) + bv).floor()));
                }
            }
            match branch {
                None => {
                    let mut t = cr.clone();
                    t.a = t.a.select_rows(&(0..nc).collect::<Vec<_>>());
                    t.b.truncate(nc);
                    t.y_fixation = Some(y);
                    t.y_indices = Some(p.binary_indices.clone());
                    t.x_indices = Some(p.cont_indices.clone());
                    templates.push(t);
                    stats.incumbent_updates += 1;
                    insert_piece(&mut inc, Piece { e: cr.e.clone(), f: cr.f.clone(), g, k, src: templates.len() - 1, tried: vec![], inf: false });
                }
                Some(br) => frac.push((cr, g, k, br)),
            }
        }
        for (cr, g, k, (pos, fl)) in frac {
            let surv = prune_against(&cr.e, &cr.f, &g, k, &inc);
            if surv.is_empty() {
                stats.pruned_regions += 1;
                continue;
            }
            let (ce, cf) = if surv.len() == 1 { surv.into_iter().next().unwrap() } else { (cr.e.clone(), cr.f.clone()) };
            let (lo, hi) = bounds[pos];
            if fl + 1.0 <= hi {
                let mut nb2 = bounds.clone();
                nb2[pos] = (fl + 1.0, hi);
                stack.push((nb2, ce.clone(), cf.clone()));
            }
            if fl >= lo {
                let mut nb2 = bounds.clone();
                nb2[pos] = (lo, fl);
                stack.push((nb2, ce, cf));
            }
        }
    }
    let sol = pieces_to_solution(program, &templates, inc, true);
    stats.regions = sol.critical_regions.len();
    stats.seconds = t0.elapsed().as_secs_f64();
    Ok((sol, stats))
}

// ------------------------------------------------------------------------------------ generator

/// Random fixed-charge mpMILP. `nb` facilities (binary `y_k`, flow `x_k <= cap_k y_k`), `nd` demand
/// rows `sum_k w_ik x_k + r_i >= D_i + G_i' theta` with an expensive recourse `r_i`, and `t`
/// parameters in [0, 1]. Variables are ordered `[x (nb), r (nd), y (nb)]`; the objective is
/// `sum c_k x_k + 12 sum r_i + sum f_k y_k`.
pub fn generate_mpmilp(nb: usize, nd: usize, t: usize, seed: u64) -> MpmiProgram {
    let mut rng = StdRng::seed_from_u64(seed);
    let nc = nb + nd;
    let nx = nc + nb;
    let rows = nb + nd + 2 * nx;
    let mut a = Mat::zeros(rows, nx);
    let mut f = Mat::zeros(rows, t);
    let mut b = vec![0.0; rows];
    let mut r = 0;
    for k in 0..nb {
        let cap = (rng.r#gen::<f64>() * 6.0).round() + 3.0;
        a[(r, k)] = 1.0;
        a[(r, nc + k)] = -cap;
        r += 1;
    }
    for i in 0..nd {
        for k in 0..nb {
            if rng.r#gen::<f64>() < 0.6 || k == i % nb {
                a[(r, k)] = -1.0;
            }
        }
        a[(r, nb + i)] = -1.0;
        b[r] = -(rng.r#gen::<f64>() * 3.0).round();
        for j in 0..t {
            f[(r, j)] = -((rng.r#gen::<f64>() * 7.0).round() + 1.0);
        }
        r += 1;
    }
    for j in 0..nx {
        a[(r, j)] = 1.0;
        b[r] = if j < nc { 20.0 } else { 1.0 };
        r += 1;
        a[(r, j)] = -1.0;
        r += 1;
    }
    let mut c = vec![0.0; nx];
    for k in 0..nb {
        c[k] = (rng.r#gen::<f64>() * 4.0).round() + 1.0;
        c[nc + k] = (rng.r#gen::<f64>() * 12.0).round() + 2.0;
    }
    for i in 0..nd {
        c[nb + i] = 12.0;
    }
    let mut a_t = Mat::zeros(2 * t, t);
    for j in 0..t {
        a_t[(j, j)] = 1.0;
        a_t[(t + j, j)] = -1.0;
    }
    let mut b_t = vec![1.0; t];
    b_t.extend(vec![0.0; t]);
    let binary: Vec<usize> = (nc..nx).collect();
    MpmiProgram::new(a, b, c, Mat::zeros(nx, t), None, a_t, b_t, f, binary, vec![])
}
