//! Small dense row-major matrix type and the handful of factorizations PPOPT needs.
//!
//! Multiparametric programs are small and row oriented (constraints are rows), so a plain
//! `Vec<f64>` in row-major order keeps every constraint contiguous in memory.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Mat {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f64>,
}

impl fmt::Debug for Mat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Mat {}x{} [", self.rows, self.cols)?;
        for i in 0..self.rows {
            writeln!(f, "  {:?}", self.row(i))?;
        }
        write!(f, "]")
    }
}

impl Mat {
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Mat { rows, cols, data: vec![0.0; rows * cols] }
    }

    pub fn eye(n: usize) -> Self {
        let mut m = Mat::zeros(n, n);
        for i in 0..n {
            m[(i, i)] = 1.0;
        }
        m
    }

    pub fn from_vec(rows: usize, cols: usize, data: Vec<f64>) -> Self {
        assert_eq!(rows * cols, data.len(), "Mat::from_vec dimension mismatch");
        Mat { rows, cols, data }
    }

    /// Builds a matrix from nested rows (all rows must have the same length).
    pub fn from_rows(rows: &[Vec<f64>]) -> Self {
        if rows.is_empty() {
            return Mat::zeros(0, 0);
        }
        let cols = rows[0].len();
        let mut data = Vec::with_capacity(rows.len() * cols);
        for r in rows {
            assert_eq!(r.len(), cols, "ragged rows");
            data.extend_from_slice(r);
        }
        Mat { rows: rows.len(), cols, data }
    }

    /// A column vector (n x 1).
    pub fn col_vec(v: &[f64]) -> Self {
        Mat { rows: v.len(), cols: 1, data: v.to_vec() }
    }

    #[inline]
    pub fn row(&self, i: usize) -> &[f64] {
        &self.data[i * self.cols..(i + 1) * self.cols]
    }

    #[inline]
    pub fn row_mut(&mut self, i: usize) -> &mut [f64] {
        let c = self.cols;
        &mut self.data[i * c..(i + 1) * c]
    }

    pub fn col(&self, j: usize) -> Vec<f64> {
        (0..self.rows).map(|i| self[(i, j)]).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.rows == 0 || self.cols == 0
    }

    pub fn transpose(&self) -> Mat {
        let mut t = Mat::zeros(self.cols, self.rows);
        for i in 0..self.rows {
            for j in 0..self.cols {
                t[(j, i)] = self[(i, j)];
            }
        }
        t
    }

    pub fn matmul(&self, other: &Mat) -> Mat {
        assert_eq!(self.cols, other.rows, "matmul dimension mismatch");
        let mut out = Mat::zeros(self.rows, other.cols);
        for i in 0..self.rows {
            let orow = &mut out.data[i * other.cols..(i + 1) * other.cols];
            for k in 0..self.cols {
                let a = self.data[i * self.cols + k];
                if a == 0.0 {
                    continue;
                }
                let brow = &other.data[k * other.cols..(k + 1) * other.cols];
                for (o, b) in orow.iter_mut().zip(brow) {
                    *o += a * b;
                }
            }
        }
        out
    }

    pub fn matvec(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(self.cols, x.len(), "matvec dimension mismatch");
        (0..self.rows).map(|i| dot(self.row(i), x)).collect()
    }

    /// x' A (row vector times matrix)
    pub fn vecmat(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(self.rows, x.len());
        let mut out = vec![0.0; self.cols];
        for i in 0..self.rows {
            let xi = x[i];
            if xi == 0.0 {
                continue;
            }
            for (o, a) in out.iter_mut().zip(self.row(i)) {
                *o += xi * a;
            }
        }
        out
    }

    pub fn select_rows(&self, idx: &[usize]) -> Mat {
        let mut data = Vec::with_capacity(idx.len() * self.cols);
        for &i in idx {
            data.extend_from_slice(self.row(i));
        }
        Mat { rows: idx.len(), cols: self.cols, data }
    }

    pub fn select_cols(&self, idx: &[usize]) -> Mat {
        let mut out = Mat::zeros(self.rows, idx.len());
        for i in 0..self.rows {
            for (jj, &j) in idx.iter().enumerate() {
                out[(i, jj)] = self[(i, j)];
            }
        }
        out
    }

    pub fn scale(&self, s: f64) -> Mat {
        Mat { rows: self.rows, cols: self.cols, data: self.data.iter().map(|v| v * s).collect() }
    }

    pub fn add(&self, other: &Mat) -> Mat {
        assert_eq!((self.rows, self.cols), (other.rows, other.cols));
        Mat { rows: self.rows, cols: self.cols, data: self.data.iter().zip(&other.data).map(|(a, b)| a + b).collect() }
    }

    pub fn sub(&self, other: &Mat) -> Mat {
        assert_eq!((self.rows, self.cols), (other.rows, other.cols));
        Mat { rows: self.rows, cols: self.cols, data: self.data.iter().zip(&other.data).map(|(a, b)| a - b).collect() }
    }

    pub fn neg(&self) -> Mat {
        self.scale(-1.0)
    }

    /// Vertical concatenation; zero-row blocks are skipped, column counts must agree.
    pub fn vstack(blocks: &[&Mat]) -> Mat {
        let cols = blocks.iter().find(|b| b.rows > 0).map(|b| b.cols).unwrap_or_else(|| blocks.first().map(|b| b.cols).unwrap_or(0));
        let rows: usize = blocks.iter().map(|b| b.rows).sum();
        let mut data = Vec::with_capacity(rows * cols);
        for b in blocks {
            if b.rows == 0 {
                continue;
            }
            assert_eq!(b.cols, cols, "vstack column mismatch");
            data.extend_from_slice(&b.data);
        }
        Mat { rows, cols, data }
    }

    /// Horizontal concatenation; row counts must agree.
    pub fn hstack(blocks: &[&Mat]) -> Mat {
        let rows = blocks.first().map(|b| b.rows).unwrap_or(0);
        let cols: usize = blocks.iter().map(|b| b.cols).sum();
        let mut out = Mat::zeros(rows, cols);
        let mut off = 0;
        for b in blocks {
            assert_eq!(b.rows, rows, "hstack row mismatch");
            for i in 0..rows {
                out.data[i * cols + off..i * cols + off + b.cols].copy_from_slice(b.row(i));
            }
            off += b.cols;
        }
        out
    }

    pub fn push_row(&mut self, r: &[f64]) {
        if self.rows == 0 && self.cols == 0 {
            self.cols = r.len();
        }
        assert_eq!(r.len(), self.cols);
        self.data.extend_from_slice(r);
        self.rows += 1;
    }

    pub fn row_norms(&self) -> Vec<f64> {
        (0..self.rows).map(|i| norm(self.row(i))).collect()
    }

    pub fn max_abs(&self) -> f64 {
        self.data.iter().fold(0.0, |m, v| m.max(v.abs()))
    }

    pub fn is_symmetric(&self, tol: f64) -> bool {
        if self.rows != self.cols {
            return false;
        }
        for i in 0..self.rows {
            for j in 0..i {
                if (self[(i, j)] - self[(j, i)]).abs() > tol {
                    return false;
                }
            }
        }
        true
    }

    pub fn to_nalgebra(&self) -> nalgebra::DMatrix<f64> {
        nalgebra::DMatrix::from_row_slice(self.rows, self.cols, &self.data)
    }

    pub fn from_nalgebra(m: &nalgebra::DMatrix<f64>) -> Mat {
        let mut out = Mat::zeros(m.nrows(), m.ncols());
        for i in 0..m.nrows() {
            for j in 0..m.ncols() {
                out[(i, j)] = m[(i, j)];
            }
        }
        out
    }

    pub fn to_rows(&self) -> Vec<Vec<f64>> {
        (0..self.rows).map(|i| self.row(i).to_vec()).collect()
    }
}

impl std::ops::Index<(usize, usize)> for Mat {
    type Output = f64;
    #[inline]
    fn index(&self, (i, j): (usize, usize)) -> &f64 {
        &self.data[i * self.cols + j]
    }
}

impl std::ops::IndexMut<(usize, usize)> for Mat {
    #[inline]
    fn index_mut(&mut self, (i, j): (usize, usize)) -> &mut f64 {
        &mut self.data[i * self.cols + j]
    }
}

#[inline]
pub fn dot(a: &[f64], b: &[f64]) -> f64 {
    let mut s = 0.0;
    for (x, y) in a.iter().zip(b) {
        s += x * y;
    }
    s
}

#[inline]
pub fn norm(a: &[f64]) -> f64 {
    dot(a, a).sqrt()
}

pub fn axpy(alpha: f64, x: &[f64], y: &mut [f64]) {
    for (yi, xi) in y.iter_mut().zip(x) {
        *yi += alpha * xi;
    }
}

pub fn allclose(a: &[f64], b: &[f64], rtol: f64, atol: f64) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() <= atol + rtol * y.abs())
}

/// Numerical rank with the numpy `matrix_rank` tolerance: sigma_max * max(m, n) * eps.
pub fn rank(m: &Mat) -> usize {
    if m.is_empty() {
        return 0;
    }
    let sv = m.to_nalgebra().singular_values();
    let smax = sv.iter().cloned().fold(0.0, f64::max);
    let tol = smax * (m.rows.max(m.cols) as f64) * f64::EPSILON;
    sv.iter().filter(|&&s| s > tol).count()
}

/// Rank of the selected rows; an empty selection counts as full rank (PPOPT convention).
pub fn is_full_rank_rows(a: &Mat, idx: &[usize]) -> bool {
    if idx.is_empty() {
        return true;
    }
    if idx.len() > a.cols {
        return false;
    }
    thread_local! {
        static BUF: std::cell::RefCell<(Vec<f64>, Vec<usize>, Vec<usize>)> = std::cell::RefCell::new((vec![], vec![], vec![]));
    }
    BUF.with(|cell| {
        let mut guard = cell.borrow_mut();
        let (m, rows, cols) = &mut *guard;
        full_rank_ge(a, idx, m, rows, cols)
    })
}

/// Gaussian elimination with complete pivoting; tolerance max(k, n) * eps * ||M||_F, an upper
/// bound of numpy's sigma_max-based threshold. Rows and columns are swapped physically so the
/// pivot search and the elimination run over contiguous memory.
fn full_rank_ge(a: &Mat, idx: &[usize], m: &mut Vec<f64>, _rows: &mut Vec<usize>, _cols: &mut Vec<usize>) -> bool {
    let k = idx.len();
    let n = a.cols;
    m.clear();
    for &i in idx {
        m.extend_from_slice(a.row(i));
    }
    let fro = m.iter().map(|v| v * v).sum::<f64>().sqrt();
    let tol = (k.max(n) as f64) * f64::EPSILON * fro;
    for step in 0..k {
        let (mut pr, mut pc, mut best) = (step, step, 0.0);
        for r in step..k {
            let row = &m[r * n + step..(r + 1) * n];
            for (c, v) in row.iter().enumerate() {
                let av = v.abs();
                if av > best {
                    best = av;
                    pr = r;
                    pc = step + c;
                }
            }
        }
        if best <= tol {
            return false;
        }
        if pr != step {
            for c in step..n {
                m.swap(pr * n + c, step * n + c);
            }
        }
        if pc != step {
            for r in step..k {
                m.swap(r * n + pc, r * n + step);
            }
        }
        let (top, bottom) = m.split_at_mut((step + 1) * n);
        let prow = &top[step * n + step..(step + 1) * n];
        let inv = 1.0 / prow[0];
        for r in 0..k - step - 1 {
            let row = &mut bottom[r * n + step..(r + 1) * n];
            let fct = row[0] * inv;
            if fct == 0.0 {
                continue;
            }
            for (x, y) in row.iter_mut().zip(prow) {
                *x -= fct * y;
            }
        }
    }
    true
}

/// LU factorization with partial pivoting of a square matrix.
pub struct Lu {
    n: usize,
    lu: Vec<f64>,
    piv: Vec<usize>,
}

impl Lu {
    pub fn new(m: &Mat) -> Option<Lu> {
        assert_eq!(m.rows, m.cols, "LU needs a square matrix");
        let n = m.rows;
        let mut lu = m.data.clone();
        let mut piv: Vec<usize> = (0..n).collect();
        let scale = m.max_abs().max(1e-300);
        for k in 0..n {
            let mut p = k;
            let mut best = lu[k * n + k].abs();
            for i in k + 1..n {
                let v = lu[i * n + k].abs();
                if v > best {
                    best = v;
                    p = i;
                }
            }
            if best <= 1e-13 * scale {
                return None;
            }
            if p != k {
                for j in 0..n {
                    lu.swap(k * n + j, p * n + j);
                }
                piv.swap(k, p);
            }
            let pivot = lu[k * n + k];
            for i in k + 1..n {
                let f = lu[i * n + k] / pivot;
                lu[i * n + k] = f;
                if f != 0.0 {
                    for j in k + 1..n {
                        lu[i * n + j] -= f * lu[k * n + j];
                    }
                }
            }
        }
        Some(Lu { n, lu, piv })
    }

    pub fn solve(&self, b: &[f64]) -> Vec<f64> {
        let n = self.n;
        let mut x: Vec<f64> = self.piv.iter().map(|&p| b[p]).collect();
        for i in 0..n {
            let mut s = x[i];
            for j in 0..i {
                s -= self.lu[i * n + j] * x[j];
            }
            x[i] = s;
        }
        for i in (0..n).rev() {
            let mut s = x[i];
            for j in i + 1..n {
                s -= self.lu[i * n + j] * x[j];
            }
            x[i] = s / self.lu[i * n + i];
        }
        x
    }

    /// Solves A X = B (all columns at once, in place on a copy of B).
    pub fn solve_mat(&self, b: &Mat) -> Mat {
        let n = self.n;
        let nc = b.cols;
        let mut x = Mat::zeros(n, nc);
        for i in 0..n {
            x.row_mut(i).copy_from_slice(b.row(self.piv[i]));
        }
        for i in 0..n {
            for j in 0..i {
                let l = self.lu[i * n + j];
                if l != 0.0 {
                    let (top, bot) = x.data.split_at_mut(i * nc);
                    let src = &top[j * nc..(j + 1) * nc];
                    for (d, s) in bot[..nc].iter_mut().zip(src) {
                        *d -= l * s;
                    }
                }
            }
        }
        for i in (0..n).rev() {
            for j in i + 1..n {
                let u = self.lu[i * n + j];
                if u != 0.0 {
                    let (top, bot) = x.data.split_at_mut(j * nc);
                    let dst = &mut top[i * nc..(i + 1) * nc];
                    for (d, s) in dst.iter_mut().zip(&bot[..nc]) {
                        *d -= u * s;
                    }
                }
            }
            let inv = 1.0 / self.lu[i * n + i];
            for v in x.row_mut(i) {
                *v *= inv;
            }
        }
        x
    }
}

/// Cholesky factor L (lower triangular, row-major) of a symmetric positive definite matrix.
#[derive(Clone, Debug)]
pub struct Cholesky {
    pub n: usize,
    pub l: Vec<f64>,
}

impl Cholesky {
    pub fn new(m: &Mat) -> Option<Cholesky> {
        let n = m.rows;
        let mut l = vec![0.0; n * n];
        for j in 0..n {
            let mut d = m[(j, j)];
            for k in 0..j {
                d -= l[j * n + k] * l[j * n + k];
            }
            if d <= 0.0 || !d.is_finite() {
                return None;
            }
            let djj = d.sqrt();
            l[j * n + j] = djj;
            for i in j + 1..n {
                let mut s = m[(i, j)];
                for k in 0..j {
                    s -= l[i * n + k] * l[j * n + k];
                }
                l[i * n + j] = s / djj;
            }
        }
        Some(Cholesky { n, l })
    }

    pub fn solve(&self, b: &[f64]) -> Vec<f64> {
        let n = self.n;
        let mut y = b.to_vec();
        for i in 0..n {
            let mut s = y[i];
            for k in 0..i {
                s -= self.l[i * n + k] * y[k];
            }
            y[i] = s / self.l[i * n + i];
        }
        for i in (0..n).rev() {
            let mut s = y[i];
            for k in i + 1..n {
                s -= self.l[k * n + i] * y[k];
            }
            y[i] = s / self.l[i * n + i];
        }
        y
    }

    pub fn solve_mat(&self, b: &Mat) -> Mat {
        self.solve_mat_inplace(b.clone())
    }

    /// Solves L L' X = B for all columns at once, overwriting B.
    pub fn solve_mat_inplace(&self, mut x: Mat) -> Mat {
        let n = self.n;
        let nc = x.cols;
        for i in 0..n {
            for k in 0..i {
                let l = self.l[i * n + k];
                if l != 0.0 {
                    let (top, bot) = x.data.split_at_mut(i * nc);
                    for (d, s) in bot[..nc].iter_mut().zip(&top[k * nc..(k + 1) * nc]) {
                        *d -= l * s;
                    }
                }
            }
            let inv = 1.0 / self.l[i * n + i];
            for v in x.row_mut(i) {
                *v *= inv;
            }
        }
        for i in (0..n).rev() {
            for k in i + 1..n {
                let l = self.l[k * n + i];
                if l != 0.0 {
                    let (top, bot) = x.data.split_at_mut(k * nc);
                    for (d, s) in top[i * nc..(i + 1) * nc].iter_mut().zip(&bot[..nc]) {
                        *d -= l * s;
                    }
                }
            }
            let inv = 1.0 / self.l[i * n + i];
            for v in x.row_mut(i) {
                *v *= inv;
            }
        }
        x
    }
}

/// Smallest eigenvalue of a symmetric matrix.
pub fn min_eigenvalue_sym(m: &Mat) -> f64 {
    if m.rows == 0 {
        return 0.0;
    }
    let sym = m.add(&m.transpose()).scale(0.5);
    let e = nalgebra::SymmetricEigen::new(sym.to_nalgebra());
    e.eigenvalues.iter().cloned().fold(f64::INFINITY, f64::min)
}

/// Moore-Penrose pseudo-inverse via SVD (numpy `pinv` default rcond = 1e-15 * max(m, n)).
pub fn pinv(m: &Mat) -> Mat {
    if m.is_empty() {
        return Mat::zeros(m.cols, m.rows);
    }
    let svd = m.to_nalgebra().svd(true, true);
    let smax = svd.singular_values.iter().cloned().fold(0.0, f64::max);
    let tol = 1e-15 * smax * (m.rows.max(m.cols) as f64);
    let p = svd.pseudo_inverse(tol).expect("pinv failed");
    Mat::from_nalgebra(&p)
}
