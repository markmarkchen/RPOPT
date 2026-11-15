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

/// Records why `rank1_case2` refused the call (census only; read by `adjacent` for the walk class).
#[cfg(feature = "lp_census")]
#[macro_export]
macro_rules! census_reason {
    ($k:expr) => {
        $crate::census::reason_set($k)
    };
}
#[cfg(not(feature = "lp_census"))]
#[macro_export]
macro_rules! census_reason {
    ($k:expr) => {};
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

/// Case-2 cost census: `census_t0!(m)` starts a lap, `census_lap!(m, Comp)` ends it and adds time, count and
/// allocations since `m` to the component (see `census::Comp`). Laps never nest, so the components add up.
#[cfg(feature = "lp_census")]
#[macro_export]
macro_rules! census_t0 {
    ($m:ident) => {
        let mut $m = $crate::census::Mark::now();
    };
}
#[cfg(not(feature = "lp_census"))]
#[macro_export]
macro_rules! census_t0 {
    ($m:ident) => {};
}
#[cfg(feature = "lp_census")]
#[macro_export]
macro_rules! census_lap {
    ($m:ident, $c:ident) => {
        $m.lap($crate::census::Comp::$c)
    };
}
#[cfg(not(feature = "lp_census"))]
#[macro_export]
macro_rules! census_lap {
    ($m:ident, $c:ident) => {};
}
/// Restarts a lap without recording (time to skip).
#[cfg(feature = "lp_census")]
#[macro_export]
macro_rules! census_skip {
    ($m:ident) => {
        $m = $crate::census::Mark::now()
    };
}
#[cfg(not(feature = "lp_census"))]
#[macro_export]
macro_rules! census_skip {
    ($m:ident) => {};
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
        /// candidate regions rejected because their active constraints are not tight under their law
        TightFlagged,
        TightKeptByQp,
        TightRejected,
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
        /// rank-1 Case-2 census: calls and ns by class, |P-| histogram, members of S and their Seen / empty status, ties, walk depth
        R1Calls0,
        R1Calls1,
        R1Calls2,
        R1Calls3,
        R1Calls4,
        R1Calls5,
        R1Nanos0,
        R1Nanos1,
        R1Nanos2,
        R1Nanos3,
        R1Nanos4,
        R1Nanos5,
        R1P0,
        R1P1,
        R1P2,
        R1P3,
        R1P4,
        R1P5,
        R1P6,
        R1P7,
        R1Members,
        R1Unseen,
        R1AllSeen,
        R1UnseenEmpty,
        R1UnseenNonEmpty,
        R1Ties,
        R1D0,
        R1D1,
        R1D2,
        R1D3to5,
        R1D6p,
        R1Out,
        /// rank-1 enumeration (Case 2 with one vanishing slack row and a zero pivot): calls it answered, calls it
        /// handed back to the walk, candidates emitted, members dropped as known, sub-facet LPs run, members
        /// accepted by a witness, members whose sub-facet is empty
        /// fast path taken through the scale-free nonzero pivot (absolute tve <= PTOL), and single-vanishing-slack-row
        /// calls whose pivot measure lies between the zero and nonzero thresholds (left to the walk)
        FastRel,
        PivotGap,
        /// fast-path calls with tve above PTOL whose pivot measure is at most MEASURE_ZERO (sent to rank-1)
        FastDemoted,
        R1Handled,
        R1Fallback,
        R1Emitted,
        R1SkipSeen,
        R1LpRun,
        R1WitnessAcc,
        R1EmptyMember,
        /// members of S whose sub-facet test (witness, LP or empty cut) was already done for the same active set earlier, and the
        /// distinct active sets tested; of the empty ones, the repeats
        R1TestRepeat,
        R1TestDistinct,
        R1EmptyRepeat,
        /// parametric_daqp: active-set tests and their outcomes (LICQ violated / empty / nonempty), ns in the tests and in the region
        /// builds, DAQP solves (stage 1, stage 2) and their ns, empties decided before any DAQP solve, general-LP fallbacks, ns in the
        /// LDL' of the Gram block and in forming the rows
        PdTests,
        PdLicq,
        PdEmpty,
        PdRegion,
        PdTestNanos,
        PdBuildNanos,
        PdDaqp1,
        PdDaqp2,
        PdDaqpNanos,
        PdEarlyEmpty,
        PdLpFallback,
        PdLdlNanos,
        PdSetupNanos,
        /// mplcp sub-facet emptiness tests answered by DAQP on the relaxed rows (empty) or sent on to the LP,
        /// and ns spent in the DAQP solves
        DqEmpty,
        DqFallback,
        DqNanos,
        /// simplex pivots of every LP (all three solver paths)
        LpPivots,
        /// walk Chebyshev LPs (PfCheb): rows of the child piece, rows added to the parent's polytope (sum and histogram 0 / 1 / 2 / 3+),
        /// pivots of the cold LP, calls with a parent point (centre, witness or hint), rows of the child that the parent point violates
        /// (all rows / added rows, sum over calls with a point), calls where the parent point is in the child piece, pivots of the
        /// cold LP on those calls, and an extra primal-shifted solve from the parent point (calls, pivots, ns; census only)
        ChebRows,
        ChebAdded,
        ChebAdd0,
        ChebAdd1,
        ChebAdd2,
        ChebAdd3p,
        ChebPivots,
        ChebHasPt,
        ChebViolAll,
        ChebViolNew,
        ChebPtInside,
        ChebPivotsPt,
        ChebShiftCalls,
        ChebShiftPivots,
        ChebShiftNs,
        ChebShiftSame,
        ChebDims,
        /// walk Chebyshev LPs split by leaf / non-leaf branch: calls, ns, cold pivots; phase-1 feasibility LP on the rows shifted by
        /// 1e-9 ||a|| for the non-leaf ones (calls, pivots, ns, same verdict as the Chebyshev LP)
        LpLeafCalls,
        LpLeafNs,
        LpLeafPiv,
        LpNlCalls,
        LpNlNs,
        LpNlPiv,
        P1Calls,
        P1Pivots,
        P1Ns,
        P1Same,
        /// box prefilter on the pieces that reach the DAQP / LP stage: tests, pieces the region box proves empty, of those the ones DAQP
        /// answered empty / the LP answered empty / the LP found nonempty (must be 0), polytope rows, rows redundant over the box,
        /// rows added to the parent piece and of those the redundant ones
        BoxTests,
        BoxEmpty,
        BoxEmptyDaqp,
        BoxEmptyLp,
        BoxEmptyNonempty,
        BoxRows,
        BoxRedRows,
        BoxCutRows,
        BoxRedCut,
        /// exact row redundancy on every 8th walk LP: samples, rows, non-redundant rows
        RedSamples,
        RedRows,
        RedNonRed,
        /// walks by facet hyperplane: walks, walks on a hyperplane walked before, of those from the other side, ns of the repeats and of all walks
        HyperWalks,
        HyperRepeat,
        HyperOther,
        HyperRepeatNs,
        HyperNs,
        /// DAQP test: calls (including paused), ns building the rows, ns in the solve, ns in the certificate check
        DqTests,
        DqBuildNs,
        DqSolveNs,
        DqCertNs,
        /// rank-1 multiplier rule (scale-free): multipliers the old guard refused that are now members of P-, ones the old guard refused that are
        /// now zero (noise), multipliers at or below PTOL that are now members, multipliers in the gap (left to the walk), and calls whose
        /// absolute mismatch refusal is now passed
        MuAccepted,
        MuNoiseZero,
        MuNewPositive,
        MuGap,
        MuMismatchFreed,
        /// `tableau_for` refusals (an absolute pivot below 1e-11), and those whose pivot is not small against the
        /// diagonal of M (pivot / M_ii above 1e-6); `resolve` returning no tableau for a region; refusals inside
        /// `candidate_law` (fresh tableau refused, or the one-pivot step below 1e-9)
        TabRefuse,
        TabRefuseRel,
        ResolveNone,
        CandLawNoTab,
        CandLawSmallPiv,
        /// bases that `tableau_for` accepted only in the second, largest-pivot-first order
        TabRescued,
        /// refined multipliers used by the enumeration: band entries decided as zero, as real, or still between (call goes to the walk)
        MuRefinedZero,
        BuildColdRetry,
        MuEmptyGuard,
        MuEmptyToWalk,
        MuRefinedReal,
        MuRefinedBand,
        /// audit (every gap-band call run both ways): compared, same neighbour sets, different, walk incomplete or enumeration undecided
        AuditCalls,
        AuditSame,
        AuditDiff,
        AuditExplained,
        AuditExtraWalk,
        AuditNoPositive,
        AuditSkip,
        AuditAltNone,
        /// end marker: the Case-2 component block starts here
        CounterEnd,
    }
    /// Case-2 cost components (see `Mark::lap`): per component and per case (2 rank-1, 3 walk) four counters:
    /// ns, laps, allocation calls, allocated bytes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(usize)]
    pub enum Comp {
        /// whole `adjacent` call (ns, calls), allocations of the whole call
        Total = 0,
        /// `adjacent` before the case is known: normalisation, vanishing-row scan (all calls)
        Pre,
        /// fast-path test and certificate check, between the scan and the rank-1 attempt
        Gate,
        /// walk only: the failed rank-1 attempt (whole call of `rank1_case2` that returned None)
        R1Try,
        /// walk: work tableau copy, null basis, facet polytope, hint
        Setup,
        /// rank-1: entry tests (zero pivot, zero columns, multipliers), members and Seen lookups, lambda rows, tie checks
        R1Entry,
        /// rank-1: facet polytope and hint
        R1Poly,
        /// rank-1: per-member cut rows (polytope copy, row pushes, empty-by-constant tests)
        R1Cuts,
        /// rank-1: output vectors (to_theta, push)
        R1Out,
        /// walk_one: per-iteration prelude (a, wcol, wrow, P and P-cap-Z sets, alpha row)
        Prelude,
        /// walk_one Case 1: lexicographic choice of the leaving row
        Case1,
        /// walk_one Case 2: Gamma_j lexicographic and cut tests per branch
        Gamma,
        /// tableau pivot (flush, defer_pivot, column lookup)
        Pivot,
        /// tableau clone for a branch
        Clone,
        /// walk-state key and the exact cycle check
        Key,
        /// leaf active-set computation and Seen lookup
        Seen,
        /// witness acceptance test of a sub-facet
        Wit,
        /// DAQP emptiness test
        Daqp,
        /// Chebyshev LP
        Lp,
        /// leaf output (push_leaf)
        Leaf,
        /// stack handling: Frame push and pop, children vector, frame drop
        Stack,
        /// after the walk: neighbour conversion, probes of an incomplete walk
        Post,
        /// census-only work (basis_seen, rank-1 probe), excluded from the totals in the report
        Census,
        /// census-only hint check inside `push_leaf`; nested in the Leaf laps, subtracted from them in the report
        HintCensus,
    }
    pub const NCOMP: usize = 24;
    pub const COMP_NAMES: [&str; NCOMP] = ["total", "pre", "gate", "rank-1 try (failed)", "walk setup", "rank-1 entry/members/lambda", "rank-1 facet polytope", "rank-1 member cuts", "rank-1 output", "walk prelude (P, Z sets)", "Case-1 lex choice", "Case-2 Gamma tests", "pivots", "tableau clones", "cycle key", "Seen lookups (leaf)", "witness tests", "DAQP tests", "Chebyshev LPs", "leaf output", "stack handling", "post-walk", "census overhead", "census in leaf output"];
    pub const C2_BASE: usize = Counter::CounterEnd as usize;
    /// slot of component `c` for case index `ci` (0 rank-1, 1 walk), field `f` (0 ns, 1 laps, 2 allocs, 3 bytes)
    pub fn c2_slot(c: usize, ci: usize, f: usize) -> usize {
        C2_BASE + (c * 2 + ci) * 4 + f
    }
    /// timer calibration: ns per lap, and laps counted (set by the driver)
    pub const CALIB: usize = C2_BASE + NCOMP * 8;
    /// walk classes (by vanishing rows): 0 one slack row, zero pivot; 1 one slack row, pivot in the gap; 2 one slack row, nonzero pivot;
    /// 3 one z row; 4 two slack rows; 5 one slack one z; 6 two z rows; 7 three or more. Fields: calls, ns, LP ns, DAQP ns, Gamma ns
    pub const NWK: usize = 12;
    pub const WK_BASE: usize = CALIB + 4;
    pub const WK_NAMES: [&str; NWK] = ["1 slack, zero pivot", "1 slack, pivot in gap", "1 slack, nonzero pivot", "1 z row", "2 slack", "1 slack + 1 z", "2 z", "3 or more", "1 slack, zero pivot: z column", "1 slack, zero pivot: symmetry mismatch", "1 slack, zero pivot: multiplier in the gap band", "1 slack, zero pivot: tie of two ratios"];
    /// walk Chebyshev LPs by outcome (0 empty at radius 1e-9, 1 nonempty); fields: calls, cold pivots, cold ns, shifted-start calls, shifted pivots,
    /// shifted ns, phase-1 pivots, phase-1 ns
    pub const EO_BASE: usize = WK_BASE + NWK * 5;
    /// rank-1 multiplier census: nu = |mu_j| sqrt(M_jj / M_cc) of every active row with mu_j > PTOL, by decade 10^-k (k = 0..=15, 15 holds
    /// everything smaller), in four classes: [0] refused by the old guard (mu <= 1e-7 mu_max), [1] not refused, each split by whether the
    /// reason-5 mismatch test passed (+2 when it failed)
    pub const NU_BASE: usize = EO_BASE + 16;
    /// refined multipliers of the gap-band calls: [0] calls, [1] entries in the tableau's band, [2] of those refined nu <= 1e-12, [3] >= 1e-6,
    /// [4] still between, [5] calls with every band entry decided, [6] ns, [7] large entries compared, [8] of those disagreeing (sign or 1e-6
    /// relative), [9..25] decade histogram of the refined nu of the band entries, [25] tableau zeros the refined value does not call zero
    pub const MU2_BASE: usize = NU_BASE + 16 * 4;
    pub const NC: usize = MU2_BASE + 32;
    pub fn bump_slot(i: usize, n: u64) {
        MINE.with(|m| m.lock().unwrap().counters[i] += n);
    }

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

    thread_local! {
        static CALL_MAXD: Cell<usize> = const { Cell::new(0) };
    }
    thread_local! {
        static TESTED: std::cell::RefCell<std::collections::HashMap<Vec<usize>, bool>> = std::cell::RefCell::new(std::collections::HashMap::new());
    }
    /// Records a sub-facet test of the active set `z` (and whether it was empty); counts repeats.
    pub fn tested(z: &[usize], empty: bool) {
        let prev = TESTED.with(|t| t.borrow_mut().insert(z.to_vec(), empty));
        match prev {
            Some(_) => {
                bump(Counter::R1TestRepeat, 1);
                if empty {
                    bump(Counter::R1EmptyRepeat, 1);
                }
            }
            None => bump(Counter::R1TestDistinct, 1),
        }
    }
    pub fn tested_reset() {
        TESTED.with(|t| t.borrow_mut().clear());
    }
    pub fn depth_reset() {
        CALL_MAXD.with(|c| c.set(0));
    }
    pub fn depth_note(d: usize) {
        CALL_MAXD.with(|c| c.set(c.get().max(d)));
    }
    pub fn depth_take() -> usize {
        CALL_MAXD.with(|c| c.get())
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

    thread_local! {
        static PIV: Cell<u64> = const { Cell::new(0) };
    }
    /// Counts one simplex pivot (thread-local, no lock).
    #[inline]
    pub fn piv_bump() {
        PIV.with(|c| c.set(c.get() + 1));
    }
    thread_local! {
        static REASON: Cell<u8> = const { Cell::new(0) };
    }
    pub fn reason_set(k: u8) {
        REASON.with(|c| c.set(k));
    }
    pub fn reason_take() -> u8 {
        REASON.with(|c| c.replace(0))
    }
    pub fn piv_get() -> u64 {
        PIV.with(|c| c.get())
    }

    thread_local! {
        static A_CALLS: Cell<u64> = const { Cell::new(0) };
        static A_BYTES: Cell<u64> = const { Cell::new(0) };
        static ACC: [Cell<u64>; NCOMP * 4] = const { [const { Cell::new(0) }; NCOMP * 4] };
    }

    /// Global allocator wrapper that counts allocation calls and bytes per thread (feature `lp_census`).
    pub struct CountingAlloc<A>(pub A);
    unsafe impl<A: std::alloc::GlobalAlloc> std::alloc::GlobalAlloc for CountingAlloc<A> {
        unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
            let _ = A_CALLS.try_with(|c| c.set(c.get() + 1));
            let _ = A_BYTES.try_with(|c| c.set(c.get() + l.size() as u64));
            unsafe { self.0.alloc(l) }
        }
        unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
            unsafe { self.0.dealloc(p, l) }
        }
        unsafe fn alloc_zeroed(&self, l: std::alloc::Layout) -> *mut u8 {
            let _ = A_CALLS.try_with(|c| c.set(c.get() + 1));
            let _ = A_BYTES.try_with(|c| c.set(c.get() + l.size() as u64));
            unsafe { self.0.alloc_zeroed(l) }
        }
        unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, n: usize) -> *mut u8 {
            let _ = A_CALLS.try_with(|c| c.set(c.get() + 1));
            let _ = A_BYTES.try_with(|c| c.set(c.get() + n as u64));
            unsafe { self.0.realloc(p, l, n) }
        }
    }

    /// A lap start: clock and allocation counters.
    pub struct Mark(Instant, u64, u64);
    impl Mark {
        #[inline]
        pub fn now() -> Mark {
            Mark(Instant::now(), A_CALLS.with(|c| c.get()), A_BYTES.with(|c| c.get()))
        }
        /// Adds the time, one lap and the allocations since the last lap (or start) to `c`, and restarts.
        #[inline]
        pub fn lap(&mut self, c: Comp) {
            let ns = self.0.elapsed().as_nanos() as u64;
            let (a, b) = (A_CALLS.with(|c| c.get()), A_BYTES.with(|c| c.get()));
            ACC.with(|acc| {
                let i = c as usize * 4;
                acc[i].set(acc[i].get() + ns);
                acc[i + 1].set(acc[i + 1].get() + 1);
                acc[i + 2].set(acc[i + 2].get() + a - self.1);
                acc[i + 3].set(acc[i + 3].get() + b - self.2);
            });
            *self = Mark::now();
        }
    }
    pub fn c2_reset() {
        ACC.with(|a| a.iter().for_each(|x| x.set(0)));
    }
    pub fn c2_snapshot() -> [u64; NCOMP * 4] {
        ACC.with(|a| std::array::from_fn(|i| a[i].get()))
    }
    pub fn c2_restore(s: [u64; NCOMP * 4]) {
        ACC.with(|a| a.iter().zip(s).for_each(|(x, v)| x.set(v)));
    }
    /// Moves the accumulated laps into the statistics under case `case` (2 rank-1, 3 walk), with the call's
    /// total time and allocations (from the mark `m0` taken at its start), and clears them.
    pub fn c2_commit(case: usize, m0: &Mark, class: usize) {
        let ns = m0.0.elapsed().as_nanos() as u64;
        let (a, b) = (A_CALLS.with(|c| c.get()) - m0.1, A_BYTES.with(|c| c.get()) - m0.2);
        let mut v = c2_snapshot();
        c2_reset();
        let t = Comp::Total as usize * 4;
        v[t] += ns;
        v[t + 1] += 1;
        v[t + 2] += a;
        v[t + 3] += b;
        let ci = case - 2;
        MINE.with(|m| {
            let mut s = m.lock().unwrap();
            for c in 0..NCOMP {
                for f in 0..4 {
                    s.counters[c2_slot(c, ci, f)] += v[c * 4 + f];
                }
            }
            if case == 3 {
                let b = WK_BASE + class * 5;
                s.counters[b] += 1;
                s.counters[b + 1] += v[Comp::Total as usize * 4] - v[Comp::Census as usize * 4] - v[Comp::HintCensus as usize * 4];
                s.counters[b + 2] += v[Comp::Lp as usize * 4];
                s.counters[b + 3] += v[Comp::Daqp as usize * 4];
                s.counters[b + 4] += v[Comp::Gamma as usize * 4];
            }
        });
    }
    /// ns per lap (clock reads and counter updates), measured on an empty lap loop.
    pub fn calibrate() -> f64 {
        c2_reset();
        let n = 200_000u32;
        let mut m = Mark::now();
        let t = Instant::now();
        for _ in 0..n {
            m.lap(Comp::Pre);
        }
        let ns = t.elapsed().as_nanos() as f64 / n as f64;
        c2_reset();
        ns
    }
    pub fn set_calib(ns: f64) {
        MINE.with(|m| m.lock().unwrap().counters[CALIB] = (ns * 1000.0) as u64);
    }

    /// Zeroes every thread's statistics.
    pub fn reset() {
        flush_groups();
        tested_reset();
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
