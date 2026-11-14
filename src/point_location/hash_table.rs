//! Hash table point location (Bayat, Johansen & Jalali, 2011, "Using hash tables to manage the
//! time-storage complexity in a point location problem: Application to explicit model predictive
//! control", Automatica 47(3)).
//!
//! The bounding box of the partition is split into a uniform grid of boxes. Offline, each box
//! stores the regions that intersect it (bounding-box prefilter, then an exact LP). Online, the box
//! index is computed with n floor operations and only that box's short candidate list is
//! searched. The grid resolution trades storage against query time.

use super::geometry::PartitionGeometry;
use super::{PointLocator, LOCATE_TOL};
use crate::linalg::{dot, Mat};
use crate::utils::chebyshev_ball;
use rayon::prelude::*;

pub struct HashTable {
    dim: usize,
    lo: Vec<f64>,
    inv_h: Vec<f64>,
    cells_per_dim: Vec<usize>,
    strides: Vec<usize>,
    cell_start: Vec<u32>,
    cell_items: Vec<u32>,
    rows: Vec<f64>,
    offsets: Vec<usize>,
}

impl HashTable {
    /// `target_cells`: approximate total number of grid boxes (split evenly across dimensions).
    pub fn new(geom: &PartitionGeometry, target_cells: usize) -> Self {
        let n = geom.dim;
        let per = ((target_cells.max(1) as f64).powf(1.0 / n as f64).round() as usize).max(1);
        Self::with_grid(geom, &vec![per; n])
    }

    pub fn with_grid(geom: &PartitionGeometry, cells_per_dim: &[usize]) -> Self {
        let n = geom.dim;
        let lo = geom.global_lo.clone();
        let hi = geom.global_hi.clone();
        let h: Vec<f64> = (0..n).map(|k| ((hi[k] - lo[k]) / cells_per_dim[k] as f64).max(1e-12)).collect();
        let inv_h: Vec<f64> = h.iter().map(|v| 1.0 / v).collect();
        let mut strides = vec![1usize; n];
        for k in 1..n {
            strides[k] = strides[k - 1] * cells_per_dim[k - 1];
        }
        let total: usize = cells_per_dim.iter().product();
        // for each region, the cells its bounding box touches, kept if the LP says the region meets the cell
        let lists: Vec<Vec<(usize, u32)>> = (0..geom.num_regions)
            .into_par_iter()
            .map(|i| {
                let mut out = vec![];
                let cl: Vec<usize> = (0..n).map(|k| (((geom.bbox_lo[i][k] - lo[k]) * inv_h[k]).floor().max(0.0) as usize).min(cells_per_dim[k] - 1)).collect();
                let ch: Vec<usize> = (0..n).map(|k| (((geom.bbox_hi[i][k] - lo[k]) * inv_h[k]).floor().max(0.0) as usize).min(cells_per_dim[k] - 1)).collect();
                let (ea, fa) = geom.region_mat(i);
                let mut idx = cl.clone();
                loop {
                    // exact test: region ∩ box nonempty (only needed when the region does not cover the box trivially)
                    let single = (0..n).all(|k| cl[k] == ch[k]);
                    let keep = single || {
                        let mut a = ea.clone();
                        let mut b = fa.clone();
                        for k in 0..n {
                            let mut e = vec![0.0; n];
                            e[k] = 1.0;
                            a.push_row(&e);
                            b.push(lo[k] + (idx[k] + 1) as f64 * h[k]);
                            e[k] = -1.0;
                            a.push_row(&e);
                            b.push(-(lo[k] + idx[k] as f64 * h[k]));
                        }
                        let _ = Mat::zeros(0, 0);
                        chebyshev_ball(&a, &b, &[]).map(|s| s.x[n] > 1e-12).unwrap_or(false)
                    };
                    if keep {
                        let cell: usize = (0..n).map(|k| idx[k] * strides[k]).sum();
                        out.push((cell, i as u32));
                    }
                    // next index in the box [cl, ch]
                    let mut k = 0;
                    loop {
                        if k == n {
                            return out;
                        }
                        if idx[k] < ch[k] {
                            idx[k] += 1;
                            break;
                        }
                        idx[k] = cl[k];
                        k += 1;
                    }
                }
            })
            .collect();
        let mut counts = vec![0u32; total + 1];
        for l in &lists {
            for &(c, _) in l {
                counts[c + 1] += 1;
            }
        }
        for c in 0..total {
            counts[c + 1] += counts[c];
        }
        let mut fill = counts.clone();
        let mut items = vec![0u32; *counts.last().unwrap() as usize];
        // candidates ordered by region size (larger first) so the likely region is tested first
        let mut order: Vec<usize> = (0..geom.num_regions).collect();
        order.sort_by(|&a, &b| geom.radii[b].partial_cmp(&geom.radii[a]).unwrap());
        let mut rank = vec![0usize; geom.num_regions];
        for (k, &i) in order.iter().enumerate() {
            rank[i] = k;
        }
        let mut all: Vec<(usize, u32)> = lists.into_iter().flatten().collect();
        all.sort_by_key(|&(c, i)| (c, rank[i as usize]));
        for (c, i) in all {
            items[fill[c] as usize] = i;
            fill[c] += 1;
        }
        let mut rows = vec![];
        for i in 0..geom.num_regions {
            let (e, f) = geom.region_rows(i);
            for k in 0..f.len() {
                rows.extend_from_slice(&e[k * n..(k + 1) * n]);
                rows.push(f[k]);
            }
        }
        HashTable { dim: n, lo, inv_h, cells_per_dim: cells_per_dim.to_vec(), strides, cell_start: counts, cell_items: items, rows, offsets: geom.offsets.clone() }
    }

    #[inline]
    fn region_contains(&self, j: usize, theta: &[f64]) -> bool {
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

    pub fn num_cells(&self) -> usize {
        self.cells_per_dim.iter().product()
    }

    pub fn mean_candidates(&self) -> f64 {
        self.cell_items.len() as f64 / self.num_cells() as f64
    }

    pub fn max_candidates(&self) -> usize {
        self.cell_start.windows(2).map(|w| (w[1] - w[0]) as usize).max().unwrap_or(0)
    }
}

impl PointLocator for HashTable {
    fn name(&self) -> &'static str {
        "hash table"
    }

    fn locate(&self, theta: &[f64]) -> Option<usize> {
        let mut cell = 0usize;
        for k in 0..self.dim {
            let u = (theta[k] - self.lo[k]) * self.inv_h[k];
            if !(u >= -1e-9) {
                return None;
            }
            let c = (u as usize).min(self.cells_per_dim[k] - 1);
            if u > self.cells_per_dim[k] as f64 + 1e-9 {
                return None;
            }
            cell += c * self.strides[k];
        }
        let s = self.cell_start[cell] as usize;
        let e = self.cell_start[cell + 1] as usize;
        self.cell_items[s..e].iter().map(|&j| j as usize).find(|&j| self.region_contains(j, theta))
    }

    fn storage_floats(&self) -> usize {
        self.cell_start.len() + self.cell_items.len() + self.rows.len() + 3 * self.dim
    }
}
