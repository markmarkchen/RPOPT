# mplp_lex: an enumeration path for small mpLPs

`mplp_lex` now has two ways to produce the lexicographic partition of an mpLP with H = 0 (cost independent of theta): the facet-to-facet
walk of Jones, Kerrigan and Maciejowski (unchanged) and an enumeration of bases. A cost-model gate picks one. The enumeration is used on
2 of the 46 mpLPs (masses5inf_N1 and masses6inf_N1, the two that lost to `combinatorial`), where it is 1.5x to 1.7x faster than the walk
and makes `mplp_lex` the fastest solver on both.

## Why the enumeration gives exactly the walk's partition

Setting: min c'x s.t. A x <= b + F theta, theta in Theta, H = 0. A basis B is a set of n_x independent rows containing the equality rows;
beta = A_B^-1, lambda = -A_B^-T c. Let s_j(theta) = (b_j + F_j theta) - A_j beta (b_B + F_B theta) be the slack of a nonbasic row j, an affine
function of theta. The perturbed program of Jones et al. (b -> b + (eps, eps^2, ..), c -> c + (delta, delta^2, ..)) has, for every theta,
exactly one optimal basis (it is primal and dual nondegenerate), the lex-optimal basis B*(theta); the sets R_B = {theta : B = B*(theta)}
partition the feasible Theta (their Prop. 14).

1. **B is lex-optimal at theta iff** (D) every basic inequality row p has a lexicographically positive multiplier row
   [ lambda_p | -beta_(.,p) ] (the delta columns in order of the variable index), and (P) every nonbasic row j has a lexicographically
   positive slack row [ s_j(theta) | e_j - T_j E_B ] (the eps columns in order of the constraint index, T = A beta). This is the walk's own
   definition (`slack_sign`, `lam_row`). For H = 0 the row in (D) does not contain theta: (D) is a property of B alone.
2. **Interior of the closed region.** Let C_B = {theta in Theta : s_j(theta) >= 0 for all nonbasic j} (the closed region of B without the
   multiplier rows, which are constants for H = 0 and are covered by (D)). If theta is an interior point of C_B, every s_j is either > 0 at theta
   or identically zero: an affine function that is >= 0 on a set with interior and vanishes at an interior point vanishes identically. Rows with
   s_j > 0 satisfy (P) at theta whatever their eps part. The identically zero rows ("Z rows", primal degeneracy: more than n_x rows active at
   x*(theta) for all theta in the region) have s_j(theta) = 0 for all theta, so their sign is the sign of e_j - T_j E_B, again a constant of B.
   Hence, for theta in int C_B: B = B*(theta) iff (D) and (Z), where (Z) says that every Z row has a lexicographically positive eps-block row.
3. **Full dimension.** R_B is contained in C_B (lex-positive implies >= 0). So if R_B has interior, C_B has interior and (D), (Z) hold (by 2,
   applied at an interior point of R_B inside int C_B). Conversely if (D), (Z) hold and C_B has interior, then int C_B is contained in R_B by 2.
   Therefore  {B : R_B full dimensional} = {B : (D) and (Z) and C_B full dimensional}.
4. **The walk produces that set.** Its regions are `gen_cr(B)` of the bases it reaches, kept if the Chebyshev radius of the closed region is above
   1e-8. By Theorem 22 of Jones et al. the graph of bases whose regions are full dimensional and meet facet to facet is connected, so the walk
   reaches every B with full-dimensional R_B. Both paths then call the same deterministic function `gen_cr_from_active_set_lambda_first(program, B)`
   on the same sorted basis, so the regions, laws and rows are identical, not just equal as sets.

What the code does: `dual_lex_feasible_bases` loops over all k-subsets of the non-equality rows (k = n_x - n_eq), solves A_B' lambda = -c by Gaussian
elimination and keeps B when every lambda_p > 1e-9 (a multiplier within 1e-9 of zero is decided by the full lexicographic row from `lam_row`).
`candidate_ok` then evaluates (Z) (a slack with constant and linear parts both below 1e-12 relative counts as zero, as in the walk) and drops a
candidate whose slack is negative over the whole box of Theta; `gen_cr` finishes with the full-dimension test. The pruning of `combinatorial`
(a pruning list of infeasible active sets, one feasibility LP per set) is replaced by (D), which is exact, needs no LP for H = 0, and is
evaluated by a 3x3 to 8x8 elimination. Without the zero-row check (Z) the enumeration returned extra regions overlapping walk regions on
dione_N2 (22 against 20) and masses2inf_N2 (214 against 202): primal degenerate bases of one vertex. With it the counts agree.

Not covered: H != 0 (the multiplier rows then depend on theta; the gate refuses), more than 8 variables (`ENUM_MAX_NX`), equality rows not at
the start. Those use the walk. If the enumeration returns no region the walk runs (infeasible programs keep their warning).

## Checks

- Region hashes (`examples/region_hash`: active set and facet rows rounded to 1e-9, sorted): the default path equals the walk on all 46 mpLPs
  (`bench/problems_lp` and `bench/problems_lp_highdim`), and the walk equals main. The enumeration forced on (`MPLP_LEX_MODE=2`) equals the walk on
  every mpLP where it can be run (11: masses5inf_N1, masses6inf_N1, five random mpLPs with 5 to 12 parameters including the 12,105-region
  randlp_t10_x6_m50_s0, diinf_N2, dione_N2, masses2inf_N2). The other 35 have more than 8 variables or more than 3e7 subsets.
- `tests/mplp_lex_enum.rs` (own binary, switches the global mode): six random mpLPs and three degenerate MPC problems, enumeration against walk, same
  active sets and rows; and the gate decision on two problems.

## The switch rule

Cost model of the enumeration pass: the number of subsets C(m, n_x - n_eq) times about 4 ns * n_x^2 (measured per subset: 23 ns at n_x = 3, 57 ns at
n_x = 5, 42 ns at n_x = 6, 120 ns on randlp_t10_x6 whose bases are mostly dual feasible; the model is conservative except for that last
one). The pass is run only if its predicted cost is at most 1 ms (`ENUM_BUDGET_NS`). The budget is the largest loss that can occur: if the
enumeration did not pay, the walk would run after at most 1 ms, and the smallest walk in the benchmarks (diinf_N2, dione_N2) takes 0.3 ms
but has 134,596 and 735,471 subsets, which the model prices at 20 ms and 188 ms, so it is never run there. After the pass nothing is
abandoned: for H = 0 the later cost is the region builds, which the walk pays too, plus rejected candidates at about one LP each, while the
walk pays about 25 facet LPs per region on top of its builds.

Calibration set: all 46 mpLPs, one run of `examples/enum_stats` (subsets, dual-feasible bases, regions, enumeration and walk times, laptop):

| problem | n_x | subsets | dual lex-feasible | regions | pass | enumeration path | walk |
|---|---:|---:|---:|---:|---:|---:|---:|
| masses6inf_N1 | 3 | 11,480 | 408 | 52 | 0.26 ms | 18 ms | 32 ms |
| masses5inf_N1 | 3 | 8,436 | 340 | 48 | 0.19 ms | 9.6 ms | 15 ms |
| diinf_N2 | 6 | 134,596 | 1,175 | 20 | 6.4 ms | 9.7 ms | 0.33 ms |
| dione_N2 | 8 | 735,471 | 1,110 | 20 | 46 ms | 49 ms | 0.35 ms |
| randlp_t12_x5_m40_s0 | 5 | 962,598 | 872 | 29 | 53 ms | 63 ms | 4.0 ms |
| randlp_t5_x5_m40_s0 | 5 | 1,086,008 | 2,385 | 562 | 63 ms | 104 ms | 32 ms |
| randlp_t5_x5_m40_s1 | 5 | 501,942 | 10,034 | 1,563 | 62 ms | 156 ms | 90 ms |
| randlp_t8_x5_m40_s0 | 5 | 749,398 | 1,252 | 557 | 43 ms | 96 ms | 75 ms |
| masses2inf_N2 | 6 | 3,838,380 | 10,376 | 202 | 160 ms | 194 ms | 8.7 ms |

Below about 20,000 subsets the enumeration wins; at 130,000 subsets and above it loses by 1.3x to 140x, and there is no problem in between, so the
constants (4 ns, 1 ms) are not fitted to individual problems. The 35 other mpLPs have more than 8 variables or more than 3e7 subsets.

## Serial A/B on Negishi (build 45883324, array 45883325; 1 CPU, 5 alternations, walk / automatic / walk again, shuffled, same binary)

| problem | n | cand/base median | range | A/A median | range | cand median ms |
|---|---|---|---|---|---|---|
| masses6inf_N1 | 5 | 0.579 | 0.573-0.586 | 1.004 | 0.990-1.007 | 35.0 |
| masses5inf_N1 | 5 | 0.650 | 0.639-0.660 | 0.994 | 0.982-1.006 | 18.4 |
| masses4one_N1 | 5 | 0.995 | 0.888-0.996 | 0.987 | 0.921-1.004 | 440.7 |
| randlp_t12_x5_m40_s0 | 5 | 1.001 | 0.589-1.104 | 1.005 | 0.999-1.334 | 7.9 |
| randlp_t5_x5_m40_s0 | 5 | 1.010 | 0.969-1.047 | 1.011 | 0.951-1.048 | 57.3 |
| diinf_N2 | 5 | 1.006 | 0.987-1.015 | 0.997 | 0.991-1.002 | 0.3 |
| dione_N2 | 5 | 1.004 | 0.994-1.014 | 1.006 | 0.995-1.007 | 0.3 |
| masses2inf_N2 | 5 | 0.998 | 0.992-1.001 | 0.999 | 0.995-1.010 | 13.4 |

The two enumerated problems run 0.58x and 0.65x of the walk; the six problems where the gate chooses the walk are at 1.00 (the gate costs nothing
measurable; the A/A pair brackets the noise of 0.95 to 1.05 on the sub-10-ms problems).

## Timing against the best other solver

16 high-dimensional mpLPs (`mplp_lex` from the A/B runs, medians; other solvers from the step-1 runs of `mplp_lex_highdim.md`; different nodes, so about 10% near 1):

| problem | parameters | mplp_lex ms | path | best other | its ms | lex / best other |
|---|---:|---:|---|---|---:|---:|
| masses3inf_N2 | 6 | 187.0 | walk | combinatorial | 657.4 | 0.28 |
| masses3inf_N3 | 6 | 3653.3 | walk | graph | 189854.9 | 0.02 |
| masses3one_N2 | 6 | 3431.8 | walk | geometric | 128204.5 | 0.03 |
| masses4inf_N2 | 8 | 1900.2 | walk | combinatorial_graph | 14585.1 | 0.13 |
| masses4one_N1 | 8 | 434.5 | walk | combinatorial | 468.3 | 0.93 |
| masses5inf_N1 | 10 | 18.4 | enumeration | combinatorial | 19.5 | 0.94 |
| masses5inf_N2 | 10 | 12854.2 | walk | none finished | | |
| masses5one_N1 | 10 | 3283.6 | walk | combinatorial | 4654.5 | 0.71 |
| masses6inf_N1 | 12 | 35.0 | enumeration | combinatorial | 42.0 | 0.83 |
| masses6one_N1 | 12 | 23649.6 | walk | combinatorial | 50442.6 | 0.47 |
| randlp_t5_x5_m40_s0 | 5 | 54.1 | walk | combinatorial_graph | 126.6 | 0.43 |
| randlp_t5_x5_m40_s1 | 5 | 155.1 | walk | combinatorial_graph | 276.6 | 0.56 |
| randlp_t6_x5_m40_s1 | 6 | 338.0 | walk | combinatorial_graph | 445.7 | 0.76 |
| randlp_t8_x5_m40_s0 | 8 | 147.7 | walk | combinatorial_graph | 167.9 | 0.88 |
| randlp_t10_x6_m50_s0 | 10 | 8592.1 | walk | combinatorial_graph | 8131.7 | 1.06 |
| randlp_t12_x5_m40_s0 | 12 | 7.6 | walk | geometric | 17.4 | 0.44 |
fastest on 14 of 15

`bench/problems_lp`, all six solvers once on Negishi (array 45883424, 1 CPU, 120 s timeout):

| problem | mplp_lex ms | best other | its ms | lex / best other |
|---|---:|---|---:|---:|
| diinf_N2 | 0.34 | geometric | 1.08 | 0.32 |
| diinf_N3 | 0.92 | geometric | 2.59 | 0.36 |
| diinf_N4 | 2.32 | geometric | 5.22 | 0.44 |
| diinf_N5 | 3.85 | geometric | 7.91 | 0.49 |
| diinf_N6 | 7.72 | geometric | 11.55 | 0.67 |
| diinf_N8 | 22.36 | geometric | 88.27 | 0.25 |
| diinf_N10 | 53.91 | geometric | 803.36 | 0.07 |
| dione_N2 | 0.32 | geometric | 0.83 | 0.39 |
| dione_N3 | 0.87 | geometric | 7.16 | 0.12 |
| dione_N4 | 1.93 | geometric | 14.45 | 0.13 |
| dione_N5 | 3.82 | geometric | 36.82 | 0.10 |
| dione_N6 | 6.82 | geometric | 118.43 | 0.06 |
| dione_N8 | 16.45 | geometric | 501.28 | 0.03 |
| dione_N10 | 31.45 | geometric | 1175.71 | 0.03 |
| masses2inf_N2 | 14.47 | combinatorial | 55.35 | 0.26 |
| masses2inf_N3 | 96.56 | graph | 1091.19 | 0.09 |
| masses2inf_N4 | 368.72 | graph | 8567.50 | 0.04 |
| masses2inf_N5 | 1109.53 | geometric | 58612.21 | 0.02 |
| masses2one_N2 | 48.62 | geometric | 89.99 | 0.54 |
| masses2one_N3 | 319.30 | geometric | 794.54 | 0.40 |
| masses2one_N4 | 1322.60 | geometric | 6381.48 | 0.21 |
| masses2one_N5 | 4660.50 | geometric | 65357.56 | 0.07 |
| sys3x2inf_N2 | 7.07 | geometric | 158.43 | 0.04 |
| sys3x2inf_N3 | 35.38 | geometric | 1316.84 | 0.03 |
| sys3x2inf_N4 | 131.53 | geometric | 8932.23 | 0.01 |
| sys3x2inf_N5 | 390.17 | geometric | 26489.39 | 0.01 |
| sys3x2one_N2 | 10.54 | geometric | 42.27 | 0.25 |
| sys3x2one_N3 | 58.90 | geometric | 430.81 | 0.14 |
| sys3x2one_N4 | 262.86 | geometric | 3265.68 | 0.08 |
| sys3x2one_N5 | 773.47 | geometric | 15949.72 | 0.05 |

`mplp_lex` is fastest on 30 of 30. (A first version of this table counted `graph` on dione_N6, N8 and N10 as faster; those runs return 0 regions
with 1,337 uncovered sample points, so they are failures and are excluded; see the round-5 section below.)

## Limits of the rule

The gate prices the pass only. After the pass the enumeration always continues, and its remaining extra cost is the candidates that
`gen_cr` rejects (about 20 us each: 294 of 346 on masses6inf_N1). That is safe when few dual lex-feasible bases lack a full-dimensional region
(ratio candidates to regions 6.6 and 7.1 here), but nothing in the rule bounds the ratio for an unseen problem: an mpLP with at most 27,000
subsets, all of them dual feasible and empty, would pay up to about 0.5 s against a walk of a few milliseconds. A bound would need the region
count in advance, which is what the walk computes. On the 46 mpLPs the rule never chooses the enumeration where the walk is faster.

## Not done

Option (3), marking which kept rows have a full-dimensional facet, was not attempted.

## Round 5: the two non-wins

### dione_N6: `graph` was a failed run, not a win

`lp_census` on dione_N6 shows `mplp_lex` with 73 regions (3.4 ms) and `graph` with 0 regions. `mplp_bench` confirms: `graph` returns 0 regions and
leaves 1,337 of 2,000 sampled parameters uncovered; the same happens on dione_N8 and dione_N10 (`bench/results/` earlier work on zero-region runs). Excluding
runs with 0 regions, gaps or errors, the best valid other solver on dione_N6 is `geometric` at 118 ms against `mplp_lex` 6.8 ms (0.06x), on N8 501 ms
against 16.5 ms, on N10 1,176 ms against 31 ms. `mplp_lex` is fastest on all 30 `bench/problems_lp` problems. The earlier claim of a loss is withdrawn.
No code change.

### randlp_t10_x6_m50_s0: a real loss of 6% to `combinatorial_graph`

Paired comparison (array 45885374): one 1-CPU task per alternation, both solvers run once in shuffled order on the same node (a243, load 65), 5 alternations.
`mplp_lex` 8,542 / 8,566 / 8,516 / 8,533 / 8,531 ms, `combinatorial_graph` 8,026 / 8,045 / 8,071 / 8,046 / 8,013 ms: ratios 1.064, 1.065, 1.055, 1.060, 1.065
(median 1.064, spread 1.055 to 1.065). So it is a loss, not noise.

Census, once each (laptop-independent, Negishi, `lp_census`, build 45885630, array 45885631; both 12,105 regions):

| LP purpose | `mplp_lex` calls | ms | `combinatorial_graph` calls | ms |
|---|---:|---:|---:|---:|
| redundancy | 234,873 | 2,055 | 234,875 | 2,050 |
| bounding box | 242,125 | 2,822 | 242,125 | 2,846 |
| neighbour (facet centers) | 202,733 | 1,974 | 0 | 0 |
| feasibility (candidate sets) | 12,110 | 3 | 1,088,792 | 468 |
| Chebyshev | 24,220 | 286 | 12,110 | 192 |
| all LPs | | 7,141 | | 5,556 |
| solve time with census | | 8,686 | | 8,386 |

The region build (redundancy and box LPs, 4.9 s) is identical. `mplp_lex` pays 2.0 s for facet LPs where `combinatorial_graph` pays 0.5 s for
feasibility LPs; outside LPs `mplp_lex` is cheaper (1.5 s against 2.8 s). Facet counters of the walk: 246k facets tried (134k more skipped because the
neighbour already crossed them), 55k settled by the region-center foot, 190k by a facet LP, and only 4 of those were empty. So these are real facets:
each of the 12,105 regions has about 35 facet rows, every interior facet is crossed once, about 20 per region.

An attempt to raise the foot hit rate with up to 8 alternating projections inside the facet plane onto violated rows settled 68k instead of 55k facets
but was not faster (4.68 against 4.46 s on the laptop) and was reverted. The gap (0.3 s of 8.4 s) would close if the facet LPs cost 15% less; no exact
change in `mplp_lex` alone does that. The remaining options are the ones named before: a facet interior point from the builder (which
already finds one for many rows while testing redundancy), or a warm-started facet LP.

## Round 6: warm-starting the facet LP from a point on the facet (measured upper bound, not implemented)

Idea: the builder's per-row redundancy LP returns a point on each facet it decides by an LP; hand it to `mplp_lex` as the start of the facet
Chebyshev LP. Before touching the shared builder and `CriticalRegion` (and its serialisation), the best case was measured: on
randlp_t10_x6_m50_s0 (laptop, gate profile, walk), for each of the 190,628 facets that need an LP, a vertex of the facet was computed (random
objective over the facet polytope, untimed) and the facet Chebyshev LP was solved cold and with `lp_point_from` starting at (vertex, r = 0):

| facet Chebyshev LP | total time | per LP |
|---|---:|---:|
| cold (`lp_point`) | 969 ms | 5.1 us |
| warm from a facet vertex | 827 ms | 4.3 us |

Same optimum on all 190,628. The saving is 15% of the facet-LP time, 0.14 s of a 4.5 s walk (3%), against a gap to `combinatorial_graph` of 6%.
A builder LP returns a vertex of the facet (a lower-dimensional face), so it cannot pass the relative-interior margin test either, and it exists
only for the rows the builder decides by an LP (the others are decided by the box, a bounding-box vertex or the projection of the center,
which is the foot already used here). So the export cannot win this problem and was not implemented; no code changed.
The facet LPs are 1.97 s of 8.7 s with the census overhead; their cold cost per LP is already small (the 202,733 LPs average 9.7 us with the census
clock reads, 5.1 us without), so the walk is near the floor of its facet-by-facet design on this problem.

## Round 7: direct crossing again, with a gate that targets where it lost

Direction check: the round-2 ratios were candidate time over base time, so direct crossing was 5 to 14% faster on the random mpLPs (0.86 to 0.95)
and 7 to 40% slower on the problems with slivers and on the 2 to 4 parameter problems. This round re-applies it (same pivot, same safeguards: neighbour
radius above 1e-4, reversed facet present, rows matched by value, ties fall back to the LP path) behind a gate.

Why it lost, from timers inside the walk (laptop): (1) on the sliver MPC problems 30 to 45% of the attempts end in a fresh `gen_cr` of a neighbour that is
then rejected (a numerically marginal or empty facet, where the LP path would have skipped the facet), 11 us each against a 6 us LP saved; the
per-region context and probes add a few percent more; (2) at 2 to 4 parameters the facet LP costs 1 to 5 us, no more than the pivot, the
check and the reverse-facet test.

The gate (all exact criteria or counts, no clocks, so runs are reproducible):
- direct crossing only for 5 or more parameters (`DIRECT_MIN_NT`);
- a running success rate s and a running rate q of attempts that fail after a fresh region build (averages over the last 32 attempts) decide per
  region: try the region when s > 0.17 + 1.8 q (an attempt costs about 0.17 of the LP it saves, a wasted build 1.8 of it, measured on the 6 to 12
  parameter problems); the first 8 regions are tried; otherwise a region is tried as a probe, and each lost probe doubles the wait for the next (4, 8, ..., at most 256 regions).
Both paths return the same neighbour or the LP path decides, so the gate changes speed only.

Gates passed:
- Region hashes identical to the walk without direct crossing on all 46 mpLPs, with `MPLP_LEX_MODE=1` (walk) and with the default path (enumeration on the two small
  problems). The enumeration code is untouched. `examples/overlap_check` (every region's Chebyshev centre lies in that region only): 0 overlaps on
  masses4inf_N2, masses3inf_N2, randlp_t5_x5_m40_s1 and randlp_t6_x5_m40_s1; `tests/mplp_lex_enum.rs` passes.
- Serial A/B on Negishi (build 45887919, array 45887920): 46 problems, 3 alternations each, one 1-CPU task per (problem, alternation), base (main) / candidate / base again
  in shuffled order; region counts equal and 0 gaps/errors on all.

| problem | cand/base median | range | A/A median | A/A range | cand ms | regions equal, 0 gaps/errs |
|---|---:|---|---:|---|---:|---|
| diinf_N10 | 1.003 | 0.997-1.004 | 1.000 | 0.997-1.011 | 53.6 | yes |
| diinf_N2 | 1.002 | 1.001-1.017 | 0.997 | 0.997-0.999 | 0.3 | yes |
| diinf_N3 | 1.008 | 1.007-1.013 | 0.997 | 0.995-1.006 | 0.9 | yes |
| diinf_N4 | 1.012 | 0.998-1.012 | 1.002 | 0.994-1.002 | 1.9 | yes |
| diinf_N5 | 0.994 | 0.992-1.004 | 0.996 | 0.996-0.998 | 3.8 | yes |
| diinf_N6 | 1.004 | 1.001-1.006 | 1.005 | 0.999-1.008 | 7.4 | yes |
| diinf_N8 | 1.001 | 0.997-1.005 | 1.003 | 0.994-1.005 | 21.2 | yes |
| dione_N10 | 1.004 | 0.998-1.010 | 1.006 | 1.005-1.008 | 30.1 | yes |
| dione_N2 | 1.008 | 1.001-1.014 | 0.996 | 0.989-1.012 | 0.3 | yes |
| dione_N3 | 1.019 | 1.018-1.020 | 1.004 | 1.000-1.009 | 0.9 | yes |
| dione_N4 | 1.001 | 0.999-1.013 | 1.003 | 1.000-1.005 | 1.9 | yes |
| dione_N5 | 1.005 | 1.002-1.006 | 1.005 | 0.992-1.005 | 3.8 | yes |
| dione_N6 | 1.005 | 1.003-1.012 | 0.992 | 0.990-1.010 | 6.2 | yes |
| dione_N8 | 0.996 | 0.996-1.002 | 1.000 | 0.998-1.000 | 14.5 | yes |
| masses2inf_N2 | 0.995 | 0.945-1.029 | 1.025 | 1.005-1.105 | 15.6 | yes |
| masses2inf_N3 | 0.996 | 0.996-0.999 | 0.999 | 0.998-1.002 | 93.1 | yes |
| masses2inf_N4 | 0.995 | 0.991-0.996 | 1.002 | 0.995-1.004 | 367.4 | yes |
| masses2inf_N5 | 0.998 | 0.995-1.005 | 1.000 | 0.997-1.004 | 1114.2 | yes |
| masses2one_N2 | 1.002 | 0.983-1.013 | 0.998 | 0.985-1.005 | 48.5 | yes |
| masses2one_N3 | 1.001 | 0.998-1.005 | 0.997 | 0.997-1.002 | 316.1 | yes |
| masses2one_N4 | 1.035 | 0.969-1.062 | 1.027 | 0.972-1.034 | 1446.8 | yes |
| masses2one_N5 | 0.995 | 0.992-1.002 | 1.031 | 1.026-1.032 | 4769.4 | yes |
| masses3inf_N2 | 1.015 | 1.008-1.015 | 0.999 | 0.999-1.001 | 181.0 | yes |
| masses3inf_N3 | 1.000 | 0.996-1.000 | 0.999 | 0.997-1.001 | 3557.9 | yes |
| masses3one_N2 | 0.989 | 0.982-1.009 | 0.978 | 0.974-0.996 | 3528.9 | yes |
| masses4inf_N2 | 1.013 | 0.996-1.048 | 0.998 | 0.987-1.051 | 1983.8 | yes |
| masses4one_N1 | 0.994 | 0.968-1.050 | 0.999 | 0.995-1.013 | 450.6 | yes |
| masses5inf_N1 | 1.014 | 1.005-1.023 | 1.001 | 0.998-1.001 | 19.0 | yes |
| masses5inf_N2 | 0.980 | 0.964-1.003 | 0.999 | 0.952-1.027 | 12919.0 | yes |
| masses5one_N1 | 0.978 | 0.973-0.988 | 0.998 | 0.988-1.007 | 3205.5 | yes |
| masses6inf_N1 | 0.997 | 0.994-1.008 | 0.999 | 0.996-0.999 | 34.5 | yes |
| masses6one_N1 | 0.970 | 0.969-0.971 | 1.001 | 0.992-1.009 | 22410.1 | yes |
| randlp_t10_x6_m50_s0 | 0.898 | 0.896-0.912 | 1.000 | 0.991-1.003 | 7709.8 | yes |
| randlp_t12_x5_m40_s0 | 0.948 | 0.946-0.949 | 1.002 | 0.998-1.007 | 7.1 | yes |
| randlp_t5_x5_m40_s0 | 0.868 | 0.855-0.870 | 0.992 | 0.991-1.008 | 47.6 | yes |
| randlp_t5_x5_m40_s1 | 0.916 | 0.882-0.977 | 0.998 | 0.923-1.008 | 153.5 | yes |
| randlp_t6_x5_m40_s1 | 0.864 | 0.800-0.909 | 0.977 | 0.891-1.007 | 335.9 | yes |
| randlp_t8_x5_m40_s0 | 0.891 | 0.882-0.990 | 1.094 | 0.977-1.095 | 142.3 | yes |
| sys3x2inf_N2 | 1.000 | 0.994-1.005 | 0.996 | 0.980-1.002 | 7.4 | yes |
| sys3x2inf_N3 | 0.993 | 0.985-1.006 | 0.988 | 0.988-1.013 | 37.6 | yes |
| sys3x2inf_N4 | 0.988 | 0.929-1.014 | 1.010 | 0.932-1.011 | 140.1 | yes |
| sys3x2inf_N5 | 1.017 | 1.012-1.018 | 1.008 | 1.001-1.008 | 421.1 | yes |
| sys3x2one_N2 | 1.000 | 0.997-1.006 | 1.000 | 0.995-1.010 | 10.8 | yes |
| sys3x2one_N3 | 1.009 | 0.993-1.021 | 1.001 | 1.000-1.013 | 60.3 | yes |
| sys3x2one_N4 | 0.994 | 0.993-1.005 | 0.999 | 0.995-1.006 | 236.7 | yes |
| sys3x2one_N5 | 0.999 | 0.999-1.001 | 1.001 | 0.994-1.005 | 773.5 | yes |

The 2 to 4 parameter problems and the sliver MPC problems are at 0.97 to 1.02 (A/A 0.95 to 1.03 on the noisy ones); the largest values (masses2one_N4 1.035) have
an A/A of 1.027 in the same row. The random mpLPs are 5 to 14% faster (randlp_t10_x6_m50_s0 0.898), the large 1-norm MPC problems 2 to 3% faster.

Paired against `combinatorial_graph` on randlp_t10_x6_m50_s0 (array 45887921, 5 alternations, both solvers in the same task, shuffled; one run each):
`mplp_lex` 7,747 / 7,691 / 7,687 / 7,715 / 7,757 ms, `combinatorial_graph` 7,973 / 7,954 / 7,970 / 7,953 / 7,975 ms: ratio 0.972, 0.967, 0.964, 0.970, 0.973 (median
0.970). `mplp_lex` now wins that problem by 3%, so it is the fastest solver on every mpLP of both sets that has a valid competitor.
