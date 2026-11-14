//! Ahmadi-Moshkenani, Johansen, Olaru (IEEE TAC 2018, DOI 10.1109/TAC.2018.2791479):
//! combinatorial mpQP solver that explores the tree of active sets downward and upward from the
//! optimal sets (adjacent critical regions differ by one constraint when LICQ and SCS hold on the
//! common facet), with explicit handling of degenerate cases (Algorithm 1 of the paper).
//!
//! Implementation notes (see the paper's Algorithm 1):
//! * Phase I seeds the search from level 1 (and deeper levels while no optimal set with LICQ and
//!   SCS is found) and collects the Feasible Set of constraints.
//! * Phase II/step 4: every optimal set with LICQ and SCS (a "good" set) has its subsets with one
//!   element less and supersets with one more feasible constraint tested for optimality; sets with
//!   an LICQ violation only have their one-element-less subsets explored (Theorems 3a, 5); sets
//!   with an SCS violation contribute the superset (active plus weakly inactive constraints) to
//!   the SCS Set.
//! * Phase III explores all full-row-rank subsets (at most n_x elements) of the union of the SCS
//!   Set without further feasibility tests, until nothing new is found.
//!
//! Optimality is decided in parameter space instead of with the paper's LP (6) over
//! (z, theta, lambda, s, t): for a full-row-rank set the KKT system is feasible iff the unreduced
//! critical region is nonempty, and SCS holds iff no multiplier or slack function vanishes
//! identically on it (a nonempty strict region is open in theta-space, hence full-dimensional);
//! for a rank-deficient set see `licq_optimal`. The decisions are those of LP (6).
//! Library convention (an extension to the paper): a full-dimensional region of an SCS-violating
//! full-rank set is also reported, as the other algorithms do.
//! Not for mpLPs (the paper treats mpQPs): those are delegated to `combinatorial`.

use crate::critical_region::CriticalRegion;
use crate::fxhash::FxHashSet as HashSet;
use crate::linalg::{dot, is_full_rank_rows, Cholesky, Mat};
use crate::mpqp_utils::{gen_cr_from_kkt, kkt_region_from_law, lambda_rows_feasible_raw};
use crate::program::{ControlLaw, MpProgram};
use std::cell::RefCell;

enum Class {
    NotOpt,
    /// optimal, LICQ and SCS hold, full-dimensional region
    Good(Box<CriticalRegion>),
    /// optimal with SCS violated: optionally the (full-dimensional) region, and the superset of
    /// active and weakly inactive constraints
    Scs(Option<Box<CriticalRegion>>, Vec<usize>),
    /// optimal with LICQ violated
    Licq,
}

/// A candidate with, if it is `parent + {hint}` for a parent with full row rank, that constraint.
type Work = (Vec<usize>, Option<usize>);

/// Law and critical-region rows of a full-row-rank set, kept for the rank-deficient children
/// `A + {d}` of that set (see `licq_optimal`).
struct Prep {
    set: Vec<usize>,
    e: Mat,
    f: Vec<f64>,
    n_lambda: usize,
    inactive: Vec<usize>,
    chol: Cholesky,
    /// rows (after the lambda rows) that can bind inside the parameter bounding box
    keep: Vec<bool>,
    n_keep: usize,
}

struct Ctx<'a> {
    program: &'a MpProgram,
    nx: usize,
    eq: Vec<usize>,
    visited: HashSet<Vec<usize>>,
    feasible: Vec<usize>,
    regions: Vec<CriticalRegion>,
    work: Vec<Work>,
    scs_union: Vec<bool>,
    union_changed: bool,
    prep: RefCell<Option<Prep>>,
}

pub fn ahmadi(program: &MpProgram) -> Vec<CriticalRegion> {
    if !program.is_qp() {
        return super::combinatorial(program, false);
    }
    let m = program.num_constraints();
    let nx = program.num_x();
    let eq = program.equality_indices.clone();
    let mut cx = Ctx {
        program,
        nx,
        eq: eq.clone(),
        visited: HashSet::default(),
        feasible: vec![],
        regions: vec![],
        work: vec![],
        scs_union: vec![false; m],
        union_changed: false,
        prep: RefCell::new(None),
    };
    cx.phase1(m);
    loop {
        cx.drain();
        if !cx.phase3() {
            break;
        }
    }
    cx.regions
}

fn sorted_with(base: &[usize], extra: &[usize]) -> Vec<usize> {
    let mut v = base.to_vec();
    v.extend_from_slice(extra);
    v.sort_unstable();
    v
}

impl<'a> Ctx<'a> {
    /// Classifies a candidate set (never tested before) as in the paper's optimality test.
    fn classify(&self, a: &[usize], hint: Option<usize>) -> Class {
        let p = self.program;
        crate::stats::rank();
        if a.len() <= self.nx {
            // Cholesky only first: a failure means (numerically) deficient rank, which the
            // rank test below confirms before the LU fallback is tried
            let law = match p.optimal_control_law_dual_rank(a, false) {
                Some((law, true)) => Some(law),
                Some((law, false)) => is_full_rank_rows(&p.a, a).then_some(law),
                None => {
                    if is_full_rank_rows(&p.a, a) {
                        p.optimal_control_law_dual(a)
                    } else {
                        None
                    }
                }
            };
            if let Some(law) = law {
                return self.classify_full_rank(a, law);
            }
        }
        // LICQ violated: the feasibility part of LP (6)
        crate::stats::lp(1);
        if self.licq_optimal(a, hint) {
            Class::Licq
        } else {
            Class::NotOpt
        }
    }

    /// Optimality of a rank-deficient set B (the feasibility part of the paper's LP (6)) in
    /// parameter space. With A a maximal independent subset of B (equalities first) and
    /// D = B \ A, each d in D has A_d = sum_k c_dk A_k. The KKT system of B is solvable at theta
    /// iff
    /// (i) lambda_A(theta) = lambda' + sum_d mu_d c_d with lambda' >= 0 (inequality rows) and
    ///     mu >= 0 (the multipliers of B are lambda' and mu),
    /// (ii) the rows of D hold with equality at z_A(theta) (slack_d(theta) = 0),
    /// (iii) the other constraints are satisfied, and (iv) theta is in the parameter set.
    /// This is a feasibility LP over (theta, mu) instead of the LP over (z, theta, lambda, s, t).
    /// When `hint = Some(d)` the set A = B \ {d} is the (full-rank) parent, whose law and rows are
    /// reused by the siblings.
    fn licq_optimal(&self, b: &[usize], hint: Option<usize>) -> bool {
        let p = self.program;
        let nt = p.num_t();
        let neq = p.num_equality_constraints();
        let (a_set, d_set): (Vec<usize>, Vec<usize>) = match hint {
            Some(d) => (b.iter().cloned().filter(|&i| i != d).collect(), vec![d]),
            None => {
                // greedy maximal independent subset, equalities first
                let mut order: Vec<usize> = b.iter().cloned().filter(|i| self.eq.contains(i)).collect();
                order.extend(b.iter().cloned().filter(|i| !self.eq.contains(i)));
                let mut a_set: Vec<usize> = vec![];
                let mut d_set: Vec<usize> = vec![];
                for &k in &order {
                    a_set.push(k);
                    if !is_full_rank_rows(&p.a, &a_set) {
                        a_set.pop();
                        d_set.push(k);
                    }
                }
                a_set.sort_unstable();
                (a_set, d_set)
            }
        };
        if d_set.is_empty() {
            return false; // cannot happen for a rank-deficient set
        }
        let mut cache = self.prep.borrow_mut();
        if cache.as_ref().map(|c| c.set != a_set).unwrap_or(true) {
            // the law and region rows assume the equalities lead the active set (as in PPOPT)
            let Some(law) = p.optimal_control_law_dual(&a_set) else { return false };
            let (e, f, n_lambda, inactive) = p.region_rows(&a_set, &law);
            let r = a_set.len();
            let mut g = Mat::zeros(r, r);
            for i in 0..r {
                for j in 0..=i {
                    let v = dot(p.a.row(a_set[i]), p.a.row(a_set[j]));
                    g[(i, j)] = v;
                    g[(j, i)] = v;
                }
            }
            let Some(chol) = Cholesky::new(&g) else { return false };
            let mut keep = vec![true; e.rows];
            if let Some((lo, hi)) = p.theta_bbox() {
                for rr in n_lambda..e.rows {
                    let ub: f64 = e.row(rr).iter().zip(lo.iter().zip(hi)).map(|(a, (l, h))| (a * l).max(a * h)).sum();
                    if ub < f[rr] - 1e-9 * (1.0 + f[rr].abs()) {
                        keep[rr] = false;
                    }
                }
            }
            let n_keep = keep.iter().filter(|&&k| k).count();
            *cache = Some(Prep { set: a_set.clone(), e, f, n_lambda, inactive, chol, keep, n_keep });
        }
        let pr = cache.as_ref().unwrap();
        let r = a_set.len();
        // coefficients c_d of the dependent rows
        let coef: Vec<Vec<f64>> = d_set
            .iter()
            .map(|&d| {
                let rhs: Vec<f64> = a_set.iter().map(|&k| dot(p.a.row(k), p.a.row(d))).collect();
                pr.chol.solve(&rhs)
            })
            .collect();
        let nd = d_set.len();
        // a dependent row that cannot reach equality inside the parameter box: not optimal
        for &d in &d_set {
            if let Ok(jj) = pr.inactive.binary_search(&d) {
                if !pr.keep[pr.n_lambda + jj] {
                    return false;
                }
            }
        }
        let nv = nt + nd;
        let rows = pr.n_keep + nd;
        let mut lp_a = Mat::zeros(rows, nv);
        let mut lp_b: Vec<f64> = Vec::with_capacity(rows);
        let mut eqs: Vec<usize> = vec![];
        // rows of the region (lambda >= 0, inactive constraints, parameter set) over theta, with
        // lambda' = lambda - sum_d c_d mu_d in the lambda rows and equality for the rows of D
        let mut pos = 0;
        for rr in 0..pr.e.rows {
            if !pr.keep[rr] {
                continue;
            }
            let out = lp_a.row_mut(pos);
            out[..nt].copy_from_slice(pr.e.row(rr));
            if rr < pr.n_lambda {
                let ii = neq + rr;
                for (di, c) in coef.iter().enumerate() {
                    out[nt + di] = c[ii];
                }
            } else if rr < pr.n_lambda + pr.inactive.len() && d_set.contains(&pr.inactive[rr - pr.n_lambda]) {
                eqs.push(pos);
            }
            lp_b.push(pr.f[rr]);
            pos += 1;
        }
        for di in 0..nd {
            lp_a.row_mut(pr.n_keep + di)[nt + di] = -1.0;
            lp_b.push(0.0);
        }
        let _ = r;
        crate::solver::lp::lp_feasible(&lp_a, &lp_b, &eqs)
    }

    fn classify_full_rank(&self, a: &[usize], law: ControlLaw) -> Class {
        let p = self.program;
        crate::stats::lp(1);
        if p.num_t() > 1 && !lambda_rows_feasible_raw(p, &law.a_l, &law.b_l) {
            return Class::NotOpt;
        }
        let mut kkt = kkt_region_from_law(p, a, law);
        crate::stats::lp(1);
        if !kkt.nonempty_in(p) {
            return Class::NotOpt;
        }
        // multipliers and slacks that vanish identically
        let tol = 1e-8;
        let zero_row = |r: usize| kkt.e.row(r).iter().all(|v| v.abs() <= tol) && kkt.f[r].abs() <= tol;
        let weak_act = (0..kkt.n_lambda).any(zero_row);
        let weak_inact: Vec<usize> = kkt.inactive.iter().enumerate().filter(|&(jj, _)| zero_row(kkt.n_lambda + jj)).map(|(_, &j)| j).collect();
        let saved = (kkt.e.clone(), kkt.f.clone(), kkt.inactive.clone(), kkt.n_lambda, kkt.point.clone());
        match gen_cr_from_kkt(p, a, kkt, true) {
            Some(r) => {
                if !weak_act && weak_inact.is_empty() {
                    Class::Good(Box::new(r))
                } else {
                    Class::Scs(Some(Box::new(r)), sorted_with(a, &weak_inact))
                }
            }
            None => {
                // lower-dimensional region: weakly inactive = slack zero over the whole region
                let (e, f, inactive, n_lambda, point) = saved;
                let mut weak = weak_inact;
                for (jj, &j) in inactive.iter().enumerate() {
                    let r = n_lambda + jj;
                    if weak.contains(&j) {
                        continue;
                    }
                    let near = match &point {
                        Some(pt) => f[r] - dot(e.row(r), pt) <= 1e-6 * (1.0 + f[r].abs()),
                        None => true,
                    };
                    if !near {
                        continue;
                    }
                    crate::stats::lp(1);
                    if let Some(s) = crate::solver::lp::solve_lp(Some(e.row(r)), &e, &f, &[]) {
                        if f[r] - s.obj <= 1e-8 * (1.0 + f[r].abs()) {
                            weak.push(j);
                        }
                    }
                }
                Class::Scs(None, sorted_with(a, &weak))
            }
        }
    }

    /// Records the outcome of a classified set and queues the sets that follow from it.
    fn handle(&mut self, a: Vec<usize>, class: Class) {
        match class {
            Class::NotOpt => {}
            Class::Good(r) => {
                self.regions.push(*r);
                // upward: one element less (equalities stay)
                for k in 0..a.len() {
                    if self.eq.contains(&a[k]) {
                        continue;
                    }
                    let mut s = a.clone();
                    s.remove(k);
                    self.push(s, None);
                }
                // downward: one more feasible constraint (the parent has full row rank)
                for fi in 0..self.feasible.len() {
                    let i = self.feasible[fi];
                    if a.binary_search(&i).is_err() {
                        let mut s = a.clone();
                        let pos = s.partition_point(|&v| v < i);
                        s.insert(pos, i);
                        self.push(s, Some(i));
                    }
                }
            }
            Class::Scs(r, sup) => {
                if let Some(r) = r {
                    self.regions.push(*r);
                }
                for i in sup {
                    if !self.scs_union[i] {
                        self.scs_union[i] = true;
                        self.union_changed = true;
                    }
                }
            }
            Class::Licq => {
                for k in 0..a.len() {
                    if self.eq.contains(&a[k]) {
                        continue;
                    }
                    let mut s = a.clone();
                    s.remove(k);
                    self.push(s, None);
                }
            }
        }
    }

    fn push(&mut self, s: Vec<usize>, hint: Option<usize>) {
        if !self.visited.contains(&s) {
            self.work.push((s, hint));
        }
    }

    fn drain(&mut self) {
        while let Some((s, hint)) = self.work.pop() {
            if !self.visited.insert(s.clone()) {
                continue;
            }
            let c = self.classify(&s, hint);
            self.handle(s, c);
        }
    }

    /// Phase I: level 1 (and deeper while no good set exists) with the Feasible Set.
    fn phase1(&mut self, m: usize) {
        let p = self.program;
        let eq = self.eq.clone();
        let mut results: Vec<(Vec<usize>, Class)> = vec![];
        self.visited.insert(eq.clone());
        results.push((eq.clone(), self.classify(&eq, None)));
        let mut found_good = matches!(results[0].1, Class::Good(_));
        for i in (0..m).filter(|i| !eq.contains(i)) {
            let s = sorted_with(&eq, &[i]);
            self.visited.insert(s.clone());
            let c = self.classify(&s, None);
            let feasible = match c {
                Class::NotOpt => p.check_feasibility(&s, true),
                _ => true,
            };
            if feasible {
                self.feasible.push(i);
            }
            found_good |= matches!(c, Class::Good(_));
            results.push((s, c));
        }
        // deeper levels while no optimal set with LICQ and SCS exists
        let mut level = 2;
        while !found_good && level <= self.nx.saturating_sub(eq.len()) {
            let free = self.feasible.clone();
            let mut combo: Vec<usize> = vec![];
            let mut sets: Vec<Vec<usize>> = vec![];
            combos(&free, level, 0, &mut combo, &mut sets);
            for c in sets {
                let s = sorted_with(&eq, &c);
                if !self.visited.insert(s.clone()) {
                    continue;
                }
                let cl = self.classify(&s, None);
                found_good |= matches!(cl, Class::Good(_));
                results.push((s, cl));
            }
            level += 1;
        }
        for (s, c) in results {
            self.handle(s, c);
        }
    }

    /// Phase III: untested full-row-rank subsets (size at most n_x) of the SCS union. Returns
    /// whether the union changed since the last call (then new sets may have been queued).
    fn phase3(&mut self) -> bool {
        if !self.union_changed {
            return false;
        }
        self.union_changed = false;
        let p = self.program;
        let free: Vec<usize> = (0..self.scs_union.len()).filter(|&i| self.scs_union[i] && !self.eq.contains(&i)).collect();
        let max_free = self.nx.saturating_sub(self.eq.len());
        let mut results = vec![];
        for k in 0..=max_free.min(free.len()) {
            let mut sets = vec![];
            let mut combo = vec![];
            combos(&free, k, 0, &mut combo, &mut sets);
            for c in sets {
                let s = sorted_with(&self.eq, &c);
                if self.visited.contains(&s) {
                    continue;
                }
                crate::stats::rank();
                if !is_full_rank_rows(&p.a, &s) {
                    continue;
                }
                self.visited.insert(s.clone());
                let cl = self.classify(&s, None);
                results.push((s, cl));
            }
        }
        for (s, c) in results {
            self.handle(s, c);
        }
        true
    }
}

fn combos(items: &[usize], k: usize, start: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if cur.len() == k {
        out.push(cur.clone());
        return;
    }
    for i in start..items.len() {
        if items.len() - i < k - cur.len() {
            break;
        }
        cur.push(items[i]);
        combos(items, k, i + 1, cur, out);
        cur.pop();
    }
}
