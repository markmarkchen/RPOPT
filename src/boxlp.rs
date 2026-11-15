//! Exact bounding box of a polytope {x : A x <= b} by 2n LPs chained through one vertex (for
//! mplcp's region builder).
//!
//! The first LP is the engine's (`lp_point_from` from the interior point), unless the caller hands
//! over a vertex of the polytope (a vertex of the parent's box that lies on the shared facet). The
//! vertex gives a basis of n independent tight rows; every further objective is optimised by a
//! primal simplex from the previous optimum. The state is the basis inverse W (A_T W = I), the
//! multipliers y = W' c, the product R = A W (so the ratio test needs no dot product: a_j.d =
//! -R[j][leaving]) and the slacks s = b - A x. One pivot is a rank-one update of W, R, y and s.
//!
//! What is checked at every accepted optimum, from the data and not from the updated state: the
//! vertex is recomputed as x = W b_T; every slack b_j - a_j.x is recomputed (O(m n)) and must be at
//! least -1e-8 (1 + |b_j|), so x is feasible up to that margin. The bound is then certified by the
//! multipliers: c = s e_k, y = W' c, r = c - sum_i y_i a_{T_i} (computed from the rows, not from R). For
//! every point x' of the polytope, c.x' <= c.x + g with
//!   g = sum_i y_i^+ |b_{T_i} - a_{T_i}.x| + sum_i y_i^- sqrt(n) D + |r|_1 D,
//! where D bounds ||x' - x||_inf over the outer box of the caller (the propagated parameter box, passed in: a valid
//! bound before any objective of the chain has been optimised, and it contains the polytope the caller screens) and y^- are the negative multipliers (the
//! simplex stops with y_i >= -1e-10). The optimum is accepted when g <= 1e-6 (1 + |c.x|); the box
//! returned to the caller is widened by g (lo_k - g, hi_k + g for the objective's side), so the
//! screening that uses it stays sound however ill-conditioned the basis was, and a large g only costs
//! screening power. The multipliers y and the product R are carried through the rank-one updates and
//! rebuilt from the basis every 50 pivots. When a check fails, W, R, y and the vertex are rebuilt from the
//! basis rows once (the updates drift when the basis is ill-conditioned) and the optimum is looked at
//! again: a rebuilt vertex that is infeasible means the simplex took a wrong step and the objective goes
//! to the engine; a rebuilt state with a negative multiplier goes on pivoting. Any other failure (no
//! blocking row, singular pivot, iteration guard) sends that objective, and a restart of the chain, to
//! the engine; so does a state that cannot be formed at the engine's optimum (that optimum can lie inside
//! a face and not at a vertex), which is counted in `Stats::init_failures`. Degeneracy:
//! pivots with a zero step count towards a switch to Bland's rule (smallest row index leaves,
//! smallest index among tied ratios enters), which cannot cycle. The box only screens rows in the
//! caller, whose later stages are exact, so a fallback or a looser value costs LPs, not
//! correctness.

use crate::linalg::{dot, Lu, Mat};
use std::cell::RefCell;

/// The result in the engine's order: lo, hi and the 2n optimal points [min x0, max x0, min x1, ...].
pub type BoxPts = (Vec<f64>, Vec<f64>, Vec<Vec<f64>>);

const GZ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// Reporting only: certified gaps g of the objectives the chain solved, by floor(log10 g) from 1e-16 (bucket 0, also g = 0) to 1e-6 (bucket 10) and above.
static GAP_HIST: [std::sync::atomic::AtomicU64; 12] = [GZ; 12];

fn note_gap(g: f64) {
    let b = if g <= 1e-16 { 0 } else { ((g.log10().floor() + 17.0).max(0.0) as usize).min(11) };
    GAP_HIST[b].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// Clears the gap histogram (call before a solve whose gaps are to be reported).
pub fn reset_gap_hist() {
    for a in GAP_HIST.iter() {
        a.store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

/// (objectives solved by the chain, median bucket upper bound of g, share of g above 1e-9, share of g above 1e-7) since the last reset.
pub fn gap_summary() -> (u64, f64, f64, f64) {
    let h: Vec<u64> = GAP_HIST.iter().map(|a| a.load(std::sync::atomic::Ordering::Relaxed)).collect();
    let tot: u64 = h.iter().sum();
    if tot == 0 {
        return (0, 0.0, 0.0, 0.0);
    }
    let mut acc = 0;
    let mut med = 0.0;
    for (b, &c) in h.iter().enumerate() {
        acc += c;
        if acc * 2 >= tot {
            med = 10f64.powi(b as i32 - 16);
            break;
        }
    }
    // bucket b holds g in [10^(b-17), 10^(b-16))
    let above = |from: usize| h[from..].iter().sum::<u64>() as f64 / tot as f64;
    (tot, med, above(8), above(10))
}

/// Per-call statistics (for the census and the fallback guard).
#[derive(Default, Clone, Copy, Debug)]
pub struct Stats {
    pub steps: u64,
    pub engine_lps: u64,
    /// objectives whose rank-one simplex failed (verification, no blocking row, singular pivot, iteration guard)
    pub fallbacks: u64,
    /// failed attempts to form a state at a vertex (the start, or an engine optimum), each followed by engine LPs for the later objectives of the chain
    pub init_failures: u64,
    /// 1 when the caller gave no finite outer box and the chain was not used at all
    pub no_outer: u64,
    /// failed verifications that the rebuild of the state resolved (verified, or the simplex went on)
    pub rebuilt: u64,
}

#[derive(Default)]
struct Scr {
    t: Vec<usize>,
    w: Vec<f64>,
    r: Vec<f64>,
    s: Vec<f64>,
    y: Vec<f64>,
    alpha: Vec<f64>,
    x: Vec<f64>,
    valid: bool,
    s_stale: bool,
    since_refresh: usize,
}

thread_local! {
    static SCR: RefCell<Scr> = RefCell::new(Scr::default());
}

fn engine_lp(e: &Mat, f: &[f64], x0: &[f64], j: usize) -> Option<Vec<f64>> {
    let n = e.cols;
    let mut obj = vec![0.0; n];
    obj[j / 2] = if j % 2 == 0 { 1.0 } else { -1.0 };
    crate::solver::lp::lp_point_from(Some(&obj), e, f, &[], x0)
}

/// R = A W (row-major m x n).
fn fill_r(e: &Mat, w: &[f64], r: &mut Vec<f64>) {
    let (m, n) = (e.rows, e.cols);
    r.clear();
    r.resize(m * n, 0.0);
    for j in 0..m {
        let a = e.row(j);
        let rj = &mut r[j * n..(j + 1) * n];
        for q in 0..n {
            let aq = a[q];
            if aq == 0.0 {
                continue;
            }
            let wq = &w[q * n..(q + 1) * n];
            for i in 0..n {
                rj[i] += aq * wq[i];
            }
        }
    }
}

/// W = (A_T)^-1 for the rows `t`; false when singular.
fn fill_w(e: &Mat, t: &[usize], w: &mut Vec<f64>) -> bool {
    let n = e.cols;
    let mut a = Mat::zeros(n, n);
    for (i, &r) in t.iter().enumerate() {
        a.row_mut(i).copy_from_slice(e.row(r));
    }
    let Some(lu) = Lu::new(&a) else { return false };
    w.clear();
    w.resize(n * n, 0.0);
    let mut ei = vec![0.0; n];
    for i in 0..n {
        ei[i] = 1.0;
        let col = lu.solve(&ei);
        ei[i] = 0.0;
        for q in 0..n {
            w[q * n + i] = col[q];
        }
    }
    true
}

/// Sets the state from a vertex x: n independent tight rows (greedy Gram-Schmidt), W, R.
fn init_state(e: &Mat, f: &[f64], x: &[f64], sc: &mut Scr) -> bool {
    let n = e.cols;
    let mut q: Vec<Vec<f64>> = vec![];
    sc.t.clear();
    for r in 0..e.rows {
        if (dot(e.row(r), x) - f[r]).abs() > 1e-9 * (1.0 + f[r].abs()) {
            continue;
        }
        let mut v = e.row(r).to_vec();
        for u in &q {
            let p = dot(&v, u);
            for k in 0..n {
                v[k] -= p * u[k];
            }
        }
        let nv = crate::linalg::norm(&v);
        if nv > 1e-6 {
            for k in 0..n {
                v[k] /= nv;
            }
            q.push(v);
            sc.t.push(r);
            if sc.t.len() == n {
                break;
            }
        }
    }
    if sc.t.len() < n {
        sc.valid = false;
        return false;
    }
    let t = std::mem::take(&mut sc.t);
    let ok = fill_w(e, &t, &mut sc.w);
    sc.t = t;
    if !ok {
        sc.valid = false;
        return false;
    }
    fill_r(e, &sc.w, &mut sc.r);
    sc.x.clear();
    sc.x.extend_from_slice(x);
    sc.since_refresh = 0;
    sc.s_stale = true;
    sc.valid = true;
    true
}

/// Rebuilds W, R, y, the vertex and the slacks from the basis rows. False when the basis is singular or the rebuilt vertex is
/// infeasible (by more than the slack margin).
fn rebuild_state(e: &Mat, f: &[f64], sc: &mut Scr, k: usize, sg: f64) -> bool {
    let (m, n) = (e.rows, e.cols);
    let t = std::mem::take(&mut sc.t);
    let ok = fill_w(e, &t, &mut sc.w);
    sc.t = t;
    if !ok {
        return false;
    }
    fill_r(e, &sc.w, &mut sc.r);
    sc.since_refresh = 0;
    sc.y.clear();
    sc.y.extend((0..n).map(|i| sg * sc.w[k * n + i]));
    for q in 0..n {
        sc.x[q] = 0.0;
    }
    for i in 0..n {
        let bi = f[sc.t[i]];
        for q in 0..n {
            sc.x[q] += bi * sc.w[q * n + i];
        }
    }
    sc.s.clear();
    sc.s.extend((0..m).map(|j| f[j] - dot(e.row(j), &sc.x)));
    sc.s_stale = false;
    !sc.s.iter().enumerate().any(|(j, &sj)| sj < -1e-8 * (1.0 + f[j].abs()))
}

/// The optimum check from the state: recomputes the vertex and the slacks, then the certified gap g of the bound (see the header).
/// None when a slack is below the margin or g is above the cap.
fn certify(e: &Mat, f: &[f64], sc: &mut Scr, k: usize, sg: f64, outer: (&[f64], &[f64])) -> Option<f64> {
    let (m, n) = (e.rows, e.cols);
    for q in 0..n {
        sc.x[q] = 0.0;
    }
    for i in 0..n {
        let bi = f[sc.t[i]];
        for q in 0..n {
            sc.x[q] += bi * sc.w[q * n + i];
        }
    }
    for j in 0..m {
        let sj = f[j] - dot(e.row(j), &sc.x);
        if sj < -1e-8 * (1.0 + f[j].abs()) {
            return None;
        }
        sc.s[j] = sj;
    }
    // D bounds ||x' - x||_inf for every x' of the outer box (which contains the polytope of the caller)
    let diam = (0..n).fold(0.0f64, |m, q| m.max((sc.x[q] - outer.0[q]).abs()).max((outer.1[q] - sc.x[q]).abs()));
    let mut g = 0.0;
    let sqrt_n = (n as f64).sqrt();
    for i in 0..n {
        let yi = sc.y[i];
        if yi >= 0.0 {
            g += yi * sc.s[sc.t[i]].abs();
        } else {
            g += -yi * sqrt_n * diam;
        }
    }
    let mut r1 = 0.0;
    for q in 0..n {
        let mut r = if q == k { sg } else { 0.0 };
        for i in 0..n {
            r -= sc.y[i] * e[(sc.t[i], q)];
        }
        r1 += r.abs();
    }
    g += r1 * diam;
    if !(g <= 1e-6 * (1.0 + (sg * sc.x[k]).abs())) {
        return None;
    }
    Some(g)
}

/// Primal simplex for max s.e_k from the current vertex of `sc`; the certified gap of the bound on success, None on failure (state
/// invalid). `outer` is a box that contains the polytope of the caller; the second result is true
/// when a failed check was resolved by rebuilding the state.
fn optimise(e: &Mat, f: &[f64], sc: &mut Scr, k: usize, sg: f64, outer: (&[f64], &[f64]), steps: &mut u64) -> (Option<f64>, bool) {
    let (m, n) = (e.rows, e.cols);
    if sc.since_refresh > 50 {
        // rebuild W and R from the basis rows
        let t = std::mem::take(&mut sc.t);
        let ok = fill_w(e, &t, &mut sc.w);
        sc.t = t;
        if !ok {
            return (None, false);
        }
        fill_r(e, &sc.w, &mut sc.r);
        sc.since_refresh = 0;
        sc.s_stale = true;
    }
    sc.y.clear();
    sc.y.extend((0..n).map(|i| sg * sc.w[k * n + i]));
    if sc.s.len() != m || sc.s_stale {
        sc.s.clear();
        sc.s.extend((0..m).map(|j| f[j] - dot(e.row(j), &sc.x)));
        sc.s_stale = false;
    }
    sc.alpha.clear();
    sc.alpha.resize(n, 0.0);
    let mut degenerate = 0usize;
    let mut rebuilt = false;
    let mut resolved = false;
    for _ in 0..(20 * (n + m) + 100) {
        let bland = degenerate > 8;
        let mut li = usize::MAX;
        for i in 0..n {
            if sc.y[i] < -1e-10 && (li == usize::MAX || (bland && sc.t[i] < sc.t[li]) || (!bland && sc.y[i] < sc.y[li])) {
                li = i;
            }
        }
        if li == usize::MAX {
            // optimal: certify the bound from the data
            if let Some(g) = certify(e, f, sc, k, sg, outer) {
                return (Some(g), resolved);
            }
            if rebuilt {
                return (None, resolved);
            }
            // the updates may have drifted: rebuild from the basis rows once and look again
            rebuilt = true;
            if !rebuild_state(e, f, sc, k, sg) {
                return (None, false);
            }
            resolved = true;
            continue;
        }
        // ratio test on the column li of R
        let mut best = usize::MAX;
        let mut bt = f64::INFINITY;
        for j in 0..m {
            let rjl = sc.r[j * n + li];
            if rjl < -1e-11 {
                let tt = sc.s[j].max(0.0) / -rjl;
                if tt < bt - 1e-12 * (1.0 + bt.min(1e300)) {
                    bt = tt;
                    best = j;
                } else if bland && best != usize::MAX && tt <= bt + 1e-12 * (1.0 + bt.min(1e300)) && j < best {
                    best = j;
                    bt = bt.min(tt);
                }
            }
        }
        if best == usize::MAX {
            return (None, false);
        }
        if bt < 1e-12 {
            degenerate += 1;
        } else {
            degenerate = 0;
        }
        let ali = sc.r[best * n + li];
        if ali.abs() < 1e-11 {
            return (None, false);
        }
        for i in 0..n {
            sc.alpha[i] = sc.r[best * n + i];
        }
        // slacks and R
        for j in 0..m {
            let rj = &mut sc.r[j * n..(j + 1) * n];
            let rjl = rj[li];
            sc.s[j] += bt * rjl;
            if rjl != 0.0 {
                let c = rjl / ali;
                for i in 0..n {
                    rj[i] -= c * sc.alpha[i];
                }
                rj[li] = c;
            }
        }
        sc.s[best] = 0.0;
        // W and y
        for q in 0..n {
            let wq = &mut sc.w[q * n..(q + 1) * n];
            let wl = wq[li];
            if wl != 0.0 {
                let c = wl / ali;
                for i in 0..n {
                    wq[i] -= c * sc.alpha[i];
                }
                wq[li] = c;
            }
        }
        let yl = sc.y[li];
        if yl != 0.0 {
            let c = yl / ali;
            for i in 0..n {
                sc.y[i] -= c * sc.alpha[i];
            }
            sc.y[li] = c;
        }
        sc.t[li] = best;
        sc.since_refresh += 1;
        *steps += 1;
        rebuilt = false;
    }
    (None, false)
}

/// A vertex of the polytope among the flattened points `pts` (n coordinates each) that lies on the
/// hyperplane `ea . x = ef` and satisfies every row; the one that maximizes x[0] (the first
/// objective of the chain).
pub fn shared_start(e: &Mat, f: &[f64], pts: &[f64], ea: &[f64], ef: f64) -> Option<Vec<f64>> {
    let n = e.cols;
    let mut best: Option<&[f64]> = None;
    for p in pts.chunks_exact(n) {
        if (dot(ea, p) - ef).abs() > 1e-9 * (1.0 + ef.abs()) {
            continue;
        }
        if (0..e.rows).any(|j| dot(e.row(j), p) > f[j] + 1e-9 * (1.0 + f[j].abs())) {
            continue;
        }
        if best.map_or(true, |b| p[0] > b[0]) {
            best = Some(p);
        }
    }
    best.map(|p| p.to_vec())
}

/// The box and its 2n points; None only when the engine itself fails. `start` is an optional
/// vertex of the polytope to begin the chain at (no engine LP when its basis can be formed). `outer` is a box that contains the
/// polytope (an interval propagation of its rows): it bounds the distance between any two points of the polytope for the certified
/// gap, and without a finite one the chain is not used and every objective goes to the engine. The returned box is the box of the
/// 2n points, widened by the certified gap of every objective that the chain solved.
pub fn box_chain_in(e: &Mat, f: &[f64], center: &[f64], start: Option<&[f64]>, outer: Option<(&[f64], &[f64])>, stats: &mut Stats) -> Option<BoxPts> {
    let n = e.cols;
    let chain = outer.map_or(false, |(lo, hi)| lo.iter().chain(hi).all(|v| v.is_finite()));
    stats.no_outer += (!chain) as u64;
    let ob = outer.unwrap_or((&[], &[]));
    // order: all maxima, then all minima reversed (adjacent objectives are close)
    let mut order: Vec<usize> = (0..n).map(|k| 2 * k + 1).collect();
    order.extend((0..n).rev().map(|k| 2 * k));
    let mut pts: Vec<Vec<f64>> = vec![vec![]; 2 * n];
    let mut gaps = vec![0.0f64; 2 * n];
    SCR.with(|cell| {
        let sc = &mut *cell.borrow_mut();
        sc.valid = false;
        if let (true, Some(x0)) = (chain, start) {
            if !init_state(e, f, x0, sc) {
                stats.init_failures += 1;
            }
        }
        for &j in &order {
            let (k, sg) = (j / 2, if j % 2 == 1 { 1.0 } else { -1.0 });
            let mut done = false;
            if sc.valid {
                match optimise(e, f, sc, k, sg, ob, &mut stats.steps) {
                    (Some(g), resolved) => {
                        pts[j] = sc.x.clone();
                        gaps[j] = g;
                        note_gap(g);
                        stats.rebuilt += resolved as u64;
                        done = true;
                    }
                    (None, _) => {
                        stats.fallbacks += 1;
                        sc.valid = false;
                    }
                }
            }
            if !done {
                stats.engine_lps += 1;
                let x = engine_lp(e, f, center, j)?;
                if chain && !init_state(e, f, &x, sc) {
                    stats.init_failures += 1;
                }
                pts[j] = x;
            }
        }
        let mut lo = vec![0.0; n];
        let mut hi = vec![0.0; n];
        for k in 0..n {
            lo[k] = pts[2 * k][k] - gaps[2 * k];
            hi[k] = pts[2 * k + 1][k] + gaps[2 * k + 1];
        }
        Some((lo, hi, std::mem::take(&mut pts)))
    })
}

/// `box_chain_in` with the outer box taken from an interval propagation of the rows alone.
pub fn box_chain(e: &Mat, f: &[f64], center: &[f64], start: Option<&[f64]>, stats: &mut Stats) -> Option<BoxPts> {
    let n = e.cols;
    let mut lo = vec![f64::NEG_INFINITY; n];
    let mut hi = vec![f64::INFINITY; n];
    crate::mpqp_utils::propagate_box_pub(e, f, &mut lo, &mut hi, 6);
    box_chain_in(e, f, center, start, Some((&lo, &hi)), stats)
}
