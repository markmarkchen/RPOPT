//! Port of `ppopt.mpmilp_program`, `ppopt.mpmiqp_program`, `mp_solvers.mitree` and
//! `mp_solvers.mpmiqp_enumeration`.

use super::{solve_mpqp_with, MpqpAlgorithm, SolveOptions};
use crate::linalg::{dot, Mat};
use crate::program::MpProgram;
use crate::solution::{Objective, Solution};
use crate::solver::SolverOutput;
use crate::utils::detect_implicit_equalities;
use rayon::prelude::*;
use std::sync::Arc;

/// Mixed-integer mpLP (q = None) or mpQP with binary variables at `binary_indices`.
#[derive(Clone, Debug)]
pub struct MpmiProgram {
    pub base: MpProgram,
    pub binary_indices: Vec<usize>,
    pub cont_indices: Vec<usize>,
}

impl Objective for MpmiProgram {
    fn evaluate_objective(&self, x: &[f64], theta: &[f64]) -> f64 {
        self.base.evaluate_objective(x, theta)
    }
    fn num_t(&self) -> usize {
        self.base.num_t()
    }
}

impl MpmiProgram {
    #[allow(clippy::too_many_arguments)]
    pub fn new(a: Mat, b: Vec<f64>, c: Vec<f64>, h: Mat, q: Option<Mat>, a_t: Mat, b_t: Vec<f64>, f: Mat, binary_indices: Vec<usize>, equality_indices: Vec<usize>) -> Self {
        let base = MpProgram::new(a, b, c, h, q, a_t, b_t, f, equality_indices, false);
        let nx = base.num_x();
        let cont_indices = (0..nx).filter(|i| !binary_indices.contains(i)).collect();
        let mut p = MpmiProgram { base, binary_indices, cont_indices };
        p.process_constraints();
        p
    }

    /// Like [`MpmiProgram::new`] but without the MILP-based constraint processing, which treats
    /// every integer variable as binary and would drop rows that bound general integers.
    #[allow(clippy::too_many_arguments)]
    pub fn new_unprocessed(a: Mat, b: Vec<f64>, c: Vec<f64>, h: Mat, q: Option<Mat>, a_t: Mat, b_t: Vec<f64>, f: Mat, integer_indices: Vec<usize>, equality_indices: Vec<usize>) -> Self {
        let base = MpProgram::new(a, b, c, h, q, a_t, b_t, f, equality_indices, false);
        let nx = base.num_x();
        let cont_indices = (0..nx).filter(|i| !integer_indices.contains(i)).collect();
        MpmiProgram { base, binary_indices: integer_indices, cont_indices }
    }

    /// MILP-based removal of strongly redundant inequality rows (PPOPT MPMILP `process_constraints`).
    pub fn process_constraints(&mut self) {
        let b = &mut self.base;
        b.scale_constraints();
        // implicit equalities
        let negf = b.f.neg();
        let probe = Mat::hstack(&[&b.a, &negf]);
        let pairs = detect_implicit_equalities(&probe, &b.b);
        if !pairs.is_empty() {
            let mut keep: Vec<usize> = pairs.iter().map(|p| p.0).collect();
            keep.sort();
            keep.dedup();
            let mut remove: Vec<usize> = pairs.iter().map(|p| p.1).collect();
            remove.retain(|i| !keep.contains(i));
            let mut temp = b.equality_indices.clone();
            for k in keep {
                if !temp.contains(&k) {
                    temp.push(k);
                }
            }
            let m = b.num_constraints();
            let mut order = temp.clone();
            order.extend((0..m).filter(|i| !temp.contains(i) && !remove.contains(i)));
            b.a = b.a.select_rows(&order);
            b.f = b.f.select_rows(&order);
            b.b = order.iter().map(|&i| b.b[i]).collect();
            b.equality_indices = (0..temp.len()).collect();
        }
        let (pa, pb) = b.xtheta_polytope();
        let neq = b.num_equality_constraints();
        let mut saved: Vec<usize> = b.equality_indices.clone();
        for i in 0..b.num_inequality_constraints() {
            let mut eqs = b.equality_indices.clone();
            eqs.push(i + neq);
            if b.solver.solve_milp(None, &pa, &pb, &eqs, &self.binary_indices).is_some() {
                saved.push(i + neq);
            }
        }
        b.a = b.a.select_rows(&saved);
        b.f = b.f.select_rows(&saved);
        b.b = saved.iter().map(|&i| b.b[i]).collect();
        b.invalidate_caches();
    }

    pub fn num_x(&self) -> usize {
        self.base.num_x()
    }

    pub fn num_t(&self) -> usize {
        self.base.num_t()
    }

    /// Fixes the binaries and returns the continuous sub-problem (an mpLP or an mpQP).
    pub fn generate_substituted_problem(&self, fixed: &[f64]) -> MpProgram {
        let p = &self.base;
        let a_cont = p.a.select_cols(&self.cont_indices);
        let a_bin = p.a.select_cols(&self.binary_indices);
        let is_not_binary = |i: usize| !(a_cont.row(i).iter().all(|v| v.abs() <= 1e-8) && p.f.row(i).iter().all(|v| v.abs() <= 1e-8));
        let ineq: Vec<usize> = (0..p.num_constraints()).filter(|i| !p.equality_indices.contains(i)).collect();
        let kept_eq: Vec<usize> = p.equality_indices.iter().cloned().filter(|&i| is_not_binary(i)).collect();
        let kept_in: Vec<usize> = ineq.into_iter().filter(|&i| is_not_binary(i)).collect();
        let mut kept = kept_eq.clone();
        kept.extend(kept_in);
        let new_eq: Vec<usize> = (0..kept_eq.len()).collect();
        let a_c = a_cont.select_rows(&kept);
        let a_b = a_bin.select_rows(&kept);
        let ab_fix = a_b.matvec(fixed);
        let b: Vec<f64> = kept.iter().zip(&ab_fix).map(|(&i, v)| p.b[i] - v).collect();
        let f = p.f.select_rows(&kept);
        let c_bin: Vec<f64> = self.binary_indices.iter().map(|&i| p.c[i]).collect();
        let mut c: Vec<f64> = self.cont_indices.iter().map(|&i| p.c[i]).collect();
        let mut c_c = p.c_c + dot(&c_bin, fixed);
        let h_c = p.h.select_rows(&self.cont_indices);
        let h_d = p.h.select_rows(&self.binary_indices);
        let hd_fix = h_d.vecmat(fixed); // fixed' H_d
        let c_t: Vec<f64> = p.c_t.iter().zip(&hd_fix).map(|(a, b)| a + b).collect();
        let q = p.q.as_ref().map(|q| {
            let q_c = q.select_rows(&self.cont_indices).select_cols(&self.cont_indices);
            let q_d = q.select_rows(&self.binary_indices).select_cols(&self.binary_indices);
            let h_alpha = q.select_rows(&self.binary_indices).select_cols(&self.cont_indices);
            let add = h_alpha.vecmat(fixed);
            for (ci, a) in c.iter_mut().zip(&add) {
                *ci += a;
            }
            c_c += 0.5 * dot(fixed, &q_d.matvec(fixed));
            q_c
        });
        let mut sub = MpProgram::new(a_c, b, c, h_c, q, p.a_t.clone(), p.b_t.clone(), f, new_eq, true);
        sub.c_c = c_c;
        sub.c_t = c_t;
        sub.q_t = p.q_t.clone();
        sub
    }

    /// Is the partial binary assignment (first k binaries) MILP-feasible?
    pub fn check_bin_feasibility(&self, partial: &[f64]) -> bool {
        let p = &self.base;
        let nx = p.num_x();
        let nt = p.num_t();
        let (pa, pb) = p.xtheta_polytope();
        let mut a = Mat::zeros(0, nx + nt);
        let mut b = vec![];
        for (k, &v) in partial.iter().enumerate() {
            let mut row = vec![0.0; nx + nt];
            row[self.binary_indices[k]] = 1.0;
            a.push_row(&row);
            b.push(v);
        }
        let a = Mat::vstack(&[&a, &pa]);
        b.extend(pb);
        let mut eqs: Vec<usize> = (0..partial.len()).collect();
        eqs.extend(p.equality_indices.iter().map(|i| i + partial.len()));
        p.solver.solve_milp(None, &a, &b, &eqs, &self.binary_indices).is_some()
    }

    /// Solves the substituted MILP/MIQP at θ.
    pub fn solve_theta(&self, theta: &[f64]) -> Option<SolverOutput> {
        let p = &self.base;
        let mut c = p.h.matvec(theta);
        for (ci, c0) in c.iter_mut().zip(&p.c) {
            *ci += c0;
        }
        let ft = p.f.matvec(theta);
        let b: Vec<f64> = p.b.iter().zip(&ft).map(|(x, y)| x + y).collect();
        let mut s = match &p.q {
            Some(q) => p.solver.solve_miqp(q, Some(&c), &p.a, &b, &p.equality_indices, &self.binary_indices)?,
            None => p.solver.solve_milp(Some(&c), &p.a, &b, &p.equality_indices, &self.binary_indices)?,
        };
        s.obj += p.c_c + dot(&p.c_t, theta) + 0.5 * dot(theta, &p.q_t.matvec(theta));
        Some(s)
    }

    /// Feasible binary combinations by depth-first tree search (PPOPT `MITree`).
    pub fn feasible_binary_combinations(&self) -> Vec<Vec<f64>> {
        let mut out = vec![];
        let mut stack: Vec<Vec<f64>> = vec![vec![]];
        while let Some(fix) = stack.pop() {
            if fix.len() == self.binary_indices.len() {
                out.push(fix);
                continue;
            }
            for v in [1.0, 0.0] {
                let mut f = fix.clone();
                f.push(v);
                if self.check_bin_feasibility(&f) {
                    stack.push(f);
                }
            }
        }
        // PPOPT order: right (0) before left (1) at every level
        out.sort_by(|a, b| a.partial_cmp(b).unwrap());
        out
    }
}

/// PPOPT `solve_mpmiqp` with the enumeration algorithm: one continuous mp problem per feasible
/// binary combination (solved in parallel), regions merged into an overlapping solution.
pub fn solve_mpmiqp_enumeration(program: &Arc<MpmiProgram>, cont_algorithm: MpqpAlgorithm, opts: &SolveOptions) -> Solution {
    let combos = program.feasible_binary_combinations();
    let sols: Vec<(Vec<f64>, Solution)> = combos
        .par_iter()
        .map(|fix| {
            let sub = Arc::new(program.generate_substituted_problem(fix));
            (fix.clone(), solve_mpqp_with(&sub, cont_algorithm, opts))
        })
        .collect();
    let mut regions = vec![];
    for (fix, sol) in sols {
        for mut r in sol.critical_regions {
            r.y_fixation = Some(fix.clone());
            r.y_indices = Some(program.binary_indices.clone());
            r.x_indices = Some(program.cont_indices.clone());
            regions.push(r);
        }
    }
    Solution {
        program: program.clone(),
        continuous: None,
        critical_regions: regions,
        is_overlapping: true,
        point_location_tolerance: 1e-5,
    }
}
