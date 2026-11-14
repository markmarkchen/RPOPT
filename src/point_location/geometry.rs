//! Offline geometry of a polyhedral partition, shared by the point location methods:
//! per-region rows, Chebyshev centers, vertices and bounding boxes, the set of distinct facet
//! hyperplanes, and verified facet adjacency between regions.

use crate::linalg::{dot, Lu, Mat};
use crate::solution::Solution;
use crate::utils::chebyshev_ball;
use rayon::prelude::*;

/// One facet row of a region and what lies across it.
#[derive(Clone, Debug)]
pub struct Facet {
    /// row index inside the region's E, f
    pub row: usize,
    /// distinct hyperplane id and orientation (+1: row = hyperplane, -1: row = -hyperplane)
    pub plane: usize,
    pub orient: f64,
    /// regions sharing an (n-1)-dimensional piece of this facet
    pub neighbors: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct PartitionGeometry {
    pub dim: usize,
    pub num_regions: usize,
    /// Point location tolerance of the solution this geometry was built from. Every locator built
    /// from the geometry uses it for its membership tests (and widens its build-time
    /// classification so that no region within it is dropped). Distances are measured on the
    /// normalized rows of this geometry; `Solution` measures the same quantity on its own rows
    /// (identical when those rows have unit norm, which the generators of this crate produce).
    pub tol: f64,
    /// region rows (normalized), region i occupies rows offsets[i]..offsets[i+1]
    pub e: Vec<f64>,
    pub f: Vec<f64>,
    pub offsets: Vec<usize>,
    pub centers: Vec<Vec<f64>>,
    pub radii: Vec<f64>,
    pub vertices: Vec<Vec<Vec<f64>>>,
    pub bbox_lo: Vec<Vec<f64>>,
    pub bbox_hi: Vec<Vec<f64>>,
    /// distinct hyperplanes a'θ <= b (normalized, canonical orientation), (dim + 1) floats each
    pub planes: Vec<Vec<f64>>,
    pub facets: Vec<Vec<Facet>>,
    /// rows on the boundary of the union (no neighbour), deduplicated
    pub domain_planes: Vec<Vec<f64>>,
    pub global_lo: Vec<f64>,
    pub global_hi: Vec<f64>,
    /// Regions whose vertex list cannot be trusted to be complete: a basic solution missed the
    /// acceptance tolerance narrowly (`enumerate_vertices_checked`), inradius below
    /// `THIN_REL_RADIUS` times the extent of the partition, or no vertices. The bases of such a
    /// region are ill-conditioned, so `enumerate_vertices` drops true vertices (and relaxing its
    /// tolerance instead adds many spurious ones). Locators must not classify a thin region as
    /// lying on one side of a plane or outside a cell from its vertices; `bbox_lo` / `bbox_hi` of
    /// a thin region come from LPs and are conservative.
    pub thin: Vec<bool>,
}

const PLANE_TOL: f64 = 1e-7;
/// A region is thin when its Chebyshev radius is below this fraction of the partition extent.
pub const THIN_REL_RADIUS: f64 = 1e-6;

impl PartitionGeometry {
    /// The same geometry with another point location tolerance for the locators built from it.
    pub fn with_tol(mut self, tol: f64) -> Self {
        self.tol = tol;
        self
    }

    pub fn new(sol: &Solution) -> Self {
        let regions = &sol.critical_regions;
        let n = sol.theta_dim();
        let r = regions.len();
        let mut e = vec![];
        let mut f = vec![];
        let mut offsets = vec![0];
        for reg in regions {
            for i in 0..reg.e.rows {
                let row = reg.e.row(i);
                let nrm = crate::linalg::norm(row);
                e.extend(row.iter().map(|v| v / nrm));
                f.push(reg.f[i] / nrm);
            }
            offsets.push(f.len());
        }
        let region_mat = |i: usize| -> (Mat, Vec<f64>) {
            let (s, t) = (offsets[i], offsets[i + 1]);
            (Mat::from_vec(t - s, n, e[s * n..t * n].to_vec()), f[s..t].to_vec())
        };

        // centers, vertices, bounding boxes
        let per_region: Vec<(Vec<f64>, f64, Vec<Vec<f64>>, bool)> = (0..r)
            .into_par_iter()
            .map(|i| {
                let (a, b) = region_mat(i);
                let (c, rad) = match chebyshev_ball(&a, &b, &[]) {
                    Some(s) => (s.x[..n].to_vec(), s.x[n]),
                    None => (vec![0.0; n], 0.0),
                };
                let (v, sus) = enumerate_vertices_checked(&a, &b);
                (c, rad, v, sus)
            })
            .collect();
        let mut centers = vec![];
        let mut radii = vec![];
        let mut vertices = vec![];
        let mut suspicious = vec![];
        let mut bbox_lo = vec![];
        let mut bbox_hi = vec![];
        let mut global_lo = vec![f64::INFINITY; n];
        let mut global_hi = vec![f64::NEG_INFINITY; n];
        for (c, rad, v, sus) in per_region {
            suspicious.push(sus);
            let mut lo = vec![f64::INFINITY; n];
            let mut hi = vec![f64::NEG_INFINITY; n];
            for p in &v {
                for k in 0..n {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
            for k in 0..n {
                global_lo[k] = global_lo[k].min(lo[k]);
                global_hi[k] = global_hi[k].max(hi[k]);
            }
            centers.push(c);
            radii.push(rad);
            vertices.push(v);
            bbox_lo.push(lo);
            bbox_hi.push(hi);
        }

        // thin regions: untrusted vertex lists. Their bounding boxes come from coordinate LPs
        // (merged with the vertex box), a safe enclosure for the hash table cells.
        let extent = (0..n).map(|k| global_hi[k] - global_lo[k]).fold(0.0f64, f64::max);
        let thin: Vec<bool> = (0..r).map(|i| suspicious[i] || vertices[i].is_empty() || !(radii[i] >= THIN_REL_RADIUS * extent)).collect();
        let boxes: Vec<Option<(Vec<f64>, Vec<f64>)>> = (0..r)
            .into_par_iter()
            .map(|i| {
                if !thin[i] {
                    return None;
                }
                let (a, b) = region_mat(i);
                let mut lo = bbox_lo[i].clone();
                let mut hi = bbox_hi[i].clone();
                for k in 0..n {
                    let mut c = vec![0.0; n];
                    c[k] = 1.0;
                    if let Some(s) = crate::solver::lp::solve_lp(Some(&c), &a, &b, &[]) {
                        lo[k] = lo[k].min(s.obj);
                    }
                    c[k] = -1.0;
                    if let Some(s) = crate::solver::lp::solve_lp(Some(&c), &a, &b, &[]) {
                        hi[k] = hi[k].max(-s.obj);
                    }
                }
                Some((lo, hi))
            })
            .collect();
        for (i, bx) in boxes.into_iter().enumerate() {
            if let Some((lo, hi)) = bx {
                bbox_lo[i] = lo;
                bbox_hi[i] = hi;
            }
        }

        // distinct hyperplanes by a sweep over the first component plus union-find
        let total = f.len();
        let mut canon: Vec<(Vec<f64>, f64)> = Vec::with_capacity(total);
        for k in 0..total {
            let mut v: Vec<f64> = e[k * n..(k + 1) * n].to_vec();
            v.push(f[k]);
            let lead = v[..n].iter().find(|x| x.abs() > 1e-9).cloned().unwrap_or(1.0);
            let s = if lead < 0.0 { -1.0 } else { 1.0 };
            for x in v.iter_mut() {
                *x *= s;
            }
            canon.push((v, s));
        }
        let mut order: Vec<usize> = (0..total).collect();
        order.sort_by(|&x, &y| canon[x].0[0].partial_cmp(&canon[y].0[0]).unwrap());
        let mut parent: Vec<usize> = (0..total).collect();
        fn find(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        for (ii, &i) in order.iter().enumerate() {
            for &j in &order[ii + 1..] {
                if canon[j].0[0] - canon[i].0[0] > PLANE_TOL {
                    break;
                }
                let close = canon[i].0.iter().zip(&canon[j].0).all(|(a, b)| (a - b).abs() <= PLANE_TOL * (1.0 + a.abs()));
                if close {
                    let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                    if ri != rj {
                        parent[ri] = rj;
                    }
                }
            }
        }
        let mut plane_of_root = std::collections::HashMap::new();
        let mut planes: Vec<Vec<f64>> = vec![];
        let mut row_plane = vec![0usize; total];
        for k in 0..total {
            let root = find(&mut parent, k);
            let id = *plane_of_root.entry(root).or_insert_with(|| {
                planes.push(canon[root].0.clone());
                planes.len() - 1
            });
            row_plane[k] = id;
        }

        // region list per plane and orientation, then verified adjacency
        let mut by_plane: Vec<Vec<(usize, usize, f64)>> = vec![vec![]; planes.len()];
        let mut row_region = vec![0usize; total];
        for i in 0..r {
            for k in offsets[i]..offsets[i + 1] {
                row_region[k] = i;
                by_plane[row_plane[k]].push((i, k, canon[k].1));
            }
        }
        let mut candidate_pairs = vec![];
        for members in &by_plane {
            for &(i, ki, si) in members {
                for &(j, kj, sj) in members {
                    if i < j && si != sj {
                        candidate_pairs.push((i, ki, j, kj));
                    }
                }
            }
        }
        let verified: Vec<bool> = candidate_pairs
            .par_iter()
            .map(|&(i, ki, j, kj)| {
                let (ai, bi) = region_mat(i);
                let (aj, bj) = region_mat(j);
                let li = ki - offsets[i];
                let lj = kj - offsets[j];
                // facet of i intersected with region j (without j's opposite row), at equality on i's row
                let keep_j: Vec<usize> = (0..aj.rows).filter(|&x| x != lj).collect();
                let aj2 = aj.select_rows(&keep_j);
                let bj2: Vec<f64> = keep_j.iter().map(|&x| bj[x]).collect();
                let a = Mat::vstack(&[&ai, &aj2]);
                let mut b = bi.clone();
                b.extend(bj2);
                match chebyshev_ball(&a, &b, &[li]) {
                    Some(s) => s.x[n] > 1e-7,
                    None => false,
                }
            })
            .collect();
        let mut neighbors_of_row: Vec<Vec<usize>> = vec![vec![]; total];
        for (&(i, ki, j, kj), ok) in candidate_pairs.iter().zip(verified) {
            if ok {
                neighbors_of_row[ki].push(j);
                neighbors_of_row[kj].push(i);
            }
        }
        let mut facets = vec![];
        let mut domain_ids = std::collections::BTreeSet::new();
        for i in 0..r {
            let mut fi = vec![];
            for k in offsets[i]..offsets[i + 1] {
                let nb = neighbors_of_row[k].clone();
                if nb.is_empty() {
                    domain_ids.insert(row_plane[k]);
                }
                fi.push(Facet { row: k - offsets[i], plane: row_plane[k], orient: canon[k].1, neighbors: nb });
            }
            facets.push(fi);
        }
        // domain planes in the orientation that keeps the union inside. A row without a neighbour
        // is only accepted as a domain plane if every vertex of every region satisfies it: rows
        // that never touch their region (kept by the redundancy test on numerically thin
        // regions) would otherwise cut through the union.
        let all_vertices: Vec<&Vec<f64>> = vertices.iter().flatten().collect();
        let vscale = global_lo.iter().chain(global_hi.iter()).fold(1.0f64, |m, v| m.max(v.abs()));
        let mut domain_planes = vec![];
        for i in 0..r {
            for fc in &facets[i] {
                if fc.neighbors.is_empty() && domain_ids.remove(&fc.plane) {
                    let k = offsets[i] + fc.row;
                    let mut v = e[k * n..(k + 1) * n].to_vec();
                    v.push(f[k]);
                    let valid = all_vertices.iter().all(|p| dot(&v[..n], p) - v[n] <= 1e-7 * vscale);
                    if valid {
                        domain_planes.push(v);
                    }
                }
            }
        }
        PartitionGeometry {
            dim: n,
            num_regions: r,
            tol: sol.point_location_tolerance,
            e,
            f,
            offsets,
            centers,
            radii,
            vertices,
            bbox_lo,
            bbox_hi,
            planes,
            facets,
            domain_planes,
            global_lo,
            global_hi,
            thin,
        }
    }

    #[inline]
    pub fn region_rows(&self, i: usize) -> (&[f64], &[f64]) {
        let (s, t) = (self.offsets[i], self.offsets[i + 1]);
        (&self.e[s * self.dim..t * self.dim], &self.f[s..t])
    }

    #[inline]
    pub fn contains(&self, i: usize, theta: &[f64], tol: f64) -> bool {
        let n = self.dim;
        let (e, f) = self.region_rows(i);
        for (k, fk) in f.iter().enumerate() {
            if dot(&e[k * n..(k + 1) * n], theta) - fk > tol {
                return false;
            }
        }
        true
    }

    /// Largest side of the bounding box of the partition.
    pub fn extent(&self) -> f64 {
        (0..self.dim).map(|k| self.global_hi[k] - self.global_lo[k]).fold(0.0f64, f64::max)
    }

    pub fn in_domain(&self, theta: &[f64], tol: f64) -> bool {
        let n = self.dim;
        self.domain_planes.iter().all(|p| dot(&p[..n], theta) - p[n] <= tol)
    }

    pub fn region_mat(&self, i: usize) -> (Mat, Vec<f64>) {
        let (e, f) = self.region_rows(i);
        (Mat::from_vec(f.len(), self.dim, e.to_vec()), f.to_vec())
    }

    pub fn num_adjacencies(&self) -> usize {
        self.facets.iter().flat_map(|fs| fs.iter().map(|f| f.neighbors.len())).sum::<usize>() / 2
    }

    /// Facets shared with two or more regions on the other side. A regular subdivision (and hence
    /// any partition that admits a linear machine) is face-to-face, so a nonzero count proves
    /// that no exact linear machine exists for this partition.
    pub fn non_facet_to_facet(&self) -> usize {
        self.facets.iter().flatten().filter(|f| f.neighbors.len() >= 2).count()
    }

    /// All (i, j, row of i) adjacency triples with i < j.
    pub fn adjacency_list(&self) -> Vec<(usize, usize, usize)> {
        let mut out = vec![];
        for i in 0..self.num_regions {
            for fc in &self.facets[i] {
                for &j in &fc.neighbors {
                    if i < j {
                        out.push((i, j, fc.row));
                    }
                }
            }
        }
        out
    }
}

/// Vertices of a bounded polytope {x : A x <= b} by enumerating n-subsets of rows.
pub fn enumerate_vertices(a: &Mat, b: &[f64]) -> Vec<Vec<f64>> {
    enumerate_vertices_checked(a, b).0
}

/// [`enumerate_vertices`] plus a flag that is true when the list may be incomplete: some basic
/// solution violated a row by more than the acceptance tolerance (1e-9 of the row scale) but by at
/// most 1e-7 of it, which is how ill-conditioned bases of a thin region show up (the true vertex
/// is computed with an error larger than the tolerance and gets rejected).
pub fn enumerate_vertices_checked(a: &Mat, b: &[f64]) -> (Vec<Vec<f64>>, bool) {
    let n = a.cols;
    let m = a.rows;
    let mut out: Vec<Vec<f64>> = vec![];
    let mut suspicious = false;
    if m < n || n == 0 {
        return (out, false);
    }
    if n == 1 {
        let mut lo = f64::NEG_INFINITY;
        let mut hi = f64::INFINITY;
        for i in 0..m {
            let ai = a[(i, 0)];
            if ai > 0.0 {
                hi = hi.min(b[i] / ai);
            } else if ai < 0.0 {
                lo = lo.max(b[i] / ai);
            }
        }
        if lo.is_finite() {
            out.push(vec![lo]);
        }
        if hi.is_finite() && (hi - lo).abs() > 1e-12 {
            out.push(vec![hi]);
        }
        return (out, false);
    }
    let mut idx: Vec<usize> = (0..n).collect();
    let scale = b.iter().fold(1.0f64, |s, v| s.max(v.abs()));
    loop {
        let sub = a.select_rows(&idx);
        if let Some(lu) = Lu::new(&sub) {
            let rhs: Vec<f64> = idx.iter().map(|&i| b[i]).collect();
            let x = lu.solve(&rhs);
            if x.iter().all(|v| v.is_finite()) {
                let viol = (0..m).map(|i| dot(a.row(i), &x) - b[i]).fold(f64::NEG_INFINITY, f64::max);
                if viol <= 1e-9 * scale {
                    if !out.iter().any(|p| p.iter().zip(&x).all(|(u, v)| (u - v).abs() <= 1e-9 * (1.0 + u.abs()))) {
                        out.push(x);
                    }
                } else if viol <= 1e-7 * scale {
                    suspicious = true;
                }
            }
        }
        // next combination
        let mut k = n;
        loop {
            if k == 0 {
                return (out, suspicious);
            }
            k -= 1;
            if idx[k] < m - n + k {
                idx[k] += 1;
                for t in k + 1..n {
                    idx[t] = idx[t - 1] + 1;
                }
                break;
            }
        }
    }
}

/// Does {x : A x <= b} contain a ball of radius above `thr`? Used to decide that a region misses a
/// cell or a cut with positive volume, so a wrong "no" drops a region a query can fall into.
///
/// The answer is conservative: `false` is returned only with a verified certificate. The plain
/// Chebyshev LP is not reliable enough on its own: for thin or badly scaled polytopes (a slab of
/// width 2.5e-4 at coordinates of magnitude 38) it returned a radius 0 to 28 times too small
/// although the polytope contained a known interior point. So:
///   * a hint point (for example a region center) inside with slack above `thr` answers `true`;
///   * the Chebyshev LP is solved on data shifted to `centre` and scaled by `scale`, and a radius
///     above `thr` answers `true`;
///   * otherwise the LP multipliers `y` are used as a Farkas-type certificate: for any `y >= 0`,
///     every ball (x, r) inside satisfies `r <= (y'b + |A'y|_1 |x - centre|_inf) / (y'|a|)`. If that
///     bound is at most `thr` the answer is `false`; if the multipliers do not prove it (or the LP
///     fails), the answer is `true`.
/// `xbound` bounds `|x - centre|_inf` over the polytope.
/// Diagnostics of [`has_interior`]: calls answered `false` (certified), and calls where the LP
/// radius was at most `thr` but the multipliers did not certify it (kept conservatively).
pub static INTERIOR_CERTIFIED_NO: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
pub static INTERIOR_UNCERTIFIED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn has_interior(a: &Mat, b: &[f64], thr: f64, hints: &[&[f64]], centre: &[f64], scale: f64, xbound: f64) -> bool {
    let n = a.cols;
    let m = a.rows;
    for h in hints {
        if h.len() == n && (0..m).all(|i| b[i] - dot(a.row(i), h) > thr * crate::linalg::norm(a.row(i)).max(1e-300) * 1.0000001) {
            return true;
        }
    }
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let norms: Vec<f64> = (0..m).map(|i| crate::linalg::norm(a.row(i))).collect();
    let mut ab = Mat::zeros(m + 1, n + 1);
    let mut bb = Vec::with_capacity(m + 1);
    for i in 0..m {
        ab.row_mut(i)[..n].copy_from_slice(a.row(i));
        ab[(i, n)] = norms[i];
        bb.push((b[i] - dot(a.row(i), centre)) / scale);
    }
    ab[(m, n)] = -1.0;
    bb.push(0.0);
    let mut c = vec![0.0; n + 1];
    c[n] = -1.0;
    let Some(sol) = crate::solver::lp::solve_lp(Some(&c), &ab, &bb, &[]) else {
        // empty polytope or numerical failure: only an independent infeasibility verdict prunes
        return crate::solver::lp::lp_feasible(a, b, &[]);
    };
    if sol.x[n] * scale > thr {
        return true;
    }
    // certificate from the multipliers of the m region rows (clamped at zero)
    let y: Vec<f64> = sol.dual[..m].iter().map(|v| if v.is_finite() { v.max(0.0) } else { 0.0 }).collect();
    let q: f64 = (0..m).map(|i| y[i] * norms[i]).sum();
    if !(q > 0.0) {
        INTERIOR_UNCERTIFIED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return true;
    }
    let d: f64 = (0..m).map(|i| y[i] * bb[i]).sum();
    let mut gsum = 0.0;
    for k in 0..n {
        gsum += (0..m).map(|i| y[i] * a[(i, k)]).sum::<f64>().abs();
    }
    let ubound = xbound / scale;
    let bound = (d + gsum * ubound) / q;
    // margin for the rounding of the bound itself
    if bound * scale <= 0.5 * thr {
        INTERIOR_CERTIFIED_NO.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        false
    } else {
        INTERIOR_UNCERTIFIED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        true
    }
}
