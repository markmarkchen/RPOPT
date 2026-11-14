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
use crate::linalg::{is_full_rank_rows, Cholesky, Mat};
use crate::mpc::{HorizonQp, MpcProblem};
use crate::mpqp_utils::gen_cr_from_active_set;
use crate::program::MpProgram;
use crate::solution::Solution;
use crate::solver::lp::{lp_feasible, solve_lp};
use crate::fxhash::FxHashSet as HashSet;
use crate::fxhash::FxHashMap as HashMap;
use rayon::prelude::*;
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
    /// Solve the LPs (5) in reduced form when G_A has full row rank: lambda and U are eliminated
    /// through the KKT system (the optimality LP has the variables (x0, t) only), and the equalities
    /// of the feasibility LP are eliminated by Gauss-Jordan. Same answers as the full LPs, which are
    /// still used for rank-deficient candidates.
    pub reduced_lps: bool,
    /// Test the candidates of equal cardinality in parallel (rayon, global pool); implies
    /// `global_order`. Candidates of the same cardinality cannot prune each other, so the result
    /// and the counters are identical to the serial run with `global_order`.
    pub parallel: bool,
    /// Largest horizon to compute (defaults to the horizon of the MPC problem).
    pub horizon: Option<usize>,
}

impl DpOptions {
    /// Mitze and Moennigmann 2020, as published (Cor. 3 pruning needs `terminal_invariant`).
    pub fn paper2020() -> Self {
        DpOptions { cardinality_limit: false, symmetry: false, terminal_invariant: false, global_order: false, reduced_lps: true, parallel: false, horizon: None }
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
    /// time inside the optimality / feasibility LPs (only measured on the full-LP fallback path) and outcome counts (optimal, infeasible, other)
    pub seconds_opt_lp: f64,
    pub seconds_feas_lp: f64,
    pub outcomes: [usize; 3],
}

impl DpStats {
    fn add(&mut self, o: &DpStats) {
        self.candidates += o.candidates;
        self.pruned += o.pruned;
        self.optimality_lps += o.optimality_lps;
        self.feasibility_lps += o.feasibility_lps;
        self.seconds_opt_lp += o.seconds_opt_lp;
        self.seconds_feas_lp += o.seconds_feas_lp;
        for k in 0..3 {
            self.outcomes[k] += o.outcomes[k];
        }
    }
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
    let mut cur: Vec<(Set, bool)> = initial_sets(&qps[0], cmax, syms[0].as_deref(), opts, stats);
    stats.set_sizes.push(cur.len());
    stats.cumulative_seconds.push(t0.elapsed().as_secs_f64());
    let mut last = 1;
    for n in 1..nbar {
        let mut ctx = Horizon::new(&qps[n], syms[n].as_deref(), opts.reduced_lps);
        let pmap = ctx.build_parent_feas(&cur);
        ctx.pfeas = Some(&pmap);
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
    active.dedup(); // two DP rows can map to one processed row when duplicate rows were merged
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
    reduced: bool,
    /// `G H^-1 G'` (q x q) and `G H^-1 fm + E` (q x nx): everything the KKT elimination needs
    w_mat: Mat,
    cx: Mat,
    /// rows with G_i = 0 (bounds on x0 only): active, they are equalities in x0 and carry no multiplier
    zero_row: Vec<bool>,
    /// feasibility systems of the parents (horizon n) for the candidates of the horizon n+1
    pfeas: Option<&'a HashMap<Set, PFeas>>,
}

/// Feasibility system of the rows of a parent T (shifted by one stage), with the equalities of T
/// eliminated: variables v = (x0, U of the non-pivot columns); the rows not in T are
/// `mat v <= b`, a candidate A_j + T makes the rows of A_j equalities. `dep_eq` are rows (equalities)
/// that remain from dependent rows of T.
struct PFeas {
    mat: Mat,
    b: Vec<f64>,
    row_of: Vec<u32>,
    n_dep: usize,
}

/// Parameter substitution `x0 = xc + T y` from the equalities of the active rows with G_i = 0.
struct Subst {
    xc: Vec<f64>,
    t: Mat,
}

impl Subst {
    /// `[rows[:, :nx] T | rows[:, nx:]]` and `b - rows[:, :nx] xc` (the extra columns are kept).
    fn apply(&self, m: &Mat, b: &mut [f64], nx: usize) -> Mat {
        let nf = self.t.cols;
        let extra = m.cols - nx;
        let mut out = Mat::zeros(m.rows, nf + extra);
        for r in 0..m.rows {
            let row = m.row(r);
            let o = out.row_mut(r);
            let mut sh = 0.0;
            for c in 0..nx {
                let v = row[c];
                if v == 0.0 {
                    continue;
                }
                sh += v * self.xc[c];
                let tr = self.t.row(c);
                for f in 0..nf {
                    o[f] += v * tr[f];
                }
            }
            o[nf..].copy_from_slice(&row[nx..]);
            b[r] -= sh;
        }
        out
    }
}

/// Result of the reduced optimality test.
enum Red {
    Optimal(f64),
    /// not optimal, but primal feasible (certificate found)
    Feasible,
    /// not optimal; feasibility undecided
    Unknown,
}

/// Rows of the active set that are linear combinations of earlier ones (G_d = sum_k C_dk G_k over the
/// independent rows k): their multipliers are free (only `mu = lambda_B + C' lambda_D` is determined)
/// and each gives an equality in x0.
struct Dep {
    rows: Vec<usize>,
    /// |D| x |B|
    c: Mat,
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

    fn new<'a>(qp: &'a HorizonQp, perm: Option<&'a [usize]>, reduced: bool) -> Horizon<'a> {
        let (w_mat, cx) = if reduced {
            let chol = Cholesky::new(&qp.h).expect("H must be positive definite");
            let hinv_gt = chol.solve_mat(&qp.g.transpose()); // H^-1 G'  (nm x q)
            let hinv_fm = chol.solve_mat(&qp.fm); // H^-1 fm  (nm x nx)
            (qp.g.matmul(&hinv_gt), qp.g.matmul(&hinv_fm).add(&qp.e))
        } else {
            (Mat::zeros(0, 0), Mat::zeros(0, 0))
        };
        let zero_row = (0..qp.num_rows()).map(|i| qp.g.row(i).iter().all(|&v| v == 0.0)).collect();
        Horizon { qp, perm, reduced, w_mat, cx, zero_row, pfeas: None }
    }

    /// Solves the equalities `aug[:, :nx] x0 = aug[:, nx]` (Gauss-Jordan, complete pivoting). `None`
    /// when they contradict each other.
    fn eliminate_params(&self, mut aug: Mat) -> Option<Subst> {
        let nx = self.qp.nx;
        let kz = aug.rows;
        let scale = aug.max_abs().max(1.0);
        let mut used = vec![false; nx];
        let mut piv: Vec<usize> = Vec::with_capacity(nx);
        let mut rank = 0;
        for r in 0..kz {
            let (mut br, mut bc, mut bv) = (r, usize::MAX, 0.0f64);
            for rr in r..kz {
                for c in 0..nx {
                    if !used[c] && aug[(rr, c)].abs() > bv {
                        bv = aug[(rr, c)].abs();
                        br = rr;
                        bc = c;
                    }
                }
            }
            if bc == usize::MAX || bv <= 1e-10 * scale {
                break;
            }
            if br != r {
                for c in 0..=nx {
                    let t = aug[(r, c)];
                    aug[(r, c)] = aug[(br, c)];
                    aug[(br, c)] = t;
                }
            }
            let pv = aug[(r, bc)];
            for c in 0..=nx {
                aug[(r, c)] /= pv;
            }
            for rr in 0..kz {
                if rr != r {
                    let f = aug[(rr, bc)];
                    if f != 0.0 {
                        for c in 0..=nx {
                            let t = aug[(r, c)];
                            aug[(rr, c)] -= f * t;
                        }
                    }
                }
            }
            used[bc] = true;
            piv.push(bc);
            rank += 1;
        }
        for rr in rank..kz {
            if aug[(rr, nx)].abs() > 1e-9 * scale {
                return None;
            }
        }
        let free: Vec<usize> = (0..nx).filter(|&c| !used[c]).collect();
        let mut t = Mat::zeros(nx, free.len());
        let mut xc = vec![0.0; nx];
        for (k, &pc) in piv.iter().enumerate() {
            xc[pc] = aug[(k, nx)];
            for (f, &fc) in free.iter().enumerate() {
                t[(pc, f)] = -aug[(k, fc)];
            }
        }
        for (f, &fc) in free.iter().enumerate() {
            t[(fc, f)] = 1.0;
        }
        Some(Subst { xc, t })
    }

    /// Optimality LP (5) for a candidate whose G_A has full row rank, in the variables (x0, t):
    /// with `S = G_A H^-1 G_A'` the multipliers are `lambda = -S^-1 (Cx_A x0 + w_A)`, the primal
    /// solution `U = -H^-1 (fm x0 + G_A' lambda)`, and the slack of an inactive row i is
    /// `-(Cx_i x0 + W_iA lambda + w_i)`. `None` when S is not positive definite.
    fn lp_full_reduced(&self, a: Set, act: &[usize], dep: &Dep, sub: Option<&Subst>) -> Option<Red> {
        let qp = self.qp;
        let (nx, q, na) = (qp.nx, qp.num_rows(), act.len());
        let mut s = Mat::zeros(na, na);
        for (r, &i) in act.iter().enumerate() {
            for (c, &j) in act.iter().enumerate() {
                s[(r, c)] = self.w_mat[(i, j)];
            }
        }
        // right hand sides [Cx_A | w_A]
        let mut rhs = Mat::zeros(na, nx + 1);
        for (r, &i) in act.iter().enumerate() {
            rhs.row_mut(r)[..nx].copy_from_slice(self.cx.row(i));
            rhs[(r, nx)] = qp.w[i];
        }
        let chol = Cholesky::new(&s)?;
        let sol = chol.solve_mat(&rhs).neg(); // [Lx | l0]
        let nd = dep.rows.len();
        let n_in = q - a.count_ones() as usize + na + nd;
        let mut m = Mat::zeros(n_in, nx + nd);
        let mut b = vec![0.0; n_in];
        let mut r = 0;
        for i in 0..q {
            if a >> i & 1 == 1 {
                continue;
            }
            // Cx_i x0 + W_iA (Lx x0 + l0) + w_i >= t
            let row = m.row_mut(r);
            let mut c0 = qp.w[i];
            for c in 0..nx {
                row[c] = -self.cx[(i, c)];
            }
            for (k, &j) in act.iter().enumerate() {
                let wij = self.w_mat[(i, j)];
                for c in 0..nx {
                    row[c] -= wij * sol[(k, c)];
                }
                c0 += wij * sol[(k, nx)];
            }
            b[r] = c0;
            r += 1;
        }
        for k in 0..na {
            // lambda_k = mu_k - sum_d C_dk lambda_d >= t
            let row = m.row_mut(r);
            for c in 0..nx {
                row[c] = -sol[(k, c)];
            }
            for d in 0..nd {
                row[nx + d] = dep.c[(d, k)];
            }
            b[r] = sol[(k, nx)];
            r += 1;
        }
        for d in 0..nd {
            // lambda_d >= t
            m[(r, nx + d)] = -1.0;
            r += 1;
        }
        let m = match sub {
            Some(sb) => sb.apply(&m, &mut b, nx),
            None => m,
        };
        // t* >= 0 iff the rows hold at t = 0; t* > T_TOL iff they hold at t = T_TOL (a feasibility LP
        // in x0 only, which has a fast path for up to four parameters)
        let feas = |b: &[f64]| if m.cols == 0 { b.iter().all(|&v| v >= -1e-9) } else { lp_feasible(&m, b, &[]) };
        if !feas(&b) {
            // not optimal; the inactive rows alone (the KKT solution U(x0) of the active rows is
            // primal feasible) are a certificate of feasibility
            let n_slack = q - a.count_ones() as usize;
            let nf = m.cols - nd;
            let mut m2 = Mat::zeros(n_slack, nf);
            for r in 0..n_slack {
                m2.row_mut(r).copy_from_slice(&m.row(r)[..nf]);
            }
            let ok = if nf == 0 { b[..n_slack].iter().all(|&v| v >= -1e-9) } else { lp_feasible(&m2, &b[..n_slack], &[]) };
            return Some(if ok { Red::Feasible } else { Red::Unknown });
        }
        for v in b.iter_mut() {
            *v -= T_TOL;
        }
        Some(Red::Optimal(if feas(&b) { 1.0 } else { 0.0 }))
    }

    /// Feasibility LP (5 without 5b, 5c), equalities eliminated by Gauss-Jordan on `[G_A | -E_A | w_A]`
    /// (complete pivoting on the G_A columns); variables (x0, U of the non-pivot columns).
    /// `None` when G_A is rank deficient. t is dropped: a solution with t >= 0 exists iff t = 0 is feasible.
    fn feasible_reduced(&self, a: Set, act: &[usize], sub: Option<&Subst>) -> Option<bool> {
        let qp = self.qp;
        let (nm, nx, q, na) = (qp.nm, qp.nx, qp.num_rows(), act.len());
        let width = nm + nx + 1;
        let mut aug = Mat::zeros(na, width);
        for (r, &i) in act.iter().enumerate() {
            let row = aug.row_mut(r);
            row[..nm].copy_from_slice(qp.g.row(i));
            for c in 0..nx {
                row[nm + c] = -qp.e[(i, c)];
            }
            row[nm + nx] = qp.w[i];
        }
        let scale = aug.max_abs().max(1.0);
        let mut pivot_cols = Vec::with_capacity(na);
        let mut used = vec![false; nm];
        for r in 0..na {
            let (mut br, mut bc, mut bv) = (r, usize::MAX, 0.0f64);
            for rr in r..na {
                for c in 0..nm {
                    if !used[c] && aug[(rr, c)].abs() > bv {
                        bv = aug[(rr, c)].abs();
                        br = rr;
                        bc = c;
                    }
                }
            }
            if bc == usize::MAX || bv <= 1e-10 * scale {
                return None;
            }
            if br != r {
                for c in 0..width {
                    let t = aug[(r, c)];
                    aug[(r, c)] = aug[(br, c)];
                    aug[(br, c)] = t;
                }
            }
            let pv = aug[(r, bc)];
            for c in 0..width {
                aug[(r, c)] /= pv;
            }
            for rr in 0..na {
                if rr != r {
                    let f = aug[(rr, bc)];
                    if f != 0.0 {
                        for c in 0..width {
                            let t = aug[(r, c)];
                            aug[(rr, c)] -= f * t;
                        }
                    }
                }
            }
            used[bc] = true;
            pivot_cols.push(bc);
        }
        let rest: Vec<usize> = (0..nm).filter(|&c| !used[c]).collect();
        let nz = nx + rest.len();
        let n_in = q - a.count_ones() as usize;
        let mut m = Mat::zeros(n_in, nz);
        let mut b = vec![0.0; n_in];
        let mut r = 0;
        for i in 0..q {
            if a >> i & 1 == 1 {
                continue;
            }
            let gi = qp.g.row(i);
            let row = m.row_mut(r);
            let mut rhs = qp.w[i];
            for c in 0..nx {
                row[c] = -qp.e[(i, c)];
            }
            for (k, &jc) in pivot_cols.iter().enumerate() {
                let g = gi[jc];
                if g == 0.0 {
                    continue;
                }
                for c in 0..nx {
                    row[c] -= g * aug[(k, nm + c)];
                }
                rhs -= g * aug[(k, nm + nx)];
                for (u, &rc) in rest.iter().enumerate() {
                    row[nx + u] -= g * aug[(k, rc)];
                }
            }
            for (u, &rc) in rest.iter().enumerate() {
                row[nx + u] += gi[rc];
            }
            b[r] = rhs;
            r += 1;
        }
        let m = match sub {
            Some(sb) => sb.apply(&m, &mut b, nx),
            None => m,
        };
        Some(if m.cols == 0 { b.iter().all(|&v| v >= -1e-9) } else { lp_feasible(&m, &b, &[]) })
    }

    /// Splits the rows `act_r` (G_i != 0) into independent rows B (greedy in index order, by the
    /// Gram matrix `W = G H^-1 G'`) and dependent rows D with G_d = sum_k C_dk G_k.
    fn split_dependent(&self, act_r: &[usize]) -> (Vec<usize>, Dep) {
        let n = act_r.len();
        let mut b_idx: Vec<usize> = Vec::with_capacity(n);
        let mut d_idx: Vec<usize> = vec![];
        // Cholesky factor of W_BB (lower, row-major, grows with B)
        let mut l: Vec<Vec<f64>> = vec![];
        let mut c_rows: Vec<Vec<f64>> = vec![];
        for &j in act_r {
            let k = b_idx.len();
            // forward solve L z = W_Bj
            let mut z = vec![0.0; k];
            for r in 0..k {
                let mut v = self.w_mat[(b_idx[r], j)];
                for c in 0..r {
                    v -= l[r][c] * z[c];
                }
                z[r] = v / l[r][r];
            }
            let wjj = self.w_mat[(j, j)];
            let res = wjj - z.iter().map(|v| v * v).sum::<f64>();
            if res > 1e-9 * wjj.abs().max(1e-300) {
                let mut row = z;
                row.push(res.sqrt());
                l.push(row);
                b_idx.push(j);
            } else {
                // C_d = W_BB^-1 W_Bd: back solve L' c = z
                let mut c = z.clone();
                for r in (0..k).rev() {
                    let mut v = c[r];
                    for rr in r + 1..k {
                        v -= l[rr][r] * c[rr];
                    }
                    c[r] = v / l[r][r];
                }
                d_idx.push(j);
                c_rows.push(c);
            }
        }
        let nb = b_idx.len();
        let mut c = Mat::zeros(d_idx.len(), nb);
        for (d, row) in c_rows.iter().enumerate() {
            // dependents found before later B members were added have shorter rows
            c.row_mut(d)[..row.len()].copy_from_slice(row);
        }
        (b_idx, Dep { rows: d_idx, c })
    }

    fn classify(&self, a: Set, stats: &mut DpStats) -> Outcome {
        if self.reduced {
            if let Some(o) = SCRATCH.with(|c| self.classify_fast(a, &mut c.borrow_mut(), stats)) {
                return o;
            }
        }
        let qp = self.qp;
        let nx = qp.nx;
        let act = bits(a);
        let mut act_r: Vec<usize> = Vec::with_capacity(act.len());
        let mut act_z: Vec<usize> = Vec::new();
        for &i in &act {
            if self.zero_row[i] { act_z.push(i) } else { act_r.push(i) }
        }
        let reduced = self.reduced;
        let (act_b, dep) = if reduced && !is_full_rank_rows(&qp.g, &act_r) {
            self.split_dependent(&act_r)
        } else {
            (act_r, Dep { rows: vec![], c: Mat::zeros(0, 0) })
        };
        let sub = if reduced && (!act_z.is_empty() || !dep.rows.is_empty()) {
            let mut aug = Mat::zeros(act_z.len() + dep.rows.len(), nx + 1);
            for (r, &i) in act_z.iter().enumerate() {
                let row = aug.row_mut(r);
                for c in 0..nx {
                    row[c] = -qp.e[(i, c)];
                }
                row[nx] = qp.w[i];
            }
            for (d, &i) in dep.rows.iter().enumerate() {
                let row = aug.row_mut(act_z.len() + d);
                let mut rhs = qp.w[i];
                for c in 0..nx {
                    row[c] = -qp.e[(i, c)];
                }
                for (k, &j) in act_b.iter().enumerate() {
                    let ck = dep.c[(d, k)];
                    if ck == 0.0 {
                        continue;
                    }
                    for c in 0..nx {
                        row[c] += ck * qp.e[(j, c)];
                    }
                    rhs -= ck * qp.w[j];
                }
                row[nx] = rhs;
            }
            match self.eliminate_params(aug) {
                Some(sb) => Some(sb),
                None => {
                    // contradicting equalities on x0: infeasible
                    stats.optimality_lps += 1;
                    stats.feasibility_lps += 1;
                    stats.outcomes[1] += 1;
                    return Outcome::Infeasible;
                }
            }
        } else {
            None
        };
        stats.optimality_lps += 1;
        let t0 = Instant::now();
        let red = if reduced { self.lp_full_reduced(a, &act_b, &dep, sub.as_ref()) } else { None };
        let red = match red {
            Some(r) => r,
            None => match self.lp(a, true) {
                Some(t) => Red::Optimal(t),
                None => Red::Unknown,
            },
        };
        stats.seconds_opt_lp += t0.elapsed().as_secs_f64();
        if let Red::Optimal(t) = red {
            stats.outcomes[0] += 1;
            return Outcome::Optimal(t <= T_TOL);
        }
        if let Red::Feasible = red {
            stats.outcomes[2] += 1;
            return Outcome::Other;
        }
        stats.feasibility_lps += 1;
        let t0 = Instant::now();
        let feas = if reduced { self.feasible_reduced(a, &act_b, sub.as_ref()) } else { None };
        let feasible = match feas {
            Some(f) => f,
            None => self.lp(a, false).is_some(),
        };
        stats.seconds_feas_lp += t0.elapsed().as_secs_f64();
        if !feasible {
            stats.outcomes[1] += 1;
            Outcome::Infeasible
        } else {
            stats.outcomes[2] += 1;
            Outcome::Other
        }
    }
}

// ------------------------------------------------------------------------------ allocation-free tests

/// Per-thread buffers of `Horizon::classify_fast`.
#[derive(Default)]
struct Scratch {
    act_r: Vec<usize>,
    act_z: Vec<usize>,
    act_b: Vec<usize>,
    dep_rows: Vec<usize>,
    /// |D| x |B| coefficients C (row major)
    dep_c: Vec<f64>,
    // split_dependent
    gl: Vec<f64>,
    gz: Vec<f64>,
    gcc: Vec<f64>,
    gck: Vec<usize>,
    // parameter equalities
    eq: Vec<f64>,
    eq_used: Vec<bool>,
    eq_piv: Vec<usize>,
    xc: Vec<f64>,
    tm: Vec<f64>,
    // multipliers
    chol: Vec<f64>,
    sol: Vec<f64>,
    rx: Vec<f64>,
    // LP data
    mat: Mat,
    mat2: Mat,
    b: Vec<f64>,
    // feasibility elimination
    aug: Vec<f64>,
    ug_used: Vec<bool>,
    ug_piv: Vec<usize>,
    ug_rest: Vec<usize>,
    rowx: Vec<f64>,
    rowu: Vec<f64>,
    eqi: Vec<usize>,
}

thread_local! {
    static SCRATCH: std::cell::RefCell<Scratch> = std::cell::RefCell::new(Scratch::default());
}

fn prep(m: &mut Mat, rows: usize, cols: usize) {
    m.rows = rows;
    m.cols = cols;
    m.data.clear();
    m.data.resize(rows * cols, 0.0);
}

/// Gauss-Jordan with complete pivoting on the first `npiv` columns of the `rows x width` matrix.
/// Rows `0..rank` end up with a unit pivot (in column `piv[k]`), the pivot column zero elsewhere.
/// Stops at the first pivot below `tol`.
fn gauss_jordan(aug: &mut [f64], rows: usize, width: usize, npiv: usize, used: &mut Vec<bool>, piv: &mut Vec<usize>, tol: f64) -> usize {
    used.clear();
    used.resize(npiv, false);
    piv.clear();
    for r in 0..rows {
        let (mut br, mut bc, mut bv) = (r, usize::MAX, 0.0f64);
        for rr in r..rows {
            let row = &aug[rr * width..rr * width + npiv];
            for c in 0..npiv {
                if !used[c] && row[c].abs() > bv {
                    bv = row[c].abs();
                    br = rr;
                    bc = c;
                }
            }
        }
        if bc == usize::MAX || bv <= tol {
            return r;
        }
        if br != r {
            for c in 0..width {
                aug.swap(r * width + c, br * width + c);
            }
        }
        let pv = aug[r * width + bc];
        for c in 0..width {
            aug[r * width + c] /= pv;
        }
        for rr in 0..rows {
            if rr != r {
                let f = aug[rr * width + bc];
                if f != 0.0 {
                    let (lo, hi) = if rr < r { let (a, b) = aug.split_at_mut(r * width); (&mut a[rr * width..(rr + 1) * width], &b[..width]) } else { let (a, b) = aug.split_at_mut(rr * width); (&mut b[..width], &a[r * width..(r + 1) * width]) };
                    for c in 0..width {
                        lo[c] -= f * hi[c];
                    }
                }
            }
        }
        used[bc] = true;
        piv.push(bc);
    }
    rows
}

/// Row `rx . x0 <= c0` in the variables y of `x0 = xc + T y` (`ident`: y = x0).
#[inline]
fn to_y(rx: &[f64], c0: f64, ident: bool, xc: &[f64], t: &[f64], nf: usize, out: &mut [f64], b: &mut f64) {
    if ident {
        out[..nf].copy_from_slice(rx);
        *b = c0;
        return;
    }
    let mut sh = 0.0;
    for (c, &v) in rx.iter().enumerate() {
        if v == 0.0 {
            continue;
        }
        sh += v * xc[c];
        let tr = &t[c * nf..(c + 1) * nf];
        for f in 0..nf {
            out[f] += v * tr[f];
        }
    }
    *b = c0 - sh;
}


type InactiveArgs<'a> = (&'a [f64], &'a [f64], &'a [f64], usize, Set, &'a [usize], &'a [f64], bool, &'a [f64], &'a [f64], usize, usize, &'a mut [f64], &'a mut [f64]);

/// The rows `Cx_i x0 + W_iB (Lx x0 + l0) + w_i >= 0` of the inactive i in the variables y (NX parameters).
/// Returns the number of rows written.
fn inactive_rows<const NX: usize>(args: InactiveArgs) -> usize {
    let (w_flat, cx_flat, w, q, a, act_b, sol, ident, xc, tm, nf, ncol, mat_data, bv) = args;
    let nc = NX + 1;
    let mut r = 0;
    for i in 0..q {
        if a >> i & 1 == 1 {
            continue;
        }
        let wrow = &w_flat[i * q..(i + 1) * q];
        let mut rx = [0.0f64; NX];
        for c in 0..NX {
            rx[c] = -cx_flat[i * NX + c];
        }
        let mut c0 = w[i];
        for (k, &j) in act_b.iter().enumerate() {
            let wij = wrow[j];
            let sk = &sol[k * nc..(k + 1) * nc];
            for c in 0..NX {
                rx[c] -= wij * sk[c];
            }
            c0 += wij * sk[NX];
        }
        let out = &mut mat_data[r * ncol..r * ncol + nf];
        if ident {
            out.copy_from_slice(&rx);
            bv[r] = c0;
        } else {
            let mut o = [0.0f64; NX];
            let mut sh = 0.0;
            for c in 0..NX {
                let v = rx[c];
                if v == 0.0 {
                    continue;
                }
                sh += v * xc[c];
                let tr = &tm[c * nf..(c + 1) * nf];
                for f in 0..nf {
                    o[f] += v * tr[f];
                }
            }
            out.copy_from_slice(&o[..nf]);
            bv[r] = c0 - sh;
        }
        r += 1;
    }
    r
}

impl Horizon<'_> {
    /// Same decisions as `classify` with the reduced LPs, on per-thread buffers. `None` when the
    /// reduced route does not apply (the caller uses the full LPs).
    fn classify_fast(&self, a: Set, sc: &mut Scratch, stats: &mut DpStats) -> Option<Outcome> {
        let qp = self.qp;
        let (nx, nm, q) = (qp.nx, qp.nm, qp.num_rows());
        let na_all = a.count_ones() as usize;
        sc.act_r.clear();
        sc.act_z.clear();
        {
            let mut x = a;
            while x != 0 {
                let i = x.trailing_zeros() as usize;
                x &= x - 1;
                if self.zero_row[i] { sc.act_z.push(i) } else { sc.act_r.push(i) }
            }
        }
        sc.dep_rows.clear();
        sc.dep_c.clear();
        self.split_dependent_fast(sc);
        let nb = sc.act_b.len();
        let nd = sc.dep_rows.len();
        // equalities on x0: rows with G_i = 0 and the dependent rows
        let neq = sc.act_z.len() + nd;
        let ident = neq == 0;
        let mut nf = nx;
        if !ident {
            let w = nx + 1;
            sc.eq.clear();
            sc.eq.resize(neq * w, 0.0);
            for (r, &i) in sc.act_z.iter().enumerate() {
                let row = &mut sc.eq[r * w..(r + 1) * w];
                for c in 0..nx {
                    row[c] = -qp.e[(i, c)];
                }
                row[nx] = qp.w[i];
            }
            for (d, &i) in sc.dep_rows.iter().enumerate() {
                let r = sc.act_z.len() + d;
                let row = &mut sc.eq[r * w..(r + 1) * w];
                let mut rhs = qp.w[i];
                for c in 0..nx {
                    row[c] = -qp.e[(i, c)];
                }
                for (k, &j) in sc.act_b.iter().enumerate() {
                    let ck = sc.dep_c[d * nb + k];
                    if ck == 0.0 {
                        continue;
                    }
                    for c in 0..nx {
                        row[c] += ck * qp.e[(j, c)];
                    }
                    rhs -= ck * qp.w[j];
                }
                row[nx] = rhs;
            }
            let mut scale = 0.0f64;
            for v in &sc.eq {
                scale = scale.max(v.abs());
            }
            let scale = scale.max(1.0);
            let rank = gauss_jordan(&mut sc.eq, neq, w, nx, &mut sc.eq_used, &mut sc.eq_piv, 1e-10 * scale);
            for rr in rank..neq {
                if sc.eq[rr * w + nx].abs() > 1e-9 * scale {
                    stats.optimality_lps += 1;
                    stats.feasibility_lps += 1;
                    stats.outcomes[1] += 1;
                    return Some(Outcome::Infeasible);
                }
            }
            nf = nx - rank;
            sc.xc.clear();
            sc.xc.resize(nx, 0.0);
            sc.tm.clear();
            sc.tm.resize(nx * nf, 0.0);
            let mut f = 0;
            for c in 0..nx {
                if !sc.eq_used[c] {
                    sc.tm[c * nf + f] = 1.0;
                    f += 1;
                }
            }
            for k in 0..rank {
                let pc = sc.eq_piv[k];
                sc.xc[pc] = sc.eq[k * w + nx];
                let mut f = 0;
                for c in 0..nx {
                    if !sc.eq_used[c] {
                        sc.tm[pc * nf + f] = -sc.eq[k * w + c];
                        f += 1;
                    }
                }
            }
        }
        stats.optimality_lps += 1;
        // ---- optimality (and the primal certificate)
        // multipliers mu = -S^-1 (Cx_B x0 + w_B), S = W_BB
        // the Cholesky factor of W_BB came with the split into independent and dependent rows
        let cs = sc.act_r.len();
        std::mem::swap(&mut sc.chol, &mut sc.gl);
        let nc = nx + 1;
        sc.sol.clear();
        sc.sol.resize(nb * nc, 0.0);
        for (r, &i) in sc.act_b.iter().enumerate() {
            sc.sol[r * nc..r * nc + nx].copy_from_slice(self.cx.row(i));
            sc.sol[r * nc + nx] = qp.w[i];
        }
        for i in 0..nb {
            for k in 0..i {
                let l = sc.chol[i * cs + k];
                if l != 0.0 {
                    let (top, bot) = sc.sol.split_at_mut(i * nc);
                    for (d, s) in bot[..nc].iter_mut().zip(&top[k * nc..(k + 1) * nc]) {
                        *d -= l * s;
                    }
                }
            }
            let inv = 1.0 / sc.chol[i * cs + i];
            for v in &mut sc.sol[i * nc..(i + 1) * nc] {
                *v *= inv;
            }
        }
        for i in (0..nb).rev() {
            for k in i + 1..nb {
                let l = sc.chol[k * cs + i];
                if l != 0.0 {
                    let (top, bot) = sc.sol.split_at_mut(k * nc);
                    for (d, s) in top[i * nc..(i + 1) * nc].iter_mut().zip(&bot[..nc]) {
                        *d -= l * s;
                    }
                }
            }
            let inv = 1.0 / sc.chol[i * cs + i];
            for v in &mut sc.sol[i * nc..(i + 1) * nc] {
                *v *= inv;
            }
        }
        for v in sc.sol.iter_mut() {
            *v = -*v;
        }
        // rows in the variables (y, lambda_D)
        let n_slack = q - na_all;
        let n_in = n_slack + nb + nd;
        let ncol = nf + nd;
        prep(&mut sc.mat, n_in, ncol);
        sc.b.clear();
        sc.b.resize(n_in, 0.0);
        sc.rx.clear();
        sc.rx.resize(nx, 0.0);
        let mut r = {
            let (mat_data, bv) = (&mut sc.mat.data[..], &mut sc.b[..]);
            let args = (&self.w_mat.data[..], &self.cx.data[..], &qp.w[..], q, a, &sc.act_b[..], &sc.sol[..], ident, &sc.xc[..], &sc.tm[..], nf, ncol, mat_data, bv);
            match nx {
                2 => inactive_rows::<2>(args),
                3 => inactive_rows::<3>(args),
                4 => inactive_rows::<4>(args),
                5 => inactive_rows::<5>(args),
                6 => inactive_rows::<6>(args),
                _ => return None,
            }
        };
        for k in 0..nb {
            // lambda_k = mu_k - sum_d C_dk lambda_d >= 0
            for c in 0..nx {
                sc.rx[c] = -sc.sol[k * nc + c];
            }
            let mut bb = 0.0;
            to_y(&sc.rx, sc.sol[k * nc + nx], ident, &sc.xc, &sc.tm, nf, &mut sc.mat.data[r * ncol..r * ncol + nf], &mut bb);
            sc.b[r] = bb;
            for d in 0..nd {
                sc.mat.data[r * ncol + nf + d] = sc.dep_c[d * nb + k];
            }
            r += 1;
        }
        for d in 0..nd {
            sc.mat.data[r * ncol + nf + d] = -1.0;
            r += 1;
        }
        let feas = |m: &Mat, b: &[f64]| if m.cols == 0 { b.iter().all(|&v| v >= -1e-9) } else { lp_feasible(m, b, &[]) };
        if feas(&sc.mat, &sc.b) {
            for v in sc.b.iter_mut() {
                *v -= T_TOL;
            }
            let t_pos = feas(&sc.mat, &sc.b);
            stats.outcomes[0] += 1;
            return Some(Outcome::Optimal(!t_pos));
        }
        // not optimal: the inactive rows alone (the KKT point U(x0) of the active rows is primal
        // feasible there) certify feasibility
        let ok = if nd == 0 {
            sc.mat.rows = n_slack;
            let ok = feas(&sc.mat, &sc.b[..n_slack]);
            sc.mat.rows = n_in;
            ok
        } else {
            prep(&mut sc.mat2, n_slack, nf);
            for r in 0..n_slack {
                let (src, dst) = (&sc.mat.data[r * ncol..r * ncol + nf], &mut sc.mat2.data[r * nf..(r + 1) * nf]);
                dst.copy_from_slice(src);
            }
            feas(&sc.mat2, &sc.b[..n_slack])
        };
        if ok {
            stats.outcomes[2] += 1;
            return Some(Outcome::Other);
        }
        // ---- feasibility
        stats.feasibility_lps += 1;
        if let Some(pf) = self.pfeas.and_then(|m| m.get(&(a >> qp.q0))) {
            // rows of the stage-0 active set are equalities of the parent's system
            sc.eqi.clear();
            let mut x = a & mask(qp.q0);
            while x != 0 {
                sc.eqi.push(pf.row_of[x.trailing_zeros() as usize] as usize);
                x &= x - 1;
            }
            let nrows = pf.mat.rows;
            for k in 0..pf.n_dep {
                sc.eqi.push(nrows - pf.n_dep + k);
            }
            let feasible = lp_feasible(&pf.mat, &pf.b, &sc.eqi);
            return Some(if feasible {
                stats.outcomes[2] += 1;
                Outcome::Other
            } else {
                stats.outcomes[1] += 1;
                Outcome::Infeasible
            });
        }
        let width = nm + nx + 1;
        sc.aug.clear();
        sc.aug.resize(nb * width, 0.0);
        for (r, &i) in sc.act_b.iter().enumerate() {
            let row = &mut sc.aug[r * width..(r + 1) * width];
            row[..nm].copy_from_slice(qp.g.row(i));
            for c in 0..nx {
                row[nm + c] = -qp.e[(i, c)];
            }
            row[nm + nx] = qp.w[i];
        }
        let mut scale = 0.0f64;
        for v in &sc.aug {
            scale = scale.max(v.abs());
        }
        let scale = scale.max(1.0);
        let rank = gauss_jordan(&mut sc.aug, nb, width, nm, &mut sc.ug_used, &mut sc.ug_piv, 1e-10 * scale);
        if rank < nb {
            // numerically dependent after all: the full LP decides
            let feasible = self.lp(a, false).is_some();
            return Some(if feasible {
                stats.outcomes[2] += 1;
                Outcome::Other
            } else {
                stats.outcomes[1] += 1;
                Outcome::Infeasible
            });
        }
        sc.ug_rest.clear();
        for c in 0..nm {
            if !sc.ug_used[c] {
                sc.ug_rest.push(c);
            }
        }
        let nr = sc.ug_rest.len();
        let ncol2 = nf + nr;
        prep(&mut sc.mat, n_slack, ncol2);
        sc.b.clear();
        sc.b.resize(n_slack, 0.0);
        sc.rowx.clear();
        sc.rowx.resize(nx, 0.0);
        sc.rowu.clear();
        sc.rowu.resize(nr, 0.0);
        let mut r = 0;
        for i in 0..q {
            if a >> i & 1 == 1 {
                continue;
            }
            let gi = qp.g.row(i);
            let mut rhs = qp.w[i];
            for c in 0..nx {
                sc.rowx[c] = -qp.e[(i, c)];
            }
            for u in 0..nr {
                sc.rowu[u] = gi[sc.ug_rest[u]];
            }
            for k in 0..nb {
                let g = gi[sc.ug_piv[k]];
                if g == 0.0 {
                    continue;
                }
                let ak = &sc.aug[k * width..(k + 1) * width];
                for c in 0..nx {
                    sc.rowx[c] -= g * ak[nm + c];
                }
                rhs -= g * ak[nm + nx];
                for u in 0..nr {
                    sc.rowu[u] -= g * ak[sc.ug_rest[u]];
                }
            }
            let out = &mut sc.mat.data[r * ncol2..(r + 1) * ncol2];
            let mut bb = 0.0;
            to_y(&sc.rowx, rhs, ident, &sc.xc, &sc.tm, nf, out, &mut bb);
            sc.b[r] = bb;
            out[nf..].copy_from_slice(&sc.rowu);
            r += 1;
        }
        let feasible = feas(&sc.mat, &sc.b);
        Some(if feasible {
            stats.outcomes[2] += 1;
            Outcome::Other
        } else {
            stats.outcomes[1] += 1;
            Outcome::Infeasible
        })
    }

    /// Feasibility systems of all parents `cur` (their rows shifted by one stage).
    fn build_parent_feas(&self, cur: &[(Set, bool)]) -> HashMap<Set, PFeas> {
        let mut map: HashMap<Set, PFeas> = HashMap::default();
        if !self.reduced {
            return map;
        }
        let (qp, q0) = (self.qp, self.qp.q0);
        let (nx, nm, q) = (qp.nx, qp.nm, qp.num_rows());
        let width = nm + nx + 1;
        for &(al, _) in cur {
            if map.contains_key(&al) {
                continue;
            }
            let tmask = al << q0;
            let tr = bits(tmask);
            let nt = tr.len();
            let mut aug = vec![0.0; nt * width];
            for (r, &i) in tr.iter().enumerate() {
                let row = &mut aug[r * width..(r + 1) * width];
                row[..nm].copy_from_slice(qp.g.row(i));
                for c in 0..nx {
                    row[nm + c] = -qp.e[(i, c)];
                }
                row[nm + nx] = qp.w[i];
            }
            let scale = aug.iter().fold(1.0f64, |m, v| m.max(v.abs()));
            let (mut used, mut piv) = (vec![], vec![]);
            let rank = gauss_jordan(&mut aug, nt, width, nm, &mut used, &mut piv, 1e-10 * scale);
            let rest: Vec<usize> = (0..nm).filter(|&c| !used[c]).collect();
            let nr = rest.len();
            let n = nx + nr;
            let n_dep = nt - rank;
            let rows_total = (q - nt) + n_dep;
            let mut mat = Mat::zeros(rows_total, n);
            let mut b = vec![0.0; rows_total];
            let mut row_of = vec![u32::MAX; q];
            let mut r = 0;
            for i in 0..q {
                if tmask >> i & 1 == 1 {
                    continue;
                }
                row_of[i] = r as u32;
                let gi = qp.g.row(i);
                let out = mat.row_mut(r);
                let mut rhs = qp.w[i];
                for c in 0..nx {
                    out[c] = -qp.e[(i, c)];
                }
                for (u, &rc) in rest.iter().enumerate() {
                    out[nx + u] = gi[rc];
                }
                for k in 0..rank {
                    let g = gi[piv[k]];
                    if g == 0.0 {
                        continue;
                    }
                    let ak = &aug[k * width..(k + 1) * width];
                    for c in 0..nx {
                        out[c] -= g * ak[nm + c];
                    }
                    rhs -= g * ak[nm + nx];
                    for (u, &rc) in rest.iter().enumerate() {
                        out[nx + u] -= g * ak[rc];
                    }
                }
                b[r] = rhs;
                r += 1;
            }
            for rr in rank..nt {
                let ak = &aug[rr * width..(rr + 1) * width];
                let out = mat.row_mut(r);
                out[..nx].copy_from_slice(&ak[nm..nm + nx]);
                b[r] = ak[nm + nx];
                r += 1;
            }
            map.insert(al, PFeas { mat, b, row_of, n_dep });
        }
        map
    }

    /// Flat version of `split_dependent` (fills `act_b`, `dep_rows`, `dep_c` of the scratch).
    fn split_dependent_fast(&self, sc: &mut Scratch) {
        let n = sc.act_r.len();
        sc.act_b.clear();
        sc.dep_rows.clear();
        sc.gl.clear();
        sc.gl.resize(n * n, 0.0);
        sc.gz.clear();
        sc.gz.resize(n, 0.0);
        sc.gcc.clear();
        sc.gck.clear();
        for idx in 0..n {
            let j = sc.act_r[idx];
            let k = sc.act_b.len();
            for r in 0..k {
                let mut v = self.w_mat[(sc.act_b[r], j)];
                for c in 0..r {
                    v -= sc.gl[r * n + c] * sc.gz[c];
                }
                sc.gz[r] = v / sc.gl[r * n + r];
            }
            let wjj = self.w_mat[(j, j)];
            let res = wjj - sc.gz[..k].iter().map(|v| v * v).sum::<f64>();
            if res > 1e-9 * wjj.abs().max(1e-300) {
                for c in 0..k {
                    sc.gl[k * n + c] = sc.gz[c];
                }
                sc.gl[k * n + k] = res.sqrt();
                sc.act_b.push(j);
            } else {
                let base = sc.gcc.len();
                sc.gcc.extend_from_slice(&sc.gz[..k]);
                for r in (0..k).rev() {
                    let mut v = sc.gcc[base + r];
                    for rr in r + 1..k {
                        v -= sc.gl[rr * n + r] * sc.gcc[base + rr];
                    }
                    sc.gcc[base + r] = v / sc.gl[r * n + r];
                }
                sc.dep_rows.push(j);
                sc.gck.push(k);
            }
        }
        let nb = sc.act_b.len();
        sc.dep_c.resize(sc.dep_rows.len() * nb, 0.0);
        let mut base = 0;
        for d in 0..sc.dep_rows.len() {
            let k = sc.gck[d];
            for c in 0..k {
                sc.dep_c[d * nb + c] = sc.gcc[base + c];
            }
            base += k;
        }
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
fn initial_sets(qp: &HorizonQp, cmax: usize, perm: Option<&[usize]>, opts: &DpOptions, stats: &mut DpStats) -> Vec<(Set, bool)> {
    let q = qp.num_rows();
    let ctx = Horizon::new(qp, perm, opts.reduced_lps);
    let mut all: Vec<Set> = (0..1u128 << q).filter(|s| (s.count_ones() as usize) <= cmax).collect();
    all.sort_by_key(|&s| (s.count_ones(), lex_key(s)));
    let mut out = vec![];
    let mut pruned: Vec<Set> = vec![];
    if perm.is_none() {
        // no symmetry: the candidates of one cardinality are independent of each other
        run_levels(&ctx, &all, &mut pruned, &mut out, opts.parallel, stats);
        return out;
    }
    let mut nonprim: Vec<Set> = vec![];
    for a in all {
        stats.candidates += 1;
        if in_descendants(&nonprim, a) {
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

/// Tests `cands` (sorted by cardinality) level by level: dismisses the candidates containing a
/// known infeasible set, classifies the others (in parallel if asked) and merges the outcomes in
/// order, so the result does not depend on the thread count.
fn run_levels(ctx: &Horizon, cands: &[Set], pruned: &mut Vec<Set>, out: &mut Vec<(Set, bool)>, parallel: bool, stats: &mut DpStats) {
    let mut start = 0;
    while start < cands.len() {
        let card = cands[start].count_ones();
        let end = start + cands[start..].iter().take_while(|c| c.count_ones() == card).count();
        let level = &cands[start..end];
        if !(parallel && level.len() > 1) {
            // serial: candidates of one cardinality cannot prune each other, so the outcomes are
            // handled in place (no per-candidate result records)
            let mut new_pruned = vec![];
            for &a in level {
                stats.candidates += 1;
                if contains_pruned(pruned, a) {
                    stats.pruned += 1;
                    continue;
                }
                match ctx.classify(a, stats) {
                    Outcome::Optimal(d) => out.push((a, d)),
                    Outcome::Infeasible => {
                        new_pruned.push(a);
                        if let Some(p) = ctx.perm {
                            new_pruned.push(apply_perm(a, p));
                        }
                    }
                    Outcome::Other => {}
                }
            }
            pruned.extend(new_pruned);
            start = end;
            continue;
        }
        let known: &[Set] = pruned;
        let one = |&a: &Set| -> (Set, Option<Outcome>, DpStats) {
            let mut st = DpStats { candidates: 1, ..Default::default() };
            if contains_pruned(known, a) {
                st.pruned = 1;
                return (a, None, st);
            }
            let o = ctx.classify(a, &mut st);
            (a, Some(o), st)
        };
        let res: Vec<(Set, Option<Outcome>, DpStats)> = if parallel && level.len() > 1 { level.par_iter().map(one).collect() } else { level.iter().map(one).collect() };
        let mut new_pruned = vec![];
        for (a, o, st) in res {
            stats.add(&st);
            match o {
                Some(Outcome::Optimal(d)) => out.push((a, d)),
                Some(Outcome::Infeasible) => {
                    new_pruned.push(a);
                    if let Some(p) = ctx.perm {
                        new_pruned.push(apply_perm(a, p));
                    }
                }
                _ => {}
            }
        }
        pruned.extend(new_pruned);
        start = end;
    }
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
    let ordered = opts.global_order || opts.parallel;
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
        // Alg. 2: the candidates of one parent A_l by increasing cardinality (pruning list shared)
        let start = cands.len();
        for &aj in subsets {
            if (aj.count_ones() as usize) > room {
                break;
            }
            cands.push(aj | (al << q0));
        }
        if !ordered {
            let batch: Vec<Set> = cands.drain(start..).collect();
            run_levels(ctx, &batch, &mut pruned, &mut out, false, stats);
        }
    }
    if ordered {
        cands.sort_by_key(|&s| s.count_ones());
        run_levels(ctx, &cands, &mut pruned, &mut out, opts.parallel, stats);
    }
    out
}
