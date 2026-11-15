# mplcp Case-2 phase 3: leaf pruning, sub-facet hints, walk limits

Branch `mplcp-case2-p3b`, on main 52f3103 (the integration of the tableau work, the builder work and Case-2
phases 1 and 2, certificate off). One commit per item:

| item | commit |
|---|---|
| 1. leaf pruning | 8588380 |
| 2. sub-facet hint | d40a98a (census table e635845) |
| 3. explicit stack, exact cycle check, pivot budget with fallback | 3c90b2c, budget a500310 and 144c156, layout d35a133 |

All timings are serial, 1 CPU, on Negishi (AMD EPYC 7763), release profile, one SLURM array task per (problem,
alternation) with every binary under comparison run once in a shuffled order on the same node; the ratio is taken
inside the task and the problem's ratio is the median over its 4 to 5 tasks. Slowest problems (above 5 s) run each
pair in its own task. Ratios above 1 mean the second binary is faster. Binaries: `main` = candidate main ed13e70
(before the Case-2 merge and the start-region fix), `integration` = main 52f3103 (what the items are built on),
`final` = the last commit of the branch. A/A is the same binary copied and run in the same tasks.

## What the walk does (counters, deterministic, one run per problem)

`lp_census` build, table 3e of `lp_census` (job array 45880603). "before" is the integration build with the old
`depth > 6` and `8n + 50` caps, "after" the final build.

| params | problem | regions | LP/region before | LP/region after | pivot_facet Chebyshev LPs/region (after) | leaf branches skipped | old hint outside the leaf sub-facet | depth > 6 calls (after) | budget stops | fallbacks found / failed |
|---:|---|---:|---:|---:|---:|---|---|---:|---:|---|
| 3 | sys3x2_N7 | 2095 / 2095 | 1.1 | 1.1 | 0.0 | 0 of 0 | 28 of 52 (54%) | 0 | 0 | 0 / 0 |
| 4 | masses2_N8 | 2837 / 2837 | 3.7 | 3.7 | 2.7 | 0 of 0 | 2137 of 4072 (52%) | 0 | 0 | 0 / 0 |
| 5 | rand_t5_x5_m20_s2 | 2301 / 2301 | 8.6 | 8.6 | 7.6 | 0 of 0 | 8591 of 11947 (72%) | 0 | 0 | 0 / 0 |
| 6 | masses3_N3 | 955 / 955 | 49.9 | 51.8 | 20.5 | 61 of 336 | 5588 of 7975 (70%) | 308 | 3 | 0 / 3 |
| 6 | pop_017 | 4142 / 4142 | 37.2 | 37.2 | 13.3 | 0 of 13 | 31340 of 43796 (72%) | 1 | 0 | 0 / 0 |
| 7 | pop_005 | 10297 / 10297 | 73.6 | 56.8 | 21.8 | 2954 of 5708 | 30012 of 44721 (67%) | 5923 | 99 | 0 / 99 |
| 7 | pop_021 | 679 / 679 | 39.3 | 39.3 | 18.8 | 0 of 0 | 7587 of 10169 (75%) | 0 | 0 | 0 / 0 |
| 8 | masses4_N3 | 2892 / 2891 | 87.4 | 92.9 | 51.6 | 718 of 2310 | 38075 of 46073 (83%) | 2516 | 26 | 3 / 23 |
| 8 | rand_t8_x5_m20_s0 | 4111 / 4111 | 32.3 | 32.3 | 10.2 | 0 of 0 | 28423 of 40792 (70%) | 0 | 0 | 0 / 0 |
| 10 | masses5_N2 | 370 / 370 | 66.5 | 65.2 | 31.6 | 22 of 231 | 4415 of 5304 (83%) | 321 | 8 | 1 / 7 |
| 12 | masses6_N2 | 380 / 380 | 77.3 | 73.0 | 35.5 | 60 of 289 | 4968 of 5948 (84%) | 370 | 10 | 2 / 8 |
| 12 | rand_t12_x5_m15_s0 | 1012 / 1012 | 34.3 | 34.3 | 8.3 | 0 of 0 | 5945 of 8364 (71%) | 0 | 0 | 0 / 0 |
| 12 | rand_t12_x5_m20_s0 | 6143 / 6143 | 84.9 | 48.4 | 21.3 | 706 of 2839 | 74018 of 97475 (76%) | 3251 | 117 | 11 / 106 |
| 14 | masses7_N2 | 390 / 390 | 71.7 | 74.1 | 33.5 | 27 of 237 | 5366 of 6375 (84%) | 312 | 5 | 1 / 4 |
| 14 | rand_t14_x5_m15_s0 | 2470 / 2470 | 41.0 | 40.8 | 10.9 | 0 of 4 | 20028 of 26892 (74%) | 36 | 1 | 1 / 0 |
| 16 | masses8_N2 | 394 / 395 | 77.7 | 78.0 | 33.8 | 21 of 256 | 5425 of 6475 (84%) | 359 | 8 | 0 / 8 |
| 16 | rand_t16_x5_m15_s0 | 1876 / 1876 | 44.4 | 44.5 | 10.1 | 0 of 0 | 13709 of 18950 (72%) | 0 | 0 | 0 / 0 |

* Leaf pruning skips 0% to 52% of the leaf branches that would have run a Chebyshev LP (25% on rand_t12_x5_m20_s0,
  52% on pop_005, 31% on masses4_N3), and none on the 3 to 5 parameter problems, whose Case-2 walks have no
  branching leaves.
* The old hint (the parent centre projected onto the row) lay outside the leaf's own sub-facet for 52% to 84% of the
  returned neighbours (76% on rand_t12_x5_m20_s0).
* The exact cycle check never fired (0 cycle stops in every run). The old caps cut real work: the walks that went
  past depth 6 are the "depth > 6 calls" column (6,000 calls on pop_005, 3,300 on rand_t12_x5_m20_s0).
* A walk that is allowed to run is very long: depth up to 1,890 on masses5_N2 and 987 on pop_005, 10,000-pivot
  walks on rand_t12_x5_m20_s0. With the budget at its first value (2^n capped at 20,000 pivots) masses5_N2 (0.5 s with the
  caps), masses6_N2 (1 s), pop_005 (9 s) and rand_t12_x5_m20_s0 (14 s) did not finish in 400 s to 1,500 s, with the same
  regions as the capped runs wherever they finished. The budget therefore had to be much smaller.

## Item 1: leaf pruning (8588380)

Before the Chebyshev LP of a Case-2 branch whose next basis is complementary (alpha leaves), the active set is looked
up in the `Seen` map; if it is known the LP and the return are skipped (`expand_candidates` would drop it anyway;
Lean `reach_skip`). Region hashes: identical to the integration build on all 49 problems of the standard set
(23 bench problems, 7 generated random problems at 3 to 5 parameters, the 12 census problems at 6 to 10 parameters, and the 7
`problems_highdim` at 12 to 16 parameters), job array 45880601. Counters: the "leaf branches skipped" column above. Time:
see the by-parameters table (column "leaf pruning"): 0.99 to 1.01, no resolvable gain, because the LPs it removes are
a few percent of the Chebyshev LPs of `pivot_facet`.

## Item 2: sub-facet hint (d40a98a)

`pivot_facet` computed the sub-facet Chebyshev centre and dropped it. It is now returned with each basis, mapped
back by theta = N theta_f - g b, and is that candidate's hint (the old hint stays for Toggle neighbours and for bases
reached without a branch LP). Counter: the "old hint outside" column above.

This is not LP-start-only. The Chebyshev centre found from another start can differ (the optimum is not unique), and
the redundancy step of the region build depends on the centre (clip order and box-vertex witnesses), so rows at
tolerance edges flip. Hash differences against the integration build: masses4_N3 (2,892 to 2,891 regions, 73,041 to 73,015 facet rows), masses8_N2 (394
to 395 regions, 32,565 to 32,635 rows), masses7_N2 (390 regions, 27,197 to 27,198 rows); the other 46 are identical. A
sampled coverage check (`lexcheck`, 20,000 points) finds no gap and no overlap in either build on those three. Time:
column "sub-facet hint" below, 1.00 to 1.03 (neutral to slightly faster).

## Item 3: explicit stack, exact cycle check, pivot budget (3c90b2c and later)

`pivot_facet` runs on an explicit stack of branch frames with no depth limit and no iteration cap. Every walk keeps the
state right after its first pivot, and a repeated state must return there (Lean `walk_repeat_returns`), so
equality with it is an exact cycle check. The pivot budget per facet is `2n` (at least 64; `WALK_BUDGET_PER_ROW`,
`WALK_BUDGET_MIN`). A cycle or a spent budget is counted (`WalkCycle`, `WalkBudget`) and the facet also gets the
QP just beyond the Chebyshev centre of the facet (`FallbackAcross`, `FallbackFail`: the QP fails when the point is
outside the feasible set, which is what happens at a facet on the boundary of the parameter set). The Lean bound on the
walk (distinct states, at most 2^n of them) is correct and useless as a cost limit, see above.

Choice of the budget (job array 45881978, release builds; an earlier sweep on gate builds is job array 45879140; ratio of the cap version to the variant, above 1 is faster; the
regions found in brackets):

| problem | n | cap version ms | budget 32 ms (regions) | budget 64 ms (regions) | 2n (min 64) ms (regions) | 4n ms (regions) |
|---|---|---:|---:|---:|---:|---:|
| pop_005 | 10297 reg | 8423.9 | 3906.1 (x2.16; 10294) | 4689.8 (x1.79; 10294) | 5348.3 (x1.57; 10297) | 7462.0 (x1.13; 10297) |
| pop_017 | 4142 reg | 796.0 | 818.4 (x0.98; 4142) | 815.2 (x0.97; 4142) | 808.5 (x0.99; 4142) | 814.7 (x0.98; 4142) |
| pop_021 | 679 reg | 178.1 | 184.5 (x0.96; 679) | 185.1 (x0.96; 679) | 183.8 (x0.97; 679) | 184.8 (x0.96; 679) |
| masses3_N3 | 955 reg | 338.1 | 327.0 (x1.03; 955) | 352.1 (x0.96; 955) | 360.5 (x0.93; 955) | 390.7 (x0.86; 955) |
| masses3_N4 | 3643 reg | 2102.6 | 2013.1 (x1.05; 3643) | 2286.8 (x0.92; 3643) | 2565.6 (x0.82; 3643) | 3014.9 (x0.70; 3643) |
| masses4_N3 | 2892 reg | 3808.5 | 3284.0 (x1.16; 2891) | 3880.3 (x0.98; 2891) | 4298.1 (x0.89; 2891) | 5155.5 (x0.74; 2891) |
| masses5_N2 | 370 reg | 505.1 | 416.2 (x1.22; 370) | 493.8 (x1.02; 370) | 512.2 (x0.98; 370) | 761.9 (x0.66; 370) |
| masses6_N2 | 380 reg | 934.8 | 706.0 (x1.33; 380) | 912.2 (x1.03; 380) | 986.2 (x0.95; 380) | 1432.1 (x0.65; 380) |
| masses7_N2 | 390 reg | 1452.1 | 1338.8 (x1.09; 390) | 1575.2 (x0.92; 390) | 1639.7 (x0.88; 390) | 2120.7 (x0.69; 390) |
| masses8_N2 | 394 reg | 2257.5 | 1904.9 (x1.19; 395) | 2217.1 (x1.02; 395) | 2292.7 (x0.98; 395) | 2753.1 (x0.82; 395) |
| rand_t8_x5_m20_s0 | 4111 reg | 1078.5 | 1108.7 (x0.97; 4111) | 1098.6 (x0.98; 4111) | 1097.6 (x0.98; 4111) | 1099.3 (x0.98; 4111) |
| rand_t10_x5_m20_s0 | 6124 reg | 2856.9 | 2923.9 (x0.98; 6124) | 2938.4 (x0.98; 6124) | 2910.5 (x0.99; 6124) | 2930.4 (x0.98; 6124) |
| rand_t12_x5_m20_s0 | 6143 reg | 13539.6 | 5177.8 (x2.62; 6143) | 5924.6 (x2.30; 6143) | 5868.4 (x2.31; 6143) | 6709.1 (x2.04; 6143) |
| rand_t14_x5_m15_s0 | 2470 reg | 2077.5 | 2079.2 (x1.00; 2470) | 2078.1 (x1.00; 2470) | 2090.1 (x0.99; 2470) | 2099.2 (x0.99; 2470) |
| rand_t16_x5_m15_s0 | 1876 reg | 2059.5 | 2067.4 (x0.99; 1876) | 2062.5 (x1.00; 1876) | 2053.3 (x1.00; 1876) | 2071.0 (x1.00; 1876) |
budget 32: geomean speedup over the cap version 1.187, min 0.963, max 2.620
budget 64: geomean speedup over the cap version 1.080, min 0.919, max 2.301
2n (min 64): geomean speedup over the cap version 1.042, min 0.819, max 2.313
4n: geomean speedup over the cap version 0.905, min 0.653, max 2.038

Budget 32 is the fastest (geometric mean 1.19x over the cap version, 2.2x on pop_005, 2.6x on rand_t12_x5_m20_s0) but pop_005 then
returns 10,294 regions instead of 10,297. `2n` keeps the regions and is 4% faster than the cap version on the geometric mean (0.82x on
masses3_N4, 0.89x on masses4_N3, 0.88x on masses7_N2, 2.3x on rand_t12_x5_m20_s0, 1.6x on pop_005). The constants are
named so that the choice can be changed in one place.

Region changes against the integration build, final build: regions equal on 47 of 49 problems (masses4_N3
2,892 to 2,891 and masses8_N2 394 to 395, both from item 2); facet rows differ on pop_005 (+9 rows, same 10,297
regions), and the hash alone differs on masses3_N4 and rand_t12_x5_m20_s0 (same counts of regions and rows; rows at tolerance
edges). Coverage by `lexcheck` (20,000 samples, job array 45880602), integration build against final build: 0 gaps and 0 lexico-infeasible
regions in both on every problem below; overlaps only on pop_005 (the known stray overlapping regions): 326 sampled
points in two regions before, 191 after.

```
ti masses3_N3 regions 955 lex_infeasible 0 feasible_samples 5459 overlaps 0 gaps 0
ti masses3_N4 regions 3643 lex_infeasible 0 feasible_samples 4971 overlaps 0 gaps 0
ti masses4_N3 regions 2892 lex_infeasible 0 feasible_samples 3351 overlaps 0 gaps 0
ti masses5_N2 regions 370 lex_infeasible 0 feasible_samples 2848 overlaps 0 gaps 0
ti masses6_N2 regions 380 lex_infeasible 0 feasible_samples 1847 overlaps 0 gaps 0
ti masses7_N2 regions 390 lex_infeasible 0 feasible_samples 1258 overlaps 0 gaps 0
ti masses8_N2 regions 394 lex_infeasible 0 feasible_samples 861 overlaps 0 gaps 0
ti pop_005 regions 10297 lex_infeasible 0 feasible_samples 6992 overlaps 326 gaps 0
ti pop_017 regions 4142 lex_infeasible 0 feasible_samples 3193 overlaps 0 gaps 0
ti rand_t12_x5_m20_s0 regions 6143 lex_infeasible 0 feasible_samples 8653 overlaps 0 gaps 0
ti rand_t14_x5_m15_s0 regions 2470 lex_infeasible 0 feasible_samples 9989 overlaps 0 gaps 0
i3 masses3_N3 regions 955 lex_infeasible 0 feasible_samples 5459 overlaps 0 gaps 0
i3 masses3_N4 regions 3643 lex_infeasible 0 feasible_samples 4971 overlaps 0 gaps 0
i3 masses4_N3 regions 2891 lex_infeasible 0 feasible_samples 3351 overlaps 0 gaps 0
i3 masses5_N2 regions 370 lex_infeasible 0 feasible_samples 2848 overlaps 0 gaps 0
i3 masses6_N2 regions 380 lex_infeasible 0 feasible_samples 1847 overlaps 0 gaps 0
i3 masses7_N2 regions 390 lex_infeasible 0 feasible_samples 1258 overlaps 0 gaps 0
i3 masses8_N2 regions 395 lex_infeasible 0 feasible_samples 861 overlaps 0 gaps 0
i3 pop_005 regions 10297 lex_infeasible 0 feasible_samples 6992 overlaps 191 gaps 0
i3 pop_017 regions 4142 lex_infeasible 0 feasible_samples 3193 overlaps 0 gaps 0
i3 rand_t12_x5_m20_s0 regions 6143 lex_infeasible 0 feasible_samples 8653 overlaps 0 gaps 0
i3 rand_t14_x5_m15_s0 regions 2470 lex_infeasible 0 feasible_samples 9989 overlaps 0 gaps 0
```

## Effect by parameter count (final against integration and against candidate main, job array 45880604)

Geometric mean over the problems of that parameter count, range in brackets. "main / final" also contains the Case-2
phases 1 and 2 and the start-region fix (masses8_N2 returns 0 regions on candidate main and its ratio is not meaningful).

| params | problems | leaf pruning | sub-facet hint | stack, cycle check, budget | integration / final | main / final | A/A |
|---:|---:|---|---|---|---|---|---|
| 2 | 13 | 0.992 (0.982 to 1.008) | 1.003 (0.992 to 1.019) | 0.985 (0.971 to 0.997) | 0.985 (0.964 to 1.000) | 1.052 (0.987 to 1.168) | 1.002 (0.982 to 1.027) |
| 3 | 6 | 0.990 (0.978 to 1.000) | 1.016 (1.012 to 1.025) | 0.986 (0.957 to 1.000) | 0.985 (0.955 to 1.003) | 1.048 (0.968 to 1.118) | 1.007 (0.998 to 1.012) |
| 4 | 7 | 0.997 (0.991 to 1.004) | 1.004 (1.001 to 1.008) | 0.983 (0.965 to 0.996) | 0.983 (0.962 to 0.996) | 0.995 (0.962 to 1.038) | 1.000 (0.996 to 1.005) |
| 5 | 4 | 0.995 (0.992 to 0.998) | 1.003 (1.000 to 1.005) | 0.978 (0.973 to 0.983) | 0.978 (0.974 to 0.980) | 0.975 (0.961 to 0.987) | 1.001 (0.998 to 1.003) |
| 6 | 4 | 1.003 (0.998 to 1.014) | 1.012 (1.002 to 1.026) | 0.917 (0.802 to 0.984) | 0.970 (0.942 to 0.989) | 1.164 (0.981 to 1.453) | 0.998 (0.992 to 1.008) |
| 7 | 2 | 1.002 (0.995 to 1.010) | 1.014 (1.010 to 1.018) | 1.220 (0.976 to 1.525) | 0.980 (0.980 to 0.980) | 1.305 (1.115 to 1.526) | 1.002 (1.001 to 1.002) |
| 8 | 3 | 0.999 (0.995 to 1.004) | 1.001 (0.991 to 1.008) | 0.950 (0.879 to 0.988) | 0.992 (0.986 to 0.998) | 1.051 (0.988 to 1.187) | 1.000 (1.000 to 1.001) |
| 10 | 3 | 1.009 (1.007 to 1.011) | 0.999 (0.992 to 1.007) | 0.991 (0.985 to 0.995) | 0.997 (0.992 to 1.002) | 1.046 (0.998 to 1.148) | 1.001 (0.998 to 1.005) |
| 12 | 3 | 1.017 (1.009 to 1.029) | 0.993 (0.983 to 1.003) | 1.285 (0.948 to 2.245) | 0.980 (0.951 to 1.009) | 1.494 (0.990 to 3.246) | 1.000 (0.995 to 1.002) |
| 14 | 2 | 1.000 (0.999 to 1.002) | 1.012 (1.011 to 1.012) | 0.936 (0.871 to 1.007) | 0.947 (0.879 to 1.021) | 1.011 (0.991 to 1.031) | 0.992 (0.987 to 0.997) |
| 16 | 2 | 1.008 (1.007 to 1.009) | 1.000 (0.995 to 1.005) | 0.985 (0.973 to 0.998) | 0.993 (0.987 to 1.000) | 0.006 (0.000 to 1.011) | 0.974 (0.945 to 1.003) |

An extra A/B for the layout fix (`inline(never)` on the walk, hint only for returned neighbours; job array 45881973),
integration against d35a133 and the previous commit against it:

| params | problems | integration / i4 | i3 / i4 | A/A |
|---:|---:|---|---|---|
| 2 | 13 | 0.985 (0.974 to 0.997) | 1.003 (0.993 to 1.026) | 0.996 (0.985 to 1.008) |
| 3 | 6 | 0.994 (0.949 to 1.034) | 1.004 (0.992 to 1.014) | 0.994 (0.968 to 1.021) |
| 4 | 7 | 0.982 (0.973 to 0.991) | 0.991 (0.977 to 0.998) | 1.006 (0.998 to 1.022) |
| 5 | 4 | 0.986 (0.970 to 1.001) | 1.005 (0.999 to 1.013) | 1.000 (0.998 to 1.001) |
| 6 | 4 | 0.932 (0.818 to 0.984) | 1.009 (0.984 to 1.075) | 0.999 (0.977 to 1.028) |
| 7 | 2 | 1.311 (0.977 to 1.757) | 1.003 (0.992 to 1.014) | 1.007 (1.002 to 1.011) |
| 8 | 3 | 0.961 (0.887 to 1.006) | 0.988 (0.916 to 1.043) | 1.013 (1.009 to 1.017) |
| 10 | 3 | 0.990 (0.986 to 0.993) | 0.993 (0.984 to 1.000) | 1.003 (1.001 to 1.006) |
| 12 | 3 | 1.366 (1.008 to 2.309) | 1.001 (0.999 to 1.003) | 1.050 (0.999 to 1.157) |
| 14 | 2 | 0.935 (0.875 to 0.999) | 1.001 (0.998 to 1.004) | 1.002 (0.997 to 1.007) |
| 16 | 2 | 0.996 (0.992 to 1.000) | 1.003 (1.003 to 1.004) | 0.999 (0.999 to 1.000) |

At 2 to 5 parameters the final build is 1% to 2% slower than the integration build (0.98 to 0.99 in both runs, A/A 1.00).
The cause is not known: pivot_facet is almost never entered there, and the layout commit did not change it.

## Per problem

| params | problem | regions (main / final) | main ms | leaf pruning | sub-facet hint | stack, cycle check, budget | integration / final | main / final | A/A |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|
| 2 | di_N10 | 11 / 11 | 0.160 | 0.987 | 0.993 | 0.981 | 0.981 | 1.032 | 0.982 |
| 2 | di_N12 | 11 / 11 | 0.191 | 1.000 | 1.006 | 0.994 | 0.994 | 1.073 | 1.005 |
| 2 | di_N15 | 11 / 11 | 0.245 | 1.005 | 1.004 | 0.991 | 1.000 | 1.086 | 1.000 |
| 2 | di_N20 | 11 / 11 | 0.365 | 0.994 | 1.000 | 0.997 | 0.988 | 1.126 | 1.011 |
| 2 | di_N25 | 11 / 11 | 0.478 | 0.995 | 1.000 | 0.995 | 0.990 | 1.166 | 1.011 |
| 2 | di_N30 | 11 / 11 | 0.655 | 0.993 | 1.009 | 0.977 | 0.991 | 1.168 | 0.998 |
| 2 | di_N4 | 9 / 9 | 0.075 | 0.987 | 1.000 | 0.986 | 0.987 | 0.987 | 1.000 |
| 2 | di_N6 | 11 / 11 | 0.111 | 0.982 | 1.000 | 0.973 | 0.964 | 0.991 | 0.982 |
| 2 | di_N8 | 11 / 11 | 0.135 | 1.008 | 1.000 | 0.984 | 0.984 | 1.039 | 1.007 |
| 2 | di_doc_N3 | 7 / 7 | 0.064 | 0.984 | 1.016 | 0.984 | 0.969 | 1.000 | 1.000 |
| 2 | dinv_N4 | 9 / 9 | 0.069 | 0.985 | 1.000 | 0.971 | 0.971 | 1.000 | 1.000 |
| 2 | dinv_N6 | 11 / 11 | 0.113 | 0.991 | 1.019 | 0.982 | 0.991 | 1.009 | 1.027 |
| 2 | dinv_N8 | 11 / 11 | 0.132 | 0.992 | 0.992 | 0.985 | 0.992 | 1.023 | 1.007 |
| 3 | rand_t3_x5_m20_s0 | 123 / 123 | 3.133 | 0.987 | 1.012 | 0.957 | 0.955 | 0.968 | 1.006 |
| 3 | sys3x2_N3 | 211 / 211 | 3.936 | 1.000 | 1.015 | 0.978 | 0.991 | 1.036 | 1.011 |
| 3 | sys3x2_N4 | 503 / 503 | 11.824 | 0.978 | 1.025 | 0.991 | 0.968 | 1.021 | 1.006 |
| 3 | sys3x2_N5 | 973 / 973 | 23.390 | 0.993 | 1.018 | 0.993 | 0.995 | 1.064 | 1.012 |
| 3 | sys3x2_N6 | 1503 / 1503 | 38.621 | 0.993 | 1.012 | 1.000 | 1.003 | 1.090 | 0.998 |
| 3 | sys3x2_N7 | 2095 / 2095 | 58.262 | 0.993 | 1.012 | 0.997 | 0.999 | 1.118 | 1.008 |
| 4 | masses2_N4 | 307 / 307 | 17.547 | 0.991 | 1.005 | 0.977 | 0.970 | 0.979 | 0.996 |
| 4 | masses2_N5 | 493 / 493 | 30.343 | 0.995 | 1.004 | 0.988 | 0.988 | 0.996 | 0.998 |
| 4 | masses2_N6 | 901 / 901 | 60.217 | 0.992 | 1.001 | 0.996 | 0.996 | 1.001 | 0.997 |
| 4 | masses2_N7 | 1641 / 1641 | 112.971 | 0.999 | 1.006 | 0.989 | 0.991 | 1.018 | 1.000 |
| 4 | masses2_N8 | 2837 / 2837 | 204.807 | 0.998 | 1.008 | 0.991 | 0.996 | 1.038 | 1.005 |
| 4 | rand_t4_x5_m20_s0 | 797 / 797 | 38.115 | 0.998 | 1.001 | 0.965 | 0.962 | 0.973 | 1.004 |
| 4 | rand_t4_x5_m20_s1 | 484 / 484 | 29.180 | 1.004 | 1.005 | 0.978 | 0.981 | 0.962 | 0.998 |
| 5 | rand_t5_x5_m20_s0 | 1423 / 1423 | 139.940 | 0.994 | 1.000 | 0.973 | 0.974 | 0.982 | 1.003 |
| 5 | rand_t5_x5_m20_s1 | 1136 / 1136 | 138.483 | 0.992 | 1.005 | 0.983 | 0.980 | 0.961 | 0.998 |
| 5 | rand_t5_x5_m20_s2 | 2301 / 2301 | 174.401 | 0.998 | 1.005 | 0.980 | 0.980 | 0.987 | 1.002 |
| 5 | rand_t5_x6_m24_s0 | 5036 / 5036 | 569.487 | 0.997 | 1.003 | 0.976 | 0.979 | 0.969 | 0.999 |
| 6 | masses3_N3 | 955 / 955 | 455.140 | 1.014 | 1.016 | 0.913 | 0.942 | 1.257 | 0.992 |
| 6 | masses3_N4 | 3643 / 3643 | 3665.776 | 1.003 | 1.026 | 0.802 | - | 1.453 | 1.008 |
| 6 | pop_017 | 4142 / 4142 | 831.019 | 0.999 | 1.003 | 0.984 | 0.989 | 1.024 | 0.998 |
| 6 | rand_t6_x5_m20_s0 | 1774 / 1774 | 245.260 | 0.998 | 1.002 | 0.980 | 0.981 | 0.981 | 0.995 |
| 7 | pop_005 | 10298 / 10297 | 8071.880 | 1.010 | 1.018 | 1.525 | - | 1.526 | 1.002 |
| 7 | pop_021 | 679 / 679 | 198.837 | 0.995 | 1.010 | 0.976 | 0.980 | 1.115 | 1.001 |
| 8 | masses4_N3 | 2892 / 2891 | 5009.726 | 1.004 | 0.991 | 0.879 | - | 1.187 | 1.001 |
| 8 | pop_027 | 1472 / 1472 | 563.798 | 0.995 | 1.008 | 0.988 | 0.998 | 0.988 | 1.000 |
| 8 | rand_t8_x5_m20_s0 | 4111 / 4111 | 1035.797 | 0.997 | 1.003 | 0.987 | 0.986 | 0.990 | 1.000 |
| 10 | masses5_N2 | 370 / 370 | 588.474 | 1.007 | 1.007 | 0.985 | 0.992 | 1.148 | 1.000 |
| 10 | rand_t10_x5_m15_s0 | 1859 / 1859 | 622.785 | 1.011 | 0.992 | 0.995 | 0.998 | 0.998 | 0.998 |
| 10 | rand_t10_x5_m20_s0 | 6124 / 6124 | 2859.039 | 1.010 | 0.997 | 0.994 | 1.002 | 1.000 | 1.005 |
| 12 | masses6_N2 | 380 / 380 | 1015.263 | 1.009 | 0.993 | 0.948 | 0.951 | 1.039 | 0.995 |
| 12 | rand_t12_x5_m15_s0 | 1012 / 1012 | 510.038 | 1.029 | 0.983 | 0.998 | 1.009 | 0.990 | 1.002 |
| 12 | rand_t12_x5_m20_s0 | 6143 / 6143 | 18879.599 | 1.013 | 1.003 | 2.245 | - | 3.246 | 1.002 |
| 14 | masses7_N2 | 390 / 390 | 1674.372 | 0.999 | 1.011 | 0.871 | 0.879 | 1.031 | 0.997 |
| 14 | rand_t14_x5_m15_s0 | 2470 / 2470 | 2379.707 | 1.002 | 1.012 | 1.007 | 1.021 | 0.991 | 0.987 |
| 16 | masses8_N2 | 0 / 395 | 0.073 | 1.007 | 1.005 | 0.973 | 0.987 | 0.000 | 0.945 |
| 16 | rand_t16_x5_m15_s0 | 1876 / 1876 | 2075.121 | 1.009 | 0.995 | 0.998 | 1.000 | 1.011 | 1.003 |

## Tests and gates

* `cargo test --profile gate` on the final tree (144c156), one 1-CPU task per test (array 45882792, 144 tests listed):
  141 passed, 0 failed, 3 ignored.
* Both builds compile: default and `--features lp_census` (`cargo check` with `--tests --examples`).
* Region hashes, coverage and the serial timings are in the sections above. The pre-existing `same=false` of
  `rand_t5_x6_m24_s0` against `geometric` (see `mplcp_build_speed.md`) is unchanged.
