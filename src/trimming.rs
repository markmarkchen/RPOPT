//! Constraint trimming from previously solved instances.
//!
//! Hou and You, "Reliably learn to trim multiparametric quadratic programs via constraint
//! removal", arXiv:2412.11098 (IEEE TAC). Algorithm numbers refer to that paper.
//!
//! WHAT IT DOES AND WHEN IT HELPS. The paper trims the QP *instance* QP(theta) at one parameter
//! value (Algorithm 1 with one solved instance, Algorithm 2 with several): from solved instances
//! (theta_k, z*_k, active set A_k) and a global Lipschitz constant kappa of the minimizer
//!   ||z*(theta1, I) - z*(theta2, I)|| <= kappa ||theta1 - theta2||   for every subset I,
//! a constraint j that is inactive at the solved instance is dropped when
//!   (b_j + F_j theta - A_j z*_k) / ||A_j|| >= kappa ||theta - theta_k||
//! (the ball B(z*_k, kappa ||theta - theta_k||) then lies inside constraint j, Lemma 1 and
//! Remark 3). Theorem 1/2 guarantee the same optimal solution at theta. So it helps *sequences of
//! related QPs* (MPC in closed loop, a grid of solves), where earlier solves are near the next
//! parameter; the savings grow with the number of learned instances and shrink with the distance.
//!
//! It does NOT help a single explicit mpQP over a whole parameter set: the trimming at theta needs
//! solved instances near theta, and a region-wide guarantee has to hold for every theta in the
//! region, so with a large parameter set the ball radius kappa*distance exceeds every slack and
//! nothing is removed (only constraints that are redundant over the whole box are, which the
//! offline `process_constraints` removes already). `trim_ball` is the (conservative) extension
//! of the paper's condition to all theta in a ball, usable to solve an mpQP restricted to a
//! small parameter region (partition-and-solve). The paper does not state that extension; it
//! follows from the same Lemma 1 by bounding the slack and the distance over the ball.
//!
//! Deviations: the paper computes kappa through MILPs (kappa_1, kappa_inf, kappa = sqrt(kappa_1
//! kappa_inf)). `lipschitz_by_enumeration` instead enumerates all LICQ subsets of at most n_x
//! constraints with a nonempty multiplier region and takes the largest 2-norm of the slope of
//! z*(theta, I), which is the exact smallest 2-norm GLC (paper section IV-A, kappa_2); it is only
//! feasible for small constraint counts, so the caller can also pass a kappa.
//! The paper's guarantee needs LICQ at every solved instance (Assumption 1); when an instance
//! violates it, only the closest instance is used (the paper's remedy, section III-D).

use crate::linalg::{dot, is_full_rank_rows, norm, Mat};
use crate::program::MpProgram;

#[derive(Clone, Debug)]
pub struct SolvedInstance {
    pub theta: Vec<f64>,
    pub z: Vec<f64>,
    /// active constraint indices (equalities included)
    pub active: Vec<usize>,
    /// LICQ holds for the active rows
    pub licq: bool,
}

/// Solves QP(theta) and records the pair (z*, A) the trimming learns from.
pub fn solve_instance(prog: &MpProgram, theta: &[f64]) -> Option<SolvedInstance> {
    let s = prog.solve_theta_uncounted(theta)?;
    let licq = is_full_rank_rows(&prog.a, &s.active_set);
    Some(SolvedInstance { theta: theta.to_vec(), z: s.sol, active: s.active_set, licq })
}

fn slack(prog: &MpProgram, inst: &SolvedInstance, j: usize, theta: &[f64]) -> f64 {
    prog.b[j] + dot(prog.f.row(j), theta) - dot(prog.a.row(j), &inst.z)
}

/// Constraints kept for the instance at `theta` (paper Algorithm 2; Algorithm 1 for one instance).
/// Always contains the equality constraints.
pub fn trim_point(prog: &MpProgram, theta: &[f64], instances: &[SolvedInstance], kappa: f64) -> Vec<usize> {
    trim_generic(prog, instances, kappa, |inst, j| {
        let an = norm(prog.a.row(j));
        let dist = norm(&theta.iter().zip(&inst.theta).map(|(a, b)| a - b).collect::<Vec<_>>());
        (slack(prog, inst, j, theta), an, dist)
    }, || {
        instances.iter().map(|i| norm(&theta.iter().zip(&i.theta).map(|(a, b)| a - b).collect::<Vec<_>>())).collect()
    })
}

/// Constraints kept for every theta in the ball ||theta - center|| <= radius: the same test with
/// the slack bounded below and the distance bounded above over the ball.
pub fn trim_ball(prog: &MpProgram, center: &[f64], radius: f64, instances: &[SolvedInstance], kappa: f64) -> Vec<usize> {
    trim_generic(prog, instances, kappa, |inst, j| {
        let an = norm(prog.a.row(j));
        let dist = norm(&center.iter().zip(&inst.theta).map(|(a, b)| a - b).collect::<Vec<_>>()) + radius;
        (slack(prog, inst, j, center) - norm(prog.f.row(j)) * radius, an, dist)
    }, || {
        instances.iter().map(|i| norm(&center.iter().zip(&i.theta).map(|(a, b)| a - b).collect::<Vec<_>>())).collect()
    })
}

fn trim_generic(
    prog: &MpProgram,
    instances: &[SolvedInstance],
    kappa: f64,
    info: impl Fn(&SolvedInstance, usize) -> (f64, f64, f64),
    dists: impl Fn() -> Vec<f64>,
) -> Vec<usize> {
    let m = prog.num_constraints();
    if instances.is_empty() {
        return (0..m).collect();
    }
    // without LICQ at every instance only the closest one may be used (paper section III-D)
    let all_licq = instances.iter().all(|i| i.licq);
    let used: Vec<&SolvedInstance> = if all_licq {
        instances.iter().collect()
    } else {
        let d = dists();
        let best = (0..instances.len()).min_by(|&a, &b| d[a].total_cmp(&d[b])).unwrap();
        vec![&instances[best]]
    };
    let mut in_i = vec![true; m];
    for inst in used {
        let mut is_active = vec![false; m];
        for &a in &inst.active {
            is_active[a] = true;
        }
        for j in 0..m {
            if !in_i[j] || is_active[j] {
                continue;
            }
            let (sl, an, dist) = info(inst, j);
            // keep j (it stays in I) when the ball around z* may cross it
            let removable = an > 0.0 && sl / an >= kappa * dist;
            if removable {
                in_i[j] = false;
            }
        }
        // I <- A U T: active constraints of the instance stay
        for &a in &inst.active {
            in_i[a] = true;
        }
    }
    (0..m).filter(|&j| in_i[j]).collect()
}

/// A copy of `prog` with only the constraints in `kept` (sorted, equalities first as in `prog`).
/// Returns the program and the map new index -> old index. The parameter set is unchanged.
pub fn trimmed_program(prog: &MpProgram, kept: &[usize]) -> (MpProgram, Vec<usize>) {
    let mut keep: Vec<usize> = kept.to_vec();
    keep.sort_unstable();
    keep.dedup();
    let mut p = prog.clone();
    p.a = prog.a.select_rows(&keep);
    p.f = prog.f.select_rows(&keep);
    p.b = keep.iter().map(|&i| prog.b[i]).collect();
    p.equality_indices = keep.iter().enumerate().filter(|&(_, &old)|prog.equality_indices.contains(&old)).map(|(new, _)| new).collect();
    p.invalidate_caches();
    (p, keep)
}

/// A copy of `prog` whose parameter set is additionally restricted to the box [lo, hi].
pub fn restrict_theta_box(prog: &MpProgram, lo: &[f64], hi: &[f64]) -> MpProgram {
    let nt = prog.num_t();
    let mut p = prog.clone();
    for k in 0..nt {
        let mut r = vec![0.0; nt];
        r[k] = 1.0;
        p.a_t.push_row(&r);
        p.b_t.push(hi[k]);
        r[k] = -1.0;
        p.a_t.push_row(&r);
        p.b_t.push(-lo[k]);
    }
    p.invalidate_caches();
    p
}

/// Reusable preprocessing step: learns solved instances and trims programs / instances.
pub struct Trimmer {
    pub kappa: f64,
    pub instances: Vec<SolvedInstance>,
}

impl Trimmer {
    pub fn new(kappa: f64) -> Self {
        Trimmer { kappa, instances: vec![] }
    }

    /// Solves QP(theta) and learns from it; returns false if infeasible.
    pub fn learn(&mut self, prog: &MpProgram, theta: &[f64]) -> bool {
        match solve_instance(prog, theta) {
            Some(i) => {
                self.instances.push(i);
                true
            }
            None => false,
        }
    }

    pub fn kept_at(&self, prog: &MpProgram, theta: &[f64]) -> Vec<usize> {
        trim_point(prog, theta, &self.instances, self.kappa)
    }

    pub fn trim_at(&self, prog: &MpProgram, theta: &[f64]) -> (MpProgram, Vec<usize>) {
        trimmed_program(prog, &self.kept_at(prog, theta))
    }

    pub fn kept_in_ball(&self, prog: &MpProgram, center: &[f64], radius: f64) -> Vec<usize> {
        trim_ball(prog, center, radius, &self.instances, self.kappa)
    }

    pub fn trim_ball(&self, prog: &MpProgram, center: &[f64], radius: f64) -> (MpProgram, Vec<usize>) {
        trimmed_program(prog, &self.kept_in_ball(prog, center, radius))
    }
}

/// Exact smallest 2-norm global Lipschitz constant of theta -> z*(theta, I) over all subsets I
/// (paper (5), kappa_2): the largest 2-norm of the slope of the equality-constrained solution of
/// any LICQ set of at most n_x constraints (equalities included) whose multipliers can be
/// nonnegative for some theta in R^{n_t}. Returns `None` when more than `max_subsets` subsets
/// would have to be examined (the paper uses MILPs for large problems).
pub fn lipschitz_by_enumeration(prog: &MpProgram, max_subsets: usize) -> Option<f64> {
    let m = prog.num_constraints();
    let nx = prog.num_x();
    let neq = prog.equality_indices.len();
    let ineq: Vec<usize> = (0..m).filter(|i| !prog.equality_indices.contains(i)).collect();
    let max_extra = nx.saturating_sub(neq).min(ineq.len());
    // number of subsets
    let mut total: f64 = 0.0;
    let mut c = 1.0f64;
    for k in 0..=max_extra {
        total += c;
        c = c * (ineq.len() - k) as f64 / (k + 1) as f64;
    }
    if total > max_subsets as f64 {
        return None;
    }
    let mut best = 0.0f64;
    let mut set: Vec<usize> = prog.equality_indices.clone();
    fn rec(prog: &MpProgram, ineq: &[usize], start: usize, left: usize, set: &mut Vec<usize>, best: &mut f64) {
        // evaluate the current set
        if let Some(law) = prog.optimal_control_law(set) {
            let ok = is_full_rank_rows(&prog.a, set) && dual_region_nonempty(prog, set, &law);
            if ok {
                let s = spectral_norm(&law.a_x);
                if s > *best {
                    *best = s;
                }
            }
        }
        if left == 0 {
            return;
        }
        for i in start..ineq.len() {
            set.push(ineq[i]);
            if is_full_rank_rows(&prog.a, set) {
                rec(prog, ineq, i + 1, left - 1, set, best);
            }
            set.pop();
        }
    }
    rec(prog, &ineq, 0, max_extra, &mut set, &mut best);
    Some(best)
}

fn dual_region_nonempty(prog: &MpProgram, set: &[usize], law: &crate::program::ControlLaw) -> bool {
    let neq = prog.equality_indices.len();
    if set.len() <= neq {
        return true;
    }
    // -L theta <= l for the inequality multipliers, theta free
    let k = set.len() - neq;
    let nt = prog.num_t();
    let mut e = Mat::zeros(k, nt);
    let mut f = vec![0.0; k];
    for r in 0..k {
        for t in 0..nt {
            e[(r, t)] = -law.a_l[(neq + r, t)];
        }
        f[r] = law.b_l[neq + r];
    }
    crate::solver::lp::lp_feasible(&e, &f, &[])
}

fn spectral_norm(a: &Mat) -> f64 {
    if a.rows == 0 || a.cols == 0 {
        return 0.0;
    }
    let g = a.transpose().matmul(a);
    let ev = nalgebra::SymmetricEigen::new(g.to_nalgebra());
    ev.eigenvalues.iter().cloned().fold(0.0, f64::max).sqrt()
}

/// Largest slope norm over the regions of an exact solution: a LOWER estimate of the GLC (only
/// the optimal active sets of the full problem, not every subset), so it is not a certificate.
pub fn lipschitz_lower_estimate(sol: &crate::solution::Solution) -> f64 {
    sol.critical_regions.iter().map(|r| spectral_norm(&r.a)).fold(0.0, f64::max)
}
