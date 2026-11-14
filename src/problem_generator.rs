//! Port of `ppopt.problem_generator` (random mpLP / mpQP instances).

use crate::linalg::{min_eigenvalue_sym, Mat};
use crate::program::MpProgram;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Random mpQP with `x` variables, `t` parameters and `m` constraints (plus 2x variable bounds).
pub fn generate_mpqp(x: usize, t: usize, m: usize, seed: u64) -> MpProgram {
    let (a, b, c, h, q, a_t, b_t, f) = raw(x, t, m, seed);
    MpProgram::new(a, b, c, h, Some(q), a_t, b_t, f, vec![], true)
}

pub fn generate_mplp(x: usize, t: usize, m: usize, seed: u64) -> MpProgram {
    let (a, b, c, h, _q, a_t, b_t, f) = raw(x, t, m, seed);
    MpProgram::new(a, b, c, h, None, a_t, b_t, f, vec![], true)
}

#[allow(clippy::type_complexity)]
fn raw(x: usize, t: usize, m: usize, seed: u64) -> (Mat, Vec<f64>, Vec<f64>, Mat, Mat, Mat, Vec<f64>, Mat) {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut r = Mat::zeros(x, x);
    for v in r.data.iter_mut() {
        *v = rng.r#gen::<f64>();
    }
    let q = r.transpose().matmul(&r).add(&Mat::eye(x));
    let mut rand = || rng.r#gen::<f64>();
    let range_value = (20.0 * rand() + 5.0).round();
    let x_border = (8.0 * rand() + 1.0).round() / 10.0;
    let x_shift = (8.0 * rand() + 1.0).round() / 10.0;
    let t_border = (8.0 * rand() + 1.0).round() / 10.0;
    let t_shift = (8.0 * rand() + 1.0).round() / 10.0;
    let denom = rand();
    let c: Vec<f64> = (0..x).map(|_| (rand() - 0.5) / denom).collect();
    let ev_min = min_eigenvalue_sym(&q);
    let ev_max = {
        let e = nalgebra::SymmetricEigen::new(q.to_nalgebra());
        e.eigenvalues.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    };
    let range = range_value * (ev_max - ev_min);
    let mut a = Mat::zeros(m + 2 * x, x);
    let mut f = Mat::zeros(m + 2 * x, t);
    for i in 0..m {
        loop {
            for j in 0..x {
                if rand() >= x_border {
                    a[(i, j)] = ((rand() - x_shift) * range).floor();
                }
            }
            if a.row(i).iter().any(|&v| v != 0.0) {
                break;
            }
        }
        for j in 0..t {
            if rand() >= t_border {
                f[(i, j)] = ((rand() - t_shift) * range).floor();
            }
        }
    }
    for j in 0..x {
        a[(m + j, j)] = 1.0;
        a[(m + x + j, j)] = -1.0;
    }
    let scale = rand();
    let mut b: Vec<f64> = (0..m).map(|_| rand() / scale).collect();
    b.extend(std::iter::repeat(1e7).take(2 * x));
    let mut a_t = Mat::zeros(2 * t, t);
    for j in 0..t {
        a_t[(j, j)] = 1.0;
        a_t[(t + j, j)] = -1.0;
    }
    let b_t = vec![range; 2 * t];
    let h = Mat::zeros(x, t);
    (a, b, c, h, q, a_t, b_t, f)
}
