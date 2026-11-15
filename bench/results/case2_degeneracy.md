# How often is a region connected to the rest only across degenerate (Case-2) facets?

Counters only, one `lp_census --pair <problem> mplcp` solve per problem (`RPOPT_CONN=1 RPOPT_CERT=0 RPOPT_ONCE=1`), Negishi, one CPU, main 1feb98b (dedupe, box chain, cold retry, refined gap-band rule) plus the counters of this branch. The problems are the benchmark QPs of `bench/problems`, `bench/problems_highdim` and `bench/problems_large` (mpLPs go to `mplp_lex` and are not in the set) except pop_002, 024, 033, 036, 051 and 095, as asked: 144 problems with 1,223,581 regions. Per-problem rows: `case2_degeneracy.csv`.

## What is counted

* **Facet classes** (per region popped, from the counter changes of each `adjacent` call; the entry facet through which a region was entered counts as Case 1): *boundary* (no neighbour: the edge of Theta or of the feasible set), *Case 1* (nondegenerate Toggle), *single swap* (rank-1 Case 2 with |P-| = 1: one valid exchange, an ordinary basis exchange), *several candidates* (rank-1 with |P-| >= 2, a walk, or the across-the-facet fallback).
* **Flags**: a region is flagged when it has an interior facet (Case 1, single swap or several) and no Case 1 facet; *substantive* when it also has no single-swap facet, so that every interior facet has several candidates or needs a walk.
* **Connectivity**: a union-find over the regions, joined for every Case-1 crossing (variant 1) and for Case-1 plus single-swap crossings (variant 2), whether the neighbour was new or already known. The neighbour search was given no `Seen` map for this census, so that crossings to known regions are returned too (the regions found are the same). 'Outside' = regions not in the start region's component; 'Case-2 crossings joining two components' = the entries the search has to make across degenerate facets (single swaps and several-candidate facets for variant 1, several-candidate facets for variant 2), 'distinct pairs' = how many different pairs of components they join.
* The certificate is off (`RPOPT_CERT=0`), so every Case-2 facet is searched.

## Results

- **Regions with every interior facet in Case 2**: 1.09% of all regions (13,291 of 1,223,581); 0.21% when only the substantive kind (no Case 1 and no single swap) counts.
- **Facets overall**: 9.3% single swaps and 33.2% several-candidate facets of 22,117,833 facets.
- **Reachable by Case 1 alone**: 68 of 144 problems have every region in the start region's component; in the other 76 problems 18,697 regions (1.53% of all regions) are outside it and need a Case-2 crossing.
- **With single swaps joined as well**: 123 of 144 problems are connected; 21 are not, with 472 regions (0.039% of all regions) outside the start component, which need a substantive (several-candidate or walk) crossing.

The component-size columns of the table of problems answer the follow-up question: 14,738 components lie outside the start component (Case 1 only); 13,437 of them (91%) are single regions, and the 18,697 regions outside the start component are spread over them at 1.3 regions per component on average. So the regions that need a Case-2 crossing are mostly isolated regions, not whole groups linked by ordinary facets; the groups of several regions are the exception.


### By family

| group | problems | regions | boundary facets | Case-1 facets | single-swap facets | several-candidate facets | regions with every interior facet Case 2 | of which no Case 1 and no single swap | problems with regions outside the start component (Case 1 only) | regions outside it | components (Case 1 only) | problems still disconnected with single swaps | regions outside it |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses | 21 | 237,838 | 20.3% | 31.0% | 14.6% | 34.2% | 4.75% | 0.83% | 21 | 16,041 | 12,678 | 6 | 308 |
| other | 8 | 80 | 35.3% | 37.9% | 0.0% | 26.8% | 0.00% | 0.00% | 0 | 0 | 8 | 0 | 0 |
| pop | 94 | 778,065 | 9.9% | 48.3% | 7.6% | 34.1% | 0.25% | 0.07% | 47 | 2,603 | 2,127 | 15 | 164 |
| rand | 17 | 200,574 | 24.5% | 37.5% | 8.1% | 29.9% | 0.02% | 0.01% | 6 | 47 | 59 | 0 | 0 |
| sys | 4 | 7,024 | 6.8% | 90.3% | 0.3% | 2.6% | 0.09% | 0.00% | 2 | 6 | 10 | 0 | 0 |

### By number of parameters

| group | problems | regions | boundary facets | Case-1 facets | single-swap facets | several-candidate facets | regions with every interior facet Case 2 | of which no Case 1 and no single swap | problems with regions outside the start component (Case 1 only) | regions outside it | components (Case 1 only) | problems still disconnected with single swaps | regions outside it |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 15 | 122 | 7.8% | 86.1% | 0.0% | 6.1% | 0.82% | 0.82% | 2 | 4 | 17 | 2 | 4 |
| 2 | 19 | 478 | 13.1% | 72.5% | 2.3% | 12.0% | 0.63% | 0.21% | 2 | 17 | 24 | 0 | 0 |
| 3 | 17 | 14,709 | 5.5% | 84.5% | 1.7% | 8.3% | 0.79% | 0.33% | 8 | 140 | 139 | 4 | 50 |
| 4 | 13 | 39,786 | 5.0% | 76.3% | 5.3% | 13.4% | 0.56% | 0.13% | 11 | 490 | 294 | 3 | 14 |
| 5 | 11 | 136,303 | 5.9% | 70.4% | 2.7% | 21.0% | 0.43% | 0.17% | 7 | 702 | 610 | 3 | 74 |
| 6 | 18 | 142,933 | 11.3% | 43.9% | 12.1% | 32.8% | 2.99% | 0.55% | 14 | 7,195 | 5,093 | 3 | 225 |
| 7 | 15 | 201,056 | 11.9% | 48.7% | 5.6% | 33.8% | 0.21% | 0.05% | 7 | 502 | 443 | 3 | 14 |
| 8 | 21 | 516,271 | 12.0% | 39.7% | 10.5% | 37.8% | 1.14% | 0.21% | 17 | 7,712 | 6,433 | 3 | 91 |
| 10 | 8 | 159,247 | 25.4% | 32.9% | 10.3% | 31.4% | 1.11% | 0.13% | 5 | 1,929 | 1,816 | 0 | 0 |
| 12 | 3 | 7,538 | 51.2% | 20.4% | 14.9% | 13.5% | 0.05% | 0.00% | 1 | 2 | 5 | 0 | 0 |
| 14 | 2 | 2,863 | 64.5% | 16.9% | 8.5% | 10.1% | 0.14% | 0.00% | 1 | 2 | 4 | 0 | 0 |
| 16 | 2 | 2,275 | 68.3% | 14.0% | 10.1% | 7.5% | 0.18% | 0.00% | 1 | 2 | 4 | 0 | 0 |

### Problems where regions are reachable only across degenerate facets

| problem | parameters | regions | Case-1 components | start component | outside | median size | max | singletons | Case-2 crossings joining two components | distinct pairs | with single swaps: components | outside | crossings (several-candidate only) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | 8 | 68247 | 4518 | 62870 | 5377 | 1 | 62870 | 4199 | 65794 | 19749 | 52 | 75 | 718 |
| masses3_N7 | 6 | 38344 | 2642 | 34415 | 3929 | 1 | 34415 | 2245 | 30908 | 9337 | 12 | 164 | 344 |
| masses3_N6 | 6 | 22152 | 1315 | 20246 | 1906 | 1 | 20246 | 1073 | 16674 | 4487 | 10 | 57 | 140 |
| masses5_N4 | 10 | 44236 | 1694 | 42428 | 1808 | 1 | 42428 | 1643 | 34710 | 8893 | 1 | 0 | 0 |
| masses4_N4 | 8 | 16342 | 1053 | 15160 | 1182 | 1 | 15160 | 1004 | 15261 | 4618 | 5 | 4 | 16 |
| masses3_N5 | 6 | 10444 | 647 | 9607 | 837 | 1 | 9607 | 571 | 7821 | 2197 | 1 | 0 | 0 |
| pop_032 | 8 | 178950 | 427 | 178271 | 679 | 1 | 178271 | 396 | 10949 | 1023 | 1 | 0 | 0 |
| masses3_N4 | 6 | 3643 | 289 | 3315 | 328 | 1 | 3315 | 266 | 3166 | 1035 | 1 | 0 | 0 |
| pop_088 | 5 | 62318 | 271 | 62038 | 280 | 1 | 62038 | 268 | 3436 | 585 | 19 | 39 | 324 |
| pop_097 | 7 | 20574 | 189 | 20328 | 246 | 1 | 20328 | 180 | 4913 | 424 | 2 | 1 | 28 |
| pop_062 | 5 | 31727 | 128 | 31526 | 201 | 1 | 31526 | 116 | 2103 | 382 | 18 | 30 | 282 |
| masses4_N3 | 8 | 2953 | 157 | 2785 | 168 | 1 | 2785 | 150 | 2489 | 572 | 1 | 0 | 0 |
| pop_047 | 8 | 48144 | 119 | 48008 | 136 | 1 | 48008 | 114 | 1388 | 223 | 10 | 12 | 92 |
| pop_083 | 5 | 13266 | 128 | 13132 | 134 | 1 | 13132 | 126 | 1833 | 280 | 4 | 5 | 63 |
| masses2_N12 | 4 | 12779 | 45 | 12659 | 120 | 1 | 12659 | 30 | 784 | 78 | 3 | 4 | 12 |
| pop_091 | 7 | 145022 | 115 | 144905 | 117 | 1 | 144905 | 113 | 2559 | 249 | 11 | 12 | 212 |
| masses5_N3 | 10 | 5273 | 111 | 5157 | 116 | 1 | 5157 | 108 | 2949 | 368 | 1 | 0 | 0 |
| pop_007 | 8 | 12235 | 86 | 12128 | 107 | 1 | 12128 | 83 | 2161 | 192 | 1 | 0 | 0 |
| pop_019 | 4 | 6857 | 36 | 6766 | 91 | 1 | 6766 | 19 | 674 | 73 | 1 | 0 | 0 |
| masses2_N10 | 4 | 6381 | 39 | 6291 | 90 | 2 | 6291 | 18 | 548 | 60 | 3 | 4 | 12 |
| pop_006 | 3 | 2375 | 83 | 2285 | 90 | 1 | 2285 | 80 | 508 | 185 | 15 | 36 | 136 |
| pop_086 | 7 | 3625 | 79 | 3539 | 86 | 1 | 3539 | 77 | 1842 | 202 | 1 | 0 | 0 |
| masses3_N3 | 6 | 955 | 71 | 883 | 72 | 1 | 883 | 68 | 720 | 202 | 1 | 0 | 0 |
| pop_001 | 5 | 3529 | 37 | 3482 | 47 | 1 | 3482 | 33 | 354 | 93 | 1 | 0 | 0 |
| pop_048 | 4 | 46 | 46 | 1 | 45 | 1 | 1 | 46 | 296 | 148 | 1 | 0 | 0 |
| pop_023 | 4 | 4729 | 40 | 4686 | 43 | 1 | 4686 | 38 | 376 | 106 | 7 | 6 | 56 |
| masses2_N8 | 4 | 2837 | 27 | 2803 | 34 | 1 | 2803 | 22 | 220 | 42 | 1 | 0 | 0 |
| pop_017 | 6 | 4142 | 34 | 4109 | 33 | 1 | 4109 | 33 | 612 | 86 | 1 | 0 | 0 |
| masses2_N6 | 4 | 901 | 27 | 869 | 32 | 1 | 869 | 24 | 180 | 48 | 1 | 0 | 0 |
| pop_018 | 6 | 20800 | 27 | 20774 | 26 | 1 | 20774 | 26 | 432 | 39 | 1 | 0 | 0 |
| pop_040 | 5 | 2007 | 25 | 1983 | 24 | 1 | 1983 | 24 | 212 | 65 | 1 | 0 | 0 |
| pop_060 | 3 | 2627 | 20 | 2603 | 24 | 1 | 2603 | 18 | 170 | 36 | 2 | 6 | 12 |
| pop_025 | 6 | 5441 | 23 | 5419 | 22 | 1 | 5419 | 22 | 458 | 43 | 1 | 0 | 0 |
| pop_005 | 7 | 10289 | 20 | 10268 | 21 | 1 | 10268 | 17 | 149 | 27 | 1 | 0 | 0 |
| masses2_N4 | 4 | 307 | 15 | 287 | 20 | 1 | 287 | 12 | 112 | 24 | 1 | 0 | 0 |
| pop_037 | 7 | 18615 | 21 | 18595 | 20 | 1 | 18595 | 20 | 442 | 32 | 2 | 1 | 16 |
| pop_049 | 6 | 9341 | 16 | 9322 | 19 | 1 | 9322 | 13 | 252 | 26 | 1 | 0 | 0 |
| rand_t8_x5_m20_s1 | 8 | 8737 | 19 | 8719 | 18 | 1 | 8719 | 18 | 492 | 52 | 1 | 0 | 0 |
| rand_t8_x6_m24_s0 | 8 | 32476 | 13 | 32459 | 17 | 1 | 32459 | 11 | 568 | 30 | 1 | 0 | 0 |
| pop_087 | 2 | 148 | 5 | 135 | 13 | 2 | 135 | 2 | 22 | 5 | 1 | 0 | 0 |
| pop_090 | 8 | 438 | 12 | 427 | 11 | 1 | 427 | 11 | 322 | 26 | 1 | 0 | 0 |
| pop_041 | 3 | 523 | 11 | 513 | 10 | 1 | 513 | 10 | 50 | 15 | 4 | 3 | 10 |
| pop_012 | 5 | 1047 | 9 | 1038 | 9 | 1 | 1038 | 7 | 46 | 10 | 1 | 0 | 0 |
| pop_043 | 4 | 899 | 9 | 890 | 9 | 1 | 890 | 7 | 80 | 14 | 1 | 0 | 0 |
| masses3_N2 | 6 | 169 | 9 | 161 | 8 | 1 | 161 | 8 | 80 | 14 | 1 | 0 | 0 |
| rand_t6_x5_m20_s1 | 6 | 5063 | 9 | 5055 | 8 | 1 | 5055 | 8 | 124 | 9 | 1 | 0 | 0 |
| pop_096 | 5 | 258 | 8 | 251 | 7 | 1 | 251 | 7 | 48 | 16 | 1 | 0 | 0 |
| masses4_N2 | 8 | 327 | 7 | 321 | 6 | 1 | 321 | 6 | 108 | 12 | 1 | 0 | 0 |
| pop_065 | 7 | 1089 | 7 | 1083 | 6 | 1 | 1083 | 6 | 94 | 11 | 1 | 0 | 0 |
| pop_080 | 7 | 273 | 4 | 267 | 6 | 4 | 267 | 2 | 82 | 4 | 1 | 0 | 0 |
| pop_009 | 4 | 63 | 6 | 58 | 5 | 1 | 58 | 5 | 46 | 7 | 1 | 0 | 0 |
| pop_011 | 6 | 397 | 3 | 392 | 5 | 4 | 392 | 1 | 16 | 2 | 2 | 4 | 8 |
| pop_045 | 3 | 61 | 3 | 56 | 5 | 4 | 56 | 1 | 28 | 3 | 3 | 5 | 28 |
| sys3x2_N5 | 3 | 973 | 5 | 969 | 4 | 1 | 969 | 4 | 24 | 4 | 1 | 0 | 0 |
| pop_071 | 2 | 5 | 2 | 1 | 4 | 4 | 4 | 1 | 2 | 1 | 1 | 0 | 0 |
| pop_027 | 8 | 1472 | 4 | 1469 | 3 | 1 | 1469 | 3 | 58 | 3 | 1 | 0 | 0 |
| pop_072 | 1 | 9 | 2 | 6 | 3 | 6 | 6 | 0 | 2 | 1 | 2 | 3 | 2 |
| pop_078 | 3 | 706 | 3 | 703 | 3 | 2 | 703 | 1 | 14 | 2 | 1 | 0 | 0 |
| sys3x2_N3 | 3 | 211 | 3 | 209 | 2 | 1 | 209 | 2 | 8 | 2 | 1 | 0 | 0 |
| masses6_N2 | 12 | 383 | 3 | 381 | 2 | 1 | 381 | 2 | 56 | 2 | 1 | 0 | 0 |
| masses7_N2 | 14 | 393 | 3 | 391 | 2 | 1 | 391 | 2 | 56 | 2 | 1 | 0 | 0 |
| masses8_N2 | 16 | 399 | 3 | 397 | 2 | 1 | 397 | 2 | 56 | 2 | 1 | 0 | 0 |
| masses5_N2 | 10 | 373 | 3 | 371 | 2 | 1 | 371 | 2 | 48 | 2 | 1 | 0 | 0 |
| pop_013 | 3 | 391 | 2 | 389 | 2 | 389 | 389 | 0 | 16 | 1 | 1 | 0 | 0 |
| pop_028 | 8 | 82116 | 3 | 82114 | 2 | 1 | 82114 | 2 | 24 | 2 | 1 | 0 | 0 |
| pop_076 | 8 | 15008 | 3 | 15006 | 2 | 1 | 15006 | 2 | 30 | 2 | 1 | 0 | 0 |
| rand_t10_x5_m20_s1 | 10 | 14849 | 3 | 14847 | 2 | 1 | 14847 | 2 | 142 | 2 | 1 | 0 | 0 |
| pop_003 | 1 | 29 | 2 | 28 | 1 | 28 | 28 | 1 | 2 | 1 | 2 | 1 | 2 |
| pop_010 | 8 | 36 | 2 | 35 | 1 | 35 | 35 | 1 | 46 | 1 | 1 | 0 | 0 |
| pop_016 | 8 | 384 | 2 | 383 | 1 | 383 | 383 | 1 | 6 | 1 | 1 | 0 | 0 |
| pop_053 | 6 | 2441 | 2 | 2440 | 1 | 2440 | 2440 | 1 | 20 | 1 | 1 | 0 | 0 |
| pop_059 | 4 | 53 | 2 | 52 | 1 | 52 | 52 | 1 | 6 | 1 | 1 | 0 | 0 |
| pop_069 | 8 | 26 | 2 | 25 | 1 | 25 | 25 | 1 | 18 | 1 | 1 | 0 | 0 |
| pop_098 | 8 | 269 | 2 | 268 | 1 | 268 | 268 | 1 | 6 | 1 | 1 | 0 | 0 |
| rand_t10_x6_m24_s0 | 10 | 71221 | 2 | 71220 | 1 | 71220 | 71220 | 1 | 48 | 1 | 1 | 0 | 0 |
| rand_t6_x6_m24_s0 | 6 | 17134 | 2 | 17133 | 1 | 17133 | 17133 | 1 | 12 | 1 | 1 | 0 | 0 |

Problems that did not finish: none.

Problems with every region connected by Case 1 alone: 68 of 144.

## Notes

* The census mode changes the work, not the result: the neighbour search runs without the `Seen` map, so Case-2 searches that would have been skipped as known are done, and some rank-1 members that are known and empty are still tested. The region counts equal those of the time-profile solves on the problems the two have in common (checked on sys3x2_N5, rand_t8, rand_t12, pop_017, pop_005, masses3_N4, masses4_N3 and masses5_N3).
* 'Parameters' is the number of columns of the parameter-set matrix. The 1 parameter group is the pop problems with one parameter.
* Heavy problems: masses4_N5 and masses5_N4 were run with 2 CPUs and 2.5 GB; see the line on unfinished problems in the table section.
