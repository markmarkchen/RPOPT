//! Linear machine (Airan, Bhushan & Bhartiya, 2017, "Linear Machine Solution to Point Location
//! Problem", IEEE TAC 62(3)).
//!
//! Each region i gets an affine discriminant g_i(θ) = w_i'θ + w_i0 such that θ ∈ R_i iff g_i is
//! the largest. Online cost: one (R x (n+1)) mat-vec and an argmax, no tree and no branching.
//!
//! Construction (offline LP): for every pair of adjacent regions (i, j) separated by a'θ <= b on
//! the side of i, require g_i - g_j = α_ij (b - a'θ) with α_ij >= 1. For a regular subdivision
//! (mpQP partitions are), these local conditions make the argmax globally correct. The LP is large
//! and sparse, so it is solved with Clarabel.
//!
//! Numerical check (after Sharma & Bhushan, 2020, "Some necessary and sufficient conditions for
//! correctness of linear machine in presence of numerical errors", IFAC-PapersOnLine 53(2)):
//! g_i - g_j is affine, so the machine is correct on R_i iff g_i >= g_j at every vertex of R_i for
//! every j. We evaluate that condition in floating point and report the worst violation and the
//! smallest interior margin. Online, the argmax region is verified and its neighbours are tried
//! next, so a near-tie at a facet cannot return a wrong region.

use super::geometry::PartitionGeometry;
use super::PointLocator;
use crate::linalg::dot;
use crate::solver::sparse_lp::SparseLp;

#[derive(Clone, Debug)]
pub struct LinearMachineReport {
    /// max over regions i, vertices v of R_i, regions j of g_j(v) - g_i(v) (should be <= ~0)
    pub max_vertex_violation: f64,
    /// min over regions of g_i(c_i) - max_{j != i} g_j(c_i) at the Chebyshev center
    pub min_center_margin: f64,
    /// regions whose Chebyshev center is classified correctly
    pub centers_correct: usize,
    pub num_regions: usize,
    /// all conditions hold within tolerance (on the regions in the machine)
    pub certified: bool,
    /// sliver regions left out of the LP (reached through the neighbour fallback)
    pub excluded: usize,
}

pub struct LinearMachine {
    dim: usize,
    /// R x (n+1) weights [w_i | w_i0]
    pub weights: Vec<f64>,
    num_regions: usize,
    rows: Vec<f64>,
    offsets: Vec<usize>,
    neighbors: Vec<Vec<usize>>,
    domain: Vec<Vec<f64>>,
    pub report: LinearMachineReport,
    /// verify the argmax region (and fall back) before returning
    pub verify: bool,
    tol: f64,
}

impl LinearMachine {
    /// Builds the machine on all regions; if the LP is infeasible (numerically thin sliver regions
    /// make the adjacency conditions inconsistent), slivers below a growing relative Chebyshev
    /// radius are excluded from the LP. Excluded regions get g = -inf and are reached through the
    /// neighbour fallback of `locate`.
    pub fn new(geom: &PartitionGeometry) -> Option<Self> {
        for rel in [0.0, 1e-7, 1e-6, 1e-5, 1e-4, 1e-3] {
            if let Some(lm) = Self::with_min_radius(geom, rel) {
                return Some(lm);
            }
        }
        None
    }

    pub fn with_min_radius(geom: &PartitionGeometry, rel_radius: f64) -> Option<Self> {
        let n = geom.dim;
        let w = n + 1;
        let r = geom.num_regions;
        let extent = (0..n).map(|k| geom.global_hi[k] - geom.global_lo[k]).fold(0.0f64, f64::max).max(1e-12);
        let excluded: Vec<bool> = (0..r).map(|i| geom.radii[i] < rel_radius * extent).collect();
        let adj: Vec<(usize, usize, usize)> = geom.adjacency_list().into_iter().filter(|&(i, j, _)| !excluded[i] && !excluded[j]).collect();
        let nv = r * w + adj.len();
        let mut lp = SparseLp::new(nv);
        for (e, &(i, j, row)) in adj.iter().enumerate() {
            let (ei, fi) = geom.region_rows(i);
            let a = &ei[row * n..(row + 1) * n];
            let b = fi[row];
            let alpha = r * w + e;
            for k in 0..n {
                lp.add_eq(&[(i * w + k, 1.0), (j * w + k, -1.0), (alpha, a[k])], 0.0);
            }
            lp.add_eq(&[(i * w + n, 1.0), (j * w + n, -1.0), (alpha, -b)], 0.0);
            lp.add_le(&[(alpha, -1.0)], -1.0);
            lp.c[alpha] = 1.0;
        }
        // gauge: first region of every connected component has zero weights
        let mut comp = vec![usize::MAX; r];
        let mut nbrs = vec![vec![]; r];
        for &(i, j, _) in &adj {
            nbrs[i].push(j);
            nbrs[j].push(i);
        }
        for s in 0..r {
            if comp[s] != usize::MAX || excluded[s] {
                continue;
            }
            comp[s] = s;
            for k in 0..w {
                lp.add_eq(&[(s * w + k, 1.0)], 0.0);
            }
            let mut st = vec![s];
            while let Some(u) = st.pop() {
                for &v in &nbrs[u] {
                    if comp[v] == usize::MAX {
                        comp[v] = s;
                        st.push(v);
                    }
                }
            }
        }
        // excluded regions: pin their (unused) weights to zero so the LP stays bounded
        for i in 0..r {
            if excluded[i] {
                for k in 0..w {
                    lp.add_eq(&[(i * w + k, 1.0)], 0.0);
                }
            }
        }
        let x = lp.solve()?;
        let mut weights = x[..r * w].to_vec();
        for i in 0..r {
            if excluded[i] {
                weights[i * w + n] = f64::NEG_INFINITY;
            }
        }
        let num_excluded = excluded.iter().filter(|&&e| e).count();

        let mut rows = vec![];
        for i in 0..r {
            let (e, f) = geom.region_rows(i);
            for k in 0..f.len() {
                rows.extend_from_slice(&e[k * n..(k + 1) * n]);
                rows.push(f[k]);
            }
        }
        let mut neighbors = vec![vec![]; r];
        for i in 0..r {
            for fc in &geom.facets[i] {
                for &j in &fc.neighbors {
                    if !neighbors[i].contains(&j) {
                        neighbors[i].push(j);
                    }
                }
            }
        }
        let mut lm = LinearMachine {
            dim: n,
            weights,
            num_regions: r,
            rows,
            offsets: geom.offsets.clone(),
            neighbors,
            domain: geom.domain_planes.clone(),
            report: LinearMachineReport { max_vertex_violation: 0.0, min_center_margin: 0.0, centers_correct: 0, num_regions: r, certified: false, excluded: num_excluded },
            verify: true,
            tol: geom.tol,
        };
        let mut rep = lm.check(geom);
        rep.excluded = num_excluded;
        lm.report = rep;
        Some(lm)
    }

    #[inline]
    fn g(&self, i: usize, theta: &[f64]) -> f64 {
        let w = self.dim + 1;
        let wi = &self.weights[i * w..(i + 1) * w];
        dot(&wi[..self.dim], theta) + wi[self.dim]
    }

    /// argmax_i g_i(θ) without verification.
    #[inline]
    pub fn argmax(&self, theta: &[f64]) -> usize {
        let w = self.dim + 1;
        let n = self.dim;
        let mut best = f64::NEG_INFINITY;
        let mut bi = 0;
        for (i, wi) in self.weights.chunks_exact(w).enumerate() {
            let v = dot(&wi[..n], theta) + wi[n];
            if v > best {
                best = v;
                bi = i;
            }
        }
        bi
    }

    fn check(&self, geom: &PartitionGeometry) -> LinearMachineReport {
        let r = self.num_regions;
        let mut max_v = f64::NEG_INFINITY;
        let mut min_margin = f64::INFINITY;
        let mut correct = 0;
        let scale = self.weights.iter().filter(|v| v.is_finite()).fold(1.0f64, |m, v| m.max(v.abs()));
        let mut counted = 0;
        for i in 0..r {
            if self.g(i, &geom.centers[i]) == f64::NEG_INFINITY {
                continue;
            }
            counted += 1;
            for v in &geom.vertices[i] {
                let gi = self.g(i, v);
                for j in 0..r {
                    if j != i {
                        max_v = max_v.max(self.g(j, v) - gi);
                    }
                }
            }
            let c = &geom.centers[i];
            let gi = self.g(i, c);
            let other = (0..r).filter(|&j| j != i).map(|j| self.g(j, c)).fold(f64::NEG_INFINITY, f64::max);
            min_margin = min_margin.min(gi - other);
            if self.argmax(c) == i {
                correct += 1;
            }
        }
        if r <= 1 {
            max_v = 0.0;
            min_margin = f64::INFINITY;
        }
        LinearMachineReport {
            max_vertex_violation: max_v,
            min_center_margin: min_margin,
            centers_correct: correct,
            num_regions: r,
            certified: correct == counted && max_v <= 1e-6 * scale,
            excluded: 0,
        }
    }

    /// Largest row violation of region j, infinity once a row exceeds the tolerance.
    #[inline]
    fn region_violation(&self, j: usize, theta: &[f64]) -> f64 {
        let w = self.dim + 1;
        let n = self.dim;
        let mut worst = f64::NEG_INFINITY;
        for k in self.offsets[j]..self.offsets[j + 1] {
            let r = &self.rows[k * w..(k + 1) * w];
            let v = dot(&r[..n], theta) - r[n];
            if v > self.tol {
                return f64::INFINITY;
            }
            worst = worst.max(v);
        }
        worst
    }
}

impl PointLocator for LinearMachine {
    fn name(&self) -> &'static str {
        "linear machine"
    }

    fn locate(&self, theta: &[f64]) -> Option<usize> {
        let i = self.argmax(theta);
        if !self.verify {
            return Some(i);
        }
        // strict-first rule shared by all locators: the argmax region answers when it contains
        // theta strictly; a strict neighbour next; otherwise the full strict-first scan
        if self.region_violation(i, theta) <= 0.0 {
            return Some(i);
        }
        let n = self.dim;
        if !self.domain.iter().all(|p| dot(&p[..n], theta) <= p[n] + self.tol) {
            return None;
        }
        if let Some(&j) = self.neighbors[i].iter().find(|&&j| self.region_violation(j, theta) <= 0.0) {
            return Some(j);
        }
        super::pick_strict_first(self.tol, 0..self.num_regions, |j| self.region_violation(j, theta))
    }

    fn storage_floats(&self) -> usize {
        self.weights.len() + if self.verify { self.rows.len() } else { 0 }
    }
}
