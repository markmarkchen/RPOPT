//! Fast path for the commonest LP of the mp algorithms: feasibility of {A x <= b} (no equality
//! rows, no objective) in dimension N <= 4, such as the KKT-region test of a candidate active set.
//!
//! Same method as `lp_core` (primal simplex on the dual min b'y, A'y = 0, y >= 0, started from the
//! artificial basis), as a revised simplex: only the N x N basis inverse and the multipliers are
//! updated, reduced costs are recomputed from the rows of A. The answer is verified on the original data (the
//! point by a row-wise check, infeasibility by the Farkas ray); if a check fails, `None` is
//! returned as "undecided" and the caller falls back to the general solver.

use crate::linalg::Mat;

pub enum Small {
    Feasible(Vec<f64>),
    Infeasible,
    Undecided,
}

const PIVOT_TOL: f64 = 1e-9;

const MAX_DEGENERATE_BEFORE_BLAND: usize = 30;

/// Dispatch on the dimension (1..=4); larger problems are undecided.
pub fn feasible(a: &Mat, b: &[f64]) -> Small {
    match a.cols {
        1 => solve::<1>(a, b),
        2 => solve::<2>(a, b),
        3 => solve::<3>(a, b),
        4 => solve::<4>(a, b),
        _ => Small::Undecided,
    }
}

fn solve<const N: usize>(a: &Mat, b: &[f64]) -> Small {
    // Revised simplex on the dual  min b'y, A'y = 0, y >= 0, from the artificial basis. Only the
    // basis inverse (N x N) and the multipliers pi are kept; reduced costs d_j = b_j - pi'a_j are
    // recomputed from the contiguous rows of A in each pricing pass (reads only).
    let m = a.rows;
    let rows = &a.data[..m * N];
    let mut inv: Vec<f64> = Vec::with_capacity(m);
    for &bj in &b[..m] {
        inv.push(1.0 / (1.0 + bj.abs()));
    }
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
                let mut d = b[j];
                for t in 0..N {
                    d -= pi[t] * row[t];
                }
                if d * inv[j] < -1e-10 {
                    enter = j;
                    de = d;
                    break;
                }
            }
        } else {
            let mut best = -1e-10;
            for (j, row) in rows.chunks_exact(N).enumerate() {
                let mut d = b[j];
                for t in 0..N {
                    d -= pi[t] * row[t];
                }
                let v = d * inv[j];
                if v < best {
                    best = v;
                    enter = j;
                    de = d;
                }
            }
        }
        if enter == usize::MAX {
            // optimal: x = pi (sigma = +1 since c = 0); verify on the original rows
            for (i, row) in rows.chunks_exact(N).enumerate() {
                let mut ax = 0.0;
                let mut mag = 0.0;
                for t in 0..N {
                    let p = row[t] * pi[t];
                    ax += p;
                    mag += p.abs();
                }
                if ax - b[i] > 1e-9 * (1.0 + b[i].abs() + mag) {
                    return Small::Undecided;
                }
            }
            return Small::Feasible(pi.to_vec());
        }
        // entering column in the current basis: u = B^{-1} a_e
        let ae = &rows[enter * N..(enter + 1) * N];
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
    Small::Undecided
}

/// Farkas ray for the revised method: y_e = 1, y_B = -u (u = B^{-1} a_e).
fn certificate_revised<const N: usize>(a: &Mat, b: &[f64], u: &[f64; N], basis: &[usize; N], e: usize, m: usize) -> Small {
    let mut idx = [0usize; 5];
    let mut val = [0.0f64; 5];
    let mut cnt = 1;
    idx[0] = e;
    val[0] = 1.0;
    for r in 0..N {
        let v = -u[r];
        if basis[r] >= m {
            if v.abs() > 1e-9 {
                return Small::Undecided;
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
            return Small::Undecided;
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
            return Small::Undecided;
        }
    }
    if by < -1e-12 * ysum { Small::Infeasible } else { Small::Undecided }
}
