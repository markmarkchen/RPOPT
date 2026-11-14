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

// ------------------------------------------------------------------------------------------
// Faster variant used by the mpLP graph repair pass (`lp_repair`): the same facet tasks, with the
// coplanar-neighbour lookup answered from a hash index and facets that a neighbour region
// provably covers skipped without any LP.

/// Hash index of all region rows by their normalized hyperplane (unit normal, offset), so that
/// the regions with a row in the plane opposite to a given facet are found without a scan.
struct PlaneIndex {
    cells: crate::fxhash::FxHashMap<Vec<i64>, Vec<(u32, u32)>>,
}

const PLANE_CELL: f64 = 1e-3;
const PLANE_TOL: f64 = 1e-6;

impl PlaneIndex {
    fn new(regions: &[CriticalRegion]) -> Self {
        let mut cells: crate::fxhash::FxHashMap<Vec<i64>, Vec<(u32, u32)>> = Default::default();
        for (ri, r) in regions.iter().enumerate() {
            for i in 0..r.e.rows {
                let ni = norm(r.e.row(i));
                let key: Vec<i64> = (0..r.e.cols).map(|d| r.e[(i, d)] / ni).chain(std::iter::once(r.f[i] / ni)).map(|v| (v / PLANE_CELL).floor() as i64).collect();
                cells.entry(key).or_default().push((ri as u32, i as u32));
            }
        }
        PlaneIndex { cells }
    }

    /// Regions (ascending, without `skip`) having a row with unit normal and offset within
    /// `PLANE_TOL` of the negated unit row `(a, b)`, together with the matching row indices.
    fn opposite(&self, regions: &[CriticalRegion], a: &[f64], b: f64, skip: usize) -> Vec<(usize, Vec<usize>)> {
        let target: Vec<f64> = a.iter().chain(std::iter::once(&b)).map(|v| -v).collect();
        // candidate cells: every coordinate may sit on either side of a cell border
        let mut keys: Vec<Vec<i64>> = vec![vec![]];
        for &t in &target {
            let lo = ((t - PLANE_TOL) / PLANE_CELL).floor() as i64;
            let hi = ((t + PLANE_TOL) / PLANE_CELL).floor() as i64;
            let mut next = vec![];
            for k in &keys {
                for c in lo..=hi {
                    let mut kk = k.clone();
                    kk.push(c);
                    next.push(kk);
                }
            }
            keys = next;
        }
        let mut hits: Vec<(usize, Vec<usize>)> = vec![];
        for key in keys {
            let Some(list) = self.cells.get(&key) else { continue };
            for &(ri, i) in list {
                let (ri, i) = (ri as usize, i as usize);
                if ri == skip {
                    continue;
                }
                let o = &regions[ri];
                let ni = norm(o.e.row(i));
                let ok = (0..a.len()).all(|d| (target[d] - o.e[(i, d)] / ni).abs() < PLANE_TOL) && (target[a.len()] - o.f[i] / ni).abs() < PLANE_TOL;
                if ok {
                    match hits.iter_mut().find(|h| h.0 == ri) {
                        Some(h) => h.1.push(i),
                        None => hits.push((ri, vec![i])),
                    }
                }
            }
        }
        hits.sort_by_key(|h| h.0);
        hits
    }
}

/// The constraints of region `r` other than row `skip` restricted to the hyperplane of row
/// `skip` of `r` (unit normal `nhat`, offset `h`), as unit in-plane rows (a_proj / |a_proj|, rhs).
/// `None` if some row is parallel to the normal (a constant on the plane).
fn in_plane_rows(r: &CriticalRegion, skip: usize, nhat: &[f64], h: f64) -> Option<Vec<(Vec<f64>, f64)>> {
    let mut out = vec![];
    for i in 0..r.e.rows {
        if i == skip {
            continue;
        }
        let a = r.e.row(i);
        let d = dot(a, nhat);
        let ap: Vec<f64> = a.iter().zip(nhat).map(|(v, n)| v - d * n).collect();
        let np = norm(&ap);
        if np < 1e-8 * norm(a).max(1e-300) {
            return None;
        }
        out.push((ap.iter().map(|v| v / np).collect(), (r.f[i] - d * h) / np));
    }
    Some(out)
}

/// Is every in-plane row of `theirs` also a row of `mine`? Then the polytope of `mine` lies inside
/// the polytope of `theirs` (it has all of their constraints and possibly more).
fn rows_inside(mine: &[(Vec<f64>, f64)], theirs: &[(Vec<f64>, f64)]) -> bool {
    let close = |p: &(Vec<f64>, f64), q: &(Vec<f64>, f64)| (p.1 - q.1).abs() < 1e-8 * (1.0 + p.1.abs()) && p.0.iter().zip(&q.0).all(|(u, v)| (u - v).abs() < 1e-8);
    theirs.iter().all(|q| mine.iter().any(|p| close(p, q)))
}

/// `complete_facets` with the same output, faster on facet-to-facet partitions: the neighbours of
/// a facet come from a plane index, and a facet whose polytope lies inside the facet of a neighbour
/// region (its in-plane rows are among ours) is covered by it and needs no LP.
pub fn complete_facets_indexed(
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
    let index = PlaneIndex::new(regions);
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
            let nk = norm(r.e.row(k));
            let nhat: Vec<f64> = r.e.row(k).iter().map(|v| v / nk).collect();
            let h = r.f[k] / nk;
            let hits = index.opposite(regions, &nhat, h, wi);
            // covered by a neighbour whose facet polytope contains this one
            if !hits.is_empty() {
                if let Some(mine) = in_plane_rows(r, k, &nhat, h) {
                    let covered = hits.iter().any(|(oi, rows)| {
                        let o = &regions[*oi];
                        rows.iter().any(|&i| {
                            let no = norm(o.e.row(i));
                            let onhat: Vec<f64> = o.e.row(i).iter().map(|v| v / no).collect();
                            in_plane_rows(o, i, &onhat, o.f[i] / no).is_some_and(|theirs| rows_inside(&mine, &theirs))
                        })
                    });
                    if covered {
                        continue;
                    }
                }
            }
            let others: Vec<&CriticalRegion> = hits.iter().map(|(oi, _)| &regions[*oi]).collect();
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
