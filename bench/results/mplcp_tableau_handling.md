# Tableau handling in `mplcp`: lazy child pivots, buffer reuse, REFACTOR period

Task #40. Baseline: main at f017013 (the tableau law is the default). Final: branch `worktree-agent-a1191b1b32cfa5a78`
(lazy child pivots, REFACTOR 32), merged with main 1ee2e20. All runs on Negishi (AMD EPYC 7763, `rustc 1.92.0`), serial,
single-threaded, 1 CPU and 1 GB per task. The shared nodes were busy (see the loads below), so only ratios are used.

## 1. Profile: the tableau share is small

`lp_census` build (`MplcpTabNanos` covers the pending pivots and `tableau_of`; the from-scratch rebuild inside
`candidate_law` is counted under region build). Job 45869897, node a047, load average 57 to 71 at the time.

| params | problem | regions | solve ms (census build) | neighbour search | region build | tableau update |
|---:|---|---:|---:|---:|---:|---:|
| 3 | sys3x2_N3 | 211 | 4.2 | 33% | 55% | 2.8% |
| 3 | sys3x2_N4 | 503 | 11.5 | 33% | 55% | 2.9% |
| 3 | sys3x2_N5 | 973 | 24.5 | 28% | 57% | 3.5% |
| 3 | sys3x2_N6 | 1503 | 41.5 | 24% | 59% | 4.3% |
| 3 | sys3x2_N7 | 2095 | 59.3 | 20% | 61% | 4.8% |
| 4 | masses2_N4 | 307 | 18.6 | 52% | 42% | 0.8% |
| 4 | masses2_N5 | 493 | 31.5 | 53% | 42% | 1.1% |
| 4 | masses2_N6 | 901 | 62.6 | 53% | 41% | 1.3% |
| 4 | masses2_N7 | 1641 | 115.5 | 50% | 43% | 1.8% |
| 4 | masses2_N8 | 2837 | 209.5 | 48% | 44% | 2.0% |
| 6 to 10 | the 8 census problems | 370 to 6124 | 0.2 to 18 s | 2% to 73% | 26% to 97% | 0.0% to 0.3% |

The double integrator problems (7 to 11 regions, 0.1 to 0.7 ms) show 3.4% to 5.7%, but they are far below the resolvable
size. The census table's 13% (sys3x2_N7) and 10% (masses2_N8) were not reproduced: here the update is 4.8% and 2.0%, so the
ceiling for any tableau-handling idea is about 5% at 3 parameters and 2% at 4, and nothing at 6 or more.

## 2. Ideas, mechanism, outcome

### Idea 1, lazy child pivots (commit 534fff9): kept, for memory
`Item.tab` becomes `TabSrc`: `Ready(Tab)`, `Lazy(Arc<Tab>, v, kk)` (the parent tableau shared by all children plus the
pivot) or `None`. The pivot is done when the child is popped: `Arc::try_unwrap` succeeds for the last child popped, which
then pivots the parent's buffers in place (the same child as before, the last one pushed); the others take the copying
`pivoted`. Number of pivots and copies are unchanged (every child is popped), so no time gain is expected, but the
queue holds one tableau per expanding parent instead of one per frontier child. `pivot_solve_par` keeps `Ready` tableaux
(it converts with `From<Option<Tab>>`) and is otherwise untouched.

* Region hash: identical to main on all 37 problems at this commit (job 45869596).
* Serial A/B against main, one sequential 1-CPU job with all variants alternating (10 alternations, 7 repetitions, 23
  problems; jobs 45869898 on a025 and 45869899 on a047, load 48 to 66 at the end): median ratio base/c1 over the 9
  problems of 4 ms or more 1.0005 and 1.0009, over all 23 problems 0.998; the A/A copy gave 1.0005 and 1.0014 (all
  problems 1.0000 both times). No time effect, as expected.
* Peak memory (max RSS of `region_hash`, release, KB, Negishi; the numbers were identical to within 0.5% over 3 runs):

| problem | main | lazy pivots | change |
|---|---:|---:|---:|
| sys3x2_N7 (3 params) | 12168 | 11632 | -4.4% |
| masses2_N8 (4 params) | 27760 | 26272 | -5.4% |
| masses3_N4 (6) | 48056 | 44396 | -7.6% |
| masses4_N3 (8) | 47258 | 41944 | -11.2% |
| pop_017 (6) | 34928 | 28056 | -19.7% |
| pop_005 (7) | 97796 | 69784 | -28.6% |
| rand_t10_x5_m20_s0 (10) | 60564 | 60464 | -0.2% |

### Idea 3, buffer reuse (commit 46a5e62, reverted in 15705ef): rejected
`pivoted_into` wrote the child tableau into the buffers of a finished tableau kept in a pool (up to 64), with no
temporary vectors. Hashes identical. Timing in the same two jobs: median ratio base/pool 0.9936 and 0.9932 over all
problems, 1.0000 and 0.9947 over the 9 of 4 ms or more, inside the 1% noise band and not above 1. Memory rose instead
(the pool keeps tableaux alive): sys3x2_N7 14206 KB (+16.8% against main), masses2_N8 29440 (+6.1%), pop_005 71606
(-26.8%, less than lazy pivots alone). mimalloc already makes the allocations cheap. Not kept.

### Idea 2, REFACTOR period (commit da0cc79, 8 to 32): kept
The tableau reached by Case-1 pivots equals the rebuilt one in exact arithmetic (Lean `tableau_unique`); the question is
float drift. Drift was measured with an experiment build (not committed) that rebuilds every region's tableau from scratch at
pop time and compares it entry by entry with the pivoted one (matched by variable), with REFACTOR = 1000 so that depth
grows freely; relative difference |a-b|/max(1,|a|,|b|) over T and rho, largest per depth bucket (job 45869597,
37 problems):

| problem | depth 0 to 7 | depth 8 to 15 | depth 16 to 31 (deepest 28) |
|---|---:|---:|---:|
| sys3x2_N7 (T / rho) | 5.0e-13 / 3.1e-13 | 1.0e-12 / 4.7e-13 | 4.3e-12 / 8.1e-13 |
| sys3x2_N6 | 6.8e-13 / 3.3e-13 | 8.1e-13 / 3.4e-13 | 8.4e-12 / 4.3e-12 |
| masses2_N8 (deepest 15) | 2.1e-11 / 7.3e-11 | 3.5e-12 / 9.1e-12 | none |
| masses2_N7 | 1.8e-11 / 8.2e-12 | 2.8e-12 / 5.2e-12 | none |
| pop_005 (7 params) | 1.9e-5 / 3.0e-5 | 1.2e-6 / 6.2e-8 | none |

The drift does not grow with depth: the deep buckets are at the same size as depth 0 to 7 (and the depth 0 comparison
itself, where both sides are built from scratch by different pivot orders, is already at 1e-13 to 1e-5). The size is set by
the conditioning of the problem (pop_005 has 1e-5 at depth 0 to 7), not by the number of pivots. Only the 3 and 4 parameter
problems and pop_005 reach depth 8 or more at all, because every Case-2 neighbour resets the depth to 0.

Tableau law against the KKT law (`mplcp_law_diff`, serial, 37 problems, R = 8, 16, 32, 1000, same job): the worst
difference is 8.78e-8 (pop_017, equal for all R), and per problem it is the same for every R except sys3x2_N7 (primal law
3.6e-12 at R = 8 and 16, 5.0e-12 at R = 32 and 1000; multiplier law 1.4e-13 throughout). masses2_N8: 7.3e-9 and 2.5e-11 for
all R. So raising R does not move the existing 6e-9 level.

R = 8 against R = 32 directly (final code path, `refactor_cmp`, job 45873328, 35 distinct problem files): the active-set
sequence is identical in every problem (same regions, same order). Largest relative differences: facet rows 2.0e-12 and
primal law 8.7e-11 and multiplier law 2.1e-12 on everything except pop_005, where they are 1.6e-8, 8.9e-8 and 2.3e-8
(the ill-conditioned case above; its KKT-law gap at R = 8 is already 8.8e-8). Problems with no case of depth 8 or more
(masses3_N4, masses4_N3, pop_017, pop_021, rand_t10_x5_m20_s0, ...) have exactly 0.

Choice of period (A/B, the same two 1-CPU jobs, experiment binary with `RPOPT_REFACTOR`; base/variant, median over the
9 problems of 4 ms or more, run 45869898 and 45869899): R = 16 1.011 and 1.015; R = 32 1.011 and 1.018; R = 64 1.012 and
1.018 (A/A 1.0005 and 1.0014). Gains come from sys3x2 and masses2: sys3x2_N7 1.034, 1.056, 1.055 for R = 16, 32, 64
in job 45869899. 32 captures all of it, and since the tableau is still rebuilt every 32 levels it bounds the worst case. The
rebuild costs about |z| pivots, so each avoided rebuild saves that.

Region hash for R = 32 (job 45873327, 37 problem files): 34 identical, 3 differ (the two copies of masses2_N8 and
pop_005). In those the active-set sequence is identical and the facet rows differ by 2e-12 (masses2_N8) and 1.6e-8
(pop_005), so the 1e-9 rounding inside the hash flips for a few rows; `region_hash` rounds E and f to 1e-9, and a
difference of 2e-12 crosses a rounding boundary now and then. The same three differ for R = 16, 32, 64, 1000 and none
differs for R = 8 against main. These are not region changes: same count, same active sets, same order.

## 3. Final A/B (lazy pivots + REFACTOR 32) against main

Array 45873330: one task per (problem, alternation); each task runs main, a second copy of main (A/A) and the final
binary once each in a shuffled order on one node (median of 41 timed solves inside a task for the double integrators, 7
for the others), so the three always share the node and the moment. 145 tasks on 23 nodes; load average at task start:
median 56, minimum 22, maximum 109. Ratios are main/final within a task, then the median over the problem's tasks (5
alternations for the double integrators, 8 for the others).

| params | problem | regions | main ms | A/A | final | final, range over tasks |
|---:|---|---:|---:|---:|---:|---|
| 3 | sys3x2_N3 | 211 | 3.96 | 0.999 | 1.010 | 0.999 to 1.028 |
| 3 | sys3x2_N4 | 503 | 11.2 | 1.000 | 1.010 | 1.001 to 1.015 |
| 3 | sys3x2_N5 | 973 | 23.7 | 0.996 | 1.017 | 1.002 to 1.032 |
| 3 | sys3x2_N6 | 1503 | 40.5 | 0.983 | 1.032 | 1.006 to 1.046 |
| 3 | sys3x2_N7 | 2095 | 65.8 | 0.991 | 1.060 | 1.039 to 1.280 |
| 4 | masses2_N4 | 307 | 18.1 | 0.992 | 0.995 | 0.954 to 1.021 |
| 4 | masses2_N5 | 493 | 30.7 | 0.999 | 0.997 | 0.992 to 1.008 |
| 4 | masses2_N6 | 901 | 63.8 | 0.999 | 1.001 | 0.989 to 1.015 |
| 4 | masses2_N7 | 1641 | 117.8 | 0.994 | 1.013 | 0.998 to 1.344 |
| 4 | masses2_N8 | 2837 | 238.6 | 1.000 | 1.055 | 1.022 to 1.081 |

Summary over the 9 problems of 4 ms or more: final median 1.0135 (range 0.995 to 1.061); A/A median 0.9964 (range 0.983 to
1.000). Over all 23 problems: final 1.0000 (0.972 to 1.061), A/A 0.9993 (0.979 to 1.014). Sub-millisecond problems
(di, dinv: 0.07 to 0.7 ms) show 0.97 to 1.02 with ranges of up to 8% inside a task and are unreliable; they are not
evidence of either effect.

By parameter count: 3 parameters, +1% (N3) to +6% (N7), growing with the size because deeper trees reach the longer
pivot chains (the tableau update is 2.8% to 4.8% of the time there, and the gain is larger than the share because the
fewer from-scratch rebuilds also remove the rebuild cost counted under region build); 4 parameters, 0% to +5.5%, only
masses2_N7 and N8 above the noise; 6 to 10 parameters: no time effect (no state ever reaches depth 8, and the share is
at most 0.3%), only the memory change above (-0.2% to -29%).

## 4. Peak memory, final against main

Max RSS of `region_hash` (release, KB, one run each, deterministic to about 0.5%): jobs 45873329 and 45874957.

| problem | main | final | change |
|---|---:|---:|---:|
| sys3x2_N7 (3 params) | 12168 | 10992 | -9.7% |
| masses2_N8 (4) | 27760 | 21804 | -21.5% |
| masses3_N4 (6) | 48056 | 44272 | -7.9% |
| masses4_N3 (8) | 47258 | 41824 | -11.5% |
| pop_017 (6) | 34928 | 28092 | -19.6% |
| pop_005 (7) | 97796 | 69828 | -28.6% |
| rand_t10_x5_m20_s0 (10) | 60564 | 60420 | -0.2% |

## 5. Gates

* Release and `--features lp_census` builds compile (`cargo check --release --features lp_census --example lp_census`).
* Region hash: see sections 2 and 3 (identical at the lazy-pivot commit; differences only in the rounding of rows for
  R = 32, same active sets and order).
* Order gate: `mplcp_law_diff` (serial old, serial new, parallel new; job 45869597, R = 8, 16, 32, 1000): `par_order=true`
  for every problem and R; `order=false` for masses3_N3, masses3_N4, masses4_N3, masses5_N2, pop_005 and pop_021 is the
  existing tableau-law against KKT-law difference of region order (the same at R = 8 on main) and does not depend on R.
* Tests: see the table in the report (per-test array on the merged tree, one thread per test).

## 6. Reproducing

Experiment build (not committed): `REFACTOR` read from `RPOPT_REFACTOR` and `set_refactor`, a `DRIFT` check in the pop
loop of `pivot_solve`, and the `refactor_cmp` example. The scripts are in `~/rpopt/worktree-agent-a1191b1b32cfa5a78/`
on Negishi (`scripts/`, `bin/`, `results/`).
