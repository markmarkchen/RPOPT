//! LP/QP census (feature `lp_census`): counts every call into the LP and QP entry points by
//! purpose, with the number that came back infeasible and the wall time inside the call, plus
//! region-build, redundancy-row and combinatorial-candidate counters.
//!
//! The purpose of a call is the thread-local "current purpose", set by an RAII guard at the call
//! sites (`census_purpose!`). Statistics are kept per thread (so rayon workers do not contend)
//! and summed by `snapshot`. With the feature off every macro expands to nothing and none of
//! the machinery is compiled.

/// Sets the purpose for the rest of the enclosing block (nested guards restore the outer one).
#[cfg(feature = "lp_census")]
#[macro_export]
macro_rules! census_purpose {
    ($p:ident) => {
        let _census_guard = $crate::census::Guard::new($crate::census::Purpose::$p);
    };
}
#[cfg(not(feature = "lp_census"))]
#[macro_export]
macro_rules! census_purpose {
    ($p:ident) => {};
}

/// Adds to a named counter (`census::Counter`).
#[cfg(feature = "lp_census")]
#[macro_export]
macro_rules! census_bump {
    ($c:ident, $n:expr) => {
        $crate::census::bump($crate::census::Counter::$c, ($n) as u64)
    };
}
#[cfg(not(feature = "lp_census"))]
#[macro_export]
macro_rules! census_bump {
    ($c:ident, $n:expr) => {};
}

/// Opens a region-build record named `$id`; it counts as empty unless `census_keep!($id)` runs.
#[cfg(feature = "lp_census")]
#[macro_export]
macro_rules! census_build {
    ($id:ident) => {
        let mut $id = $crate::census::BuildGuard::new();
    };
}
#[cfg(not(feature = "lp_census"))]
#[macro_export]
macro_rules! census_build {
    ($id:ident) => {};
}

#[cfg(feature = "lp_census")]
#[macro_export]
macro_rules! census_keep {
    ($id:ident) => {
        $id.keep()
    };
}
#[cfg(not(feature = "lp_census"))]
#[macro_export]
macro_rules! census_keep {
    ($id:ident) => {};
}

/// Evaluates an expression and adds its wall time (ns) to a counter.
#[cfg(feature = "lp_census")]
#[macro_export]
macro_rules! census_timed {
    ($c:ident, $e:expr) => {{
        let __t = $crate::census::begin();
        let __r = $e;
        $crate::census::bump($crate::census::Counter::$c, __t.elapsed().as_nanos() as u64);
        __r
    }};
}
#[cfg(not(feature = "lp_census"))]
#[macro_export]
macro_rules! census_timed {
    ($c:ident, $e:expr) => {
        $e
    };
}

/// Times an LP/QP entry point body (`$kind` is `Lp` or `Qp`) and records whether it came back
/// infeasible. The body may `return` and use `?`.
#[cfg(feature = "lp_census")]
#[macro_export]
macro_rules! census_call {
    ($kind:ident, $body:block) => {{
        let __t = $crate::census::begin();
        let __r = (|| $body)();
        $crate::census::end($crate::census::Kind::$kind, __t, $crate::census::Outcome::infeasible(&__r));
        __r
    }};
}
#[cfg(not(feature = "lp_census"))]
#[macro_export]
macro_rules! census_call {
    ($kind:ident, $body:block) => {
        $body
    };
}

#[cfg(feature = "lp_census")]
pub use imp::*;

#[cfg(feature = "lp_census")]
mod imp {
    use std::cell::Cell;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(usize)]
    pub enum Purpose {
        /// (a) one LP per tested row while building a region
        Redundancy = 0,
        /// (b) feasibility of a candidate active set (rank-passing sets, lambda/omega rows, witnesses)
        Feasibility = 1,
        /// (c) neighbour search across a facet (facet centers, facet steps, facet completion)
        Neighbour = 2,
        /// (d) Chebyshev ball, full-dimension and empty-region tests of a region
        Chebyshev = 3,
        /// (e) bounding box of a region and of the parameter set
        Bbox = 4,
        /// initial point, sampling and start-up checks
        Init = 5,
        /// (g) anything else
        Other = 6,
    }

    pub const NP: usize = 7;
    pub const NAMES: [&str; NP] = ["redundancy", "feasibility", "neighbour", "chebyshev", "bbox", "init", "other"];

    #[derive(Clone, Copy, Debug)]
    pub enum Kind {
        Lp = 0,
        Qp = 1,
    }

    #[derive(Clone, Copy, Debug)]
    #[repr(usize)]
    pub enum Counter {
        /// region constructions started (after the cheap dispatch), kept, and empty or lower dimensional
        Built,
        Kept,
        Empty,
        /// candidate active sets dropped because already seen
        Dup,
        /// rows entering redundancy removal, and rows kept by it
        RedRowsIn,
        RedRowsKept,
        /// combinatorial: candidate sets generated, pruned by the list, rank deficient, accepted
        /// without an LP by a witness, tested by LP, found infeasible by LP
        CombGen,
        CombPruned,
        CombRankFail,
        CombWitness,
        CombLp,
        CombInfeasible,
        /// wall time inside region constructions (`gen_cr_from_kkt` and friends), and the part of
        /// it spent in LP calls (with their count)
        BuildNanos,
        BuildLpNanos,
        BuildLpCalls,
        /// final facet rows (rows of E) of the regions kept by a builder
        FacetRows,
        /// vertex clipping (`clip_flags`): calls, success, and None by reason
        ClipCalls,
        ClipOk,
        ClipNtOut,
        ClipManyHyp,
        ClipInfBox,
        ClipManyVerts,
        ClipFewVerts,
        ClipViolate,
        /// mplcp `adjacent`: calls, no zero row (no neighbour), Case-1 fast path, Case 2 (pivot_facet)
        /// with the time spent in each
        AdjCalls,
        AdjNone,
        AdjFast,
        AdjCase2,
        AdjNoneNanos,
        AdjFastNanos,
        AdjCase2Nanos,
        /// mplcp time split: neighbour search (`expand_candidates`), region build, tableau update
        MplcpNbrNanos,
        MplcpBuildNanos,
        MplcpTabNanos,
        /// graph-type candidates: generated and passed to the region test
        CandGen,
        /// mplcp: candidates returned by `adjacent` (fast path Toggle / slow path Z), how many were
        /// already seen, and `adjacent` calls (with at least one candidate) whose candidates were all seen
        NbrFastRet,
        NbrFastSeen,
        NbrSlowRet,
        NbrSlowSeen,
        GrpFast,
        GrpFastAllSeen,
        GrpSlow,
        GrpSlowAllSeen,
        /// rows of a mplcp region that share a hyperplane (clip bit) with a box face or an earlier row
        ClipDupRows,
        /// `pivot_facet`: calls, loop iterations ending in Case 1 / Case 2, Case-2 branches tried,
        /// branches cut by `lex_nonneg`, Chebyshev LPs and those with an empty facet, branches that
        /// reach a basis already reached in the same `adjacent` call, `depth > 6` returns, iteration
        /// limit exits, calls per recursion depth 0..=7
        PfCalls,
        PfCase1,
        PfCase2,
        PfBranches,
        PfLexCut,
        PfCheb,
        PfChebEmpty,
        PfRepeatBasis,
        PfDepthLimit,
        PfIterLimit,
        PfDepth0,
        PfDepth1,
        PfDepth2,
        PfDepth3,
        PfDepth4,
        PfDepth5,
        PfDepth6,
        PfDepth7,
        /// mplcp facet certificate: facets examined, certified (known single neighbour), certified
        /// ones that `adjacent` took the slow / fast path on (audit mode), time spent in the check,
        /// and certified facets whose search also reached a new region
        CertFacets,
        CertHit,
        CertNanos,
        CertExtra,
        AdjCertSkip,
        AdjCertSkipNanos,
        /// `pivot_facet` leaf branches (the next basis is complementary): skipped without their
        /// Chebyshev LP because the active set is already in the `Seen` map, and not skipped
        LeafSkip,
        LeafKept,
        /// leaves returned with a hint of their own (the sub-facet Chebyshev centre) and, of the
        /// returned leaves, those whose old hint (parent centre projected onto the row) lies inside /
        /// outside the leaf's sub-facet
        HintCentre,
        HintInside,
        HintOutside,
        /// facet walks stopped by the exact cycle check or by the pivot budget, and the fallbacks
        /// (QP just across the facet) that found / did not find a neighbour
        WalkCycle,
        WalkBudget,
        FallbackAcross,
        FallbackFail,
        /// non-leaf Case-2 branches accepted as nonempty by a witness point (no Chebyshev LP), of which
        /// the ones whose piece equals the parent polytope; branches where candidates were tried and
        /// none cleared the margin (the LP then runs)
        WitnessAccept,
        WitnessNoCuts,
        WitnessFail,
    }
    pub const NC: usize = 81;

    #[derive(Clone, Debug)]
    pub struct Stats {
        pub calls: [[u64; NP]; 2],
        pub infeasible: [[u64; NP]; 2],
        pub nanos: [[u64; NP]; 2],
        pub counters: [u64; NC],
    }

    impl Default for Stats {
        fn default() -> Self {
            Stats { calls: [[0; NP]; 2], infeasible: [[0; NP]; 2], nanos: [[0; NP]; 2], counters: [0; NC] }
        }
    }

    impl Stats {
        fn add(&mut self, o: &Stats) {
            for k in 0..2 {
                for p in 0..NP {
                    self.calls[k][p] += o.calls[k][p];
                    self.infeasible[k][p] += o.infeasible[k][p];
                    self.nanos[k][p] += o.nanos[k][p];
                }
            }
            for i in 0..NC {
                self.counters[i] += o.counters[i];
            }
        }
        pub fn counter(&self, c: Counter) -> u64 {
            self.counters[c as usize]
        }
        pub fn lp_calls(&self) -> u64 {
            self.calls[0].iter().sum()
        }
        pub fn lp_nanos(&self) -> u64 {
            self.nanos[0].iter().sum()
        }
        pub fn qp_calls(&self) -> u64 {
            self.calls[1].iter().sum()
        }
        pub fn qp_nanos(&self) -> u64 {
            self.nanos[1].iter().sum()
        }
    }

    static REGISTRY: Mutex<Vec<Arc<Mutex<Stats>>>> = Mutex::new(Vec::new());

    thread_local! {
        static CUR: Cell<Purpose> = const { Cell::new(Purpose::Other) };
        static IN_BUILD: Cell<bool> = const { Cell::new(false) };
        static MINE: Arc<Mutex<Stats>> = {
            let s = Arc::new(Mutex::new(Stats::default()));
            REGISTRY.lock().unwrap().push(s.clone());
            s
        };
    }

    pub struct Guard(Purpose);

    impl Guard {
        #[inline]
        pub fn new(p: Purpose) -> Guard {
            Guard(CUR.with(|c| c.replace(p)))
        }
    }

    impl Drop for Guard {
        #[inline]
        fn drop(&mut self) {
            CUR.with(|c| c.set(self.0));
        }
    }

    /// How an entry point's result reads as "infeasible" (None, or false for feasibility tests).
    pub trait Outcome {
        fn infeasible(&self) -> bool;
    }
    impl Outcome for bool {
        fn infeasible(&self) -> bool {
            !*self
        }
    }
    impl<T> Outcome for Option<T> {
        fn infeasible(&self) -> bool {
            self.is_none()
        }
    }

    #[inline]
    pub fn begin() -> Instant {
        Instant::now()
    }

    #[inline]
    pub fn end(kind: Kind, t: Instant, infeasible: bool) {
        let ns = t.elapsed().as_nanos() as u64;
        let p = CUR.with(|c| c.get()) as usize;
        let k = kind as usize;
        let inb = IN_BUILD.with(|c| c.get());
        MINE.with(|m| {
            let mut s = m.lock().unwrap();
            if inb {
                s.counters[Counter::BuildLpNanos as usize] += ns;
                s.counters[Counter::BuildLpCalls as usize] += 1;
            }
            s.calls[k][p] += 1;
            s.nanos[k][p] += ns;
            s.infeasible[k][p] += infeasible as u64;
        });
    }

    thread_local! {
        static MUTE_CLIP: Cell<bool> = const { Cell::new(false) };
    }

    /// While alive, the `Clip*` counters ignore bumps (a fallback that repeats a clip already counted).
    pub struct MuteClip(bool);
    impl MuteClip {
        pub fn new() -> Self {
            MuteClip(MUTE_CLIP.with(|c| c.replace(true)))
        }
    }
    impl Default for MuteClip {
        fn default() -> Self {
            Self::new()
        }
    }
    impl Drop for MuteClip {
        fn drop(&mut self) {
            MUTE_CLIP.with(|c| c.set(self.0));
        }
    }

    /// Groups of candidates (one group per `adjacent` call) awaiting their seen/fresh verdicts.
    struct Groups {
        pending: std::collections::VecDeque<(u32, bool)>,
        cur: Option<(u32, bool, u32, u32)>, // group, fast, count, seen
    }
    thread_local! {
        static GROUPS: std::cell::RefCell<Groups> = const { std::cell::RefCell::new(Groups { pending: std::collections::VecDeque::new(), cur: None }) };
        static GROUP_ID: Cell<u32> = const { Cell::new(0) };
        static BASES: std::cell::RefCell<std::collections::HashSet<Vec<usize>>> = std::cell::RefCell::new(std::collections::HashSet::new());
    }
    fn finish_group(g: &mut Groups) {
        if let Some((_, fast, cnt, seen)) = g.cur.take() {
            let (a, b) = if fast { (Counter::GrpFast, Counter::GrpFastAllSeen) } else { (Counter::GrpSlow, Counter::GrpSlowAllSeen) };
            bump(a, 1);
            if cnt == seen {
                bump(b, 1);
            }
        }
    }
    /// Starts a batch: one `(is_fast_path)` flag per candidate, in order; `adjacent` calls are
    /// told apart by the group number.
    pub fn batch(flags: &[(u32, bool)]) {
        GROUPS.with(|g| {
            let mut g = g.borrow_mut();
            finish_group(&mut g);
            g.pending.clear();
            g.pending.extend(flags.iter().cloned());
        });
    }
    pub fn next_group() -> u32 {
        GROUP_ID.with(|c| {
            c.set(c.get() + 1);
            c.get()
        })
    }
    /// The verdict (`fresh`) for the next candidate of the batch.
    pub fn seen(fresh: bool) {
        GROUPS.with(|g| {
            let mut g = g.borrow_mut();
            let Some((id, fast)) = g.pending.pop_front() else { return };
            if g.cur.map(|c| c.0 != id).unwrap_or(false) {
                finish_group(&mut g);
            }
            let cur = g.cur.get_or_insert((id, fast, 0, 0));
            cur.2 += 1;
            cur.3 += (!fresh) as u32;
            bump(if fast { Counter::NbrFastSeen } else { Counter::NbrSlowSeen }, (!fresh) as u64);
        });
    }
    pub fn flush_groups() {
        GROUPS.with(|g| finish_group(&mut g.borrow_mut()));
    }
    /// Records the row set of a basis reached inside the current `adjacent` call; true when already seen.
    pub fn basis_seen(key: Vec<usize>) -> bool {
        BASES.with(|b| !b.borrow_mut().insert(key))
    }
    pub fn basis_reset() {
        BASES.with(|b| b.borrow_mut().clear());
    }

    #[inline]
    pub fn bump(c: Counter, n: u64) {
        if (Counter::ClipCalls as usize..=Counter::ClipViolate as usize).contains(&(c as usize)) && MUTE_CLIP.with(|m| m.get()) {
            return;
        }
        MINE.with(|m| m.lock().unwrap().counters[c as usize] += n);
    }

    /// Counts a region construction; empty unless `keep` is called before it drops.
    pub struct BuildGuard(bool, Instant);
    impl BuildGuard {
        pub fn new() -> Self {
            bump(Counter::Built, 1);
            IN_BUILD.with(|c| c.set(true));
            BuildGuard(false, Instant::now())
        }
        pub fn keep(&mut self) {
            self.0 = true;
        }
    }
    impl Default for BuildGuard {
        fn default() -> Self {
            Self::new()
        }
    }
    impl Drop for BuildGuard {
        fn drop(&mut self) {
            IN_BUILD.with(|c| c.set(false));
            bump(Counter::BuildNanos, self.1.elapsed().as_nanos() as u64);
            bump(if self.0 { Counter::Kept } else { Counter::Empty }, 1);
        }
    }

    /// Zeroes every thread's statistics.
    pub fn reset() {
        flush_groups();
        for s in REGISTRY.lock().unwrap().iter() {
            *s.lock().unwrap() = Stats::default();
        }
    }

    /// Sum over all threads.
    pub fn snapshot() -> Stats {
        flush_groups();
        let mut out = Stats::default();
        for s in REGISTRY.lock().unwrap().iter() {
            out.add(&s.lock().unwrap());
        }
        out
    }
}
