//! Descriptor function (Baotić, Borrelli, Bemporad & Morari, 2008, "Efficient on-line computation
//! of constrained optimal control", SIAM J. Control Optim. 47(5)).
//!
//! A continuous PWA scalar f(θ) = f_i(θ) on R_i whose pieces differ between neighbours is built
//! from the optimiser, f_i(θ) = w'(A_i θ + b_i) with a random w. Because f is continuous, f_i - f_j
//! vanishes on the facet shared by i and j, so its sign on R_i is a constant s_ij and
//!
//! ```text
//! θ ∈ R_i  <=>  sign(f_i(θ) - f_j(θ)) = s_ij for all neighbours j  (and domain facets hold)
//! ```
//!
//! Online, all R values f_j(θ) are computed once (R dot products of length n + 1), then each region
//! is tested with scalar comparisons instead of facet dot products. Facets on the domain boundary,
//! or between neighbours whose pieces coincide or where f is not continuous, keep their explicit
//! row test.

use super::geometry::PartitionGeometry;
use super::{PointLocator, LOCATE_TOL};
use crate::linalg::dot;
use crate::solution::Solution;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[derive(Clone, Copy)]
enum Test {
    /// sign(f_i - f_j) must be `sign` (+1 / -1)
    Compare { j: u32, sign: f64 },
    /// explicit row k (index into `rows`)
    Row(u32),
}

pub struct DescriptorFunction {
    dim: usize,
    /// R x (n+1) descriptor pieces [grad | offset]
    pieces: Vec<f64>,
    tests: Vec<Test>,
    test_offsets: Vec<usize>,
    rows: Vec<f64>,
    /// tolerance on f_i - f_j comparisons (relative to the piece scale)
    tol: Vec<f64>,
    pub explicit_rows: usize,
    pub compared_facets: usize,
}

impl DescriptorFunction {
    pub fn new(sol: &Solution, geom: &PartitionGeometry, seed: u64) -> Self {
        let n = geom.dim;
        let w = n + 1;
        let r = geom.num_regions;
        let nx = sol.critical_regions.first().map(|c| c.a.rows).unwrap_or(0);
        let mut rng = StdRng::seed_from_u64(seed);
        let wv: Vec<f64> = (0..nx).map(|_| crate::program::standard_normal(&mut rng)).collect();
        let mut pieces = vec![0.0; r * w];
        for (i, cr) in sol.critical_regions.iter().enumerate() {
            // f_i(θ) = w'(Aθ + b) = (A'w)'θ + w'b
            let grad = cr.a.vecmat(&wv);
            pieces[i * w..i * w + n].copy_from_slice(&grad);
            pieces[i * w + n] = dot(&wv, &cr.b);
        }
        let fval = |i: usize, th: &[f64]| dot(&pieces[i * w..i * w + n], th) + pieces[i * w + n];
        let mut tests = vec![];
        let mut test_offsets = vec![0];
        let mut rows = vec![];
        let mut explicit_rows = 0;
        let mut compared = 0;
        let mut tol = vec![0.0; r];
        for i in 0..r {
            let (e, f) = geom.region_rows(i);
            let c = &geom.centers[i];
            let scale = pieces[i * w..(i + 1) * w].iter().fold(1e-12f64, |m, v| m.max(v.abs()));
            tol[i] = 1e-9 * scale.max(1.0);
            for fc in &geom.facets[i] {
                let a = &e[fc.row * n..(fc.row + 1) * n];
                let b = f[fc.row];
                let mut chosen = None;
                for &j in &fc.neighbors {
                    let gi = fval(i, c) - fval(j, c);
                    // continuity: at a point on the shared facet both pieces must agree
                    let slack = b - dot(a, c);
                    let on_facet: Vec<f64> = c.iter().zip(a).map(|(ci, ai)| ci + ai * slack).collect();
                    let jump = (fval(i, &on_facet) - fval(j, &on_facet)).abs();
                    // f_i - f_j = κ (b - a'θ): κ from the center
                    let kappa = if slack > 0.0 { gi / slack } else { 0.0 };
                    if kappa.abs() > 1e-6 * scale.max(1.0) && jump <= 1e-6 * scale.max(1.0) * (1.0 + slack) {
                        chosen = Some((j, gi.signum()));
                        break;
                    }
                }
                match chosen {
                    Some((j, s)) => {
                        tests.push(Test::Compare { j: j as u32, sign: s });
                        compared += 1;
                    }
                    None => {
                        let k = rows.len() / w;
                        rows.extend_from_slice(a);
                        rows.push(b);
                        tests.push(Test::Row(k as u32));
                        explicit_rows += 1;
                    }
                }
            }
            test_offsets.push(tests.len());
        }
        DescriptorFunction { dim: n, pieces, tests, test_offsets, rows, tol, explicit_rows, compared_facets: compared }
    }
}

impl PointLocator for DescriptorFunction {
    fn name(&self) -> &'static str {
        "descriptor function"
    }

    fn locate(&self, theta: &[f64]) -> Option<usize> {
        let n = self.dim;
        let w = n + 1;
        let r = self.test_offsets.len() - 1;
        // all descriptor values once
        let mut fv: Vec<f64> = Vec::with_capacity(r);
        for p in self.pieces.chunks_exact(w) {
            fv.push(dot(&p[..n], theta) + p[n]);
        }
        'region: for i in 0..r {
            let fi = fv[i];
            for t in &self.tests[self.test_offsets[i]..self.test_offsets[i + 1]] {
                match *t {
                    Test::Compare { j, sign } => {
                        if sign * (fi - fv[j as usize]) < -self.tol[i] {
                            continue 'region;
                        }
                    }
                    Test::Row(k) => {
                        let row = &self.rows[k as usize * w..(k as usize + 1) * w];
                        if dot(&row[..n], theta) > row[n] + LOCATE_TOL {
                            continue 'region;
                        }
                    }
                }
            }
            return Some(i);
        }
        None
    }

    fn storage_floats(&self) -> usize {
        self.pieces.len() + self.tests.len() * 2 + self.rows.len()
    }
}
