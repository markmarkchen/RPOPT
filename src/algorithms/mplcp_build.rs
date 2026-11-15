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
pub(super) type Built = (CriticalRegion, Vec<f64>, Option<Vec<f64>>);

pub(super) fn build_region(program: &MpProgram, active_set: &[usize], hint: Option<&[f64]>, law: Option<crate::program::ControlLaw>, want_verts: bool) -> Option<Built> {
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
    let s = cheb_from(&e, &f, &start_r)?;
    if s.x[nt] <= 1e-8 {
        return None;
    }
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
    let flags: Vec<bool> = match clip_flags(&e, &f, &order, &blo, &bhi, want_verts) {
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
            let r = crate::mpqp_utils::gen_cr_from_prepared(program, active_set, false, crate::mpqp_utils::Prepared { law, n_lambda, inactive, pos, e, f, kept_rows_len: kept_rows.len(), center, cheb_radius }).map(|(cr, c)| (cr, c, None));
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
        },
        center,
        verts,
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
    let r = match e.cols {
        2 => clip_n::<2>(e, f, order, lo, hi, want),
        3 => clip_n::<3>(e, f, order, lo, hi, want),
        4 => clip_n::<4>(e, f, order, lo, hi, want),
        5 => clip_n::<5>(e, f, order, lo, hi, want),
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

/// Vertex clipping of the box by the rows: per row 1 when it is tight at a vertex of box ∩ region,
/// 0 when it clears every vertex by 1e-7 (relative), -1 when undecided.
fn clip_n<const N: usize>(e: &Mat, f: &[f64], order: &[usize], lo: &[f64], hi: &[f64], want: bool) -> Option<(Vec<i8>, Vec<f64>)> {
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
