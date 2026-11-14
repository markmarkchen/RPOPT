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
use crate::linalg::Mat;
use crate::utils::chebyshev_ball;
use rayon::prelude::*;

pub struct HashTable {
    dim: usize,
    lo: Vec<f64>,
    inv_h: Vec<f64>,
    cells_per_dim: Vec<usize>,
    strides: Vec<usize>,
    /// per cell: 0 = no region, 1 + j = the cell lies entirely inside region j, otherwise
    /// `mixed_base + w` where w is the word offset of the cell's record in `stream`
    info: Vec<u32>,
    mixed_base: u32,
    /// record of a cell that needs row tests: [ncand, then per candidate (region << 32 | nrows)
    /// followed by nrows packed rows [a | b] (f64 bits)]. Only rows whose hyperplane can cut the
    /// cell are kept. Contiguous, so one cache line chain per query.
    stream: Vec<u64>,
    mixed: usize,
    total_cands: usize,
    total_rows: usize,
    max_cands: usize,
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
        // Per cell and candidate keep only the rows that can reject a point of the cell: a row
        // whose halfspace contains the whole (slightly enlarged) cell with a margin can never
        // fail the online test, so dropping it does not change any answer. A candidate left with
        // no rows contains the whole cell and is returned at once; later candidates are unreachable.
        let mixed_base = geom.num_regions as u32 + 1;
        let mut info = vec![0u32; total + 1];
        let mut stream: Vec<u64> = vec![];
        let (mut mixed, mut total_cands, mut total_rows, mut max_cands) = (0usize, 0usize, 0usize, 0usize);
        let w = n + 1;
        for c in 0..total {
            let mut corner_lo = vec![0.0; n];
            let mut corner_hi = vec![0.0; n];
            for k in 0..n {
                let ik = (c / strides[k]) % cells_per_dim[k];
                corner_lo[k] = lo[k] + ik as f64 * h[k] - 1e-6 * h[k];
                corner_hi[k] = lo[k] + (ik + 1) as f64 * h[k] + 1e-6 * h[k];
            }
            let mut cell_cands: Vec<(u32, Vec<f64>)> = vec![];
            for &j in &items[counts[c] as usize..counts[c + 1] as usize] {
                let (e, f) = geom.region_rows(j as usize);
                let mut rws: Vec<f64> = vec![];
                for r in 0..f.len() {
                    let a = &e[r * n..(r + 1) * n];
                    let mut maxdot = 0.0;
                    for k in 0..n {
                        maxdot += (a[k] * corner_lo[k]).max(a[k] * corner_hi[k]);
                    }
                    if maxdot > f[r] - 1e-8 {
                        rws.extend_from_slice(a);
                        rws.push(f[r]);
                    }
                }
                let empty = rws.is_empty();
                cell_cands.push((j, rws));
                if empty {
                    break;
                }
            }
            // order candidates by how many sample points of the cell they contain (the likely
            // region first). Only done when no candidate covers the cell entirely.
            if cell_cands.len() > 1 && cell_cands.iter().all(|(_, r)| !r.is_empty()) {
                let samples = 24usize;
                let mut state = 0x9E3779B97F4A7C15u64 ^ c as u64;
                let mut pts: Vec<Vec<f64>> = vec![];
                for _ in 0..samples {
                    let mut p = vec![0.0; n];
                    for k in 0..n {
                        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                        let t = (state >> 11) as f64 / (1u64 << 53) as f64;
                        p[k] = corner_lo[k] + t * (corner_hi[k] - corner_lo[k]);
                    }
                    pts.push(p);
                }
                let hits: Vec<usize> = cell_cands
                    .iter()
                    .map(|(j, _)| {
                        let (e, f) = geom.region_rows(*j as usize);
                        pts.iter().filter(|p| (0..f.len()).all(|r| (0..n).map(|k| e[r * n + k] * p[k]).sum::<f64>() <= f[r] + LOCATE_TOL)).count()
                    })
                    .collect();
                let mut idx: Vec<usize> = (0..cell_cands.len()).collect();
                idx.sort_by_key(|&i| std::cmp::Reverse(hits[i]));
                cell_cands = idx.into_iter().map(|i| cell_cands[i].clone()).collect();
            }
            if cell_cands.is_empty() {
                info[c] = 0;
            } else if cell_cands.len() == 1 && cell_cands[0].1.is_empty() {
                info[c] = cell_cands[0].0 + 1;
            } else {
                info[c] = mixed_base + stream.len() as u32;
                mixed += 1;
                max_cands = max_cands.max(cell_cands.len());
                total_cands += cell_cands.len();
                stream.push(cell_cands.len() as u64);
                for (j, rws) in &cell_cands {
                    let nr = rws.len() / w;
                    total_rows += nr;
                    stream.push(((*j as u64) << 32) | nr as u64);
                    stream.extend(rws.iter().map(|v| v.to_bits()));
                }
            }
        }
        assert!(stream.len() < (u32::MAX - mixed_base) as usize);
        HashTable { dim: n, lo, inv_h, cells_per_dim: cells_per_dim.to_vec(), strides, info, mixed_base, stream, mixed, total_cands, total_rows, max_cands }
    }

    #[inline(always)]
    fn locate_g(&self, theta: &[f64], n: usize) -> Option<usize> {
        let mut cell = 0usize;
        let mut inside = true;
        for k in 0..n {
            let u = (theta[k] - self.lo[k]) * self.inv_h[k];
            let m = self.cells_per_dim[k];
            // NaN fails both comparisons and counts as outside
            inside &= (u >= -1e-9) & (u <= m as f64 + 1e-9);
            cell += (u as usize).min(m - 1) * self.strides[k];
        }
        // outside the grid: the extra last entry of `info` means "no region"
        let cell = if inside { cell } else { self.info.len() - 1 };
        let inf = self.info[cell];
        if inf < self.mixed_base {
            return (inf as usize).checked_sub(1);
        }
        let mut p = (inf - self.mixed_base) as usize;
        let nc = self.stream[p] as usize;
        p += 1;
        for _ in 0..nc {
            let head = self.stream[p];
            let nr = (head & 0xffff_ffff) as usize;
            let mut ok = true;
            let rows = &self.stream[p + 1..p + 1 + nr * (n + 1)];
            for row in rows.chunks_exact(n + 1) {
                let mut d = 0.0;
                for k in 0..n {
                    d += f64::from_bits(row[k]) * theta[k];
                }
                if d > f64::from_bits(row[n]) + LOCATE_TOL {
                    ok = false;
                    break;
                }
            }
            if ok {
                return Some((head >> 32) as usize);
            }
            p += 1 + nr * (n + 1);
        }
        None
    }

    pub fn mixed_cells(&self) -> usize {
        self.mixed
    }

    pub fn mean_rows_per_mixed_cell(&self) -> f64 {
        self.total_rows as f64 / self.mixed.max(1) as f64
    }

    pub fn num_cells(&self) -> usize {
        self.cells_per_dim.iter().product()
    }

    pub fn mean_candidates(&self) -> f64 {
        self.total_cands as f64 / self.num_cells() as f64
    }

    pub fn max_candidates(&self) -> usize {
        self.max_cands
    }
}

impl PointLocator for HashTable {
    fn name(&self) -> &'static str {
        "hash table"
    }

    #[inline]
    fn locate(&self, theta: &[f64]) -> Option<usize> {
        match self.dim {
            2 => self.locate_g(theta, 2),
            3 => self.locate_g(theta, 3),
            4 => self.locate_g(theta, 4),
            n => self.locate_g(theta, n),
        }
    }

    fn storage_floats(&self) -> usize {
        self.info.len() + self.stream.len() + 3 * self.dim
    }
}
