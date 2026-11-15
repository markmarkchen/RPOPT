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
//! Exactness: every optimum is verified (all rows feasible within 1e-8 at the recomputed vertex,
//! and c a nonnegative combination of the basis rows, so the point is optimal by LP duality)
//! before it is accepted. Any failure (no blocking row, singular pivot, iteration guard,
//! verification) sends that objective, and a restart of the chain, to the engine. Degeneracy:
//! pivots with a zero step count towards a switch to Bland's rule (smallest row index leaves,
//! smallest index among tied ratios enters), which cannot cycle. The box only screens rows in the
//! caller, whose later stages are exact, so a fallback or a looser value costs LPs, not
//! correctness.

use crate::linalg::{dot, Lu, Mat};
use std::cell::RefCell;

/// The result in the engine's order: lo, hi and the 2n optimal points [min x0, max x0, min x1, ...].
pub type BoxPts = (Vec<f64>, Vec<f64>, Vec<Vec<f64>>);

/// Per-call statistics (for the census and the fallback guard).
#[derive(Default, Clone, Copy, Debug)]
pub struct Stats {
    pub steps: u64,
    pub engine_lps: u64,
    pub fallbacks: u64,
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
    lps: usize,
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

/// Primal simplex for max s.e_k from the current vertex of `sc`; false on failure (state invalid).
fn optimise(e: &Mat, f: &[f64], sc: &mut Scr, k: usize, sg: f64, steps: &mut u64, exact_final: bool) -> bool {
    let (m, n) = (e.rows, e.cols);
    if sc.since_refresh > 50 {
        // rebuild W and R from the basis rows
        let t = std::mem::take(&mut sc.t);
        let ok = fill_w(e, &t, &mut sc.w);
        sc.t = t;
        if !ok {
            return false;
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
    for _ in 0..(20 * (n + m) + 100) {
        let bland = degenerate > 8;
        let mut li = usize::MAX;
        for i in 0..n {
            if sc.y[i] < -1e-10 && (li == usize::MAX || (bland && sc.t[i] < sc.t[li]) || (!bland && sc.y[i] < sc.y[li])) {
                li = i;
            }
        }
        if li == usize::MAX {
            // optimal: recompute the vertex from the basis and verify
            for q in 0..n {
                sc.x[q] = 0.0;
            }
            for i in 0..n {
                let bi = f[sc.t[i]];
                for q in 0..n {
                    sc.x[q] += bi * sc.w[q * n + i];
                }
            }
            sc.lps += 1;
            if sc.lps % 4 == 0 || exact_final {
                for j in 0..m {
                    let sj = f[j] - dot(e.row(j), &sc.x);
                    if sj < -1e-8 * (1.0 + f[j].abs()) {
                        return false;
                    }
                    sc.s[j] = sj;
                }
            } else {
                for j in 0..m {
                    if sc.s[j] < -1e-8 * (1.0 + f[j].abs()) {
                        return false;
                    }
                }
            }
            for q in 0..n {
                let mut r = if q == k { sg } else { 0.0 };
                for i in 0..n {
                    r -= sc.y[i] * e[(sc.t[i], q)];
                }
                if r.abs() > 1e-8 {
                    return false;
                }
            }
            return true;
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
            return false;
        }
        if bt < 1e-12 {
            degenerate += 1;
        } else {
            degenerate = 0;
        }
        let ali = sc.r[best * n + li];
        if ali.abs() < 1e-11 {
            return false;
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
    }
    false
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
/// vertex of the polytope to begin the chain at (no engine LP when its basis can be formed).
pub fn box_chain(e: &Mat, f: &[f64], center: &[f64], start: Option<&[f64]>, stats: &mut Stats) -> Option<BoxPts> {
    let n = e.cols;
    // order: all maxima, then all minima reversed (adjacent objectives are close)
    let mut order: Vec<usize> = (0..n).map(|k| 2 * k + 1).collect();
    order.extend((0..n).rev().map(|k| 2 * k));
    let mut pts: Vec<Vec<f64>> = vec![vec![]; 2 * n];
    SCR.with(|cell| {
        let sc = &mut *cell.borrow_mut();
        sc.valid = false;
        if let Some(x0) = start {
            init_state(e, f, x0, sc);
        }
        for &j in &order {
            let (k, sg) = (j / 2, if j % 2 == 1 { 1.0 } else { -1.0 });
            let mut done = false;
            if sc.valid {
                if optimise(e, f, sc, k, sg, &mut stats.steps, j == *order.last().unwrap()) {
                    pts[j] = sc.x.clone();
                    done = true;
                } else {
                    stats.fallbacks += 1;
                    sc.valid = false;
                }
            }
            if !done {
                stats.engine_lps += 1;
                let x = engine_lp(e, f, center, j)?;
                init_state(e, f, &x, sc);
                pts[j] = x;
            }
        }
        let mut lo = vec![0.0; n];
        let mut hi = vec![0.0; n];
        for k in 0..n {
            lo[k] = pts[2 * k][k];
            hi[k] = pts[2 * k + 1][k];
        }
        Some((lo, hi, std::mem::take(&mut pts)))
    })
}
