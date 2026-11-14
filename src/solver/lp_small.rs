//! Fast path for the commonest LP of the mp algorithms: feasibility of {A x <= b} (no equality
//! rows, no objective) in dimension N <= 4, such as the KKT-region test of a candidate active set.
//!
//! Same method as `lp_core` (primal simplex on the dual min b'y, A'y = 0, y >= 0, started from the
//! artificial basis), as a revised simplex: only the N x N basis inverse and the multipliers are
//! updated, reduced costs are recomputed from the rows of A. The answer is verified on the original data (the
//! point by a row-wise check, infeasibility by the Farkas ray); if a check fails, `None` is
//! returned as "undecided" and the caller falls back to the general solver.
//!
//! The pricing pass computes every reduced cost d_j = b_j - pi'a_j; it stops the method only when
//! none is below -1e-10 (1 + |b_j|), which implies the row-wise check of the point (tolerance
//! 1e-9 (1 + |b_j| + sum_t |a_jt pi_t|)) whenever pi is finite (rounding of the two sums differs
//! by about N eps (|b_j| + sum_t |a_jt pi_t|)), so the check is a by-product of the last pass.
//! Pricing, ratio test and pivots are unchanged, so every point and decision of the simplex is
//! the same as before (no heap allocation, no separate check pass). A bound screen runs first
//! (`bound_screen`): it decides, with a margin, the LPs that a single row makes infeasible
//! inside the variable bounds (about 70% of the region tests).

use crate::linalg::Mat;

pub enum Small {
    Feasible(Vec<f64>),
    Infeasible,
    Undecided,
}

/// `Small` without the heap: the point lives in a fixed array (first `a.cols` entries).
pub(crate) enum Verdict {
    Feasible([f64; 4]),
    Infeasible,
    Undecided,
}

const PIVOT_TOL: f64 = 1e-9;

const MAX_DEGENERATE_BEFORE_BLAND: usize = 30;

/// Rows up to which the scaling factors live on the stack.
const STACK_ROWS: usize = 64;

/// Dispatch on the dimension (1..=4); larger problems are undecided.
pub fn feasible(a: &Mat, b: &[f64]) -> Small {
    match verdict(a, b) {
        Verdict::Feasible(x) => Small::Feasible(x[..a.cols].to_vec()),
        Verdict::Infeasible => Small::Infeasible,
        Verdict::Undecided => Small::Undecided,
    }
}

/// `feasible` without allocating.
pub(crate) fn verdict(a: &Mat, b: &[f64]) -> Verdict {
    match a.cols {
        1 => solve::<1>(a, b),
        2 => solve::<2>(a, b),
        3 => solve::<3>(a, b),
        4 => solve::<4>(a, b),
        _ => Verdict::Undecided,
    }
}

/// Bound screen. Rows with a single nonzero coefficient bound one variable each (every region LP
/// carries the parameter box); if a row's minimum over those bounds exceeds its right hand side,
/// the row and the bound rows of its minimizing corner form a Farkas ray. The margin, 1e-7 (1 +
/// |b_g| + sum_k |a_gk| (1/|c_k| + 3 B_k)) with c_k the bound row's coefficient and B_k the
/// largest bound magnitude, exceeds 100 times the sum of the verification tolerances of those
/// rows weighted by the ray, so no point can pass the simplex's (or the tableau's) row check:
/// the simplex would answer Infeasible too. Without a hit the simplex runs as before.
///
/// The bounds are read from the trailing rows with one nonzero (region LPs list the parameter
/// box last); the screen needs a finite lower and upper bound on every variable.
#[inline(always)]
fn bound_screen<const N: usize>(rows: &[f64], b: &[f64]) -> bool {
    let m = b.len();
    // (a constant fill pattern makes LLVM call memset_pattern16 for these tiny arrays)
    let inf = std::hint::black_box(f64::INFINITY);
    let mut lo = [-inf; N];
    let mut hi = [inf; N];
    let mut clo = [0.0f64; N];
    let mut chi = [0.0f64; N];
    let mut g = m;
    while g > 0 {
        let row = &rows[(g - 1) * N..g * N];
        let mut nz = 0;
        let mut k1 = 0;
        for (k, &v) in row.iter().enumerate() {
            if v != 0.0 {
                nz += 1;
                k1 = k;
            }
        }
        if nz != 1 {
            break;
        }
        let c = row[k1];
        let bound = b[g - 1] / c;
        if c > 0.0 {
            if bound < hi[k1] {
                hi[k1] = bound;
                chi[k1] = c;
            }
        } else if bound > lo[k1] {
            lo[k1] = bound;
            clo[k1] = -c;
        }
        g -= 1;
    }
    if g == 0 || m - g < 2 * N {
        return false;
    }
    let mut w = [0.0f64; N];
    for k in 0..N {
        if !(lo[k].is_finite() && hi[k].is_finite()) {
            return false;
        }
        // weight of |a_gk| in the margin: tolerance of the bound row (1/c) and |x_k| <= B_k
        w[k] = (1.0 / clo[k]).max(1.0 / chi[k]) + 3.0 * lo[k].abs().max(hi[k].abs());
    }
    for (i, row) in rows[..g * N].chunks_exact(N).enumerate() {
        let mut mn = 0.0;
        let mut scale = 1.0 + b[i].abs();
        for k in 0..N {
            let v = row[k];
            mn += v * if v > 0.0 { lo[k] } else { hi[k] };
            scale += v.abs() * w[k];
        }
        if mn - b[i] > 1e-7 * scale {
            return true;
        }
    }
    false
}

fn solve<const N: usize>(a: &Mat, b: &[f64]) -> Verdict {
    let m = a.rows;
    if bound_screen::<N>(&a.data[..m * N], &b[..m]) {
        return Verdict::Infeasible;
    }
    if m <= STACK_ROWS {
        let mut buf = [std::mem::MaybeUninit::<f64>::uninit(); STACK_ROWS];
        for (v, bj) in buf.iter_mut().zip(&b[..m]) {
            v.write(1.0 / (1.0 + bj.abs()));
        }
        // SAFETY: the first m entries were written above (b has at least m entries)
        let inv = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const f64, m) };
        solve_with::<N>(a, b, inv)
    } else {
        let inv: Vec<f64> = b[..m].iter().map(|bj| 1.0 / (1.0 + bj.abs())).collect();
        solve_with::<N>(a, b, &inv)
    }
}

/// d_j = b_j - pi'a_j, in the order of the original sequential loop (the pricing decisions
/// depend on its rounding).
#[inline(always)]
fn reduced<const N: usize>(bj: f64, row: &[f64], pi: &[f64; N]) -> f64 {
    let row: &[f64; N] = row.try_into().unwrap();
    let mut d = bj;
    for t in 0..N {
        d -= pi[t] * row[t];
    }
    d
}

fn solve_with<const N: usize>(a: &Mat, b: &[f64], inv: &[f64]) -> Verdict {
    // Revised simplex on the dual  min b'y, A'y = 0, y >= 0, from the artificial basis. Only the
    // basis inverse (N x N) and the multipliers pi are kept; reduced costs d_j = b_j - pi'a_j are
    // recomputed from the contiguous rows of A in each pricing pass (reads only).
    let m = a.rows;
    let rows = &a.data[..m * N];
    let b = &b[..m];
    // basis: column index (m + k = artificial e_k), basis inverse, multipliers
    let mut basis = [0usize; N];
    for (k, bk) in basis.iter_mut().enumerate() {
        *bk = m + k;
    }
    let mut binv = [[0.0f64; N]; N];
    for (k, row) in binv.iter_mut().enumerate() {
        row[k] = 1.0;
    }
    let mut pi = [0.0f64; N];
    let max_iter = 50 * (m + N) + 1000;
    let mut degenerate = 0usize;
    for _ in 0..max_iter {
        let bland = degenerate > MAX_DEGENERATE_BEFORE_BLAND;
        let mut enter = usize::MAX;
        let mut de = 0.0;
        if bland {
            for (j, row) in rows.chunks_exact(N).enumerate() {
                let d = reduced::<N>(b[j], row, &pi);
                if d * inv[j] < -1e-10 {
                    enter = j;
                    de = d;
                    break;
                }
            }
        } else {
            let mut best = -1e-10;
            for (j, row) in rows.chunks_exact(N).enumerate() {
                let d = reduced::<N>(b[j], row, &pi);
                let v = d * inv[j];
                if v < best {
                    best = v;
                    enter = j;
                    de = d;
                }
            }
        }
        if enter == usize::MAX {
            // optimal: x = pi (sigma = +1 since c = 0); the last pass verified every row
            if !pi.iter().all(|p| p.is_finite()) {
                return Verdict::Undecided;
            }
            let mut x = [0.0f64; 4];
            x[..N].copy_from_slice(&pi);
            return Verdict::Feasible(x);
        }
        let ae = &rows[enter * N..(enter + 1) * N];
        // entering column in the current basis: u = B^{-1} a_e
        let mut u = [0.0f64; N];
        for r in 0..N {
            let mut v = 0.0;
            for t in 0..N {
                v += binv[r][t] * ae[t];
            }
            u[r] = v;
        }
        // ratio test: all basic values are zero, every step is degenerate; largest eligible pivot
        // (Bland: smallest basis index); basic artificials block
        let mut leave = usize::MAX;
        let mut bestp = 0.0;
        for r in 0..N {
            let v = u[r];
            let artificial = basis[r] >= m;
            let eligible = if artificial { v.abs() > PIVOT_TOL } else { v > PIVOT_TOL };
            if !eligible {
                continue;
            }
            let better = if leave == usize::MAX {
                true
            } else if bland {
                basis[r] < basis[leave]
            } else {
                v.abs() > bestp
            };
            if better {
                bestp = v.abs();
                leave = r;
            }
        }
        if leave == usize::MAX {
            return certificate_revised::<N>(a, b, &u, &basis, enter, m);
        }
        degenerate += 1;
        // eta update of B^{-1} and of pi (pi' += (d_e / u_r) * row_r(B^{-1}))
        let ur = u[leave];
        let mut prow = binv[leave];
        for v in prow.iter_mut() {
            *v /= ur;
        }
        for r in 0..N {
            if r == leave {
                continue;
            }
            let f = u[r];
            if f != 0.0 {
                for t in 0..N {
                    binv[r][t] -= f * prow[t];
                }
            }
        }
        binv[leave] = prow;
        for t in 0..N {
            pi[t] += de * prow[t];
        }
        basis[leave] = enter;
    }
    Verdict::Undecided
}

/// Farkas ray for the revised method: y_e = 1, y_B = -u (u = B^{-1} a_e).
fn certificate_revised<const N: usize>(a: &Mat, b: &[f64], u: &[f64; N], basis: &[usize; N], e: usize, m: usize) -> Verdict {
    let mut idx = [0usize; 5];
    let mut val = [0.0f64; 5];
    let mut cnt = 1;
    idx[0] = e;
    val[0] = 1.0;
    for r in 0..N {
        let v = -u[r];
        if basis[r] >= m {
            if v.abs() > 1e-9 {
                return Verdict::Undecided;
            }
            continue;
        }
        if v != 0.0 {
            idx[cnt] = basis[r];
            val[cnt] = v;
            cnt += 1;
        }
    }
    let ysum: f64 = val[..cnt].iter().map(|v| v.abs()).sum::<f64>().max(1e-300);
    let mut amax: f64 = 0.0;
    let mut by = 0.0;
    for q in 0..cnt {
        if val[q] < -1e-9 * ysum {
            return Verdict::Undecided;
        }
        let row = &a.data[idx[q] * N..(idx[q] + 1) * N];
        amax = row.iter().fold(amax, |s, v| s.max(v.abs()));
        by += b[idx[q]] * val[q];
    }
    for k in 0..N {
        let mut s = 0.0;
        for q in 0..cnt {
            s += a.data[idx[q] * N + k] * val[q];
        }
        if s.abs() > 1e-9 * ysum * amax.max(1e-300) {
            return Verdict::Undecided;
        }
    }
    if by < -1e-12 * ysum { Verdict::Infeasible } else { Verdict::Undecided }
}
