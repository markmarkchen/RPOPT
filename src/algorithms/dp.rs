//! Exact explicit MPC by dynamic programming over the horizon: the set of optimal active sets for
//! horizon N+1 is built from the one for horizon N (stagewise order of the constraints).
//!
//! * R. Mitze, M. Moennigmann, "A dynamic programming approach to solving constrained
//!   linear-quadratic optimal control problems", Automatica 120, 2020 (Lemma 1, Cor. 3, Prop. 4,
//!   Algs. 2 to 4);
//! * R. Mitze, M. Moennigmann, "Improved active set dynamic programming for solving
//!   linear-quadratic optimal control problems", CDC 2022 (cardinality limit, Cor. 2, Algs. 1 to 3);
//! * R. Mitze, M. Kvasnica, M. Moennigmann, "Exploiting symmetries in active set enumeration for
//!   constrained linear-quadratic optimal control", Automatica 151, 2023 (central symmetry of the
//!   active sets, reduced solution S^red, Algs. 1 to 3, Cor. 7).
//!
//! The input is the MPC structure (`crate::mpc::MpcProblem`), not the condensed mpQP; the optimal
//! active sets found are mapped to the rows of the condensed `MpProgram` and turned into
//! `CriticalRegion`s with `gen_cr_from_active_set`, so the result is comparable with the other
//! algorithms.
//!
//! Active sets are bit masks (`u128`, bit i = constraint i of the stagewise order), so the shift
//! `A (+) {q0}` is `A << q0` and the subset test `A >= pruned` is `A & p == p`.

use crate::critical_region::CriticalRegion;
use crate::linalg::{is_full_rank_rows, Mat};
use crate::mpc::{HorizonQp, MpcProblem};
use crate::mpqp_utils::gen_cr_from_active_set;
use crate::program::MpProgram;
use crate::solution::Solution;
use crate::solver::lp::solve_lp;
use crate::fxhash::FxHashSet as HashSet;
use std::sync::Arc;
use std::time::Instant;

type Set = u128;

/// Options selecting the variant of the algorithm.
#[derive(Clone, Debug)]
pub struct DpOptions {
    /// CDC 2022: only candidate active sets with at most `N_target * n_u` elements.
    pub cardinality_limit: bool,
    /// Automatica 2023: exploit the central symmetry (u, x) -> (-u, -x) when the constraints have it.
    pub symmetry: bool,
    /// The terminal set T is the maximal admissible set of the unconstrained LQR (the setting of the
    /// 2020 paper): use Lemma 2 / Cor. 3 (copy the sets without terminal constraints, extend only
    /// the sets reaching the last stages) and stop when no active set touches the terminal set
    /// (Prop. 4). Wrong for other terminal sets (such as T = X with a LQR terminal cost).
    pub terminal_invariant: bool,
    /// Process all candidates of one horizon step by increasing cardinality (more pruning) instead
    /// of per parent set A_l as in Alg. 2. Same result.
    pub global_order: bool,
    /// Largest horizon to compute (defaults to the horizon of the MPC problem).
    pub horizon: Option<usize>,
}

impl DpOptions {
    /// Mitze and Moennigmann 2020, as published (Cor. 3 pruning needs `terminal_invariant`).
    pub fn paper2020() -> Self {
        DpOptions { cardinality_limit: false, symmetry: false, terminal_invariant: false, global_order: false, horizon: None }
    }

    /// 2020 plus the cardinality limit of the CDC 2022 paper.
    pub fn cdc2022() -> Self {
        DpOptions { cardinality_limit: true, ..Self::paper2020() }
    }

    /// 2022 plus the symmetry reduction of the 2023 paper.
    pub fn symmetric() -> Self {
        DpOptions { symmetry: true, ..Self::cdc2022() }
    }
}

/// Counters of the work done (the quantities of Fig. 4 and Table I of the papers).
#[derive(Clone, Debug, Default)]
pub struct DpStats {
    /// candidate active sets generated
    pub candidates: usize,
    /// candidates dismissed because they contain a known infeasible active set (pruning)
    pub pruned: usize,
    /// LPs (5) solved for optimality and (5) without (5b), (5c) for feasibility
    pub optimality_lps: usize,
    pub feasibility_lps: usize,
    /// rank tests of the final step
    pub rank_tests: usize,
    /// |S_n| per horizon n = 1, 2, ...
    pub set_sizes: Vec<usize>,
    /// active sets of M_N without a region in the processed program (removed redundant row) or lost
    pub unmapped: usize,
    /// wall time of the enumeration up to horizon n = 1, 2, ... (cumulative)
    pub cumulative_seconds: Vec<f64>,
    pub horizon_reached: usize,
    pub seconds_enumeration: f64,
    pub seconds_regions: f64,
}

pub struct DpResult {
    pub solution: Solution,
    pub stats: DpStats,
}

/// The optimal active sets S_N (with the flag "t* = 0, possibly lower dimensional") for the target
/// horizon, as bit masks over the stagewise constraint order, plus the horizon actually reached
/// (smaller than the target when Prop. 4 applies). With `symmetry` only one set per orbit.
pub fn enumerate_active_sets(mpc: &MpcProblem, opts: &DpOptions, stats: &mut DpStats) -> (Vec<(u128, bool)>, usize) {
    let t0 = Instant::now();
    let nbar = opts.horizon.unwrap_or(mpc.n);
    let nu = mpc.nu();
    let qps: Vec<HorizonQp> = (1..=nbar).map(|n| mpc.horizon_qp(n)).collect();
    assert!(qps[nbar - 1].num_rows() <= 128, "DP active sets are limited to 128 constraints");
    let cmax = if opts.cardinality_limit { nbar * nu } else { usize::MAX };
    let syms: Vec<Option<Vec<usize>>> = qps.iter().map(|qp| if opts.symmetry { central_symmetry(qp) } else { None }).collect();
    let q0 = qps[0].q0;
    let subsets = subsets_by_cardinality(q0);

    // S_1 (Alg. 3 / CDC Alg. 1 / symmetric Alg. 1)
    let mut cur: Vec<(Set, bool)> = initial_sets(&qps[0], cmax, syms[0].as_deref(), stats);
    stats.set_sizes.push(cur.len());
    stats.cumulative_seconds.push(t0.elapsed().as_secs_f64());
    let mut last = 1;
    for n in 1..nbar {
        let ctx = Horizon { qp: &qps[n], perm: syms[n].as_deref() };
        cur = extend(&ctx, &qps[n - 1], &cur, &subsets, cmax, opts, stats);
        stats.set_sizes.push(cur.len());
        stats.cumulative_seconds.push(t0.elapsed().as_secs_f64());
        last = n + 1;
        // Prop. 4: no active set with a terminal constraint left, so the solution does not change
        if opts.terminal_invariant && cur.iter().all(|&(a, _)| a & !mask(last * q0) == 0) {
            break;
        }
    }
    stats.seconds_enumeration = t0.elapsed().as_secs_f64();
    stats.horizon_reached = last;
    (cur, last)
}

/// Solves the explicit MPC problem; `program` is the condensed mpQP of the same problem (it must
/// have been built from the data `mpc.original` describes) and supplies the critical regions.
pub fn solve_mpc_dp(mpc: &MpcProblem, program: &Arc<MpProgram>, opts: &DpOptions) -> DpResult {
    let mut stats = DpStats::default();
    let nbar = opts.horizon.unwrap_or(mpc.n);
    let (cur, last) = enumerate_active_sets(mpc, opts, &mut stats);
    let qps: Vec<HorizonQp> = vec![mpc.horizon_qp(last), mpc.horizon_qp(nbar)];
    let syms: Vec<Option<Vec<usize>>> = qps.iter().map(|qp| if opts.symmetry { central_symmetry(qp) } else { None }).collect();
    // M_N: full row rank, and (for degenerate ones) a full-dimensional polytope; then regions
    let t1 = Instant::now();
    let qp = &qps[0];
    let maps = RowMap::new(mpc, program, qps[1].num_rows());
    let perm_last = syms[0].as_deref();
    let mut regions: Vec<CriticalRegion> = vec![];
    let mut seen: HashSet<Set> = HashSet::default();
    for &(a, degen) in &cur {
        stats.rank_tests += 1;
        let idx = bits(a);
        if !is_full_rank_rows(&qp.g, &idx) {
            continue;
        }
        let orbit = match perm_last {
            Some(p) => orbit_of(a, p),
            None => vec![a],
        };
        // Theorem 4: all members of an orbit are optimal and full-dimensional or none is
        let Some(first) = region_of(&maps, program, a, degen) else {
            continue;
        };
        for m in orbit {
            if !seen.insert(m) {
                continue;
            }
            if m == a {
                regions.push(first.clone());
            } else if let Some(cr) = region_of(&maps, program, m, false) {
                regions.push(cr);
            } else {
                stats.unmapped += 1;
            }
        }
    }
    stats.seconds_regions = t1.elapsed().as_secs_f64();
    DpResult { solution: Solution::new(program.clone(), regions), stats }
}

fn mask(n: usize) -> Set {
    if n >= 128 { !0 } else { (1u128 << n) - 1 }
}

fn bits(a: Set) -> Vec<usize> {
    let mut v = Vec::with_capacity(a.count_ones() as usize);
    let mut x = a;
    while x != 0 {
        v.push(x.trailing_zeros() as usize);
        x &= x - 1;
    }
    v
}

/// Critical region of `a` in the condensed program, `None` when a row is absent from the processed
/// program or the region is empty (or lower dimensional when `check_full_dim`).
fn region_of(maps: &RowMap, program: &MpProgram, a: Set, check_full_dim: bool) -> Option<CriticalRegion> {
    let mut active: Vec<usize> = (0..program.num_equality_constraints()).collect();
    for r in bits(a) {
        active.push(maps.to_processed(r)?);
    }
    active.sort_unstable();
    gen_cr_from_active_set(program, &active, check_full_dim)
}

/// DP constraint row -> row of the processed program. The processed program drops redundant rows
/// and rescales the others, so rows are matched up to a positive factor.
struct RowMap {
    map: Vec<Option<usize>>,
}

impl RowMap {
    fn new(mpc: &MpcProblem, program: &MpProgram, q: usize) -> Self {
        let orig = mpc.original.as_ref().expect("MpcProblem.original (problem rows) is required to map active sets");
        assert_eq!(mpc.row_map.len(), q);
        let key = |a: &Mat, f: &Mat, b: &[f64], i: usize| -> Vec<f64> {
            let nrm = crate::linalg::norm(a.row(i));
            a.row(i).iter().chain(f.row(i).iter()).map(|v| v / nrm).chain(std::iter::once(b[i] / nrm)).collect()
        };
        let neq = program.num_equality_constraints();
        let keys_p: Vec<Vec<f64>> = (0..program.a.rows).map(|i| key(&program.a, &program.f, &program.b, i)).collect();
        let map = mpc
            .row_map
            .iter()
            .map(|&o| {
                if o < 0 {
                    return None;
                }
                let ko = key(&orig.a, &orig.f, &orig.b, o as usize);
                (neq..program.a.rows).find(|&i| keys_p[i].iter().zip(&ko).all(|(x, y)| (x - y).abs() <= 1e-7 * (1.0 + y.abs())))
            })
            .collect();
        RowMap { map }
    }

    fn to_processed(&self, r: usize) -> Option<usize> {
        self.map[r]
    }
}

// ------------------------------------------------------------------------------ symmetry

/// The pair (Theta, Omega) = (-I, -I) is a symmetry of every problem whose constraint polytopes are
/// point symmetric about the origin: row i then has a partner j with `G_i = -G_j`, `E_i = -E_j`,
/// `w_i = w_j` (Theorem 2). Returns the permutation i -> j, or `None` when some row has no partner
/// (or the cost has no such symmetry, which cannot happen: H, F are invariant under u, x -> -u, -x).
fn central_symmetry(qp: &HorizonQp) -> Option<Vec<usize>> {
    let q = qp.num_rows();
    let mut perm = vec![usize::MAX; q];
    let close = |x: &[f64], y: &[f64]| x.iter().zip(y).all(|(a, b)| (a + b).abs() <= 1e-9 * (1.0 + a.abs()));
    for i in 0..q {
        for j in 0..q {
            if close(qp.g.row(i), qp.g.row(j)) && close(qp.e.row(i), qp.e.row(j)) && (qp.w[i] - qp.w[j]).abs() <= 1e-9 * (1.0 + qp.w[i].abs()) {
                perm[i] = j;
                break;
            }
        }
        if perm[i] == usize::MAX {
            return None;
        }
    }
    let mut hit = vec![false; q];
    for &j in &perm {
        if hit[j] {
            return None;
        }
        hit[j] = true;
    }
    Some(perm)
}

fn apply_perm(a: Set, perm: &[usize]) -> Set {
    let mut out = 0;
    let mut x = a;
    while x != 0 {
        out |= 1u128 << perm[x.trailing_zeros() as usize];
        x &= x - 1;
    }
    out
}

/// Orbit of `a` under the group generated by the involution `perm` (including `a` first).
fn orbit_of(a: Set, perm: &[usize]) -> Vec<Set> {
    let b = apply_perm(a, perm);
    if b == a { vec![a] } else { vec![a, b] }
}

// ------------------------------------------------------------------------------ LP tests

struct Horizon<'a> {
    qp: &'a HorizonQp,
    perm: Option<&'a [usize]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Outcome {
    /// (5) has a solution; `true` when t* = 0 (the polytope may be lower dimensional)
    Optimal(bool),
    /// (5) without (5b), (5c) has no solution: every superset is not optimal either
    Infeasible,
    Other,
}

const T_TOL: f64 = 1e-7;

impl Horizon<'_> {
    /// The LP (5) (`full`) or (5) without (5b) and (5c) for the candidate `a`: maximize t with
    /// t <= 1 (it is unbounded in degenerate cases otherwise). Variables (U, x0, lambda_A, t).
    fn lp(&self, a: Set, full: bool) -> Option<f64> {
        let qp = self.qp;
        let (nm, nx, q) = (qp.nm, qp.nx, qp.num_rows());
        let act = bits(a);
        let na = act.len();
        let nl = if full { na } else { 0 };
        let nz = nm + nx + nl + 1;
        let n_eq = na + if full { nm } else { 0 };
        let n_in = (q - na) + nl + 2;
        let mut m = Mat::zeros(n_eq + n_in, nz);
        let mut b = vec![0.0; n_eq + n_in];
        let mut r = 0;
        if full {
            for p in 0..nm {
                let row = m.row_mut(r);
                row[..nm].copy_from_slice(qp.h.row(p));
                row[nm..nm + nx].copy_from_slice(qp.fm.row(p));
                for (k, &i) in act.iter().enumerate() {
                    row[nm + nx + k] = qp.g[(i, p)];
                }
                r += 1;
            }
        }
        for &i in &act {
            let row = m.row_mut(r);
            row[..nm].copy_from_slice(qp.g.row(i));
            for c in 0..nx {
                row[nm + c] = -qp.e[(i, c)];
            }
            b[r] = qp.w[i];
            r += 1;
        }
        for i in 0..q {
            if a >> i & 1 == 1 {
                continue;
            }
            let row = m.row_mut(r);
            row[..nm].copy_from_slice(qp.g.row(i));
            for c in 0..nx {
                row[nm + c] = -qp.e[(i, c)];
            }
            row[nz - 1] = 1.0;
            b[r] = qp.w[i];
            r += 1;
        }
        for k in 0..nl {
            let row = m.row_mut(r);
            row[nm + nx + k] = -1.0;
            row[nz - 1] = 1.0;
            r += 1;
        }
        m[(r, nz - 1)] = -1.0;
        r += 1;
        m[(r, nz - 1)] = 1.0;
        b[r] = 1.0;
        let mut c = vec![0.0; nz];
        c[nz - 1] = -1.0;
        let eq: Vec<usize> = (0..n_eq).collect();
        solve_lp(Some(&c), &m, &b, &eq).map(|s| s.x[nz - 1])
    }

    fn classify(&self, a: Set, stats: &mut DpStats) -> Outcome {
        stats.optimality_lps += 1;
        if let Some(t) = self.lp(a, true) {
            return Outcome::Optimal(t <= T_TOL);
        }
        stats.feasibility_lps += 1;
        if self.lp(a, false).is_none() { Outcome::Infeasible } else { Outcome::Other }
    }
}

// ------------------------------------------------------------------------------ enumeration

/// All subsets of {0..q0-1} by increasing cardinality (lexicographic within a cardinality).
fn subsets_by_cardinality(q0: usize) -> Vec<Set> {
    let mut v: Vec<Set> = (0..1u128 << q0).collect();
    v.sort_by_key(|&s| (s.count_ones(), lex_key(s)));
    v
}

/// Order of the combinatorial tree (Fig. 1 of the 2023 paper): smallest index first.
fn lex_key(s: Set) -> Set {
    !s.reverse_bits()
}

fn contains_pruned(pruned: &[Set], a: Set) -> bool {
    pruned.iter().any(|&p| a & p == p)
}

/// S_1 and S_1^degen: every subset of the horizon-1 constraints (cardinality at most `cmax`) that is
/// optimal. With a symmetry only the primary active set of each orbit is tested and kept.
fn initial_sets(qp: &HorizonQp, cmax: usize, perm: Option<&[usize]>, stats: &mut DpStats) -> Vec<(Set, bool)> {
    let q = qp.num_rows();
    let ctx = Horizon { qp, perm };
    let mut all: Vec<Set> = (0..1u128 << q).filter(|s| (s.count_ones() as usize) <= cmax).collect();
    all.sort_by_key(|&s| (s.count_ones(), lex_key(s)));
    let mut out = vec![];
    let mut pruned: Vec<Set> = vec![];
    let mut nonprim: Vec<Set> = vec![];
    for a in all {
        stats.candidates += 1;
        if perm.is_some() && in_descendants(&nonprim, a) {
            continue;
        }
        if contains_pruned(&pruned, a) {
            stats.pruned += 1;
        } else {
            match ctx.classify(a, stats) {
                Outcome::Optimal(d) => out.push((a, d)),
                Outcome::Infeasible => {
                    pruned.push(a);
                    if let Some(p) = perm {
                        pruned.push(apply_perm(a, p));
                    }
                }
                Outcome::Other => {}
            }
        }
        if let Some(p) = perm {
            let img = apply_perm(a, p);
            if img != a && !in_descendants(&nonprim, img) {
                nonprim.push(img);
            }
        }
    }
    out
}

/// `a` is `a_tilde` or a descendant of it in the combinatorial tree: a superset whose extra
/// indices all exceed the largest index of `a_tilde`.
fn in_descendants(nonprim: &[Set], a: Set) -> bool {
    nonprim.iter().any(|&t| {
        if a & t != t {
            return false;
        }
        let top = if t == 0 { 0 } else { 128 - t.leading_zeros() as usize };
        (a & !t) & mask(top) == 0
    })
}

/// S_{n+1} and its degenerate subset from S_n (Alg. 2 of the 2020 paper; with the cardinality limit
/// of the 2022 paper). `ctx` is the horizon n+1 problem.
fn extend(ctx: &Horizon, prev: &HorizonQp, s_n: &[(Set, bool)], subsets: &[Set], cmax: usize, opts: &DpOptions, stats: &mut DpStats) -> Vec<(Set, bool)> {
    let q0 = prev.q0;
    let n = prev.n;
    let mut out: Vec<(Set, bool)> = vec![];
    let mut pruned: Vec<Set> = vec![];
    let mut cands: Vec<Set> = vec![];
    let test = |a: Set, out: &mut Vec<(Set, bool)>, pruned: &mut Vec<Set>, stats: &mut DpStats| {
        stats.candidates += 1;
        if contains_pruned(pruned, a) {
            stats.pruned += 1;
            return;
        }
        match ctx.classify(a, stats) {
            Outcome::Optimal(d) => out.push((a, d)),
            Outcome::Infeasible => {
                pruned.push(a);
                if let Some(p) = ctx.perm {
                    pruned.push(apply_perm(a, p));
                }
            }
            Outcome::Other => {}
        }
    };
    for &(al, degen) in s_n {
        let card_l = al.count_ones() as usize;
        if opts.terminal_invariant {
            // (10a): the sets without terminal constraints are optimal for the longer horizon too
            if al & !mask(n * q0) == 0 {
                out.push((al, degen));
            }
            // (10b): only sets with a constraint in stage N-1 or at the terminal set are extended
            if al & !mask((n - 1) * q0) == 0 {
                continue;
            }
        }
        if card_l > cmax {
            continue;
        }
        let room = cmax.saturating_sub(card_l);
        for &aj in subsets {
            if (aj.count_ones() as usize) > room {
                break;
            }
            let cand = aj | (al << q0);
            if opts.global_order {
                cands.push(cand);
            } else {
                test(cand, &mut out, &mut pruned, stats);
            }
        }
    }
    if opts.global_order {
        cands.sort_by_key(|&s| s.count_ones());
        for cand in cands {
            test(cand, &mut out, &mut pruned, stats);
        }
    }
    out
}
