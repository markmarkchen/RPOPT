//! Port of `ppopt.utils` (chebyshev_ball, constraint_utilities, general_utils).

use crate::linalg::{dot, is_full_rank_rows, norm, rank, Mat};
use crate::solver::lp::{solve_lp, LpSolution};

/// Chebyshev ball of {x : A x <= b, A[eq] x = b[eq]}: max r s.t. A x + ||A_i|| r <= b, r >= 0.
/// Returns the LP solution with the variables ordered (x, r).
pub fn chebyshev_ball(a: &Mat, b: &[f64], eq: &[usize]) -> Option<LpSolution> {
    let n = a.cols;
    let m = a.rows;
    let mut ab = Mat::zeros(m + 1, n + 1);
    let mut is_eq = vec![false; m];
    for &e in eq {
        is_eq[e] = true;
    }
    for i in 0..m {
        let r = a.row(i);
        ab.row_mut(i)[..n].copy_from_slice(r);
        ab[(i, n)] = if is_eq[i] { 0.0 } else { norm(r) };
    }
    ab[(m, n)] = -1.0;
    let mut bb = b.to_vec();
    bb.push(0.0);
    let mut c = vec![0.0; n + 1];
    c[n] = -1.0;
    // callers only use the point (x, r)
    let x = crate::solver::lp::lp_point(Some(&c), &ab, &bb, eq)?;
    let obj = -x[n];
    Some(LpSolution { x, obj, dual: vec![], slack: vec![], active_set: vec![] })
}

/// Chebyshev center and radius, convenience wrapper.
pub fn chebyshev_center(a: &Mat, b: &[f64]) -> Option<(Vec<f64>, f64)> {
    let s = chebyshev_ball(a, b, &[])?;
    let n = a.cols;
    Some((s.x[..n].to_vec(), s.x[n]))
}

/// L2 norm of each row.
pub fn constraint_norm(a: &Mat) -> Vec<f64> {
    a.row_norms()
}

/// Normalizes each row of [A | b] by ||A_i||.
pub fn scale_constraint(a: &Mat, b: &[f64]) -> (Mat, Vec<f64>) {
    let mut a2 = a.clone();
    let mut b2 = b.to_vec();
    for i in 0..a.rows {
        let nrm = norm(a.row(i));
        let inv = 1.0 / nrm;
        for v in a2.row_mut(i) {
            *v *= inv;
        }
        b2[i] *= inv;
    }
    (a2, b2)
}

/// Rows whose entries are not all within 1e-8 of zero (numpy `allclose(row, 0, atol=1e-8)`).
pub fn numerically_nonzero_rows(a: &Mat) -> Vec<usize> {
    (0..a.rows).filter(|&i| !a.row(i).iter().all(|v| v.abs() <= 1e-8)).collect()
}

pub fn remove_numerically_zero_rows(a: &Mat, b: &[f64]) -> (Mat, Vec<f64>) {
    let keep = numerically_nonzero_rows(a);
    (a.select_rows(&keep), keep.iter().map(|&i| b[i]).collect())
}

/// Exact zero rows removed (numpy `nonzero`).
pub fn remove_zero_rows(a: &Mat, b: &[f64]) -> (Mat, Vec<f64>) {
    let keep: Vec<usize> = (0..a.rows).filter(|&i| a.row(i).iter().any(|&v| v != 0.0)).collect();
    (a.select_rows(&keep), keep.iter().map(|&i| b[i]).collect())
}

/// Removes exact duplicate rows of [A | b], keeping first occurrences in order (numpy `unique`).
pub fn remove_duplicate_rows(a: &Mat, b: &[f64]) -> (Mat, Vec<f64>) {
    if a.rows == 0 {
        return (a.clone(), b.to_vec());
    }
    let mut keep: Vec<usize> = Vec::with_capacity(a.rows);
    // A row is a duplicate when every entry (and the right-hand side) has the same canonical bits
    // as an earlier kept row. Pairwise comparison with an early exit on the first differing
    // entry beats hashing a Vec per row for the few dozen rows of a region.
    let same = |i: usize, j: usize| -> bool { canonical_bits(b[i]) == canonical_bits(b[j]) && a.row(i).iter().zip(a.row(j)).all(|(x, y)| canonical_bits(*x) == canonical_bits(*y)) };
    for i in 0..a.rows {
        if !keep.iter().any(|&k| same(k, i)) {
            keep.push(i);
        }
    }
    (a.select_rows(&keep), keep.iter().map(|&i| b[i]).collect())
}

#[inline]
fn canonical_bits(v: f64) -> u64 {
    if v == 0.0 { 0 } else { v.to_bits() }
}

/// For the kept rows `rows` of the unit-norm system `e x <= f`: true when no other row of the whole system lies on the same
/// hyperplane (either orientation) within 1e-6. A tableau row can vanish identically on the facet of row p only if its affine
/// function is proportional to row p's, that is, only if such a row exists. `dropped_zero`: some row of the unreduced system was
/// dropped as numerically zero and is not in `e`; it may vanish identically, so nothing is certified then.
pub fn unique_hyperplane_flags(e: &Mat, f: &[f64], rows: &[usize], dropped_zero: bool) -> Vec<bool> {
    if dropped_zero {
        return vec![false; rows.len()];
    }
    const TOL: f64 = 1e-6;
    rows.iter()
        .map(|&p| {
            let a = e.row(p);
            let fp = f[p];
            let ot = TOL * (1.0 + fp.abs());
            for q in 0..e.rows {
                if q == p {
                    continue;
                }
                let b = e.row(q);
                if (f[q] - fp).abs() <= ot && a.iter().zip(b).all(|(x, y)| (x - y).abs() <= TOL) {
                    return false;
                }
                if (f[q] + fp).abs() <= ot && a.iter().zip(b).all(|(x, y)| (x + y).abs() <= TOL) {
                    return false;
                }
            }
            true
        })
        .collect()
}

pub fn cheap_remove_redundant_constraints(a: &Mat, b: &[f64]) -> (Mat, Vec<f64>) {
    let (a, b) = remove_zero_rows(a, b);
    let (a, b) = scale_constraint(&a, &b);
    let (a, b) = remove_duplicate_rows(&a, &b);
    remove_near_duplicate_rows(&a, &b, 1e-12)
}

/// Removes rows of [A | b] (unit-normalised rows) that agree with an earlier kept row to within
/// `tol` in every entry, keeping the first. Exact duplicates are handled by `remove_duplicate_rows`;
/// two rows that differ by an ulp are not equal there, and the weak-redundancy test of
/// `facet_ball_elimination` then drops both of them (each blocks the other's facet ball).
pub fn remove_near_duplicate_rows(a: &Mat, b: &[f64], tol: f64) -> (Mat, Vec<f64>) {
    if a.rows < 2 || a.rows > 4000 {
        return (a.clone(), b.to_vec());
    }
    let mut keep: Vec<usize> = Vec::with_capacity(a.rows);
    for j in 0..a.rows {
        let dup = keep.iter().any(|&i| (b[i] - b[j]).abs() <= tol && a.row(i).iter().zip(a.row(j)).all(|(x, y)| (x - y).abs() <= tol));
        if !dup {
            keep.push(j);
        }
    }
    if keep.len() == a.rows {
        return (a.clone(), b.to_vec());
    }
    (a.select_rows(&keep), keep.iter().map(|&i| b[i]).collect())
}

/// Keeps constraint i iff the polytope with row i (plus `equality_set`) held at equality is
/// nonempty; returns the kept indices (PPOPT `find_redundant_constraints`).
pub fn find_redundant_constraints(a: &Mat, b: &[f64], equality_set: &[usize]) -> Vec<usize> {
    crate::census_purpose!(Redundancy);
    let mut is_eq = vec![false; a.rows];
    for &e in equality_set {
        is_eq[e] = true;
    }
    let mut kept = Vec::with_capacity(a.rows);
    let mut eqs = equality_set.to_vec();
    for i in 0..a.rows {
        if is_eq[i] {
            kept.push(i);
            continue;
        }
        eqs.push(i);
        if crate::solver::lp::lp_feasible(a, b, &eqs) {
            kept.push(i);
        }
        eqs.pop();
    }
    kept
}

/// Removes strongly redundant constraints (rows that can never be active).
pub fn remove_strongly_redundant_constraints(a: &Mat, b: &[f64]) -> (Mat, Vec<f64>, Vec<usize>) {
    crate::census_purpose!(Redundancy);
    let keep: Vec<usize> = (0..a.rows).filter(|&i| crate::solver::lp::lp_feasible(a, b, &[i])).collect();
    (a.select_rows(&keep), keep.iter().map(|&i| b[i]).collect(), keep)
}

/// Removes weakly redundant constraints (facet-ball elimination, Oberdieck et al. appendix):
/// keep row i iff a ball touching facet i fits strictly inside the polytope.
pub fn facet_ball_elimination(a: &Mat, b: &[f64]) -> (Mat, Vec<f64>) {
    let (ap, bp) = scale_constraint(a, b);
    let keep = calculate_redundant_constraints(&ap, &bp);
    (a.select_rows(&keep), keep.iter().map(|&i| b[i]).collect())
}

pub fn calculate_redundant_constraints(a: &Mat, b: &[f64]) -> Vec<usize> {
    crate::census_purpose!(Redundancy);
    let (ap, bp) = scale_constraint(a, b);
    let n = ap.cols;
    let mut out = vec![];
    for i in 0..ap.rows {
        // PPOPT: A_norm = || 1 - ||A_ps @ A_ps[i]|| || * ones  (a scalar), zero for row i
        let prod = ap.matvec(ap.row(i));
        let s = (1.0 - norm(&prod)).abs();
        let mut ab = Mat::zeros(ap.rows, n + 1);
        for r in 0..ap.rows {
            ab.row_mut(r)[..n].copy_from_slice(ap.row(r));
            ab[(r, n)] = if r == i { 0.0 } else { s };
        }
        let mut c = vec![0.0; n + 1];
        c[n] = -1.0;
        if let Some(sol) = solve_lp(Some(&c), &ab, &bp, &[i]) {
            if sol.x[n] > 0.0 {
                out.push(i);
            }
        }
    }
    out
}

/// Removes all strongly and weakly redundant constraints.
pub fn process_region_constraints(a: &Mat, b: &[f64]) -> (Mat, Vec<f64>) {
    let (a, b) = cheap_remove_redundant_constraints(a, b);
    let (a, b, _) = remove_strongly_redundant_constraints(&a, &b);
    facet_ball_elimination(&a, &b)
}

/// Pairs (i, j) of inequality rows of [A | b] with a_i ~ -a_j (implicit equalities).
pub fn detect_implicit_equalities(a: &Mat, b: &[f64]) -> Vec<(usize, usize)> {
    let n = a.cols;
    let mut block = Mat::zeros(a.rows, n + 1);
    for i in 0..a.rows {
        block.row_mut(i)[..n].copy_from_slice(a.row(i));
        block[(i, n)] = b[i];
        for _ in 0..2 {
            let nr = norm(block.row(i));
            for v in block.row_mut(i) {
                *v /= nr;
            }
        }
    }
    let mut pairs = vec![];
    for i in 0..block.rows {
        for j in i..block.rows {
            let ri = block.row(i);
            let rj = block.row(j);
            let mut checks = 0;
            if (dot(ri, rj) + 1.0).abs() <= 1e-8 {
                checks += 1;
            }
            let diff: f64 = ri.iter().zip(rj).map(|(x, y)| (x - y) * (x - y)).sum::<f64>().sqrt();
            if diff <= 1e-12 {
                checks += 1;
            }
            // numpy allclose(u, -v): |u + v| <= 1e-8 + 1e-5 |v|
            if ri.iter().zip(rj).all(|(x, y)| (x + y).abs() <= 1e-8 + 1e-5 * y.abs()) {
                checks += 1;
            }
            if checks >= 2 {
                pairs.push((i, j));
            }
        }
    }
    pairs
}

/// Indices of rows with norm >= epsilon (kept) and below (removed).
pub fn get_indices_of_zero_rows(a: &Mat, epsilon: f64) -> (Vec<usize>, Vec<usize>) {
    let mut kept = vec![];
    let mut removed = vec![];
    for i in 0..a.rows {
        if norm(a.row(i)) >= epsilon {
            kept.push(i);
        } else {
            removed.push(i);
        }
    }
    (kept, removed)
}

/// Linearly independent rows (incremental rank test, PPOPT `get_independent_rows`).
pub fn get_independent_rows(a: &Mat) -> Vec<usize> {
    let num_rows = a.rows;
    let mut row_ranks = vec![0usize; num_rows];
    for i in 0..num_rows.saturating_sub(1) {
        let idx: Vec<usize> = (0..=i).collect();
        row_ranks[i] = rank(&a.select_rows(&idx));
    }
    let mut out = vec![];
    let mut prev = 0usize;
    for (i, &r) in row_ranks.iter().enumerate() {
        if r > prev {
            out.push(i);
        }
        prev = r;
    }
    out
}

pub fn is_full_rank(a: &Mat, idx: &[usize]) -> bool {
    is_full_rank_rows(a, idx)
}

pub fn select_not_in(n: usize, coll: &[usize]) -> Vec<usize> {
    let mut mask = vec![false; n];
    for &c in coll {
        if c < n {
            mask[c] = true;
        }
    }
    (0..n).filter(|&i| !mask[i]).collect()
}

/// PPOPT `latex_matrix`-style number rendering.
pub fn render_number(x: f64) -> String {
    if x.abs() < 1e-14 {
        "0".into()
    } else if x.abs() > 1e-4 {
        format!("{:.4}", x)
    } else {
        let e = x.abs().log10().floor() as i32;
        format!("{:.4} 10^{{{}}}", x / 10f64.powi(e), e)
    }
}

pub fn latex_matrix(a: &Mat) -> String {
    let rows: Vec<String> = (0..a.rows)
        .map(|i| a.row(i).iter().map(|v| render_number(*v)).collect::<Vec<_>>().join(" & "))
        .collect();
    format!("\\left[\\begin{{matrix}}{}\\end{{matrix}}\\right]", rows.join("\\\\"))
}

pub fn latex_vector(v: &[String]) -> String {
    format!("\\left[\\begin{{matrix}}{}\\end{{matrix}}\\right]", v.join("\\\\"))
}

pub fn num_cpu_cores() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}
