//! Kenefake and Pistikopoulos (2023), "A novel parallel combinatorial algorithm for multiparametric
//! programming" (Optimization and Engineering 25:1333-1361), ported onto the fast combinatorial
//! machinery (bit masks, feasibility witnesses, lambda-first region test, rank certificates).
//!
//! By paper section:
//!  - Sect. 2.2 / Algorithm 3: E, the constraints that can be tight at an optimal (KKT) point for
//!    some parameter. A constraint outside E is in no critical region's active set. Pruning rule 1
//!    (Algorithm 1): a candidate set must lie inside E.
//!  - Sect. 2.3 / Algorithm 4: W, the pairs of constraints that cannot be tight together at any
//!    KKT point. Pruning rule 2 (Algorithm 2): a candidate set containing a pair of W is dropped.
//!  - Sect. 3 / Algorithms 5 and 6: breadth-first exploration of the combinatorial tree with both
//!    rules, then the feasibility test of Gupta et al. on each surviving child.
//!
//! Deviations from the paper (the output is the region set of `combinatorial`):
//!  - The paper answers "can constraints P be tight at a point of the pKKT system?" with a big-M
//!    MILP (Eq. 3). This crate has no MILP solver, so the pKKT system (stationarity, primal and
//!    parameter feasibility, lambda >= 0) is solved as an LP and the complementarity conditions are
//!    enforced by branching on a violated pair (lambda_j = 0 or s_j = 0). An infeasible node is
//!    exact; when a node budget runs out the answer is "possible", which only weakens pruning. The
//!    cardinality bound N (Eq. 6) is not used.
//!  - Primal tests of Appendix B are subsumed: the pKKT system contains primal feasibility.
//!  - The infeasibility pruning list of Gupta et al. is kept (the paper drops it for NUMA
//!    scalability; here it is cheap and removes most infeasible children).
//!  - Pairs can be classified up front (`Pairs::Eager`, the paper) or when a candidate first needs
//!    them (`Pairs::Lazy`, the default): a pair is only asked about when a feasible parent contains
//!    one of its elements and the other is a candidate child.
//!  - A classification LP has about nx + nt + m variables, far more than the (x, theta) LPs of the
//!    enumeration, so a query costs 10 to 100 children tests. Queries are therefore cheap and
//!    shallow first (`NODE_BUDGET`); pairs left open that block many candidates of a sample of
//!    parents (`HOT`) are asked again with a large budget; and the classification is only switched
//!    on at the first level with at least `activate_at` candidate children, so small problems
//!    run exactly as `combinatorial`. If it is switched on late, parents that already break rule 1
//!    or 2 get no children.
//!
//! Safety. A critical region of an active set A is a full-dimensional set of parameters theta at
//! which (x*, lambda) solves the pKKT system with every constraint of A tight (and lambda_A >= 0).
//! So a region with i, j in A yields a pKKT point with s_i = s_j = 0. If the pKKT system with
//! s_i = s_j = 0 has no solution, no region contains both, and by the same argument for a single
//! constraint, none contains i. Every prefix of a region's set (ascending order) is rank
//! independent and feasible, hence is generated, and holds only members of E and no pair of W, so
//! the pruning never cuts a path to a region. The "no solution" verdicts are made robust against
//! solver tolerance by solving a relaxed system (inequalities loosened by `DELTA`, lambda >= -DELTA),
//! so a verdict holds with a margin and every infeasibility is a Farkas ray checked by the LP
//! solver on the original data. Any unresolved question (budget, tolerance) is answered "possible".

use super::*;
use crate::linalg::Mat;
use crate::solver::lp::lp_point;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Margin by which the inequalities of the pKKT system are loosened before declaring it infeasible.
const DELTA: f64 = 1e-6;
/// Complementarity tolerance on the slack and multiplier of an LP point.
const COMP_TOL: f64 = 1e-7;
/// Node budget of one complementarity branch and bound.
const NODE_BUDGET: usize = 6;
/// Node budget when an unresolved pair is hot (blocks many candidates).
const RESOLVE_BUDGET: usize = 120;
/// A set that is not generated also spares its descendants: a blocked candidate is worth this many
/// tested children.
const SUBTREE: f64 = 8.0;
/// An unresolved pair is re-asked when its expected saving exceeds this multiple of the cost.
const HOT_RATIO: f64 = 1.0;
/// Random-walk samples of the parameter space whose optimal active sets seed E and the known pairs.
const SAMPLES: usize = 300;

/// When pairs are classified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pairs {
    /// all pairs of E before the enumeration (the paper)
    Eager,
    /// a pair when a feasible parent first makes it relevant
    Lazy,
}

/// Counters of one solve.
#[derive(Clone, Debug, Default)]
pub struct KpStats {
    /// |E| and the number of constraints
    pub e_size: usize,
    pub m: usize,
    /// pairs of W known at the end
    pub w_pairs: usize,
    /// classification LPs (nodes of the branch and bounds) and queries answered by them
    pub class_lps: u64,
    pub class_queries: u64,
    pub class_unknown: u64,
    pub class_resolved: u64,
    pub class_secs: f64,
    /// candidate children i above max(S) of every explored parent S
    pub generated: u64,
    /// dropped by the mpLP size bound, rule 1 (not in E), rule 2 (pair of W), the pruning list
    pub pruned_size: u64,
    pub pruned_e: u64,
    pub pruned_w: u64,
    pub pruned_list: u64,
    /// children not generated because the parent itself breaks rule 1 or 2 (classification late)
    pub pruned_parent: u64,
    /// levels at which the classification was switched on and (if at all) off again
    pub on_level: Option<usize>,
    pub off_level: Option<usize>,
    /// children given to the rank test / feasibility LP / region test
    pub tested: u64,
    /// tested children that were feasible, and those that gave a region
    pub feasible: u64,
    pub regions: u64,
}

#[derive(Default)]
struct Counters {
    generated: AtomicU64,
    pruned_size: AtomicU64,
    pruned_e: AtomicU64,
    pruned_w: AtomicU64,
    pruned_list: AtomicU64,
    pruned_parent: AtomicU64,
    tested: AtomicU64,
    feasible: AtomicU64,
    /// audit parents: candidates only the classification dropped (the list would keep them), and
    /// candidates tested
    audit_saved: AtomicU64,
    audit_tested: AtomicU64,
}

/// E and the pair table. `no[i]` bit j: i and j can never be tight together (symmetric);
/// `asked[i]` bit j: the pair has been classified (yes, no or unresolved); `unk[i]` bit j: the
/// budget ran out on the pair; `heat[i * m + j]`: audit parents in which the unresolved pair
/// blocked a candidate.
struct Classes<'a> {
    kkt: Pkkt<'a>,
    m: usize,
    e: u64,
    eq_mask: u64,
    no: Vec<AtomicU64>,
    asked: Vec<AtomicU64>,
    unk: Vec<AtomicU64>,
    heat: Vec<AtomicU32>,
    lps: AtomicU64,
    /// time spent in pair queries (summed over threads)
    nanos: AtomicU64,
    queries: AtomicU64,
    unknown: AtomicU64,
    resolved: AtomicU64,
    resolved_no: AtomicU64,
}

impl Classes<'_> {
    fn add_to(&self, st: &mut KpStats) {
        st.class_lps += self.lps.load(Ordering::Relaxed);
        st.class_queries += self.queries.load(Ordering::Relaxed);
        st.class_unknown += self.unknown.load(Ordering::Relaxed);
        st.class_resolved += self.resolved.load(Ordering::Relaxed);
        st.w_pairs += self.no.iter().map(|x| x.load(Ordering::Relaxed).count_ones() as usize).sum::<usize>() / 2;
    }

    fn is_no(&self, i: usize, j: usize) -> bool {
        self.no[i].load(Ordering::Relaxed) >> j & 1 == 1
    }

    /// The constraints of `t` are tight together at a KKT point: all their pairs are possible.
    fn mark_yes(&self, t: u64) {
        let mut r = t;
        while r != 0 {
            let i = r.trailing_zeros() as usize;
            r &= r - 1;
            self.asked[i].fetch_or(t, Ordering::Relaxed);
            self.unk[i].fetch_and(!t, Ordering::Relaxed);
            // a point beats an infeasibility verdict that the tolerance of a solver may have blurred
            if self.no[i].load(Ordering::Relaxed) & t != 0 {
                self.no[i].fetch_and(!t, Ordering::Relaxed);
            }
        }
    }

    /// Classify the pair (i, j) unless known.
    fn ask(&self, i: usize, j: usize) {
        if self.asked[i].load(Ordering::Relaxed) >> j & 1 == 1 {
            return;
        }
        self.queries.fetch_add(1, Ordering::Relaxed);
        self.run(i, j, NODE_BUDGET);
    }

    /// Re-ask an unresolved pair with a larger budget.
    fn resolve(&self, i: usize, j: usize) {
        if self.unk[i].load(Ordering::Relaxed) >> j & 1 == 0 {
            return;
        }
        self.resolved.fetch_add(1, Ordering::Relaxed);
        // a deeper search pays only where earlier ones kept finding exclusions
        let (res, no) = (self.resolved.load(Ordering::Relaxed), self.resolved_no.load(Ordering::Relaxed));
        self.run(i, j, if res >= 8 && no * 3 >= res { 4 * RESOLVE_BUDGET } else { RESOLVE_BUDGET });
        if self.is_no(i, j) {
            self.resolved_no.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn run(&self, i: usize, j: usize, budget: usize) {
        let t0 = std::time::Instant::now();
        let (out, lps) = self.kkt.query(1 << i | 1 << j, budget);
        self.nanos.fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
        self.lps.fetch_add(lps as u64, Ordering::Relaxed);
        match out {
            Outcome::No => {
                self.no[i].fetch_or(1 << j, Ordering::Relaxed);
                self.no[j].fetch_or(1 << i, Ordering::Relaxed);
                self.asked[i].fetch_or(1 << j, Ordering::Relaxed);
                self.asked[j].fetch_or(1 << i, Ordering::Relaxed);
                self.unk[i].fetch_and(!(1 << j), Ordering::Relaxed);
                self.unk[j].fetch_and(!(1 << i), Ordering::Relaxed);
            }
            Outcome::Yes(t) => self.mark_yes(t | 1 << i | 1 << j),
            Outcome::Unknown => {
                if self.unk[i].load(Ordering::Relaxed) >> j & 1 == 0 {
                    self.unknown.fetch_add(1, Ordering::Relaxed);
                }
                self.asked[i].fetch_or(1 << j, Ordering::Relaxed);
                self.asked[j].fetch_or(1 << i, Ordering::Relaxed);
                self.unk[i].fetch_or(1 << j, Ordering::Relaxed);
                self.unk[j].fetch_or(1 << i, Ordering::Relaxed);
            }
        }
    }
}

/// pKKT system without complementarity in the variables z = (x, theta, lambda).
struct Pkkt<'a> {
    program: &'a MpProgram,
    a: Mat,
    /// right-hand sides with the inequalities loosened by DELTA
    b: Vec<f64>,
    nx: usize,
    nt: usize,
    m: usize,
    /// row of the primal constraint j; row of lambda_j >= 0 (usize::MAX for equalities)
    primal0: usize,
    lam_row: Vec<usize>,
    eq_mask: u64,
}

enum Outcome {
    /// proven impossible
    No,
    /// a KKT point exists (mask of its tight constraints)
    Yes(u64),
    /// budget exhausted: cannot exclude
    Unknown,
}

impl<'a> Pkkt<'a> {
    fn new(program: &'a MpProgram) -> Self {
        let (nx, nt, m) = (program.num_x(), program.num_t(), program.num_constraints());
        let mt = program.a_t.rows;
        let eq_mask = program.equality_indices.iter().fold(0u64, |k, &i| k | (1 << i));
        let nz = nx + nt + m;
        let n_ineq = m - program.equality_indices.len();
        let rows = nx + m + mt + n_ineq;
        let mut a = Mat::zeros(rows, nz);
        let mut b = vec![0.0; rows];
        for r in 0..nx {
            let row = a.row_mut(r);
            if let Some(q) = &program.q {
                row[..nx].copy_from_slice(q.row(r));
            }
            row[nx..nx + nt].copy_from_slice(program.h.row(r));
            for j in 0..m {
                row[nx + nt + j] = program.a[(j, r)];
            }
            b[r] = -program.c[r];
        }
        let primal0 = nx;
        for j in 0..m {
            let row = a.row_mut(primal0 + j);
            row[..nx].copy_from_slice(program.a.row(j));
            for (k, v) in program.f.row(j).iter().enumerate() {
                row[nx + k] = -v;
            }
            b[primal0 + j] = program.b[j] + DELTA;
        }
        for i in 0..mt {
            a.row_mut(primal0 + m + i)[nx..nx + nt].copy_from_slice(program.a_t.row(i));
            b[primal0 + m + i] = program.b_t[i] + DELTA;
        }
        let mut lam_row = vec![usize::MAX; m];
        let mut r = primal0 + m + mt;
        for j in 0..m {
            if eq_mask >> j & 1 == 0 {
                a[(r, nx + nt + j)] = -1.0;
                b[r] = DELTA;
                lam_row[j] = r;
                r += 1;
            }
        }
        Pkkt { program, a, b, nx, nt, m, primal0, lam_row, eq_mask }
    }

    /// Is there a KKT point with every constraint of `forced` tight? Also returns the LP count.
    fn query(&self, forced: u64, budget: usize) -> (Outcome, usize) {
        let mut stack: Vec<(u64, u64)> = vec![(forced | self.eq_mask, 0)];
        let mut nodes = 0;
        let p = self.program;
        while let Some((tight, lam0)) = stack.pop() {
            if nodes >= budget {
                return (Outcome::Unknown, nodes);
            }
            nodes += 1;
            let mut eq: Vec<usize> = (0..self.nx).collect();
            let mut b = self.b.clone();
            for j in 0..self.m {
                if tight >> j & 1 == 1 {
                    eq.push(self.primal0 + j);
                    b[self.primal0 + j] = p.b[j];
                }
                if lam0 >> j & 1 == 1 {
                    eq.push(self.lam_row[j]);
                    b[self.lam_row[j]] = 0.0;
                }
            }
            let Some(z) = lp_point(None, &self.a, &b, &eq) else { continue };
            // primal heuristic: the optimal active set at the LP point's parameter is a genuine
            // KKT point, and often already has the forced constraints tight
            if let Some(act) = p.active_set_at(&z[self.nx..self.nx + self.nt]) {
                let t = act.iter().fold(self.eq_mask, |t, &i| t | 1 << i);
                if forced & !t == 0 {
                    return (Outcome::Yes(t), nodes);
                }
            }
            let (mut worst, mut wj, mut tight_first) = (0.0f64, usize::MAX, true);
            let mut t: u64 = self.eq_mask;
            for j in 0..self.m {
                let s = p.b[j] - crate::linalg::dot(p.a.row(j), &z[..self.nx]) + crate::linalg::dot(p.f.row(j), &z[self.nx..self.nx + self.nt]);
                let l = z[self.nx + self.nt + j];
                if s <= COMP_TOL {
                    t |= 1 << j;
                }
                if self.eq_mask >> j & 1 == 0 {
                    let v = s.min(l);
                    if v > COMP_TOL && v > worst {
                        worst = v;
                        wj = j;
                        tight_first = s <= l;
                    }
                }
            }
            if wj == usize::MAX {
                return (Outcome::Yes(t), nodes);
            }
            // explore first the branch nearer to the LP point
            if tight_first {
                stack.push((tight, lam0 | 1 << wj));
                stack.push((tight | 1 << wj, lam0));
            } else {
                stack.push((tight | 1 << wj, lam0));
                stack.push((tight, lam0 | 1 << wj));
            }
        }
        (Outcome::No, nodes)
    }
}

/// E (Algorithm 3), pair table seeded with the active sets of sampled parameters, and, for
/// `Pairs::Eager`, W (Algorithm 4).
fn classify<'a>(program: &'a MpProgram, pairs: Pairs, parallel: bool) -> Classes<'a> {
    let m = program.num_constraints();
    let kkt = Pkkt::new(program);
    let eq_mask = kkt.eq_mask;
    let z = || (0..m).map(|_| AtomicU64::new(0)).collect::<Vec<_>>();
    let cl = Classes { kkt, m, e: 0, eq_mask, no: z(), asked: z(), unk: z(), heat: (0..m * m).map(|_| AtomicU32::new(0)).collect(), lps: AtomicU64::new(0), nanos: AtomicU64::new(0), queries: AtomicU64::new(0), unknown: AtomicU64::new(0), resolved: AtomicU64::new(0), resolved_no: AtomicU64::new(0) };
    let mut e: u64 = eq_mask;
    // optimal active sets at sampled parameters are KKT points: they put their constraints in E
    // and make their pairs co-optimal without a solve (the paper's set Q, filled cheaply)
    {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        for a in program.sample_theta_space(SAMPLES, &mut rng) {
            let t = a.iter().fold(eq_mask, |t, &i| t | 1 << i);
            e |= t;
            cl.mark_yes(t);
        }
    }
    for i in 0..m {
        if e >> i & 1 == 1 {
            continue;
        }
        cl.queries.fetch_add(1, Ordering::Relaxed);
        let (out, lps) = cl.kkt.query(1 << i, NODE_BUDGET * 3);
        cl.lps.fetch_add(lps as u64, Ordering::Relaxed);
        match out {
            Outcome::No => {}
            Outcome::Yes(t) => {
                e |= t | 1 << i;
                cl.mark_yes(t | 1 << i);
            }
            Outcome::Unknown => {
                cl.unknown.fetch_add(1, Ordering::Relaxed);
                e |= 1 << i;
            }
        }
    }
    let cl = Classes { e, ..cl };
    if pairs == Pairs::Eager {
        let reg = e & !eq_mask;
        let list: Vec<(usize, usize)> = (0..m).filter(|&i| reg >> i & 1 == 1).flat_map(|i| (i + 1..m).filter(move |&j| reg >> j & 1 == 1).map(move |j| (i, j))).collect();
        // chunks, so that the points found by a chunk spare the pairs of the next
        let chunk = if parallel { 64 } else { 1 };
        for c in list.chunks(chunk) {
            let todo: Vec<&(usize, usize)> = c.iter().filter(|&&(i, j)| cl.asked[i].load(Ordering::Relaxed) >> j & 1 == 0).collect();
            if parallel {
                todo.par_iter().for_each(|&&(i, j)| cl.ask(i, j));
            } else {
                todo.iter().for_each(|&&(i, j)| cl.ask(i, j));
            }
        }
    }
    cl
}

#[derive(Default)]
struct KpGroup {
    feasible: Vec<(u64, Witnesses)>,
    infeasible: Vec<u64>,
    regions: Vec<CriticalRegion>,
}

fn range_mask(lo: usize, hi: usize) -> u64 {
    if hi <= lo {
        return 0;
    }
    (if hi >= 64 { u64::MAX } else { (1u64 << hi) - 1 }) & !((1u64 << lo) - 1)
}

/// Children S + {i}, i above the largest element of S: rule 1, rule 2 (with `cl`), the pruning
/// list, then the feasibility and region test of `GroupCtx`. A `dry` call only asks the pairs it
/// needs, records which unresolved pairs block candidates (heat), and counts the candidates that
/// only the classification removed and those left to test; it tests nothing.
#[allow(clippy::too_many_arguments)]
fn explore(program: &MpProgram, s: u64, w: &Witnesses, cl: Option<&Classes>, lazy: bool, dry: bool, murder: &CombinationTester, m: usize, nx: usize, is_lp: bool, cnt: &Counters) -> KpGroup {
    let start = 64 - s.leading_zeros() as usize;
    let mut out = KpGroup::default();
    // a parent that breaks rule 1 or 2 (only possible when the classification came late) has no
    // child that can reach a region
    if let Some(cl) = cl {
        let mut bad = s & !cl.e != 0;
        let mut r = s & !cl.eq_mask;
        while !bad && r != 0 {
            let j = r.trailing_zeros() as usize;
            r &= r - 1;
            bad = cl.no[j].load(Ordering::Relaxed) & s != 0;
        }
        if bad {
            if !dry {
                cnt.pruned_parent.fetch_add(range_mask(start, m).count_ones() as u64, Ordering::Relaxed);
            }
            return out;
        }
    }
    let mut set = mask_to_vec(s);
    set.push(0);
    let mut g = GroupCtx::new(program, w);
    let mut ok: Vec<usize> = vec![];
    // size bound for mpLPs
    let end = if is_lp { m.min((set.len() + m - nx).max(start)) } else { m };
    let all = range_mask(start, m);
    let sized = range_mask(start, end);
    if !dry {
        cnt.generated.fetch_add(all.count_ones() as u64, Ordering::Relaxed);
        cnt.pruned_size.fetch_add((all & !sized).count_ones() as u64, Ordering::Relaxed);
    }
    let mut cand = sized;
    let mut dropped = 0u64;
    if let Some(cl) = cl {
        if !dry {
            cnt.pruned_e.fetch_add((cand & !cl.e).count_ones() as u64, Ordering::Relaxed);
        }
        dropped = cand & !cl.e;
        cand &= cl.e;
        if lazy {
            let mut r = s & !cl.eq_mask;
            while r != 0 {
                let j = r.trailing_zeros() as usize;
                r &= r - 1;
                let mut c = cand & !cl.asked[j].load(Ordering::Relaxed);
                while c != 0 {
                    let i = c.trailing_zeros() as usize;
                    c &= c - 1;
                    cl.ask(j, i);
                }
            }
        }
        let mut forbidden = 0u64;
        let mut r = s & !cl.eq_mask;
        while r != 0 {
            let j = r.trailing_zeros() as usize;
            r &= r - 1;
            forbidden |= cl.no[j].load(Ordering::Relaxed);
        }
        if !dry {
            cnt.pruned_w.fetch_add((cand & forbidden).count_ones() as u64, Ordering::Relaxed);
        }
        dropped |= cand & forbidden;
        cand &= !forbidden;
        if dry {
            // only candidates that would reach the feasibility test count
            let mut pass = 0u64;
            let mut c = cand;
            while c != 0 {
                let i = c.trailing_zeros() as usize;
                c &= c - 1;
                pass |= (murder.check_child64(s, i) as u64) << i;
            }
            let mut r = s & !cl.eq_mask;
            while r != 0 {
                let j = r.trailing_zeros() as usize;
                r &= r - 1;
                let mut u = pass & cl.unk[j].load(Ordering::Relaxed);
                while u != 0 {
                    let i = u.trailing_zeros() as usize;
                    u &= u - 1;
                    cl.heat[j * cl.m + i].fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
    if dry {
        let mut saved = 0u64;
        while dropped != 0 {
            let i = dropped.trailing_zeros() as usize;
            dropped &= dropped - 1;
            saved += murder.check_child64(s, i) as u64;
        }
        let mut left = 0u64;
        while cand != 0 {
            let i = cand.trailing_zeros() as usize;
            cand &= cand - 1;
            left += murder.check_child64(s, i) as u64;
        }
        cnt.audit_saved.fetch_add(saved, Ordering::Relaxed);
        cnt.audit_tested.fetch_add(left, Ordering::Relaxed);
        return out;
    }
    while cand != 0 {
        let i = cand.trailing_zeros() as usize;
        cand &= cand - 1;
        if !murder.check_child64(s, i) {
            cnt.pruned_list.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        cnt.tested.fetch_add(1, Ordering::Relaxed);
        *set.last_mut().unwrap() = i;
        match g.check(&set) {
            None => out.infeasible.push(s | (1u64 << i)),
            Some(region) => {
                cnt.feasible.fetch_add(1, Ordering::Relaxed);
                out.regions.extend(region);
                ok.push(i);
            }
        }
    }
    out.feasible = ok.into_iter().map(|i| (s | (1u64 << i), g.witnesses_for(i))).collect();
    out
}

/// When to classify, and how.
#[derive(Clone, Copy, Debug)]
pub struct KpOptions {
    pub pairs: Pairs,
    /// run the classification at all (false: the plain `combinatorial_mask` loop, for counts)
    pub classify: bool,
    /// switch the classification on at the first level whose candidate children number at least
    /// this many (0: before the enumeration, as in the paper). Small problems then cost nothing.
    pub activate_at: u64,
    /// switch it off again when the audit parents of a level show that the classification removes
    /// less than this fraction of the children that survive the pruning list (0: never)
    pub min_benefit: f64,
}

impl Default for KpOptions {
    fn default() -> Self {
        KpOptions { pairs: Pairs::Lazy, classify: true, activate_at: 2_000_000, min_benefit: 0.0 }
    }
}

impl KpOptions {
    /// The paper's setting: classify everything before the enumeration and never switch off.
    pub fn paper() -> Self {
        KpOptions { pairs: Pairs::Eager, classify: true, activate_at: 0, min_benefit: 0.0 }
    }

    /// Classify lazily from the start and never switch off.
    pub fn always() -> Self {
        KpOptions { pairs: Pairs::Lazy, classify: true, activate_at: 0, min_benefit: 0.0 }
    }

    /// No classification: the loop of `combinatorial_mask` with counters.
    pub fn plain() -> Self {
        KpOptions { pairs: Pairs::Lazy, classify: false, activate_at: 0, min_benefit: 0.0 }
    }
}

/// Parents of a level whose children are audited.
const AUDIT_PARENTS: usize = 48;
/// Candidates the audit sample must hold before it can switch the classification off.
const AUDIT_MIN_CANDIDATES: f64 = 3000.0;
/// Largest audit sample.
const AUDIT_MAX_PARENTS: usize = 768;

/// Region set of `combinatorial` (m <= 64) with the counters.
pub fn solve_with_stats(program: &MpProgram, parallel: bool, opts: KpOptions) -> (Vec<CriticalRegion>, KpStats) {
    let m = program.num_constraints();
    assert!(m <= 64, "bit mask path needs at most 64 constraints");
    let nx = program.num_x();
    let nt = program.num_t();
    let eq = program.equality_indices.clone();
    let max_depth = nx.max(nt).saturating_sub(eq.len());
    let is_lp = !program.is_qp();
    let mut stats = KpStats { m, ..Default::default() };
    let lazy = opts.pairs == Pairs::Lazy;
    let cnt = Counters::default();
    let mut murder = CombinationTester::new(m);
    let mut regions: Vec<CriticalRegion> = vec![];
    let eq_mask = eq.iter().fold(0u64, |k, &i| k | (1u64 << i));
    let mut parents: Vec<(u64, Witnesses)> = vec![(eq_mask, Arc::new(vec![]))];
    let mut cl: Option<Classes> = None;
    let mut allowed = opts.classify;
    let mut enum_secs = 0.0f64;
    let threads = if parallel { rayon::current_num_threads() as f64 } else { 1.0 };
    for level in 0..max_depth {
        if parents.is_empty() {
            break;
        }
        if allowed && cl.is_none() {
            let est: u64 = parents.iter().map(|(s, _)| (m - (64 - s.leading_zeros() as usize).min(m)) as u64).sum();
            if est >= opts.activate_at {
                let t0 = std::time::Instant::now();
                let c = classify(program, opts.pairs, parallel);
                stats.class_secs += t0.elapsed().as_secs_f64();
                stats.e_size = (c.e & !c.eq_mask).count_ones() as usize;
                stats.on_level = Some(level);
                cl = Some(c);
            }
        }
        let n = parents.len();
        let run = |idxs: &[usize], dry: bool, cl: Option<&Classes>| -> Vec<KpGroup> {
            let f = |&idx: &usize| {
                let (s, w) = &parents[idx];
                explore(program, *s, w, cl, lazy, dry, &murder, m, nx, is_lp, &cnt)
            };
            if parallel { idxs.par_iter().map(f).collect() } else { idxs.iter().map(f).collect() }
        };
        if cl.is_some() {
            // audit parents spread over the level: ask what they need, re-ask the hot unresolved
            // pairs harder, and drop the classification when it would not pay on this sample
            let c = cl.as_ref().unwrap();
            let sample = |k: usize| -> Vec<usize> { if n >= 2 * k { (0..n).step_by(n / k).collect() } else { (0..n).collect() } };
            // grow the sample until it holds enough candidates to judge
            let mut k = AUDIT_PARENTS;
            let mut audit_idx = sample(k);
            loop {
                c.heat.iter().for_each(|h| h.store(0, Ordering::Relaxed));
                let (saved0, tested0) = (cnt.audit_saved.load(Ordering::Relaxed), cnt.audit_tested.load(Ordering::Relaxed));
                run(&audit_idx, true, cl.as_ref());
                let cands = (cnt.audit_saved.load(Ordering::Relaxed) - saved0 + cnt.audit_tested.load(Ordering::Relaxed) - tested0) as f64;
                if cands >= AUDIT_MIN_CANDIDATES || audit_idx.len() >= n || k >= AUDIT_MAX_PARENTS {
                    break;
                }
                k *= 4;
                audit_idx = sample(k);
            }
            let scale = (n / audit_idx.len().max(1)).max(1) as u64;
            let tested_so_far = cnt.tested.load(Ordering::Relaxed) as f64;
            let class_s = c.nanos.load(Ordering::Relaxed) as f64 * 1e-9 / threads;
            let t_child = if tested_so_far > 0.0 { ((enum_secs - class_s).max(0.0) / tested_so_far).max(1e-7) } else { 3e-6 };
            let lps_so_far = c.lps.load(Ordering::Relaxed) as f64;
            let t_lp = if lps_so_far > 0.0 { c.nanos.load(Ordering::Relaxed) as f64 * 1e-9 / lps_so_far } else { 1e-4 };
            // unresolved pairs that block candidates, hottest first
            let mut hot: Vec<(f64, usize, usize)> = vec![];
            for j in 0..m {
                let mut u = c.unk[j].load(Ordering::Relaxed);
                while u != 0 {
                    let i = u.trailing_zeros() as usize;
                    u &= u - 1;
                    let blocked = c.heat[j * m + i].load(Ordering::Relaxed) as u64 * scale;
                    if i > j && blocked > 0 {
                        hot.push((blocked as f64 * t_child * SUBTREE, j, i));
                    }
                }
            }
            hot.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
            // the saving counts only if the pair is a No: the share of No among the pairs asked
            // again so far (prior: even odds) says how likely that is; stop when it does not pay
            for chunk in hot.chunks(4) {
                let (res, no) = (c.resolved.load(Ordering::Relaxed) as f64, c.resolved_no.load(Ordering::Relaxed) as f64);
                let p_no = (no + 0.5) / (res + 2.0);
                let todo: Vec<&(f64, usize, usize)> = chunk.iter().filter(|h| p_no * h.0 >= HOT_RATIO * RESOLVE_BUDGET as f64 * t_lp).collect();
                if todo.is_empty() {
                    break;
                }
                if parallel {
                    todo.par_iter().for_each(|&&(_, j, i)| c.resolve(j, i));
                } else {
                    todo.iter().for_each(|&&(_, j, i)| c.resolve(j, i));
                }
            }
            let (saved0, tested0) = (cnt.audit_saved.load(Ordering::Relaxed), cnt.audit_tested.load(Ordering::Relaxed));
            if opts.min_benefit > 0.0 {
                run(&audit_idx, true, cl.as_ref());
            }
            let saved = (cnt.audit_saved.load(Ordering::Relaxed) - saved0) as f64;
            let tested = (cnt.audit_tested.load(Ordering::Relaxed) - tested0) as f64;
            c.heat.iter().for_each(|h| h.store(0, Ordering::Relaxed));
            // a sample of few candidates says nothing (the shallow levels are small): decide later
            if opts.min_benefit > 0.0 && saved + tested >= AUDIT_MIN_CANDIDATES && saved / (saved + tested) < opts.min_benefit {
                if let Some(c) = cl.take() {
                    c.add_to(&mut stats);
                }
                allowed = false;
                stats.off_level = Some(level);
            }
        }
        let all_idx: Vec<usize> = (0..n).collect();
        let t_run = std::time::Instant::now();
        let res: Vec<KpGroup> = run(&all_idx, false, cl.as_ref());
        enum_secs += t_run.elapsed().as_secs_f64();
        for g in &res {
            for &c in &g.infeasible {
                murder.add_combo(&mask_to_vec(c));
            }
        }
        parents = vec![];
        for g in res {
            regions.extend(g.regions);
            parents.extend(g.feasible);
        }
    }
    if let Some(c) = cl.take() {
        c.add_to(&mut stats);
    }
    combinatorial_equality_set(program, &eq, &mut regions);
    stats.generated = cnt.generated.load(Ordering::Relaxed);
    stats.pruned_size = cnt.pruned_size.load(Ordering::Relaxed);
    stats.pruned_e = cnt.pruned_e.load(Ordering::Relaxed);
    stats.pruned_w = cnt.pruned_w.load(Ordering::Relaxed);
    stats.pruned_list = cnt.pruned_list.load(Ordering::Relaxed);
    stats.pruned_parent = cnt.pruned_parent.load(Ordering::Relaxed);
    stats.tested = cnt.tested.load(Ordering::Relaxed);
    stats.feasible = cnt.feasible.load(Ordering::Relaxed);
    stats.regions = regions.len() as u64;
    (regions, stats)
}

/// Verdicts of the classification, for tests: (E, W as pairs), equalities included in E.
pub fn classification(program: &MpProgram, pairs: Pairs) -> (Vec<usize>, Vec<(usize, usize)>) {
    let m = program.num_constraints();
    let cl = classify(program, pairs, false);
    let e = (0..m).filter(|&i| cl.e >> i & 1 == 1).collect();
    let w = (0..m).flat_map(|i| (i + 1..m).map(move |j| (i, j))).filter(|&(i, j)| cl.is_no(i, j)).collect();
    (e, w)
}

pub fn combinatorial_kp(program: &MpProgram, parallel: bool) -> Vec<CriticalRegion> {
    if program.num_constraints() > 64 {
        return combinatorial(program, parallel);
    }
    solve_with_stats(program, parallel, KpOptions::default()).0
}
