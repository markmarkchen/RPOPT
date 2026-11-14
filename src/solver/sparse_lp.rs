//! Sparse LP via the Clarabel interior-point solver. Used for the large but very sparse offline
//! construction LPs (for example the linear-machine weights), which the dense simplex is not meant
//! for.

use clarabel::algebra::CscMatrix;
use clarabel::solver::{DefaultSettingsBuilder, DefaultSolver, IPSolver, SolverStatus, SupportedConeT};

/// Sparse LP:  min c'x  s.t.  A_eq x = b_eq,  A_in x <= b_in. Matrices are given as triplets.
pub struct SparseLp {
    pub n: usize,
    pub c: Vec<f64>,
    pub eq: Vec<(usize, usize, f64)>,
    pub b_eq: Vec<f64>,
    pub ineq: Vec<(usize, usize, f64)>,
    pub b_in: Vec<f64>,
}

impl SparseLp {
    pub fn new(n: usize) -> Self {
        SparseLp { n, c: vec![0.0; n], eq: vec![], b_eq: vec![], ineq: vec![], b_in: vec![] }
    }

    /// Adds sum_k coef_k x_{idx_k} = rhs, returns the row index.
    pub fn add_eq(&mut self, terms: &[(usize, f64)], rhs: f64) -> usize {
        let r = self.b_eq.len();
        for &(j, v) in terms {
            if v != 0.0 {
                self.eq.push((r, j, v));
            }
        }
        self.b_eq.push(rhs);
        r
    }

    pub fn add_le(&mut self, terms: &[(usize, f64)], rhs: f64) -> usize {
        let r = self.b_in.len();
        for &(j, v) in terms {
            if v != 0.0 {
                self.ineq.push((r, j, v));
            }
        }
        self.b_in.push(rhs);
        r
    }

    /// Returns the primal solution, or None if Clarabel did not reach (almost) solved status.
    pub fn solve(&self) -> Option<Vec<f64>> {
        let meq = self.b_eq.len();
        let min = self.b_in.len();
        let mut ii = Vec::with_capacity(self.eq.len() + self.ineq.len());
        let mut jj = Vec::with_capacity(ii.capacity());
        let mut vv = Vec::with_capacity(ii.capacity());
        for &(r, c, v) in &self.eq {
            ii.push(r);
            jj.push(c);
            vv.push(v);
        }
        for &(r, c, v) in &self.ineq {
            ii.push(meq + r);
            jj.push(c);
            vv.push(v);
        }
        let a = CscMatrix::new_from_triplets(meq + min, self.n, ii, jj, vv);
        let p = CscMatrix::new_from_triplets(self.n, self.n, vec![], vec![], vec![]);
        let mut b = self.b_eq.clone();
        b.extend_from_slice(&self.b_in);
        let mut cones = vec![];
        if meq > 0 {
            cones.push(SupportedConeT::ZeroConeT(meq));
        }
        if min > 0 {
            cones.push(SupportedConeT::NonnegativeConeT(min));
        }
        let settings = DefaultSettingsBuilder::default()
            .verbose(false)
            .max_iter(400)
            .tol_gap_abs(1e-10)
            .tol_gap_rel(1e-10)
            .tol_feas(1e-10)
            .build()
            .ok()?;
        let mut solver = DefaultSolver::new(&p, &self.c, &a, &b, &cones, settings).ok()?;
        solver.solve();
        match solver.solution.status {
            SolverStatus::Solved | SolverStatus::AlmostSolved => Some(solver.solution.x.clone()),
            s => {
                if std::env::var("PPOPT_DEBUG").is_ok() {
                    eprintln!("clarabel status {:?} after {} iterations", s, solver.info.iterations);
                }
                None
            }
        }
    }
}
