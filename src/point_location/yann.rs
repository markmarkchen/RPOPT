//! Y-wise affine neural network (Braniff & Tian, 2026, "YANNs: Y-wise affine neural networks for
//! exact and efficient representations of piecewise linear functions", Computers & Chemical
//! Engineering; arXiv:2505.07054).
//!
//! The network is written down from the explicit solution, no training:
//!
//! * layer 1 (binary step): one node per region inequality, H(b - a'θ)
//! * layer 2 (ReLU): one node per region, max(Σ_rows - q_s + 1, 0) = 1 iff all its rows hold
//! * layer 3 (ReLU, lower-triangular weights): keeps the first active indicator (one-hot)
//! * layer 4 (ReLU, big-M): 2m nodes per region, M·1_k + (A_k θ + b_k)_r - M and M·1_k - (...)_r - M
//! * layer 5 (linear): sums the ReLU pairs, giving u = A_k θ + b_k of the selected region
//!
//! `locate` runs layers 1-3 (the region selector), `evaluate` runs all five. Everything is computed
//! in f64 (the paper reports FP32 errors that grow with M).

use super::{PointLocator, LOCATE_TOL};
use crate::linalg::dot;
use crate::solution::Solution;

pub struct Yann {
    dim: usize,
    out_dim: usize,
    /// layer 1: all region rows stacked, [a | b]
    l1: Vec<f64>,
    /// layer 2: rows per region (q_s), contiguous so the "weights of ones" are a segment sum
    l2_offsets: Vec<usize>,
    /// layer 4: per region the law [A_k | b_k] (m x (n+1))
    laws: Vec<f64>,
    pub big_m: f64,
}

impl Yann {
    pub fn new(sol: &Solution) -> Self {
        let n = sol.theta_dim();
        let out_dim = sol.critical_regions.first().map(|c| c.a.rows).unwrap_or(0);
        let mut l1 = vec![];
        let mut offs = vec![0];
        let mut laws = vec![];
        let mut lo = vec![f64::INFINITY; n];
        let mut hi = vec![f64::NEG_INFINITY; n];
        for cr in &sol.critical_regions {
            for i in 0..cr.e.rows {
                l1.extend_from_slice(cr.e.row(i));
                l1.push(cr.f[i]);
            }
            offs.push(offs.last().unwrap() + cr.e.rows);
            for r in 0..out_dim {
                laws.extend_from_slice(cr.a.row(r));
                laws.push(cr.b[r]);
            }
            if let Some(v) = cr.chebyshev() {
                for k in 0..n {
                    lo[k] = lo[k].min(v.0[k] - v.1);
                    hi[k] = hi[k].max(v.0[k] + v.1);
                }
            }
        }
        // M >= |f_k(θ)| over the whole domain: bound each law over the bounding box of the
        // parameter constraints (falls back to the regions' extent)
        let (blo, bhi) = parameter_box(sol).unwrap_or((lo, hi));
        let mut big_m: f64 = 1.0;
        for law in laws.chunks_exact(n + 1) {
            let mut s = law[n].abs();
            for k in 0..n {
                s += law[k].abs() * blo[k].abs().max(bhi[k].abs());
            }
            big_m = big_m.max(s);
        }
        Yann { dim: n, out_dim, l1, l2_offsets: offs, laws, big_m: 2.0 * big_m }
    }

    pub fn num_regions(&self) -> usize {
        self.l2_offsets.len() - 1
    }

    /// Layers 1-3: one-hot region selector (index of the hot neuron).
    #[inline]
    fn select(&self, theta: &[f64]) -> Option<usize> {
        let n = self.dim;
        let w = n + 1;
        let q = self.l1.len() / w;
        // layer 1: binary step on every inequality
        let mut y1 = Vec::with_capacity(q);
        for row in self.l1.chunks_exact(w) {
            y1.push(if row[n] - dot(&row[..n], theta) >= -LOCATE_TOL { 1.0f64 } else { 0.0 });
        }
        // layer 2: ReLU(Σ y1 over region rows - q_s + 1)
        // layer 3: ReLU(d_i - Σ_{l<i} d_l), computed as a running prefix sum (same weights as the
        // lower-triangular matrix of the paper)
        let mut prefix = 0.0;
        let mut hot = None;
        for i in 0..self.num_regions() {
            let (s, e) = (self.l2_offsets[i], self.l2_offsets[i + 1]);
            let sum: f64 = y1[s..e].iter().sum();
            let d = (sum - (e - s) as f64 + 1.0).max(0.0);
            let y3 = (d - prefix).max(0.0);
            prefix += d;
            if y3 > 0.5 && hot.is_none() {
                hot = Some(i);
            }
        }
        hot
    }

    /// All five layers: u = A_k θ + b_k for the selected region (zeros outside the domain).
    pub fn evaluate(&self, theta: &[f64]) -> Vec<f64> {
        let n = self.dim;
        let w = n + 1;
        let r = self.num_regions();
        let hot = self.select(theta);
        let mut u = vec![0.0; self.out_dim];
        let m = self.big_m;
        for k in 0..r {
            let ind = if Some(k) == hot { 1.0 } else { 0.0 };
            for o in 0..self.out_dim {
                let law = &self.laws[(k * self.out_dim + o) * w..(k * self.out_dim + o + 1) * w];
                let v = dot(&law[..n], theta) + law[n];
                let p1 = (m * ind + v - m).max(0.0);
                let p2 = (m * ind - v - m).max(0.0);
                u[o] += p1 - p2;
            }
        }
        u
    }
}

fn parameter_box(sol: &Solution) -> Option<(Vec<f64>, Vec<f64>)> {
    let prog = sol.continuous.as_ref()?;
    let n = prog.num_t();
    let mut lo = vec![0.0; n];
    let mut hi = vec![0.0; n];
    for k in 0..n {
        let mut c = vec![0.0; n];
        c[k] = 1.0;
        lo[k] = crate::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[])?.obj;
        c[k] = -1.0;
        hi[k] = -crate::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[])?.obj;
    }
    Some((lo, hi))
}

impl PointLocator for Yann {
    fn name(&self) -> &'static str {
        "YANN"
    }

    fn locate(&self, theta: &[f64]) -> Option<usize> {
        self.select(theta)
    }

    fn storage_floats(&self) -> usize {
        self.l1.len() + self.laws.len() + self.l2_offsets.len()
    }
}
