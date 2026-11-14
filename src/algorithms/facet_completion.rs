//! Facet completion for degenerate multiparametric programs (used by `geometric`).
//!
//! When the active set at a point has dependent constraints (LICQ violation) or more than n_x
//! constraints, the critical-region partition is not facet-to-facet (Spjotvold, Tondel, Johansen
//! 2006): a facet of a region can be shared by several neighbours, so stepping out of the facet
//! centre finds only one of them. For every facet this finds the parts that no other region
//! touches and steps out of each of them (the disjunctive subtraction of the neighbours' closures
//! follows the polytope-difference construction of Bemporad 2015). Together with the canonical
//! basis choice in `common::canonical_basis` this makes the exploration complete and the regions
//! disjoint on degenerate problems.

use super::common::fathem_facet_degen;
use crate::critical_region::CriticalRegion;
use crate::fxhash::FxHashSet as HashSet;
use crate::linalg::{dot, norm, Mat};
use crate::program::MpProgram;
use crate::solution::Solution;
use rayon::prelude::*;
use std::sync::atomic::AtomicBool;

type Rows = Vec<(Vec<f64>, f64)>;

/// Chebyshev ball inside the hyperplane `plane.0 . t = plane.1` of {t : a t <= b for (a, b) in
/// rows}, with the radius measured in the hyperplane. Rows parallel to the normal are constants
/// there: they make the set empty (violated at `c0`) or are dropped.
fn cheb_in_plane(rows: &Rows, plane: &(Vec<f64>, f64), nhat: &[f64], c0: &[f64], min_radius: f64, strict_last: bool) -> Option<(Vec<f64>, f64)> {
    let n = plane.0.len();
    let mut ab = Mat::zeros(rows.len() + 2, n + 1);
    let mut bb = Vec::with_capacity(rows.len() + 2);
    let mut m = 0;
    for (ri, (a, b)) in rows.iter().enumerate() {
        let d = dot(a, nhat);
        let mut ap = a.clone();
        for (v, h) in ap.iter_mut().zip(nhat) {
            *v -= d * h;
        }
        let npl = norm(&ap);
        if npl < 1e-10 * norm(a).max(1e-300) {
            // a cut row (last) is a strict violation of a closed neighbour row: a constant that
            // is tight on the plane cuts nothing
            let v = dot(a, c0) - b;
            if v > 1e-9 || (strict_last && ri + 1 == rows.len() && v > -1e-9) {
                return None;
            }
            continue;
        }
        ab.row_mut(m)[..n].copy_from_slice(a);
        ab[(m, n)] = npl;
        bb.push(*b);
        m += 1;
    }
    ab.row_mut(m)[..n].copy_from_slice(&plane.0);
    bb.push(plane.1);
    let eq_row = m;
    m += 1;
    ab[(m, n)] = -1.0;
    bb.push(0.0);
    m += 1;
    ab.rows = m;
    ab.data.truncate(m * (n + 1));
    let mut c = vec![0.0; n + 1];
    c[n] = -1.0;
    let x = crate::solver::lp::lp_point(Some(&c), &ab, &bb, &[eq_row])?;
    if x[n] > min_radius {
        Some((x[..n].to_vec(), x[n]))
    } else {
        None
    }
}

fn region_rows(r: &CriticalRegion) -> Rows {
    (0..r.e.rows).map(|i| (r.e.row(i).to_vec(), r.f[i])).collect()
}

/// Parts of facet `k` of region `r` that no region in `others` touches (convex pieces, each as an
/// in-facet Chebyshev center and radius). Subtracting a neighbour R_j from a piece gives the union
/// over the rows i of R_j of piece ∩ {row i violated} ∩ {rows before i satisfied}.
fn uncovered_pieces(r: &CriticalRegion, k: usize, others: &[&CriticalRegion]) -> Vec<(Vec<f64>, f64)> {
    const MIN_R: f64 = 1e-7;
    let ek = r.e.row(k);
    let nk = norm(ek);
    if nk < 1e-14 {
        return vec![];
    }
    let nhat: Vec<f64> = ek.iter().map(|v| v / nk).collect();
    let plane = (ek.to_vec(), r.f[k]);
    let mut base: Rows = region_rows(r);
    base.remove(k);
    let Some((c0, _)) = cheb_in_plane(&base, &plane, &nhat, &ek.iter().map(|v| v * r.f[k] / (nk * nk)).collect::<Vec<f64>>(), 1e-12, false) else { return vec![] };
    let mut pieces: Vec<Rows> = vec![base];
    for rj in others {
        let rj_rows = region_rows(rj);
        let mut next: Vec<Rows> = vec![];
        for p in pieces {
            let mut both = p.clone();
            both.extend(rj_rows.iter().cloned());
            if cheb_in_plane(&both, &plane, &nhat, &c0, MIN_R, false).is_none() {
                next.push(p);
                continue;
            }
            let mut prefix: Rows = vec![];
            for (a, b) in &rj_rows {
                let mut cand = p.clone();
                cand.extend(prefix.iter().cloned());
                cand.push((a.iter().map(|v| -v).collect(), -b));
                if cheb_in_plane(&cand, &plane, &nhat, &c0, MIN_R, true).is_some() {
                    next.push(cand);
                }
                prefix.push((a.clone(), *b));
            }
        }
        pieces = next;
        if pieces.is_empty() {
            break;
        }
    }
    pieces.iter().filter_map(|p| cheb_in_plane(p, &plane, &nhat, &c0, MIN_R, false)).collect()
}

fn is_theta_boundary(program: &MpProgram, e: &[f64], f: f64) -> bool {
    let ne = norm(e);
    (0..program.a_t.rows).any(|i| {
        let at = program.a_t.row(i);
        let na = norm(at);
        (0..e.len()).all(|j| (e[j] / ne - at[j] / na).abs() < 1e-9) && (f / ne - program.b_t[i] / na).abs() < 1e-9
    })
}

/// For the facets of regions `wave` of `solution`, finds the parts touched by no other region and
/// steps out of each; returns the new regions (one per active set).
pub fn complete_facets(
    program: &MpProgram,
    solution: &Solution,
    suspects: &[(usize, Option<Vec<f64>>)],
    indexed: &HashSet<Vec<usize>>,
    degen: &AtomicBool,
    parallel: bool,
) -> Vec<CriticalRegion> {
    if program.num_t() < 2 {
        return vec![];
    }
    let regions = &solution.critical_regions;
    let facet_tasks = |(wi, only): &(usize, Option<Vec<f64>>)| -> Vec<(usize, Vec<f64>, Vec<f64>, f64)> {
        let wi = *wi;
        let r = &regions[wi];
        let mut out = vec![];
        for k in 0..r.e.rows {
            if is_theta_boundary(program, r.e.row(k), r.f[k]) {
                continue;
            }
            if let Some(nrm) = only {
                let nk = norm(r.e.row(k));
                if !(0..nrm.len()).all(|d| (r.e[(k, d)] / nk - nrm[d] / norm(nrm)).abs() < 1e-9) {
                    continue;
                }
            }
            // a region that touches the facet from the other side has a facet in the same plane
            let nk = norm(r.e.row(k));
            let coplanar = |o: &CriticalRegion| -> bool {
                (0..o.e.rows).any(|i| {
                    let ni = norm(o.e.row(i));
                    (0..r.e.cols).all(|d| (r.e[(k, d)] / nk + o.e[(i, d)] / ni).abs() < 1e-6) && (r.f[k] / nk + o.f[i] / ni).abs() < 1e-6
                })
            };
            let others: Vec<&CriticalRegion> = (0..regions.len()).filter(|&i| i != wi && coplanar(&regions[i])).map(|i| &regions[i]).collect();
            for (c, rad) in uncovered_pieces(r, k, &others) {
                out.push((wi, c, r.e.row(k).to_vec(), rad));
            }
        }
        out
    };
    let tasks: Vec<_> = if parallel { suspects.par_iter().flat_map_iter(facet_tasks).collect() } else { suspects.iter().flat_map(facet_tasks).collect() };
    let claimed = std::sync::Mutex::new(HashSet::default());
    let step = |(wi, c, n, rad): &(usize, Vec<f64>, Vec<f64>, f64)| {
        let nn = norm(n);
        let nrm: Vec<f64> = n.iter().map(|v| v / nn).collect();
        fathem_facet_degen(c, &nrm, rad.max(0.05), program, indexed, Some(&claimed), &regions[*wi].active_set, Some(solution), Some(degen))
    };
    let found: Vec<Option<CriticalRegion>> = if parallel { tasks.par_iter().map(step).collect() } else { tasks.iter().map(step).collect() };
    found.into_iter().flatten().collect()
}
