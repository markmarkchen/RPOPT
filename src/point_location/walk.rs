//! Subdivision walking (Wang, Jones & Maciejowski, "Efficient point location via subdivision
//! walking with application to explicit MPC", ECC 2007, Sec. III, Algorithm 1).
//!
//! Queries that arrive in sequence (closed-loop explicit MPC) tend to stay close to the previous
//! answer. The walk starts in a seed region (the previous answer, or a caller hint) with a seed
//! point `x_s` inside it (the Chebyshev center), and follows the ray from `x_s` towards the query
//! `x_m`: in the current region, the crossing parameter of every bounding hyperplane with
//! `H_j nu > 0` is `lambda_j = (K_j - H_j x_s) / (H_j nu)`, `nu = x_m - x_s`; the smallest one
//! names the exit facet, and the walk continues in the neighbour across it, until the region
//! containing `x_m` is found (Theorem III.1: at most R iterations). Because the seed point and
//! direction stay fixed, the crossing parameter is measured along one line and the epsilon step of
//! Algorithm 1 (line 13) is not needed.
//!
//! Partitions with several neighbours across a facet (not facet-to-facet, end of Sec. III): the
//! neighbour is the one that contains the point of the ray just beyond the crossing point.
//!
//! Exactness: the walk only proposes a candidate. The candidate is verified with the f64 halfspace
//! test of the other locators (`LOCATE_TOL`). Every case in which the walk cannot proceed (no
//! neighbour across the exit facet: outside the feasible set or a non-convex union; no neighbour
//! contains the continuation point; a crossing parameter that decreases; more than `max_steps`
//! steps; an unusable seed) falls back to an exact locator (the hash table), never a guess.

use super::geometry::PartitionGeometry;
use super::hash_table::HashTable;
use super::{PointLocator, LOCATE_TOL};
use crate::linalg::dot;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// largest parameter dimension handled by the stack buffers of the walks
pub(crate) const MAX_DIM: usize = 16;

/// Region rows (normalised, as in [`PartitionGeometry`]) and, for every row, the regions across
/// that facet in compressed sparse row form. The adjacency list `G(r, f)` of Herceg et al.
pub struct RowGraph {
    pub dim: usize,
    pub num_regions: usize,
    e: Box<[f64]>,
    f: Box<[f64]>,
    offsets: Box<[u32]>,
    nb_off: Box<[u32]>,
    nb: Box<[u32]>,
}

impl RowGraph {
    pub fn new(geom: &PartitionGeometry) -> Self {
        let mut nb_off = vec![0u32];
        let mut nb = vec![];
        for fs in &geom.facets {
            // facets are stored per region; index them by row
            let mut by_row: Vec<&[usize]> = vec![&[]; fs.len()];
            for fc in fs {
                by_row[fc.row] = &fc.neighbors;
            }
            for nbs in by_row {
                nb.extend(nbs.iter().map(|&j| j as u32));
                nb_off.push(nb.len() as u32);
            }
        }
        RowGraph {
            dim: geom.dim,
            num_regions: geom.num_regions,
            e: geom.e.clone().into(),
            f: geom.f.clone().into(),
            offsets: geom.offsets.iter().map(|&o| o as u32).collect(),
            nb_off: nb_off.into(),
            nb: nb.into(),
        }
    }

    #[inline]
    pub fn rows(&self, i: usize) -> (&[f64], &[f64]) {
        let (s, t) = (self.offsets[i] as usize, self.offsets[i + 1] as usize);
        (&self.e[s * self.dim..t * self.dim], &self.f[s..t])
    }

    /// global index of the first row of region i
    #[inline]
    pub fn row0(&self, i: usize) -> usize {
        self.offsets[i] as usize
    }

    /// regions across the facet given by global row index `g`
    #[inline]
    pub fn neighbors(&self, g: usize) -> &[u32] {
        &self.nb[self.nb_off[g] as usize..self.nb_off[g + 1] as usize]
    }

    /// The membership test shared with the other locators (within `LOCATE_TOL`).
    #[inline]
    pub fn contains(&self, i: usize, theta: &[f64]) -> bool {
        self.violation(i, theta) <= LOCATE_TOL
    }

    /// Largest row violation of region i at theta; infinity once a row exceeds `LOCATE_TOL`.
    /// A value at most 0 means that theta lies in the region strictly.
    #[inline]
    pub fn violation(&self, i: usize, theta: &[f64]) -> f64 {
        let n = self.dim;
        let (e, f) = self.rows(i);
        let mut worst = f64::NEG_INFINITY;
        for (k, fk) in f.iter().enumerate() {
            let v = dot(&e[k * n..(k + 1) * n], theta) - fk;
            if v > LOCATE_TOL {
                return f64::INFINITY;
            }
            worst = worst.max(v);
        }
        worst
    }

    pub fn storage_floats(&self) -> usize {
        self.e.len() + self.f.len() + (self.offsets.len() + self.nb_off.len() + self.nb.len()).div_ceil(2)
    }
}

/// Outcome of one query with its cost.
#[derive(Clone, Copy, Debug)]
pub struct WalkOutcome {
    pub region: Option<usize>,
    /// facet crossings made before the answer (or before giving up)
    pub steps: u32,
    /// true if the exact fallback answered
    pub fell_back: bool,
}

pub struct SubdivisionWalk {
    g: RowGraph,
    /// seed point of every region (flattened), valid where `seed_ok`
    seeds: Box<[f64]>,
    seed_ok: Box<[bool]>,
    fallback: Arc<dyn PointLocator>,
    max_steps: usize,
    /// last answer of `locate`; a hint only, so races between threads are harmless
    last: AtomicUsize,
}

impl SubdivisionWalk {
    /// With the default hash table as exact fallback.
    pub fn new(geom: &PartitionGeometry) -> Self {
        Self::with_fallback(geom, Arc::new(HashTable::with_default_limit(geom)))
    }

    pub fn with_fallback(geom: &PartitionGeometry, fallback: Arc<dyn PointLocator>) -> Self {
        let g = RowGraph::new(geom);
        let n = geom.dim;
        let mut seeds = vec![0.0; n * geom.num_regions];
        let mut seed_ok = vec![false; geom.num_regions];
        for i in 0..geom.num_regions {
            let mut cand: Vec<Vec<f64>> = vec![];
            if geom.radii[i] > 1e-9 {
                cand.push(geom.centers[i].clone());
            }
            if !geom.vertices[i].is_empty() {
                let mut m = vec![0.0; n];
                for v in &geom.vertices[i] {
                    for k in 0..n {
                        m[k] += v[k] / geom.vertices[i].len() as f64;
                    }
                }
                cand.push(m);
            }
            for c in cand {
                if g.contains(i, &c) {
                    seeds[i * n..(i + 1) * n].copy_from_slice(&c);
                    seed_ok[i] = true;
                    break;
                }
            }
        }
        SubdivisionWalk {
            g,
            seeds: seeds.into(),
            seed_ok: seed_ok.into(),
            fallback,
            max_steps: geom.num_regions.clamp(8, 4096),
            last: AtomicUsize::new(0),
        }
    }

    /// The walk alone: `Ok((region, steps))` or `Err(steps)` when it cannot answer.
    #[inline]
    fn walk(&self, theta: &[f64], start: usize) -> Result<(usize, u32), u32> {
        let n = self.g.dim;
        if start >= self.g.num_regions || n > MAX_DIM || theta.len() < n {
            return Err(0);
        }
        // the common case in a sequence: the previous region still contains the point
        if self.g.violation(start, theta) <= 0.0 {
            return Ok((start, 0));
        }
        if !self.seed_ok[start] {
            return Err(0);
        }
        let c = &self.seeds[start * n..(start + 1) * n];
        let mut delta = [0.0f64; MAX_DIM];
        for k in 0..n {
            delta[k] = theta[k] - c[k];
        }
        let delta = &delta[..n];
        let mut cur = start;
        let mut t_prev = 0.0f64;
        for step in 0..self.max_steps as u32 {
            let (e, f) = self.g.rows(cur);
            let mut best = f64::INFINITY;
            let mut bk = usize::MAX;
            for k in 0..f.len() {
                let row = &e[k * n..(k + 1) * n];
                let d = dot(row, delta);
                if d > 1e-14 {
                    let t = (f[k] - dot(row, c)).max(0.0) / d;
                    if t < best {
                        best = t;
                        bk = k;
                    }
                }
            }
            if bk == usize::MAX || best >= 1.0 {
                // strict-first rule: a point that lies in this region only within the tolerance may
                // lie strictly in a neighbour, which the exact fallback decides
                return if self.g.violation(cur, theta) <= 0.0 { Ok((cur, step)) } else { Err(step) };
            }
            if best < t_prev - 1e-9 {
                return Err(step);
            }
            t_prev = t_prev.max(best);
            let nbs = self.g.neighbors(self.g.row0(cur) + bk);
            cur = match nbs.len() {
                0 => return Err(step),
                1 => nbs[0] as usize,
                _ => {
                    // several regions across the facet: take the one containing the ray just beyond
                    let tq = (best + 1e-6).min(0.5 * (best + 1.0));
                    let mut q = [0.0f64; MAX_DIM];
                    let mut p = [0.0f64; MAX_DIM];
                    for k in 0..n {
                        q[k] = c[k] + tq * delta[k];
                        p[k] = c[k] + best * delta[k];
                    }
                    match nbs.iter().find(|&&j| self.g.contains(j as usize, &q[..n])).or_else(|| nbs.iter().find(|&&j| self.g.contains(j as usize, &p[..n]))) {
                        Some(&j) => j as usize,
                        None => return Err(step),
                    }
                }
            };
        }
        Err(self.max_steps as u32)
    }

    /// Locates `theta` starting the walk in `start`; exact (falls back when the walk fails).
    pub fn locate_from_stats(&self, theta: &[f64], start: usize) -> WalkOutcome {
        match self.walk(theta, start) {
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

impl PointLocator for SubdivisionWalk {
    fn name(&self) -> &'static str {
        "subdivision walking"
    }

    fn locate(&self, theta: &[f64]) -> Option<usize> {
        self.locate_stats(theta).region
    }

    /// walk data only (rows, adjacency, seeds); the fallback table is extra
    fn storage_floats(&self) -> usize {
        self.g.storage_floats() + self.seeds.len()
    }
}
