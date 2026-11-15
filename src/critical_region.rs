//! Port of `ppopt.critical_region`.

use crate::linalg::{dot, Mat};
use crate::utils::chebyshev_ball;
use serde::{Deserialize, Serialize};

/// A critical region: x(θ) = Aθ + b, λ(θ) = Cθ + d on Θ = {θ : Eθ <= f}.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CriticalRegion {
    pub a: Mat,
    pub b: Vec<f64>,
    pub c: Mat,
    pub d: Vec<f64>,
    pub e: Mat,
    pub f: Vec<f64>,
    pub active_set: Vec<usize>,
    /// indices (into A_t) of parametric constraints that bound the region
    pub omega_set: Vec<usize>,
    /// active constraints whose λ = 0 facet bounds the region
    pub lambda_set: Vec<usize>,
    /// (indices into the inactive list, original constraint indices) of facets where an inactive
    /// constraint becomes active
    pub regular_set: (Vec<usize>, Vec<usize>),
    pub y_fixation: Option<Vec<f64>>,
    pub y_indices: Option<Vec<usize>>,
    pub x_indices: Option<Vec<usize>>,
    /// Chebyshev radius computed during construction (same polytope), reused by
    /// `is_full_dimension` instead of a second LP.
    #[serde(skip)]
    pub cheb_radius: Option<f64>,
}

impl CriticalRegion {
    /// x*(θ) = Aθ + b, with fixed binaries spliced in for mixed-integer solutions.
    pub fn evaluate(&self, theta: &[f64]) -> Vec<f64> {
        let mut cont = self.a.matvec(theta);
        for (v, bi) in cont.iter_mut().zip(&self.b) {
            *v += bi;
        }
        match (&self.y_fixation, &self.y_indices, &self.x_indices) {
            (Some(y), Some(yi), Some(xi)) => {
                let mut out = vec![0.0; xi.len() + yi.len()];
                for (k, &i) in xi.iter().enumerate() {
                    out[i] = cont[k];
                }
                for (k, &i) in yi.iter().enumerate() {
                    out[i] = y[k];
                }
                out
            }
            _ => cont,
        }
    }

    pub fn lagrange_multipliers(&self, theta: &[f64]) -> Vec<f64> {
        let mut l = self.c.matvec(theta);
        for (v, di) in l.iter_mut().zip(&self.d) {
            *v += di;
        }
        l
    }

    /// Eθ - f < tol for every row.
    #[inline]
    pub fn is_inside(&self, theta: &[f64], tol: f64) -> bool {
        (0..self.e.rows).all(|i| dot(self.e.row(i), theta) - self.f[i] < tol)
    }

    pub fn is_full_dimension(&self) -> bool {
        crate::census_purpose!(Chebyshev);
        crate::stats::lp(1);
        if let Some(r) = self.cheb_radius {
            return r > 1e-8;
        }
        match chebyshev_ball(&self.e, &self.f, &[]) {
            Some(s) => s.x[s.x.len() - 1] > 1e-8,
            None => false,
        }
    }

    pub fn chebyshev(&self) -> Option<(Vec<f64>, f64)> {
        let s = chebyshev_ball(&self.e, &self.f, &[])?;
        let n = self.e.cols;
        Some((s.x[..n].to_vec(), s.x[n]))
    }

    pub fn get_constraints(&self) -> (&Mat, &[f64]) {
        (&self.e, &self.f)
    }

    pub fn num_facets(&self) -> usize {
        self.e.rows
    }
}
