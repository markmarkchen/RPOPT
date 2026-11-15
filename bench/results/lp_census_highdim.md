# LP census at 3 to 10 parameters: why mplcp and geometric lose ground to graph-type methods

Measurement on 14 problems of `bench/problems_large` (3 to 10 parameters), serial algorithms, census build
(`cargo run --release --features lp_census --example lp_census -- --problems ... --highdim`). Counts are exact.
Times were taken on a busy shared machine (1-minute load 4 to 65 during the runs), so only ratios and shares are
used, never absolute times. The census adds two clock reads per LP call. Each pair ran once or three times
(median reported when the first solve took under 5 s). Not run: geometric on masses3_N4 and masses4_N3 and
combinatorial on sys3x2_N7 exceeded the 40 s cap with the census on.

Parameter counts: sys3x2_N7 3; masses2_N8 4; masses3_N3, masses3_N4, rand_t6_x5_m20_s0, pop_017 6; pop_021, pop_005 7;
masses4_N3, rand_t8_x5_m20_s0, pop_027 8; masses5_N2, rand_t10_x5_m15_s0, rand_t10_x5_m20_s0 10.

## Summary

1. The ratio in the question reproduces from `large.json` (geometric mean over problems solved by both, 0-region runs
   excluded): mplcp/graph 0.17, 0.57, 0.45, 0.91, 1.29, 1.44, 1.66, 1.89, 1.86 for 1 to 10 parameters. geometric/graph
   follows the same path (0.23, 0.84, 0.70, 1.02, 2.32, 2.04, 2.10, 2.41, 2.83), while combinatorial_graph/graph
   rises to 1.90 at 4 parameters and falls back to 0.91 to 0.94 at 8 to 10. So the decline belongs to the two
   region-to-region walkers, and it is the walkers' neighbour search that grows, not the shared region builder.
2. Per region, graph and combinatorial_graph pay for the region build only. At 8 to 10 parameters their time is
   dominated by the 2n bounding-box LPs of the LP redundancy path (43 to 79% of solve time at 8 and 10 parameters,
   for graph 64% and 71 to 75% on the random family) and about two feasibility LPs per region (1.4 candidates per region
   on the random family). Their cost does not depend on the number of facets.
3. mplcp pays for the same build plus a neighbour search that visits every facet: `adjacent` runs 15 to 33 times per
   region (equal to facets per region, which grows from 6 at 3 parameters to 29 to 36 at 10). Of those calls 24 to 65%
   at 6 to 10 parameters (3% at 3 parameters, 26% at 4) leave the fast path and enter `pivot_facet` (Case 2). Case 2 is
   94 to 100% of the time spent inside `adjacent` at 6 or more parameters, and the neighbour search is 39 to 80% of
   mplcp's solve time at 7 to 10 parameters (4 to 51% at 3 to 6, where the region build dominates).
4. geometric pays one facet-centre Chebyshev LP per facet plus a facet-step QP: 22 to 60 neighbour LPs and 7 to 17
   QPs per region at 6 to 10 parameters, which is 41 to 52% of its time at 8 to 10 parameters (80% on masses5_N2, 396
   neighbour LPs per region, 43 facets).
5. Both walkers redo work: 82 to 98% of the candidates returned by `adjacent` are already known regions (fast path
   82 to 98%, slow path 84 to 99%). The cost of a neighbour search grows with the facet count while the number of
   new regions per region stays near one.
6. Clipping caps (hypothesis d) are a large constant cost for every solver at 7 or more parameters, but lifting them
   does not help: see "Part 2".

## Hypotheses

| hypothesis | verdict | evidence |
|---|---|---|
| (a) cost per facet differs and grows differently | supported, main cause | mplcp and geometric do work per facet that needs an LP or a tableau pass (see Tables 1, 3, 5); graph and combinatorial_graph do work per candidate (1.4 to 8 per region at 6 to 10 parameters), independent of the facet count. Facets per region rise from 6 (3 parameters) to 30 (10). |
| (b) problem mix | partly | the family `rand_t6/t8/t10_x5_m20_s0` (same x, same 20 constraints, only the parameter count changes) shows mplcp/graph = 1.3, 1.9, 1.9 to 2.1 and geometric/graph = 2.5 to 2.8, 4.2, 2.5 to 4.3 in two census runs (1.65, 1.78, 2.79 and 2.18, 2.32, 2.57 in `large.json`), so the trend is there without a change of mix, but milder and noisy. But mix does matter at 3 to 4 parameters: graph visits 303 (sys3x2_N7) and 32 (masses2_N8) candidates per region, which is why mplcp looks 5 to 50 times faster there. |
| (c) degeneracy | partly, in the slow path | Case-2 share of `adjacent` calls rises from 3% (sys3x2_N7) and 26% (masses2_N8) to 24 to 65% at 6 to 10 parameters, but Chebyshev empty-facet answers are 0 to 4% on the random family and pop_027 and 28 to 80% on the masses problems and pop_005/017/021, so Case 2 on the random family is mostly non-degenerate branching that returns one candidate. The exits at the hard limits (below) show genuine degeneracy on the masses problems and pop_005. |
| (d) clipping caps | real, but shared | all 14 problems at 7 or more parameters take the LP path in the shipped code (100% `nt outside 2..=6`). It costs 2n bounding-box LPs per region and 1 to 20 row LPs. It hits all four solvers, graph hardest in relative terms. It does not explain the divergence, and lifting the caps makes everything slower. |

## Part 1 tables

All tables are for the shipped code (caps at 2 to 6 parameters). "us/region" is solve time divided by the final region
count (noisy). Shares are of total solve time. LP counts include LPs of every purpose; QPs are listed separately.

Table 0. Geometric mean of microseconds per region by parameter count, over the problems in the list (the mix changes
with the count, so read the controlled family in the Hypotheses table too).

| params | problems | mplcp us/region | graph | combinatorial_graph | geometric | facets/region | mplcp / graph | geometric / graph |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 3 | 1 | 13 | 763 | 2998 | 27 | 6.1 | 0.02 | 0.04 |
| 4 | 1 | 42 | 156 | 683 | 56 | 8.5 | 0.27 | 0.36 |
| 6 | 4 | 1215 | 1141 | 908 | 1022 | 15.7 | 1.07 | 0.90 |
| 7 | 2 | 384 | 104 | 135 | 253 | 18.0 | 3.71 | 2.44 |
| 8 | 3 | 303 | 145 | 138 | 225 | 25.9 | 2.10 | 1.55 |
| 10 | 3 | 369 | 179 | 194 | 939 | 35.8 | 2.07 | 5.26 |

Table 1. Cost per region (facets per region = rows of E kept by the builder).

| params | problem | algorithm | regions | solve ms | us/region | LP/region | QP/region | LP+QP share | facets/region | build ms/region |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 3 | sys3x2_N7 | geometric | 2095 | 58 | 27 | 3.6 | 1.2 | 33% | 6.1 | 0.01 |
| 3 | sys3x2_N7 | graph | 2095 | 1599 | 763 | 414.7 | 0.0 | 13% | 6.1 | 0.01 |
| 3 | sys3x2_N7 | combinatorial_graph | 2095 | 6281 | 2998 | 2158.8 | 0.0 | 10% | 6.1 | 0.01 |
| 3 | sys3x2_N7 | mplcp | 2095 | 27 | 13 | 1.1 | 0.0 | 10% | 6.1 | 0.01 |
| 4 | masses2_N8 | geometric | 2837 | 159 | 56 | 5.6 | 3.1 | 29% | 8.5 | 0.02 |
| 4 | masses2_N8 | graph | 2837 | 441 | 156 | 49.2 | 0.0 | 18% | 8.5 | 0.02 |
| 4 | masses2_N8 | combinatorial_graph | 2837 | 1936 | 683 | 340.2 | 0.0 | 29% | 8.5 | 0.03 |
| 4 | masses2_N8 | mplcp | 2837 | 120 | 42 | 3.7 | 0.0 | 10% | 8.5 | 0.02 |
| 4 | masses2_N8 | combinatorial | 2837 | 20821 | 7339 | 2646.7 | 0.0 | 31% | 8.5 | 0.19 |
| 6 | masses3_N3 | geometric | 955 | 668 | 699 | 61.9 | 14.7 | 26% | 16.2 | 0.37 |
| 6 | masses3_N3 | graph | 955 | 1744 | 1827 | 9.2 | 0.1 | 1% | 16.8 | 1.81 |
| 6 | masses3_N3 | combinatorial_graph | 955 | 615 | 644 | 15.7 | 0.0 | 4% | 17.8 | 0.62 |
| 6 | masses3_N3 | mplcp | 955 | 739 | 774 | 38.3 | 0.0 | 10% | 16.2 | 0.85 |
| 6 | masses3_N3 | combinatorial | 955 | 307 | 322 | 12.1 | 0.0 | 6% | 16.2 | 0.31 |
| 6 | masses3_N4 | graph | 3642 | 16185 | 4444 | 15.6 | 0.0 | 1% | 16.6 | 4.42 |
| 6 | masses3_N4 | combinatorial_graph | 3642 | 7138 | 1960 | 38.2 | 0.0 | 3% | 17.4 | 1.90 |
| 6 | masses3_N4 | mplcp | 3642 | 20900 | 5739 | 64.8 | 0.0 | 5% | 15.3 | 7.47 |
| 6 | masses3_N4 | combinatorial | 3642 | 6718 | 1845 | 25.2 | 0.0 | 5% | 15.3 | 1.77 |
| 6 | pop_017 | geometric | 4139 | 15066 | 3640 | 52.0 | 11.6 | 5% | 14.2 | 2.79 |
| 6 | pop_017 | graph | 4143 | 5798 | 1400 | 6.5 | 0.0 | 1% | 14.3 | 1.39 |
| 6 | pop_017 | combinatorial_graph | 4143 | 11376 | 2746 | 11.5 | 0.0 | 1% | 14.4 | 2.73 |
| 6 | pop_017 | mplcp | 4142 | 10019 | 2419 | 17.5 | 0.0 | 2% | 14.2 | 3.18 |
| 6 | pop_017 | combinatorial | 4143 | 11955 | 2886 | 10.5 | 0.0 | 1% | 14.2 | 2.87 |
| 6 | rand_t6_x5_m20_s0 | geometric | 1774 | 743 | 419 | 33.6 | 7.9 | 33% | 15.0 | 0.17 |
| 6 | rand_t6_x5_m20_s0 | graph | 1774 | 264 | 149 | 3.8 | 0.0 | 3% | 15.0 | 0.13 |
| 6 | rand_t6_x5_m20_s0 | combinatorial_graph | 1774 | 348 | 196 | 7.0 | 0.0 | 10% | 15.0 | 0.16 |
| 6 | rand_t6_x5_m20_s0 | mplcp | 1774 | 360 | 203 | 7.8 | 0.0 | 7% | 15.0 | 0.15 |
| 6 | rand_t6_x5_m20_s0 | combinatorial | 1774 | 254 | 143 | 9.7 | 0.0 | 8% | 15.0 | 0.13 |
| 7 | pop_005 | geometric | 10289 | 4644 | 451 | 50.6 | 7.3 | 47% | 15.9 | 0.15 |
| 7 | pop_005 | graph | 10287 | 1869 | 182 | 42.0 | 0.0 | 71% | 15.9 | 0.14 |
| 7 | pop_005 | combinatorial_graph | 10289 | 3338 | 324 | 80.8 | 0.0 | 70% | 15.9 | 0.16 |
| 7 | pop_005 | mplcp | 10289 | 7982 | 776 | 86.2 | 0.0 | 53% | 15.9 | 0.29 |
| 7 | pop_005 | combinatorial | 10289 | 3367 | 327 | 97.4 | 0.0 | 67% | 15.9 | 0.17 |
| 7 | pop_021 | geometric | 679 | 96 | 142 | 41.1 | 12.5 | 75% | 20.1 | 0.05 |
| 7 | pop_021 | graph | 679 | 40 | 59 | 23.6 | 0.1 | 77% | 20.1 | 0.05 |
| 7 | pop_021 | combinatorial_graph | 679 | 38 | 56 | 23.8 | 0.0 | 82% | 20.1 | 0.05 |
| 7 | pop_021 | mplcp | 679 | 129 | 190 | 46.6 | 0.0 | 57% | 20.1 | 0.11 |
| 7 | pop_021 | combinatorial | 679 | 37 | 55 | 25.0 | 0.0 | 81% | 20.1 | 0.05 |
| 8 | masses4_N3 | graph | 2891 | 1410 | 488 | 58.3 | 0.0 | 90% | 30.2 | 0.45 |
| 8 | masses4_N3 | combinatorial_graph | 2890 | 1209 | 418 | 65.7 | 0.0 | 92% | 33.4 | 0.39 |
| 8 | masses4_N3 | mplcp | 2890 | 3187 | 1103 | 119.3 | 0.0 | 67% | 25.3 | 0.62 |
| 8 | masses4_N3 | combinatorial | 2890 | 671 | 232 | 48.9 | 0.0 | 92% | 25.2 | 0.22 |
| 8 | pop_027 | geometric | 1472 | 260 | 177 | 54.6 | 17.3 | 84% | 25.7 | 0.09 |
| 8 | pop_027 | graph | 1471 | 134 | 91 | 33.9 | 0.1 | 85% | 25.7 | 0.08 |
| 8 | pop_027 | combinatorial_graph | 1472 | 135 | 92 | 34.7 | 0.0 | 85% | 25.7 | 0.09 |
| 8 | pop_027 | mplcp | 1472 | 285 | 194 | 45.2 | 0.0 | 56% | 25.7 | 0.18 |
| 8 | pop_027 | combinatorial | 1472 | 133 | 90 | 35.2 | 0.0 | 87% | 25.7 | 0.09 |
| 8 | rand_t8_x5_m20_s0 | geometric | 4111 | 1172 | 285 | 61.0 | 9.8 | 71% | 21.8 | 0.06 |
| 8 | rand_t8_x5_m20_s0 | graph | 4111 | 278 | 68 | 24.3 | 0.0 | 85% | 21.8 | 0.06 |
| 8 | rand_t8_x5_m20_s0 | combinatorial_graph | 4111 | 284 | 69 | 25.2 | 0.0 | 85% | 21.8 | 0.06 |
| 8 | rand_t8_x5_m20_s0 | mplcp | 4111 | 533 | 130 | 33.3 | 0.0 | 64% | 21.8 | 0.13 |
| 8 | rand_t8_x5_m20_s0 | combinatorial | 4111 | 279 | 68 | 26.2 | 0.0 | 87% | 21.8 | 0.06 |
| 10 | masses5_N2 | geometric | 369 | 2384 | 6461 | 391.5 | 45.3 | 84% | 42.5 | 0.31 |
| 10 | masses5_N2 | graph | 369 | 109 | 295 | 42.0 | 0.2 | 93% | 48.6 | 0.29 |
| 10 | masses5_N2 | combinatorial_graph | 369 | 125 | 339 | 46.7 | 0.0 | 94% | 55.0 | 0.33 |
| 10 | masses5_N2 | mplcp | 368 | 261 | 710 | 71.8 | 0.0 | 79% | 42.5 | 0.49 |
| 10 | masses5_N2 | combinatorial | 369 | 93 | 253 | 37.3 | 0.0 | 93% | 42.5 | 0.25 |
| 10 | rand_t10_x5_m15_s0 | geometric | 1859 | 475 | 256 | 46.9 | 7.5 | 81% | 28.9 | 0.10 |
| 10 | rand_t10_x5_m15_s0 | graph | 1859 | 182 | 98 | 24.2 | 0.0 | 89% | 28.9 | 0.09 |
| 10 | rand_t10_x5_m15_s0 | combinatorial_graph | 1859 | 179 | 96 | 25.0 | 0.0 | 90% | 28.9 | 0.09 |
| 10 | rand_t10_x5_m15_s0 | mplcp | 1859 | 314 | 169 | 32.2 | 0.0 | 74% | 28.9 | 0.19 |
| 10 | rand_t10_x5_m15_s0 | combinatorial | 1859 | 178 | 96 | 25.4 | 0.0 | 90% | 28.9 | 0.09 |
| 10 | rand_t10_x5_m20_s0 | geometric | 6124 | 3070 | 501 | 65.0 | 11.1 | 72% | 29.9 | 0.11 |
| 10 | rand_t10_x5_m20_s0 | graph | 6124 | 1208 | 197 | 26.0 | 0.0 | 87% | 29.9 | 0.19 |
| 10 | rand_t10_x5_m20_s0 | combinatorial_graph | 6124 | 1377 | 225 | 26.7 | 0.0 | 90% | 29.9 | 0.21 |
| 10 | rand_t10_x5_m20_s0 | mplcp | 6124 | 2560 | 418 | 38.4 | 0.0 | 71% | 29.9 | 0.41 |
| 10 | rand_t10_x5_m20_s0 | combinatorial | 6124 | 1254 | 205 | 27.2 | 0.0 | 91% | 29.9 | 0.20 |

Table 2. LP and QP calls per region by purpose, with the share of solve time in brackets. "redundancy" holds the
per-row tests of the LP redundancy path, "bbox" the 2n LPs of the region bounding box, "neighbour" the facet centres,
facet steps and mplcp's Chebyshev balls in `pivot_facet`.

| params | problem | algorithm | redundancy | feasibility | neighbour | chebyshev | bbox | init | other |
|---:|---|---|---|---|---:|---:|---|---:|---|
| 3 | sys3x2_N7 | geometric | - | - | 2.7 (26%) | 2.0 (7%) | - | 0.0 (0%) | - |
| 3 | sys3x2_N7 | graph | - | 413.7 (13%) | - | 1.0 (0%) | - | 0.0 (0%) | - |
| 3 | sys3x2_N7 | combinatorial_graph | - | 2157.8 (10%) | - | 1.0 (0%) | 0.0 (0%) | 0.0 (0%) | - |
| 3 | sys3x2_N7 | mplcp | - | - | 0.0 (0%) | 1.0 (10%) | 0.0 (0%) | 0.0 (0%) | 0.0 (0%) |
| 4 | masses2_N8 | geometric | - | - | 6.6 (24%) | 2.0 (6%) | 0.0 (0%) | 0.0 (0%) | - |
| 4 | masses2_N8 | graph | - | 48.2 (17%) | - | 1.0 (2%) | - | 0.0 (0%) | - |
| 4 | masses2_N8 | combinatorial_graph | - | 339.2 (29%) | - | 1.0 (0%) | - | 0.0 (0%) | - |
| 4 | masses2_N8 | mplcp | - | - | 2.7 (4%) | 1.0 (6%) | - | 0.0 (0%) | 0.0 (0%) |
| 4 | masses2_N8 | combinatorial | - | 2588.6 (30%) | - | 58.1 (1%) | 0.0 (0%) | - | - |
| 6 | masses3_N3 | geometric | 1.0 (0%) | - | 70.4 (22%) | 4.5 (3%) | 0.7 (0%) | 0.0 (0%) | - |
| 6 | masses3_N3 | graph | 2.1 (0%) | 4.8 (0%) | - | 1.1 (0%) | 1.2 (0%) | 0.1 (0%) | - |
| 6 | masses3_N3 | combinatorial_graph | 1.7 (0%) | 11.6 (2%) | - | 1.3 (1%) | 1.1 (1%) | 0.0 (0%) | - |
| 6 | masses3_N3 | mplcp | 1.0 (0%) | - | 35.5 (9%) | 1.2 (1%) | 0.7 (0%) | 0.0 (0%) | 0.0 (0%) |
| 6 | masses3_N3 | combinatorial | 1.0 (0%) | 6.5 (1%) | - | 3.9 (3%) | 0.7 (1%) | - | - |
| 6 | masses3_N4 | graph | 5.0 (0%) | 7.8 (0%) | - | 1.1 (0%) | 1.7 (0%) | 0.0 (0%) | - |
| 6 | masses3_N4 | combinatorial_graph | 2.9 (0%) | 32.7 (2%) | - | 1.4 (1%) | 1.1 (0%) | 0.0 (0%) | - |
| 6 | masses3_N4 | mplcp | 1.6 (0%) | - | 61.3 (4%) | 1.3 (1%) | 0.6 (0%) | 0.0 (0%) | 0.0 (0%) |
| 6 | masses3_N4 | combinatorial | 1.7 (0%) | 15.8 (2%) | - | 7.1 (2%) | 0.6 (0%) | - | - |
| 6 | pop_017 | geometric | 0.6 (0%) | - | 60.2 (5%) | 2.0 (0%) | 0.7 (0%) | 0.0 (0%) | - |
| 6 | pop_017 | graph | 0.6 (0%) | 4.2 (0%) | - | 1.0 (0%) | 0.7 (0%) | 0.0 (0%) | - |
| 6 | pop_017 | combinatorial_graph | 0.6 (0%) | 9.1 (0%) | - | 1.0 (0%) | 0.7 (0%) | 0.0 (0%) | - |
| 6 | pop_017 | mplcp | 0.4 (0%) | - | 15.6 (1%) | 1.1 (0%) | 0.4 (0%) | 0.0 (0%) | 0.0 (0%) |
| 6 | pop_017 | combinatorial | 0.6 (0%) | 5.0 (0%) | - | 4.2 (0%) | 0.7 (0%) | - | - |
| 6 | rand_t6_x5_m20_s0 | geometric | 0.0 (0%) | - | 39.5 (31%) | 2.0 (1%) | 0.0 (0%) | 0.0 (0%) | - |
| 6 | rand_t6_x5_m20_s0 | graph | 0.0 (0%) | 2.8 (1%) | - | 1.0 (2%) | 0.0 (0%) | 0.0 (0%) | - |
| 6 | rand_t6_x5_m20_s0 | combinatorial_graph | 0.0 (0%) | 6.0 (8%) | - | 1.0 (2%) | 0.0 (0%) | 0.0 (0%) | - |
| 6 | rand_t6_x5_m20_s0 | mplcp | 0.0 (0%) | - | 6.8 (5%) | 1.0 (2%) | 0.0 (0%) | 0.0 (0%) | 0.0 (0%) |
| 6 | rand_t6_x5_m20_s0 | combinatorial | 0.0 (0%) | 6.6 (3%) | - | 3.0 (5%) | 0.0 (0%) | - | - |
| 7 | pop_005 | geometric | 20.0 (11%) | - | 21.9 (19%) | 2.0 (2%) | 14.0 (15%) | 0.0 (0%) | - |
| 7 | pop_005 | graph | 20.0 (28%) | 7.0 (7%) | - | 1.0 (4%) | 14.0 (32%) | 0.0 (0%) | - |
| 7 | pop_005 | combinatorial_graph | 20.0 (17%) | 45.7 (30%) | - | 1.0 (2%) | 14.0 (21%) | 0.0 (0%) | - |
| 7 | pop_005 | mplcp | 20.0 (7%) | - | 50.2 (38%) | 2.0 (1%) | 14.0 (7%) | 0.0 (0%) | 0.0 (0%) |
| 7 | pop_005 | combinatorial | 20.0 (13%) | 49.0 (26%) | - | 14.5 (13%) | 14.0 (16%) | - | - |
| 7 | pop_021 | geometric | 5.6 (6%) | - | 32.1 (44%) | 2.0 (3%) | 14.0 (22%) | 0.0 (0%) | - |
| 7 | pop_021 | graph | 5.6 (14%) | 3.1 (4%) | - | 1.0 (5%) | 14.0 (53%) | 0.1 (0%) | - |
| 7 | pop_021 | combinatorial_graph | 5.6 (15%) | 3.3 (5%) | - | 1.0 (6%) | 14.0 (56%) | 0.0 (0%) | - |
| 7 | pop_021 | mplcp | 5.6 (4%) | - | 25.0 (33%) | 2.0 (3%) | 14.0 (17%) | 0.0 (0%) | 0.0 (0%) |
| 7 | pop_021 | combinatorial | 5.6 (15%) | 3.1 (4%) | - | 2.3 (8%) | 14.0 (55%) | - | - |
| 8 | masses4_N3 | graph | 33.4 (33%) | 4.5 (3%) | - | 1.3 (10%) | 19.2 (44%) | 0.0 (0%) | - |
| 8 | masses4_N3 | combinatorial_graph | 33.1 (28%) | 9.0 (5%) | - | 2.0 (16%) | 21.6 (42%) | 0.0 (0%) | - |
| 8 | masses4_N3 | mplcp | 24.0 (9%) | - | 77.0 (39%) | 2.2 (6%) | 16.0 (13%) | 0.0 (0%) | 0.0 (0%) |
| 8 | masses4_N3 | combinatorial | 24.0 (28%) | 5.2 (2%) | - | 3.7 (19%) | 16.0 (43%) | - | - |
| 8 | pop_027 | geometric | 14.7 (14%) | - | 39.3 (41%) | 2.0 (3%) | 16.0 (26%) | 0.0 (0%) | - |
| 8 | pop_027 | graph | 14.7 (27%) | 2.2 (2%) | - | 1.0 (5%) | 16.0 (51%) | 0.1 (0%) | - |
| 8 | pop_027 | combinatorial_graph | 14.7 (27%) | 3.0 (3%) | - | 1.0 (5%) | 16.0 (51%) | 0.0 (0%) | - |
| 8 | pop_027 | mplcp | 14.7 (13%) | - | 12.5 (15%) | 2.0 (4%) | 16.0 (24%) | 0.0 (0%) | 0.0 (0%) |
| 8 | pop_027 | combinatorial | 14.7 (27%) | 2.2 (2%) | - | 2.3 (7%) | 16.0 (51%) | - | - |
| 8 | rand_t8_x5_m20_s0 | geometric | 5.1 (3%) | - | 47.7 (51%) | 2.0 (2%) | 16.0 (15%) | 0.0 (0%) | - |
| 8 | rand_t8_x5_m20_s0 | graph | 5.1 (12%) | 2.2 (3%) | - | 1.0 (5%) | 16.0 (64%) | 0.0 (0%) | - |
| 8 | rand_t8_x5_m20_s0 | combinatorial_graph | 5.1 (12%) | 3.1 (4%) | - | 1.0 (5%) | 16.0 (63%) | 0.0 (0%) | - |
| 8 | rand_t8_x5_m20_s0 | mplcp | 5.1 (7%) | - | 10.2 (18%) | 2.0 (5%) | 16.0 (34%) | 0.0 (0%) | 0.0 (0%) |
| 8 | rand_t8_x5_m20_s0 | combinatorial | 5.1 (12%) | 2.8 (3%) | - | 2.2 (8%) | 16.0 (64%) | - | - |
| 10 | masses5_N2 | geometric | 12.3 (1%) | - | 395.7 (79%) | 8.7 (1%) | 20.0 (2%) | 0.0 (0%) | - |
| 10 | masses5_N2 | graph | 16.0 (21%) | 2.5 (1%) | - | 1.2 (9%) | 22.4 (62%) | 0.2 (0%) | - |
| 10 | masses5_N2 | combinatorial_graph | 16.2 (18%) | 2.9 (1%) | - | 1.8 (12%) | 25.8 (62%) | 0.0 (0%) | - |
| 10 | masses5_N2 | mplcp | 12.4 (6%) | - | 37.2 (44%) | 2.2 (5%) | 20.0 (23%) | 0.0 (0%) | 0.0 (0%) |
| 10 | masses5_N2 | combinatorial | 12.3 (18%) | 2.5 (1%) | - | 2.4 (11%) | 20.0 (64%) | - | - |
| 10 | rand_t10_x5_m15_s0 | geometric | 1.2 (1%) | - | 31.2 (46%) | 2.0 (3%) | 20.0 (32%) | 0.0 (0%) | - |
| 10 | rand_t10_x5_m15_s0 | graph | 1.2 (3%) | 2.0 (2%) | - | 1.0 (6%) | 20.0 (79%) | 0.0 (0%) | - |
| 10 | rand_t10_x5_m15_s0 | combinatorial_graph | 1.2 (3%) | 2.8 (3%) | - | 1.0 (6%) | 20.0 (78%) | 0.0 (0%) | - |
| 10 | rand_t10_x5_m15_s0 | mplcp | 1.2 (2%) | - | 9.0 (21%) | 2.0 (7%) | 20.0 (45%) | 0.0 (0%) | 0.0 (0%) |
| 10 | rand_t10_x5_m15_s0 | combinatorial | 1.2 (3%) | 2.2 (2%) | - | 2.0 (7%) | 20.0 (79%) | - | - |
| 10 | rand_t10_x5_m20_s0 | geometric | 3.0 (2%) | - | 51.1 (51%) | 2.0 (2%) | 20.0 (18%) | 0.0 (0%) | - |
| 10 | rand_t10_x5_m20_s0 | graph | 3.0 (8%) | 2.0 (1%) | - | 1.0 (8%) | 20.0 (71%) | 0.0 (0%) | - |
| 10 | rand_t10_x5_m20_s0 | combinatorial_graph | 3.0 (8%) | 2.7 (3%) | - | 1.0 (4%) | 20.0 (75%) | 0.0 (0%) | - |
| 10 | rand_t10_x5_m20_s0 | mplcp | 3.0 (3%) | - | 13.4 (25%) | 2.0 (5%) | 20.0 (37%) | 0.0 (0%) | 0.0 (0%) |
| 10 | rand_t10_x5_m20_s0 | combinatorial | 3.0 (5%) | 2.2 (2%) | - | 2.0 (5%) | 20.0 (79%) | - | - |

Table 3. mplcp neighbour search. "no zero row" = `adjacent` returns at once (boundary facet); "Case 1 fast" = the
single-candidate toggle; "Case 2" = `pivot_facet`. Time split of the solve: neighbour search (`expand_candidates`),
region build (`build_cand`), tableau update, rest.

| params | problem | regions | adjacent/region | no zero row | Case 1 fast | Case 2 | us/call none | us/call fast | us/call Case 2 | Case 2 share of adjacent time | neighbour search | region build | tableau update | rest |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 3 | sys3x2_N7 | 2095 | 6.1 | 7% | 91% | 3% | 0.2 | 0.3 | 3.1 | 22% | 24% | 58% | 13% | 5% |
| 4 | masses2_N8 | 2837 | 8.5 | 4% | 70% | 26% | 0.4 | 0.5 | 7.7 | 85% | 51% | 37% | 10% | 2% |
| 6 | masses3_N3 | 955 | 16.2 | 20% | 25% | 55% | 0.3 | 0.4 | 18.8 | 99% | 22% | 77% | 0% | 0% |
| 6 | masses3_N4 | 3642 | 15.3 | 15% | 30% | 54% | 0.4 | 0.5 | 77.1 | 99% | 11% | 89% | 0% | 0% |
| 6 | pop_017 | 4142 | 14.2 | 6% | 41% | 53% | 0.2 | 0.7 | 12.5 | 96% | 4% | 96% | 0% | 0% |
| 6 | rand_t6_x5_m20_s0 | 1774 | 15.0 | 15% | 40% | 45% | 0.2 | 0.3 | 5.3 | 94% | 20% | 70% | 0% | 9% |
| 7 | pop_005 | 10289 | 15.9 | 24% | 42% | 34% | 0.4 | 0.8 | 113.5 | 99% | 80% | 19% | 0% | 1% |
| 7 | pop_021 | 679 | 20.1 | 23% | 20% | 57% | 0.2 | 0.3 | 10.9 | 99% | 68% | 30% | 0% | 1% |
| 8 | masses4_N3 | 2890 | 21.2 | 23% | 18% | 59% | 0.2 | 0.4 | 61.0 | 100% | 69% | 30% | 0% | 0% |
| 8 | pop_027 | 1472 | 25.7 | 16% | 19% | 65% | 0.2 | 0.3 | 5.8 | 98% | 51% | 47% | 0% | 1% |
| 8 | rand_t8_x5_m20_s0 | 4111 | 21.8 | 24% | 36% | 41% | 0.1 | 0.2 | 6.2 | 96% | 45% | 53% | 0% | 2% |
| 10 | masses5_N2 | 368 | 32.6 | 46% | 8% | 45% | 0.2 | 0.3 | 29.9 | 99% | 63% | 37% | 0% | 0% |
| 10 | rand_t10_x5_m15_s0 | 1859 | 28.9 | 48% | 28% | 24% | 0.1 | 0.2 | 8.9 | 96% | 39% | 60% | 0% | 1% |
| 10 | rand_t10_x5_m20_s0 | 6124 | 29.9 | 36% | 28% | 36% | 0.3 | 0.2 | 17.2 | 97% | 46% | 51% | 1% | 1% |

Table 3b. Candidates returned by `adjacent` that end at an already known region (the drop in `pivot_solve`).
Fast = `Nbr::Toggle`, slow = `Nbr::Z`. "all seen" = adjacent calls (with at least one candidate) whose candidates were
all already seen. The last column counts rows of the region builder's clipping that share a hyperplane with a box
face or an earlier row (the shared-bit duplicate detection); it is 0.0 at 7 or more parameters because clipping is not
compiled there, and 0.3 to 3.1 per region at 3 to 6. It marks duplicate rows, not degenerate facets, and is far
below the 30 to 99% of calls that end at a known region, so it is not a useful certificate by itself.

| params | problem | candidates/region | fast returned | fast already seen | slow returned | slow already seen | adjacent calls with candidates | all seen (fast) | all seen (slow) | clip rows sharing a hyperplane/region |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 3 | sys3x2_N7 | 5.6 | 11596 | 82% | 52 | 85% | 5.6/region | 82% | 84% | 0.4 |
| 4 | masses2_N8 | 7.4 | 16812 | 87% | 4072 | 84% | 7.1/region | 87% | 83% | 0.3 |
| 6 | masses3_N3 | 12.8 | 3881 | 90% | 8349 | 91% | 10.0/region | 90% | 89% | 3.1 |
| 6 | masses3_N4 | 14.2 | 16879 | 93% | 34768 | 90% | 10.6/region | 93% | 87% | 2.1 |
| 6 | pop_017 | 16.6 | 24281 | 88% | 44455 | 97% | 12.3/region | 88% | 96% | 1.6 |
| 6 | rand_t6_x5_m20_s0 | 12.4 | 10530 | 84% | 11514 | 99% | 11.6/region | 84% | 99% | 2.2 |
| 7 | pop_005 | 11.8 | 69238 | 88% | 52083 | 92% | 9.9/region | 88% | 92% | 0.0 |
| 7 | pop_021 | 20.5 | 2692 | 95% | 11202 | 95% | 14.5/region | 95% | 93% | 0.0 |
| 8 | masses4_N3 | 20.4 | 10953 | 92% | 47867 | 92% | 13.1/region | 92% | 91% | 0.0 |
| 8 | pop_027 | 17.1 | 7266 | 86% | 17900 | 97% | 16.2/region | 86% | 97% | 0.0 |
| 8 | rand_t8_x5_m20_s0 | 17.7 | 31832 | 90% | 40792 | 98% | 15.3/region | 90% | 97% | 0.0 |
| 10 | masses5_N2 | 17.0 | 1002 | 82% | 5259 | 93% | 12.6/region | 82% | 94% | 0.0 |
| 10 | rand_t10_x5_m15_s0 | 17.0 | 14842 | 96% | 16696 | 93% | 13.8/region | 96% | 92% | 0.0 |
| 10 | rand_t10_x5_m20_s0 | 21.6 | 50786 | 98% | 81326 | 94% | 17.6/region | 98% | 92% | 0.0 |

Table 3c. `pivot_facet` (the slow path). Depth columns are calls per recursion depth. "depth>6 returns" counts calls that
return at the `depth > 6` guard, "iteration-limit exits" the loops that run `8n + 50` iterations without returning.
"repeated basis" counts Case-2 branches whose next basis (set of `row_var`) was already reached earlier in the same
`adjacent` call.

| params | problem | calls/region | Case-1 iters | Case-2 iters | branches | cut by lex_nonneg | Chebyshev LPs | empty facet | repeated basis | depth 0/1/2/3/4/5/6/7+ | depth>6 returns | iteration-limit exits |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 3 | sys3x2_N7 | 0.2 | 324 | 50 | 102 | 0% | 102 | 49% | 0% | 324/52/0/0/0/0/0/0 | 0 | 0 |
| 4 | masses2_N8 | 3.7 | 6351 | 3431 | 7730 | 0% | 7729 | 47% | 0% | 6352/4072/1/0/0/0/0/0 | 0 | 0 |
| 6 | masses3_N3 | 19.9 | 7726 | 7801 | 34217 | 1% | 33882 | 68% | 0% | 8448/8392/1495/122/126/140/146/139 | 119 | 0 |
| 6 | masses3_N4 | 21.6 | 26030 | 33633 | 226769 | 2% | 223229 | 78% | 0% | 30339/35445/9292/883/603/638/682/736 | 655 | 0 |
| 6 | pop_017 | 18.5 | 30503 | 27727 | 64841 | 0% | 64520 | 29% | 0% | 31094/44219/1306/4/2/2/2/2 | 2 | 0 |
| 6 | rand_t6_x5_m20_s0 | 13.3 | 12060 | 9979 | 12026 | 0% | 12026 | 4% | 0% | 12060/11514/0/0/0/0/0/0 | 0 | 0 |
| 7 | pop_005 | 14.7 | 49281 | 74328 | 537115 | 4% | 516525 | 80% | 3% | 55236/48248/10438/2944/4677/6975/9854/12957 | 10486 | 0 |
| 7 | pop_021 | 29.5 | 7588 | 8048 | 16952 | 0% | 16952 | 28% | 0% | 7836/11023/1202/0/0/0/0/0 | 0 | 0 |
| 8 | masses4_N3 | 36.0 | 32362 | 43494 | 226547 | 2% | 222644 | 69% | 0% | 35882/48229/13093/1513/1159/1136/1523/1403 | 1194 | 0 |
| 8 | pop_027 | 28.8 | 24461 | 16517 | 18422 | 0% | 18422 | 3% | 0% | 24461/17900/0/0/0/0/0/0 | 0 | 0 |
| 8 | rand_t8_x5_m20_s0 | 18.8 | 36354 | 30879 | 41776 | 0% | 41776 | 2% | 0% | 36354/40792/0/0/0/0/0/0 | 0 | 0 |
| 10 | masses5_N2 | 35.0 | 5262 | 5195 | 13895 | 1% | 13702 | 45% | 1% | 5427/5781/862/79/122/134/252/217 | 202 | 0 |
| 10 | rand_t10_x5_m15_s0 | 15.9 | 12871 | 10870 | 16726 | 0% | 16726 | 0% | 0% | 12871/16696/0/0/0/0/0/0 | 0 | 0 |
| 10 | rand_t10_x5_m20_s0 | 24.0 | 65869 | 56847 | 82342 | 0% | 82342 | 1% | 0% | 65869/81326/0/0/0/0/0/0 | 0 | 0 |

Hard limits. The iteration limit is never reached (0 on all 14 problems). The `depth > 6` guard is reached on 6
problems, so on them neighbours across some facets are silently not produced (the final region counts of those
problems agree with graph to within the usual few regions, see `large.md`):
masses3_N3 (6 parameters) 119 times, masses3_N4 (6) 655, pop_017 (6) 2, pop_005 (7) 10486, masses4_N3 (8) 1194,
masses5_N2 (10) 202. The counters do not record which regions. The other problems (sys3x2_N7, masses2_N8,
rand_t6/t8/t10, pop_021, pop_027) never go deeper than depth 2. Memoization would save little: repeated bases are 0 to 3% of the
branches. Cutting branches without an LP (`lex_nonneg`) removes 0 to 4%. Empty-facet answers (Chebyshev LP finds no
interior) are 0 to 4% on the random family and pop_027, and 28 to 80% on the masses problems and pop_005/017/021.

Table 4. Vertex clipping outcomes per call to `clip_flags` (shipped caps), graph and mplcp.

| params | problem | algorithm | clip calls/region | ok | nt outside 2..=6 | more than 128 hyperplanes | infinite box | more than 600 vertices | too few vertices | vertex violates a row | redundancy LPs/region |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 3 | sys3x2_N7 | graph | 1.00 | 100% | 0% | 0% | 0% | 0% | 0% | 0% | 0.0 |
| 3 | sys3x2_N7 | mplcp | 1.00 | 100% | 0% | 0% | 0% | 0% | 0% | 0% | 0.0 |
| 4 | masses2_N8 | graph | 1.00 | 100% | 0% | 0% | 0% | 0% | 0% | 0% | 0.0 |
| 4 | masses2_N8 | mplcp | 1.00 | 100% | 0% | 0% | 0% | 0% | 0% | 0% | 0.0 |
| 6 | masses3_N3 | graph | 1.06 | 91% | 0% | 0% | 0% | 8% | 1% | 0% | 2.1 |
| 6 | masses3_N3 | mplcp | 1.00 | 94% | 0% | 0% | 0% | 5% | 0% | 0% | 1.0 |
| 6 | masses3_N4 | graph | 1.11 | 87% | 0% | 0% | 0% | 12% | 2% | 0% | 5.0 |
| 6 | masses3_N4 | mplcp | 1.00 | 95% | 0% | 0% | 0% | 5% | 0% | 0% | 1.6 |
| 6 | pop_017 | graph | 1.00 | 94% | 0% | 0% | 0% | 6% | 0% | 0% | 0.6 |
| 6 | pop_017 | mplcp | 1.00 | 96% | 0% | 0% | 0% | 4% | 0% | 0% | 0.4 |
| 6 | rand_t6_x5_m20_s0 | graph | 1.00 | 100% | 0% | 0% | 0% | 0% | 0% | 0% | 0.0 |
| 6 | rand_t6_x5_m20_s0 | mplcp | 1.00 | 100% | 0% | 0% | 0% | 0% | 0% | 0% | 0.0 |
| 7 | pop_005 | graph | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 20.0 |
| 7 | pop_005 | mplcp | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 20.0 |
| 7 | pop_021 | graph | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 5.6 |
| 7 | pop_021 | mplcp | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 5.6 |
| 8 | masses4_N3 | graph | 1.26 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 33.4 |
| 8 | masses4_N3 | mplcp | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 24.0 |
| 8 | pop_027 | graph | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 14.7 |
| 8 | pop_027 | mplcp | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 14.7 |
| 8 | rand_t8_x5_m20_s0 | graph | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 5.1 |
| 8 | rand_t8_x5_m20_s0 | mplcp | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 5.1 |
| 10 | masses5_N2 | graph | 1.15 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 16.0 |
| 10 | masses5_N2 | mplcp | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 12.4 |
| 10 | rand_t10_x5_m15_s0 | graph | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 1.2 |
| 10 | rand_t10_x5_m15_s0 | mplcp | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 1.2 |
| 10 | rand_t10_x5_m20_s0 | graph | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 3.0 |
| 10 | rand_t10_x5_m20_s0 | mplcp | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 3.0 |

At 6 parameters the clip already fails on 4 to 12% of the regions (more than 600 vertices) and falls back to LPs.
At 7 and more it never applies. The 128-hyperplane, infinite-box and too-few-vertices reasons never occur here, and a
vertex violating a row never occurs either.

Table 5. Candidates per region (graph, combinatorial_graph: active-set candidates tried; mplcp: new candidates after the
duplicate check) and neighbour LP/QP per region.

| params | problem | algorithm | regions | candidates tried/region | duplicates/region | neighbour LP+QP/region | feasibility LP/region |
|---:|---|---|---:|---:|---:|---:|---:|
| 3 | sys3x2_N7 | geometric | 2095 | 0.0 | 0.0 | 2.7 | 0.0 |
| 3 | sys3x2_N7 | graph | 2095 | 303.3 | 1.8 | 0.0 | 413.7 |
| 3 | sys3x2_N7 | combinatorial_graph | 2095 | 3610.6 | 10965.1 | 0.0 | 2157.8 |
| 3 | sys3x2_N7 | mplcp | 2095 | 1.0 | 4.6 | 0.0 | 0.0 |
| 4 | masses2_N8 | geometric | 2837 | 0.0 | 1.0 | 6.6 | 0.0 |
| 4 | masses2_N8 | graph | 2837 | 31.6 | 2.4 | 0.0 | 48.2 |
| 4 | masses2_N8 | combinatorial_graph | 2837 | 483.4 | 1002.1 | 0.0 | 339.2 |
| 4 | masses2_N8 | mplcp | 2837 | 1.0 | 6.4 | 2.7 | 0.0 |
| 6 | masses3_N3 | geometric | 955 | 0.0 | 6.0 | 70.4 | 0.0 |
| 6 | masses3_N3 | graph | 955 | 5.5 | 4.4 | 0.0 | 4.8 |
| 6 | masses3_N3 | combinatorial_graph | 955 | 33.8 | 115.1 | 0.0 | 11.6 |
| 6 | masses3_N3 | mplcp | 955 | 1.2 | 11.6 | 35.5 | 0.0 |
| 6 | masses3_N4 | graph | 3642 | 6.7 | 4.1 | 0.0 | 7.8 |
| 6 | masses3_N4 | combinatorial_graph | 3642 | 65.2 | 206.7 | 0.0 | 32.7 |
| 6 | masses3_N4 | mplcp | 3642 | 1.3 | 12.9 | 61.3 | 0.0 |
| 6 | pop_017 | geometric | 4139 | 0.0 | 6.3 | 60.2 | 0.0 |
| 6 | pop_017 | graph | 4143 | 3.9 | 6.4 | 0.0 | 4.2 |
| 6 | pop_017 | combinatorial_graph | 4143 | 15.6 | 67.5 | 0.0 | 9.1 |
| 6 | pop_017 | mplcp | 4142 | 1.0 | 15.6 | 15.6 | 0.0 |
| 6 | rand_t6_x5_m20_s0 | geometric | 1774 | 0.0 | 5.6 | 39.5 | 0.0 |
| 6 | rand_t6_x5_m20_s0 | graph | 1774 | 1.9 | 2.5 | 0.0 | 2.8 |
| 6 | rand_t6_x5_m20_s0 | combinatorial_graph | 1774 | 15.9 | 69.7 | 0.0 | 6.0 |
| 6 | rand_t6_x5_m20_s0 | mplcp | 1774 | 1.0 | 11.4 | 6.8 | 0.0 |
| 7 | pop_005 | geometric | 10289 | 0.0 | 3.0 | 21.9 | 0.0 |
| 7 | pop_005 | graph | 10287 | 4.4 | 3.1 | 0.0 | 7.0 |
| 7 | pop_005 | combinatorial_graph | 10289 | 64.1 | 192.6 | 0.0 | 45.7 |
| 7 | pop_005 | mplcp | 10289 | 1.2 | 10.6 | 50.2 | 0.0 |
| 7 | pop_021 | geometric | 679 | 0.0 | 10.0 | 32.1 | 0.0 |
| 7 | pop_021 | graph | 679 | 5.5 | 9.0 | 0.0 | 3.1 |
| 7 | pop_021 | combinatorial_graph | 679 | 7.3 | 33.8 | 0.0 | 3.3 |
| 7 | pop_021 | mplcp | 679 | 1.0 | 19.5 | 25.0 | 0.0 |
| 8 | masses4_N3 | graph | 2891 | 6.8 | 7.5 | 0.0 | 4.5 |
| 8 | masses4_N3 | combinatorial_graph | 2890 | 43.5 | 178.7 | 0.0 | 9.0 |
| 8 | masses4_N3 | mplcp | 2890 | 1.6 | 18.8 | 77.0 | 0.0 |
| 8 | pop_027 | geometric | 1472 | 0.0 | 10.9 | 39.3 | 0.0 |
| 8 | pop_027 | graph | 1471 | 3.3 | 3.2 | 0.0 | 2.2 |
| 8 | pop_027 | combinatorial_graph | 1472 | 17.7 | 78.1 | 0.0 | 3.0 |
| 8 | pop_027 | mplcp | 1472 | 1.0 | 16.1 | 12.5 | 0.0 |
| 8 | rand_t8_x5_m20_s0 | geometric | 4111 | 0.0 | 7.4 | 47.7 | 0.0 |
| 8 | rand_t8_x5_m20_s0 | graph | 4111 | 1.4 | 3.4 | 0.0 | 2.2 |
| 8 | rand_t8_x5_m20_s0 | combinatorial_graph | 4111 | 8.7 | 49.9 | 0.0 | 3.1 |
| 8 | rand_t8_x5_m20_s0 | mplcp | 4111 | 1.0 | 16.7 | 10.2 | 0.0 |
| 10 | masses5_N2 | geometric | 369 | 0.0 | 14.0 | 395.7 | 0.0 |
| 10 | masses5_N2 | graph | 369 | 8.1 | 9.9 | 0.0 | 2.5 |
| 10 | masses5_N2 | combinatorial_graph | 369 | 19.2 | 77.6 | 0.0 | 2.9 |
| 10 | masses5_N2 | mplcp | 368 | 1.5 | 15.5 | 37.2 | 0.0 |
| 10 | rand_t10_x5_m15_s0 | geometric | 1859 | 0.0 | 5.4 | 31.2 | 0.0 |
| 10 | rand_t10_x5_m15_s0 | graph | 1859 | 1.4 | 3.7 | 0.0 | 2.0 |
| 10 | rand_t10_x5_m15_s0 | combinatorial_graph | 1859 | 6.6 | 36.8 | 0.0 | 2.8 |
| 10 | rand_t10_x5_m15_s0 | mplcp | 1859 | 1.0 | 16.0 | 9.0 | 0.0 |
| 10 | rand_t10_x5_m20_s0 | geometric | 6124 | 0.0 | 8.6 | 51.1 | 0.0 |
| 10 | rand_t10_x5_m20_s0 | graph | 6124 | 1.4 | 3.8 | 0.0 | 2.0 |
| 10 | rand_t10_x5_m20_s0 | combinatorial_graph | 6124 | 8.5 | 51.7 | 0.0 | 2.7 |
| 10 | rand_t10_x5_m20_s0 | mplcp | 6124 | 1.0 | 20.6 | 13.4 | 0.0 |

## Part 2: lifting the clipping caps (experiment, not kept)

Change: `clip_n` for N = 7, 8, 9, 10 in `mpqp_utils.rs` and `mplcp_build.rs`, the hash key widened to
`[i64; 11]` (10 coefficients plus the rhs), the 128-hyperplane and 600-vertex guards kept.

Outcome of `clip_flags` with the caps lifted (graph and mplcp, 7 or more parameters):

| params | problem | algorithm | clip calls/region | ok | nt outside 2..=6 | more than 128 hyperplanes | infinite box | more than 600 vertices | too few vertices | vertex violates a row | redundancy LPs/region |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 7 | pop_005 | graph | 1.00 | 93% | 0% | 0% | 0% | 7% | 0% | 0% | 1.7 |
| 7 | pop_005 | mplcp | 1.00 | 93% | 0% | 0% | 0% | 7% | 0% | 0% | 1.6 |
| 7 | pop_021 | graph | 1.00 | 79% | 0% | 0% | 0% | 21% | 0% | 0% | 1.0 |
| 7 | pop_021 | mplcp | 1.00 | 79% | 0% | 0% | 0% | 21% | 0% | 0% | 1.0 |
| 8 | masses4_N3 | graph | 1.26 | 20% | 0% | 0% | 0% | 79% | 0% | 0% | 27.3 |
| 8 | masses4_N3 | mplcp | 1.00 | 26% | 0% | 0% | 0% | 74% | 0% | 0% | 17.7 |
| 8 | pop_027 | graph | 1.00 | 14% | 0% | 0% | 0% | 86% | 0% | 0% | 12.4 |
| 8 | pop_027 | mplcp | 1.00 | 14% | 0% | 0% | 0% | 86% | 0% | 0% | 12.3 |
| 8 | rand_t8_x5_m20_s0 | graph | 1.00 | 17% | 0% | 0% | 0% | 83% | 0% | 0% | 4.1 |
| 8 | rand_t8_x5_m20_s0 | mplcp | 1.00 | 16% | 0% | 0% | 0% | 84% | 0% | 0% | 4.1 |
| 10 | masses5_N2 | graph | 1.15 | 0% | 0% | 0% | 0% | 100% | 0% | 0% | 16.0 |
| 10 | masses5_N2 | mplcp | 1.00 | 0% | 0% | 0% | 0% | 100% | 0% | 0% | 12.4 |
| 10 | rand_t10_x5_m15_s0 | graph | 1.00 | 0% | 0% | 0% | 0% | 100% | 0% | 0% | 1.2 |
| 10 | rand_t10_x5_m15_s0 | mplcp | 1.00 | 0% | 0% | 0% | 0% | 100% | 0% | 0% | 1.2 |
| 10 | rand_t10_x5_m20_s0 | graph | 1.00 | 0% | 0% | 0% | 0% | 100% | 0% | 0% | 3.0 |
| 10 | rand_t10_x5_m20_s0 | mplcp | 1.00 | 0% | 0% | 0% | 0% | 100% | 0% | 0% | 3.0 |

The 600-vertex guard rejects 7% (pop_005) to 100% (all 10-parameter problems; the box alone has 1024 vertices) of the
calls, and the failed attempt costs time before the LP path runs anyway. Where the clip does succeed it is no faster
than the LPs it replaces (pop_005 at 7 parameters: 93% success, solve time 2.5 times longer for graph).

Regions. `region_hash` (active set, E and f rounded to 1e-9, sorted) before and after, 43 solver and problem pairs
(mplcp, graph, geometric, combinatorial_graph on 12 problems): the region counts and active sets are identical in all.
The facet rows are identical in 37 pairs, and differ in 6 (pop_005 and masses4_N3, three solvers each) by 1 to 4 rows
out of 73000 to 163000 (one pair has one row fewer). Presumably a row that touches the region only at a vertex is
decided differently by the clipping and by the LP (not investigated), so the regions would be the same polytopes with a
differing redundant row; this was not verified.

A/B, frozen baseline binary (`ppopt_bench`, caps at 6) against the lifted binary, alternating runs, two full rounds,
faster of the two per cell, `--no-pl --no-traj`. Load average (1 minute) was 16.6 at the start, 65 at the end of round
1 and 16 at the end of round 2, so absolute seconds are unreliable, but the slowdowns are far larger than any noise:

| params | problem | solver | baseline s | lifted s | lifted / baseline |
|---|---|---|---:|---:|---:|
| 7 | pop_005 | combinatorial_graph | 1.931 | 3.662 | 1.90 |
| 7 | pop_005 | geometric | 2.431 | 5.886 | 2.42 |
| 7 | pop_005 | graph | 1.359 | 3.451 | 2.54 |
| 7 | pop_005 | mplcp | 5.953 | 7.693 | 1.29 |
| 7 | pop_021 | combinatorial_graph | 0.034 | 0.464 | 13.73 |
| 7 | pop_021 | geometric | 0.086 | 0.898 | 10.45 |
| 7 | pop_021 | graph | 0.037 | 0.433 | 11.80 |
| 7 | pop_021 | mplcp | 0.105 | 0.471 | 4.46 |
| 8 | pop_027 | combinatorial_graph | 0.129 | 1.008 | 7.83 |
| 8 | pop_027 | geometric | 0.252 | 1.502 | 5.96 |
| 8 | pop_027 | graph | 0.127 | 1.016 | 8.02 |
| 8 | pop_027 | mplcp | 0.267 | 1.791 | 6.70 |
| 8 | rand_t8_x5_m20_s0 | combinatorial_graph | 0.267 | 2.460 | 9.21 |
| 8 | rand_t8_x5_m20_s0 | geometric | 1.120 | 3.272 | 2.92 |
| 8 | rand_t8_x5_m20_s0 | graph | 0.263 | 2.454 | 9.34 |
| 8 | rand_t8_x5_m20_s0 | mplcp | 0.553 | 4.656 | 8.42 |
| 10 | masses5_N2 | combinatorial_graph | 0.159 | 0.445 | 2.81 |
| 10 | masses5_N2 | geometric | 2.335 | 2.560 | 1.10 |
| 10 | masses5_N2 | graph | 0.107 | 0.383 | 3.57 |
| 10 | masses5_N2 | mplcp | 0.253 | 0.709 | 2.81 |
| 10 | rand_t10_x5_m15_s0 | combinatorial_graph | 0.176 | 2.458 | 13.94 |
| 10 | rand_t10_x5_m15_s0 | geometric | 0.441 | 2.566 | 5.82 |
| 10 | rand_t10_x5_m15_s0 | graph | 0.173 | 2.299 | 13.27 |
| 10 | rand_t10_x5_m15_s0 | mplcp | 0.316 | 4.650 | 14.71 |
| 10 | rand_t10_x5_m20_s0 | combinatorial_graph | 0.690 | 7.859 | 11.39 |
| 10 | rand_t10_x5_m20_s0 | geometric | 3.017 | 12.151 | 4.03 |
| 10 | rand_t10_x5_m20_s0 | graph | 0.705 | 7.977 | 11.32 |
| 10 | rand_t10_x5_m20_s0 | mplcp | 1.368 | 15.780 | 11.54 |

Decision: the cap lifting is slower for every solver on every problem at 7 or more parameters (1.1 to 14.7 times
slower), so it is not kept. The real obstacle is vertex enumeration growth (up to 2^N box vertices, and every inside/outside
vertex pair is an edge candidate at each cut), not the hard-coded array sizes. A different certificate at 7 to 10 parameters (for example a
cheap bound on the 2n bounding-box LPs, or warm-started box LPs) would be the way to reduce the shared constant.

## Machine load and reproduction

Census counts: exact. Census times: load 4 to 25. A/B: load 16 to 65 (see above). Scripts and logs are in
`/Users/markchen/.claude/jobs/187003ec/tmp/highdim/`. Reproduce with

    cargo run --release --features lp_census --example lp_census -- --problems pop_005,rand_t8_x5_m20_s0 --algs mplcp,graph,geometric --highdim

`--problems` takes names from `bench/problems_large` (or `bench/problems`), `--algs` a comma list, `--highdim` adds
the tables above. `examples/region_hash.rs` prints the region count and the region hash used for the identity check.

## Gates for the kept code (counters only, caps at 2 to 6 parameters)

`cargo test --release`: 133 passed, 0 failed, 3 ignored. `ppopt_bench bench/problems --no-pl --no-traj`: 204 solver runs,
all `same-as-first: true`. The counters compile to nothing without the feature (the default build was tested above).
