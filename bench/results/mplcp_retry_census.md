# mplcp: census of four retried ideas (certificate re-time, interior-point acceptance, parent dual support, deferring Case-2 facets)

Base main 04e671e. Census code `src/census_retry.rs`, driver `examples/retry_census.rs`, scripts `bench/retry_census/`; off by default (feature `lp_census` and `RPOPT_RETRY=2,3` or `4`), no default changed. Negishi, AMD EPYC 7763, every task 1 CPU (pop_002 census: 2 CPUs, 4000M, 32 min, peak 2.9 GB). Item 1 and the base times use a release `mplcp_bench` built from a clean 04e671e tree; the censuses use a gate-profile build with `lp_census` of the census tree (667aa50). Counts are deterministic; one census solve per problem. The design was reviewed by audit-rejected before submission (it asked for the feasible-start, clipping, Toggle-start and recentring changes to the Build 4 probe, at least 5 alternations in item 1, and the 20 ms restriction in item 4; all applied).

## Verdicts

| item | rule | result | recommendation |
|---|---|---|---|
| 1. facet certificate | keep, narrow to 3 to 4 parameters, or drop | Turning it off is faster by a median 1.4% over 15 problems (A/A 1.001). It costs 21% on pop_019 (4 parameters) and 6% on pop_001 (5), 1% to 3% on the masses2 problems, and helps only on pop_006 (3.7%) | **Drop** (the default range 3 to 5 gains nothing in the median; a 3-parameter-only version would keep only pop_006) |
| 2. accept a region from an interior point | go if at least 80% pass | best affordable variant passes 47% to 76% of accepted regions at m > 1e-8 (weighted by Chebyshev ns 42% to 72%); fails on all 9 problems | **No-go** |
| 3. parent dual support | go if validity above 50% | validity 1.8% to 17.5% (8 of 9 problems with per-row LPs; sys3x2_N5 has none); net time negative on every problem | **No-go** |
| 4. defer Case-2 facets | build if 10 or more problems of 20 ms or more would save 4% or more | 8 of 31 problems of 20 ms or more reach 4% even by the gross upper bound | **No-go** (and the bound is gross) |

## Item 1: the certificate against `RPOPT_CERT=0`

One task per (problem, alternation); each task runs three separate processes (certificate off, default, default again) in a shuffled order on one node, `reps` solves each (3 to 101, chosen from a pilot to fill about 0.6 s), median per process. Ratio new/base is certificate off over default, so a value below 1 means the certificate costs more than it saves. 15 problems (`masses2_N9` and `masses2_N11` do not exist), 5 to 11 alternations (more for the short problems). Region counts are equal in every task.

| problem | parameters | alternations | base ms | new/base median | min | max | A/A median | A/A min | A/A max | regions equal |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| sys3x2_N3 | 3 | 11 | 3.033 | 0.998 | 0.985 | 1.017 | 1.001 | 0.993 | 1.023 | yes |
| sys3x2_N4 | 3 | 9 | 8.313 | 0.998 | 0.981 | 1.013 | 1.001 | 0.982 | 1.046 | yes |
| sys3x2_N5 | 3 | 9 | 17.866 | 0.989 | 0.969 | 0.999 | 0.997 | 0.984 | 1.008 | yes |
| sys3x2_N6 | 3 | 7 | 29.826 | 0.986 | 0.960 | 0.991 | 1.001 | 0.947 | 1.007 | yes |
| pop_006 | 3 | 5 | 46.911 | 1.037 | 1.020 | 1.053 | 0.979 | 0.963 | 0.986 | yes |
| sys3x2_N7 | 3 | 5 | 46.062 | 0.980 | 0.958 | 1.026 | 0.971 | 0.964 | 1.046 | yes |
| masses2_N4 | 4 | 9 | 11.473 | 0.970 | 0.962 | 0.974 | 1.003 | 0.984 | 1.006 | yes |
| masses2_N5 | 4 | 7 | 19.442 | 0.984 | 0.976 | 0.996 | 1.002 | 0.997 | 1.010 | yes |
| masses2_N6 | 4 | 7 | 37.825 | 0.993 | 0.976 | 1.016 | 1.012 | 0.962 | 1.022 | yes |
| masses2_N7 | 4 | 5 | 82.543 | 0.982 | 0.964 | 0.985 | 1.003 | 0.986 | 1.010 | yes |
| masses2_N8 | 4 | 5 | 146.358 | 0.986 | 0.959 | 1.088 | 1.013 | 0.922 | 1.138 | yes |
| pop_019 | 4 | 5 | 315.274 | 0.788 | 0.773 | 0.802 | 0.982 | 0.973 | 1.012 | yes |
| masses2_N10 | 4 | 5 | 322.994 | 0.969 | 0.880 | 1.088 | 1.000 | 0.896 | 1.008 | yes |
| masses2_N12 | 4 | 5 | 824.792 | 0.989 | 0.966 | 1.013 | 1.005 | 0.990 | 1.044 | yes |
| pop_001 | 5 | 5 | 258.524 | 0.939 | 0.909 | 0.965 | 1.019 | 0.998 | 1.036 | yes |

| parameters | problems | median new/base | median A/A |
|---:|---:|---:|---:|
| all | 15 | 0.986 | 1.001 |
| 3 | 6 | 0.994 | 0.999 |
| 4 | 8 | 0.983 | 1.003 |
| 5 | 1 | 0.939 | 1.019 |

### Item 2: interior point from the hint (accepted regions; margin m of the chord midpoint)

| problem | parameters | regions | stage calls | refused | accepted | hint infeasible | non-unit rows | A m>1e-8 (of entry children) | B m>1e-8 | C available | C infeasible start | C m>1e-8 | best of A,B,C m>1e-6 | m>1e-8 | after 3 recentring steps m>1e-6 | m>1e-8 | m>1e-8 weighted by stage ns | feasible hint: pass | infeasible hint: pass | steps 1 / 2 / 3 | probe ns/region | stage ns/region | refused with m>1e-8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|---:|
| masses3_N4 | 6 | 3643 | 4020 | 378 | 3642 | 24.0% | 0 | 24.4% | 68.2% | 1063 | 19 | 48.4% | 57.0% | 75.6% | 57.5% | 75.9% | 70.2% | 92.0% | 24.7% | 75.9% / 75.9% / 75.9% | 8726 | 14980 | 0 |
| masses4_N3 | 8 | 2953 | 3848 | 896 | 2952 | 34.7% | 0 | 12.2% | 57.6% | 689 | 6 | 28.7% | 36.7% | 62.1% | 37.0% | 62.4% | 42.0% | 88.8% | 12.7% | 62.4% / 62.4% / 62.4% | 7427 | 37759 | 0 |
| masses5_N3 | 10 | 5273 | 9177 | 3905 | 5272 | 18.4% | 0 | 14.5% | 68.3% | 799 | 32 | 47.4% | 37.4% | 73.4% | 37.6% | 73.7% | 62.0% | 85.6% | 21.1% | 73.7% / 73.7% / 73.7% | 10278 | 90195 | 0 |
| pop_002 | 7 | 321026 | 321878 | 853 | 321025 | 63.6% | 0 | 29.0% | 35.9% | 252969 | 747 | 28.2% | 42.0% | 46.8% | 42.4% | 47.1% | 47.9% | 98.2% | 17.7% | 47.0% / 47.0% / 47.1% | 10600 | 14630 | 0 |
| pop_005 | 7 | 10289 | 10327 | 34 | 10293 | 56.3% | 0 | 31.5% | 45.2% | 6642 | 204 | 36.3% | 53.1% | 53.8% | 54.0% | 54.6% | 54.0% | 99.9% | 19.4% | 54.5% / 54.6% / 54.6% | 9011 | 9453 | 0 |
| pop_017 | 6 | 4142 | 4148 | 7 | 4141 | 44.0% | 0 | 38.7% | 57.4% | 2493 | 2 | 41.4% | 62.8% | 65.6% | 63.1% | 65.8% | 65.8% | 99.4% | 23.1% | 65.8% / 65.8% / 65.8% | 5338 | 6145 | 0 |
| rand_t12_x5_m20_s0 | 12 | 6143 | 6142 | 0 | 6142 | 43.6% | 0 | 43.5% | 56.5% | 4366 | 15 | 43.8% | 63.8% | 65.5% | 64.8% | 66.1% | 61.7% | 98.6% | 24.1% | 65.8% / 65.9% / 66.1% | 8721 | 16730 | 0 |
| rand_t8_x5_m20_s0 | 8 | 4111 | 4110 | 0 | 4110 | 42.8% | 0 | 46.4% | 58.3% | 2817 | 0 | 50.9% | 67.0% | 67.0% | 67.3% | 67.3% | 64.8% | 100.0% | 23.6% | 67.3% / 67.3% / 67.3% | 5453 | 8274 | 0 |
| sys3x2_N5 | 3 | 973 | 972 | 0 | 972 | 29.6% | 0 | 69.3% | 72.9% | 0 | 0 | n/a | 71.8% | 72.9% | 71.8% | 72.9% | 72.4% | 99.9% | 9.0% | 72.9% / 72.9% / 72.9% | 4215 | 2939 | 0 |

### Item 3: parent dual support (rows that reach the per-row LP)

| problem | parameters | regions | LP rows/region | LP us/row | tries (counterpart dropped by parent LP) | valid (support system) | valid share | missing row | of which toggled constraint | no span | negative | bound | singular | us/try | LP ns saved / LP ns of tries | (saved - try ns) / LP ns of all rows | valid share (tight-row LP) | us/try (tight-row LP) | support size mean | valid but LP keeps |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses3_N4 | 6 | 3642 | 27.0 | 2.99 | 68603 | 8342 | 12.2% | 57.2% | 47.7% | 7.9% | 21.3% | 0.4% | 1.1% | 0.69 | 11.3% | -8.2% | 16.7% | 2.44 | 5.89 | 0 |
| masses4_N3 | 8 | 2952 | 24.2 | 5.38 | 38430 | 2404 | 6.3% | 48.3% | 42.0% | 14.3% | 30.0% | 0.4% | 0.8% | 0.98 | 5.4% | -6.9% | 8.9% | 3.77 | 8.03 | 0 |
| masses5_N3 | 10 | 2031 | 28.8 | 10.55 | 24743 | 1350 | 5.5% | 59.3% | 56.1% | 13.0% | 21.3% | 0.2% | 0.7% | 1.18 | 4.0% | -3.0% | 8.0% | 5.52 | 10.09 | 0 |
| pop_002 | 7 | 321025 | 22.5 | 3.55 | 5600880 | 519366 | 9.3% | 46.8% | 46.8% | 4.2% | 38.6% | 1.0% | 0.1% | 1.31 | 8.8% | -21.6% | 9.5% | 3.10 | 7.02 | 0 |
| pop_005 | 7 | 10293 | 20.0 | 2.86 | 133840 | 23415 | 17.5% | 40.7% | 40.7% | 1.2% | 39.2% | 1.3% | 0.1% | 1.03 | 17.0% | -12.3% | 17.7% | 3.22 | 7.00 | 0 |
| pop_017 | 6 | 4141 | 10.2 | 2.04 | 22984 | 1954 | 8.5% | 38.9% | 38.9% | 8.1% | 42.0% | 2.1% | 0.4% | 0.83 | 8.0% | -17.6% | 10.3% | 2.63 | 5.83 | 0 |
| rand_t12_x5_m20_s0 | 12 | 6142 | 2.1 | 5.88 | 279 | 5 | 1.8% | 54.8% | 54.8% | 15.8% | 26.9% | 0.0% | 0.7% | 2.09 | 1.5% | -0.7% | 3.6% | 9.52 | 12.02 | 0 |
| rand_t8_x5_m20_s0 | 8 | 4110 | 5.1 | 3.37 | 7978 | 495 | 6.2% | 63.9% | 63.9% | 0.1% | 29.1% | 0.7% | 0.0% | 0.92 | 5.9% | -7.9% | 6.1% | 3.87 | 8.00 | 0 |
| sys3x2_N5 | 3 | 972 | 0.0 | 0.00 | 0 | 0 | n/a | n/a | n/a | n/a | n/a | n/a | n/a | 0.00 | n/a | n/a | n/a | 0.00 | 0.00 | 0 |

### Item 4: sub-facet tests of rank-1 calls (found = the call emitted at least one candidate; its time is the gross deferral saving)

| problem | base ms | census-build ms | calls | found | found ms | % of base | % of census build | no candidate | ms | % of base | back to walk | ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| di_N10 | 0.2 | 1.9 | 0 | 0 | 0.00 | 0.00% | 0.00% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| di_N12 | 0.2 | 0.4 | 0 | 0 | 0.00 | 0.00% | 0.00% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| di_N15 | 0.2 | 0.5 | 0 | 0 | 0.00 | 0.00% | 0.00% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| di_N20 | 0.3 | 0.8 | 0 | 0 | 0.00 | 0.00% | 0.00% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| di_N25 | 0.4 | 0.8 | 0 | 0 | 0.00 | 0.00% | 0.00% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| di_N30 | 0.6 | 5.1 | 0 | 0 | 0.00 | 0.00% | 0.00% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| di_N4 | 0.1 | 0.7 | 0 | 0 | 0.00 | 0.00% | 0.00% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| di_N6 | 0.1 | 7.6 | 0 | 0 | 0.00 | 0.00% | 0.00% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| di_N8 | 0.1 | 0.5 | 0 | 0 | 0.00 | 0.00% | 0.00% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| di_doc_N3 | 0.1 | 1.6 | 0 | 0 | 0.00 | 0.00% | 0.00% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| dinv_N4 | 0.1 | 0.5 | 0 | 0 | 0.00 | 0.00% | 0.00% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| dinv_N6 | 0.1 | 6.4 | 0 | 0 | 0.00 | 0.00% | 0.00% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| dinv_N8 | 0.1 | 5.8 | 0 | 0 | 0.00 | 0.00% | 0.00% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| masses2_N4 | 11.4 | 14.8 | 170 | 128 | 0.68 | 5.90% | 4.57% | 42 | 0.18 | 1.62% | 0 | 0.00 |
| masses2_N5 | 19.4 | 24.1 | 310 | 182 | 1.04 | 5.38% | 4.33% | 128 | 0.70 | 3.61% | 0 | 0.00 |
| masses2_N6 | 37.2 | 45.0 | 522 | 290 | 1.84 | 4.94% | 4.08% | 232 | 1.37 | 3.67% | 0 | 0.00 |
| masses2_N7 | 71.4 | 83.7 | 672 | 400 | 2.56 | 3.58% | 3.05% | 272 | 1.84 | 2.58% | 0 | 0.00 |
| masses2_N8 | 130.4 | 147.9 | 962 | 600 | 4.08 | 3.13% | 2.76% | 362 | 2.48 | 1.90% | 0 | 0.00 |
| masses3_N3 | 147.4 | 166.4 | 1437 | 504 | 4.46 | 3.03% | 2.68% | 933 | 7.42 | 5.03% | 0 | 0.00 |
| masses3_N4 | 827.1 | 915.7 | 7558 | 2397 | 22.87 | 2.76% | 2.50% | 5161 | 46.54 | 5.63% | 0 | 0.00 |
| masses4_N3 | 1486.8 | 1545.6 | 6097 | 2299 | 34.55 | 2.32% | 2.24% | 3798 | 53.66 | 3.61% | 0 | 0.00 |
| masses4_N4 | 15034.2 | 15233.1 | 48861 | 15248 | 269.83 | 1.79% | 1.77% | 33613 | 491.16 | 3.27% | 0 | 0.00 |
| masses5_N2 | 183.5 | 191.4 | 197 | 121 | 2.73 | 1.49% | 1.42% | 76 | 1.06 | 0.58% | 0 | 0.00 |
| masses5_N3 | 7854.7 | 7774.7 | 13192 | 6775 | 192.73 | 2.45% | 2.48% | 6417 | 180.08 | 2.29% | 0 | 0.00 |
| masses6_N2 | 286.9 | 296.0 | 173 | 129 | 4.34 | 1.51% | 1.47% | 44 | 1.66 | 0.58% | 0 | 0.00 |
| masses7_N2 | 532.9 | 531.5 | 153 | 131 | 7.22 | 1.36% | 1.36% | 22 | 0.62 | 0.12% | 0 | 0.00 |
| masses8_N2 | 729.3 | 743.8 | 32 | 0 | 0.00 | 0.00% | 0.00% | 32 | 1.25 | 0.17% | 0 | 0.00 |
| pop_005 | 1924.9 | 2043.8 | 10479 | 1845 | 21.05 | 1.09% | 1.03% | 8634 | 78.80 | 4.09% | 0 | 0.00 |
| pop_017 | 419.2 | 483.9 | 7662 | 1174 | 10.44 | 2.49% | 2.16% | 6488 | 46.07 | 10.99% | 0 | 0.00 |
| pop_021 | 90.5 | 105.0 | 2472 | 510 | 5.82 | 6.44% | 5.54% | 1962 | 17.98 | 19.87% | 0 | 0.00 |
| pop_027 | 298.7 | 330.6 | 766 | 461 | 4.63 | 1.55% | 1.40% | 305 | 2.73 | 0.91% | 0 | 0.00 |
| rand_t10_x5_m15_s0 | 301.1 | 321.8 | 923 | 919 | 18.46 | 6.13% | 5.74% | 4 | 0.06 | 0.02% | 0 | 0.00 |
| rand_t10_x5_m20_s0 | 1299.1 | 1398.6 | 5162 | 4817 | 78.38 | 6.03% | 5.60% | 345 | 5.38 | 0.41% | 0 | 0.00 |
| rand_t12_x5_m15_s0 | 208.7 | 227.7 | 244 | 244 | 5.10 | 2.45% | 2.24% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| rand_t12_x5_m20_s0 | 1805.6 | 1920.8 | 1429 | 1233 | 31.25 | 1.73% | 1.63% | 196 | 4.65 | 0.26% | 0 | 0.00 |
| rand_t14_x5_m15_s0 | 669.2 | 705.0 | 791 | 791 | 24.64 | 3.68% | 3.50% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| rand_t16_x5_m15_s0 | 643.6 | 674.4 | 900 | 900 | 40.23 | 6.25% | 5.96% | 0 | 0.00 | 0.00% | 0 | 0.00 |
| rand_t3_x5_m20_s0 | 2.1 | 3.5 | 45 | 35 | 0.12 | 6.03% | 3.53% | 10 | 0.03 | 1.33% | 0 | 0.00 |
| rand_t4_x5_m20_s0 | 27.6 | 34.1 | 470 | 315 | 1.96 | 7.10% | 5.74% | 155 | 0.64 | 2.31% | 0 | 0.00 |
| rand_t4_x5_m20_s1 | 15.3 | 21.8 | 659 | 274 | 2.09 | 13.65% | 9.60% | 385 | 1.91 | 12.46% | 0 | 0.00 |
| rand_t5_x5_m20_s0 | 153.5 | 164.0 | 1233 | 1023 | 5.76 | 3.76% | 3.51% | 210 | 1.27 | 0.83% | 0 | 0.00 |
| rand_t5_x5_m20_s1 | 80.7 | 100.3 | 2901 | 890 | 8.58 | 10.63% | 8.56% | 2011 | 12.87 | 15.94% | 0 | 0.00 |
| rand_t5_x5_m20_s2 | 120.1 | 138.4 | 2059 | 474 | 4.12 | 3.43% | 2.98% | 1585 | 8.13 | 6.77% | 0 | 0.00 |
| rand_t5_x6_m24_s0 | 498.4 | 512.3 | 10547 | 2757 | 21.65 | 4.34% | 4.23% | 7790 | 48.29 | 9.69% | 0 | 0.00 |
| rand_t6_x5_m20_s0 | 155.5 | 164.5 | 309 | 78 | 0.77 | 0.49% | 0.47% | 231 | 1.34 | 0.86% | 0 | 0.00 |
| rand_t8_x5_m20_s0 | 679.8 | 605.0 | 1233 | 838 | 11.37 | 1.67% | 1.88% | 395 | 4.06 | 0.60% | 0 | 0.00 |
| sys3x2_N3 | 3.3 | 5.2 | 22 | 15 | 0.09 | 2.80% | 1.80% | 7 | 0.03 | 1.03% | 0 | 0.00 |
| sys3x2_N4 | 8.2 | 12.3 | 52 | 35 | 0.26 | 3.18% | 2.13% | 17 | 0.10 | 1.22% | 0 | 0.00 |
| sys3x2_N5 | 17.8 | 24.6 | 64 | 37 | 0.32 | 1.78% | 1.29% | 27 | 0.22 | 1.24% | 0 | 0.00 |
| sys3x2_N6 | 29.7 | 40.0 | 44 | 28 | 0.23 | 0.77% | 0.57% | 16 | 0.18 | 0.62% | 0 | 0.00 |
| sys3x2_N7 | 43.7 | 56.5 | 11 | 8 | 0.05 | 0.12% | 0.09% | 3 | 0.02 | 0.05% | 0 | 0.00 |

Problems of 20 ms or more with found-call time of 4% or more of the base solve: 8 of 31 (rule: 10 or more)

## Reading the tables

**Item 1.** By parameter count the medians are 0.994 (3 parameters, 6 problems), 0.983 (4 parameters, 8 problems) and 0.939 (5 parameters, 1 problem); the A/A medians are 0.999 to 1.019, and single-problem A/A ranges reach 0.92 to 1.14 on masses2_N8 and masses2_N10, so only pop_019 (0.788, A/A 0.982) and pop_001 (0.939, A/A 1.019), and the masses2_N4 value 0.970, stand clear of the noise. The certificate pays only on pop_006 (1.037, A/A 0.979). Earlier phase-2 numbers (`mplcp_facet_certificate.md`) found the certificate neutral to helpful before the neighbour pre-check; with rank-1 all-known calls at about 1 us it now costs more than the Case-2 searches it removes. Decision: drop. If a narrower rule is wanted, 3 parameters is the only count with a positive result and it rests on one problem.

**Item 2.** The hint is the parent's Chebyshev centre projected onto the row's hyperplane, so it can lie outside the facet and outside the child. The table separates the cases: with a hint inside the child (feasible hint) the chord midpoint passes m > 1e-8 on 85.6% to 100% of regions (the probe costs 4 to 11 us per region against 3 to 90 us for the Chebyshev stage), with a hint outside it only 9% to 25%, and those are 18% to 64% of the accepted regions (column "hint infeasible"). Variant C (mean of the parent's box points on the entry facet, Toggle children with a parent box only) is available for 15% to 79% of the accepted regions (none on sys3x2_N5, which has no parent box) and passes 28% to 51% of those; it does not rescue the infeasible hints. Recentring (up to 3 steps) adds at most 0.6 points. No row of any problem has a norm differing from 1 by more than 1e-9, so the margin equals the ball radius proved. No refused region ever shows m > 1e-8 (`B_REJ_M8` is 0 everywhere), so the check is consistent with the Chebyshev stage. Passing 80% would need a feasible start for the infeasible hints, which is a search for a point in the child, that is, the work the Chebyshev LP does. Stage cost per region is 2.9 to 90 us, and the stage is 2.6% to 4.5% of the solve (Build study 1), so at 47% to 76% coverage the most it could save is under 3% of the solve.

**Item 3.** Validity (valid Farkas certificate on the child's versions of the parent's dual support, among LP-stage rows whose counterpart the parent's LP dropped) is 1.8% to 17.5%. The failures: a support row is missing in the child (39% to 64% of tries, almost all of them because the missing row belongs to the toggled constraint itself, see the 'toggled constraint' column; that is intrinsic, the child lies on the other side of that row), a negative multiplier (21% to 42%), the support does not span the row in the child (0.1% to 15%). The variant that solves a small LP over the rows tight at the parent's primal optimum validates only 3.6% to 17.7% and costs 2.4 to 9.5 us per try, as much as the per-row LP it would replace (2 to 11 us). `valid but LP keeps` is 0 on every problem here, but it need not be 0 in general: a row that only grazes the region has a Farkas bound equal to f_k, and the tolerance (+1e-9) is the lenient side, so the validity is an upper bound. The saved LP ns (1.5% to 17% of the LP ns of the rows tried) are smaller than the cost of trying every row, so the net is negative on every problem (-0.7% to -21.6% of the LP ns). Counts: the instrumented builder paths are the clip path (2 to 5 parameters) and `lp_path` (6 or more); regions built by the shared fallback are not counted (masses5_N3 has 2031 of its 5273 regions in the table, the rest took the shared builder after the chain guard switched the chain off; sys3x2_N5 has no per-row LP at all).

**Item 4.** "Found" is a rank-1 call that ran sub-facet tests and emitted at least one candidate; its time is the gross upper bound of what the corrected deferral can save (those calls become known-skips of about 1 us when the Case-1 facets have filled the space first; the drop in tested calls in `mplcp_defer_census.md` equals the drop in found calls). It is gross: the real deferral also pays one tableau resolve per region with a Case-2 facet and a call with other empty unseen members stays a tested call, so failing the rule is decisive and passing it would not have been a go. Counter check: pop_017 has 7662 calls, 1174 found, against 7644 and 1173 in the earlier census. The numerator is from the gate-profile census build and the denominator from the release base time (median of 3 tasks); the next column divides by the census-build solve time, the two readings agree within 1 point. Problems of 20 ms or more at 4% or more by the upper bound: masses2_N6, pop_021, rand_t10_x5_m15_s0, rand_t10_x5_m20_s0, rand_t16_x5_m15_s0, rand_t4_x5_m20_s0, rand_t5_x5_m20_s1, rand_t5_x6_m24_s0: 8 of 31 (rule: 10). The di and dinv problems (0.1 to 0.6 ms) have no rank-1 sub-facet tests. The masses problems, where the Case-2 time is, reach only 1.4% to 3% (masses4_N4 1.8%, masses5_N3 2.5%, masses4_N3 2.3%).

## Laptop samples before the cluster run (gate build, one solve)

pop_017: stage margins with the first probe (hint-only rays) m > 1e-8 on 57%; sub-facet calls 7662 with 1174 found; Build 7 validity 8.5% with 39% of tries missing a row. These agree with the cluster numbers above (the cluster census is the same counters on the same code, with the recentring and Toggle-start variants added).

## Tables (summary.md of the merge job, `bench/retry_census/merge.py`)

