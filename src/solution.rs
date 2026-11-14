//! Port of `ppopt.solution.Solution`.

use crate::critical_region::CriticalRegion;
use crate::linalg::allclose;
use crate::program::MpProgram;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Objective evaluator used for overlapping (mixed-integer) solutions.
pub trait Objective: Send + Sync {
    fn evaluate_objective(&self, x: &[f64], theta: &[f64]) -> f64;
    fn num_t(&self) -> usize;
}

impl Objective for MpProgram {
    fn evaluate_objective(&self, x: &[f64], theta: &[f64]) -> f64 {
        MpProgram::evaluate_objective(self, x, theta)
    }
    fn num_t(&self) -> usize {
        MpProgram::num_t(self)
    }
}

#[derive(Clone)]
pub struct Solution {
    pub program: Arc<dyn Objective>,
    /// The continuous program (None for mixed-integer solutions, whose program is held by `program`).
    pub continuous: Option<Arc<MpProgram>>,
    pub critical_regions: Vec<CriticalRegion>,
    pub is_overlapping: bool,
    pub point_location_tolerance: f64,
}

#[derive(Serialize, Deserialize)]
pub struct SolutionData {
    pub critical_regions: Vec<CriticalRegion>,
    pub is_overlapping: bool,
    pub point_location_tolerance: f64,
}

/// Default point location tolerance, shared by `Solution::get_region` and every point locator
/// (`point_location::LOCATE_TOL`). PPOPT uses 1e-5; see the README for why this crate uses 1e-9.
pub const DEFAULT_POINT_LOCATION_TOL: f64 = 1e-9;

impl Solution {
    pub fn new(program: Arc<MpProgram>, critical_regions: Vec<CriticalRegion>) -> Self {
        Solution {
            program: program.clone(),
            continuous: Some(program),
            critical_regions,
            is_overlapping: false,
            point_location_tolerance: DEFAULT_POINT_LOCATION_TOL,
        }
    }

    pub fn add_region(&mut self, r: CriticalRegion) {
        self.critical_regions.push(r);
    }

    pub fn len(&self) -> usize {
        self.critical_regions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.critical_regions.is_empty()
    }

    pub fn theta_dim(&self) -> usize {
        self.program.num_t()
    }

    pub fn get_region_index(&self, theta: &[f64]) -> Option<usize> {
        if self.is_overlapping { self.get_region_overlap(theta) } else { self.get_region_no_overlap(theta) }
    }

    pub fn get_region(&self, theta: &[f64]) -> Option<&CriticalRegion> {
        self.get_region_index(theta).map(|i| &self.critical_regions[i])
    }

    /// The first region that contains θ strictly (tolerance 0). If none does, the first region
    /// that contains θ within the point location tolerance. Taking a region that holds θ only
    /// within the tolerance when its neighbour holds it strictly would extrapolate that region's
    /// law past its own boundary (an infeasible x near a facet). Interior points return at the
    /// first strict hit, as before.
    pub fn get_region_no_overlap(&self, theta: &[f64]) -> Option<usize> {
        let tol = self.point_location_tolerance;
        let mut near = None;
        for (i, r) in self.critical_regions.iter().enumerate() {
            let mut viol = f64::NEG_INFINITY;
            for k in 0..r.e.rows {
                viol = viol.max(crate::linalg::dot(r.e.row(k), theta) - r.f[k]);
                if viol >= tol {
                    break;
                }
            }
            if viol <= 0.0 {
                return Some(i);
            }
            if viol < tol && near.is_none() {
                near = Some(i);
            }
        }
        near
    }

    /// Among the regions containing θ, the one with the lowest objective. Regions that contain θ
    /// strictly (tolerance 0) take precedence: a region that holds θ only within the tolerance
    /// extrapolates its law slightly outside its own cell and can look better than the optimum.
    /// The tolerance applies only when no region contains θ strictly.
    pub fn get_region_overlap(&self, theta: &[f64]) -> Option<usize> {
        let mut best = [f64::INFINITY; 2];
        let mut best_i: [Option<usize>; 2] = [None, None];
        for (i, r) in self.critical_regions.iter().enumerate() {
            let mut viol = f64::NEG_INFINITY;
            for k in 0..r.e.rows {
                viol = viol.max(crate::linalg::dot(r.e.row(k), theta) - r.f[k]);
                if viol >= self.point_location_tolerance {
                    break;
                }
            }
            if viol >= self.point_location_tolerance {
                continue;
            }
            let class = if viol <= 0.0 { 0 } else { 1 };
            let x = r.evaluate(theta);
            let obj = self.program.evaluate_objective(&x, theta);
            if obj <= best[class] {
                best[class] = obj;
                best_i[class] = Some(i);
            }
        }
        best_i[0].or(best_i[1])
    }

    pub fn evaluate(&self, theta: &[f64]) -> Option<Vec<f64>> {
        self.get_region(theta).map(|r| r.evaluate(theta))
    }

    pub fn evaluate_objective(&self, theta: &[f64]) -> Option<f64> {
        let x = self.evaluate(theta)?;
        Some(self.program.evaluate_objective(&x, theta))
    }

    /// Re-solves the program at each region's Chebyshev center and compares x*, λ* and the active
    /// set (the intent of PPOPT's `verify_solution`).
    pub fn verify_solution(&self) -> bool {
        let Some(prog) = &self.continuous else { return false };
        for r in &self.critical_regions {
            let Some((theta, _)) = r.chebyshev() else { return false };
            let Some(sol) = prog.solve_theta(&theta) else { return false };
            let x = r.evaluate(&theta);
            if !allclose(&sol.sol, &x, 1e-5, 1e-8) {
                return false;
            }
            let lam = r.lagrange_multipliers(&theta);
            let duals: Vec<f64> = r.active_set.iter().map(|&i| sol.dual[i]).collect();
            if !allclose(&duals, &lam, 1e-5, 1e-6) {
                return false;
            }
        }
        true
    }

    /// Checks one θ against a direct solve.
    pub fn verify_theta(&self, theta: &[f64]) -> bool {
        let Some(prog) = &self.continuous else { return false };
        let region = self.get_region(theta);
        let sol = prog.solve_theta(theta);
        match (region, sol) {
            (None, None) => true,
            (Some(r), Some(s)) => allclose(&s.sol, &r.evaluate(theta), 1e-5, 1e-6),
            _ => false,
        }
    }

    pub fn to_data(&self) -> SolutionData {
        SolutionData {
            critical_regions: self.critical_regions.clone(),
            is_overlapping: self.is_overlapping,
            point_location_tolerance: self.point_location_tolerance,
        }
    }
}
