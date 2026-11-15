# mplcp: pop_081 (missed regions) and the stray overlapping regions (pop_005, pop_007, pop_011)

Both defects were present on main before phase 1 and are fixed in `ppopt_rs/src/algorithms/mplcp.rs`. Measured on Negishi
(every check a 1-CPU task, builds 8 CPUs, `--profile gate`), `lexcheck` with 4000 sample parameters per problem.

## 1. pop_081: the walk lost two of four regions

pop_081 has one variable and 18 constraints, several of them parallel (two copies of `-x <= 11`, two of `x <= 37`) and a
tiny Q (5.6e-6), so the unconstrained region is a slab 1e-4 wide between two big regions. `mplcp` returned 2 regions
(`[1]` and `[]`); `geometric` finds 4, and 3070 of 4000 feasible samples lay in no region.

**Root cause.** Tracing `adjacent` on the facet θ2 = −1 of region `[1]` (where the two copies of the row 12.5 + 1.5 θ2 ≥ -x
become active together): `pivot_facet` Case 1 picks one twin (row 4), α enters, the entering variable becomes the twin's
complement z4, and the Schur complement of a rank-one M is zero, so z4 cannot move the slack: the only leaving
candidate is the multiplier z1 (Case 2, depth 1), after which the entering variable is w1 and **no row decreases (P is
empty)**, while α rises (T[α, w1] = 1.2). The code did `if pset.is_empty() { return; }` *before* the test "no
vanishing row in P and α rising, then the entering variable replaces α, which is the neighbour". So the neighbour
was dropped. The same happens at the slab's other facet.

**Fix.** The α test is made before the empty-P return (`pivot_facet`, comment in the code). pop_081 returns 4 regions
(`[1]`, `[3]`, `[4]`, `[]`), 4000 of 4000 samples covered, no overlap. Regression test:
`tests/all/dependent_start.rs::mplcp_no_gap_pop_081`.

## 2. Stray overlapping regions

Before: pop_011 119 of 3682 feasible samples strictly inside two regions, pop_005 70 of 1385, pop_007 23 of 612 (main:
119, 70, 7). In every pair both regions were lexico-feasible with Chebyshev radius 0.6 to 5, the laws disagreed, and the
direct QP solve agreed with the smaller region.

**Root cause (pop_011, region `[1,3,8,11,18]`).** Rows 11 and 18 of pop_011 are parallel (`x3 <= 2.5042 − 0.181 θ0` and
`x3 <= 2.3005`), so they can be active together only on the hyperplane θ0 = 1.126: a basis containing both is singular
(M_BB has two equal rows) and its region has no interior. The walk returned it anyway as a Case-2 neighbour of
`[1,6,8,11]` across facet 2, because a pivot was taken on a candidate leaving row with a = 1.58e-10, just above
the absolute `PTOL` = 1e-10, in a row whose own entries are of magnitude 3.8e3 (relative size 4e-14: rounding noise).
Pivoting on it gave a tableau with multipliers of 1e13 to 1e14 (`stray` example: active 1, 11, 18 have |λ| = 3e14,
3e14, 9e12), and the builder derived a region of radius 0.63 from that garbage law. None of the three suspects of the
Lean model applies as such: the facet rows are what the law gives (`hreg` holds for the law it is given), the
lexico-feasibility test is passed by the noise (`hlex`), and the tableau is inexact (a pivot on a numerical zero).
The `Facet::sc` hole of Lean Finding 9 was tested separately (give every non-basic variable the largest row scale): no
change on pop_011, pop_081 or masses4_N3, so it is not the cause here.

**Rejected alternative: a relative pivot tolerance.** Requiring a > PREL × (largest entry of the entering column or of
the row) removes the strays, but also removes real regions: with PREL = 1e-11 times the column maximum masses4_N3 loses
440 of 2892 regions; with 1e-12 times the row maximum masses5_N3 loses 10 of 5054. The lost regions have radius 1e-8
to 1e-6 and a direct QP at their centres confirms the active set (`extra_regions`, centre in no remaining region),
so they are real slivers. A threshold that keeps them (1e-13 times the row maximum) does not remove the strays
(pop_005 69, pop_007 17, pop_011 17 overlapping samples remain). The stray's pivot is not distinguishable from a
legitimate small one by size alone.

**Fix (first version: the check as a verdict; see "Round 2" below for the final rule).** A candidate region is rejected when its own law does not make its active constraints tight
(`tight_residual`, `TIGHT_TOL` = 1e-3): at the region's Chebyshev centre x(θ) = Aθ + b is evaluated and
|a_i x − b_i − F_i θ| / (1 + |b_i + F_i θ| + Σ|a_ij x_j|) is checked for every active row. A singular basis gets a
law that satisfies only one of two parallel rows, so the residual is large: the 13 strays of pop_011 have residuals
0.1 to 0.9, all other 396 regions at most 1e-7; over masses5_N3 (5053 regions, laws up to 1e16) the largest legitimate
residual is 1.5e-5. The gap of four orders of magnitude to 1e-3 is the margin. Law size, by contrast, cannot separate
them (masses5_N3 has legitimate laws of 1e16). Counter `TightReject` in the census. Regression test:
`mplcp_no_overlap_pop_011` (also asserts 397 regions).

### Before and after on the 135 problems that finish at 2 GB

(`lexcheck 4000`, certificate off; before = main plus phase 1, after = this fix.)

| | regions | lexico-infeasible | samples inside two regions | samples in no region |
|---|---:|---:|---:|---:|
| before | 1,097,907 | 0 | 212 (pop_005 70, pop_007 23, pop_011 119) | 3070 (pop_081) |
| after | 1,097,794 | 0 | 0 | 0 |

Problems whose region count changed (before, after): pop_081 2, 4; pop_005 10297, 10289; pop_007 12254, 12235;
pop_011 410, 397; pop_032 178965, 178950; pop_047 48146, 48144; masses3_N6 22151, 22142; masses3_N7 38370, 38323;
masses4_N4 16044, 16042. The other 126 problems have identical counts.

What was removed: the strays of pop_005, pop_007 and pop_011 (overlap now 0), and on the others regions that sampling did
not hit. For masses3_N6 the nine removed regions were examined (`extra_regions` on the removed set): in eight the direct QP at
the region's centre is infeasible (centre outside the feasible set, so the region was garbage), in one
(`[0,1,10,44,57,77]`, radius 3.4e-8) the QP agrees with the region, so one sliver of 3e-8 may have been lost. The
removals in pop_032, pop_047, masses3_N7 and masses4_N4 were not examined one by one.

Not fixed: the 8 problems that do not fit in 2 GB (peaks above) were not re-run.

## 3. Certificate soundness (mode 3: on at 2 to 6 parameters)

With the strays rejected, the certificate returns exactly the same set of active sets as the run without it on 27
problems (`examples/cert_diff.rs`, off against mode 3): the 12 bench problems (di, dinv, masses2_N4/N6, sys3x2_N3/N5),
and sys3x2_N7, masses2_N8, masses2_N10, masses3_N2 to N5, masses4_N2/N3, masses5_N2, pop_001, pop_006, pop_017,
rand_t6_x5_m20_s0 and s1. The earlier loss on masses3_N4 (`[12,31,33]`, radius 5.9e-6) is gone: masses3_N4 has 3643
regions with and without the certificate. So the certifying region there was not sound before the fix. I did not
identify which region it was; the loss disappears with the strays and the tight check, which is consistent with a
stray R that passed containment while not being the true region across F. The audit runs at 6 parameters are not
repeated beyond this set comparison.

## Tests

`cargo test --profile gate`, one 1-CPU task per test, tree with the fix: 142 passed, 0 failed, 3 ignored, plus two
long point-location tests (`pl_wnn`) still running when this was written (they passed in the previous gate).
New tests: `mplcp_no_gap_pop_081`, `mplcp_no_overlap_pop_011`.

## Round 2: the tight check is a flag, a QP decides

The verdict version lost one genuine sliver (masses3_N6, radius 3.4e-8). Final rule (`build_cand`): a candidate whose
tight residual exceeds 1e-3 is only flagged (census `TightFlagged`); a direct QP is solved at its Chebyshev centre (the
builder's centre, then the centre of the final rows) and the candidate is kept (`TightKeptByQp`) when the QP's active set
is the candidate's or the candidate's law gives the QP's primal solution to 1e-6; otherwise it is rejected
(`TightRejected`). `mplcp::set_tight_check(false)` switches the flag off for comparisons.

### Every removed region, nine problems (`examples/removed_check.rs`, flag on against flag off, same code)

"infeasible": the QP at the region's centre has no solution (centre outside the feasible set); "other": the QP's active
set differs and its solution differs from the region's law. "real" (QP agrees) must be 0.

| problem | regions flag off | flag on | removed | infeasible | other | real | only in the flag-on run |
|---|---:|---:|---:|---:|---:|---:|---:|
| pop_081 | 4 | 4 | 0 | 0 | 0 | 0 | 0 |
| pop_011 | 410 | 397 | 13 | 1 | 12 | 0 | 0 |
| pop_005 | 10297 | 10289 | 8 | 0 | 8 | 0 | 0 |
| pop_007 | 12245 | 12235 | 10 | 5 | 5 | 0 | 0 |
| pop_032 | 178965 | 178950 | 15 | 7 | 8 | 0 | 0 |
| pop_047 | 48146 | 48144 | 2 | 0 | 2 | 0 | 0 |
| masses3_N6 | 22152 | 22144 | 8 | 8 | 0 | 0 | 0 |
| masses3_N7 | 38363 | 38329 | 34 | 34 | 0 | 0 | 0 |
| masses4_N4 | 16051 | 16049 | 2 | 2 | 0 | 0 | 0 |

All 92 removed regions are bogus, and the sliver of masses3_N6 is kept (it is no longer in the removed list: 8 removed
instead of 9). The first version of the QP rule still lost one real region on masses4_N4 (`[1,9,53,56]`, radius 1.03e-8,
tight residual 1.4e-3): the QP flipped at the builder's centre of a region that thin; trying the centre of the final rows
as well restored it (masses4_N4 now removes 2, both infeasible). pop_081 is unaffected by the flag (its fix is the α
test). (pop_081's regions are 4 with the flag on or off.) "Flag off" differs from the pre-fix main only by the α test.

### Coverage before and after (135 problems, 4000 samples, certificate off), tree merged with main b5c8f6d

| | regions | lexico-infeasible | samples in two regions | samples in no region |
|---|---:|---:|---:|---:|
| main plus phase 1 | 1,097,907 | 0 | 212 | 3070 |
| this branch | 1,097,810 | 0 | 0 | 0 |

(main's phase 3 changed counts on some other problems too: masses3_N5 10441 to 10438, masses4_N3 2892 to 2891, masses4_N4
16044 to 16049, masses5_N3 5054 to 5062, pop_076, pop_091, rand_t6_x5_m20_s1, rand_t8_x6_m24_s0, rand_t10_x6_m24_s0 by 1.
That is phase 3, not this fix.)

### Certificate at the default range (mode 1: on at 3 to 5 parameters) against off

`cert_diff` (active-set comparison, both directions) on the 41 problems of the 135 with 3 to 5 parameters plus the seven
12 to 16 parameter problems of `bench/problems_highdim` (masses6 to masses8 at N=2, rand_t12 x3, rand_t14, rand_t16; for
more than 5 parameters mode 1 does nothing, since there are no clip vertices): **48 of 48 identical**, no active set in
only one run. Together with the mode 3 comparison on 27 problems above, nothing found where the certificate loses a
region. I propose mode 1 as the default. Mode 3 (6 parameters) stays off because the check costs more than the search
there (rand_t6_x5_m20_s0 0.74x in the eager version, neutral after).

### Cost of the tight check (serial, 1 CPU, release, main b5c8f6d against this branch before the last QP-centre change)

Per (problem, alternation) task all three binaries (main, a copy of main, branch) once in a shuffled order, ratio inside the
task, median over 3 to 5 tasks; the heavy problems run one pair per task. Ratio above 1 = branch faster.

| group | main / branch, median (min to max) | A/A main / copy of main |
|---|---|---|
| all 34 problems | 0.992 (0.388 to 1.815) | 1.000 (0.974 to 1.060) |
| 19 problems of 4 ms or more | 1.000 (0.984 to 1.815) | 0.999 (0.990 to 1.011) |
| di, dinv (13 problems, below 1 ms) | 0.92 to 1.06, median 0.975 | noise 0.97 to 1.06 |
| sys3x2_N3 to N7 | 0.982, 0.986, 0.984, 0.992, 0.991 | 0.99 to 1.00 |
| masses2_N4 to N8 | 0.994, 0.997, 0.995, 0.992, 1.000 | 0.996 to 1.011 |
| pop_001, pop_017, masses3_N4 | 0.996, 1.007, 1.021 | 1.00 |
| pop_005, pop_007, masses4_N4, masses3_N6, masses3_N7, pop_047 | 1.041, 1.039, 1.042, 1.056, 1.069, 1.003 | 0.996 to 1.004 |
| pop_011 | 1.815 (no stray regions to build) | 1.005 |
| pop_081 | 0.388 (it now finds the other 2 of 4 regions) | 1.019 |

The check costs 1 to 2% on the 3 to 60 ms problems (sys3x2 and masses2: 0.98 to 1.00 against an A/A of 0.99 to 1.01) and
is neutral or a gain (3 to 7%) on the large ones (fewer garbage regions to build). pop_032 was not timed (its pair tasks
were OOM-killed at 1900 MB; it needs a 2-CPU allocation). The binary timed predates the change that also tries the final rows' centre for
flagged candidates; that change touches only flagged candidates (rare).

## Tests

1-CPU task per test on the merged tree: 142 passed, 0 failed, 3 ignored, plus one long point-location test
(`pl_wnn::random_mpqp_theta8_b`) still running when this was written. `mplcp_no_gap_pop_081` and `mplcp_no_overlap_pop_011`
pass on the phase 3 structure (the α test sits before the empty-P return in the walk step).
