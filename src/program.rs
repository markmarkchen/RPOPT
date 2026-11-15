//! Port of `ppopt.mplp_program.MPLP_Program` and `ppopt.mpqp_program.MPQP_Program`.
//!
//! One struct covers both: `q == None` is an mpLP, `q == Some(Q)` an mpQP.
//!
//! ```text
//!   min_x  1/2 x'Qx + θ'H'x + c'x + c_c + c_t'θ + 1/2 θ'Q_tθ
//!   s.t.   A x <= b + Fθ        (rows in equality_indices hold with equality)
//!          A_t θ <= b_t
//! ```

use crate::linalg::{dot, is_full_rank_rows, min_eigenvalue_sym, pinv, Cholesky, Lu, Mat};
use crate::solver::{Solver, SolverOutput};
use crate::utils::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Precomputed Schur-complement data for fast KKT solves of strictly convex mpQPs.
#[derive(Clone, Debug)]
pub struct KktCache {
    pub qi_h: Mat,      // Q^{-1} H            (n_x x n_t)
    pub qi_c: Vec<f64>, // Q^{-1} c
    pub a_qi: Mat,      // A Q^{-1}            (m x n_x)
    pub m: Mat,         // A Q^{-1} A'         (m x m)
    pub a_qi_h: Mat,    // A Q^{-1} H          (m x n_t)
    pub a_qi_c: Vec<f64>,
    /// -(A Q^{-1} H + F): rows of the inactive-constraint block before the active-set terms
    pub base_e: Mat,
    /// b + A Q^{-1} c
    pub base_f: Vec<f64>,
}

/// Optimal control law of an active set: x = A_x θ + b_x, λ = A_l θ + b_l.
#[derive(Clone, Debug)]
pub struct ControlLaw {
    pub a_x: Mat,
    pub b_x: Vec<f64>,
    pub a_l: Mat,
    pub b_l: Vec<f64>,
}

/// Result of PPOPT's `check_optimality` LP.
#[derive(Clone, Debug)]
pub struct OptimalityCertificate {
    pub x: Vec<f64>,
    pub theta: Vec<f64>,
    pub lambda: Vec<f64>,
    pub slack: Vec<f64>,
    pub t: f64,
    pub equality_indices: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MpProgram {
    pub a: Mat,
    pub b: Vec<f64>,
    pub c: Vec<f64>,
    /// n_x x n_t, the objective has θ'H'x
    pub h: Mat,
    /// None for an mpLP
    pub q: Option<Mat>,
    pub a_t: Mat,
    pub b_t: Vec<f64>,
    pub f: Mat,
    pub c_c: f64,
    pub c_t: Vec<f64>,
    pub q_t: Mat,
    pub equality_indices: Vec<usize>,
    #[serde(skip)]
    pub solver: Solver,
    #[serde(skip)]
    pub warnings: Vec<String>,
    #[serde(skip)]
    kkt: OnceLock<Option<KktCache>>,
    #[serde(skip)]
    xtheta: OnceLock<(Mat, Vec<f64>)>,
    #[serde(skip)]
    qp_factor: OnceLock<Option<crate::solver::qp::QpFactor>>,
    #[serde(skip)]
    theta_bbox: OnceLock<Option<(Vec<f64>, Vec<f64>)>>,
}

/// Raw problem data in PPOPT's JSON layout (also what `bench/mpc_problems.py` writes).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProblemData {
    #[serde(rename = "A")]
    pub a: Vec<Vec<f64>>,
    pub b: Vec<Vec<f64>>,
    pub c: Vec<Vec<f64>>,
    #[serde(rename = "H")]
    pub h: Vec<Vec<f64>>,
    #[serde(rename = "Q", default)]
    pub q: Option<Vec<Vec<f64>>>,
    #[serde(rename = "A_t")]
    pub a_t: Vec<Vec<f64>>,
    pub b_t: Vec<Vec<f64>>,
    #[serde(rename = "F")]
    pub f: Vec<Vec<f64>>,
    #[serde(default)]
    pub equality_indices: Vec<usize>,
}

fn flat(v: &[Vec<f64>]) -> Vec<f64> {
    v.iter().flat_map(|r| r.iter().cloned()).collect()
}

impl MpProgram {
    /// Builds and processes an mpQP (PPOPT `MPQP_Program(...)` with `post_process=True`).
    #[allow(clippy::too_many_arguments)]
    pub fn new_mpqp(a: Mat, b: Vec<f64>, c: Vec<f64>, h: Mat, q: Mat, a_t: Mat, b_t: Vec<f64>, f: Mat, equality_indices: Vec<usize>) -> Self {
        Self::new(a, b, c, h, Some(q), a_t, b_t, f, equality_indices, true)
    }

    /// Builds and processes an mpLP (PPOPT `MPLP_Program(...)`).
    #[allow(clippy::too_many_arguments)]
    pub fn new_mplp(a: Mat, b: Vec<f64>, c: Vec<f64>, h: Mat, a_t: Mat, b_t: Vec<f64>, f: Mat, equality_indices: Vec<usize>) -> Self {
        Self::new(a, b, c, h, None, a_t, b_t, f, equality_indices, true)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        a: Mat,
        b: Vec<f64>,
        c: Vec<f64>,
        h: Mat,
        q: Option<Mat>,
        a_t: Mat,
        b_t: Vec<f64>,
        f: Mat,
        equality_indices: Vec<usize>,
        post_process: bool,
    ) -> Self {
        let nt = f.cols;
        let mut p = MpProgram {
            a,
            b,
            c,
            h,
            q,
            a_t,
            b_t,
            f,
            c_c: 0.0,
            c_t: vec![0.0; nt],
            q_t: Mat::zeros(nt, nt),
            equality_indices,
            solver: Solver::default(),
            warnings: vec![],
            kkt: OnceLock::new(),
            xtheta: OnceLock::new(),
            qp_factor: OnceLock::new(),
            theta_bbox: OnceLock::new(),
        };
        p.base_constraint_processing();
        p.warnings = p.compute_warnings();
        if post_process {
            p.process_constraints();
        }
        p
    }

    /// Assembles an already processed program from its stored fields without any preprocessing
    /// (used by binary loading). Derived caches are rebuilt lazily.
    #[allow(clippy::too_many_arguments)]
    pub fn from_raw_parts(a: Mat, b: Vec<f64>, c: Vec<f64>, h: Mat, q: Option<Mat>, a_t: Mat, b_t: Vec<f64>, f: Mat, c_c: f64, c_t: Vec<f64>, q_t: Mat, equality_indices: Vec<usize>) -> Self {
        MpProgram {
            a,
            b,
            c,
            h,
            q,
            a_t,
            b_t,
            f,
            c_c,
            c_t,
            q_t,
            equality_indices,
            solver: Solver::default(),
            warnings: vec![],
            kkt: OnceLock::new(),
            xtheta: OnceLock::new(),
            qp_factor: OnceLock::new(),
            theta_bbox: OnceLock::new(),
        }
    }

    /// Loads the JSON format written by `bench/mpc_problems.py`; mpQP if "Q" is present.
    pub fn from_problem_data(d: &ProblemData) -> Self {
        let a = Mat::from_rows(&d.a);
        let nx = a.cols;
        let f = Mat::from_rows(&d.f);
        let nt = f.cols;
        let h = if d.h.is_empty() { Mat::zeros(nx, nt) } else { Mat::from_rows(&d.h) };
        let a_t = Mat::from_rows(&d.a_t);
        let q = d.q.as_ref().map(|q| Mat::from_rows(q));
        MpProgram::new(a, flat(&d.b), flat(&d.c), h, q, a_t, flat(&d.b_t), f, d.equality_indices.clone(), true)
    }

    pub fn from_json_file(path: &std::path::Path) -> std::io::Result<Self> {
        let txt = std::fs::read_to_string(path)?;
        let d: ProblemData = serde_json::from_str(&txt).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Self::from_problem_data(&d))
    }

    pub fn is_qp(&self) -> bool {
        self.q.is_some()
    }

    pub fn num_x(&self) -> usize {
        self.a.cols
    }

    pub fn num_t(&self) -> usize {
        self.f.cols
    }

    pub fn num_constraints(&self) -> usize {
        self.a.rows
    }

    pub fn num_inequality_constraints(&self) -> usize {
        self.a.rows - self.equality_indices.len()
    }

    pub fn num_equality_constraints(&self) -> usize {
        self.equality_indices.len()
    }

    /// Drops cached derived data; call after editing the matrices directly.
    pub fn invalidate_caches(&mut self) {
        self.invalidate();
    }

    fn invalidate(&mut self) {
        self.kkt = OnceLock::new();
        self.xtheta = OnceLock::new();
        self.qp_factor = OnceLock::new();
        self.theta_bbox = OnceLock::new();
    }

    /// Bounding box of the parameter set {A_t θ <= b_t} (2 n_t LPs, cached). Constraint processing
    /// can drop parameter bounds that the other constraints imply (e.g. with a terminal set), which
    /// leaves {A_t θ <= b_t} unbounded; the box of the feasible parameters (the projection of the
    /// (x, θ) polytope) is used then. None only if that is unbounded too.
    pub fn theta_bbox(&self) -> Option<&(Vec<f64>, Vec<f64>)> {
        self.theta_bbox
            .get_or_init(|| {
                crate::census_purpose!(Bbox);
                let n = self.num_t();
                let bbox = |a: &Mat, b: &[f64], off: usize| -> Option<(Vec<f64>, Vec<f64>)> {
                    let mut lo = vec![0.0; n];
                    let mut hi = vec![0.0; n];
                    let mut c = vec![0.0; a.cols];
                    for k in 0..n {
                        c[off + k] = 1.0;
                        lo[k] = crate::solver::lp::lp_point(Some(&c), a, b, &[])?[off + k];
                        c[off + k] = -1.0;
                        hi[k] = crate::solver::lp::lp_point(Some(&c), a, b, &[])?[off + k];
                        c[off + k] = 0.0;
                    }
                    Some((lo, hi))
                };
                let own = if self.a_t.rows == 0 { None } else { bbox(&self.a_t, &self.b_t, 0) };
                own.or_else(|| {
                    let (pa, pb) = self.xtheta_cached();
                    let eq: Vec<usize> = self.equality_indices.clone();
                    if eq.is_empty() {
                        bbox(pa, pb, self.num_x())
                    } else {
                        // equality rows must hold: bound θ on {A x - F θ <= b, rows eq at equality}
                        let mut lo = vec![0.0; n];
                        let mut hi = vec![0.0; n];
                        let mut c = vec![0.0; pa.cols];
                        let off = self.num_x();
                        for k in 0..n {
                            c[off + k] = 1.0;
                            lo[k] = crate::solver::lp::lp_point(Some(&c), pa, pb, &eq)?[off + k];
                            c[off + k] = -1.0;
                            hi[k] = crate::solver::lp::lp_point(Some(&c), pa, pb, &eq)?[off + k];
                            c[off + k] = 0.0;
                        }
                        Some((lo, hi))
                    }
                })
            })
            .as_ref()
    }

    /// Rows of {Eθ <= f} that can bind inside the parameter bounding box: a row whose maximum over
    /// the box is strictly below its rhs is implied by the box rows (always part of a region) and
    /// is dropped. The feasible set, and any Chebyshev ball, are unchanged.
    pub fn box_reduced(&self, e: &Mat, f: &[f64]) -> Option<(Mat, Vec<f64>)> {
        let (lo, hi) = self.theta_bbox()?;
        let n = e.cols;
        let mut keep = Vec::with_capacity(e.rows);
        for i in 0..e.rows {
            let row = e.row(i);
            let ub: f64 = row.iter().zip(lo.iter().zip(hi)).map(|(a, (l, h))| (a * l).max(a * h)).sum();
            if !(ub < f[i] - 1e-9 * (1.0 + f[i].abs())) {
                keep.push(i);
            }
        }
        if keep.len() == e.rows {
            return None;
        }
        let mut er = Mat::zeros(keep.len(), n);
        let mut fr = Vec::with_capacity(keep.len());
        for (r, &i) in keep.iter().enumerate() {
            er.row_mut(r).copy_from_slice(e.row(i));
            fr.push(f[i]);
        }
        Some((er, fr))
    }

    /// Cached [A, -F; 0, A_t] and [b; b_t] (valid once construction has finished).
    pub fn xtheta_cached(&self) -> &(Mat, Vec<f64>) {
        self.xtheta.get_or_init(|| self.xtheta_polytope())
    }

    /// [A, -F; 0, A_t] and [b; b_t]: the (x, θ) feasible polytope.
    pub fn xtheta_polytope(&self) -> (Mat, Vec<f64>) {
        let nx = self.num_x();
        let nt = self.num_t();
        let m = self.num_constraints();
        let mt = self.a_t.rows;
        let mut big = Mat::zeros(m + mt, nx + nt);
        for i in 0..m {
            let row = big.row_mut(i);
            row[..nx].copy_from_slice(self.a.row(i));
            for (k, v) in self.f.row(i).iter().enumerate() {
                row[nx + k] = -v;
            }
        }
        for i in 0..mt {
            big.row_mut(m + i)[nx..].copy_from_slice(self.a_t.row(i));
        }
        let mut bb = self.b.clone();
        bb.extend_from_slice(&self.b_t);
        (big, bb)
    }

    // ------------------------------------------------------------------ processing

    pub fn base_constraint_processing(&mut self) {
        self.invalidate();
        let m = self.num_constraints();
        if !self.equality_indices.is_empty() {
            let mut order = self.equality_indices.clone();
            order.extend(select_not_in(m, &self.equality_indices));
            self.reorder_rows(&order);
            self.equality_indices = (0..self.equality_indices.len()).collect();
        }
        self.process_program_constraints(1e-6);
        self.scale_constraints();
        self.find_implicit_equalities();
        self.generate_reduced_equality_constraints();
    }

    fn reorder_rows(&mut self, order: &[usize]) {
        self.a = self.a.select_rows(order);
        self.f = self.f.select_rows(order);
        self.b = order.iter().map(|&i| self.b[i]).collect();
    }

    /// Moves 0 <= b_i rows and purely parametric rows (||A_i|| = 0) into A_t θ <= b_t.
    fn process_program_constraints(&mut self, epsilon: f64) {
        for pass in 0..2 {
            let probe = if pass == 0 {
                let negf = self.f.neg();
                Mat::hstack(&[&self.a, &negf])
            } else {
                self.a.clone()
            };
            let (keep, moved) = get_indices_of_zero_rows(&probe, epsilon);
            if moved.is_empty() {
                continue;
            }
            for &i in &moved {
                let row: Vec<f64> = self.f.row(i).iter().map(|v| -v).collect();
                self.a_t.push_row(&row);
                self.b_t.push(self.b[i]);
            }
            // keep equality bookkeeping consistent with the kept rows
            let mut new_eq = vec![];
            for (new_i, &old_i) in keep.iter().enumerate() {
                if self.equality_indices.contains(&old_i) {
                    new_eq.push(new_i);
                }
            }
            self.reorder_rows(&keep);
            self.equality_indices = new_eq;
        }
    }

    /// Rescales [A | b, F] rows so that ||[A | -F]_i||_2 = 1.
    pub fn scale_constraints(&mut self) {
        for i in 0..self.num_constraints() {
            let nrm = (dot(self.a.row(i), self.a.row(i)) + dot(self.f.row(i), self.f.row(i))).sqrt();
            if nrm == 0.0 {
                continue;
            }
            for v in self.a.row_mut(i) {
                *v /= nrm;
            }
            for v in self.f.row_mut(i) {
                *v /= nrm;
            }
            self.b[i] /= nrm;
        }
        self.invalidate();
    }

    fn find_implicit_equalities(&mut self) {
        let negf = self.f.neg();
        let probe = Mat::hstack(&[&self.a, &negf]);
        let pairs = detect_implicit_equalities(&probe, &self.b);
        if pairs.is_empty() {
            return;
        }
        let mut keep: Vec<usize> = pairs.iter().map(|p| p.0).collect();
        keep.sort();
        keep.dedup();
        let mut remove: Vec<usize> = pairs.iter().map(|p| p.1).collect();
        remove.sort();
        remove.dedup();
        remove.retain(|i| !keep.contains(i));
        let mut temp: Vec<usize> = self.equality_indices.clone();
        for k in keep {
            if !temp.contains(&k) {
                temp.push(k);
            }
        }
        let m = self.num_constraints();
        let kept_ineqs: Vec<usize> = (0..m).filter(|i| !temp.contains(i) && !remove.contains(i)).collect();
        let mut order = temp.clone();
        order.extend(kept_ineqs);
        self.reorder_rows(&order);
        self.equality_indices = (0..temp.len()).collect();
    }

    fn generate_reduced_equality_constraints(&mut self) {
        if self.equality_indices.is_empty() || is_full_rank_rows(&self.a, &self.equality_indices) {
            return;
        }
        // greedy independent subset of the equality rows (PPOPT's helper drops the last row; we keep it if independent)
        let mut kept: Vec<usize> = vec![];
        let mut r = 0;
        for &e in &self.equality_indices {
            let mut trial = kept.clone();
            trial.push(e);
            let rk = crate::linalg::rank(&self.a.select_rows(&trial));
            if rk > r {
                kept.push(e);
                r = rk;
            }
        }
        let m = self.num_constraints();
        let mut order = kept.clone();
        order.extend(select_not_in(m, &self.equality_indices));
        self.reorder_rows(&order);
        self.equality_indices = (0..kept.len()).collect();
    }

    /// Removes strongly redundant rows of the (x, θ) polytope (PPOPT `process_constraints`).
    pub fn process_constraints(&mut self) {
        self.invalidate();
        let (pa, pb) = self.xtheta_polytope();
        let saved = find_redundant_constraints(&pa, &pb, &self.equality_indices);
        let m = self.num_constraints();
        let upper: Vec<usize> = saved.iter().cloned().filter(|&i| i < m).collect();
        let lower: Vec<usize> = saved.iter().cloned().filter(|&i| i >= m).map(|i| i - m).collect();
        let mut new_eq = vec![];
        for (new_i, &old_i) in upper.iter().enumerate() {
            if self.equality_indices.contains(&old_i) {
                new_eq.push(new_i);
            }
        }
        self.reorder_rows(&upper);
        self.equality_indices = new_eq;
        self.a_t = self.a_t.select_rows(&lower);
        self.b_t = lower.iter().map(|&i| self.b_t[i]).collect();
    }

    fn compute_warnings(&self) -> Vec<String> {
        let mut w = vec![];
        if self.c.len() != self.num_x() {
            w.push(format!("The A and c matrices disagree in number of parameters A({}, {}), c({})", self.a.rows, self.a.cols, self.c.len()));
        }
        if self.a.rows != self.b.len() {
            w.push("The A and b matrices disagree in vertical dimension".into());
        }
        if self.a.rows != self.f.rows {
            w.push("The A and F matrices disagree in vertical dimension".into());
        }
        if self.a_t.rows > 0 && self.f.cols != self.a_t.cols {
            w.push("The F and A_t matrices disagree in dimension, inconsistent number of parameters".into());
        }
        if w.is_empty() {
            if self.feasible_space_chebychev_ball().is_none() {
                w.push("The chebychev ball has either a radius of zero, or the problem is not feasible!".into());
            }
            if !self.check_feasibility(&self.equality_indices, true) {
                w.push("The multiparametric program, as stated, is not feasible!".into());
            }
        }
        if let Some(q) = &self.q {
            if q.rows != q.cols {
                w.push("Q matrix is not square".into());
            } else {
                let ev = min_eigenvalue_sym(q);
                if ev < 0.0 {
                    w.push(format!("Non-convex quadratic program detected, min eigenvalue {ev}"));
                } else if ev < 1e-4 {
                    w.push(format!("Possible positive semi-definite nature detected in Q, min eigenvalue {ev}"));
                }
            }
        }
        w
    }

    // ------------------------------------------------------------------ evaluation

    pub fn evaluate_objective(&self, x: &[f64], theta: &[f64]) -> f64 {
        let ht = self.h.matvec(theta);
        let mut v = dot(&ht, x) + dot(&self.c, x) + self.c_c + dot(&self.c_t, theta) + 0.5 * dot(theta, &self.q_t.matvec(theta));
        if let Some(q) = &self.q {
            v += 0.5 * dot(x, &q.matvec(x));
        }
        v
    }

    pub fn valid_parameter_realization(&self, theta: &[f64]) -> bool {
        (0..self.a_t.rows).all(|i| dot(self.a_t.row(i), theta) <= self.b_t[i])
    }

    /// Substitutes θ and solves the resulting LP/QP.
    pub fn solve_theta(&self, theta: &[f64]) -> Option<SolverOutput> {
        crate::census_purpose!(Init);
        if !self.valid_parameter_realization(theta) {
            return None;
        }
        if self.q.is_some() {
            crate::stats::qp();
        } else {
            crate::stats::lp(1);
        }
        self.solve_theta_uncounted(theta)
    }

    /// `solve_theta` without touching the PPOPT operation counters.
    pub fn solve_theta_uncounted(&self, theta: &[f64]) -> Option<SolverOutput> {
        if !self.valid_parameter_realization(theta) {
            return None;
        }
        let mut c = self.h.matvec(theta);
        for (ci, c0) in c.iter_mut().zip(&self.c) {
            *ci += c0;
        }
        let ft = self.f.matvec(theta);
        let b: Vec<f64> = self.b.iter().zip(&ft).map(|(x, y)| x + y).collect();
        let mut sol = match &self.q {
            Some(q) => {
                let fac = self.qp_factor.get_or_init(|| crate::solver::qp::QpFactor::with_equalities(q, &self.a, &self.equality_indices)).as_ref()?;
                crate::solver::qp::solve_qp_factored(fac, q, Some(&c), &self.a, &b, &self.equality_indices)?.into()
            }
            None => self.solver.solve_lp(Some(&c), &self.a, &b, &self.equality_indices)?,
        };
        sol.obj += self.c_c + dot(&self.c_t, theta) + 0.5 * dot(theta, &self.q_t.matvec(theta));
        Some(sol)
    }

    /// Active set of the substituted problem at θ (what the graph initialisation and the geometric
    /// facet step use), skipping multipliers and slacks. Not counted (callers count).
    pub fn active_set_at(&self, theta: &[f64]) -> Option<Vec<usize>> {
        if !self.valid_parameter_realization(theta) {
            return None;
        }
        match &self.q {
            Some(q) => {
                let mut c = self.h.matvec(theta);
                for (ci, c0) in c.iter_mut().zip(&self.c) {
                    *ci += c0;
                }
                let mut b = self.f.matvec(theta);
                for (bi, b0) in b.iter_mut().zip(&self.b) {
                    *bi += b0;
                }
                let fac = self.qp_factor.get_or_init(|| crate::solver::qp::QpFactor::with_equalities(q, &self.a, &self.equality_indices)).as_ref()?;
                crate::solver::qp::solve_qp_active_set(fac, q, Some(&c), &self.a, &b, &self.equality_indices).map(|s| s.active_set)
            }
            None => self.solve_theta_uncounted(theta).map(|s| s.active_set),
        }
    }

    /// Leaves θ as a variable: min c'x over the (x, θ) polytope.
    pub fn solve_theta_variable(&self) -> Option<SolverOutput> {
        crate::census_purpose!(Init);
        let (pa, pb) = self.xtheta_polytope();
        let mut c = self.c.clone();
        c.extend(std::iter::repeat(0.0).take(self.num_t()));
        self.solver.solve_lp(Some(&c), &pa, &pb, &self.equality_indices)
    }

    /// Schur-complement data of a strictly convex mpQP (None for mpLPs).
    pub fn kkt_cache_pub(&self) -> Option<&KktCache> {
        self.kkt_cache()
    }

    fn kkt_cache(&self) -> Option<&KktCache> {
        self.kkt
            .get_or_init(|| {
                let q = self.q.as_ref()?;
                let chol = Cholesky::new(q)?;
                let qi_h = chol.solve_mat(&self.h);
                let qi_c = chol.solve(&self.c);
                let qi_at = chol.solve_mat(&self.a.transpose()); // n_x x m
                let a_qi = qi_at.transpose();
                let mut m = self.a.matmul(&qi_at);
                // exact symmetry so that row i of M can stand in for column i
                for i in 0..m.rows {
                    for j in 0..i {
                        let v = 0.5 * (m[(i, j)] + m[(j, i)]);
                        m[(i, j)] = v;
                        m[(j, i)] = v;
                    }
                }
                let a_qi_h = self.a.matmul(&qi_h);
                let a_qi_c = self.a.matvec(&qi_c);
                let base_e = a_qi_h.add(&self.f).neg();
                let base_f: Vec<f64> = self.b.iter().zip(&a_qi_c).map(|(b, q)| b + q).collect();
                Some(KktCache { qi_h, qi_c, a_qi, m, a_qi_h, a_qi_c, base_e, base_f })
            })
            .as_ref()
    }

    /// Optimal x*(θ), λ*(θ) for an active set (PPOPT `optimal_control_law`).
    pub fn optimal_control_law(&self, active_set: &[usize]) -> Option<ControlLaw> {
        let mut law = self.optimal_control_law_dual(active_set)?;
        self.complete_primal_law(active_set, &mut law);
        Some(law)
    }

    /// x*(θ) from λ*(θ) for a law built by `optimal_control_law_dual` (no-op if already present).
    pub fn complete_primal_law(&self, active_set: &[usize], law: &mut ControlLaw) {
        if law.b_x.len() == self.num_x() {
            return;
        }
        let nx = self.num_x();
        let nt = self.num_t();
        let kc = self.kkt_cache().expect("primal completion needs the KKT cache");
        // K = -QiH - (AQi_AS)' L, k = -Qic - (AQi_AS)' l
        let mut a_x = kc.qi_h.neg();
        let mut b_x: Vec<f64> = kc.qi_c.iter().map(|v| -v).collect();
        for (ii, &i) in active_set.iter().enumerate() {
            let aq = kc.a_qi.row(i);
            let lrow = law.a_l.row(ii);
            for r in 0..nx {
                let w = aq[r];
                if w == 0.0 {
                    continue;
                }
                for (o, l) in a_x.data[r * nt..(r + 1) * nt].iter_mut().zip(lrow) {
                    *o -= w * l;
                }
                b_x[r] -= w * law.b_l[ii];
            }
        }
        law.a_x = a_x;
        law.b_x = b_x;
    }

    /// The multiplier law λ*(θ) only (x*(θ) left empty when the Schur path is used); enough for
    /// the critical-region rows. `complete_primal_law` adds x*(θ).
    pub fn optimal_control_law_dual(&self, active_set: &[usize]) -> Option<ControlLaw> {
        self.optimal_control_law_dual_rank(active_set, true).map(|(law, _)| law)
    }

    /// `optimal_control_law_dual` plus whether the Schur complement S = A_AS Q^{-1} A_AS' was
    /// factored by Cholesky with all pivots above 1e-10 of its largest diagonal entry. S is positive
    /// definite exactly when A_AS has full row rank, and such pivots certify full rank far above
    /// numpy's matrix_rank tolerance, so the caller can skip the separate rank test.
    /// With `lu_fallback = false` a failed Cholesky returns None instead of trying LU.
    pub fn optimal_control_law_dual_rank(&self, active_set: &[usize], lu_fallback: bool) -> Option<(ControlLaw, bool)> {
        let nx = self.num_x();
        let nt = self.num_t();
        let k = active_set.len();
        match &self.q {
            Some(q) => {
                if let Some(kc) = self.kkt_cache() {
                    // Schur complement S = A_AS Q^{-1} A_AS'
                    let mut s = Mat::zeros(k, k);
                    for (ii, &i) in active_set.iter().enumerate() {
                        for (jj, &j) in active_set.iter().enumerate() {
                            s[(ii, jj)] = kc.m[(i, j)];
                        }
                    }
                    let mut certified = true;
                    let (a_l, b_l) = if k == 0 {
                        (Mat::zeros(0, nt), vec![])
                    } else {
                        // L = -S^{-1}(F_AS + AQiH_AS), l = -S^{-1}(b_AS + AQic_AS)
                        let mut rhs = Mat::zeros(k, nt + 1);
                        for (ii, &i) in active_set.iter().enumerate() {
                            for t in 0..nt {
                                rhs[(ii, t)] = -(self.f[(i, t)] + kc.a_qi_h[(i, t)]);
                            }
                            rhs[(ii, nt)] = -(self.b[i] + kc.a_qi_c[i]);
                        }
                        let smax = (0..k).map(|i| s[(i, i)]).fold(0.0f64, f64::max).max(1e-300);
                        let sol = match crate::linalg::Cholesky::new(&s) {
                            Some(ch) if (0..k).all(|i| ch.l[i * k + i] * ch.l[i * k + i] > 1e-10 * smax) => ch.solve_mat_inplace(rhs),
                            _ => {
                                if !lu_fallback {
                                    return None;
                                }
                                certified = false;
                                Lu::new(&s)?.solve_mat(&rhs)
                            }
                        };
                        let mut a_l = Mat::zeros(k, nt);
                        let mut b_l = vec![0.0; k];
                        for ii in 0..k {
                            a_l.row_mut(ii).copy_from_slice(&sol.row(ii)[..nt]);
                            b_l[ii] = sol[(ii, nt)];
                        }
                        (a_l, b_l)
                    };
                    let _ = nx;
                    Some((ControlLaw { a_x: Mat::zeros(0, 0), b_x: vec![], a_l, b_l }, certified))
                } else {
                    // general KKT solve [A_AS 0; Q A_AS'] [x; λ] = [b_AS + F_AS θ; -c - Hθ]
                    let a_hat = self.a.select_rows(active_set);
                    let dim = k + nx;
                    let mut mm = Mat::zeros(dim, dim);
                    for i in 0..k {
                        for j in 0..nx {
                            mm[(i, j)] = a_hat[(i, j)];
                        }
                    }
                    for i in 0..nx {
                        for j in 0..nx {
                            mm[(k + i, j)] = q[(i, j)];
                        }
                        for j in 0..k {
                            mm[(k + i, nx + j)] = a_hat[(j, i)];
                        }
                    }
                    let lu = Lu::new(&mm)?;
                    let mut rhs = Mat::zeros(dim, nt + 1);
                    for (ii, &i) in active_set.iter().enumerate() {
                        for t in 0..nt {
                            rhs[(ii, t)] = self.f[(i, t)];
                        }
                        rhs[(ii, nt)] = self.b[i];
                    }
                    for r in 0..nx {
                        for t in 0..nt {
                            rhs[(k + r, t)] = -self.h[(r, t)];
                        }
                        rhs[(k + r, nt)] = -self.c[r];
                    }
                    let sol = lu.solve_mat(&rhs);
                    let mut a_x = Mat::zeros(nx, nt);
                    let mut b_x = vec![0.0; nx];
                    let mut a_l = Mat::zeros(k, nt);
                    let mut b_l = vec![0.0; k];
                    for r in 0..nx {
                        for t in 0..nt {
                            a_x[(r, t)] = sol[(r, t)];
                        }
                        b_x[r] = sol[(r, nt)];
                    }
                    for r in 0..k {
                        for t in 0..nt {
                            a_l[(r, t)] = sol[(nx + r, t)];
                        }
                        b_l[r] = sol[(nx + r, nt)];
                    }
                    Some((ControlLaw { a_x, b_x, a_l, b_l }, false))
                }
            }
            None => {
                // mpLP: x = A_AS^+ (b + Fθ), λ = -A_AS^{+T} (c + Hθ)
                let a_hat = self.a.select_rows(active_set);
                let aux = if k == nx {
                    match Lu::new(&a_hat) {
                        Some(lu) => lu.solve_mat(&Mat::eye(nx)),
                        None => pinv(&a_hat),
                    }
                } else {
                    pinv_full_row_rank(&a_hat).unwrap_or_else(|| pinv(&a_hat))
                };
                let f_as = self.f.select_rows(active_set);
                let b_as: Vec<f64> = active_set.iter().map(|&i| self.b[i]).collect();
                let a_x = aux.matmul(&f_as);
                let b_x = aux.matvec(&b_as);
                let aux_t = aux.transpose();
                let a_l = aux_t.matmul(&self.h).neg();
                let b_l: Vec<f64> = aux_t.matvec(&self.c).iter().map(|v| -v).collect();
                Some((ControlLaw { a_x, b_x, a_l, b_l }, false))
            }
        }
    }

    pub fn check_active_set_rank(&self, active_set: &[usize]) -> bool {
        is_full_rank_rows(&self.a, active_set)
    }

    /// Is there an (x, θ) with the active set at equality? (PPOPT `check_feasibility`)
    pub fn check_feasibility(&self, active_set: &[usize], check_rank: bool) -> bool {
        crate::census_purpose!(Feasibility);
        if check_rank {
            crate::stats::rank();
            if !is_full_rank_rows(&self.a, active_set) {
                return false;
            }
        }
        crate::stats::lp(1);
        let (pa, pb) = self.xtheta_cached();
        crate::solver::lp::lp_feasible(pa, pb, active_set)
    }

    /// `check_feasibility` that also returns a feasible (x, θ) point; the LP starts from `x0`.
    pub fn check_feasibility_from(&self, active_set: &[usize], x0: Option<&[f64]>) -> Option<Vec<f64>> {
        crate::census_purpose!(Feasibility);
        crate::stats::rank();
        if !is_full_rank_rows(&self.a, active_set) {
            return None;
        }
        crate::stats::lp(1);
        let (pa, pb) = self.xtheta_cached();
        crate::solver::lp::lp_feasible_point(pa, pb, active_set, x0)
    }

    /// Rows of the (unreduced) critical region of an active set given its control law:
    /// λ rows, inactive-constraint rows, then the parametric constraints. Returns (E, f, n_lambda, inactive list).
    pub fn region_rows(&self, active_set: &[usize], law: &ControlLaw) -> (Mat, Vec<f64>, usize, Vec<usize>) {
        let neq = self.num_equality_constraints();
        let nt = self.num_t();
        let m = self.num_constraints();
        let k = active_set.len();
        let mut in_as = vec![false; m];
        for &i in active_set {
            in_as[i] = true;
        }
        let inactive: Vec<usize> = (0..m).filter(|&i| !in_as[i]).collect();
        let n_lambda = k.saturating_sub(neq);
        let rows = n_lambda + inactive.len() + self.a_t.rows;
        let mut e = Mat::zeros(rows, nt);
        let mut f = Vec::with_capacity(rows);
        // λ >= 0  ->  -A_l θ <= b_l
        for ii in neq..k {
            let r = ii - neq;
            for t in 0..nt {
                e[(r, t)] = -law.a_l[(ii, t)];
            }
            f.push(law.b_l[ii]);
        }
        // inactive constraints: (A_I K - F_I) θ <= b_I - A_I k
        let cache = if self.q.is_some() { self.kkt_cache() } else { None };
        let fused = cache.is_some() && (2..=4).contains(&nt);
        if let (true, Some(kc)) = (fused, cache) {
            // rows j: -(AQiH + F)_j - sum_i M[i, j] L_i ;  rhs: (b + AQic)_j + sum_i M[i, j] l_i
            f.resize(n_lambda + inactive.len(), 0.0);
            match nt {
                2 => inactive_fused::<2>(&mut e.data, &mut f, n_lambda, &inactive, active_set, kc, law),
                3 => inactive_fused::<3>(&mut e.data, &mut f, n_lambda, &inactive, active_set, kc, law),
                _ => inactive_fused::<4>(&mut e.data, &mut f, n_lambda, &inactive, active_set, kc, law),
            }
        } else if let Some(kc) = cache {
            // rows j: -(AQiH + F)_j - sum_i M[i, j] L_i ;  rhs: (b + AQic)_j + sum_i M[i, j] l_i
            // (M symmetric), streaming the contiguous row M[i, :] for each active i
            for (jj, &j) in inactive.iter().enumerate() {
                let r = n_lambda + jj;
                e.data[r * nt..(r + 1) * nt].copy_from_slice(kc.base_e.row(j));
                f.push(kc.base_f[j]);
            }
            match nt {
                _ => {
                    for (ii, &i) in active_set.iter().enumerate() {
                        let mrow = kc.m.row(i);
                        let lrow = law.a_l.row(ii);
                        let lb = law.b_l[ii];
                        for (jj, &j) in inactive.iter().enumerate() {
                            let mij = mrow[j];
                            if mij == 0.0 {
                                continue;
                            }
                            let r = n_lambda + jj;
                            for (o, l) in e.data[r * nt..(r + 1) * nt].iter_mut().zip(lrow) {
                                *o -= mij * l;
                            }
                            f[r] += mij * lb;
                        }
                    }
                }
            }
        }
        for (jj, &j) in inactive.iter().enumerate() {
            if cache.is_some() {
                break;
            }
            let r = n_lambda + jj;
            let arow = self.a.row(j);
            if let Some(kc) = cache {
                // A_j K = -AQiH_j - M[j, AS] L ;  A_j k = -AQic_j - M[j, AS] l, written in place
                let row = &mut e.data[r * nt..(r + 1) * nt];
                let aqh = kc.a_qi_h.row(j);
                let fj = self.f.row(j);
                for ((o, q), fv) in row.iter_mut().zip(aqh).zip(fj) {
                    *o = -q - fv;
                }
                let mrow = kc.m.row(j);
                let mut akc = -kc.a_qi_c[j];
                for (ii, &i) in active_set.iter().enumerate() {
                    let mji = mrow[i];
                    if mji == 0.0 {
                        continue;
                    }
                    for (o, l) in row.iter_mut().zip(law.a_l.row(ii)) {
                        *o -= mji * l;
                    }
                    akc -= mji * law.b_l[ii];
                }
                f.push(self.b[j] - akc);
            } else {
                for t in 0..nt {
                    let mut s = 0.0;
                    for (xi, &av) in arow.iter().enumerate() {
                        s += av * law.a_x[(xi, t)];
                    }
                    e[(r, t)] = s - self.f[(j, t)];
                }
                f.push(self.b[j] - dot(arow, &law.b_x));
            }
        }
        for i in 0..self.a_t.rows {
            e.row_mut(n_lambda + inactive.len() + i).copy_from_slice(self.a_t.row(i));
            f.push(self.b_t[i]);
        }
        (e, f, n_lambda, inactive)
    }

    /// θ-space form of `check_optimality`: with LICQ, x and λ are affine in θ, so the KKT system
    /// is feasible iff the unreduced critical region is nonempty. Same answer as the big LP below.
    pub fn check_optimality_fast(&self, active_set: &[usize]) -> bool {
        crate::census_purpose!(Feasibility);
        if !self.is_qp() && active_set.len() != self.num_x() {
            return false;
        }
        crate::stats::lp(1);
        let Some(law) = self.optimal_control_law(active_set) else { return false };
        let (e, f, _, _) = self.region_rows(active_set, &law);
        crate::solver::lp::lp_feasible(&e, &f, &[])
    }

    /// PPOPT's `check_optimality` LP over (x, θ, λ, s, t).
    pub fn check_optimality(&self, active_set: &[usize]) -> Option<OptimalityCertificate> {
        if !self.is_qp() && active_set.len() != self.num_x() {
            return None;
        }
        let nx = self.num_x();
        let nt = self.num_t();
        let m = self.num_constraints();
        let na = active_set.len();
        let neq = self.num_equality_constraints();
        let nact = na.saturating_sub(neq);
        let mut in_as = vec![false; m];
        for &i in active_set {
            in_as[i] = true;
        }
        let inactive: Vec<usize> = (0..m).filter(|&i| !in_as[i]).collect();
        let ni = inactive.len();
        let nv = nx + nt + na + ni + 1;
        let (ox, ot, ol, os, otv) = (0, nx, nx + nt, nx + nt + na, nx + nt + na + ni);
        let mut a = Mat::zeros(0, nv);
        let mut b = vec![];
        // 1) Qx + Hθ + A_AS'λ = -c
        for r in 0..nx {
            let mut row = vec![0.0; nv];
            if let Some(q) = &self.q {
                for j in 0..nx {
                    row[ox + j] = q[(r, j)];
                }
            }
            for t in 0..nt {
                row[ot + t] = self.h[(r, t)];
            }
            for (ii, &i) in active_set.iter().enumerate() {
                row[ol + ii] = self.a[(i, r)];
            }
            a.push_row(&row);
            b.push(-self.c[r]);
        }
        // 2) A_AS x - F_AS θ = b_AS
        for &i in active_set {
            let mut row = vec![0.0; nv];
            row[ox..ox + nx].copy_from_slice(self.a.row(i));
            for t in 0..nt {
                row[ot + t] = -self.f[(i, t)];
            }
            a.push_row(&row);
            b.push(self.b[i]);
        }
        // 3) A_I x - F_I θ + s = b_I
        for (jj, &j) in inactive.iter().enumerate() {
            let mut row = vec![0.0; nv];
            row[ox..ox + nx].copy_from_slice(self.a.row(j));
            for t in 0..nt {
                row[ot + t] = -self.f[(j, t)];
            }
            row[os + jj] = 1.0;
            a.push_row(&row);
            b.push(self.b[j]);
        }
        let n_equalities = a.rows;
        // 4) t <= λ_activated ; 7) λ_activated >= 0
        for ii in neq..na {
            let mut row = vec![0.0; nv];
            row[ol + ii] = -1.0;
            row[otv] = 1.0;
            a.push_row(&row);
            b.push(0.0);
        }
        // 5) t <= s
        for jj in 0..ni {
            let mut row = vec![0.0; nv];
            row[os + jj] = -1.0;
            row[otv] = 1.0;
            a.push_row(&row);
            b.push(0.0);
        }
        // 6) t >= 0
        let mut row = vec![0.0; nv];
        row[otv] = -1.0;
        a.push_row(&row);
        b.push(0.0);
        for ii in neq..na {
            let mut row = vec![0.0; nv];
            row[ol + ii] = -1.0;
            a.push_row(&row);
            b.push(0.0);
        }
        // 8) s >= 0
        for jj in 0..ni {
            let mut row = vec![0.0; nv];
            row[os + jj] = -1.0;
            a.push_row(&row);
            b.push(0.0);
        }
        // 9) A_t θ <= b_t
        for i in 0..self.a_t.rows {
            let mut row = vec![0.0; nv];
            row[ot..ot + nt].copy_from_slice(self.a_t.row(i));
            a.push_row(&row);
            b.push(self.b_t[i]);
        }
        let _ = nact;
        let mut c = vec![0.0; nv];
        c[otv] = -1.0;
        let eqs: Vec<usize> = (0..n_equalities).collect();
        let sol = self.solver.solve_lp(Some(&c), &a, &b, &eqs)?;
        Some(OptimalityCertificate {
            x: sol.sol[ox..ot].to_vec(),
            theta: sol.sol[ot..ol].to_vec(),
            lambda: sol.sol[ol..os].to_vec(),
            slack: sol.sol[os..otv].to_vec(),
            t: sol.sol[otv],
            equality_indices: active_set.to_vec(),
        })
    }

    /// Chebyshev ball of the (x, θ) feasible space.
    pub fn feasible_space_chebychev_ball(&self) -> Option<crate::solver::lp::LpSolution> {
        crate::census_purpose!(Init);
        crate::stats::lp(1);
        let (pa, pb) = self.xtheta_polytope();
        chebyshev_ball(&pa, &pb, &self.equality_indices).or_else(|| chebyshev_ball_interior_point(&pa, &pb, &self.equality_indices))
    }

    pub fn feasible_theta_point(&self) -> Option<Vec<f64>> {
        let s = self.feasible_space_chebychev_ball()?;
        let nx = self.num_x();
        Some(s.x[nx..nx + self.num_t()].to_vec())
    }

    /// Samples around the Chebyshev center until an optimal active set of size <= n_x is found.
    pub fn gen_optimal_active_set<R: Rng>(&self, rng: &mut R) -> Option<Vec<usize>> {
        let s = self.feasible_space_chebychev_ball()?;
        let nx = self.num_x();
        let nt = self.num_t();
        let center = &s.x[nx..nx + nt];
        let radius = s.x[s.x.len() - 1];
        for _ in 0..500 {
            let test: Vec<f64> = center.iter().map(|c| c + rng.gen_range(-radius..=radius)).collect();
            if let Some(sol) = self.solve_theta(&test) {
                // a start whose active rows are dependent (duplicate or parallel constraints binding
                // together) has no multiplier law, so its region cannot be built
                if sol.active_set.len() <= nx && crate::linalg::is_full_rank_rows(&self.a, &sol.active_set) {
                    return Some(sol.active_set);
                }
            }
        }
        // With many parameters the cube [c - r, c + r]^p lies almost entirely outside the feasible
        // set (the inscribed ball fills about 4e-6 of it at p = 16), so the samples above all miss.
        // Fall back to the centre and to points inside the ball, which are feasible by construction.
        ball_points(center, radius, rng).find_map(|test| {
            let sol = self.solve_theta(&test)?;
            (sol.active_set.len() <= nx && crate::linalg::is_full_rank_rows(&self.a, &sol.active_set)).then_some(sol.active_set)
        })
    }

    /// Random walk in θ (PPOPT `sample_theta_space`); returns the distinct active sets found.
    pub fn sample_theta_space<R: Rng>(&self, num_samples: usize, rng: &mut R) -> Vec<Vec<usize>> {
        crate::census_purpose!(Init);
        let Some(s) = self.feasible_space_chebychev_ball() else { return vec![] };
        let nx = self.num_x();
        let nt = self.num_t();
        let mut theta = s.x[nx..nx + nt].to_vec();
        let radius = s.x[s.x.len() - 1];
        let mut found: Vec<Vec<usize>> = vec![];
        for _ in 0..num_samples {
            let dir: Vec<f64> = (0..nt).map(|_| standard_normal(rng)).collect();
            let dn = crate::linalg::norm(&dir).max(1e-300);
            let step = rng.r#gen::<f64>() * radius;
            let new_theta: Vec<f64> = theta.iter().zip(&dir).map(|(t, d)| t + step * d / dn).collect();
            if self.valid_parameter_realization(&new_theta) {
                if self.q.is_some() {
                    crate::stats::qp();
                } else {
                    crate::stats::lp(1);
                }
            }
            if let Some(active) = self.active_set_at(&new_theta) {
                if !found.contains(&active) {
                    found.push(active);
                }
                theta = new_theta;
            }
        }
        found
    }

    /// LaTeX rendering of the program (PPOPT `latex`).
    pub fn latex(&self) -> Vec<String> {
        let x: Vec<String> = (0..self.num_x()).map(|i| format!("x_{{{i}}}")).collect();
        let th: Vec<String> = (0..self.num_t()).map(|i| format!("\\theta_{{{i}}}")).collect();
        let xl = latex_vector(&x);
        let tl = latex_vector(&th);
        let cvec = Mat::col_vec(&self.c);
        let mut out = vec![];
        let added = if self.h.max_abs() > 0.0 { format!(" + {tl}^{{T}}{}^{{T}}{xl}", latex_matrix(&self.h)) } else { String::new() };
        match &self.q {
            Some(q) => out.push(format!("$$\\min_{{x}} \\frac{{1}}{{2}}{xl}^{{T}}{}{xl}+{}^T{xl}{added}$$", latex_matrix(q), latex_matrix(&cvec))),
            None => out.push(format!("$$\\min_{{x}}{}^T{xl}{added}$$", latex_matrix(&cvec))),
        }
        let ineq = select_not_in(self.num_constraints(), &self.equality_indices);
        if !ineq.is_empty() {
            let bi: Vec<f64> = ineq.iter().map(|&i| self.b[i]).collect();
            out.push(format!(
                "$${}{xl}\\leq{}+{}{tl}$$",
                latex_matrix(&self.a.select_rows(&ineq)),
                latex_matrix(&Mat::col_vec(&bi)),
                latex_matrix(&self.f.select_rows(&ineq))
            ));
        }
        if !self.equality_indices.is_empty() {
            let be: Vec<f64> = self.equality_indices.iter().map(|&i| self.b[i]).collect();
            out.push(format!(
                "$${}{xl}={}+{}{tl}$$",
                latex_matrix(&self.a.select_rows(&self.equality_indices)),
                latex_matrix(&Mat::col_vec(&be)),
                latex_matrix(&self.f.select_rows(&self.equality_indices))
            ));
        }
        out.push(format!("$${}{tl}\\leq{}$$", latex_matrix(&self.a_t), latex_matrix(&Mat::col_vec(&self.b_t))));
        out
    }
}

/// One pass per inactive row for a fixed parameter dimension NT: base row minus the active-set
/// terms, accumulated in registers (M is small and stays in L1, so the M[i, j] reads are cheap).
#[inline]
fn inactive_fused<const NT: usize>(e: &mut [f64], f: &mut [f64], n_lambda: usize, inactive: &[usize], active_set: &[usize], kc: &KktCache, law: &ControlLaw) {
    let k = active_set.len();
    let mut lt = [[0.0f64; NT]; 64];
    let big = k > 64;
    if !big {
        for ii in 0..k {
            lt[ii].copy_from_slice(&law.a_l.row(ii)[..NT]);
        }
    }
    let m = kc.m.cols;
    let mdat = &kc.m.data;
    let e = &mut e[n_lambda * NT..(n_lambda + inactive.len()) * NT];
    let f = &mut f[n_lambda..n_lambda + inactive.len()];
    for ((er, fr), &j) in e.chunks_exact_mut(NT).zip(f.iter_mut()).zip(inactive) {
        let mut acc = [0.0f64; NT];
        acc.copy_from_slice(&kc.base_e.data[j * NT..(j + 1) * NT]);
        let mut fa = kc.base_f[j];
        for (ii, &i) in active_set.iter().enumerate() {
            let mij = mdat[i * m + j];
            let l: &[f64] = if big { &law.a_l.row(ii)[..NT] } else { &lt[ii] };
            for t in 0..NT {
                acc[t] -= mij * l[t];
            }
            fa += mij * law.b_l[ii];
        }
        er.copy_from_slice(&acc);
        *fr = fa;
    }
}

pub fn standard_normal<R: Rng>(rng: &mut R) -> f64 {
    // Box-Muller
    let u1: f64 = rng.r#gen::<f64>().max(1e-300);
    let u2: f64 = rng.r#gen::<f64>();
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

/// The centre, then 500 points drawn uniformly from the ball of radius `radius` around it (a
/// Gaussian direction scaled by radius·u^(1/p)). With x held at the centre, every point satisfies
/// every inequality row of the feasible set (Lean `chebyshev_ball_feasible`). Equality rows hold
/// only at the centre, so with equality constraints a sample can be infeasible; the QP solve that
/// follows rejects it.
pub(crate) fn ball_points<'a, R: Rng>(center: &'a [f64], radius: f64, rng: &'a mut R) -> impl Iterator<Item = Vec<f64>> + 'a {
    let p = center.len();
    std::iter::once(center.to_vec()).chain((0..500).map(move |_| {
        let dir: Vec<f64> = (0..p).map(|_| standard_normal(rng)).collect();
        let dn = crate::linalg::norm(&dir).max(1e-300);
        let s = radius * rng.r#gen::<f64>().powf(1.0 / p.max(1) as f64);
        center.iter().zip(&dir).map(|(c, d)| c + s * d / dn).collect()
    }))
}

/// Chebyshev ball of {x : A x <= b, A[eq] x = b[eq]} by the Clarabel interior-point solver, the
/// fallback when the dense simplex returns no verified point. On masses8_N2 (16 parameters) the
/// simplex stops at a basis whose point violates one row by 1.1e-8, which its 1e-9 check rejects,
/// so no solver found a start region. The point returned here is checked against every row with
/// a tolerance of 1e-9 (1 + |b|), and the radius is reduced by 1e-7 to leave a margin.
fn chebyshev_ball_interior_point(a: &Mat, b: &[f64], eq: &[usize]) -> Option<crate::solver::lp::LpSolution> {
    let (n, m) = (a.cols, a.rows);
    let mut lp = crate::solver::sparse_lp::SparseLp::new(n + 1);
    lp.c[n] = -1.0;
    for i in 0..m {
        let row = a.row(i);
        let mut terms: Vec<(usize, f64)> = row.iter().copied().enumerate().collect();
        if eq.contains(&i) {
            lp.add_eq(&terms, b[i]);
        } else {
            terms.push((n, crate::linalg::norm(row)));
            lp.add_le(&terms, b[i]);
        }
    }
    lp.add_le(&[(n, -1.0)], 0.0);
    let mut x = lp.solve()?;
    x[n] = (x[n] - 1e-7).max(0.0);
    for i in 0..m {
        let ax: f64 = a.row(i).iter().zip(&x[..n]).map(|(u, v)| u * v).sum();
        let tol = 1e-9 * (1.0 + b[i].abs());
        if (eq.contains(&i) && (ax - b[i]).abs() > tol) || ax > b[i] + tol {
            return None;
        }
    }
    let obj = -x[n];
    Some(crate::solver::lp::LpSolution { x, obj, dual: vec![], slack: vec![], active_set: vec![] })
}

/// Pseudo-inverse A' (A A')^-1 of a well-conditioned full-row-rank matrix (k < n) through a
/// Cholesky factorization of the Gram matrix, much cheaper than the SVD. `None` (use the SVD
/// pseudo-inverse) when A A' is not safely positive definite (dependent or nearly dependent rows).
fn pinv_full_row_rank(a: &Mat) -> Option<Mat> {
    let k = a.rows;
    if k == 0 || k > a.cols {
        return None;
    }
    let mut g = Mat::zeros(k, k);
    for i in 0..k {
        for j in 0..=i {
            let v = dot(a.row(i), a.row(j));
            g[(i, j)] = v;
            g[(j, i)] = v;
        }
    }
    let smax = (0..k).map(|i| g[(i, i)]).fold(0.0f64, f64::max);
    let ch = Cholesky::new(&g)?;
    if !(0..k).all(|i| ch.l[i * k + i] * ch.l[i * k + i] > 1e-8 * smax) {
        return None;
    }
    // X = G^-1 A, so that A' G^-1 = X'
    Some(ch.solve_mat_inplace(a.clone()).transpose())
}
