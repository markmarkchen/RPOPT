//! Sequential search.
//!
//! [`SequentialSearch`] is a direct port of PPOPT's numba `PointLocation`: evaluate every stacked
//! region constraint `Aθ <= b` first, then return the first region whose rows all hold.
//! [`FlatSequential`] stops at the first violated row of each region instead.

use super::{PointLocator, LOCATE_TOL};
use crate::linalg::dot;
use crate::solution::Solution;

pub struct SequentialSearch {
    dim: usize,
    a: Vec<f64>,
    b: Vec<f64>,
    offsets: Vec<usize>,
}

impl SequentialSearch {
    pub fn new(sol: &Solution) -> Self {
        let dim = sol.theta_dim();
        let mut a = vec![];
        let mut b = vec![];
        let mut offsets = vec![0];
        for r in &sol.critical_regions {
            a.extend_from_slice(&r.e.data);
            b.extend_from_slice(&r.f);
            offsets.push(b.len());
        }
        SequentialSearch { dim, a, b, offsets }
    }
}

impl PointLocator for SequentialSearch {
    fn name(&self) -> &'static str {
        "sequential (PPOPT)"
    }

    fn locate(&self, theta: &[f64]) -> Option<usize> {
        let n = self.dim;
        // test = A θ <= b for every stacked row, as in PPOPT
        let test: Vec<bool> = (0..self.b.len()).map(|k| dot(&self.a[k * n..(k + 1) * n], theta) <= self.b[k] + LOCATE_TOL).collect();
        for j in 0..self.offsets.len() - 1 {
            if test[self.offsets[j]..self.offsets[j + 1]].iter().all(|&t| t) {
                return Some(j);
            }
        }
        None
    }

    fn storage_floats(&self) -> usize {
        self.a.len() + self.b.len()
    }
}

/// Early-exit sequential search over contiguous rows stored as [a | b].
pub struct FlatSequential {
    dim: usize,
    rows: Vec<f64>,
    offsets: Vec<usize>,
}

impl FlatSequential {
    pub fn new(sol: &Solution) -> Self {
        let dim = sol.theta_dim();
        let mut rows = vec![];
        let mut offsets = vec![0];
        for r in &sol.critical_regions {
            for i in 0..r.e.rows {
                rows.extend_from_slice(r.e.row(i));
                rows.push(r.f[i]);
            }
            offsets.push(offsets.last().unwrap() + r.e.rows);
        }
        FlatSequential { dim, rows, offsets }
    }

    #[inline]
    pub fn region_contains(&self, j: usize, theta: &[f64]) -> bool {
        let w = self.dim + 1;
        let n = self.dim;
        for k in self.offsets[j]..self.offsets[j + 1] {
            let r = &self.rows[k * w..(k + 1) * w];
            if dot(&r[..n], theta) > r[n] + LOCATE_TOL {
                return false;
            }
        }
        true
    }

    /// Largest row violation of region j, infinity once a row exceeds `LOCATE_TOL`.
    #[inline]
    pub fn region_violation(&self, j: usize, theta: &[f64]) -> f64 {
        let w = self.dim + 1;
        let n = self.dim;
        let mut worst = f64::NEG_INFINITY;
        for k in self.offsets[j]..self.offsets[j + 1] {
            let r = &self.rows[k * w..(k + 1) * w];
            let v = dot(&r[..n], theta) - r[n];
            if v > LOCATE_TOL {
                return f64::INFINITY;
            }
            worst = worst.max(v);
        }
        worst
    }

    pub fn num_regions(&self) -> usize {
        self.offsets.len() - 1
    }
}

impl PointLocator for FlatSequential {
    fn name(&self) -> &'static str {
        "sequential (early exit)"
    }

    fn locate(&self, theta: &[f64]) -> Option<usize> {
        super::pick_strict_first(0..self.num_regions(), |j| self.region_violation(j, theta))
    }

    fn storage_floats(&self) -> usize {
        self.rows.len()
    }
}
