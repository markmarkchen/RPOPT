//! Bordered (Schur) update of the KKT data for a superset A ∪ {i} of an active set A whose data is
//! known. Used by the combinatorial graph algorithm, which tests every superset of each KKT-feasible
//! active set.
//!
//! With S = A_AS Q^{-1} A_AS' and the multiplier law λ_A(θ) = L θ + l for A, adding constraint i
//! (s = S[A, i], σ = S[i, i], w = S_A^{-1} s, γ = σ - s'w) gives
//!
//! ```text
//! μ(θ)  = (r_i(θ) - s'λ_A(θ)) / γ                      (multiplier of i)
//! λ'(θ) = λ_A(θ) - w μ(θ)                               (multipliers of A)
//! E'_j  = E_j + q_j μ_θ,   f'_j = f_j - q_j μ_0,   q = Σ_l w_l M[a_l, :] - M[i, :]
//! ```
//!
//! for the rows of the inactive constraints (E_j, f_j: the parent's rows). γ > 0 is the last
//! Cholesky pivot of S_{A ∪ i}, so a comfortably positive γ also certifies full row rank. The cost is
//! O(k^2 + m k) per child instead of O(k^3 + m k n_θ).

use crate::linalg::{Cholesky, Mat};
use crate::mpqp_utils::KktRegion;
use crate::program::{ControlLaw, MpProgram};

/// Outcome of the bordered update for a child.
pub enum ChildKkt {
    /// law and unreduced region rows
    Region(KktRegion),
    /// the λ >= 0 and parameter-space rows alone are infeasible: the region is empty
    LambdaEmpty,
}

/// Data of a KKT-feasible active set kept for its supersets.
pub struct ParentInfo {
    pub a: Vec<usize>,
    chol: Cholesky,
    a_l: Mat,
    b_l: Vec<f64>,
    /// rows -(AQ^{-1}H + F)_j - Σ_l M[a_l, j] L_l and their rhs, for every constraint j
    e_all: Vec<f64>,
    f_all: Vec<f64>,
}

impl ParentInfo {
    pub fn new(program: &MpProgram, a: &[usize], law: &ControlLaw) -> Option<ParentInfo> {
        let kc = program.kkt_cache_pub()?;
        let k = a.len();
        let nt = program.num_t();
        let m = program.num_constraints();
        let mut s = Mat::zeros(k, k);
        for (ii, &i) in a.iter().enumerate() {
            let mrow = kc.m.row(i);
            for (jj, &j) in a.iter().enumerate() {
                s[(ii, jj)] = mrow[j];
            }
        }
        let chol = Cholesky::new(&s)?;
        let mut e_all = kc.base_e.data.clone();
        let mut f_all = kc.base_f.clone();
        for (ii, &i) in a.iter().enumerate() {
            let mrow = kc.m.row(i);
            let lrow = law.a_l.row(ii);
            let lb = law.b_l[ii];
            for j in 0..m {
                let mij = mrow[j];
                if mij == 0.0 {
                    continue;
                }
                let row = &mut e_all[j * nt..(j + 1) * nt];
                for (o, l) in row.iter_mut().zip(lrow) {
                    *o -= mij * l;
                }
                f_all[j] += mij * lb;
            }
        }
        Some(ParentInfo { a: a.to_vec(), chol, a_l: law.a_l.clone(), b_l: law.b_l.clone(), e_all, f_all })
    }

    /// KKT data of A ∪ {i} (rows in the same order as `MpProgram::region_rows`); full rank is
    /// certified (γ > 1e-10 σ). The λ and parameter rows are tested first
    /// (`lambda_rows_feasible_raw`), and the inactive rows are only built when they pass. None if γ
    /// is not comfortably positive: the caller then takes the general path.
    pub fn child(&self, program: &MpProgram, i: usize) -> Option<ChildKkt> {
        let kc = program.kkt_cache_pub()?;
        let k = self.a.len();
        let nt = program.num_t();
        let m = program.num_constraints();
        let neq = program.num_equality_constraints();
        let mi = kc.m.row(i);
        let s: Vec<f64> = self.a.iter().map(|&l| mi[l]).collect();
        let sigma = mi[i];
        let w = if k > 0 { self.chol.solve(&s) } else { vec![] };
        let gamma = sigma - crate::linalg::dot(&s, &w);
        if !(gamma > 1e-10 * sigma.abs().max(1e-300)) {
            return None;
        }
        // μ = (r_i - s'λ_A) / γ with r_i = (base_e_i, -base_f_i)
        let mut mu_t = kc.base_e.row(i).to_vec();
        let mut mu_c = -kc.base_f[i];
        for l in 0..k {
            let sl = s[l];
            if sl == 0.0 {
                continue;
            }
            for (o, v) in mu_t.iter_mut().zip(self.a_l.row(l)) {
                *o -= sl * v;
            }
            mu_c -= sl * self.b_l[l];
        }
        for v in mu_t.iter_mut() {
            *v /= gamma;
        }
        mu_c /= gamma;
        // multipliers of B = A ∪ {i} in sorted order
        let pos = self.a.iter().take_while(|&&l| l < i).count();
        let kb = k + 1;
        let mut active = Vec::with_capacity(kb);
        let mut a_l = Mat::zeros(kb, nt);
        let mut b_l = vec![0.0; kb];
        for idx in 0..kb {
            let (src, row): (Option<usize>, usize) = if idx < pos { (Some(idx), self.a[idx]) } else if idx == pos { (None, i) } else { (Some(idx - 1), self.a[idx - 1]) };
            active.push(row);
            match src {
                Some(l) => {
                    let wl = w[l];
                    let dst = a_l.row_mut(idx);
                    for ((o, v), mt) in dst.iter_mut().zip(self.a_l.row(l)).zip(&mu_t) {
                        *o = v - wl * mt;
                    }
                    b_l[idx] = self.b_l[l] - wl * mu_c;
                }
                None => {
                    a_l.row_mut(idx).copy_from_slice(&mu_t);
                    b_l[idx] = mu_c;
                }
            }
        }
        if !crate::mpqp_utils::lambda_rows_feasible_raw(program, &a_l, &b_l) {
            return Some(ChildKkt::LambdaEmpty);
        }
        // q = Σ_l w_l M[a_l, :] - M[i, :]
        let mut q: Vec<f64> = mi.iter().map(|v| -v).collect();
        for (l, &al) in self.a.iter().enumerate() {
            let wl = w[l];
            if wl == 0.0 {
                continue;
            }
            for (o, v) in q.iter_mut().zip(kc.m.row(al)) {
                *o += wl * v;
            }
        }
        // rows: λ >= 0 for the non-equality multipliers, inactive constraints, parametric box
        let mut in_b = vec![false; m];
        for &r in &active {
            in_b[r] = true;
        }
        let inactive: Vec<usize> = (0..m).filter(|&j| !in_b[j]).collect();
        let n_lambda = kb - neq;
        let rows = n_lambda + inactive.len() + program.a_t.rows;
        let mut e = Mat::zeros(rows, nt);
        let mut f = Vec::with_capacity(rows);
        for idx in neq..kb {
            let r = idx - neq;
            for (o, v) in e.row_mut(r).iter_mut().zip(a_l.row(idx)) {
                *o = -v;
            }
            f.push(b_l[idx]);
        }
        for (jj, &j) in inactive.iter().enumerate() {
            let r = n_lambda + jj;
            let qj = q[j];
            let src = &self.e_all[j * nt..(j + 1) * nt];
            for ((o, v), mt) in e.row_mut(r).iter_mut().zip(src).zip(&mu_t) {
                *o = v + qj * mt;
            }
            f.push(self.f_all[j] - qj * mu_c);
        }
        for t in 0..program.a_t.rows {
            e.row_mut(n_lambda + inactive.len() + t).copy_from_slice(program.a_t.row(t));
            f.push(program.b_t[t]);
        }
        let law = ControlLaw { a_x: Mat::zeros(0, 0), b_x: vec![], a_l, b_l };
        Some(ChildKkt::Region(KktRegion { law, e, f, n_lambda, inactive, point: None }))
    }
}
