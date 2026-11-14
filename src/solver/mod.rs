//! Deterministic solver layer (PPOPT `solver.py` / `solver_interface`).
//!
//! PPOPT dispatches to Gurobi, GLPK, quadprog or DAQP through Python. Here every sub-problem is
//! solved natively: a dense dual-simplex-style LP, the Goldfarb-Idnani QP, and branch and bound for
//! the mixed-integer variants.

pub mod lp;
pub(crate) mod lp_core;
pub mod lp_small;
pub mod qp;
pub mod sparse_lp;

use crate::linalg::{dot, Mat};
use serde::{Deserialize, Serialize};

/// PPOPT's `SolverOutput`.
#[derive(Clone, Debug)]
pub struct SolverOutput {
    pub obj: f64,
    pub sol: Vec<f64>,
    pub slack: Vec<f64>,
    pub active_set: Vec<usize>,
    pub dual: Vec<f64>,
}

impl From<lp::LpSolution> for SolverOutput {
    fn from(s: lp::LpSolution) -> Self {
        SolverOutput { obj: s.obj, sol: s.x, slack: s.slack, active_set: s.active_set, dual: s.dual }
    }
}

impl From<qp::QpSolution> for SolverOutput {
    fn from(s: qp::QpSolution) -> Self {
        SolverOutput { obj: s.obj, sol: s.x, slack: s.slack, active_set: s.active_set, dual: s.dual }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Solver {
    /// Maximum number of branch-and-bound nodes for MILP/MIQP sub-problems.
    pub max_bb_nodes: Option<usize>,
}

impl Solver {
    pub fn solve_lp(&self, c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize]) -> Option<SolverOutput> {
        lp::solve_lp(c, a, b, eq).map(Into::into)
    }

    pub fn solve_qp(&self, q: &Mat, c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize]) -> Option<SolverOutput> {
        qp::solve_qp(q, c, a, b, eq).map(Into::into)
    }

    /// MILP by depth-first branch and bound on LP relaxations. `c = None` stops at the first
    /// integer-feasible point (feasibility problems).
    pub fn solve_milp(&self, c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize], bin_vars: &[usize]) -> Option<SolverOutput> {
        branch_and_bound(None, c, a, b, eq, bin_vars, self.max_bb_nodes.unwrap_or(1_000_000))
    }

    /// MIQP by depth-first branch and bound on QP relaxations (Q must be positive definite).
    pub fn solve_miqp(&self, q: &Mat, c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize], bin_vars: &[usize]) -> Option<SolverOutput> {
        branch_and_bound(Some(q), c, a, b, eq, bin_vars, self.max_bb_nodes.unwrap_or(1_000_000))
    }
}

fn branch_and_bound(
    q: Option<&Mat>,
    c: Option<&[f64]>,
    a: &Mat,
    b: &[f64],
    eq: &[usize],
    bin_vars: &[usize],
    max_nodes: usize,
) -> Option<SolverOutput> {
    let n = a.cols.max(q.map(|q| q.rows).unwrap_or(0));
    if bin_vars.is_empty() {
        return match q {
            Some(q) => qp::solve_qp(q, c, a, b, eq).map(Into::into),
            None => lp::solve_lp(c, a, b, eq).map(Into::into),
        };
    }
    let feasibility_only = c.is_none() && q.is_none();
    let m = a.rows;
    // node = fixed values (-1 free, 0, 1) for every binary
    let mut stack: Vec<Vec<i8>> = vec![vec![-1; bin_vars.len()]];
    let mut best: Option<SolverOutput> = None;
    let mut nodes = 0usize;
    while let Some(fix) = stack.pop() {
        nodes += 1;
        if nodes > max_nodes {
            break;
        }
        // build relaxation: original rows, then per binary either x=v (equality) or 0<=x<=1
        let mut rows = a.clone();
        let mut rhs = b.to_vec();
        let mut eqs = eq.to_vec();
        for (k, &v) in bin_vars.iter().enumerate() {
            let mut e = vec![0.0; n];
            e[v] = 1.0;
            match fix[k] {
                -1 => {
                    rows.push_row(&e);
                    rhs.push(1.0);
                    let ne: Vec<f64> = e.iter().map(|x| -x).collect();
                    rows.push_row(&ne);
                    rhs.push(0.0);
                }
                val => {
                    rows.push_row(&e);
                    rhs.push(val as f64);
                    eqs.push(rows.rows - 1);
                }
            }
        }
        let relax: Option<SolverOutput> = match q {
            Some(q) => qp::solve_qp(q, c, &rows, &rhs, &eqs).map(Into::into),
            None => lp::solve_lp(c, &rows, &rhs, &eqs).map(Into::into),
        };
        let Some(sol) = relax else { continue };
        if let Some(inc) = &best {
            if sol.obj >= inc.obj - 1e-9 * inc.obj.abs().max(1.0) {
                continue;
            }
        }
        // most fractional binary
        let mut branch = None;
        let mut worst = 1e-7;
        for (k, &v) in bin_vars.iter().enumerate() {
            if fix[k] >= 0 {
                continue;
            }
            let f = (sol.sol[v] - sol.sol[v].round()).abs();
            if f > worst {
                worst = f;
                branch = Some(k);
            }
        }
        match branch {
            None => {
                let mut s = sol;
                for &v in bin_vars {
                    s.sol[v] = s.sol[v].round();
                }
                s.slack.truncate(m);
                s.dual.truncate(m);
                s.active_set.retain(|&i| i < m);
                // recompute objective with rounded binaries
                let lin = c.map(|c| dot(c, &s.sol)).unwrap_or(0.0);
                let quad = q.map(|q| 0.5 * dot(&s.sol, &q.matvec(&s.sol))).unwrap_or(0.0);
                s.obj = lin + quad;
                best = Some(s);
                if feasibility_only {
                    break;
                }
            }
            Some(k) => {
                let v = sol.sol[bin_vars[k]];
                let (first, second) = if v >= 0.5 { (0i8, 1i8) } else { (1i8, 0i8) };
                // push the less likely branch first so the closer one is explored next
                let mut f1 = fix.clone();
                f1[k] = first;
                let mut f2 = fix;
                f2[k] = second;
                stack.push(f1);
                stack.push(f2);
            }
        }
    }
    best
}
