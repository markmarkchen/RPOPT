//! MPC structure of a constrained linear-quadratic regulator, the input of the active-set
//! dynamic programming algorithms (`algorithms::dp`).
//!
//! ```text
//!   min  x(N)'P x(N) + sum_{k<N} x(k)'Q x(k) + u(k)'R u(k)        (the QP carries a factor 1/2)
//!   s.t. x(k+1) = A x(k) + B u(k),  u(k) in U, x(k) in X (k < N),  x(N) in T
//! ```
//!
//! with polytopes `{z : H z <= k}`. The QP in the input sequence U = (u(0), ..., u(N-1)) is
//! `1/2 U'H U + x0' F' U` s.t. `G U <= w + E x0`; the rows of `G, E, w` follow the stagewise order
//! of Mitze and Moennigmann (2020, eq. 3): for k = 0..N-1 the rows of u(k) and then of x(k), then
//! the rows of the terminal set. The x(0) rows have `G = 0` (they bound the parameter).

use crate::linalg::Mat;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Polytope {
    #[serde(rename = "H")]
    pub h: Vec<Vec<f64>>,
    pub k: Vec<f64>,
}

/// Rows of the condensed problem as stored in the benchmark JSON before constraint processing.
#[derive(Clone, Debug)]
pub struct OriginalRows {
    pub a: Mat,
    pub b: Vec<f64>,
    pub f: Mat,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MpcProblem {
    #[serde(rename = "A")]
    pub a: Vec<Vec<f64>>,
    #[serde(rename = "B")]
    pub b: Vec<Vec<f64>>,
    #[serde(rename = "Q")]
    pub q: Vec<Vec<f64>>,
    #[serde(rename = "R")]
    pub r: Vec<Vec<f64>>,
    #[serde(rename = "P")]
    pub p: Vec<Vec<f64>>,
    #[serde(rename = "N")]
    pub n: usize,
    pub u_poly: Polytope,
    pub x_poly: Polytope,
    pub t_poly: Polytope,
    /// DP constraint row -> row of the condensed problem's `A` (before RPOPT's processing); -1 for
    /// the x(0) rows. Length `N (q_U + q_X) + q_T`.
    pub row_map: Vec<i64>,
    #[serde(skip)]
    pub original: Option<OriginalRows>,
}

/// The horizon-n QP: `1/2 U'H U + x0' fm' ... ` is `H`, `fm` (n_u n x n_x), `G U <= w + E x0`.
#[derive(Clone, Debug)]
pub struct HorizonQp {
    pub n: usize,
    pub nm: usize,
    pub nx: usize,
    /// constraints per stage (u and x rows) and terminal rows
    pub q0: usize,
    pub qt: usize,
    pub h: Mat,
    /// F of the paper transposed: gradient term is `fm x0`
    pub fm: Mat,
    pub g: Mat,
    pub e: Mat,
    pub w: Vec<f64>,
}

impl HorizonQp {
    pub fn num_rows(&self) -> usize {
        self.n * self.q0 + self.qt
    }
}

impl MpcProblem {
    pub fn nx(&self) -> usize {
        self.a.len()
    }

    pub fn nu(&self) -> usize {
        self.b[0].len()
    }

    /// Number of constraints per stage (rows of U and X).
    pub fn q0(&self) -> usize {
        self.u_poly.k.len() + self.x_poly.k.len()
    }

    /// Reads the `"mpc"` entry (and the original constraint rows) of a benchmark problem JSON.
    pub fn from_json_file(path: &std::path::Path) -> std::io::Result<Self> {
        let txt = std::fs::read_to_string(path)?;
        let v: serde_json::Value = serde_json::from_str(&txt).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let inv = |m: String| std::io::Error::new(std::io::ErrorKind::InvalidData, m);
        let mpc = v.get("mpc").ok_or_else(|| inv("no \"mpc\" entry".into()))?;
        let mut p: MpcProblem = serde_json::from_value(mpc.clone()).map_err(|e| inv(e.to_string()))?;
        let rows = |key: &str| -> std::io::Result<Vec<Vec<f64>>> { serde_json::from_value(v[key].clone()).map_err(|e| inv(format!("{key}: {e}"))) };
        let flat = |m: Vec<Vec<f64>>| m.into_iter().flatten().collect::<Vec<f64>>();
        p.original = Some(OriginalRows { a: Mat::from_rows(&rows("A")?), b: flat(rows("b")?), f: Mat::from_rows(&rows("F")?) });
        Ok(p)
    }

    /// The horizon-n problem (same data, horizon changed). Cost matrices use the convention of
    /// `bench/mpc_problems.py`: `H = I (x) R + sum_k Gamma_k' W_k Gamma_k`, `fm = sum_k Gamma_k' W_k Phi_k`.
    pub fn horizon_qp(&self, n: usize) -> HorizonQp {
        let (nx, nu) = (self.nx(), self.nu());
        let a = Mat::from_rows(&self.a);
        let b = Mat::from_rows(&self.b);
        let (qm, rm, pm) = (Mat::from_rows(&self.q), Mat::from_rows(&self.r), Mat::from_rows(&self.p));
        let nm = nu * n;
        let mut phi = vec![Mat::eye(nx)]; // A^k
        for k in 1..=n {
            let next = a.matmul(&phi[k - 1]);
            phi.push(next);
        }
        // x(k) = phi[k] x0 + gamma[k] U
        let mut gamma = vec![Mat::zeros(nx, nm)];
        for k in 1..=n {
            let mut gk = Mat::zeros(nx, nm);
            for j in 0..k {
                let blk = phi[k - 1 - j].matmul(&b);
                for r in 0..nx {
                    for c in 0..nu {
                        gk[(r, j * nu + c)] = blk[(r, c)];
                    }
                }
            }
            gamma.push(gk);
        }
        let mut h = Mat::zeros(nm, nm);
        for k in 0..n {
            for r in 0..nu {
                for c in 0..nu {
                    h[(k * nu + r, k * nu + c)] = rm[(r, c)];
                }
            }
        }
        let mut fm = Mat::zeros(nm, nx);
        for k in 1..=n {
            let w = if k == n { &pm } else { &qm };
            let gt = gamma[k].transpose();
            h = h.add(&gt.matmul(w).matmul(&gamma[k]));
            fm = fm.add(&gt.matmul(w).matmul(&phi[k]));
        }
        let h = h.add(&h.transpose()).scale(0.5);
        let (hu, ku) = (Mat::from_rows(&self.u_poly.h), &self.u_poly.k);
        let (hx, kx) = (Mat::from_rows(&self.x_poly.h), &self.x_poly.k);
        let (ht, kt) = (Mat::from_rows(&self.t_poly.h), &self.t_poly.k);
        let q0 = ku.len() + kx.len();
        let qt = kt.len();
        let q = n * q0 + qt;
        let mut g = Mat::zeros(q, nm);
        let mut e = Mat::zeros(q, nx);
        let mut w = Vec::with_capacity(q);
        let mut row = 0;
        for k in 0..n {
            for r in 0..ku.len() {
                for c in 0..nu {
                    g[(row, k * nu + c)] = hu[(r, c)];
                }
                w.push(ku[r]);
                row += 1;
            }
            let hg = hx.matmul(&gamma[k]);
            let hp = hx.matmul(&phi[k]);
            for r in 0..kx.len() {
                g.row_mut(row).copy_from_slice(hg.row(r));
                for c in 0..nx {
                    e[(row, c)] = -hp[(r, c)];
                }
                w.push(kx[r]);
                row += 1;
            }
        }
        let hg = ht.matmul(&gamma[n]);
        let hp = ht.matmul(&phi[n]);
        for r in 0..kt.len() {
            g.row_mut(row).copy_from_slice(hg.row(r));
            for c in 0..nx {
                e[(row, c)] = -hp[(r, c)];
            }
            w.push(kt[r]);
            row += 1;
        }
        HorizonQp { n, nm, nx, q0, qt, h, fm, g, e, w }
    }
}
