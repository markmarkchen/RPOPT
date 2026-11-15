//! Exact bounding box of a polytope {x : A x <= b} by 2n LPs chained through one vertex (for
//! mplcp's region builder). The first LP is the engine's (`lp_point_from` from the interior
//! point); its optimal vertex gives a basis of n independent tight rows, and every further
//! objective is optimised by a primal simplex from the previous optimum, with the basis inverse
//! updated by one rank-one step per pivot.
//!
//! Exactness: every optimum is verified (primal feasible within 1e-8, all multipliers >= -1e-10,
//! so the point is optimal by LP duality) before it is accepted; any failure (no entering row,
//! cycling guard, infeasible or non-optimal result, singular basis) sends that objective, and the
//! rest of the chain, to the engine. Degeneracy: pivots with a zero step count towards a switch to
//! Bland's rule (smallest row index leaves, smallest index among tied ratios enters), which cannot
//! cycle.

use crate::linalg::{dot, Lu, Mat};

/// The result in the engine's order: lo, hi and the 2n optimal points [min x0, max x0, min x1, ...].
pub type BoxPts = (Vec<f64>, Vec<f64>, Vec<Vec<f64>>);

/// Per-call statistics (for the census).
#[derive(Default, Clone, Copy, Debug)]
pub struct Stats {
    pub steps: u64,
    pub engine_lps: u64,
    pub fallbacks: u64,
}

fn engine_lp(e: &Mat, f: &[f64], x0: &[f64], j: usize) -> Option<Vec<f64>> {
    let n = e.cols;
    let mut obj = vec![0.0; n];
    obj[j / 2] = if j % 2 == 0 { 1.0 } else { -1.0 };
    crate::solver::lp::lp_point_from(Some(&obj), e, f, &[], x0)
}

/// n independent tight rows at x (greedy Gram-Schmidt), with the inverse of their matrix.
fn basis_at(e: &Mat, f: &[f64], x: &[f64]) -> Option<(Vec<usize>, Vec<f64>)> {
    let n = e.cols;
    let mut q: Vec<Vec<f64>> = vec![];
    let mut t: Vec<usize> = vec![];
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
            t.push(r);
            if t.len() == n {
                break;
            }
        }
    }
    if t.len() < n {
        return None;
    }
    let mut a = Mat::zeros(n, n);
    for (i, &r) in t.iter().enumerate() {
        a.row_mut(i).copy_from_slice(e.row(r));
    }
    let lu = Lu::new(&a)?;
    // W[q][i] = (A_T^-1 e_i)_q, row-major n x n
    let mut w = vec![0.0; n * n];
    let mut ei = vec![0.0; n];
    for i in 0..n {
        ei[i] = 1.0;
        let col = lu.solve(&ei);
        ei[i] = 0.0;
        for qd in 0..n {
            w[qd * n + i] = col[qd];
        }
    }
    Some((t, w))
}

/// Primal simplex for max c'x with c = s e_k from the vertex (x, t, w); returns false on failure.
fn optimise(e: &Mat, f: &[f64], x: &mut Vec<f64>, t: &mut Vec<usize>, w: &mut Vec<f64>, k: usize, s: f64, steps: &mut u64) -> bool {
    let (m, n) = (e.rows, e.cols);
    let mut y: Vec<f64> = (0..n).map(|i| s * w[k * n + i]).collect();
    let mut in_t = vec![false; m];
    for &r in t.iter() {
        in_t[r] = true;
    }
    let mut degenerate = 0usize;
    let mut d = vec![0.0; n];
    let mut alpha = vec![0.0; n];
    for _ in 0..(20 * (n + m) + 100) {
        // leaving row: most negative multiplier (Bland: smallest row index among the negative ones)
        let bland = degenerate > 8;
        let mut li = usize::MAX;
        for i in 0..n {
            if y[i] < -1e-10 {
                if li == usize::MAX || (bland && t[i] < t[li]) || (!bland && y[i] < y[li]) {
                    li = i;
                }
            }
        }
        if li == usize::MAX {
            // optimal: re-solve the vertex from the basis, then verify
            let mut xs = vec![0.0; n];
            for i in 0..n {
                let bi = f[t[i]];
                for q in 0..n {
                    xs[q] += bi * w[q * n + i];
                }
            }
            for r in 0..m {
                if dot(e.row(r), &xs) > f[r] + 1e-8 * (1.0 + f[r].abs()) {
                    return false;
                }
            }
            // optimality certificate: c = sum y_i a_{t_i} with y >= 0
            for q in 0..n {
                let mut r = if q == k { s } else { 0.0 };
                for i in 0..n {
                    r -= y[i] * e[(t[i], q)];
                }
                if r.abs() > 1e-8 {
                    return false;
                }
            }
            *x = xs;
            return true;
        }
        for q in 0..n {
            d[q] = -w[q * n + li];
        }
        // ratio test
        let mut best = usize::MAX;
        let mut bt = f64::INFINITY;
        for j in 0..m {
            if in_t[j] {
                continue;
            }
            let ad = dot(e.row(j), &d);
            if ad > 1e-11 {
                let tt = (f[j] - dot(e.row(j), x)).max(0.0) / ad;
                if tt < bt - 1e-12 * (1.0 + bt.min(1e300)) {
                    bt = tt;
                    best = j;
                } else if tt <= bt + 1e-12 * (1.0 + bt.min(1e300)) && best != usize::MAX && bland && j < best {
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
        for q in 0..n {
            x[q] += bt * d[q];
        }
        // alpha_i = a_best . w_i
        let a = e.row(best);
        for i in 0..n {
            let mut sacc = 0.0;
            for q in 0..n {
                sacc += a[q] * w[q * n + i];
            }
            alpha[i] = sacc;
        }
        let ai = alpha[li];
        if ai.abs() < 1e-11 {
            return false;
        }
        for q in 0..n {
            let wl = w[q * n + li];
            for i in 0..n {
                if i != li {
                    w[q * n + i] -= wl * alpha[i] / ai;
                }
            }
            w[q * n + li] = wl / ai;
        }
        let yl = y[li];
        for i in 0..n {
            if i != li {
                y[i] -= yl * alpha[i] / ai;
            }
        }
        y[li] = yl / ai;
        in_t[t[li]] = false;
        in_t[best] = true;
        t[li] = best;
        *steps += 1;
    }
    false
}

/// The box and its 2n points; None only when the engine itself fails.
pub fn box_chain(e: &Mat, f: &[f64], center: &[f64], start: Option<&[f64]>, stats: &mut Stats) -> Option<BoxPts> {
    let n = e.cols;
    // order 2: all maxima, then all minima reversed (adjacent objectives are close)
    let mut order: Vec<usize> = (0..n).map(|k| 2 * k + 1).collect();
    order.extend((0..n).rev().map(|k| 2 * k));
    let mut pts: Vec<Vec<f64>> = vec![vec![]; 2 * n];
    let mut state: Option<(Vec<f64>, Vec<usize>, Vec<f64>)> = start.and_then(|x0| basis_at(e, f, x0).map(|(t, w)| (x0.to_vec(), t, w)));
    for &j in &order {
        let (k, s) = (j / 2, if j % 2 == 1 { 1.0 } else { -1.0 });
        let mut done = false;
        if let Some((x, t, w)) = state.as_mut() {
            if optimise(e, f, x, t, w, k, s, &mut stats.steps) {
                pts[j] = x.clone();
                done = true;
            } else {
                stats.fallbacks += 1;
                state = None;
            }
        }
        if !done {
            stats.engine_lps += 1;
            let x = engine_lp(e, f, center, j)?;
            state = basis_at(e, f, &x).map(|(t, w)| (x.clone(), t, w));
            pts[j] = x;
        }
    }
    let mut lo = vec![0.0; n];
    let mut hi = vec![0.0; n];
    for k in 0..n {
        lo[k] = pts[2 * k][k];
        hi[k] = pts[2 * k + 1][k];
    }
    Some((lo, hi, pts))
}
