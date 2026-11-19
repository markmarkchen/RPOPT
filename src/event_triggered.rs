//! Event-triggered multiparametric programming (ET-mp): an explicit solution that is built on demand.
//!
//! Instead of enumerating every critical region offline, the cache starts empty. Each query θ is
//! located among the regions built so far (a hit: the answer is the region's affine law at θ). A
//! query outside every cached region is an event (a miss): the program is solved at θ, the critical
//! region of the optimal active set is built and cached, and later queries near θ hit it. This is
//! the scheme of Tsai, Brahmbhatt & Avraamidou (ET-mp Benders, FOCAPO/CPC) and, with a bounded table
//! searched by frequency, of Pannocchia, Rawlings & Wright (partial enumeration MPC, Automatica 43,
//! 2007). Works for mpLP and mpQP programs (`MpProgram` with or without Q).
//!
//! # Switches ([`EtOptions`])
//!
//! * miss behaviour ([`MissPolicy`]): `Exact` (solve, cache, return the exact answer), `SolveOnly`
//!   (solve and return, never cache: a frozen cache), `Deferred` (return the nearest cached region's
//!   law at once, queue θ; [`EtCache::flush`] builds the queued regions later, as partial
//!   enumeration does), `Report` (return status `Miss`; the caller solves elsewhere and may hand the
//!   active set back with [`EtCache::insert_active_set`]).
//! * cache limit: `max_regions` with LRU or LFU eviction ([`Eviction`]).
//! * parallelism: `parallel` runs the batch lookups, the solves of a batch's misses and the lazy
//!   row reductions on the rayon pool.
//! * basis rule ([`BasisRule`]): which optimal basis a degenerate θ gets: the lexicographic one
//!   (the default for an mpLP), the solver's (the default for an mpQP), or Pareto-optimal
//!   multipliers for Benders-type cuts.
//!
//! # Locating θ in a growing cache
//!
//! Cheapest test first:
//! 1. **Stream memory.** A query may name a stream (a scenario, a controller, any sequence whose θ
//!    moves gradually). The stream remembers its last region, a reference point θ_ref in it, and the
//!    slack s_i = f_i - e_i θ_ref of every row (rows are unit norm, so s_i is the distance to facet
//!    i). This is how parameters that change slowly or not at all save work, with no labelling:
//!    * *safe-ball test*: with Δ = θ - θ_ref, every row moves by at most
//!      min(|Δ|₂, Σ_k w_k |Δ_k|) (w_k = largest |e_ik| of the region). If that bound is at most
//!      min_i s_i, θ is inside: O(n_θ) work and no row is touched. Small moves of continuous
//!      parameters end here.
//!    * *sparse slack update*: otherwise only the coordinates with Δ_k != 0 enter,
//!      s_i -= Σ_{k changed} e_ik Δ_k. Coordinates that did not change (scenario data, set points)
//!      cost nothing. The slacks are recomputed in full every few updates to bound rounding drift.
//! 2. **Learned transitions.** Every region counts the regions its streams moved to next; those
//!    successors are tested next.
//! 3. **Hot list.** All regions in order of hit count (re-sorted periodically; a new region goes to
//!    the front), each test stopping at the first violated row.
//!
//! # Building a region on a miss
//!
//! The active set comes from the solver's answer at θ. For an mpLP it is completed to a basis
//! (equalities, the rows with a nonzero multiplier, then tight rows by Gram-Schmidt until n_x
//! independent rows); a primal degenerate vertex has more tight rows than n_x, and the full tight
//! set has no unique law. By default this basis only starts a few lexicographic pivots to the
//! lex-optimal basis, which is unique at every θ, so the cached multipliers do not depend on how
//! a solver breaks ties (they decide the Benders cuts of a degenerate subproblem). If a region
//! cannot be built, the solver's completion and two other orders of the tight rows are tried. For
//! an mpQP it is the equalities plus the rows with a positive multiplier. The region is inserted with its rows unreduced (only zero rows and rows that cannot
//! bind inside the parameter bounding box are dropped; no LP); once it has `reduce_after` hits its
//! redundant rows are removed exactly (`gen_cr_from_active_set`), so only hot regions pay for it.

use crate::critical_region::CriticalRegion;
use crate::fxhash::FxHashMap;
use crate::linalg::{dot, norm, Mat};
use crate::program::MpProgram;
use crate::solver::SolverOutput;
use rayon::prelude::*;
use std::sync::Arc;

/// What a query outside every cached region does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissPolicy {
    /// Solve at θ, build and cache the region, return the exact answer.
    Exact,
    /// Solve at θ and return the exact answer; the cache is not changed.
    SolveOnly,
    /// Return the law of the cached region nearest θ (status `Approx`, or `Miss` with an empty
    /// cache) and queue θ; [`EtCache::flush`] solves the queue and caches the regions.
    Deferred,
    /// Return status `Miss` without solving.
    Report,
}

impl MissPolicy {
    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
            "exact" => Some(Self::Exact),
            "solve_only" | "solve" | "no_cache" => Some(Self::SolveOnly),
            "deferred" | "defer" => Some(Self::Deferred),
            "report" | "none" => Some(Self::Report),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::SolveOnly => "solve_only",
            Self::Deferred => "deferred",
            Self::Report => "report",
        }
    }
}

/// Which region leaves a full cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eviction {
    /// least recently hit
    Lru,
    /// fewest hits (ties: least recently hit)
    Lfu,
}

impl Eviction {
    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "lru" => Some(Self::Lru),
            "lfu" => Some(Self::Lfu),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Lru => "lru",
            Self::Lfu => "lfu",
        }
    }
}

#[derive(Clone, Debug)]
pub struct EtOptions {
    pub miss: MissPolicy,
    /// Largest number of cached regions (None: unbounded).
    pub max_regions: Option<usize>,
    pub eviction: Eviction,
    /// Use the rayon pool for batch lookups, miss solves and row reductions.
    pub parallel: bool,
    /// Point location tolerance on the unit-norm rows (a distance in θ space). A new region must
    /// contain its θ within it, or it is not cached.
    pub tol: f64,
    /// Remove the redundant rows of a region once it has this many hits (None: never).
    pub reduce_after: Option<u64>,
    /// Cache only full-dimensional regions (one Chebyshev LP per new region).
    pub check_full_dim: bool,
    /// Which optimal basis a degenerate θ gets (see [`BasisRule`]).
    pub basis: BasisRule,
}

impl Default for EtOptions {
    fn default() -> Self {
        EtOptions {
            miss: MissPolicy::Exact,
            max_regions: None,
            eviction: Eviction::Lru,
            parallel: true,
            tol: 1e-8,
            reduce_after: Some(16),
            check_full_dim: true,
            basis: BasisRule::Auto,
        }
    }
}

/// Which optimal basis (active set) a θ gets when several are optimal (a degenerate θ). Every
/// choice gives the same optimal objective, and for an mpQP the same x; they differ in the
/// multipliers (hence in the Benders cuts built from them), and for an mpLP with several optimal x
/// in x itself. The event-triggered cache makes the choice matter more than in a single solve: the
/// first region built at a degenerate θ answers every later query it contains.
#[derive(Clone, Debug, PartialEq)]
pub enum BasisRule {
    /// The default: `Lex` for an mpLP, `Solver` for an mpQP (whose optimiser is unique).
    Auto,
    /// The solver's answer: equalities, rows with a positive multiplier, then tight rows.
    Solver,
    /// mpLP: the lexicographically optimal basis (Jones, Kerrigan & Maciejowski 2007), unique at
    /// every θ; its regions do not overlap and the optimiser is continuous where θ enters only the
    /// right hand side. A few lexicographic pivots from the solver's basis per miss.
    Lex,
    /// Pareto-optimal multipliers (Magnanti & Wong 1981) for cuts in the θ coordinates `coords`,
    /// which must enter the right hand side only (φ convex in them): among the optimal multipliers,
    /// those whose cut  φ(θ̂) + g'(θ - θ̂)  (g = -F[:, coords]' λ) is highest at the core point
    /// (θ̂ with θ[coords] = `core`, chosen in the relative interior of where θ[coords] can go).
    /// On a miss: one LP over the optimal dual face. On a hit at a degenerate θ (on the boundary of
    /// the region found first): every cached region containing θ is scored and the best answers.
    Pareto { coords: Vec<usize>, core: Vec<f64> },
}

impl BasisRule {
    pub fn name(&self) -> &'static str {
        match self {
            BasisRule::Auto => "auto",
            BasisRule::Solver => "solver",
            BasisRule::Lex => "lex",
            BasisRule::Pareto { .. } => "pareto",
        }
    }

    /// The rule in effect for `program` (`Auto` resolved).
    pub fn resolve(&self, program: &MpProgram) -> BasisRule {
        match self {
            BasisRule::Auto if program.q.is_none() => BasisRule::Lex,
            BasisRule::Auto => BasisRule::Solver,
            r => r.clone(),
        }
    }

    /// Checks the rule against the program.
    pub fn validate(&self, program: &MpProgram) -> Result<(), String> {
        match self {
            BasisRule::Auto | BasisRule::Solver => Ok(()),
            BasisRule::Lex => {
                if program.q.is_some() {
                    Err("basis rule 'lex' applies to an mpLP (an mpQP has a unique optimiser)".into())
                } else {
                    Ok(())
                }
            }
            BasisRule::Pareto { coords, core } => {
                let nt = program.num_t();
                if coords.is_empty() || coords.len() != core.len() {
                    return Err(format!("pareto: {} cut coordinates and {} core values", coords.len(), core.len()));
                }
                let mut seen = vec![false; nt];
                for &k in coords {
                    if k >= nt || seen[k] {
                        return Err(format!("pareto: cut coordinate {k} is out of range or repeated (n_theta = {nt})"));
                    }
                    seen[k] = true;
                }
                if coords.iter().any(|&k| (0..program.h.rows).any(|r| program.h[(r, k)] != 0.0)) || coords.iter().any(|&k| program.c_t.get(k).is_some_and(|v| *v != 0.0)) {
                    return Err("pareto: a cut coordinate enters the objective; cut coordinates must enter the right hand side only".into());
                }
                if core.iter().any(|v| !v.is_finite()) {
                    return Err("pareto: the core point must be finite".into());
                }
                Ok(())
            }
        }
    }
}

/// How a query was answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    /// a cached region contains θ
    Hit = 0,
    /// miss: solved, the new region is cached
    Built = 1,
    /// miss: solved, nothing cached (policy `SolveOnly`, or the region could not be built)
    Solved = 2,
    /// miss: the nearest cached region's law, extrapolated (policy `Deferred`)
    Approx = 3,
    /// miss: no answer (policy `Report`, or `Deferred` with an empty cache)
    Miss = 4,
    /// θ is infeasible (the solver found no solution)
    Infeasible = 5,
}

impl Status {
    pub const NAMES: [&'static str; 6] = ["hit", "built", "solved", "approx", "miss", "infeasible"];

    pub fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }
}

/// Answer to one query. `x` (n_x) and `duals` (one per constraint of the program, zero for
/// inactive rows) are NaN when there is no answer (`Miss`, `Infeasible`).
#[derive(Clone, Debug)]
pub struct Answer {
    pub status: Status,
    /// id of the region that answered (hit, built or approx); ids are never reused
    pub region: Option<u64>,
    pub x: Vec<f64>,
    pub duals: Vec<f64>,
    pub objective: f64,
}

#[derive(Clone, Debug, Default)]
pub struct EtStats {
    pub queries: u64,
    pub hits: u64,
    /// hits settled by the safe-ball test of the stream's region
    pub ball_hits: u64,
    /// hits in the stream's region after the sparse slack update
    pub stream_hits: u64,
    /// hits in a learned successor of the stream's region
    pub successor_hits: u64,
    /// hits found by the hot-list scan
    pub scan_hits: u64,
    /// regions tested (beyond the safe-ball test) over all queries
    pub regions_tested: u64,
    pub built: u64,
    /// solved misses whose active set was already cached (its region did not contain θ within tol)
    pub duplicates: u64,
    pub solved: u64,
    pub approx: u64,
    pub reported: u64,
    pub infeasible: u64,
    pub build_failures: u64,
    pub evictions: u64,
    pub reductions: u64,
    /// misses where the basis rule (lex or pareto) failed and the solver's basis was used
    pub rule_fallbacks: u64,
    /// pareto: degenerate hits answered by another region than the one found first
    pub pareto_switches: u64,
}

/// Why a region was not built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildError {
    /// no basis / independent active set, or a singular KKT system
    Rank,
    /// a zero row with a negative right hand side
    Empty,
    /// θ lies outside the region by more than the tolerance
    NotContaining,
    LowerDimensional,
}

const NSUCC: usize = 4;
/// full slack recomputation after this many sparse updates
const REBASE_LIMIT: u32 = 32;
/// smallest batch that is looked up on the rayon pool
const PAR_MIN: usize = 64;

struct Region {
    uid: u64,
    active_set: Vec<usize>,
    nrows: usize,
    /// row-major [e_i | f_i], unit-norm e_i
    rows: Vec<f64>,
    /// per coordinate: largest |e_ik| over the rows
    wmax: Vec<f64>,
    a_x: Mat,
    b_x: Vec<f64>,
    a_l: Mat,
    b_l: Vec<f64>,
    hits: u64,
    last_hit: u64,
    reduced: bool,
    rows_ver: u32,
    /// (slot, generation, count) of the regions streams moved to next, by count
    succ: [(u32, u32, u32); NSUCC],
}

impl Region {
    #[inline]
    fn row(&self, i: usize, w: usize) -> &[f64] {
        &self.rows[i * w..(i + 1) * w]
    }

    /// Slacks f - eθ of every row into `buf`; false at the first row violated by more than `tol`
    /// (or not a number).
    #[inline]
    fn slacks(&self, theta: &[f64], tol: f64, buf: &mut Vec<f64>) -> bool {
        let n = theta.len();
        buf.clear();
        for i in 0..self.nrows {
            let r = self.row(i, n + 1);
            let s = r[n] - dot(&r[..n], theta);
            if !(s >= -tol) {
                return false;
            }
            buf.push(s);
        }
        true
    }

    /// Largest violation eθ - f over the rows.
    fn max_violation(&self, theta: &[f64]) -> f64 {
        let n = theta.len();
        (0..self.nrows).map(|i| {
            let r = self.row(i, n + 1);
            dot(&r[..n], theta) - r[n]
        }).fold(f64::NEG_INFINITY, f64::max)
    }

    fn set_rows(&mut self, rows: Vec<f64>, nt: usize) {
        self.nrows = rows.len() / (nt + 1);
        self.wmax = vec![0.0; nt];
        for i in 0..self.nrows {
            for k in 0..nt {
                self.wmax[k] = self.wmax[k].max(rows[i * (nt + 1) + k].abs());
            }
        }
        self.rows = rows;
    }
}

#[derive(Clone)]
struct StreamState {
    slot: u32,
    epoch: u32,
    rows_ver: u32,
    theta_ref: Vec<f64>,
    slack: Vec<f64>,
    min_slack: f64,
    rebases: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum How {
    Ball,
    Stream,
    Successor,
    Scan,
    /// the query's own miss built the region (not a cache hit)
    Built,
}

struct Lookup {
    found: Option<(u32, How)>,
    /// replacement stream state (None: keep the current one)
    state: Option<StreamState>,
    tested: u32,
    /// pareto: the region found first was replaced by a better-scoring one
    switched: bool,
}

/// Solver result plus (optionally) the region built from it.
enum Solved {
    Infeasible,
    Answer { x: Vec<f64>, duals: Vec<f64>, objective: f64, region: Option<Result<Region, BuildError>>, fallback: bool },
}

/// The event-triggered cache. See the module documentation.
pub struct EtCache {
    program: Arc<MpProgram>,
    opts: EtOptions,
    nt: usize,
    nx: usize,
    m: usize,
    slots: Vec<Option<Region>>,
    gens: Vec<u32>,
    free: Vec<u32>,
    live: usize,
    /// live slots, most hit first
    order: Vec<u32>,
    hits_since_sort: u64,
    by_set: FxHashMap<Vec<usize>, u32>,
    streams: FxHashMap<u64, StreamState>,
    pending: Vec<Vec<f64>>,
    next_uid: u64,
    tick: u64,
    stats: EtStats,
}

impl EtCache {
    /// Panics if the basis rule does not fit the program; [`EtCache::try_new`] returns the error.
    pub fn new(program: Arc<MpProgram>, opts: EtOptions) -> Self {
        Self::try_new(program, opts).unwrap_or_else(|e| panic!("{e}"))
    }

    pub fn try_new(program: Arc<MpProgram>, mut opts: EtOptions) -> Result<Self, String> {
        if program.num_t() == 0 {
            return Err("the program has no parameters".into());
        }
        opts.basis.validate(&program)?;
        opts.basis = opts.basis.resolve(&program);
        Ok(Self::build(program, opts))
    }

    fn build(program: Arc<MpProgram>, opts: EtOptions) -> Self {
        let (nt, nx, m) = (program.num_t(), program.num_x(), program.num_constraints());
        // the bounding box is used by every region build; compute it once, outside the hot path
        let _ = program.theta_bbox();
        EtCache {
            program,
            opts,
            nt,
            nx,
            m,
            slots: vec![],
            gens: vec![],
            free: vec![],
            live: 0,
            order: vec![],
            hits_since_sort: 0,
            by_set: FxHashMap::default(),
            streams: FxHashMap::default(),
            pending: vec![],
            next_uid: 0,
            tick: 0,
            stats: EtStats::default(),
        }
    }

    pub fn program(&self) -> &Arc<MpProgram> {
        &self.program
    }

    pub fn options(&self) -> &EtOptions {
        &self.opts
    }

    pub fn set_miss_policy(&mut self, miss: MissPolicy) {
        self.opts.miss = miss;
    }

    pub fn set_parallel(&mut self, parallel: bool) {
        self.opts.parallel = parallel;
    }

    /// Changes the limit; a smaller limit evicts at once.
    pub fn set_max_regions(&mut self, max_regions: Option<usize>) {
        self.opts.max_regions = max_regions;
        if let Some(cap) = max_regions {
            while self.live > cap.max(1) {
                self.evict_one();
            }
        }
    }

    pub fn len(&self) -> usize {
        self.live
    }

    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    pub fn num_pending(&self) -> usize {
        self.pending.len()
    }

    pub fn stats(&self) -> &EtStats {
        &self.stats
    }

    /// Total rows over the cached regions (online memory is about rows * (n_θ + 1) floats).
    pub fn total_rows(&self) -> usize {
        self.slots.iter().flatten().map(|r| r.nrows).sum()
    }

    /// Forgets every region, stream and queued θ (statistics are kept).
    pub fn clear(&mut self) {
        for g in self.gens.iter_mut() {
            *g = g.wrapping_add(1);
        }
        self.free = (0..self.slots.len() as u32).rev().collect();
        for s in self.slots.iter_mut() {
            *s = None;
        }
        self.live = 0;
        self.order.clear();
        self.by_set.clear();
        self.streams.clear();
        self.pending.clear();
    }

    /// Forgets the stream memories only.
    pub fn clear_streams(&mut self) {
        self.streams.clear();
    }

    // -------------------------------------------------------------------------------- queries

    /// One query. `stream` names the sequence θ belongs to (see the module documentation).
    pub fn query(&mut self, theta: &[f64], stream: Option<u64>) -> Answer {
        let streams = stream.map(|s| [s]);
        self.query_batch(theta, streams.as_ref().map(|s| &s[..])).pop().expect("one answer")
    }

    /// Queries the rows of `thetas` (row-major, n_θ entries per query). `streams`, if given, has
    /// one id per query. The misses of a batch are solved in rounds (1, 2, 4, .. solves, up to the
    /// thread count; one at a time with `parallel` off) and the remaining misses are re-tested
    /// against each round's new regions, so that misses in one region are rarely solved twice.
    pub fn query_batch(&mut self, thetas: &[f64], streams: Option<&[u64]>) -> Vec<Answer> {
        let nt = self.nt;
        assert!(nt > 0 && thetas.len() % nt == 0, "thetas must hold n_theta = {nt} entries per query");
        let n = thetas.len() / nt;
        if let Some(s) = streams {
            assert_eq!(s.len(), n, "one stream id per query");
        }
        let th = |i: usize| &thetas[i * nt..(i + 1) * nt];
        self.stats.queries += n as u64;

        // a non-finite θ is answered `Infeasible` without a lookup (a NaN slack would pass every
        // containment test, and a stream state built from it would accept every later query)
        let finite = |i: usize| th(i).iter().all(|v| v.is_finite());

        // 1. lookups (read only)
        let looks: Vec<Lookup> = {
            let me = &*self;
            let f = |i: usize| {
                if !finite(i) {
                    return Lookup { found: None, state: None, tested: 0, switched: false };
                }
                me.lookup(th(i), streams.and_then(|s| me.streams.get(&s[i])), streams.is_some())
            };
            if me.opts.parallel && n >= PAR_MIN { (0..n).into_par_iter().map(f).collect() } else { (0..n).map(f).collect() }
        };

        // 2. hits, in query order
        let mut answers: Vec<Option<Answer>> = (0..n).map(|_| None).collect();
        let mut misses = vec![];
        let mut to_reduce = vec![];
        for (i, lk) in looks.into_iter().enumerate() {
            self.stats.regions_tested += lk.tested as u64;
            if lk.switched {
                self.stats.pareto_switches += 1;
            }
            match lk.found {
                Some((slot, how)) => {
                    self.record_hit(slot, how, streams.map(|s| s[i]), lk.state, &mut to_reduce);
                    answers[i] = Some(self.answer_from(slot, th(i), Status::Hit));
                }
                None if !finite(i) => {
                    self.stats.infeasible += 1;
                    answers[i] = Some(self.no_answer(Status::Infeasible));
                }
                None => misses.push(i),
            }
        }

        // 3. misses
        if !misses.is_empty() {
            match self.opts.miss {
                MissPolicy::Exact => self.misses_exact(thetas, streams, misses, &mut answers, &mut to_reduce),
                MissPolicy::SolveOnly => {
                    let outs = self.solve_many(thetas, &misses, false);
                    for (&i, out) in misses.iter().zip(outs) {
                        answers[i] = Some(self.answer_unbuilt(out));
                    }
                }
                MissPolicy::Deferred => {
                    for &i in &misses {
                        answers[i] = Some(self.approx(th(i)));
                        self.pending.push(th(i).to_vec());
                    }
                }
                MissPolicy::Report => {
                    for &i in &misses {
                        self.stats.reported += 1;
                        answers[i] = Some(self.no_answer(Status::Miss));
                    }
                }
            }
        }

        // 4. housekeeping
        self.reduce(to_reduce);
        if self.hits_since_sort >= (self.live as u64).max(256) {
            self.resort();
        }
        answers.into_iter().map(|a| a.expect("every query answered")).collect()
    }

    fn misses_exact(&mut self, thetas: &[f64], streams: Option<&[u64]>, mut pending: Vec<usize>, answers: &mut [Option<Answer>], to_reduce: &mut Vec<u32>) {
        let nt = self.nt;
        let th = |i: usize| &thetas[i * nt..(i + 1) * nt];
        // round width 1, 2, 4, .. up to the thread count: misses of a batch often share regions,
        // and a small first round lets the re-test below answer them without a solve
        let max_width = if self.opts.parallel { rayon::current_num_threads().max(1) } else { 1 };
        let mut width = 1;
        let mut buf = vec![];
        while !pending.is_empty() {
            let take: Vec<usize> = pending.drain(..width.min(pending.len())).collect();
            width = (width * 2).min(max_width);
            let outs = self.solve_many(thetas, &take, true);
            let mut new_slots: Vec<(u32, u32)> = vec![];
            for (&i, out) in take.iter().zip(outs) {
                let Solved::Answer { x, duals, objective, region, fallback } = out else {
                    self.stats.infeasible += 1;
                    answers[i] = Some(self.no_answer(Status::Infeasible));
                    continue;
                };
                if fallback {
                    self.stats.rule_fallbacks += 1;
                }
                let mut ans = Answer { status: Status::Solved, region: None, x, duals, objective };
                match region {
                    Some(Ok(r)) => {
                        let (slot, fresh) = self.insert(r);
                        if fresh {
                            self.stats.built += 1;
                            new_slots.push((slot, self.gens[slot as usize]));
                        } else {
                            self.stats.duplicates += 1;
                        }
                        let reg = self.slots[slot as usize].as_ref().unwrap();
                        if reg.slacks(th(i), self.opts.tol, &mut buf) {
                            let state = self.fresh_state(slot, th(i), &buf);
                            self.record_hit(slot, How::Built, streams.map(|s| s[i]), Some(state), to_reduce);
                            // the answer of the cached basis (its multipliers are the ones the basis rule
                            // chose, and every later hit in the region gets the same law)
                            ans = self.answer_from(slot, th(i), Status::Built);
                        } else {
                            self.stats.solved += 1;
                        }
                    }
                    Some(Err(_)) | None => {
                        self.stats.build_failures += 1;
                        self.stats.solved += 1;
                    }
                }
                answers[i] = Some(ans);
            }
            if new_slots.is_empty() {
                continue;
            }
            // later misses of the batch that the new regions cover are hits now
            let mut left = vec![];
            for i in pending {
                let mut hit = None;
                let mut tested = 0;
                for &(slot, epoch) in &new_slots {
                    if let Some(r) = self.live_region(slot, epoch) {
                        tested += 1;
                        if r.slacks(th(i), self.opts.tol, &mut buf) {
                            hit = Some(slot);
                            break;
                        }
                    }
                }
                self.stats.regions_tested += tested;
                match hit {
                    Some(slot) => {
                        let state = self.fresh_state(slot, th(i), &buf);
                        // the basis rule's tie-break applies here as in a lookup
                        let lk = self.pareto_adjust(th(i), true, Lookup { found: Some((slot, How::Scan)), state: Some(state), tested: 0, switched: false });
                        if lk.switched {
                            self.stats.pareto_switches += 1;
                        }
                        self.stats.regions_tested += lk.tested as u64;
                        let (slot, _) = lk.found.expect("a region was found");
                        self.record_hit(slot, How::Scan, streams.map(|s| s[i]), lk.state, to_reduce);
                        answers[i] = Some(self.answer_from(slot, th(i), Status::Hit));
                    }
                    None => left.push(i),
                }
            }
            pending = left;
        }
    }

    /// Solves the queued θ of the `Deferred` policy (in parallel when enabled) and caches their
    /// regions. Returns the number of new regions.
    pub fn flush(&mut self) -> usize {
        let queue = std::mem::take(&mut self.pending);
        if queue.is_empty() {
            return 0;
        }
        let nt = self.nt;
        let flat: Vec<f64> = queue.concat();
        let n = flat.len() / nt;
        let th = |j: usize| &flat[j * nt..(j + 1) * nt];
        let width = if self.opts.parallel { rayon::current_num_threads().max(1) } else { 1 };
        let mut added = 0;
        let mut next = 0;
        // rounds, as for a batch: a queued θ that an earlier round's region covers is not solved
        while next < n {
            let mut take = vec![];
            while next < n && take.len() < width {
                if self.lookup(th(next), None, false).found.is_none() {
                    take.push(next);
                }
                next += 1;
            }
            for out in self.solve_many(&flat, &take, true) {
                match out {
                    Solved::Answer { region: Some(Ok(r)), .. } => {
                        let (_, fresh) = self.insert(r);
                        if fresh {
                            self.stats.built += 1;
                            added += 1;
                        } else {
                            self.stats.duplicates += 1;
                        }
                    }
                    Solved::Answer { .. } => self.stats.build_failures += 1,
                    Solved::Infeasible => {}
                }
            }
        }
        added
    }

    /// Solves at θ and caches the region, whatever the miss policy. Returns the region id, or
    /// None if θ is infeasible or the region could not be built.
    pub fn insert_theta(&mut self, theta: &[f64]) -> Option<u64> {
        assert_eq!(theta.len(), self.nt);
        if theta.iter().any(|v| !v.is_finite()) {
            return None;
        }
        match solve_and_build(&self.program, &self.opts, theta, true) {
            Solved::Answer { region: Some(Ok(r)), .. } => {
                let (slot, fresh) = self.insert(r);
                if fresh {
                    self.stats.built += 1;
                } else {
                    self.stats.duplicates += 1;
                }
                self.slots[slot as usize].as_ref().map(|r| r.uid)
            }
            Solved::Answer { .. } => {
                self.stats.build_failures += 1;
                None
            }
            Solved::Infeasible => None,
        }
    }

    /// Caches the region of an active set obtained elsewhere (for example the optimal basis from
    /// an external LP solver). Indices refer to the constraints of the program as processed; the
    /// equality indices are added if missing. Returns the region id.
    pub fn insert_active_set(&mut self, active_set: &[usize]) -> Result<u64, BuildError> {
        let mut set: Vec<usize> = active_set.to_vec();
        if set.iter().any(|&i| i >= self.m) {
            return Err(BuildError::Rank);
        }
        set.extend(self.program.equality_indices.iter().copied());
        set.sort_unstable();
        set.dedup();
        let r = build_region(&self.program, &self.opts, &set, None)?;
        let (slot, fresh) = self.insert(r);
        if fresh {
            self.stats.built += 1;
        } else {
            self.stats.duplicates += 1;
        }
        Ok(self.slots[slot as usize].as_ref().unwrap().uid)
    }

    /// The cached regions as `CriticalRegion`s (by id), e.g. to build an offline point locator or
    /// to save them. Facet bookkeeping (`omega_set`, `lambda_set`, `regular_set`) is left empty.
    pub fn regions(&self) -> Vec<(u64, CriticalRegion)> {
        let nt = self.nt;
        let mut out: Vec<(u64, CriticalRegion)> = self
            .slots
            .iter()
            .flatten()
            .map(|r| {
                let mut e = Mat::zeros(r.nrows, nt);
                let mut f = Vec::with_capacity(r.nrows);
                for i in 0..r.nrows {
                    let row = r.row(i, nt + 1);
                    e.row_mut(i).copy_from_slice(&row[..nt]);
                    f.push(row[nt]);
                }
                let cr = CriticalRegion {
                    a: r.a_x.clone(),
                    b: r.b_x.clone(),
                    c: r.a_l.clone(),
                    d: r.b_l.clone(),
                    e,
                    f,
                    active_set: r.active_set.clone(),
                    omega_set: vec![],
                    lambda_set: vec![],
                    regular_set: (vec![], vec![]),
                    y_fixation: None,
                    y_indices: None,
                    x_indices: None,
                    cheb_radius: None,
                    facet_single: vec![],
                };
                (r.uid, cr)
            })
            .collect();
        out.sort_by_key(|(u, _)| *u);
        out
    }

    /// Hit count of every cached region, by id.
    pub fn region_hits(&self) -> Vec<(u64, u64)> {
        let mut v: Vec<(u64, u64)> = self.slots.iter().flatten().map(|r| (r.uid, r.hits)).collect();
        v.sort_by_key(|(u, _)| *u);
        v
    }

    // -------------------------------------------------------------------------------- lookup

    #[inline]
    fn live_region(&self, slot: u32, epoch: u32) -> Option<&Region> {
        let s = slot as usize;
        if s < self.slots.len() && self.gens[s] == epoch { self.slots[s].as_ref() } else { None }
    }

    fn fresh_state(&self, slot: u32, theta: &[f64], slack: &[f64]) -> StreamState {
        let r = self.slots[slot as usize].as_ref().unwrap();
        StreamState {
            slot,
            epoch: self.gens[slot as usize],
            rows_ver: r.rows_ver,
            theta_ref: theta.to_vec(),
            slack: slack.to_vec(),
            min_slack: slack.iter().cloned().fold(f64::INFINITY, f64::min),
            rebases: 0,
        }
    }

    /// `lookup_first`, then (basis rule pareto) the tie-break among the regions containing a
    /// degenerate θ. `has_stream`: the query names a stream (a hit then returns a stream state
    /// even when the stream has none yet).
    fn lookup(&self, theta: &[f64], st: Option<&StreamState>, has_stream: bool) -> Lookup {
        let lk = self.lookup_first(theta, st, has_stream);
        self.pareto_adjust(theta, has_stream, lk)
    }

    /// The pareto tie-break on a found region: if θ lies on its boundary, every cached region
    /// containing θ is scored at the core point and the best one answers.
    fn pareto_adjust(&self, theta: &[f64], has_stream: bool, mut lk: Lookup) -> Lookup {
        let BasisRule::Pareto { coords, core } = &self.opts.basis else { return lk };
        let Some((slot, _)) = lk.found else { return lk };
        // a θ in the interior of the region found holds no other optimal basis of a primal
        // degenerate program; only boundary points (within a few tolerances) are scored
        let r = self.slots[slot as usize].as_ref().unwrap();
        let tie = 10.0 * self.opts.tol.max(1e-10);
        let n = self.nt;
        let interior = (0..r.nrows).all(|i| {
            let row = r.row(i, n + 1);
            row[n] - dot(&row[..n], theta) > tie
        });
        if interior {
            return lk;
        }
        let delta: Vec<f64> = coords.iter().zip(core).map(|(&k, c)| c - theta[k]).collect();
        let mut best = (slot, self.pareto_score(r, theta, coords, &delta));
        let mut buf = Vec::new();
        let mut best_buf: Option<Vec<f64>> = None;
        for &sl in &self.order {
            if sl == slot {
                continue;
            }
            let rr = self.slots[sl as usize].as_ref().unwrap();
            lk.tested += 1;
            if rr.slacks(theta, self.opts.tol, &mut buf) {
                let score = self.pareto_score(rr, theta, coords, &delta);
                if score > best.1 + 1e-9 * (1.0 + best.1.abs()) {
                    best = (sl, score);
                    best_buf = Some(buf.clone());
                }
            }
        }
        if let Some(sl) = best_buf.map(|b| (best.0, b)) {
            lk.state = has_stream.then(|| self.fresh_state(sl.0, theta, &sl.1));
            lk.found = Some((sl.0, How::Scan));
            lk.switched = true;
        }
        lk
    }

    /// Cut gain  g'(core - θ)  of a region's multipliers at θ, with  g = -F[:, coords]' λ(θ).
    fn pareto_score(&self, r: &Region, theta: &[f64], coords: &[usize], delta: &[f64]) -> f64 {
        let f = &self.program.f;
        let mut s = 0.0;
        for (ii, &i) in r.active_set.iter().enumerate() {
            let fd: f64 = coords.iter().zip(delta).map(|(&k, d)| f[(i, k)] * d).sum();
            if fd != 0.0 {
                s -= (dot(r.a_l.row(ii), theta) + r.b_l[ii]) * fd;
            }
        }
        s
    }

    fn lookup_first(&self, theta: &[f64], st: Option<&StreamState>, has_stream: bool) -> Lookup {
        let tol = self.opts.tol;
        let nt = self.nt;
        let mut tested = 0u32;
        let mut tried: [u32; NSUCC + 1] = [u32::MAX; NSUCC + 1];
        let mut buf = Vec::new();
        if let Some(s) = st {
            if let Some(r) = self.live_region(s.slot, s.epoch) {
                tried[0] = s.slot;
                tested += 1;
                let same_rows = s.rows_ver == r.rows_ver;
                if same_rows {
                    // safe ball around θ_ref
                    let mut d2 = 0.0;
                    let mut wsum = 0.0;
                    for k in 0..nt {
                        let d = theta[k] - s.theta_ref[k];
                        d2 += d * d;
                        wsum += r.wmax[k] * d.abs();
                    }
                    if d2.sqrt().min(wsum) <= s.min_slack + tol {
                        return Lookup { found: Some((s.slot, How::Ball)), state: None, tested: 0, switched: false };
                    }
                }
                // slacks at θ: sparse update over the changed coordinates, or in full
                let changed: Vec<usize> = (0..nt).filter(|&k| theta[k] != s.theta_ref[k]).collect();
                let w = nt + 1;
                let (ok, slack, rebases) = if same_rows && changed.len() * 2 <= nt && s.rebases < REBASE_LIMIT {
                    let mut sl = s.slack.clone();
                    let mut ok = true;
                    for (i, v) in sl.iter_mut().enumerate() {
                        let row = r.row(i, w);
                        for &k in &changed {
                            *v -= row[k] * (theta[k] - s.theta_ref[k]);
                        }
                        if !(*v >= -tol) {
                            ok = false;
                            break;
                        }
                    }
                    (ok, sl, s.rebases + 1)
                } else {
                    let ok = r.slacks(theta, tol, &mut buf);
                    (ok, std::mem::take(&mut buf), 0)
                };
                if ok {
                    let min_slack = slack.iter().cloned().fold(f64::INFINITY, f64::min);
                    let state = StreamState { slot: s.slot, epoch: s.epoch, rows_ver: r.rows_ver, theta_ref: theta.to_vec(), slack, min_slack, rebases };
                    return Lookup { found: Some((s.slot, How::Stream)), state: Some(state), tested, switched: false };
                }
                // learned successors
                for (j, &(sl, g, cnt)) in r.succ.iter().enumerate() {
                    if cnt == 0 {
                        break;
                    }
                    if let Some(rr) = self.live_region(sl, g) {
                        tried[j + 1] = sl;
                        tested += 1;
                        if rr.slacks(theta, tol, &mut buf) {
                            return Lookup { found: Some((sl, How::Successor)), state: Some(self.fresh_state(sl, theta, &buf)), tested, switched: false };
                        }
                    }
                }
            }
        }
        for &sl in &self.order {
            if tried.contains(&sl) {
                continue;
            }
            tested += 1;
            let r = self.slots[sl as usize].as_ref().unwrap();
            if r.slacks(theta, tol, &mut buf) {
                let state = has_stream.then(|| self.fresh_state(sl, theta, &buf));
                return Lookup { found: Some((sl, How::Scan)), state, tested, switched: false };
            }
        }
        Lookup { found: None, state: None, tested, switched: false }
    }

    fn record_hit(&mut self, slot: u32, how: How, stream: Option<u64>, state: Option<StreamState>, to_reduce: &mut Vec<u32>) {
        self.tick += 1;
        self.hits_since_sort += 1;
        if how != How::Built {
            self.stats.hits += 1;
        }
        match how {
            How::Ball => self.stats.ball_hits += 1,
            How::Stream => self.stats.stream_hits += 1,
            How::Successor => self.stats.successor_hits += 1,
            How::Scan => self.stats.scan_hits += 1,
            How::Built => {}
        }
        let epoch = self.gens[slot as usize];
        if let Some(id) = stream {
            let prev = self.streams.get(&id).map(|s| (s.slot, s.epoch));
            if let Some((ps, pg)) = prev {
                if ps != slot && self.live_region(ps, pg).is_some() {
                    let p = self.slots[ps as usize].as_mut().unwrap();
                    bump_successor(&mut p.succ, slot, epoch);
                }
            }
            if let Some(s) = state {
                self.streams.insert(id, s);
            }
        }
        let tick = self.tick;
        let r = self.slots[slot as usize].as_mut().unwrap();
        r.hits += 1;
        r.last_hit = tick;
        if let Some(k) = self.opts.reduce_after {
            if !r.reduced && r.hits >= k && !to_reduce.contains(&slot) {
                to_reduce.push(slot);
            }
        }
    }

    fn resort(&mut self) {
        let slots = &self.slots;
        self.order.sort_by_key(|&s| std::cmp::Reverse(slots[s as usize].as_ref().map_or(0, |r| r.hits)));
        self.hits_since_sort = 0;
    }

    // -------------------------------------------------------------------------------- answers

    fn answer_from(&self, slot: u32, theta: &[f64], status: Status) -> Answer {
        let r = self.slots[slot as usize].as_ref().unwrap();
        let mut x = r.a_x.matvec(theta);
        for (v, b) in x.iter_mut().zip(&r.b_x) {
            *v += b;
        }
        let mut duals = vec![0.0; self.m];
        for (ii, &i) in r.active_set.iter().enumerate() {
            duals[i] = dot(r.a_l.row(ii), theta) + r.b_l[ii];
        }
        let objective = self.program.evaluate_objective(&x, theta);
        Answer { status, region: Some(r.uid), x, duals, objective }
    }

    fn no_answer(&self, status: Status) -> Answer {
        Answer { status, region: None, x: vec![f64::NAN; self.nx], duals: vec![f64::NAN; self.m], objective: f64::NAN }
    }

    fn answer_unbuilt(&mut self, out: Solved) -> Answer {
        match out {
            Solved::Infeasible => {
                self.stats.infeasible += 1;
                self.no_answer(Status::Infeasible)
            }
            Solved::Answer { x, duals, objective, .. } => {
                self.stats.solved += 1;
                Answer { status: Status::Solved, region: None, x, duals, objective }
            }
        }
    }

    /// The law of the cached region that θ violates least.
    fn approx(&mut self, theta: &[f64]) -> Answer {
        let best = self.order.iter().map(|&s| (s, self.slots[s as usize].as_ref().unwrap().max_violation(theta))).min_by(|a, b| a.1.total_cmp(&b.1));
        match best {
            Some((slot, _)) => {
                self.stats.approx += 1;
                self.answer_from(slot, theta, Status::Approx)
            }
            None => {
                self.stats.reported += 1;
                self.no_answer(Status::Miss)
            }
        }
    }

    // -------------------------------------------------------------------------------- storage

    fn solve_many(&self, thetas: &[f64], idx: &[usize], build: bool) -> Vec<Solved> {
        let nt = self.nt;
        let f = |&i: &usize| solve_and_build(&self.program, &self.opts, &thetas[i * nt..(i + 1) * nt], build);
        if self.opts.parallel && idx.len() > 1 { idx.par_iter().map(f).collect() } else { idx.iter().map(f).collect() }
    }

    /// Caches a region; an active set that is already cached keeps its region. Returns the slot
    /// and whether the region is new.
    fn insert(&mut self, mut r: Region) -> (u32, bool) {
        if let Some(&slot) = self.by_set.get(&r.active_set) {
            return (slot, false);
        }
        if let Some(cap) = self.opts.max_regions {
            while self.live >= cap.max(1) {
                self.evict_one();
            }
        }
        self.tick += 1;
        r.uid = self.next_uid;
        self.next_uid += 1;
        r.hits = 0;
        r.last_hit = self.tick;
        let slot = match self.free.pop() {
            Some(s) => s,
            None => {
                self.slots.push(None);
                self.gens.push(0);
                (self.slots.len() - 1) as u32
            }
        };
        self.by_set.insert(r.active_set.clone(), slot);
        self.slots[slot as usize] = Some(r);
        self.live += 1;
        // a new region was needed just now: test it first
        self.order.insert(0, slot);
        (slot, true)
    }

    fn evict_one(&mut self) {
        let lfu = self.opts.eviction == Eviction::Lfu;
        let victim = self
            .order
            .iter()
            .copied()
            .min_by_key(|&s| {
                let r = self.slots[s as usize].as_ref().unwrap();
                if lfu { (r.hits, r.last_hit) } else { (r.last_hit, r.hits) }
            });
        let Some(v) = victim else { return };
        let r = self.slots[v as usize].take().unwrap();
        self.by_set.remove(&r.active_set);
        self.gens[v as usize] = self.gens[v as usize].wrapping_add(1);
        self.free.push(v);
        self.order.retain(|&s| s != v);
        self.live -= 1;
        self.stats.evictions += 1;
    }

    /// Exact redundant-row removal of the regions in `slots` (rows replaced, same polytope).
    fn reduce(&mut self, slots: Vec<u32>) {
        if slots.is_empty() {
            return;
        }
        let nt = self.nt;
        let jobs: Vec<(u32, u32, Vec<usize>)> = slots.iter().filter_map(|&s| self.slots[s as usize].as_ref().map(|r| (s, self.gens[s as usize], r.active_set.clone()))).collect();
        let program = &self.program;
        let f = |(s, g, set): &(u32, u32, Vec<usize>)| (*s, *g, crate::mpqp_utils::gen_cr_from_active_set(program, set, false));
        let built: Vec<(u32, u32, Option<CriticalRegion>)> = if self.opts.parallel && jobs.len() > 1 { jobs.par_iter().map(f).collect() } else { jobs.iter().map(f).collect() };
        for (s, g, cr) in built {
            if self.gens[s as usize] != g {
                continue;
            }
            let Some(r) = self.slots[s as usize].as_mut() else { continue };
            r.reduced = true;
            let Some(cr) = cr else { continue };
            // unit-norm rows, as everywhere in the cache
            let mut rows = Vec::with_capacity(cr.e.rows * (nt + 1));
            for i in 0..cr.e.rows {
                let e = cr.e.row(i);
                let nrm = norm(e);
                if nrm <= 1e-12 {
                    continue;
                }
                rows.extend(e.iter().map(|v| v / nrm));
                rows.push(cr.f[i] / nrm);
            }
            if rows.is_empty() && r.nrows > 0 {
                continue;
            }
            r.set_rows(rows, nt);
            r.rows_ver = r.rows_ver.wrapping_add(1);
            self.stats.reductions += 1;
        }
    }
}

fn bump_successor(succ: &mut [(u32, u32, u32); NSUCC], slot: u32, epoch: u32) {
    if let Some(e) = succ.iter_mut().find(|e| e.2 > 0 && e.0 == slot && e.1 == epoch) {
        e.2 = e.2.saturating_add(1);
    } else {
        // replace the weakest entry
        let k = (0..NSUCC).min_by_key(|&k| succ[k].2).unwrap();
        succ[k] = (slot, epoch, 1);
    }
    succ.sort_by_key(|e| std::cmp::Reverse(e.2));
}

// ------------------------------------------------------------------------------------ building

fn solve_and_build(program: &MpProgram, opts: &EtOptions, theta: &[f64], build: bool) -> Solved {
    let Some(sol) = program.solve_theta_uncounted(theta) else { return Solved::Infeasible };
    let (region, fallback) = if build {
        let (r, f) = region_from_solution(program, opts, &sol, theta);
        (Some(r), f)
    } else {
        (None, false)
    };
    let m = program.num_constraints();
    let mut duals = sol.dual;
    duals.resize(m, 0.0);
    Solved::Answer { x: sol.sol, duals, objective: sol.obj, region, fallback }
}

/// Region of the optimal active set at θ chosen by the basis rule (see "Building a region" in the
/// module documentation). The flag is set when the rule (lex or pareto) failed and the solver's
/// basis was used instead.
fn region_from_solution(program: &MpProgram, opts: &EtOptions, sol: &SolverOutput, theta: &[f64]) -> (Result<Region, BuildError>, bool) {
    let candidates = candidate_sets(program, sol);
    let preferred: Option<Vec<usize>> = match (&opts.basis, program.q.is_none()) {
        (BasisRule::Lex, true) => crate::algorithms::mplp_lex::lex_optimal_basis(program, theta, candidates.first().map(|v| &v[..])),
        (BasisRule::Pareto { coords, core }, true) => pareto_basis(program, sol, theta, coords, core),
        _ => None,
    };
    let ruled = !matches!(opts.basis, BasisRule::Solver) && program.q.is_none();
    if let Some(set) = &preferred {
        if let Ok(r) = build_region(program, opts, set, Some(theta)) {
            return (Ok(r), false);
        }
    }
    let mut last = BuildError::Rank;
    for set in candidates {
        match build_region(program, opts, &set, Some(theta)) {
            Ok(r) => return (Ok(r), ruled),
            Err(e) => last = e,
        }
    }
    (Err(last), ruled)
}

/// Rows of the right hand side direction of a pareto cut:  r_i = -F[i, coords] · (core - θ[coords]),
/// so that  r'λ = g'(core - θ)  for the cut gradient  g = -F[:, coords]' λ.
fn pareto_direction(program: &MpProgram, theta: &[f64], coords: &[usize], core: &[f64]) -> Vec<f64> {
    let delta: Vec<f64> = coords.iter().zip(core).map(|(&k, c)| c - theta[k]).collect();
    (0..program.num_constraints()).map(|i| -coords.iter().zip(&delta).map(|(&k, d)| program.f[(i, k)] * d).sum::<f64>()).collect()
}

/// mpLP basis with Pareto-optimal multipliers at θ (Magnanti & Wong): the LP
///   max r'λ  s.t.  A_T'λ = -(c + Hθ),  λ_i >= 0 (inequality rows of T)
/// over the rows T that are tight at the solver's optimum (complementary slackness keeps λ in the
/// optimal dual face), r from `pareto_direction`. Its vertex has at most n_x independent nonzeros;
/// they go into the basis first, then tight rows complete it. None if the LP fails or is unbounded
/// (the core point lies outside the domain of φ).
fn pareto_basis(program: &MpProgram, sol: &SolverOutput, theta: &[f64], coords: &[usize], core: &[f64]) -> Option<Vec<usize>> {
    let (m, nx) = (program.num_constraints(), program.num_x());
    let eq = &program.equality_indices;
    let mut is_eq = vec![false; m];
    for &i in eq {
        is_eq[i] = true;
    }
    let rhs = {
        let mut v = program.f.matvec(theta);
        for (x, b) in v.iter_mut().zip(&program.b) {
            *x += b;
        }
        v
    };
    let tight: Vec<usize> = (0..m.min(sol.slack.len())).filter(|&i| is_eq[i] || sol.slack[i].abs() <= 1e-9 * (1.0 + rhs[i].abs())).collect();
    let nt = tight.len();
    let r = pareto_direction(program, theta, coords, core);
    let cost = {
        let mut v = program.h.matvec(theta);
        for (x, c) in v.iter_mut().zip(&program.c) {
            *x += c;
        }
        v
    };
    // rows 0..nx: A_T' z = -(c + Hθ) (equalities); then -z_j <= 0 for the inequality rows of T
    let n_ineq = tight.iter().filter(|&&i| !is_eq[i]).count();
    let mut a = Mat::zeros(nx + n_ineq, nt);
    let mut b = vec![0.0; nx + n_ineq];
    for k in 0..nx {
        for (j, &i) in tight.iter().enumerate() {
            a[(k, j)] = program.a[(i, k)];
        }
        b[k] = -cost[k];
    }
    let mut row = nx;
    for (j, &i) in tight.iter().enumerate() {
        if !is_eq[i] {
            a[(row, j)] = -1.0;
            row += 1;
        }
    }
    let obj: Vec<f64> = tight.iter().map(|&i| -r[i]).collect();
    let eq_rows: Vec<usize> = (0..nx).collect();
    let z = crate::solver::lp::solve_lp(Some(&obj), &a, &b, &eq_rows)?.x;
    let zmax = z.iter().fold(0.0f64, |acc, v| acc.max(v.abs()));
    let mut support: Vec<(usize, f64)> = tight.iter().zip(&z).filter(|(i, v)| !is_eq[**i] && **v > 1e-10 * (1.0 + zmax)).map(|(&i, &v)| (i, v)).collect();
    support.sort_by(|x, y| y.1.total_cmp(&x.1));
    let in_support = {
        let mut v = vec![false; m];
        for &(i, _) in &support {
            v[i] = true;
        }
        v
    };
    let rest = tight.iter().copied().filter(|&i| !is_eq[i] && !in_support[i]);
    let mut set = greedy_independent(&program.a, eq, support.iter().map(|&(i, _)| i).chain(rest), nx);
    if set.len() != nx {
        return None;
    }
    set.sort_unstable();
    Some(set)
}

/// Active sets to try, best first.
fn candidate_sets(program: &MpProgram, sol: &SolverOutput) -> Vec<Vec<usize>> {
    let m = program.num_constraints();
    let nx = program.num_x();
    let eq = &program.equality_indices;
    let mut is_eq = vec![false; m];
    for &i in eq {
        is_eq[i] = true;
    }
    let dmax = sol.dual.iter().take(m).fold(0.0f64, |a, v| a.max(v.abs()));
    let dtol = 1e-10 * (1.0 + dmax);
    let mut prio: Vec<usize> = (0..m.min(sol.dual.len())).filter(|&i| !is_eq[i] && sol.dual[i] > dtol).collect();
    prio.sort_by(|&a, &b| sol.dual[b].total_cmp(&sol.dual[a]));
    let rhs_scale = |i: usize| 1.0 + program.b[i].abs();
    let lp = program.q.is_none();
    if !lp {
        let mut set = greedy_independent(&program.a, eq, prio.iter().copied(), usize::MAX);
        set.sort_unstable();
        return vec![set];
    }
    let in_prio = {
        let mut v = vec![false; m];
        for &i in &prio {
            v[i] = true;
        }
        v
    };
    let mut tight: Vec<usize> = (0..m.min(sol.slack.len())).filter(|&i| !is_eq[i] && !in_prio[i] && sol.slack[i].abs() <= 1e-9 * rhs_scale(i)).collect();
    tight.sort_by(|&a, &b| sol.slack[a].abs().total_cmp(&sol.slack[b].abs()));
    let mut orders = vec![tight.clone()];
    if tight.len() > 1 {
        orders.push(tight.iter().rev().copied().collect());
        let h = tight.len() / 2;
        orders.push(tight[h..].iter().chain(&tight[..h]).copied().collect());
    }
    let mut out: Vec<Vec<usize>> = vec![];
    for ord in orders {
        let mut set = greedy_independent(&program.a, eq, prio.iter().copied().chain(ord), nx);
        if set.len() != nx {
            continue;
        }
        set.sort_unstable();
        if !out.contains(&set) {
            out.push(set);
        }
    }
    out
}

/// The equality rows, then the rows of `order` that are linearly independent of those taken
/// (modified Gram-Schmidt with one reorthogonalisation), until `limit` rows.
fn greedy_independent(a: &Mat, eq: &[usize], order: impl Iterator<Item = usize>, limit: usize) -> Vec<usize> {
    let mut basis: Vec<Vec<f64>> = vec![];
    let mut out: Vec<usize> = eq.to_vec();
    let take = |i: usize, basis: &mut Vec<Vec<f64>>| -> bool {
        let mut v = a.row(i).to_vec();
        let n0 = norm(&v);
        if n0 == 0.0 {
            return false;
        }
        for _ in 0..2 {
            for q in basis.iter() {
                let p = dot(q, &v);
                for (x, y) in v.iter_mut().zip(q) {
                    *x -= p * y;
                }
            }
        }
        let n1 = norm(&v);
        if n1 <= 1e-9 * n0 {
            return false;
        }
        v.iter_mut().for_each(|x| *x /= n1);
        basis.push(v);
        true
    };
    for &i in eq {
        take(i, &mut basis);
    }
    for i in order {
        if out.len() >= limit {
            break;
        }
        if take(i, &mut basis) {
            out.push(i);
        }
    }
    out
}

/// The region of `active_set` (sorted, equalities included) with unit-norm unreduced rows;
/// `theta`, if given, must lie in it within the tolerance.
fn build_region(program: &MpProgram, opts: &EtOptions, active_set: &[usize], theta: Option<&[f64]>) -> Result<Region, BuildError> {
    let nt = program.num_t();
    let nx = program.num_x();
    // the law of an mpLP is only optimal for a basis (n_x independent rows): with fewer or more
    // rows, `optimal_control_law` falls back to a pseudo-inverse (a least-squares x); an mpQP needs
    // independent rows (LICQ)
    if (program.q.is_none() && active_set.len() != nx) || !program.check_active_set_rank(active_set) {
        return Err(BuildError::Rank);
    }
    let law = program.optimal_control_law(active_set).ok_or(BuildError::Rank)?;
    if law.b_x.len() != nx || law.a_l.rows != active_set.len() || law.b_x.iter().any(|v| !v.is_finite()) || law.b_l.iter().any(|v| !v.is_finite()) {
        return Err(BuildError::Rank);
    }
    let (e, f, _, _) = program.region_rows(active_set, &law);
    let bbox = program.theta_bbox();
    let mut rows = Vec::with_capacity(e.rows * (nt + 1));
    for i in 0..e.rows {
        let r = e.row(i);
        let nrm = norm(r);
        if !(nrm > 1e-10) {
            if f[i] < -1e-8 {
                return Err(BuildError::Empty);
            }
            continue;
        }
        let fi = f[i] / nrm;
        // a row that no point of the parameter bounding box can reach is implied by the
        // parameter-set rows, which are part of every region
        if let Some((lo, hi)) = bbox {
            let ub: f64 = r.iter().zip(lo.iter().zip(hi)).map(|(a, (l, h))| (a * l).max(a * h)).sum::<f64>() / nrm;
            if ub < fi - 1e-9 * (1.0 + fi.abs()) {
                continue;
            }
        }
        rows.extend(r.iter().map(|v| v / nrm));
        rows.push(fi);
    }
    let mut region = Region {
        uid: 0,
        active_set: active_set.to_vec(),
        nrows: 0,
        rows: vec![],
        wmax: vec![],
        a_x: law.a_x,
        b_x: law.b_x,
        a_l: law.a_l,
        b_l: law.b_l,
        hits: 0,
        last_hit: 0,
        reduced: false,
        rows_ver: 0,
        succ: [(0, 0, 0); NSUCC],
    };
    region.set_rows(rows, nt);
    if let Some(th) = theta {
        if !(region.max_violation(th) <= opts.tol) {
            return Err(BuildError::NotContaining);
        }
    }
    // the Chebyshev LP checks the dimension, and, for a region given without a point in it
    // (`insert_active_set`), that it is not empty
    if region.nrows > 0 && (opts.check_full_dim || theta.is_none()) {
        let mut em = Mat::zeros(region.nrows, nt);
        let mut fv = Vec::with_capacity(region.nrows);
        for i in 0..region.nrows {
            let row = region.row(i, nt + 1);
            em.row_mut(i).copy_from_slice(&row[..nt]);
            fv.push(row[nt]);
        }
        match crate::utils::chebyshev_ball_status(&em, &fv, &[]) {
            (Some(s), _) => {
                if opts.check_full_dim && !(s.x[nt] > 1e-8) {
                    return Err(BuildError::LowerDimensional);
                }
            }
            (None, crate::solver::lp::LpStatus::Infeasible) => return Err(BuildError::Empty),
            // unbounded (no parameter set rows) or an undecided LP: the region is kept
            (None, _) => {}
        }
    }
    Ok(region)
}
