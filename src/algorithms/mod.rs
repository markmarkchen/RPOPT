//! Port of `ppopt.mp_solvers` (mpLP/mpQP algorithms) with rayon-parallel variants.

pub mod ahmadi;
pub mod approx;
pub mod common;
pub mod dp;
pub mod facet_completion;
pub mod incremental;
pub mod mpmilp_bb;
pub mod mpmiqp;
pub mod mpmiqp_exact;
pub mod mplcp;
pub mod mplp_lex;
pub mod mpr;
pub mod pdaqp;

use crate::critical_region::CriticalRegion;
use crate::linalg::{min_eigenvalue_sym, Mat};
use crate::mpqp_utils::gen_cr_from_active_set;
use crate::program::MpProgram;
use crate::solution::Solution;
use common::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rayon::prelude::*;
use crate::fxhash::FxHashSet as HashSet;
use std::collections::BinaryHeap;
use std::sync::Arc;

/// PPOPT `mpqp_algorithm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MpqpAlgorithm {
    Combinatorial,
    CombinatorialParallel,
    Graph,
    /// graph without the pruning list (PPOPT `graph_exp`)
    GraphExp,
    GraphParallel,
    CombinatorialGraph,
    CombinatorialGraphParallel,
    Geometric,
    GeometricParallel,
    /// Ahmadi-Moshkenani, Johansen, Olaru (IEEE TAC 2018) downward-upward exploration
    Ahmadi,
    /// Jones-Morari mp-LCP pivoting
    Mplcp,
    /// Herceg et al. enumeration of pLCP bases (mpQP and mpLP)
    MplcpEnum,
    /// Jones-Kerrigan-Maciejowski lexicographic perturbation (mpLP only; mpQPs go to `geometric`)
    MplpLex,
    /// Arnström and Axehill (CDC 2024) combinatorial connected graph with DAQP feasibility problems
    ParametricDaqp,
    /// `Mplcp` with each wave of regions explored in parallel
    MplcpParallel,
}

impl MpqpAlgorithm {
    pub fn all() -> &'static [MpqpAlgorithm] {
        use MpqpAlgorithm::*;
        &[Combinatorial, CombinatorialParallel, Graph, GraphExp, GraphParallel, CombinatorialGraph, CombinatorialGraphParallel, Geometric, GeometricParallel, Ahmadi, Mplcp, MplcpEnum, MplpLex, ParametricDaqp, MplcpParallel]
    }

    pub fn name(&self) -> &'static str {
        use MpqpAlgorithm::*;
        match self {
            Combinatorial => "combinatorial",
            CombinatorialParallel => "combinatorial_parallel",
            Graph => "graph",
            GraphExp => "graph_exp",
            GraphParallel => "graph_parallel",
            CombinatorialGraph => "combinatorial_graph",
            CombinatorialGraphParallel => "combinatorial_graph_parallel",
            Geometric => "geometric",
            GeometricParallel => "geometric_parallel",
            Ahmadi => "ahmadi",
            Mplcp => "mplcp",
            MplcpEnum => "mplcp_enum",
            MplpLex => "mplp_lex",
            ParametricDaqp => "parametric_daqp",
            MplcpParallel => "mplcp_parallel",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        Self::all().iter().cloned().find(|a| a.name() == s)
    }
}

#[derive(Clone, Debug)]
pub struct SolveOptions {
    /// seed for the random initialisation (graph / geometric / combinatorial graph)
    pub seed: u64,
    /// samples for the initial active-set search of the graph algorithm
    pub graph_samples: usize,
}

impl Default for SolveOptions {
    fn default() -> Self {
        SolveOptions { seed: 0, graph_samples: 100 }
    }
}

/// Solves an mpLP/mpQP (PPOPT `solve_mpqp`). Default algorithm in PPOPT is combinatorial.
pub fn solve_mpqp(program: &Arc<MpProgram>, algorithm: MpqpAlgorithm) -> Solution {
    solve_mpqp_with(program, algorithm, &SolveOptions::default())
}

pub fn solve_mpqp_with(program: &Arc<MpProgram>, algorithm: MpqpAlgorithm, opts: &SolveOptions) -> Solution {
    use MpqpAlgorithm::*;
    let regions = match algorithm {
        Combinatorial => combinatorial(program, false),
        CombinatorialParallel => combinatorial(program, true),
        Graph => graph(program, opts, true),
        GraphExp => graph(program, opts, false),
        GraphParallel => graph_parallel(program, opts),
        CombinatorialGraph => combinatorial_graph(program, opts, false),
        CombinatorialGraphParallel => combinatorial_graph(program, opts, true),
        Geometric => geometric(program, opts, false),
        GeometricParallel => geometric(program, opts, true),
        Ahmadi => ahmadi::ahmadi(program),
        Mplcp if program.q.is_none() => mplp_lex::regions(program, opts.seed),
        Mplcp => mplcp::regions(program, opts.seed, mplcp::Method::Pivot),
        MplcpEnum => mplcp::regions(program, opts.seed, mplcp::Method::Enumeration),
        MplcpParallel if program.q.is_none() => mplp_lex::regions(program, opts.seed),
        MplcpParallel => mplcp::regions(program, opts.seed, mplcp::Method::PivotParallel),
        MplpLex => {
            if program.is_qp() {
                geometric(program, opts, false)
            } else {
                mplp_lex::regions(program, opts.seed)
            }
        }
        ParametricDaqp => pdaqp::pdaqp(program),
    };
    let mut sol = Solution::new(program.clone(), regions);
    // PPOPT flags every solution as overlapping (its isinstance check also matches MPQP). We only
    // flag the cases where overlap is actually possible: mpLPs (degeneracy) and non-convex mpQPs.
    sol.is_overlapping = match &program.q {
        None => algorithm != MplpLex,
        Some(q) => min_eigenvalue_sym(q) <= 0.0,
    };
    sol
}

// ---------------------------------------------------------------------------- combinatorial

/// Gupta et al. (2011) combinatorial enumeration with the infeasible-set pruning list.
///
/// PPOPT calls `check_optimality` (a large LP over x, θ, λ, s) before building each region; that
/// LP is feasible exactly when the unreduced critical region is nonempty, which the region
/// construction itself checks, so the call is skipped here (identical output).
fn combinatorial(program: &MpProgram, parallel: bool) -> Vec<CriticalRegion> {
    let m = program.num_constraints();
    if m <= 64 {
        return combinatorial_mask(program, parallel);
    }
    let nx = program.num_x();
    let nt = program.num_t();
    let eq = program.equality_indices.clone();
    let max_depth = nx.max(nt).saturating_sub(eq.len());
    let mut murder = CombinationTester::new(m);
    let mut regions: Vec<CriticalRegion> = vec![];
    let is_lp = !program.is_qp();

    // candidates are grouped by parent; each group carries feasible (x, θ) points of the parent
    // (witnesses, see `check_group`)
    let mut groups: Vec<(Vec<Vec<usize>>, Witnesses)> = vec![(generate_children_sets(&eq, m, Some(&murder)), Arc::new(vec![]))];
    for depth in 0..max_depth {
        if is_lp {
            for g in groups.iter_mut() {
                g.0.retain(|child| !(child[child.len() - 1] >= child.len() + m - nx));
            }
        }
        // feasibility (rank + LP) and, for feasible sets, the region; infeasible sets go to the
        // pruning list. Regions keep PPOPT's order (feasible sets in candidate order).
        let check = |(c, w): &(Vec<Vec<usize>>, Witnesses)| {
            let mut g = GroupCtx::new(program, w);
            let res: Vec<Option<Option<CriticalRegion>>> = c.iter().map(|c| g.check(c)).collect();
            res.into_iter().zip(c).map(|(r, c)| r.map(|region| (g.witnesses_for(c[c.len() - 1]), region))).collect::<Vec<ChildResult>>()
        };
        let res: Vec<Vec<ChildResult>> = if parallel { groups.par_iter().map(check).collect() } else { groups.iter().map(check).collect() };
        let mut feasible: Vec<Vec<usize>> = vec![];
        let mut points: Vec<Witnesses> = vec![];
        for ((children, _), res) in groups.into_iter().zip(res) {
            for (c, r) in children.into_iter().zip(res) {
                match r {
                    Some((p, region)) => {
                        regions.extend(region);
                        feasible.push(c);
                        points.push(p);
                    }
                    None => murder.add_combo(&c),
                }
            }
        }
        if depth + 1 != max_depth {
            let children = |(c, p): (&Vec<usize>, &Witnesses)| -> (Vec<Vec<usize>>, Witnesses) { (generate_children_sets(c, m, Some(&murder)), p.clone()) };
            groups = if parallel {
                feasible.par_iter().zip(points.par_iter()).map(children).filter(|g| !g.0.is_empty()).collect()
            } else {
                feasible.iter().zip(points.iter()).map(children).filter(|g| !g.0.is_empty()).collect()
            };
        } else {
            groups = vec![];
        }
    }
    combinatorial_equality_set(program, &eq, &mut regions);
    regions
}

/// PPOPT's last step: the region of the equality set alone.
fn combinatorial_equality_set(program: &MpProgram, eq: &[usize], regions: &mut Vec<CriticalRegion>) {
    if program.check_feasibility(eq, true) && (program.is_qp() || eq.len() == program.num_x()) {
        if let Some(r) = gen_cr_from_active_set(program, eq, true) {
            if r.is_full_dimension() {
                regions.push(r);
            }
        }
    }
}

/// `combinatorial` for at most 64 constraints, with active sets as bit masks. Each step takes the
/// feasible sets of the previous depth with their witnesses; the children S ∪ {i} (i above the
/// largest element of S, surviving the pruning list) are generated and checked inside the parallel
/// map, which also returns the next depth's sets in order. Only the pruning list and the region
/// list are updated serially (both in PPOPT's candidate order).
fn combinatorial_mask(program: &MpProgram, parallel: bool) -> Vec<CriticalRegion> {
    let m = program.num_constraints();
    let nx = program.num_x();
    let nt = program.num_t();
    let eq = program.equality_indices.clone();
    let max_depth = nx.max(nt).saturating_sub(eq.len());
    let mut murder = CombinationTester::new(m);
    let mut regions: Vec<CriticalRegion> = vec![];
    let is_lp = !program.is_qp();
    let eq_mask = eq.iter().fold(0u64, |k, &i| k | (1u64 << i));
    let mut parents: Vec<(u64, Witnesses)> = vec![(eq_mask, Arc::new(vec![]))];
    for _ in 0..max_depth {
        if parents.is_empty() {
            break;
        }
        let check = |(s, w): &(u64, Witnesses)| check_group_mask(program, *s, w, &murder, m, nx, is_lp);
        let res: Vec<GroupOut> = if parallel { parents.par_iter().map(check).collect() } else { parents.iter().map(check).collect() };
        for g in &res {
            for &c in &g.infeasible {
                murder.add_combo(&mask_to_vec(c));
            }
        }
        let old = std::mem::take(&mut parents);
        if parallel {
            regions.extend(res.iter().flat_map(|g| g.regions.iter().cloned()));
            parents = res.into_par_iter().flat_map_iter(|g| g.feasible).collect();
            old.into_par_iter().for_each(drop);
        } else {
            for g in res {
                regions.extend(g.regions);
                parents.extend(g.feasible);
            }
        }
    }
    combinatorial_equality_set(program, &eq, &mut regions);
    regions
}

fn mask_to_vec(mut k: u64) -> Vec<usize> {
    let mut v = Vec::with_capacity(k.count_ones() as usize);
    while k != 0 {
        v.push(k.trailing_zeros() as usize);
        k &= k - 1;
    }
    v
}

/// Children of one set in `combinatorial_mask`, in candidate order.
#[derive(Default)]
struct GroupOut {
    feasible: Vec<(u64, Witnesses)>,
    infeasible: Vec<u64>,
    regions: Vec<CriticalRegion>,
}

fn check_group_mask(program: &MpProgram, s: u64, w: &Witnesses, murder: &CombinationTester, m: usize, nx: usize, is_lp: bool) -> GroupOut {
    let start = 64 - s.leading_zeros() as usize;
    let mut set = mask_to_vec(s);
    set.push(0);
    let mut g = GroupCtx::new(program, w);
    let mut out = GroupOut::default();
    let mut ok: Vec<usize> = vec![];
    for i in start..m {
        // PPOPT: children surviving the pruning list; for mpLPs, sets that can still reach n_x
        if !murder.check_child64(s, i) || (is_lp && i >= set.len() + m - nx) {
            continue;
        }
        *set.last_mut().unwrap() = i;
        match g.check(&set) {
            None => out.infeasible.push(s | (1u64 << i)),
            Some(region) => {
                out.regions.extend(region);
                ok.push(i);
            }
        }
    }
    out.feasible = ok.into_iter().map(|i| (s | (1u64 << i), g.witnesses_for(i))).collect();
    out
}

/// Feasible (x, θ) points that satisfy every equality of a set (shared by its children).
type Witnesses = Arc<Vec<Arc<Vec<f64>>>>;

/// Most witnesses carried per set.
const MAX_WITNESSES: usize = 8;

/// Relative tolerance for accepting a witness as tight on a new equality (100x stricter than the
/// LP's own primal acceptance test).
const WITNESS_TOL: f64 = 1e-11;

/// Per child: None (rank deficient or infeasible), or its witnesses and its region if any.
type ChildResult = Option<(Witnesses, Option<CriticalRegion>)>;

/// Feasibility of the children `S ∪ {k}` of one set S, given points `w` feasible for S with S's
/// constraints tight, and the critical region of each feasible child (PPOPT: `check_optimality`,
/// skipped, then `gen_cr_from_active_set`).
///
/// Witnesses: a point that also makes constraint k tight is feasible for the child, so the LP is
/// skipped; every LP solution (a vertex, tight on its basis) joins the pool, since it can witness
/// its siblings too. Feasibility decisions equal plain `check_feasibility` up to the tightness
/// tolerance.
///
/// Law first (QP, two or more parameters): the KKT law needed by the region test is computed
/// before the rank test; when its Cholesky certifies full rank (see
/// `optimal_control_law_dual_rank`) the elimination rank test is skipped. The region test then
/// starts with the λ rows (`gen_cr_from_law_lambda_first`).
///
/// The PPOPT-equivalent rank tests and LPs are counted as before.
struct GroupCtx<'a> {
    program: &'a MpProgram,
    pa: &'a Mat,
    pb: &'a [f64],
    /// inherited witnesses (borrowed) and the group's own LP solutions
    w: &'a [Arc<Vec<f64>>],
    found: Vec<Arc<Vec<f64>>>,
    /// every LP of the group starts at the first witness: its shifted rhs b - A x0 is shared
    shifted: Vec<f64>,
    is_lp: bool,
    nx: usize,
    law_first: bool,
}

impl<'a> GroupCtx<'a> {
    fn new(program: &'a MpProgram, w: &'a Witnesses) -> Self {
        let (pa, pb) = program.xtheta_cached();
        let is_lp = !program.is_qp();
        GroupCtx { program, pa, pb, w: w.as_slice(), found: vec![], shifted: vec![], is_lp, nx: program.num_x(), law_first: !is_lp && program.num_t() > 1 }
    }

    fn tight(&self, z: &[f64], k: usize) -> bool {
        let mut ax = 0.0;
        let mut mag = 0.0;
        for (u, v) in self.pa.row(k).iter().zip(z.iter()) {
            let p = u * v;
            ax += p;
            mag += p.abs();
        }
        (self.pb[k] - ax).abs() <= WITNESS_TOL * (1.0 + self.pb[k].abs() + mag)
    }

    /// None if the child `c` (sorted, new element last) is rank deficient or infeasible, else its
    /// region if any.
    fn check(&mut self, c: &[usize]) -> Option<Option<CriticalRegion>> {
        let program = self.program;
        crate::stats::rank();
        let (law, certified) = match self.law_first && c.len() <= self.nx {
            true => match program.optimal_control_law_dual_rank(c, true) {
                Some((l, cert)) => (Some(l), cert),
                None => (None, false),
            },
            false => (None, false),
        };
        if !certified && !crate::linalg::is_full_rank_rows(&program.a, c) {
            return None;
        }
        crate::stats::lp(1);
        let k = c[c.len() - 1];
        if !self.w.iter().chain(self.found.iter()).any(|z| self.tight(z, k)) {
            let z = match self.w.first() {
                Some(x0) => {
                    if self.shifted.is_empty() {
                        let (pa, pb) = (self.pa, self.pb);
                        self.shifted.extend((0..pa.rows).map(|i| pb[i] - crate::linalg::dot(pa.row(i), x0)));
                    }
                    crate::solver::lp::lp_feasible_point_shifted(self.pa, &self.shifted, c, x0)
                }
                None => crate::solver::lp::lp_feasible_point(self.pa, self.pb, c, None),
            }?;
            self.found.push(Arc::new(z));
        }
        if self.is_lp && c.len() != self.nx {
            return Some(None);
        }
        crate::stats::lp(1); // PPOPT's check_optimality LP, skipped (see `combinatorial`)
        Some(if self.law_first {
            law.and_then(|l| crate::mpqp_utils::gen_cr_from_law_lambda_first(program, c, l, true))
        } else {
            crate::mpqp_utils::gen_cr_from_active_set_lambda_first(program, c, true)
        })
    }

    /// Witnesses of a feasible child with new element `k` (call after the whole group).
    fn witnesses_for(&self, k: usize) -> Witnesses {
        Arc::new(self.w.iter().chain(self.found.iter()).filter(|z| self.tight(z, k)).take(MAX_WITNESSES).cloned().collect())
    }
}

// ---------------------------------------------------------------------------- graph

#[derive(PartialEq, Eq)]
struct Queued {
    len: usize,
    seq: usize,
    set: Vec<usize>,
}

impl Ord for Queued {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // min-heap on (len, seq): mimics PPOPT's stable sort by cardinality then pop(0)
        other.len.cmp(&self.len).then(other.seq.cmp(&self.seq))
    }
}

impl PartialOrd for Queued {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Oberdieck et al. (2016) connected-graph algorithm.
fn graph(program: &Arc<MpProgram>, opts: &SolveOptions, use_pruning: bool) -> Vec<CriticalRegion> {
    let mut rng = StdRng::seed_from_u64(opts.seed);
    let initial = program.sample_theta_space(opts.graph_samples, &mut rng);
    let m = program.num_constraints();
    let eq = program.equality_indices.clone();
    let mut attempted: HashSet<Vec<usize>> = HashSet::default();
    let mut murder = CombinationTester::new(m);
    let (is_lp, nx) = (!program.is_qp(), program.num_x());
    let mut regions = vec![];
    let mut heap = BinaryHeap::new();
    let mut seq = 0usize;
    let push = |heap: &mut BinaryHeap<Queued>, s: Vec<usize>, seq: &mut usize| {
        heap.push(Queued { len: s.len(), seq: *seq, set: s });
        *seq += 1;
    };
    for s in initial {
        push(&mut heap, s, &mut seq);
    }
    while let Some(Queued { set: candidate, .. }) = heap.pop() {
        if attempted.contains(&candidate) {
            continue;
        }
        attempted.insert(candidate.clone());
        let mref = if use_pruning { Some(&murder) } else { None };
        // mpLP: only vertex-defining sets (size n_x) can be regions; smaller sets lead nowhere
        if is_lp && candidate.len() < nx {
            continue;
        }
        crate::stats::rank();
        if !program.check_active_set_rank(&candidate) {
            if is_lp && candidate.len() <= nx {
                if use_pruning {
                    murder.add_combo(&candidate);
                }
                continue;
            }
            let red = generate_reduce(&candidate, mref, Some(&attempted), &eq);
            for s in red {
                push(&mut heap, s, &mut seq);
            }
            if use_pruning && !is_lp {
                murder.add_combo(&candidate);
            }
            continue;
        }
        // PPOPT's feasibility test here is `is None` on a bool and never fires; we keep that behaviour.
        let Some(region) = optimal_region(program, &candidate) else {
            if is_lp && candidate.len() <= nx {
                continue;
            }
            for s in generate_reduce(&candidate, mref, Some(&attempted), &eq) {
                push(&mut heap, s, &mut seq);
            }
            continue;
        };
        let Some(region) = region else { continue };
        if region.is_full_dimension() {
            let new_sets = if is_lp {
                generate_lp_swaps(&candidate, &region.regular_set.1, &region.lambda_set, m, &eq, mref, Some(&attempted))
            } else {
                let mut ns = generate_reduce(&candidate, mref, Some(&attempted), &eq);
                ns.extend(generate_extra(&candidate, &region.regular_set.1, mref, Some(&attempted)));
                ns
            };
            regions.push(region);
            for s in new_sets {
                push(&mut heap, s, &mut seq);
            }
        }
    }
    if is_lp {
        regions = lp_repair(program, regions, false);
    }
    regions
}

enum GraphOutcome {
    RankDeficient(Vec<Vec<usize>>),
    NotOptimal(Vec<Vec<usize>>),
    Region(Box<CriticalRegion>, Vec<Vec<usize>>),
    Nothing,
}

/// Level-synchronous parallel version of the graph algorithm: every candidate of the current
/// frontier is processed in parallel, then the bookkeeping is merged.
fn graph_parallel(program: &Arc<MpProgram>, opts: &SolveOptions) -> Vec<CriticalRegion> {
    let mut rng = StdRng::seed_from_u64(opts.seed);
    let initial = program.sample_theta_space(opts.graph_samples, &mut rng);
    let m = program.num_constraints();
    let eq = program.equality_indices.clone();
    let mut attempted: HashSet<Vec<usize>> = HashSet::default();
    let mut murder = CombinationTester::new(m);
    let (is_lp, nx) = (!program.is_qp(), program.num_x());
    let mut regions = vec![];
    let mut frontier: Vec<Vec<usize>> = initial;
    while !frontier.is_empty() {
        let mut uniq: Vec<Vec<usize>> = vec![];
        let mut seen = HashSet::default();
        for s in frontier.drain(..) {
            if !attempted.contains(&s) && seen.insert(s.clone()) {
                uniq.push(s);
            }
        }
        for s in &uniq {
            attempted.insert(s.clone());
        }
        let outcomes: Vec<GraphOutcome> = uniq
            .par_iter()
            .map(|cand| {
                if is_lp && cand.len() < nx {
                    return GraphOutcome::Nothing;
                }
                crate::stats::rank();
                if !program.check_active_set_rank(cand) {
                    if is_lp && cand.len() <= nx {
                        return GraphOutcome::RankDeficient(vec![]);
                    }
                    return GraphOutcome::RankDeficient(generate_reduce(cand, Some(&murder), Some(&attempted), &eq));
                }
                let Some(region) = optimal_region(program, cand) else {
                    if is_lp && cand.len() <= nx {
                        return GraphOutcome::Nothing;
                    }
                    return GraphOutcome::NotOptimal(generate_reduce(cand, Some(&murder), Some(&attempted), &eq));
                };
                match region {
                    Some(r) if r.is_full_dimension() => {
                        let ns = if is_lp {
                            generate_lp_swaps(cand, &r.regular_set.1, &r.lambda_set, m, &eq, Some(&murder), Some(&attempted))
                        } else {
                            let mut ns = generate_reduce(cand, Some(&murder), Some(&attempted), &eq);
                            ns.extend(generate_extra(cand, &r.regular_set.1, Some(&murder), Some(&attempted)));
                            ns
                        };
                        GraphOutcome::Region(Box::new(r), ns)
                    }
                    _ => GraphOutcome::Nothing,
                }
            })
            .collect();
        for (cand, out) in uniq.iter().zip(outcomes) {
            match out {
                GraphOutcome::RankDeficient(ns) => {
                    if !(is_lp && cand.len() > nx) {
                        murder.add_combo(cand);
                    }
                    frontier.extend(ns);
                }
                GraphOutcome::NotOptimal(ns) => frontier.extend(ns),
                GraphOutcome::Region(r, ns) => {
                    regions.push(*r);
                    frontier.extend(ns);
                }
                GraphOutcome::Nothing => {}
            }
        }
        frontier.sort_by_key(|s| s.len());
    }
    if is_lp {
        regions = lp_repair(program, regions, false);
    }
    regions
}

/// Degenerate mpLPs: the vertex bases of neighbouring regions need not differ by one pivot of
/// full-dimensional bases, so the connected-graph search can stop short. Like `geometric`, step
/// out of every facet part that no found region covers (see facet_completion) and continue from
/// the regions found that way. On nondegenerate problems this finds nothing.
fn lp_repair(program: &Arc<MpProgram>, regions: Vec<CriticalRegion>, parallel: bool) -> Vec<CriticalRegion> {
    use std::sync::atomic::AtomicBool;
    let mut indexed: HashSet<Vec<usize>> = regions.iter().map(|r| r.active_set.clone()).collect();
    let mut solution = Solution::new(program.clone(), regions);
    let degen = AtomicBool::new(true);
    let mut suspects: Vec<(usize, Option<Vec<f64>>)> = (0..solution.critical_regions.len()).map(|i| (i, None)).collect();
    while !suspects.is_empty() {
        let found = facet_completion::complete_facets(program, &solution, &suspects, &indexed, &degen, parallel);
        suspects.clear();
        for cr in found {
            if indexed.insert(cr.active_set.clone()) {
                suspects.push((solution.critical_regions.len(), None));
                solution.add_region(cr);
            }
        }
    }
    solution.critical_regions
}

// ---------------------------------------------------------------------------- combinatorial graph

/// Arnström et al. (2024) joint combinatorial / connected-graph exploration.
fn combinatorial_graph(program: &MpProgram, opts: &SolveOptions, parallel: bool) -> Vec<CriticalRegion> {
    if program.num_constraints() <= 128 {
        return combinatorial_graph_bits(program, opts, parallel);
    }
    combinatorial_graph_vec(program, opts, parallel)
}

/// Same exploration (same visiting order) with active sets stored as u128 bitmasks, which avoids
/// allocating one vector per recorded neighbour (m <= 128 constraints).
fn combinatorial_graph_bits(program: &MpProgram, opts: &SolveOptions, parallel: bool) -> Vec<CriticalRegion> {
    let mut rng = StdRng::seed_from_u64(opts.seed);
    let initial = program.sample_theta_space(1, &mut rng);
    let m = program.num_constraints();
    let is_lp = !program.is_qp();
    let nx = program.num_x();
    let to_bits = |a: &[usize]| a.iter().fold(0u128, |k, &i| k | (1u128 << i));
    let from_bits = |k: u128| -> Vec<usize> {
        let mut v = Vec::with_capacity(k.count_ones() as usize);
        let mut r = k;
        while r != 0 {
            let i = r.trailing_zeros() as usize;
            v.push(i);
            r &= r - 1;
        }
        v
    };
    let eq_mask = to_bits(&program.equality_indices);
    let full: u128 = if m == 128 { u128::MAX } else { (1u128 << m) - 1 };
    let mut excluded: HashSet<u128> = HashSet::default();
    // stack entries: (active set, rank hint, parent data). The hint is Some(true) for subsets of a
    // full-rank set, Some(b) from the incremental span test, None when unknown; supersets of a
    // KKT-feasible set carry its data for the bordered update (`incremental.rs`).
    type Entry = (u128, Option<bool>, Option<std::sync::Arc<incremental::ParentInfo>>);
    let mut stack: Vec<Entry> = vec![];
    for s in initial {
        let k = to_bits(&s);
        if excluded.insert(k) {
            stack.push((k, None, None));
        }
    }
    let mut regions = vec![];
    // returns (region, explore_subsets, explore_supersets, data for the supersets)
    let process = |k: u128, hint: Option<bool>, parent: Option<&incremental::ParentInfo>| -> (Option<CriticalRegion>, bool, bool, Option<std::sync::Arc<incremental::ParentInfo>>) {
        crate::stats::rank(); // PPOPT is_full_rank, answered here from the hint when available
        let a = from_bits(k);
        if hint == Some(false) {
            return (None, true, false, None);
        }
        // superset of a known set: bordered update (gamma > 0 also certifies full rank)
        // the λ >= 0 and parameter rows are tested before the inactive rows are built: most
        // candidates fail there (same answer as the full test)
        let mut kkt = None;
        if let Some(p) = parent {
            let i = (k & !to_bits(&p.a)).trailing_zeros() as usize;
            match p.child(program, i) {
                Some(incremental::ChildKkt::Region(r)) => kkt = Some(r),
                Some(incremental::ChildKkt::LambdaEmpty) => {
                    crate::stats::lp(1); // PPOPT feasability_check LP
                    return (None, false, false, None);
                }
                None => {}
            }
        }
        if kkt.is_none() {
            // the Schur factorization of the law certifies full rank for most candidates; the
            // matrix_rank-style test runs only when it does not
            let mut law = program.optimal_control_law_dual_rank(&a, false);
            let certified = matches!(&law, Some((_, true)));
            if hint.is_none() && !certified && !program.check_active_set_rank(&a) {
                return (None, true, false, None);
            }
            if law.is_none() {
                law = program.optimal_control_law_dual_rank(&a, true);
            }
            let Some((law, _)) = law else {
                crate::stats::lp(1);
                return (None, false, false, None);
            };
            if !crate::mpqp_utils::lambda_rows_feasible_raw(program, &law.a_l, &law.b_l) {
                crate::stats::lp(1); // PPOPT feasability_check LP
                return (None, false, false, None);
            }
            kkt = Some(crate::mpqp_utils::kkt_region_from_law(program, &a, law));
        }
        crate::stats::lp(1); // PPOPT feasability_check LP
        let mut kkt = kkt.unwrap();
        if kkt.nonempty() {
            if is_lp && a.len() != nx {
                return (None, true, true, None);
            }
            let info = incremental::ParentInfo::new(program, &a, &kkt.law).map(std::sync::Arc::new);
            return (crate::mpqp_utils::gen_cr_from_kkt(program, &a, kkt, true), true, true, info);
        }
        (None, false, false, None)
    };
    // `sup` is only set for full-rank sets (their KKT region was nonempty), so subsets inherit
    // full rank and supersets get the parent data (or the incremental span test without it)
    let expand = |k: u128, sub: bool, sup: bool, info: Option<&std::sync::Arc<incremental::ParentInfo>>, out: &mut Vec<Entry>| {
        if sub {
            let mut r = k & !eq_mask;
            while r != 0 {
                let i = r.trailing_zeros();
                out.push((k & !(1u128 << i), if sup { Some(true) } else { None }, None));
                r &= r - 1;
            }
        }
        if sup {
            let basis = if info.is_none() { RowBasis::new(&program.a, &from_bits(k)) } else { None };
            let mut r = full & !k;
            while r != 0 {
                let i = r.trailing_zeros();
                let kk = k | (1u128 << i);
                if (k.count_ones() as usize) + 1 > nx {
                    out.push((kk, Some(false), None));
                } else if let Some(p) = info {
                    out.push((kk, None, Some(p.clone())));
                } else {
                    let hint = basis.as_ref().and_then(|b| b.extends_rank(program.a.row(i as usize)));
                    out.push((kk, hint, None));
                }
                r &= r - 1;
            }
        }
    };
    if !parallel {
        let mut nb: Vec<Entry> = Vec::with_capacity(m);
        // first in, first out: breadth-first from the initial region, which builds the large
        // regions around it early (PPOPT pops from a Python set, i.e. in hash order). The explored
        // set does not depend on the order.
        let mut queue: std::collections::VecDeque<Entry> = std::mem::take(&mut stack).into();
        while let Some((k, hint, parent)) = queue.pop_front() {
            let (r, sub, sup, info) = process(k, hint, parent.as_deref());
            if let Some(r) = r {
                regions.push(r);
            }
            nb.clear();
            expand(k, sub, sup, info.as_ref(), &mut nb);
            for e in nb.drain(..) {
                if excluded.insert(e.0) {
                    queue.push_back(e);
                }
            }
        }
    } else {
        while !stack.is_empty() {
            let wave: Vec<Entry> = std::mem::take(&mut stack);
            let res: Vec<(Option<CriticalRegion>, Vec<Entry>)> = wave
                .par_iter()
                .map(|(k, hint, parent)| {
                    let (r, sub, sup, info) = process(*k, *hint, parent.as_deref());
                    let mut nb = vec![];
                    expand(*k, sub, sup, info.as_ref(), &mut nb);
                    (r, nb)
                })
                .collect();
            for (r, nb) in res {
                if let Some(r) = r {
                    regions.push(r);
                }
                for e in nb {
                    if excluded.insert(e.0) {
                        stack.push(e);
                    }
                }
            }
        }
    }
    regions
}

/// Orthonormal basis of the rows of a full-rank active set (modified Gram-Schmidt), for the
/// incremental rank test of its supersets.
struct RowBasis {
    q: Vec<Vec<f64>>,
    smax: f64,
    k: usize,
    n: usize,
}

impl RowBasis {
    fn new(a: &Mat, rows: &[usize]) -> Option<RowBasis> {
        let n = a.cols;
        let mut q: Vec<Vec<f64>> = Vec::with_capacity(rows.len());
        let mut smax: f64 = 0.0;
        for &r in rows {
            let mut v = a.row(r).to_vec();
            smax = smax.max(crate::linalg::norm(&v));
            for _ in 0..2 {
                for b in &q {
                    let d = crate::linalg::dot(&v, b);
                    for (x, y) in v.iter_mut().zip(b) {
                        *x -= d * y;
                    }
                }
            }
            let nv = crate::linalg::norm(&v);
            if nv <= 1e-12 * smax.max(1e-300) {
                return None;
            }
            for x in v.iter_mut() {
                *x /= nv;
            }
            q.push(v);
        }
        Some(RowBasis { q, smax, k: rows.len(), n })
    }

    /// Some(true/false) when adding `row` clearly keeps / loses full row rank; None if borderline
    /// (the caller then runs the full rank test).
    fn extends_rank(&self, row: &[f64]) -> Option<bool> {
        if self.k + 1 > self.n {
            return Some(false);
        }
        let mut v = row.to_vec();
        let nr = crate::linalg::norm(&v);
        for _ in 0..2 {
            for b in &self.q {
                let d = crate::linalg::dot(&v, b);
                for (x, y) in v.iter_mut().zip(b) {
                    *x -= d * y;
                }
            }
        }
        let res = crate::linalg::norm(&v);
        let scale = self.smax.max(nr).max(1e-300);
        if res > 1e-6 * scale {
            Some(true)
        } else if res < 1e-15 * scale {
            Some(false)
        } else {
            None
        }
    }
}

fn combinatorial_graph_vec(program: &MpProgram, opts: &SolveOptions, parallel: bool) -> Vec<CriticalRegion> {
    let mut rng = StdRng::seed_from_u64(opts.seed);
    let initial = program.sample_theta_space(1, &mut rng);
    let m = program.num_constraints();
    let eq = program.equality_indices.clone();
    let is_lp = !program.is_qp();
    let nx = program.num_x();
    let mut excluded: HashSet<Vec<usize>> = HashSet::default();
    let mut stack: Vec<Vec<usize>> = vec![];
    for s in initial {
        let mut s = s;
        s.sort_unstable();
        if excluded.insert(s.clone()) {
            stack.push(s);
        }
    }
    let mut regions = vec![];

    // returns (region, explore_subsets, explore_supersets)
    let process = |a: &Vec<usize>| -> (Option<CriticalRegion>, bool, bool) {
        crate::stats::rank();
        if !program.check_active_set_rank(a) {
            return (None, true, false);
        }
        crate::stats::lp(1); // PPOPT feasability_check LP
        let Some(mut kkt) = crate::mpqp_utils::kkt_region(program, a) else { return (None, false, false) };
        if kkt.nonempty() {
            if is_lp && a.len() != nx {
                return (None, true, true);
            }
            return (crate::mpqp_utils::gen_cr_from_kkt(program, a, kkt, true), true, true);
        }
        (None, false, false)
    };
    let expand = |a: &Vec<usize>, sub: bool, sup: bool, out: &mut Vec<Vec<usize>>| {
        if sub {
            for &i in a {
                if !eq.contains(&i) {
                    out.push(a.iter().cloned().filter(|&j| j != i).collect());
                }
            }
        }
        if sup {
            for i in 0..m {
                if !a.contains(&i) {
                    let mut t = a.clone();
                    t.push(i);
                    t.sort_unstable();
                    out.push(t);
                }
            }
        }
    };

    if !parallel {
        while let Some(a) = stack.pop() {
            let (r, sub, sup) = process(&a);
            if let Some(r) = r {
                regions.push(r);
            }
            let mut nb = vec![];
            expand(&a, sub, sup, &mut nb);
            for t in nb {
                if excluded.insert(t.clone()) {
                    stack.push(t);
                }
            }
        }
    } else {
        while !stack.is_empty() {
            let wave: Vec<Vec<usize>> = std::mem::take(&mut stack);
            let res: Vec<(Option<CriticalRegion>, Vec<Vec<usize>>)> = wave
                .par_iter()
                .map(|a| {
                    let (r, sub, sup) = process(a);
                    let mut nb = vec![];
                    expand(a, sub, sup, &mut nb);
                    (r, nb)
                })
                .collect();
            for (r, nb) in res {
                if let Some(r) = r {
                    regions.push(r);
                }
                for t in nb {
                    if excluded.insert(t.clone()) {
                        stack.push(t);
                    }
                }
            }
        }
    }
    regions
}

/// PPOPT graph step for a full-rank candidate: `check_optimality` (one LP), then the region built
/// without the full-dimension check. Outer None: not optimal; inner None: region not built.
fn optimal_region(program: &MpProgram, cand: &[usize]) -> Option<Option<CriticalRegion>> {
    if !program.is_qp() && cand.len() != program.num_x() {
        return None;
    }
    crate::stats::lp(1); // PPOPT's check_optimality LP
    // λ and parameter rows first: most candidates fail there (same answer as the full test)
    let law = program.optimal_control_law_dual(cand)?;
    if !crate::mpqp_utils::lambda_rows_feasible_raw(program, &law.a_l, &law.b_l) {
        return None;
    }
    let mut kkt = crate::mpqp_utils::kkt_region_from_law(program, cand, law);
    if !kkt.nonempty() {
        return None;
    }
    Some(gen_cr_from_active_set_kkt(program, cand, kkt))
}

fn gen_cr_from_active_set_kkt(program: &MpProgram, cand: &[usize], kkt: crate::mpqp_utils::KktRegion) -> Option<CriticalRegion> {
    crate::mpqp_utils::gen_cr_from_kkt(program, cand, kkt, false)
}

/// The unreduced critical region of an active set is nonempty (Arnström et al. feasibility check).
pub fn kkt_region_nonempty(program: &MpProgram, active_set: &[usize]) -> bool {
    crate::stats::lp(1);
    let Some(law) = program.optimal_control_law(active_set) else { return false };
    if !crate::mpqp_utils::lambda_rows_feasible_raw(program, &law.a_l, &law.b_l) {
        return false;
    }
    let (e, f, _, _) = program.region_rows(active_set, &law);
    crate::solver::lp::lp_feasible(&e, &f, &[])
}

// ---------------------------------------------------------------------------- geometric

/// Start for `geometric` when every sampled optimal active set is over-determined (the sampled
/// ball lies inside one degenerate vertex cone): the canonical basis of a sampled active set.
fn degenerate_start(program: &MpProgram, rng: &mut StdRng) -> Option<Vec<usize>> {
    use rand::Rng;
    let s = program.feasible_space_chebychev_ball()?;
    let (nx, nt) = (program.num_x(), program.num_t());
    let center = &s.x[nx..nx + nt];
    let radius = s.x[s.x.len() - 1];
    for _ in 0..500 {
        let test: Vec<f64> = center.iter().map(|c| c + rng.gen_range(-radius..=radius)).collect();
        if let Some(active) = program.active_set_at(&test) {
            if let Some(b) = canonical_basis(program, &active, &test) {
                return Some(b);
            }
        }
    }
    None
}

/// Spjøtvold et al. facet-stepping exploration.
fn geometric(program: &Arc<MpProgram>, opts: &SolveOptions, parallel: bool) -> Vec<CriticalRegion> {
    let mut rng = StdRng::seed_from_u64(opts.seed);
    let Some(active_set) = program.gen_optimal_active_set(&mut rng).or_else(|| degenerate_start(program, &mut rng)) else { return vec![] };
    let Some(initial) = gen_cr_from_active_set(program, &active_set, false) else { return vec![] };
    let mut solution = Solution::new(program.clone(), vec![initial.clone()]);
    let mut indexed: HashSet<Vec<usize>> = HashSet::default();
    indexed.insert(active_set.clone());
    use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
    let degen = AtomicBool::new(false);
    // Degeneracy bookkeeping: facets whose step met a degenerate active set (or whose region has a
    // duplicated facet row) are completed afterwards (see facet_completion); all else is untouched.
    // Entries are (index in the solution, Some(facet normal) or None for every facet).
    let mut suspects: Vec<(usize, Option<Vec<f64>>)> = vec![];
    let mut unchecked: std::collections::VecDeque<(usize, CriticalRegion)> = std::collections::VecDeque::new();
    unchecked.push_back((0, initial));
    if !parallel {
        while let Some((ci, cur)) = unchecked.pop_front() {
            let region_flag = AtomicBool::new(false);
            let centers = get_facet_centers_degen(&cur.e, &cur.f, Some(&region_flag));
            if region_flag.load(Relaxed) {
                suspects.push((ci, None));
            }
            for (center, normal, radius) in centers {
                let flag = AtomicBool::new(false);
                let found = fathem_facet_degen(&center, &normal, radius, program, &indexed, None, &cur.active_set, Some(&solution), Some(&flag));
                let flagged = flag.load(Relaxed);
                if flagged {
                    suspects.push((ci, Some(normal.clone())));
                }
                if let Some(cr) = found {
                    indexed.insert(cr.active_set.clone());
                    let idx = solution.critical_regions.len();
                    if flagged {
                        suspects.push((idx, None));
                    }
                    unchecked.push_back((idx, cr.clone()));
                    solution.add_region(cr);
                }
            }
        }
    } else {
        while !unchecked.is_empty() {
            let wave: Vec<(usize, CriticalRegion)> = unchecked.drain(..).collect();
            let tasks: Vec<(usize, Vec<f64>, Vec<f64>, f64)> = wave
                .par_iter()
                .enumerate()
                .flat_map_iter(|(wi, (_, r))| {
                    let region_flag = AtomicBool::new(false);
                    let mut v: Vec<_> = get_facet_centers_degen(&r.e, &r.f, Some(&region_flag)).into_iter().map(|(c, n, rad)| (wi, c, n, rad)).collect();
                    if region_flag.load(Relaxed) {
                        v.push((wi, vec![], vec![], f64::NAN)); // marker: whole region suspect
                    }
                    v
                })
                .collect();
            let claimed = std::sync::Mutex::new(HashSet::default());
            let found: Vec<(Option<CriticalRegion>, bool)> = tasks
                .par_iter()
                .map(|(wi, c, n, rad)| {
                    if rad.is_nan() {
                        return (None, true);
                    }
                    let flag = AtomicBool::new(false);
                    let cr = fathem_facet_degen(c, n, *rad, program, &indexed, Some(&claimed), &wave[*wi].1.active_set, Some(&solution), Some(&flag));
                    (cr, flag.load(Relaxed))
                })
                .collect();
            for ((wi, _, n, rad), (cr, flagged)) in tasks.iter().zip(found) {
                if flagged {
                    suspects.push((wave[*wi].0, if rad.is_nan() { None } else { Some(n.clone()) }));
                }
                if let Some(cr) = cr {
                    if indexed.insert(cr.active_set.clone()) {
                        let idx = solution.critical_regions.len();
                        if flagged {
                            suspects.push((idx, None));
                        }
                        unchecked.push_back((idx, cr.clone()));
                        solution.add_region(cr);
                    }
                }
            }
        }
    }
    // Degenerate problem (dependent or over-determined active sets met): the partition need not be
    // facet-to-facet, so complete the suspect facets (see facet_completion).
    while !suspects.is_empty() {
        degen.store(true, Relaxed);
        let found = facet_completion::complete_facets(program, &solution, &suspects, &indexed, &degen, parallel);
        suspects.clear();
        for cr in found {
            if indexed.insert(cr.active_set.clone()) {
                suspects.push((solution.critical_regions.len(), None));
                solution.add_region(cr);
            }
        }
    }
    solution.critical_regions
}

/// Convenience: a region-count summary used by tests and the benchmark.
pub fn region_signature(regions: &[CriticalRegion]) -> Vec<Vec<usize>> {
    let mut v: Vec<Vec<usize>> = regions.iter().map(|r| r.active_set.clone()).collect();
    v.sort();
    v
}

#[allow(dead_code)]
fn _unused(_: &Mat) {}
