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

impl Solution {
    pub fn new(program: Arc<MpProgram>, critical_regions: Vec<CriticalRegion>) -> Self {
        Solution {
            program: program.clone(),
            continuous: Some(program),
            critical_regions,
            is_overlapping: false,
            point_location_tolerance: 1e-5,
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

    pub fn get_region_no_overlap(&self, theta: &[f64]) -> Option<usize> {
        self.critical_regions.iter().position(|r| r.is_inside(theta, self.point_location_tolerance))
    }

    /// Among all regions containing θ, the one with the lowest objective.
    pub fn get_region_overlap(&self, theta: &[f64]) -> Option<usize> {
        let mut best = f64::INFINITY;
        let mut best_i = None;
        for (i, r) in self.critical_regions.iter().enumerate() {
            if r.is_inside(theta, self.point_location_tolerance) {
                let x = r.evaluate(theta);
                let obj = self.program.evaluate_objective(&x, theta);
                if obj <= best {
                    best = obj;
                    best_i = Some(i);
                }
            }
        }
        best_i
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
