//! Private critical-region builder of the mplcp solver (included from `mplcp.rs`).
//!
//! A copy of `mpqp_utils::gen_cr_from_law_hinted` (same rows, same flags, same result) with a
//! cheaper vertex clipping: a row that clears the running bounding box of the clipped polytope is
//! skipped without a pass over the vertices (it can neither cut a vertex nor be tight at one),
//! and no interval propagation of the parameter box is done. Falls back to the shared builder when the clipping does not apply (more than 128
//! rows, or nt outside 2..=5).

use crate::critical_region::CriticalRegion;
use crate::linalg::Mat;
use crate::program::MpProgram;
use std::cmp::Ordering;

/// The region, its Chebyshev center, and (only when the fast clip succeeded) the vertices of the
/// region polytope, flattened (nt coordinates each).
pub(super) type Built = (CriticalRegion, Vec<f64>, Option<Vec<f64>>, Option<Vec<f64>>);

pub(super) fn build_region(program: &MpProgram, active_set: &[usize], hint: Option<&[f64]>, law: Option<crate::program::ControlLaw>, want_verts: bool, parent_box: Option<&[f64]>, entry: Option<&(Vec<f64>, f64)>, guard: &ChainGuard) -> Option<Built> {
    use crate::mpqp_utils::kkt_region_from_law;
    crate::census_purpose!(Chebyshev);
    #[cfg(feature = "lp_census")]
    let _tb = crate::census_diag::t0();
    use crate::utils::{numerically_nonzero_rows, remove_duplicate_rows};
    let nt = program.num_t();
    let law = match law {
        Some(l) => l,
        None => program.optimal_control_law_dual(active_set)?,
    };
    let neq = program.num_equality_constraints();
    #[cfg(feature = "lp_census")]
    let _tr = crate::census_diag::t0();
    let crate::mpqp_utils::KktRegion { mut law, e: e_all, f: f_all, n_lambda, inactive, point: _ } = kkt_region_from_law(program, active_set, law);
    let n_inactive = inactive.len();
    let n_omega = program.a_t.rows;

    let kept_rows = numerically_nonzero_rows(&e_all);
    let mut pos = vec![usize::MAX; e_all.rows];
    for (p, &r) in kept_rows.iter().enumerate() {
        pos[r] = p;
    }
    let dropped_zero = (0..e_all.rows).any(|r| pos[r] == usize::MAX && f_all[r].abs() <= 1e-8);
    let mut e = e_all.select_rows(&kept_rows);
    let mut f: Vec<f64> = kept_rows.iter().map(|&r| f_all[r]).collect();
    for i in 0..e.rows {
        let nrm = crate::linalg::norm(e.row(i));
        for v in e.row_mut(i) {
            *v /= nrm;
        }
        f[i] /= nrm;
    }
    #[cfg(feature = "lp_census")]
    crate::census_diag::lap(crate::census_diag::s::ROWS_NS, _tr);
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
    #[cfg(feature = "lp_census")]
    let _tc = crate::census_diag::t0();
    let mut start_r = start;
    start_r.push(0.0);
    // The LP started at the hint (a point on the facet the region was reached through, so on its boundary) can stop at radius 0 or fail on a thin
    // region (masses5_N3: regions of radius 2e-6 and 4e-8 were dropped, and their active sets stayed claimed in `Seen`, so no later facet tried again);
    // a cold Chebyshev LP then decides, as it does for every region built without a hint.
    let warm = cheb_from(&e, &f, &start_r).filter(|s| s.x[nt] > 1e-8);
    let s = match warm {
        Some(s) => s,
        None => {
            crate::census_bump!(BuildColdRetry, 1);
            let s = crate::utils::chebyshev_ball(&e, &f, &[])?;
            if s.x[nt] <= 1e-8 {
                return None;
            }
            s
        }
    };
    #[cfg(feature = "lp_census")]
    crate::census_diag::lap(crate::census_diag::s::CHEB_NS, _tc);
    let cheb_radius = Some(s.x[nt]);
    let center: Vec<f64> = s.x[..nt].to_vec();
    // after the radius test: only the output fields need the primal law
    program.complete_primal_law(active_set, &mut law);
    crate::stats::lp(kept_rows.len() as u64);
    // The parameter box is the starting polytope: the running bounding box of the clipped vertices
    // makes a separate interval propagation unnecessary (it measured slower than it saved).
    let (blo, bhi) = match program.theta_bbox() {
        Some((l, h)) => (l.clone(), h.clone()),
        None => (vec![f64::NEG_INFINITY; nt], vec![f64::INFINITY; nt]),
    };
    let mut order: Vec<usize> = (0..e.rows).collect();
    let slack: Vec<f64> = (0..e.rows).map(|i| f[i] - crate::linalg::dot(e.row(i), &center)).collect();
    // ties broken by the row index: the order of the stable sort
    order.sort_unstable_by(|&x, &y| slack[x].partial_cmp(&slack[y]).unwrap_or(Ordering::Equal).then(x.cmp(&y)));
    let verts: Option<Vec<f64>>;
    #[cfg(feature = "lp_census")]
    let _tclip = crate::census_diag::t0();
    let clip_res = clip_flags(&e, &f, &order, &blo, &bhi, want_verts);
    #[cfg(feature = "lp_census")]
    {
        crate::census_diag::lap(crate::census_diag::s::PF_CLIP_NS, _tclip);
        if clip_res.is_none() && nt <= 5 {
            crate::census_diag::lap(crate::census_diag::s::PF_CLIP_FAIL_NS, _tclip);
            crate::census_diag::add(crate::census_diag::s::PF_CLIP_FAILS, 1);
        }
    }
    let flags: Vec<bool> = match clip_res {
        Some((fl, vt)) => {
            verts = if want_verts { Some(vt) } else { None };
            fl.iter().enumerate().map(|(p, &v)| if v >= 0 { v == 1 } else { crate::solver::lp::lp_feasible_from(&e, &f, &[p], &center) }).collect()
        }
        None => {
            #[cfg(feature = "lp_census")]
            let _mute = crate::census::MuteClip::new();
            // the same rows and centre go to the shared redundancy step; its own clipping of
            // them would fail the same way (measured: it never succeeded after a failed first pass)
            #[cfg(feature = "lp_census")]
            let _tred = crate::census_diag::t0();
            let prep = crate::mpqp_utils::Prepared { law, n_lambda, inactive, pos, e, f, kept_rows_len: kept_rows.len(), center, cheb_radius, dropped_zero };
            let r = if nt >= 6 && guard.chain_on() { lp_path(program, active_set, prep, parent_box, entry, guard) } else { crate::mpqp_utils::gen_cr_from_prepared(program, active_set, false, prep).map(|(cr, c)| (cr, c, None, None)) };
            #[cfg(feature = "lp_census")]
            {
                crate::census_diag::lap(crate::census_diag::s::RED_NS, _tred);
                crate::census_diag::lap(crate::census_diag::s::BUILD_NS, _tb);
            }
            return r;
        }
    };
    let mut kept_lambda = vec![];
    let mut kept_ineq = vec![];
    let mut kept_omega = vec![];
    let mut non_redundant = vec![];
    for idx in 0..n_lambda {
        let p = pos[idx];
        if p != usize::MAX && flags[p] {
            kept_lambda.push(idx);
            non_redundant.push(p);
        }
    }
    for idx in 0..n_inactive {
        let p = pos[n_lambda + idx];
        if p != usize::MAX && flags[p] {
            kept_ineq.push(idx);
            non_redundant.push(p);
        }
    }
    for idx in 0..n_omega {
        let p = pos[n_lambda + n_inactive + idx];
        if p != usize::MAX && flags[p] {
            kept_omega.push(idx);
            non_redundant.push(p);
        }
    }
    let active: Vec<usize> = active_set[neq..].to_vec();
    let relevant_lambda: Vec<usize> = kept_lambda.iter().map(|&i| active[i]).collect();
    let real_regular: Vec<usize> = kept_ineq.iter().map(|&i| inactive[i]).collect();
    let facet_single = crate::utils::unique_hyperplane_flags(&e, &f, &non_redundant, dropped_zero);
    let e_red = e.select_rows(&non_redundant);
    let f_red: Vec<f64> = non_redundant.iter().map(|&p| f[p]).collect();
    let (e_red, f_red) = remove_duplicate_rows(&e_red, &f_red);
    crate::stats::region_built(active_set);
    crate::census_bump!(FacetRows, e_red.rows);
    Some((
        CriticalRegion {
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
        facet_single,
            },
        center,
        verts,
        None,
    ))
}

// ------------------------------------------------------------------ LP path with chained box LPs

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering as AO};

/// Switch for tests and comparisons: `false` sends every region to the shared builder. Read once at the start of a solve.
static CHAIN: AtomicBool = AtomicBool::new(true);

pub(super) fn set_chain(on: bool) {
    CHAIN.store(on, AO::Relaxed);
}

/// Fallback guard of one solve: after 100 regions, a chain whose objectives fall back to the engine in more than 20% of the cases is
/// switched off for the rest of that solve (the shared builder takes over). The state belongs to the solve (it is created in
/// `pivot_solve` and `pivot_solve_par` and passed to the builder), so two solves, on one thread or on several, never steer each other.
pub(super) struct ChainGuard {
    on: bool,
    reg: AtomicU64,
    fb: AtomicU64,
    eng: AtomicU64,
    init: AtomicU64,
    reb: AtomicU64,
    off: AtomicBool,
}

impl ChainGuard {
    pub(super) fn new() -> ChainGuard {
        ChainGuard { on: CHAIN.load(AO::Relaxed), reg: AtomicU64::new(0), fb: AtomicU64::new(0), eng: AtomicU64::new(0), init: AtomicU64::new(0), reb: AtomicU64::new(0), off: AtomicBool::new(false) }
    }

    fn chain_on(&self) -> bool {
        self.on && !self.off.load(AO::Relaxed)
    }

    fn note(&self, st: &crate::boxlp::Stats, n: usize) {
        let regions = 1;
        self.eng.fetch_add(st.engine_lps, AO::Relaxed);
        self.init.fetch_add(st.init_failures, AO::Relaxed);
        self.reb.fetch_add(st.rebuilt, AO::Relaxed);
        let r = self.reg.fetch_add(regions, AO::Relaxed) + regions;
        // an objective that went to the engine because the simplex failed, and a chain that could not be started at a vertex, both count
        let fallbacks = st.fallbacks + st.init_failures;
        let f = self.fb.fetch_add(fallbacks, AO::Relaxed) + fallbacks;
        if r >= 100 && (f as f64) > 0.2 * (r as f64) * (2 * n) as f64 {
            self.off.store(true, AO::Relaxed);
        }
    }

    /// Writes this solve's counters to the "last solve" record that `chain_stats` reports.
    pub(super) fn publish(&self) {
        LAST[0].store(self.reg.load(AO::Relaxed), AO::Relaxed);
        LAST[1].store(self.fb.load(AO::Relaxed), AO::Relaxed);
        LAST[2].store(self.eng.load(AO::Relaxed), AO::Relaxed);
        LAST[3].store(self.off.load(AO::Relaxed) as u64, AO::Relaxed);
        LAST[4].store(self.init.load(AO::Relaxed), AO::Relaxed);
        LAST[5].store(self.reb.load(AO::Relaxed), AO::Relaxed);
    }
}

const LAST0: AtomicU64 = AtomicU64::new(0);
/// Counters of the last finished solve, for reporting only (nothing reads them to decide anything).
static LAST: [AtomicU64; 6] = [LAST0; 6];

/// (regions built by the chain, objectives that fell back to the engine, engine LPs, chain switched off by the guard) of the last solve.
/// (chains that could not start at a vertex, failed checks resolved by the rebuild) of the last solve.
pub(super) fn chain_detail() -> (u64, u64) {
    (LAST[4].load(AO::Relaxed), LAST[5].load(AO::Relaxed))
}

pub(super) fn chain_stats() -> (u64, u64, u64, bool) {
    (LAST[0].load(AO::Relaxed), LAST[1].load(AO::Relaxed), LAST[2].load(AO::Relaxed), LAST[3].load(AO::Relaxed) != 0)
}

/// The redundancy step of `mpqp_utils::gen_cr_from_prepared` for 6 or more parameters, with the 2n
/// bounding-box LPs of the screening done by `boxlp::box_chain` (started at a vertex of the
/// parent's box on the shared facet when there is one). Same rows, same flags up to ties at the
/// box's optimal vertices; the box only screens rows and every later stage is exact. Returns the
/// region, its centre and the box's 2n points (flattened) for the region's own children.
fn lp_path(program: &MpProgram, active_set: &[usize], prep: crate::mpqp_utils::Prepared, parent_box: Option<&[f64]>, entry: Option<&(Vec<f64>, f64)>, guard: &ChainGuard) -> Option<Built> {
    use crate::utils::remove_duplicate_rows;
    let crate::mpqp_utils::Prepared { law, n_lambda, inactive, pos, e, f, kept_rows_len, center, cheb_radius, dropped_zero } = prep;
    let nt = program.num_t();
    let neq = program.num_equality_constraints();
    let n_inactive = inactive.len();
    let n_omega = program.a_t.rows;
    crate::census_build!(cbuild);
    crate::stats::lp(kept_rows_len as u64);
    let (mut blo, mut bhi) = match program.theta_bbox() {
        Some((l, h)) => (l.clone(), h.clone()),
        None => (vec![f64::NEG_INFINITY; nt], vec![f64::INFINITY; nt]),
    };
    crate::mpqp_utils::propagate_box_pub(&e, &f, &mut blo, &mut bhi, 6);
    crate::census_bump!(RedRowsIn, e.rows);
    let mut boxflat: Option<Vec<f64>> = None;
    let flags: Vec<bool> = {
        crate::census_purpose!(Redundancy);
        let reject_by = |a: &[f64], fp: f64, lo: &[f64], hi: &[f64]| -> bool { crate::mpqp_utils::box_max_pub(a, lo, hi) < fp - 1e-7 * (1.0 + fp.abs()) };
        let mut live_pos = vec![usize::MAX; e.rows];
        let mut live: Vec<usize> = vec![];
        for p in 0..e.rows {
            if !reject_by(e.row(p), f[p], &blo, &bhi) {
                live_pos[p] = live.len();
                live.push(p);
            }
        }
        let el = e.select_rows(&live);
        let fl: Vec<f64> = live.iter().map(|&p| f[p]).collect();
        let bbox = {
            crate::census_purpose!(Bbox);
            let start = match (parent_box, entry) {
                (Some(pb), Some((ea, ef))) => crate::boxlp::shared_start(&el, &fl, pb, ea, *ef),
                _ => None,
            };
            let mut st = crate::boxlp::Stats::default();
            #[cfg(feature = "lp_census")]
            let _tbox = crate::census_diag::t0();
            let r = crate::boxlp::box_chain_in(&el, &fl, &center, start.as_deref(), Some((&blo, &bhi)), &mut st);
            #[cfg(feature = "lp_census")]
            crate::census_diag::lap(crate::census_diag::s::PF_BOX_NS, _tbox);
            guard.note(&st, nt);
            if let Some((_, _, pts)) = &r {
                boxflat = Some(pts.iter().flat_map(|p| p.iter().copied()).collect());
            }
            r
        };
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
            let sft = fp - crate::linalg::dot(a, &center);
            let q: Vec<f64> = center.iter().zip(a).map(|(ci, ai)| ci + sft * ai).collect();
            let inside = (0..el.rows).all(|i| i == lp_ || crate::linalg::dot(el.row(i), &q) <= fl[i] + 1e-9 * (1.0 + fl[i].abs()));
            if inside && (crate::linalg::dot(a, &q) - fp).abs() <= tol {
                return true;
            }
            crate::solver::lp::lp_feasible_from(&el, &fl, &[lp_], &center)
        };
        #[cfg(feature = "lp_census")]
        let _trow = crate::census_diag::t0();
        let fl_out: Vec<bool> = (0..e.rows).map(test).collect();
        #[cfg(feature = "lp_census")]
        crate::census_diag::lap(crate::census_diag::s::PF_PERROW_NS, _trow);
        fl_out
    };
    let mut kept_lambda = vec![];
    let mut kept_ineq = vec![];
    let mut kept_omega = vec![];
    let mut non_redundant = vec![];
    for idx in 0..n_lambda {
        let p = pos[idx];
        if p != usize::MAX && flags[p] {
            kept_lambda.push(idx);
            non_redundant.push(p);
        }
    }
    for idx in 0..n_inactive {
        let p = pos[n_lambda + idx];
        if p != usize::MAX && flags[p] {
            kept_ineq.push(idx);
            non_redundant.push(p);
        }
    }
    for idx in 0..n_omega {
        let p = pos[n_lambda + n_inactive + idx];
        if p != usize::MAX && flags[p] {
            kept_omega.push(idx);
            non_redundant.push(p);
        }
    }
    let active: Vec<usize> = active_set[neq..].to_vec();
    let relevant_lambda: Vec<usize> = kept_lambda.iter().map(|&i| active[i]).collect();
    let real_regular: Vec<usize> = kept_ineq.iter().map(|&i| inactive[i]).collect();
    let facet_single = crate::utils::unique_hyperplane_flags(&e, &f, &non_redundant, dropped_zero);
    let e_red = e.select_rows(&non_redundant);
    let f_red: Vec<f64> = non_redundant.iter().map(|&p| f[p]).collect();
    let (e_red, f_red) = remove_duplicate_rows(&e_red, &f_red);
    crate::stats::region_built(active_set);
    crate::census_bump!(RedRowsKept, non_redundant.len());
    crate::census_bump!(FacetRows, e_red.rows);
    crate::census_keep!(cbuild);
    Some((
        CriticalRegion {
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
        facet_single,
            },
        center,
        None,
        boxflat,
    ))
}

/// Chebyshev ball LP started at x0 = (θ0, r0).
fn cheb_from(a: &Mat, b: &[f64], x0: &[f64]) -> Option<crate::solver::lp::LpSolution> {
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

fn clip_flags(e: &Mat, f: &[f64], order: &[usize], lo: &[f64], hi: &[f64], want: bool) -> Option<(Vec<i8>, Vec<f64>)> {
    crate::census_bump!(ClipCalls, 1);
    if lo.iter().chain(hi).any(|v| !v.is_finite()) {
        crate::census_bump!(ClipInfBox, 1);
        return None;
    }
    if e.rows + 2 * e.cols > 128 {
        crate::census_bump!(ClipManyHyp, 1);
        return None;
    }
    let budget = CLIP_BUDGET.load(AO::Relaxed);
    let mut work = 0u64;
    let r = match e.cols {
        2 => clip_n::<2>(e, f, order, lo, hi, want, budget, &mut work),
        3 => clip_n::<3>(e, f, order, lo, hi, want, budget, &mut work),
        4 => clip_n::<4>(e, f, order, lo, hi, want, budget, &mut work),
        5 => clip_n::<5>(e, f, order, lo, hi, want, budget, &mut work),
        // 6 parameters and more take the LP path: on 6 parameters the clip needs hundreds of vertices
        // (up to 40 ms per region on masses3_N4, and as much again when it hits the 600-vertex guard
        // and fails), while the LP path takes about 0.1 ms; at 5 parameters the clip is still faster.
        _ => {
            crate::census_bump!(ClipNtOut, 1);
            None
        }
    };
    if r.is_some() {
        crate::census_bump!(ClipOk, 1);
    }
    #[cfg(feature = "lp_census")]
    if e.cols <= 5 {
        let b = (64 - work.leading_zeros()) as usize;
        let base = if r.is_some() { crate::census_diag::s::PF_WORK_OK } else { crate::census_diag::s::PF_WORK_FAIL };
        crate::census_diag::add(base + b.min(27), 1);
    }
    r
}

/// Work budget of one vertex clip (vertex-row products, edge pairs examined and duplicate scans, counted in `clip_n`).
/// A clip that exceeds it gives up and the region takes the LP path, which costs about 0.1 to 0.3 ms per region. At
/// about 1.5 ns per unit, 2^18 units are about 0.4 ms. Census over 60 problems at 2 to 5 parameters
/// (`bench/results/mplcp_clip_budget.md`): 1,430,000 successful clips, of which 3 needed more than 2^18 units, against
/// 1,500,000,000 and more units for the clips that failed at the 600-vertex guard (12 to 170 ms each). `u64::MAX` = no budget.
static CLIP_BUDGET: AtomicU64 = AtomicU64::new(super::DEFAULT_CLIP_BUDGET);

pub(super) fn set_clip_budget(b: u64) {
    CLIP_BUDGET.store(b, AO::Relaxed);
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

/// Vertex clipping of the box by the rows: per row 1 when it is tight at a vertex of box ∩ region,
/// 0 when it clears every vertex by 1e-7 (relative), -1 when undecided.
fn clip_n<const N: usize>(e: &Mat, f: &[f64], order: &[usize], lo: &[f64], hi: &[f64], want: bool, budget: u64, work: &mut u64) -> Option<(Vec<i8>, Vec<f64>)> {
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
    // one bit per distinct hyperplane: rows equal to a box face or to an earlier processed row
    // share its bit (looked up only for the rows that reach the cutting stage)
    let mut bit: Vec<usize> = (0..m).collect();
    let hkey = |a: &[f64], fr: f64| -> [i64; 7] {
        let mut k = [0i64; 7];
        for (o, v) in k.iter_mut().zip(a) {
            *o = (v * 1e8).round() as i64;
        }
        k[6] = (fr * 1e8).round() as i64;
        k
    };
    let mut seen: crate::fxhash::FxHashMap<[i64; 7], usize> = crate::fxhash::FxHashMap::with_capacity_and_hasher(2 * N + 16, Default::default());
    {
        let mut unit = [0.0; N];
        for k in 0..N {
            unit[k] = 1.0;
            seen.insert(hkey(&unit, hi[k]), m + 2 * k + 1);
            unit[k] = -1.0;
            seen.insert(hkey(&unit, -lo[k]), m + 2 * k);
            unit[k] = 0.0;
        }
    }
    let mut cur_lo = [0.0; N];
    let mut cur_hi = [0.0; N];
    cur_lo.copy_from_slice(&lo[..N]);
    cur_hi.copy_from_slice(&hi[..N]);
    let mut npts: Vec<[f64; N]> = Vec::with_capacity(64);
    let mut ntight: Vec<u128> = Vec::with_capacity(64);
    let mut val: Vec<f64> = Vec::with_capacity(64);
    let mut ins: Vec<usize> = vec![];
    let mut outs: Vec<usize> = vec![];
    for &r in order {
        let a = e.row(r);
        let fr = f[r];
        let tol = 1e-8 * (1.0 + fr.abs());
        // every vertex is inside the running bounding box: a row clearing it by more than tol
        // neither cuts nor touches a vertex
        let mut mx = 0.0;
        for k in 0..N {
            mx += if a[k] > 0.0 { a[k] * cur_hi[k] } else { a[k] * cur_lo[k] };
        }
        if mx - fr < -tol * 1.0000001 {
            continue;
        }
        let nv = pts.len();
        *work += nv as u64;
        val.clear();
        ins.clear();
        outs.clear();
        match seen.get(&hkey(a, fr)) {
            Some(&b) => {
                bit[r] = b;
                crate::census_bump!(ClipDupRows, 1);
            }
            None => {
                seen.insert(hkey(a, fr), r);
            }
        }
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
            if *work > budget {
                #[cfg(feature = "lp_census")]
                crate::census_diag::add(crate::census_diag::s::PF_CLIP_BUDGET_ABORTS, 1);
                return None;
            }
            for &w in &outs {
                let common = tight[u] & tight[w];
                if (common.count_ones() as usize) < N - 1 {
                    continue;
                }
                *work += npts.len() as u64 + 1;
                let t = val[u] / (val[u] - val[w]);
                let mut px = [0.0; N];
                for k in 0..N {
                    px[k] = pts[u][k] + t * (pts[w][k] - pts[u][k]);
                }
                let bits = common | rbit;
                let mut dup = false;
                for idx in 0..npts.len() {
                    if idx >= kept || ntight[idx] & rbit != 0 {
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
        if *work > budget {
            #[cfg(feature = "lp_census")]
            crate::census_diag::add(crate::census_diag::s::PF_CLIP_BUDGET_ABORTS, 1);
            return None;
        }
        if npts.len() <= N {
            crate::census_bump!(ClipFewVerts, 1);
            return None;
        }
        std::mem::swap(&mut pts, &mut npts);
        std::mem::swap(&mut tight, &mut ntight);
        cur_lo = [f64::INFINITY; N];
        cur_hi = [f64::NEG_INFINITY; N];
        for x in &pts {
            for k in 0..N {
                cur_lo[k] = cur_lo[k].min(x[k]);
                cur_hi[k] = cur_hi[k].max(x[k]);
            }
        }
    }
    let mut out = Vec::with_capacity(m);
    for r in 0..m {
        let a = e.row(r);
        let fp = f[r];
        let sc = 1.0 + fp.abs();
        let mut mx = 0.0;
        for k in 0..N {
            mx += if a[k] > 0.0 { a[k] * cur_hi[k] } else { a[k] * cur_lo[k] };
        }
        if fp - mx >= 1e-7 * sc * 1.0000001 {
            out.push(0);
            continue;
        }
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
    // the vertices of box ∩ region (the box contains the parameter set and its rows are in `e`,
    // so these are the vertices of the region itself)
    let mut flat = Vec::with_capacity(if want { pts.len() * N } else { 0 });
    if want {
        for x in &pts {
            flat.extend_from_slice(x);
        }
    }
    Some((out, flat))
}
