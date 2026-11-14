//! Dense LP solver for the small, free-variable LPs that dominate multiparametric programming.
//!
//! The primal problem is
//!
//! ```text
//!     min  c'x   s.t.  A_I x <= b_I,  A_E x = b_E,  x free
//! ```
//!
//! Multiparametric programs produce LPs with few variables (the parameter dimension, plus a
//! radius) and many constraints. We therefore run a two-phase primal simplex on the *dual*
//!
//! ```text
//!     min  b'y   s.t.  A'y = -c,  y_I >= 0,  y_E free (split into y+ - y-)
//! ```
//!
//! whose tableau has only `n` rows (one per primal variable). The primal optimum is read off the
//! simplex multipliers of the dual (`x = pi`), and the primal Lagrange multipliers are the dual
//! basic solution `y`. Dual unbounded means primal infeasible; dual infeasible means primal
//! unbounded (or infeasible); both return `None`, matching how PPOPT treats non-optimal statuses.
//!
//! Robustness: when the simplex stops, the tableau is rebuilt from the final basis and the
//! original data (`refactor`), and the answer is re-verified there: optimality by recomputed
//! reduced costs, infeasibility by checking the Farkas ray `A'y = 0, y >= 0, b'y < 0` on the
//! original matrix. If a check fails the iterations resume from the clean tableau.

use crate::linalg::{dot, Mat};
use super::lp_core::{solve, Want};

#[derive(Clone, Debug)]
pub struct LpSolution {
    pub x: Vec<f64>,
    pub obj: f64,
    /// Lagrange multiplier of every primal constraint (>= 0 for inequalities).
    pub dual: Vec<f64>,
    /// b - A x
    pub slack: Vec<f64>,
    /// constraints with |slack| <= 1e-10 or a nonzero multiplier (PPOPT convention)
    pub active_set: Vec<usize>,
}

/// Number of LPs solved so far (all threads, sharded counter), for benchmarking.
pub static LP_CALLS: crate::stats::Sharded = crate::stats::Sharded::new();

/// Solves `min c'x s.t. A x <= b` with rows listed in `eq` as equalities and x free.
/// `c = None` is a pure feasibility problem.
pub fn solve_lp(c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize]) -> Option<LpSolution> {
    LP_CALLS.add(1);
    let core = solve(c, a, b, eq, Want::Full)?;
    let m = a.rows;
    let x = core.x;
    let slack: Vec<f64> = (0..m).map(|i| b[i] - dot(a.row(i), &x)).collect();
    let obj = c.map(|c| dot(c, &x)).unwrap_or(0.0);
    let dual = core.dual;
    let active_set = (0..m).filter(|&i| slack[i].abs() <= 1e-10 || dual[i] != 0.0).collect();
    Some(LpSolution { x, obj, dual, slack, active_set })
}

/// Feasibility of {A x <= b, A[eq] x = b[eq]} without building the solution.
pub fn lp_feasible(a: &Mat, b: &[f64], eq: &[usize]) -> bool {
    LP_CALLS.add(1);
    match small_feasible(a, b, eq) {
        super::lp_small::Small::Feasible(_) => return true,
        super::lp_small::Small::Infeasible => return false,
        super::lp_small::Small::Undecided => {}
    }
    solve(None, a, b, eq, Want::Feasible).is_some()
}

/// Revised-simplex fast path for inequality-only feasibility LPs with n <= 4 variables.
fn small_feasible(a: &Mat, b: &[f64], eq: &[usize]) -> super::lp_small::Small {
    if eq.is_empty() && a.cols <= 4 {
        return super::lp_small::feasible(a, b);
    }
    super::lp_small::Small::Undecided
}

/// `lp_feasible` starting the simplex from `x0` (see `lp_core::solve_from`).
pub fn lp_feasible_from(a: &Mat, b: &[f64], eq: &[usize], x0: &[f64]) -> bool {
    LP_CALLS.add(1);
    super::lp_core::solve_from(None, a, b, eq, Some(x0), Want::Feasible).is_some()
}

/// A feasible point (or None), starting the simplex from `x0` when given.
pub fn lp_feasible_point(a: &Mat, b: &[f64], eq: &[usize], x0: Option<&[f64]>) -> Option<Vec<f64>> {
    LP_CALLS.add(1);
    if eq.is_empty() && a.cols <= 4 {
        let shifted;
        let bb: &[f64] = match x0 {
            Some(x0) => {
                shifted = (0..a.rows).map(|i| b[i] - crate::linalg::dot(a.row(i), x0)).collect::<Vec<f64>>();
                &shifted
            }
            None => b,
        };
        match small_feasible(a, bb, eq) {
            super::lp_small::Small::Feasible(mut z) => {
                if let Some(x0) = x0 {
                    for (v, o) in z.iter_mut().zip(x0) {
                        *v += o;
                    }
                }
                return Some(z);
            }
            super::lp_small::Small::Infeasible => return None,
            super::lp_small::Small::Undecided => {}
        }
    }
    super::lp_core::solve_from(None, a, b, eq, x0, Want::Point).map(|c| c.x)
}

/// `lp_feasible_point(a, b, eq, Some(x0))` given the shifted right hand side
/// `bs[i] = b[i] - a.row(i)·x0` (computed once by callers that start several LPs at `x0`).
pub fn lp_feasible_point_shifted(a: &Mat, bs: &[f64], eq: &[usize], x0: &[f64]) -> Option<Vec<f64>> {
    LP_CALLS.add(1);
    if eq.is_empty() && a.cols <= 4 {
        return match super::lp_small::feasible(a, bs) {
            super::lp_small::Small::Feasible(mut z) => {
                for (v, o) in z.iter_mut().zip(x0) {
                    *v += o;
                }
                Some(z)
            }
            super::lp_small::Small::Infeasible => None,
            super::lp_small::Small::Undecided => super::lp_core::solve_shifted(a, bs, eq, x0, Want::Point).map(|c| c.x),
        };
    }
    super::lp_core::solve_shifted(a, bs, eq, x0, Want::Point).map(|c| c.x)
}

/// `lp_point` starting the simplex from `x0`.
pub fn lp_point_from(c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize], x0: &[f64]) -> Option<Vec<f64>> {
    LP_CALLS.add(1);
    super::lp_core::solve_from(c, a, b, eq, Some(x0), Want::Point).map(|c| c.x)
}

/// Optimal point only (no duals, slacks or active set).
pub fn lp_point(c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize]) -> Option<Vec<f64>> {
    LP_CALLS.add(1);
    solve(c, a, b, eq, Want::Point).map(|c| c.x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_lp() {
        // min -x - y s.t. x + 2y <= 4, 3x + y <= 6, x >= 0, y >= 0 -> (1.6, 1.2)
        let a = Mat::from_rows(&[vec![1.0, 2.0], vec![3.0, 1.0], vec![-1.0, 0.0], vec![0.0, -1.0]]);
        let b = vec![4.0, 6.0, 0.0, 0.0];
        let s = solve_lp(Some(&[-1.0, -1.0]), &a, &b, &[]).unwrap();
        assert!((s.x[0] - 1.6).abs() < 1e-9 && (s.x[1] - 1.2).abs() < 1e-9, "{:?}", s.x);
        assert!((s.obj + 2.8).abs() < 1e-9);
        assert!(s.active_set == vec![0, 1]);
    }

    #[test]
    fn infeasible_and_unbounded() {
        let a = Mat::from_rows(&[vec![1.0], vec![-1.0]]);
        assert!(solve_lp(None, &a, &[1.0, -2.0], &[]).is_none());
        let a = Mat::from_rows(&[vec![1.0]]);
        assert!(solve_lp(Some(&[-1.0]), &Mat::from_rows(&[vec![-1.0]]), &[0.0], &[]).is_none());
        assert!(solve_lp(Some(&[1.0]), &a, &[3.0], &[]).is_none());
    }

    #[test]
    fn equality_lp() {
        let a = Mat::from_rows(&[vec![1.0, 1.0], vec![1.0, -1.0], vec![-1.0, 0.0]]);
        let s = solve_lp(Some(&[1.0, 1.0]), &a, &[2.0, 0.0, 0.0], &[0]).unwrap();
        assert!((s.obj - 2.0).abs() < 1e-9);
        assert!((s.x[0] + s.x[1] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn feasibility_cone() {
        let a = Mat::from_rows(&[vec![1.0, 0.0], vec![-1.0, 0.0], vec![0.0, 1.0], vec![0.0, -1.0]]);
        let s = solve_lp(None, &a, &[1.0, -1.0, 3.0, 3.0], &[]).unwrap();
        assert!((s.x[0] - 1.0).abs() < 1e-9);
        assert!(solve_lp(None, &a, &[1.0, -1.5, 3.0, 3.0], &[]).is_none());
    }

    #[test]
    fn chebyshev_box() {
        let a = Mat::from_rows(&[vec![1.0, 0.0], vec![-1.0, 0.0], vec![0.0, 1.0], vec![0.0, -1.0]]);
        let s = crate::utils::chebyshev_ball(&a, &[1.0, 0.0, 1.0, 0.0], &[]).unwrap();
        assert!((s.x[2] - 0.5).abs() < 1e-9, "{:?}", s.x);
    }

    #[test]
    fn badly_scaled_rows() {
        // a huge redundant row must not loosen the tolerance on the others
        let a = Mat::from_rows(&[vec![1.0, 0.0], vec![-1.0, 0.0], vec![0.0, 1.0], vec![0.0, -1.0], vec![0.6, 0.8]]);
        let b = [1.0, -1.001, 1.0, 1.0, 9e7];
        assert!(solve_lp(None, &a, &b, &[]).is_none());
    }
}
