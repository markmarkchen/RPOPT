//! Port of `ppopt.utils.mpqp_utils`: critical region construction from an active set.

use crate::critical_region::CriticalRegion;
use crate::linalg::Mat;
use crate::program::MpProgram;
use crate::utils::{chebyshev_ball, numerically_nonzero_rows, remove_duplicate_rows};

/// Control law and unreduced critical-region rows of an active set, plus (optionally) a point of
/// the region from a feasibility LP. Shared by the optimality test and region construction so the
/// law, the rows and the LP are computed once.
pub struct KktRegion {
    pub law: crate::program::ControlLaw,
    pub e: Mat,
    pub f: Vec<f64>,
    pub n_lambda: usize,
    pub inactive: Vec<usize>,
    pub point: Option<Vec<f64>>,
}

pub fn kkt_region(program: &MpProgram, active_set: &[usize]) -> Option<KktRegion> {
    let law = program.optimal_control_law_dual(active_set)?;
    Some(kkt_region_from_law(program, active_set, law))
}

pub fn kkt_region_from_law(program: &MpProgram, active_set: &[usize], law: crate::program::ControlLaw) -> KktRegion {
    let (e, f, n_lambda, inactive) = program.region_rows(active_set, &law);
    KktRegion { law, e, f, n_lambda, inactive, point: None }
}

impl KktRegion {
    /// Is the unreduced region nonempty? Stores the point found.
    pub fn nonempty(&mut self) -> bool {
        self.point = crate::solver::lp::lp_feasible_point(&self.e, &self.f, &[], None);
        self.point.is_some()
    }

    /// `nonempty` on the rows that can bind inside the parameter box (same answer, smaller LP).
    pub fn nonempty_in(&mut self, program: &MpProgram) -> bool {
        match program.box_reduced(&self.e, &self.f) {
            Some((e, f)) => {
                self.point = crate::solver::lp::lp_feasible_point(&e, &f, &[], None);
                self.point.is_some()
            }
            None => self.nonempty(),
        }
    }
}

/// Builds the critical region of an active set; `None` if empty or (with `check_full_dim`) lower
/// dimensional. Same row classification and redundancy test as PPOPT.
pub fn gen_cr_from_active_set(program: &MpProgram, active_set: &[usize], check_full_dim: bool) -> Option<CriticalRegion> {
    if program.num_t() == 1 {
        return gen_cr_from_active_set_1d(program, active_set, check_full_dim);
    }
    let kkt = kkt_region(program, active_set)?;
    gen_cr_from_kkt(program, active_set, kkt, check_full_dim)
}

/// `gen_cr_from_active_set` that first tests the λ >= 0 rows and the parameter-space rows alone,
/// filtered and normalized exactly as in `gen_cr_from_kkt` (so they are bitwise the same rows of
/// the full system). An empty subsystem (Farkas-certified by the LP) makes the region empty, so
/// the inactive-constraint rows, the primal law and the full LP are only computed for active
/// sets that pass. Same result; the PPOPT-equivalent Chebyshev LP is counted as before. In the
/// combinatorial algorithm almost every feasible active set is rejected here.
pub fn gen_cr_from_active_set_lambda_first(program: &MpProgram, active_set: &[usize], check_full_dim: bool) -> Option<CriticalRegion> {
    if program.num_t() == 1 {
        return gen_cr_from_active_set_1d(program, active_set, check_full_dim);
    }
    let law = program.optimal_control_law_dual(active_set)?;
    gen_cr_from_law_lambda_first(program, active_set, law, check_full_dim)
}

/// `gen_cr_from_active_set_lambda_first` given the law `optimal_control_law_dual(active_set)`
/// (two or more parameters).
pub fn gen_cr_from_law_lambda_first(program: &MpProgram, active_set: &[usize], law: crate::program::ControlLaw, check_full_dim: bool) -> Option<CriticalRegion> {
    if lambda_omega_feasible(program, &law) != Some(true) {
        if check_full_dim {
            crate::stats::lp(1); // PPOPT's Chebyshev LP (it has no zero-row shortcut)
        }
        return None;
    }
    let kkt = kkt_region_from_law(program, active_set, law);
    gen_cr_from_kkt(program, active_set, kkt, check_full_dim)
}

/// Feasibility of the λ >= 0 and parameter-space rows of a critical region (see
/// `gen_cr_from_active_set_lambda_first`); None when one of them is a numerically zero row with a
/// negative right hand side (the full construction returns None there before its LP).
fn lambda_omega_feasible(program: &MpProgram, law: &crate::program::ControlLaw) -> Option<bool> {
    thread_local! {
        static BUF: std::cell::RefCell<(Mat, Vec<f64>)> = std::cell::RefCell::new((Mat::zeros(0, 0), vec![]));
    }
    let neq = program.num_equality_constraints();
    let nt = program.num_t();
    let k = law.a_l.rows;
    BUF.with(|cell| {
        let mut guard = cell.borrow_mut();
        let (e, f) = &mut *guard;
        e.cols = nt;
        e.rows = 0;
        e.data.clear();
        f.clear();
        let mut push = |row: &mut dyn Iterator<Item = f64>, rhs: f64| -> bool {
            let start = e.data.len();
            e.data.extend(row);
            let r = &mut e.data[start..];
            if r.iter().all(|v| v.abs() <= 1e-8) {
                e.data.truncate(start);
                return rhs >= -1e-8;
            }
            let nrm = crate::linalg::norm(r);
            for v in r.iter_mut() {
                *v /= nrm;
            }
            f.push(rhs / nrm);
            e.rows += 1;
            true
        };
        for ii in neq..k {
            if !push(&mut law.a_l.row(ii).iter().map(|v| -v), law.b_l[ii]) {
                return None;
            }
        }
        for i in 0..program.a_t.rows {
            if !push(&mut program.a_t.row(i).iter().copied(), program.b_t[i]) {
                return None;
            }
        }
        Some(crate::solver::lp::lp_feasible(e, f, &[]))
    })
}

/// Feasibility of the λ >= 0 rows and the parameter-space rows of the unreduced region, as the same
/// raw rows (no filtering or scaling) that `MpProgram::region_rows` puts first and last. An
/// infeasible subsystem makes `KktRegion::nonempty` false.
pub fn lambda_rows_feasible_raw(program: &MpProgram, a_l: &Mat, b_l: &[f64]) -> bool {
    thread_local! {
        static BUF: std::cell::RefCell<(Mat, Vec<f64>)> = std::cell::RefCell::new((Mat::zeros(0, 0), vec![]));
    }
    let neq = program.num_equality_constraints();
    BUF.with(|cell| {
        let mut guard = cell.borrow_mut();
        let (e, f) = &mut *guard;
        e.cols = program.num_t();
        e.rows = 0;
        e.data.clear();
        f.clear();
        for ii in neq..a_l.rows {
            e.data.extend(a_l.row(ii).iter().map(|v| -v));
            f.push(b_l[ii]);
            e.rows += 1;
        }
        for i in 0..program.a_t.rows {
            e.data.extend_from_slice(program.a_t.row(i));
            f.push(program.b_t[i]);
            e.rows += 1;
        }
        crate::solver::lp::lp_feasible(e, f, &[])
    })
}

/// `gen_cr_from_active_set` from precomputed law and rows (and a known region point if any).
pub fn gen_cr_from_kkt(program: &MpProgram, active_set: &[usize], kkt: KktRegion, check_full_dim: bool) -> Option<CriticalRegion> {
    if program.num_t() == 1 {
        return gen_cr_from_active_set_1d(program, active_set, check_full_dim);
    }
    let neq = program.num_equality_constraints();
    let KktRegion { mut law, e: e_all, f: f_all, n_lambda, inactive, point } = kkt;
    program.complete_primal_law(active_set, &mut law);
    let n_inactive = inactive.len();
    let n_omega = program.a_t.rows;
    let nt = program.num_t();

    let kept_rows = numerically_nonzero_rows(&e_all);
    // position of each original row in the reduced system
    let mut pos = vec![usize::MAX; e_all.rows];
    for (p, &r) in kept_rows.iter().enumerate() {
        pos[r] = p;
    }
    let mut e = e_all.select_rows(&kept_rows);
    let mut f: Vec<f64> = kept_rows.iter().map(|&r| f_all[r]).collect();
    for i in 0..e.rows {
        let nrm = crate::linalg::norm(e.row(i));
        for v in e.row_mut(i) {
            *v /= nrm;
        }
        f[i] /= nrm;
    }
    // PPOPT's Chebyshev LP (counted before the zero-row shortcut, which PPOPT does not have)
    if check_full_dim {
        crate::stats::lp(1);
    }
    // a numerically zero row with a negative right hand side makes the region empty
    for r in 0..e_all.rows {
        if pos[r] == usize::MAX && f_all[r] < -1e-8 {
            return None;
        }
    }

    // Chebyshev ball of the region: PPOPT computes it only for the full-dimension check; here it
    // is always computed because its center starts the row tests below
    let mut center: Option<Vec<f64>> = None;
    let mut cheb_radius: Option<f64> = None;
    // an empty region is detected faster by a plain feasibility LP (no phase 1); a nonempty one
    // starts the Chebyshev LP at the feasible point found
    let start = match point {
        Some(p) => p,
        None => crate::solver::lp::lp_feasible_point(&e, &f, &[], None)?,
    };
    let mut start_r = start;
    start_r.push(0.0);
    match chebyshev_ball_from(&e, &f, &start_r) {
        Some(s) => {
            if check_full_dim && s.x[nt] <= 1e-8 {
                return None;
            }
            cheb_radius = Some(s.x[nt]);
            center = Some(s.x[..nt].to_vec());
        }
        None => {
            if check_full_dim {
                return None;
            }
        }
    }

    crate::stats::lp(kept_rows.len() as u64); // PPOPT: one LP per nonzero row
    let mut kept_lambda = vec![];
    let mut kept_ineq = vec![];
    let mut kept_omega = vec![];
    let mut non_redundant = vec![];
    // PPOPT keeps row p iff {Eθ <= f, row p at equality} is feasible (one LP per row). The same
    // answer is usually available without an LP: a row whose maximum over the region's bounding
    // box is below its rhs cannot be reached, and a row whose hyperplane contains a feasible point
    // (the projection of the Chebyshev center) is reached. Only the remaining rows get an LP,
    // started at the center.
    let bbox = center.as_ref().and_then(|c| region_bbox(&e, &f, c));
    let test = |p: usize| -> bool {
        let Some(c) = &center else { return crate::solver::lp::lp_feasible(&e, &f, &[p]) };
        let a = e.row(p);
        let fp = f[p];
        let tol = 1e-9 * (1.0 + fp.abs());
        if let Some((lo, hi)) = &bbox {
            let ub: f64 = a.iter().zip(lo.iter().zip(hi)).map(|(ak, (l, h))| (ak * l).max(ak * h)).sum();
            if ub < fp - 1e-7 * (1.0 + fp.abs()) {
                return false;
            }
        }
        // projection of the center onto the hyperplane of row p (rows are unit norm)
        let s = fp - crate::linalg::dot(a, c);
        let q: Vec<f64> = c.iter().zip(a).map(|(ci, ai)| ci + s * ai).collect();
        let inside = (0..e.rows).all(|i| i == p || crate::linalg::dot(e.row(i), &q) <= f[i] + 1e-9 * (1.0 + f[i].abs()));
        if inside && (crate::linalg::dot(a, &q) - fp).abs() <= tol {
            return true;
        }
        crate::solver::lp::lp_feasible_from(&e, &f, &[p], c)
    };
    for idx in 0..n_lambda {
        let p = pos[idx];
        if p != usize::MAX && test(p) {
            kept_lambda.push(idx);
            non_redundant.push(p);
        }
    }
    for idx in 0..n_inactive {
        let p = pos[n_lambda + idx];
        if p != usize::MAX && test(p) {
            kept_ineq.push(idx);
            non_redundant.push(p);
        }
    }
    for idx in 0..n_omega {
        let p = pos[n_lambda + n_inactive + idx];
        if p != usize::MAX && test(p) {
            kept_omega.push(idx);
            non_redundant.push(p);
        }
    }
    let active: Vec<usize> = active_set[neq..].to_vec();
    let relevant_lambda: Vec<usize> = kept_lambda.iter().map(|&i| active[i]).collect();
    let real_regular: Vec<usize> = kept_ineq.iter().map(|&i| inactive[i]).collect();
    let e_red = e.select_rows(&non_redundant);
    let f_red: Vec<f64> = non_redundant.iter().map(|&p| f[p]).collect();
    let (e_red, f_red) = remove_duplicate_rows(&e_red, &f_red);
    crate::stats::region_built(active_set);
    Some(CriticalRegion {
        a: law.a_x,
        b: law.b_x,
        c: law.a_l,
        d: law.b_l,
        e: e_red,
        f: f_red,
        active_set: active_set.to_vec(),
        omega_set: kept_omega,
        lambda_set: relevant_lambda,
        regular_set: (kept_ineq, real_regular),
        y_fixation: None,
        y_indices: None,
        x_indices: None,
        cheb_radius,
    })
}

/// Chebyshev ball LP started at `x0` = (θ0, r0).
fn chebyshev_ball_from(a: &Mat, b: &[f64], x0: &[f64]) -> Option<crate::solver::lp::LpSolution> {
    let n = a.cols;
    let m = a.rows;
    let mut ab = Mat::zeros(m + 1, n + 1);
    for i in 0..m {
        let r = a.row(i);
        ab.row_mut(i)[..n].copy_from_slice(r);
        ab[(i, n)] = crate::linalg::norm(r);
    }
    ab[(m, n)] = -1.0;
    let mut bb = b.to_vec();
    bb.push(0.0);
    let mut c = vec![0.0; n + 1];
    c[n] = -1.0;
    let x = crate::solver::lp::lp_point_from(Some(&c), &ab, &bb, &[], x0)?;
    let obj = -x[n];
    Some(crate::solver::lp::LpSolution { x, obj, dual: vec![], slack: vec![], active_set: vec![] })
}

/// Bounding box of {Eθ <= f} (2n LPs started at the interior point `c`); None if unbounded.
fn region_bbox(e: &Mat, f: &[f64], c: &[f64]) -> Option<(Vec<f64>, Vec<f64>)> {
    let n = e.cols;
    let mut lo = vec![0.0; n];
    let mut hi = vec![0.0; n];
    let mut obj = vec![0.0; n];
    for k in 0..n {
        obj[k] = 1.0;
        lo[k] = crate::solver::lp::lp_point_from(Some(&obj), e, f, &[], c)?[k];
        obj[k] = -1.0;
        hi[k] = crate::solver::lp::lp_point_from(Some(&obj), e, f, &[], c)?[k];
        obj[k] = 0.0;
    }
    Some((lo, hi))
}

fn get_bounds_1d(e: &Mat, f: &[f64]) -> (f64, f64) {
    let mut lo = f64::NEG_INFINITY;
    let mut hi = f64::INFINITY;
    for i in 0..e.rows {
        let a = e[(i, 0)];
        if a > 0.0 {
            hi = hi.min(f[i] / a);
        } else {
            lo = lo.max(f[i] / a);
        }
    }
    (lo, hi)
}

pub fn gen_cr_from_active_set_1d(program: &MpProgram, active_set: &[usize], check_full_dim: bool) -> Option<CriticalRegion> {
    let neq = program.num_equality_constraints();
    let law = program.optimal_control_law(active_set)?;
    let (e_all, f_all, n_lambda, inactive) = program.region_rows(active_set, &law);
    let n_inactive = inactive.len();
    let n_omega = program.a_t.rows;
    let kept_rows = numerically_nonzero_rows(&e_all);
    let mut pos = vec![usize::MAX; e_all.rows];
    for (p, &r) in kept_rows.iter().enumerate() {
        pos[r] = p;
    }
    for r in 0..e_all.rows {
        if pos[r] == usize::MAX && f_all[r] < -1e-8 {
            return None;
        }
    }
    let mut e = e_all.select_rows(&kept_rows);
    let mut f: Vec<f64> = kept_rows.iter().map(|&r| f_all[r]).collect();
    for i in 0..e.rows {
        let nrm = e[(i, 0)].abs();
        e[(i, 0)] /= nrm;
        f[i] /= nrm;
    }
    let (lo, hi) = get_bounds_1d(&e, &f);
    if check_full_dim && lo + 1e-8 > hi {
        return None;
    }
    let mut kept_lambda = vec![];
    let mut kept_ineq = vec![];
    let mut kept_omega = vec![];
    let non_red = |p: usize| {
        let v = f[p] / e[(p, 0)];
        lo <= v && v <= hi
    };
    for idx in 0..n_lambda {
        let p = pos[idx];
        if p != usize::MAX && non_red(p) {
            kept_lambda.push(idx);
        }
    }
    for idx in 0..n_inactive {
        let p = pos[n_lambda + idx];
        if p != usize::MAX && non_red(p) {
            kept_ineq.push(idx);
        }
    }
    for idx in 0..n_omega {
        let p = pos[n_lambda + n_inactive + idx];
        if p != usize::MAX && non_red(p) {
            kept_omega.push(idx);
        }
    }
    let active: Vec<usize> = active_set[neq..].to_vec();
    crate::stats::region_built(active_set);
    Some(CriticalRegion {
        a: law.a_x,
        b: law.b_x,
        c: law.a_l,
        d: law.b_l,
        e: Mat::from_rows(&[vec![1.0], vec![-1.0]]),
        f: vec![hi, -lo],
        active_set: active_set.to_vec(),
        omega_set: kept_omega,
        lambda_set: kept_lambda.iter().map(|&i| active[i]).collect(),
        regular_set: (kept_ineq.clone(), kept_ineq.iter().map(|&i| inactive[i]).collect()),
        y_fixation: None,
        y_indices: None,
        x_indices: None,
        cheb_radius: None,
    })
}

/// Builds the region without facet culling (PPOPT `build_suboptimal_critical_region`).
pub fn build_suboptimal_critical_region(program: &MpProgram, active_set: &[usize]) -> Option<(Mat, Vec<f64>)> {
    let law = program.optimal_control_law(active_set)?;
    let (e, f, _, _) = program.region_rows(active_set, &law);
    let (e, f) = crate::utils::cheap_remove_redundant_constraints(&e, &f);
    Some(remove_duplicate_rows(&e, &f))
}

/// Is {x : A x <= b} full dimensional (Chebyshev radius > 1e-8)?
pub fn is_full_dimensional(a: &Mat, b: &[f64]) -> bool {
    match chebyshev_ball(a, b, &[]) {
        Some(s) => s.x[a.cols] > 1e-8,
        None => false,
    }
}
