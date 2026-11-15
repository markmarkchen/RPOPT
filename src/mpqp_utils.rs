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
        crate::census_purpose!(Feasibility);
        self.point = crate::solver::lp::lp_feasible_point(&self.e, &self.f, &[], None);
        self.point.is_some()
    }

    /// `nonempty` on the rows that can bind inside the parameter box (same answer, smaller LP).
    pub fn nonempty_in(&mut self, program: &MpProgram) -> bool {
        crate::census_purpose!(Feasibility);
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
    crate::census_purpose!(Feasibility);
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
    crate::census_purpose!(Feasibility);
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

/// Redundancy-removal diagnostics: LP calls spent inside the row tests, and regions processed
/// (meaningful for single-threaded runs only).
pub static RED_LPS: crate::stats::Sharded = crate::stats::Sharded::new();
pub static RED_REGIONS: crate::stats::Sharded = crate::stats::Sharded::new();

/// `gen_cr_from_active_set` from precomputed law and rows (and a known region point if any).
pub fn gen_cr_from_kkt(program: &MpProgram, active_set: &[usize], kkt: KktRegion, check_full_dim: bool) -> Option<CriticalRegion> {
    if program.num_t() == 1 {
        return gen_cr_from_active_set_1d(program, active_set, check_full_dim);
    }
    crate::census_purpose!(Chebyshev);
    crate::census_build!(cbuild);
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
    match chebyshev_ball_from_or_cold(&e, &f, &start_r) {
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
    let red_lp0 = crate::solver::lp::LP_CALLS.load();
    // Fast path (same answers, far fewer LPs): interval-propagated parameter box plus exact
    // vertex clipping of that box, see `row_flags`.
    let flags: Option<Vec<bool>> = center.as_ref().map(|c| row_flags(program, &e, &f, c));
    let bbox = if flags.is_some() { None } else { center.as_ref().and_then(|c| region_bbox(&e, &f, c)) };
    let test = |p: usize| -> bool {
        crate::census_bump!(RedRowsIn, 1);
        if let Some(fl) = &flags {
            return fl[p];
        }
        crate::census_purpose!(Redundancy);
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
    RED_LPS.add(crate::solver::lp::LP_CALLS.load() - red_lp0);
    RED_REGIONS.add(1);
    let active: Vec<usize> = active_set[neq..].to_vec();
    let relevant_lambda: Vec<usize> = kept_lambda.iter().map(|&i| active[i]).collect();
    let real_regular: Vec<usize> = kept_ineq.iter().map(|&i| inactive[i]).collect();
    let e_red = e.select_rows(&non_redundant);
    let f_red: Vec<f64> = non_redundant.iter().map(|&p| f[p]).collect();
    let (e_red, f_red) = remove_duplicate_rows(&e_red, &f_red);
    crate::stats::region_built(active_set);
    crate::census_bump!(RedRowsKept, non_redundant.len());
    crate::census_bump!(FacetRows, e_red.rows);
    crate::census_keep!(cbuild);
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

/// `chebyshev_ball_from`; when the warm LP fails, the cold Chebyshev LP, and when that fails too, the cold LP on
/// the rows in reverse order, then with the right hand side relaxed by 1e-9 (relative). The dense LP can fail on a
/// nonempty system (found on masses8inf_N1, 16 parameters: the central region, radius 1.6, 95 rows, fails as given
/// and solves after removing any one of 40 rows or after reordering; it was dropped by every exact solver, 17 of 2,000
/// sampled parameters uncovered). The retries solve the same problem, so they change only runs that previously
/// lost the region; the radius from a relaxed right hand side is off by at most 1e-9.
fn chebyshev_ball_from_or_cold(a: &Mat, b: &[f64], x0: &[f64]) -> Option<crate::solver::lp::LpSolution> {
    chebyshev_ball_from(a, b, x0).or_else(|| crate::utils::chebyshev_ball(a, b, &[])).or_else(|| {
        let order: Vec<usize> = (0..a.rows).rev().collect();
        let (ar, br) = (a.select_rows(&order), order.iter().map(|&i| b[i]).collect::<Vec<f64>>());
        crate::utils::chebyshev_ball(&ar, &br, &[])
    }).or_else(|| {
        let relaxed: Vec<f64> = b.iter().map(|v| v + 1e-9 * (1.0 + v.abs())).collect();
        crate::utils::chebyshev_ball(a, &relaxed, &[])
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
    crate::census_purpose!(Bbox);
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
    crate::census_build!(cbuild);
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
    crate::census_keep!(cbuild);
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
    crate::census_purpose!(Chebyshev);
    match chebyshev_ball(a, b, &[]) {
        Some(s) => s.x[a.cols] > 1e-8,
        None => false,
    }
}

// ------------------------------------------------------------------------------------------
// Region construction for pivoting algorithms (mplcp): same rows as `gen_cr_from_kkt`, fewer LPs.

/// max of a'θ over the box [lo, hi].
fn box_max(a: &[f64], lo: &[f64], hi: &[f64]) -> f64 {
    let mut s = 0.0;
    for k in 0..a.len() {
        if a[k] > 0.0 {
            s += a[k] * hi[k];
        } else if a[k] < 0.0 {
            s += a[k] * lo[k];
        }
    }
    s
}

/// Tightens the box [lo, hi] with the rows of {E θ <= f} (interval constraint propagation). Every
/// point of the polytope stays inside the box.
fn propagate_box(e: &Mat, f: &[f64], lo: &mut [f64], hi: &mut [f64], rounds: usize) {
    let n = e.cols;
    for _ in 0..rounds {
        let mut changed = false;
        for i in 0..e.rows {
            let a = e.row(i);
            // minimum of a'θ over the box, and the number of infinite contributions
            let mut s = 0.0;
            let mut ninf = 0;
            for k in 0..n {
                let t = if a[k] > 0.0 {
                    a[k] * lo[k]
                } else if a[k] < 0.0 {
                    a[k] * hi[k]
                } else {
                    continue;
                };
                if t.is_finite() {
                    s += t;
                } else {
                    ninf += 1;
                }
            }
            if ninf > 1 {
                continue;
            }
            for k in 0..n {
                if a[k] == 0.0 {
                    continue;
                }
                let own = if a[k] > 0.0 { a[k] * lo[k] } else { a[k] * hi[k] };
                let m = if own.is_finite() {
                    if ninf > 0 {
                        continue;
                    }
                    s - own
                } else {
                    s
                };
                let v = (f[i] - m) / a[k];
                if a[k] > 0.0 {
                    if v < hi[k] - 1e-9 {
                        hi[k] = v;
                        changed = true;
                    }
                } else if v > lo[k] + 1e-9 {
                    lo[k] = v;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
}

/// `region_bbox` that also returns the 2n optimal vertices.
fn region_bbox_pts(e: &Mat, f: &[f64], c: &[f64]) -> Option<(Vec<f64>, Vec<f64>, Vec<Vec<f64>>)> {
    crate::census_purpose!(Bbox);
    let n = e.cols;
    let mut lo = vec![0.0; n];
    let mut hi = vec![0.0; n];
    let mut pts = Vec::with_capacity(2 * n);
    let mut obj = vec![0.0; n];
    for k in 0..n {
        obj[k] = 1.0;
        let x = crate::solver::lp::lp_point_from(Some(&obj), e, f, &[], c)?;
        lo[k] = x[k];
        pts.push(x);
        obj[k] = -1.0;
        let x = crate::solver::lp::lp_point_from(Some(&obj), e, f, &[], c)?;
        hi[k] = x[k];
        pts.push(x);
        obj[k] = 0.0;
    }
    Some((lo, hi, pts))
}

/// For every row of {E θ <= f}: does its hyperplane touch the polytope? Computed from all vertices
/// of {lo <= θ <= hi} ∩ {E θ <= f}, found by cutting the box with the rows one at a time (rows in
/// `order`). A vertex carries the set of hyperplanes (one bit per distinct row or box face) it is
/// tight on; the new vertices of a cut lie on the edges (pairs of vertices sharing at least nt - 1
/// tight hyperplanes) that cross the cutting hyperplane. A hyperplane touches a bounded polytope
/// exactly when it contains a vertex. In degenerate cases extra points of the polytope can appear
/// in the list; they change neither a maximum nor a "some point lies on this hyperplane" answer.
/// Result per row: 1 touches (a vertex within 1e-9 of it), 0 does not (all vertices at least 1e-7
/// away), -1 undecided (in between). None when the problem does not fit (nt outside 2..=5, more
/// than 128 hyperplanes, an infinite box, too many vertices) or a vertex violates a row.
fn clip_flags(e: &Mat, f: &[f64], order: &[usize], lo: &[f64], hi: &[f64]) -> Option<Vec<i8>> {
    crate::census_bump!(ClipCalls, 1);
    if lo.iter().chain(hi).any(|v| !v.is_finite()) {
        crate::census_bump!(ClipInfBox, 1);
        return None;
    }
    if e.rows + 2 * e.cols > 128 {
        crate::census_bump!(ClipManyHyp, 1);
        return None;
    }
    let r = match e.cols {
        2 => clip_n::<2>(e, f, order, lo, hi),
        3 => clip_n::<3>(e, f, order, lo, hi),
        4 => clip_n::<4>(e, f, order, lo, hi),
        5 => clip_n::<5>(e, f, order, lo, hi),
        // 6 parameters and more take the LP path, which is 1.2x to 10x faster there (see mplcp_build)
        _ => {
            crate::census_bump!(ClipNtOut, 1);
            None
        }
    };
    if r.is_some() {
        crate::census_bump!(ClipOk, 1);
    }
    r
}

#[inline(always)]
fn dotn<const N: usize>(a: &[f64], x: &[f64; N]) -> f64 {
    let a: &[f64; N] = a[..N].try_into().unwrap();
    let mut s = 0.0;
    for k in 0..N {
        s += a[k] * x[k];
    }
    s
}

fn clip_n<const N: usize>(e: &Mat, f: &[f64], order: &[usize], lo: &[f64], hi: &[f64]) -> Option<Vec<i8>> {
    let m = e.rows;
    let mut pts: Vec<[f64; N]> = Vec::with_capacity(64);
    let mut tight: Vec<u128> = Vec::with_capacity(64);
    for mask in 0..(1usize << N) {
        let mut t = 0u128;
        let mut x = [0.0; N];
        for k in 0..N {
            let hi_k = (mask >> k) & 1 == 1;
            x[k] = if hi_k { hi[k] } else { lo[k] };
            t |= 1u128 << (m + 2 * k + hi_k as usize);
        }
        pts.push(x);
        tight.push(t);
    }
    // one bit per distinct hyperplane: rows equal to a box face or to an earlier row share its bit
    let mut bit: Vec<usize> = (0..m).collect();
    {
        let hkey = |a: &[f64], fr: f64| -> [i64; 7] {
            let mut k = [0i64; 7];
            for (o, v) in k.iter_mut().zip(a) {
                *o = (v * 1e8).round() as i64;
            }
            k[6] = (fr * 1e8).round() as i64;
            k
        };
        let mut seen: crate::fxhash::FxHashMap<[i64; 7], usize> = crate::fxhash::FxHashMap::with_capacity_and_hasher(m + 2 * N, Default::default());
        let mut unit = [0.0; N];
        for k in 0..N {
            unit[k] = 1.0;
            seen.insert(hkey(&unit, hi[k]), m + 2 * k + 1);
            unit[k] = -1.0;
            seen.insert(hkey(&unit, -lo[k]), m + 2 * k);
            unit[k] = 0.0;
        }
        for r in 0..m {
            match seen.get(&hkey(e.row(r), f[r])) {
                Some(&b) => bit[r] = b,
                None => {
                    seen.insert(hkey(e.row(r), f[r]), r);
                }
            }
        }
    }
    let mut npts: Vec<[f64; N]> = Vec::with_capacity(64);
    let mut ntight: Vec<u128> = Vec::with_capacity(64);
    let mut val: Vec<f64> = Vec::with_capacity(64);
    let mut ins: Vec<usize> = vec![];
    let mut outs: Vec<usize> = vec![];
    for &r in order {
        let a = e.row(r);
        let fr = f[r];
        let tol = 1e-8 * (1.0 + fr.abs());
        let nv = pts.len();
        val.clear();
        ins.clear();
        outs.clear();
        let rbit = 1u128 << bit[r];
        for v in 0..nv {
            let s = dotn::<N>(a, &pts[v]) - fr;
            val.push(s);
            if s > tol {
                outs.push(v);
            } else if s < -tol {
                ins.push(v);
            } else {
                tight[v] |= rbit;
            }
        }
        if outs.is_empty() {
            continue;
        }
        npts.clear();
        ntight.clear();
        for v in 0..nv {
            if val[v] <= tol {
                npts.push(pts[v]);
                ntight.push(tight[v]);
            }
        }
        let kept = npts.len();
        for &u in &ins {
            for &w in &outs {
                let common = tight[u] & tight[w];
                if (common.count_ones() as usize) < N - 1 {
                    continue;
                }
                let t = val[u] / (val[u] - val[w]);
                let mut px = [0.0; N];
                for k in 0..N {
                    px[k] = pts[u][k] + t * (pts[w][k] - pts[u][k]);
                }
                let bits = common | rbit;
                // skip a point already present among the vertices on the cut and the new ones
                let mut dup = false;
                for idx in 0..npts.len() {
                    if idx >= kept || val_on(idx, &ntight, rbit) {
                        let q = &npts[idx];
                        let mut d = 0.0f64;
                        for k in 0..N {
                            d = d.max((q[k] - px[k]).abs());
                        }
                        if d <= 1e-9 {
                            ntight[idx] |= bits;
                            dup = true;
                            break;
                        }
                    }
                }
                if !dup {
                    npts.push(px);
                    ntight.push(bits);
                }
            }
        }
        if npts.len() > 600 {
            crate::census_bump!(ClipManyVerts, 1);
            return None;
        }
        if npts.len() <= N {
            crate::census_bump!(ClipFewVerts, 1);
            return None;
        }
        std::mem::swap(&mut pts, &mut npts);
        std::mem::swap(&mut tight, &mut ntight);
    }
    let mut out = Vec::with_capacity(m);
    for r in 0..m {
        let a = e.row(r);
        let fp = f[r];
        let sc = 1.0 + fp.abs();
        let mut smin = f64::INFINITY;
        for x in &pts {
            smin = smin.min(fp - dotn::<N>(a, x));
        }
        if smin < -1e-7 * sc {
            crate::census_bump!(ClipViolate, 1);
            return None;
        }
        out.push(if smin <= 1e-9 * sc {
            1
        } else if smin >= 1e-7 * sc {
            0
        } else {
            -1
        });
    }
    Some(out)
}

/// Is vertex `idx` (of the kept ones) tight on the cutting hyperplane `rbit`?
#[inline(always)]
fn val_on(idx: usize, tight: &[u128], rbit: u128) -> bool {
    tight[idx] & rbit != 0
}

/// `gen_cr_from_law_lambda_first(.., check_full_dim = true)` with the same result rows but fewer
/// LPs (two or more parameters):
/// * `hint`, a point believed to lie in the region (e.g. on a facet shared with a neighbour),
///   starts the Chebyshev LP, so the two feasibility LPs are skipped (an infeasible hint only
///   costs a longer first phase);
/// * rows that cannot touch the region because of an interval-propagated bounding box are dropped
///   from the LPs that follow (a redundant row never changes a feasibility answer);
/// * a row tight at one of the bounding-box vertices is kept without its own LP.
pub fn gen_cr_from_law_hinted(program: &MpProgram, active_set: &[usize], law: crate::program::ControlLaw, hint: Option<&[f64]>) -> Option<(CriticalRegion, Vec<f64>)> {
    let nt = program.num_t();
    if nt == 1 {
        return gen_cr_from_active_set_1d(program, active_set, true).map(|c| (c, vec![0.0]));
    }
    crate::census_purpose!(Chebyshev);
    let KktRegion { mut law, e: e_all, f: f_all, n_lambda, inactive, point: _ } = kkt_region_from_law(program, active_set, law);
    program.complete_primal_law(active_set, &mut law);

    let kept_rows = numerically_nonzero_rows(&e_all);
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
    crate::stats::lp(1);
    for r in 0..e_all.rows {
        if pos[r] == usize::MAX && f_all[r] < -1e-8 {
            return None;
        }
    }
    let start = match hint {
        Some(h) => h.to_vec(),
        None => crate::solver::lp::lp_feasible_point(&e, &f, &[], None)?,
    };
    let mut start_r = start;
    start_r.push(0.0);
    let s = chebyshev_ball_from_or_cold(&e, &f, &start_r)?;
    if s.x[nt] <= 1e-8 {
        return None;
    }
    let cheb_radius = Some(s.x[nt]);
    let center: Vec<f64> = s.x[..nt].to_vec();

    gen_cr_from_prepared(program, active_set, true, Prepared { law, n_lambda, inactive, pos, e, f, kept_rows_len: kept_rows.len(), center, cheb_radius })
}

/// Everything `gen_cr_from_law_hinted` has computed before the redundancy step: the completed
/// law, the unit-norm kept rows `e x <= f` (`pos` maps an unreduced row to its place, or
/// `usize::MAX`), and the Chebyshev centre and radius.
pub struct Prepared {
    pub law: crate::program::ControlLaw,
    pub n_lambda: usize,
    pub inactive: Vec<usize>,
    pub pos: Vec<usize>,
    pub e: Mat,
    pub f: Vec<f64>,
    pub kept_rows_len: usize,
    pub center: Vec<f64>,
    pub cheb_radius: Option<f64>,
}

/// The redundancy step of `gen_cr_from_law_hinted` on prepared rows. It is also the fallback of
/// mplcp's own builder, which has done the preparation already (no second KKT build and second
/// Chebyshev LP). `try_clip` = false skips the vertex clipping, for a caller whose own clipping
/// of the same rows has just failed.
pub fn gen_cr_from_prepared(program: &MpProgram, active_set: &[usize], try_clip: bool, prep: Prepared) -> Option<(CriticalRegion, Vec<f64>)> {
    let Prepared { law, n_lambda, inactive, pos, e, f, kept_rows_len, center, cheb_radius } = prep;
    let nt = program.num_t();
    let neq = program.num_equality_constraints();
    let n_inactive = inactive.len();
    let n_omega = program.a_t.rows;
    crate::census_build!(cbuild);
    let red_lp0 = crate::solver::lp::LP_CALLS.load();
    crate::stats::lp(kept_rows_len as u64);
    // rows that provably cannot touch the region (interval propagation from the parameter box)
    let (mut blo, mut bhi) = match program.theta_bbox() {
        Some((l, h)) => (l.clone(), h.clone()),
        None => (vec![f64::NEG_INFINITY; nt], vec![f64::INFINITY; nt]),
    };
    propagate_box(&e, &f, &mut blo, &mut bhi, 6);
    crate::census_bump!(RedRowsIn, e.rows);
    let flags: Vec<bool> = {
        crate::census_purpose!(Redundancy);
        let mut order: Vec<usize> = (0..e.rows).collect();
        let slack: Vec<f64> = (0..e.rows).map(|i| f[i] - crate::linalg::dot(e.row(i), &center)).collect();
        order.sort_by(|&x, &y| slack[x].partial_cmp(&slack[y]).unwrap_or(std::cmp::Ordering::Equal));
        match if try_clip { clip_flags(&e, &f, &order, &blo, &bhi) } else { None } {
            Some(fl) => fl.iter().enumerate().map(|(p, &v)| if v >= 0 { v == 1 } else { crate::solver::lp::lp_feasible_from(&e, &f, &[p], &center) }).collect(),
            None => lp_flags(&e, &f, &center, &blo, &bhi),
        }
    };
    let test = |p: usize| -> bool { flags[p] };
    let mut kept_lambda = vec![];
    let mut kept_ineq = vec![];
    let mut kept_omega = vec![];
    let mut non_redundant = vec![];
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
    RED_LPS.add(crate::solver::lp::LP_CALLS.load() - red_lp0);
    RED_REGIONS.add(1);
    let active: Vec<usize> = active_set[neq..].to_vec();
    let relevant_lambda: Vec<usize> = kept_lambda.iter().map(|&i| active[i]).collect();
    let real_regular: Vec<usize> = kept_ineq.iter().map(|&i| inactive[i]).collect();
    let e_red = e.select_rows(&non_redundant);
    let f_red: Vec<f64> = non_redundant.iter().map(|&p| f[p]).collect();
    let (e_red, f_red) = remove_duplicate_rows(&e_red, &f_red);
    crate::stats::region_built(active_set);
    crate::census_bump!(RedRowsKept, non_redundant.len());
    crate::census_bump!(FacetRows, e_red.rows);
    crate::census_keep!(cbuild);
    Some((CriticalRegion {
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
    }, center))
}

/// For every row of {E θ <= f} (unit-norm rows, `center` interior): does its hyperplane touch the
/// region? Same decision rule as `gen_cr_from_law_hinted`: interval-propagated parameter box,
/// exact vertex clipping of that box (no LP) with LPs only for undecided rows, or the box-screened
/// LP path when clipping does not apply.
fn row_flags(program: &MpProgram, e: &Mat, f: &[f64], center: &[f64]) -> Vec<bool> {
    crate::census_purpose!(Redundancy);
    let nt = e.cols;
    let (mut blo, mut bhi) = match program.theta_bbox() {
        Some((l, h)) => (l.clone(), h.clone()),
        None => (vec![f64::NEG_INFINITY; nt], vec![f64::INFINITY; nt]),
    };
    propagate_box(e, f, &mut blo, &mut bhi, 6);
    let mut order: Vec<usize> = (0..e.rows).collect();
    let slack: Vec<f64> = (0..e.rows).map(|i| f[i] - crate::linalg::dot(e.row(i), center)).collect();
    order.sort_by(|&x, &y| slack[x].partial_cmp(&slack[y]).unwrap_or(std::cmp::Ordering::Equal));
    // Measured: vertex clipping saves LPs on mpQP regions (about 1.15x on masses2_N6 geometric and
    // graph), but on the degenerate mpLP regions it costs more than the LPs it saves (0.65x on
    // sys3x2inf_N2 graph), so mpLPs use the box-screened LP path.
    if program.q.is_none() {
        return lp_flags(e, f, center, &blo, &bhi);
    }
    match clip_flags(e, f, &order, &blo, &bhi) {
        Some(fl) => fl.iter().enumerate().map(|(p, &v)| if v >= 0 { v == 1 } else { crate::solver::lp::lp_feasible_from(e, f, &[p], center) }).collect(),
        None => lp_flags(e, f, center, &blo, &bhi),
    }
}

/// Per-row "touches the region" flags by bounding-box screening and LPs (the general path).
fn lp_flags(e: &Mat, f: &[f64], center: &[f64], blo: &[f64], bhi: &[f64]) -> Vec<bool> {
    let reject_by = |a: &[f64], fp: f64, lo: &[f64], hi: &[f64]| -> bool { box_max(a, lo, hi) < fp - 1e-7 * (1.0 + fp.abs()) };
    let mut live_pos = vec![usize::MAX; e.rows];
    let mut live: Vec<usize> = vec![];
    for p in 0..e.rows {
        if !reject_by(e.row(p), f[p], blo, bhi) {
            live_pos[p] = live.len();
            live.push(p);
        }
    }
    let el = e.select_rows(&live);
    let fl: Vec<f64> = live.iter().map(|&p| f[p]).collect();
    let bbox = region_bbox_pts(&el, &fl, center);
    let test = |p: usize| -> bool {
        let lp_ = live_pos[p];
        if lp_ == usize::MAX {
            return false;
        }
        let a = el.row(lp_);
        let fp = fl[lp_];
        let tol = 1e-9 * (1.0 + fp.abs());
        if let Some((lo, hi, pts)) = &bbox {
            if reject_by(a, fp, lo, hi) {
                return false;
            }
            if pts.iter().any(|v| (crate::linalg::dot(a, v) - fp).abs() <= tol) {
                return true;
            }
        }
        let sft = fp - crate::linalg::dot(a, center);
        let q: Vec<f64> = center.iter().zip(a).map(|(ci, ai)| ci + sft * ai).collect();
        let inside = (0..el.rows).all(|i| i == lp_ || crate::linalg::dot(el.row(i), &q) <= fl[i] + 1e-9 * (1.0 + fl[i].abs()));
        if inside && (crate::linalg::dot(a, &q) - fp).abs() <= tol {
            return true;
        }
        crate::solver::lp::lp_feasible_from(&el, &fl, &[lp_], center)
    };
    (0..e.rows).map(test).collect()
}

#[cfg(test)]
mod redundancy_screen_tests {
    use super::*;

    // unit square 0 <= x, y <= 1 plus the diagonal row x + y <= 3 (unit-norm form), which the
    // square's bounding box already proves redundant
    fn square() -> (Mat, Vec<f64>) {
        let h = std::f64::consts::FRAC_1_SQRT_2;
        let rows = [[-1.0, 0.0], [0.0, -1.0], [1.0, 0.0], [0.0, 1.0], [h, h]];
        let mut e = Mat::zeros(rows.len(), 2);
        for (i, r) in rows.iter().enumerate() {
            e.row_mut(i).copy_from_slice(r);
        }
        (e, vec![0.0, 0.0, 1.0, 1.0, 3.0 * h])
    }

    #[test]
    fn box_proves_row_redundant() {
        let (e, f) = square();
        let (lo, hi) = (vec![0.0, 0.0], vec![1.0, 1.0]);
        // max over the box of the diagonal row is 1.414 < 2.12: redundant without any LP
        assert!(box_max(e.row(4), &lo, &hi) < f[4] - 1e-7);
        // the facet rows are reached by the box
        assert!(box_max(e.row(2), &lo, &hi) >= f[2] - 1e-12);
        // interval propagation recovers the box from the rows alone
        let (mut l, mut h) = (vec![-10.0; 2], vec![10.0; 2]);
        propagate_box(&e, &f, &mut l, &mut h, 6);
        assert!(l.iter().all(|v| v.abs() < 1e-9) && h.iter().all(|v| (v - 1.0).abs() < 1e-9));
    }

    #[test]
    fn clipping_certifies_rows_without_lp() {
        let (e, f) = square();
        let order: Vec<usize> = (0..e.rows).collect();
        let fl = clip_flags(&e, &f, &order, &[0.0, 0.0], &[1.0, 1.0]).expect("fits");
        // the four sides touch the region (vertices are witnesses), the diagonal does not
        assert_eq!(fl, vec![1, 1, 1, 1, 0]);
    }

    #[test]
    fn witness_point_certifies_row() {
        // the projection of the interior point (0.5, 0.5) onto x <= 1 is (1, 0.5): it satisfies
        // all other rows, so x <= 1 is irredundant with no LP
        let (e, f) = square();
        let c = [0.5, 0.5];
        let p = 2;
        let s = f[p] - crate::linalg::dot(e.row(p), &c);
        let q: Vec<f64> = c.iter().zip(e.row(p)).map(|(ci, ai)| ci + s * ai).collect();
        assert!((0..e.rows).all(|i| i == p || crate::linalg::dot(e.row(i), &q) <= f[i] + 1e-9));
        assert!((crate::linalg::dot(e.row(p), &q) - f[p]).abs() < 1e-9);
    }
}
