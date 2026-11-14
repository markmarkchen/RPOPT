//! Goldfarb-Idnani dual active-set QP solver (the algorithm behind `quadprog`, PPOPT's default QP
//! backend). Being a dual method it needs no feasible starting point and reports an exact active
//! set, which is what the geometric mpQP algorithm relies on.
//!
//! Problem:  min 1/2 x'Qx + c'x  s.t.  A x <= b, rows in `eq` hold with equality. Q must be
//! symmetric positive definite.

use crate::linalg::{dot, Mat};

#[derive(Clone, Debug)]
pub struct QpSolution {
    pub x: Vec<f64>,
    pub obj: f64,
    /// Lagrange multipliers in the standard sign convention Qx + c + A'lambda = 0, lambda_I >= 0.
    pub dual: Vec<f64>,
    pub slack: Vec<f64>,
    /// constraints with |slack| <= 1e-10 or a nonzero multiplier (PPOPT/quadprog convention)
    pub active_set: Vec<usize>,
}

#[inline]
fn hypot(a: f64, b: f64) -> f64 {
    a.hypot(b)
}

struct Gi<'w> {
    n: usize,
    j: &'w mut Vec<f64>, // n x n, stored transposed: J[k][c] at c*n + k (columns contiguous)
    r: &'w mut Vec<f64>, // n x n upper triangular
    r_norm: f64,
}

/// Reused working storage of the QP solver (one per thread).
#[derive(Default)]
struct QpWork {
    j: Vec<f64>,
    r: Vec<f64>,
    is_eq: Vec<bool>,
    ineq: Vec<usize>,
    aset: Vec<i64>,
    a_old: Vec<i64>,
    u: Vec<f64>,
    u_old: Vec<f64>,
    iai: Vec<i64>,
    iaexcl: Vec<bool>,
    s: Vec<f64>,
    d: Vec<f64>,
    z: Vec<f64>,
    rv: Vec<f64>,
    np: Vec<f64>,
    x_old: Vec<f64>,
}

thread_local! {
    static QP_WORK: std::cell::RefCell<QpWork> = std::cell::RefCell::new(QpWork::default());
}

impl Gi<'_> {
    #[inline]
    fn jget(&self, k: usize, c: usize) -> f64 {
        self.j[c * self.n + k]
    }

    /// d = J' np (row i of J' is contiguous)
    fn compute_d(&self, d: &mut [f64], np: &[f64]) {
        let n = self.n;
        for i in 0..n {
            let col = &self.j[i * n..(i + 1) * n];
            let mut s = 0.0;
            for (jv, nv) in col.iter().zip(np) {
                s += jv * nv;
            }
            d[i] = s;
        }
    }

    /// z = J[:, iq..] d[iq..], accumulated column by column in the same order as before
    fn update_z(&self, z: &mut [f64], d: &[f64], iq: usize) {
        let n = self.n;
        for v in z.iter_mut() {
            *v = 0.0;
        }
        for c in iq..n {
            let dc = d[c];
            let col = &self.j[c * n..(c + 1) * n];
            for (zv, jv) in z.iter_mut().zip(col) {
                *zv += jv * dc;
            }
        }
    }

    fn update_r(&self, rv: &mut [f64], d: &[f64], iq: usize) {
        let n = self.n;
        for i in (0..iq).rev() {
            let mut s = 0.0;
            for c in i + 1..iq {
                s += self.r[i * n + c] * rv[c];
            }
            rv[i] = (d[i] - s) / self.r[i * n + i];
        }
    }

    fn add_constraint(&mut self, d: &mut [f64], iq: &mut usize) -> bool {
        let n = self.n;
        let mut jj = n;
        while jj > *iq + 1 {
            jj -= 1;
            let j = jj;
            let mut cc = d[j - 1];
            let mut ss = d[j];
            let h = hypot(cc, ss);
            if h.abs() < f64::EPSILON {
                continue;
            }
            d[j] = 0.0;
            ss /= h;
            cc /= h;
            if cc < 0.0 {
                cc = -cc;
                ss = -ss;
                d[j - 1] = -h;
            } else {
                d[j - 1] = h;
            }
            let xny = ss / (1.0 + cc);
            let (lo, hi) = self.j.split_at_mut(j * n);
            let c0 = &mut lo[(j - 1) * n..j * n];
            let c1 = &mut hi[..n];
            for (a0, a1) in c0.iter_mut().zip(c1.iter_mut()) {
                let t1 = *a0;
                let t2 = *a1;
                let nj1 = t1 * cc + t2 * ss;
                *a0 = nj1;
                *a1 = xny * (t1 + nj1) - t2;
            }
        }
        *iq += 1;
        for i in 0..*iq {
            self.r[i * n + *iq - 1] = d[i];
        }
        if d[*iq - 1].abs() <= f64::EPSILON * self.r_norm {
            return false;
        }
        self.r_norm = self.r_norm.max(d[*iq - 1].abs());
        true
    }

    fn delete_constraint(&mut self, a: &mut [i64], u: &mut [f64], p: usize, iq: &mut usize, l: i64) {
        let n = self.n;
        let mut qq = usize::MAX;
        for i in p..*iq {
            if a[i] == l {
                qq = i;
                break;
            }
        }
        if qq == usize::MAX {
            return;
        }
        for i in qq..*iq - 1 {
            a[i] = a[i + 1];
            u[i] = u[i + 1];
            for jr in 0..n {
                self.r[jr * n + i] = self.r[jr * n + i + 1];
            }
        }
        a[*iq - 1] = a[*iq];
        u[*iq - 1] = u[*iq];
        a[*iq] = 0;
        u[*iq] = 0.0;
        for jr in 0..*iq {
            self.r[jr * n + *iq - 1] = 0.0;
        }
        *iq -= 1;
        if *iq == 0 {
            return;
        }
        for j in qq..*iq {
            let mut cc = self.r[j * n + j];
            let mut ss = self.r[(j + 1) * n + j];
            let h = hypot(cc, ss);
            if h.abs() < f64::EPSILON {
                continue;
            }
            cc /= h;
            ss /= h;
            self.r[(j + 1) * n + j] = 0.0;
            if cc < 0.0 {
                self.r[j * n + j] = -h;
                cc = -cc;
                ss = -ss;
            } else {
                self.r[j * n + j] = h;
            }
            let xny = ss / (1.0 + cc);
            for k in j + 1..*iq {
                let t1 = self.r[j * n + k];
                let t2 = self.r[(j + 1) * n + k];
                let v = t1 * cc + t2 * ss;
                self.r[j * n + k] = v;
                self.r[(j + 1) * n + k] = xny * (t1 + v) - t2;
            }
            let (lo, hi) = self.j.split_at_mut((j + 1) * n);
            let c0 = &mut lo[j * n..(j + 1) * n];
            let c1 = &mut hi[..n];
            for (a0, a1) in c0.iter_mut().zip(c1.iter_mut()) {
                let t1 = *a0;
                let t2 = *a1;
                let v = t1 * cc + t2 * ss;
                *a0 = v;
                *a1 = xny * (v + t1) - t2;
            }
        }
    }
}

/// Factorization of the Hessian reused across QP solves with the same Q (Cholesky factor, the
/// initial Goldfarb-Idnani matrix J = L^{-T} and the two trace constants).
#[derive(Clone, Debug)]
pub struct QpFactor {
    n: usize,
    chol: crate::linalg::Cholesky,
    j0: Vec<f64>,
    c1: f64,
    c2: f64,
    eq_state: Option<EqState>,
}

/// State of the Goldfarb-Idnani factors after the equality constraints have been added. It depends
/// only on the equality rows (not on b or c), so for a fixed program it is computed once and each
/// solve replays only the right-hand-side updates.
#[derive(Clone, Debug)]
struct EqState {
    rows: Vec<usize>,
    j: Vec<f64>,
    r: Vec<f64>,
    r_norm: f64,
    /// per equality: step direction z, dual step r (first ie entries), z'z and z'n
    z: Vec<Vec<f64>>,
    rv: Vec<Vec<f64>>,
    zz: Vec<f64>,
    znp: Vec<f64>,
}

impl QpFactor {
    pub fn new(q: &Mat) -> Option<QpFactor> {
        let n = q.rows;
        assert_eq!(q.cols, n);
        let mut g = Mat::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                g[(i, j)] = 0.5 * (q[(i, j)] + q[(j, i)]);
            }
        }
        let c1: f64 = (0..n).map(|i| g[(i, i)]).sum();
        let chol = crate::linalg::Cholesky::new(&g)?;
        let mut j0 = vec![0.0; n * n];
        let mut c2 = 0.0;
        let mut z = vec![0.0; n];
        for i in 0..n {
            for v in z.iter_mut() {
                *v = 0.0;
            }
            for r in i..n {
                let mut s = if r == i { 1.0 } else { 0.0 };
                for k in i..r {
                    s -= chol.l[r * n + k] * z[k];
                }
                z[r] = s / chol.l[r * n + r];
            }
            // J = L^{-T}: J[i][c] = z[c]; stored transposed (J[i][c] at c*n + i)
            for c in 0..n {
                j0[c * n + i] = z[c];
            }
            c2 += z[i];
        }
        Some(QpFactor { n, chol, j0, c1, c2, eq_state: None })
    }

    /// Factor plus the cached equality phase for the rows `eq` of `a` (None if Q is not positive
    /// definite or the equality rows are linearly dependent, the cases where every solve fails).
    pub fn with_equalities(q: &Mat, a: &Mat, eq: &[usize]) -> Option<QpFactor> {
        let mut fac = QpFactor::new(q)?;
        if eq.is_empty() {
            return Some(fac);
        }
        let n = fac.n;
        let mut j = fac.j0.clone();
        let mut r = vec![0.0; n * n];
        let (mut zs, mut rvs, mut zzs, mut znps) = (vec![], vec![], vec![], vec![]);
        let mut d = vec![0.0; n];
        let mut z = vec![0.0; n];
        let mut rv = vec![0.0; eq.len() + 1];
        let mut iq = 0usize;
        let r_norm;
        {
            let mut gi = Gi { n, j: &mut j, r: &mut r, r_norm: 1.0 };
            for &row in eq {
                let np = a.row(row);
                gi.compute_d(&mut d, np);
                gi.update_z(&mut z, &d, iq);
                gi.update_r(&mut rv, &d, iq);
                zs.push(z.clone());
                rvs.push(rv[..iq].to_vec());
                zzs.push(dot(&z, &z));
                znps.push(dot(&z, np));
                if !gi.add_constraint(&mut d, &mut iq) {
                    return None;
                }
            }
            r_norm = gi.r_norm;
        }
        fac.eq_state = Some(EqState { rows: eq.to_vec(), j, r, r_norm, z: zs, rv: rvs, zz: zzs, znp: znps });
        Some(fac)
    }
}

/// Solves the strictly convex QP. Returns `None` if infeasible, if Q is not positive definite or if
/// the equality constraints are linearly dependent (the cases where quadprog raises).
pub fn solve_qp(q: &Mat, c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize]) -> Option<QpSolution> {
    let fac = QpFactor::new(q)?;
    solve_qp_factored(&fac, q, c, a, b, eq)
}

/// `solve_qp` with a precomputed factorization of Q.
pub fn solve_qp_factored(fac: &QpFactor, q: &Mat, c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize]) -> Option<QpSolution> {
    QP_WORK.with(|w| {
        let mut guard = w.borrow_mut();
        solve_qp_ws(&mut guard, fac, q, c, a, b, eq, false)
    })
}

/// Only x and the PPOPT active set (no multipliers, slacks or objective).
pub fn solve_qp_active_set(fac: &QpFactor, q: &Mat, c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize]) -> Option<QpSolution> {
    QP_WORK.with(|w| {
        let mut guard = w.borrow_mut();
        solve_qp_ws(&mut guard, fac, q, c, a, b, eq, true)
    })
}

#[allow(clippy::too_many_arguments)]
fn solve_qp_ws(ws: &mut QpWork, fac: &QpFactor, q: &Mat, c: Option<&[f64]>, a: &Mat, b: &[f64], eq: &[usize], active_only: bool) -> Option<QpSolution> {
    let n = fac.n;
    let zero_c;
    let c = match c {
        Some(c) => c,
        None => {
            zero_c = vec![0.0; n];
            &zero_c
        }
    };
    let m_total = a.rows;
    let QpWork { j: wj, r: wr, is_eq, ineq, aset, a_old, u, u_old, iai, iaexcl, s, d, z, rv, np, x_old } = ws;
    is_eq.clear();
    is_eq.resize(m_total, false);
    for &e in eq {
        is_eq[e] = true;
    }
    ineq.clear();
    ineq.extend((0..m_total).filter(|&i| !is_eq[i]));
    let ineq: &Vec<usize> = ineq;
    let p = eq.len();
    let mi = ineq.len();

    // symmetrized Q (as PPOPT does) is factored once in `fac`
    let (c1, c2, chol) = (fac.c1, fac.c2, &fac.chol);
    let cached_eq = fac.eq_state.as_ref().filter(|st| st.rows == eq);
    wj.clear();
    wr.clear();
    match cached_eq {
        Some(st) => {
            wj.extend_from_slice(&st.j);
            wr.extend_from_slice(&st.r);
        }
        None => {
            wj.extend_from_slice(&fac.j0);
            wr.resize(n * n, 0.0);
        }
    }
    let mut gi = Gi { n, j: wj, r: wr, r_norm: cached_eq.map(|st| st.r_norm).unwrap_or(1.0) };

    // unconstrained minimizer
    let mut x: Vec<f64> = chol.solve(c).iter().map(|v| -v).collect();
    let mut f_value = 0.5 * dot(c, &x);

    let cap = p + mi + 1;
    fn reset<T: Clone>(v: &mut Vec<T>, len: usize, val: T) {
        v.clear();
        v.resize(len, val);
    }
    reset(aset, cap, 0);
    reset(a_old, cap, 0);
    reset(u, cap, 0.0);
    reset(u_old, cap, 0.0);
    reset(iai, mi + 1, 0);
    reset(iaexcl, mi + 1, true);
    reset(s, mi, 0.0);
    reset(d, n, 0.0);
    reset(z, n, 0.0);
    reset(rv, cap, 0.0);
    reset(np, n, 0.0);
    reset(x_old, n, 0.0);
    let eps = f64::EPSILON;
    let inf = f64::INFINITY;

    // constraint normal (n_i) and offset for the >= form:  n_i' x + c0_i >= 0
    //   inequality a x <= b  ->  n = -a, c0 = b
    //   equality   a x  = b  ->  n =  a, c0 = -b
    let mut iq = 0usize;
    if let Some(st) = cached_eq {
        // replay of the equality phase: same operations on x, u, f as the loop below
        for (ie, &row) in eq.iter().enumerate() {
            let npr = a.row(row);
            let ce0 = -b[row];
            let zv = &st.z[ie];
            let mut t2 = 0.0;
            if st.zz[ie].abs() > f64::EPSILON {
                t2 = (-dot(npr, &x) - ce0) / st.znp[ie];
            }
            for k in 0..n {
                x[k] += t2 * zv[k];
            }
            u[iq] = t2;
            for k in 0..iq {
                u[k] -= t2 * st.rv[ie][k];
            }
            f_value += 0.5 * t2 * t2 * st.znp[ie];
            aset[ie] = -(ie as i64) - 1;
            iq += 1;
        }
    }
    for (ie, &row) in eq.iter().enumerate() {
        if cached_eq.is_some() {
            break;
        }
        np.copy_from_slice(a.row(row));
        let ce0 = -b[row];
        gi.compute_d(d, &np);
        gi.update_z(z, &d, iq);
        gi.update_r(rv, &d, iq);
        let mut t2 = 0.0;
        let zz = dot(&z, &z);
        if zz.abs() > eps {
            t2 = (-dot(&np, &x) - ce0) / dot(&z, &np);
        }
        for k in 0..n {
            x[k] += t2 * z[k];
        }
        u[iq] = t2;
        for k in 0..iq {
            u[k] -= t2 * rv[k];
        }
        f_value += 0.5 * t2 * t2 * dot(&z, &np);
        aset[ie] = -(ie as i64) - 1;
        if !gi.add_constraint(d, &mut iq) {
            return None; // linearly dependent equality constraints
        }
    }

    for i in 0..mi {
        iai[i] = i as i64;
    }

    let ci = |i: usize, out: &mut [f64]| {
        let r = a.row(ineq[i]);
        for k in 0..out.len() {
            out[k] = -r[k];
        }
    };
    let ci_val = |i: usize, x: &[f64]| -> f64 { -dot(a.row(ineq[i]), x) + b[ineq[i]] };

    let max_iter = 100 * (n + mi + p) + 100;
    let mut iter = 0usize;

    'l1: loop {
        iter += 1;
        if iter > max_iter {
            return None;
        }
        for i in p..iq {
            let ip = aset[i];
            iai[ip as usize] = -1;
        }
        let mut psi = 0.0;
        for i in 0..mi {
            iaexcl[i] = true;
            let sum = ci_val(i, &x);
            s[i] = sum;
            psi += sum.min(0.0);
        }
        if psi.abs() <= (mi as f64) * eps * c1 * c2 * 100.0 {
            break 'l1;
        }
        for i in 0..iq {
            u_old[i] = u[i];
            a_old[i] = aset[i];
        }
        x_old.copy_from_slice(&x);

        'l2: loop {
            iter += 1;
            if iter > max_iter {
                return None;
            }
            let mut ss = 0.0;
            let mut ip = 0usize;
            for i in 0..mi {
                if s[i] < ss && iai[i] != -1 && iaexcl[i] {
                    ss = s[i];
                    ip = i;
                }
            }
            if ss >= 0.0 {
                break 'l1;
            }
            ci(ip, np);
            u[iq] = 0.0;
            aset[iq] = ip as i64;

            'l2a: loop {
                iter += 1;
                if iter > max_iter {
                    return None;
                }
                gi.compute_d(d, &np);
                gi.update_z(z, &d, iq);
                gi.update_r(rv, &d, iq);
                let mut l: i64 = 0;
                let mut t1 = inf;
                for k in p..iq {
                    if rv[k] > 0.0 && u[k] / rv[k] < t1 {
                        t1 = u[k] / rv[k];
                        l = aset[k];
                    }
                }
                let mut t2;
                if dot(&z, &z).abs() > eps {
                    t2 = -s[ip] / dot(&z, &np);
                    if t2 < 0.0 {
                        t2 = inf;
                    }
                } else {
                    t2 = inf;
                }
                let t = t1.min(t2);
                if t >= inf {
                    return None; // infeasible
                }
                if t2 >= inf {
                    for k in 0..iq {
                        u[k] -= t * rv[k];
                    }
                    u[iq] += t;
                    iai[l as usize] = l;
                    gi.delete_constraint(aset, u, p, &mut iq, l);
                    continue 'l2a;
                }
                for k in 0..n {
                    x[k] += t * z[k];
                }
                f_value += t * dot(&z, &np) * (0.5 * t + u[iq]);
                for k in 0..iq {
                    u[k] -= t * rv[k];
                }
                u[iq] += t;
                if (t - t2).abs() < eps {
                    if !gi.add_constraint(d, &mut iq) {
                        iaexcl[ip] = false;
                        gi.delete_constraint(aset, u, p, &mut iq, ip as i64);
                        for i in 0..mi {
                            iai[i] = i as i64;
                        }
                        for i in p..iq {
                            aset[i] = a_old[i];
                            u[i] = u_old[i];
                            iai[aset[i] as usize] = -1;
                        }
                        x.copy_from_slice(&x_old);
                        continue 'l2;
                    } else {
                        iai[ip] = -1;
                    }
                    continue 'l1;
                }
                // partial step: drop constraint l
                iai[l as usize] = l;
                gi.delete_constraint(aset, u, p, &mut iq, l);
                s[ip] = ci_val(ip, &x);
                continue 'l2a;
            }
        }
    }
    let _ = f_value;
    let _ = gi.jget(0, 0);

    if active_only {
        // PPOPT active set: |slack| <= 1e-10 or a nonzero multiplier (multipliers are only
        // nonzero for constraints in the final working set)
        let mut nz = vec![false; m_total];
        for k in 0..iq {
            if u[k] != 0.0 {
                let id = aset[k];
                let row = if id < 0 { eq[(-id - 1) as usize] } else { ineq[id as usize] };
                nz[row] = true;
            }
        }
        let active_set = (0..m_total).filter(|&i| nz[i] || (b[i] - dot(a.row(i), &x)).abs() <= 1e-10).collect();
        return Some(QpSolution { x, obj: 0.0, dual: vec![], slack: vec![], active_set });
    }
    // multipliers in the standard convention
    let mut dual = vec![0.0; m_total];
    for k in 0..iq {
        let id = aset[k];
        if id < 0 {
            let e = eq[(-id - 1) as usize];
            dual[e] = -u[k];
        } else {
            dual[ineq[id as usize]] = u[k];
        }
    }
    let slack: Vec<f64> = (0..m_total).map(|i| b[i] - dot(a.row(i), &x)).collect();
    let qx = q.matvec(&x);
    let obj = 0.5 * dot(&x, &qx) + dot(c, &x);
    let active_set = (0..m_total).filter(|&i| slack[i].abs() <= 1e-10 || dual[i] != 0.0).collect();
    Some(QpSolution { x, obj, dual, slack, active_set })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_qp() {
        // min 1/2 |x|^2 - [2, 2]'x s.t. x <= 1 -> x = (1, 1), lambda = (1, 1)
        let q = Mat::eye(2);
        let a = Mat::from_rows(&[vec![1.0, 0.0], vec![0.0, 1.0], vec![-1.0, 0.0]]);
        let s = solve_qp(&q, Some(&[-2.0, -2.0]), &a, &[1.0, 1.0, 5.0], &[]).unwrap();
        assert!((s.x[0] - 1.0).abs() < 1e-12 && (s.x[1] - 1.0).abs() < 1e-12);
        assert!((s.dual[0] - 1.0).abs() < 1e-12 && (s.dual[1] - 1.0).abs() < 1e-12);
        assert_eq!(s.active_set, vec![0, 1]);
    }

    #[test]
    fn equality_qp() {
        // min 1/2 (x^2 + y^2) s.t. x + y = 2 -> (1,1), stationarity: x + lambda = 0 -> lambda = -1
        let q = Mat::eye(2);
        let a = Mat::from_rows(&[vec![1.0, 1.0], vec![-1.0, 0.0]]);
        let s = solve_qp(&q, None, &a, &[2.0, 0.0], &[0]).unwrap();
        assert!((s.x[0] - 1.0).abs() < 1e-12);
        assert!((s.dual[0] + 1.0).abs() < 1e-12);
    }

    #[test]
    fn infeasible_qp() {
        let q = Mat::eye(1);
        let a = Mat::from_rows(&[vec![1.0], vec![-1.0]]);
        assert!(solve_qp(&q, None, &a, &[-1.0, -1.0], &[]).is_none());
    }
}
