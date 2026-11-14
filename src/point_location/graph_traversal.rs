//! Graph traversal (Herceg, Mariethoz & Morari, "Evaluation of piecewise affine control law via
//! graph traversal", ECC 2013, Sec. III-B, Algorithm 1).
//!
//! The regions form a graph (vertices: regions, edges: shared facets). Starting from a region,
//! the halfspace test `H x0 <= l` is evaluated; the violated rows `F` (eq. 6) name the facets that
//! lead towards `x0`. One facet is chosen and the walk moves to the adjacent region across it
//! (eq. 7, 8) until no row is violated. The facet is chosen by option c) of Sec. III-B: the
//! violated row with the largest normalised distance `(h_j' x0 - l_j) / |h_j|` (rows are stored
//! normalised, so this is the largest violation); this needs no extra points and the paper
//! reports it as the option with the fewest iterations.
//!
//! The paper assumes a convex union without gaps, minimal hyperplane representation, and
//! (Remark 3.2) lets the walk pick any region of a multi-neighbour facet. This implementation
//! picks, among the neighbours across the chosen facet, the one with the smallest maximum
//! violation at `x0`. The default start of a cold `locate` is region 0 (level 1 of the BFS in the
//! paper); regions are numbered by the solver here, not by BFS level.
//!
//! Exactness: the region returned is the one whose rows all satisfy the shared f64 test
//! (`LOCATE_TOL`). If the traversal cannot proceed (violated facet without neighbour, which is the
//! case for queries outside the feasible set; Remark 3.3 cycles on non-convex unions; more than
//! `max_steps` steps) it falls back to an exact locator (the hash table).

use super::geometry::PartitionGeometry;
use super::hash_table::HashTable;
use super::walk::{RowGraph, WalkOutcome};
use super::{PointLocator, LOCATE_TOL};
use crate::linalg::dot;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct GraphTraversal {
    g: RowGraph,
    fallback: Arc<dyn PointLocator>,
    max_steps: usize,
    /// last answer of `locate`; a hint only, so races between threads are harmless
    last: AtomicUsize,
}

impl GraphTraversal {
    pub fn new(geom: &PartitionGeometry) -> Self {
        Self::with_fallback(geom, Arc::new(HashTable::with_default_limit(geom)))
    }

    pub fn with_fallback(geom: &PartitionGeometry, fallback: Arc<dyn PointLocator>) -> Self {
        GraphTraversal { g: RowGraph::new(geom), fallback, max_steps: geom.num_regions.clamp(8, 4096), last: AtomicUsize::new(0) }
    }

    /// largest violation of region i at theta and the row achieving it
    #[inline]
    fn worst(&self, i: usize, theta: &[f64]) -> (f64, usize) {
        let n = self.g.dim;
        let (e, f) = self.g.rows(i);
        let mut vmax = f64::NEG_INFINITY;
        let mut kmax = 0;
        for k in 0..f.len() {
            let v = dot(&e[k * n..(k + 1) * n], theta) - f[k];
            if v > vmax {
                vmax = v;
                kmax = k;
            }
        }
        (vmax, kmax)
    }

    #[inline]
    fn traverse(&self, theta: &[f64], start: usize) -> Result<(usize, u32), u32> {
        if start >= self.g.num_regions || theta.len() < self.g.dim {
            return Err(0);
        }
        let mut cur = start;
        for step in 0..self.max_steps as u32 {
            let (v, k) = self.worst(cur, theta);
            if v.is_nan() {
                return Err(step);
            }
            if v <= LOCATE_TOL {
                return Ok((cur, step));
            }
            let nbs = self.g.neighbors(self.g.row0(cur) + k);
            cur = match nbs.len() {
                0 => return Err(step),
                1 => nbs[0] as usize,
                _ => {
                    let mut best = (f64::INFINITY, nbs[0]);
                    for &j in nbs {
                        let w = self.worst(j as usize, theta).0;
                        if w < best.0 {
                            best = (w, j);
                        }
                    }
                    best.1 as usize
                }
            };
        }
        Err(self.max_steps as u32)
    }

    /// Locates `theta` starting the traversal in `start`; exact (falls back when it fails).
    pub fn locate_from_stats(&self, theta: &[f64], start: usize) -> WalkOutcome {
        match self.traverse(theta, start) {
            Ok((r, steps)) => WalkOutcome { region: Some(r), steps, fell_back: false },
            Err(steps) => WalkOutcome { region: self.fallback.locate(theta), steps, fell_back: true },
        }
    }

    pub fn locate_from(&self, theta: &[f64], start: usize) -> Option<usize> {
        self.locate_from_stats(theta, start).region
    }

    /// `locate_from` with the last answer of this instance as the start; updates it.
    pub fn locate_stats(&self, theta: &[f64]) -> WalkOutcome {
        let o = self.locate_from_stats(theta, self.last.load(Ordering::Relaxed));
        if let Some(r) = o.region {
            self.last.store(r, Ordering::Relaxed);
        }
        o
    }

    pub fn graph(&self) -> &RowGraph {
        &self.g
    }
}

impl PointLocator for GraphTraversal {
    fn name(&self) -> &'static str {
        "graph traversal"
    }

    fn locate(&self, theta: &[f64]) -> Option<usize> {
        self.locate_stats(theta).region
    }

    /// region rows and adjacency list; the fallback table is extra
    fn storage_floats(&self) -> usize {
        self.g.storage_floats()
    }
}
