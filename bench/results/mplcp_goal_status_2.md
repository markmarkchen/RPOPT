# mplcp against the best other solver, second measurement (main 59b2afb)

Measurement only, no solver change. Same 111 problems as `mplcp_goal_status.md` (f234e64), release profile, Negishi (AMD EPYC 7763),
serial, 1 CPU per task. Main 59b2afb adds the scale-free pivot test, the neighbour-search fix after refused tableau rebuilds, the
DAQP emptiness filter, cheaper Gamma tests, the 512 MiB tableau budget and the scale-free multiplier rule in `rank1_case2`.

**What was re-run (user's instruction: re-run the improved `mplcp` only).**

* **Validity of `mplcp`:** one cold solve per problem with the sampled gap check (`examples/gapcheck.rs`, 5,000 samples, limits
  15 minutes and 1.9 GB), array 45914215: 111 of 111 completed with regions and no sampled gap.
* **Timing of `mplcp`:** array of 333 tasks, one 1-CPU task per (problem, alternation), 3 alternations. A task runs `mplcp` and a second
  copy of `mplcp` (the A/A pair) with `alg_time`, in a shuffled order, with the previous repetition rule (one warm-up solve, then
  1 timed solve above 5 s, 3 from 1 to 5 s, 7 from 4 ms to 1 s, 51 and 301 below). The `mplcp` time is the median over the three tasks.
* **Other solvers:** not re-run. The reference is the best-other time stored in `mplcp_goal_status.md` (same solver, same problem).
  `graph` on `masses5_N4` and `masses4_N5` is 152.9 s and 140.0 s (`mplcp_scale_free_pivot.md`).

**Cross-run caveat.** The ratio is new `mplcp` ms over a stored time from another run, on other nodes and loads, so each ratio carries the
cross-run noise. The 1-CPU A/A range of one problem was 0.93 to 1.03, so a ratio from 0.93 to 1.07 is marked "within cross-run
noise" and is not a win or a loss. A/A of this run over 111 problems: median 1.000, range 0.979 to 1.032.
Nodes (17): a002 (42), a005 (25), a007 (44), a009 (11), a012 (21), a014 (27), a017 (16), a020 (48), a021 (4), a022 (5), a023 (20), a025 (12), a026 (14), a032 (19), a033 (6), a043 (12), a055 (7). Load average at task start and end: median 59, range 32 to 96
(shared 128-core nodes).

## Result

`mplcp` is at or below the best other solver on 42 of 111 problems (ratio at most 1; previous measurement: 38), and at or below
1.07 on 47. Time of `mplcp` against the previous measurement (geometric mean of new ms over old ms, all 111): 0.879;
by parameter count: 2: 1.049, 3: 0.951, 4: 0.984, 5: 0.944, 6: 0.709, 7: 0.958, 8: 0.851, 10: 0.777, 12: 0.722, 14: 0.777, 16: 0.802.

Region counts of `mplcp` changed against the previous file on 11 problems, all `masses` (old to new):
masses3_N5 10437 to 10436, masses3_N6 22145 to 22143, masses3_N7 38330 to 38326, masses4_N2 325 to 327, masses4_N3 2891 to 2947, masses4_N4 16050 to 16312, masses5_N2 370 to 373, masses5_N3 5060 to 5266, masses6_N2 380 to 383, masses7_N2 390 to 393, masses8_N2 395 to 399. The sampled gap check found no gap on any of them.

| params | problems | mplcp / best other, geometric mean | worst problem (ratio, winner) | not fastest (ratio above 1) | of those, ratio above 1.07 | previous (f234e64): geometric mean, not fastest |
|---:|---:|---:|---|---:|---:|---|
| 2 | 13 | 0.487 | di_N25 (0.63, parametric_daqp) | 0 | 0 | 0.462, 0 |
| 3 | 5 | 0.513 | sys3x2_N4 (0.61, geometric) | 0 | 0 | 0.533, 0 |
| 4 | 15 | 0.785 | pop_048 (3.74, combinatorial) | 4 | 4 | 0.792, 4 |
| 5 | 11 | 0.486 | pop_096 (1.82, combinatorial_graph) | 3 | 1 | 0.513, 4 |
| 6 | 18 | 1.190 | pop_081 (2.00, parametric_daqp) | 15 | 13 | 1.672, 17 |
| 7 | 15 | 1.230 | pop_021 (1.68, combinatorial_graph) | 14 | 13 | 1.287, 15 |
| 8 | 20 | 1.363 | masses4_N4 (2.66, parametric_daqp) | 19 | 19 | 1.599, 19 |
| 10 | 7 | 1.493 | masses5_N3 (3.21, parametric_daqp) | 7 | 7 | 1.921, 7 |
| 12 | 3 | 1.374 | masses6_N2 (1.51, parametric_daqp) | 3 | 3 | 1.905, 3 |
| 14 | 2 | 1.308 | masses7_N2 (1.54, parametric_daqp) | 2 | 2 | 1.683, 2 |
| 16 | 2 | 1.272 | masses8_N2 (1.41, parametric_daqp) | 2 | 2 | 1.581, 2 |
| all | 111 | 0.939 | pop_048 | 69 | 64 | 1.064, 73 |

### Problems whose side of 1 changed
| params | problem | previous | now | direction |
|---:|---|---:|---:|---|
| 5 | pop_001 | 1.031 | 0.924 | now below 1 |
| 6 | masses3_N6 | 3.715 | 0.988 | now below 1 (within noise) |
| 6 | masses3_N7 | 3.236 | 0.713 | now below 1 |
| 7 | pop_091 | 1.084 | 0.952 | now below 1 (within noise) |

pop_001, masses3_N6 and masses3_N7 were losses; masses3_N7 (0.71) is a clear win, pop_001 and pop_091 are near-ties, masses3_N6 is within noise.

### Large problems (one cold solve each)
| problem | mplcp s | peak resident GB | graph s (stored) | mplcp / graph |
|---|---:|---:|---:|---:|
| masses5_N4 | 456.4 | 0.85 | 152.9 | 2.98 |
| masses4_N5 | 425.2 | 0.83 | 140.0 | 3.04 |

Both are single solves in one task (the sampled check followed the solve; 44,065 and 67,963 regions, no gap). Before the scale-free
multiplier rule these took 1799 s and 1863 s (project memory), so `mplcp` is about 4x faster, still about 3x slower than `graph`. The
six large POP problems were not re-run.

### Problems where mplcp is not fastest (ratio above 1)
| params | problem | regions | mplcp ms | best other | best other ms | mplcp / best | previous ratio | note |
|---:|---|---:|---:|---|---:|---:|---:|---|
| 4 | pop_009 | 63 | 2.23 | parametric_daqp | 1.62 | 1.377 | 1.353 |  |
| 4 | pop_014 | 19 | 0.55 | parametric_daqp | 0.40 | 1.363 | 1.347 |  |
| 4 | pop_048 | 46 | 29.02 | combinatorial | 7.76 | 3.740 | 3.720 |  |
| 4 | pop_059 | 53 | 1.84 | parametric_daqp | 1.30 | 1.417 | 1.415 |  |
| 5 | pop_012 | 1047 | 66.24 | graph | 64.25 | 1.031 | 1.025 | within cross-run noise |
| 5 | pop_083 | 13266 | 1183.75 | parametric_daqp | 1110.59 | 1.066 | 1.219 | within cross-run noise |
| 5 | pop_096 | 258 | 37.71 | combinatorial_graph | 20.72 | 1.820 | 1.858 |  |
| 6 | masses3_N2 | 169 | 20.96 | parametric_daqp | 15.91 | 1.318 | 1.728 |  |
| 6 | masses3_N3 | 955 | 176.54 | parametric_daqp | 144.98 | 1.218 | 2.155 |  |
| 6 | masses3_N4 | 3643 | 947.00 | parametric_daqp | 906.70 | 1.044 | 2.688 | within cross-run noise |
| 6 | masses3_N5 | 10436 | 4825.43 | graph | 4516.38 | 1.068 | 4.764 | within cross-run noise |
| 6 | pop_011 | 397 | 32.83 | graph | 25.62 | 1.281 | 1.310 |  |
| 6 | pop_017 | 4142 | 532.70 | parametric_daqp | 420.23 | 1.268 | 1.255 |  |
| 6 | pop_025 | 5441 | 754.52 | parametric_daqp | 674.21 | 1.119 | 1.191 |  |
| 6 | pop_046 | 24 | 2.06 | graph | 1.15 | 1.794 | 1.874 |  |
| 6 | pop_049 | 9341 | 1341.76 | graph | 1207.84 | 1.111 | 1.123 |  |
| 6 | pop_053 | 2441 | 261.90 | graph | 213.54 | 1.226 | 1.223 |  |
| 6 | pop_081 | 4 | 0.16 | parametric_daqp | 0.08 | 2.000 | 1.902 |  |
| 6 | pop_084 | 665 | 47.84 | graph | 39.71 | 1.205 | 1.242 |  |
| 6 | rand_t6_x5_m20_s0 | 1774 | 171.77 | graph | 145.26 | 1.183 | 1.165 |  |
| 6 | rand_t6_x5_m20_s1 | 5062 | 555.12 | combinatorial_graph | 466.56 | 1.190 | 1.225 |  |
| 6 | rand_t6_x6_m24_s0 | 17134 | 2716.56 | combinatorial_graph | 2039.18 | 1.332 | 1.169 |  |
| 7 | pop_004 | 247 | 23.08 | parametric_daqp | 21.62 | 1.068 | 1.056 | within cross-run noise |
| 7 | pop_005 | 10289 | 2348.97 | graph | 2089.09 | 1.124 | 2.168 |  |
| 7 | pop_021 | 679 | 113.54 | combinatorial_graph | 67.66 | 1.678 | 1.647 |  |
| 7 | pop_030 | 286 | 24.02 | graph | 21.55 | 1.115 | 1.121 |  |
| 7 | pop_037 | 18615 | 3256.20 | parametric_daqp | 2332.59 | 1.396 | 1.369 |  |
| 7 | pop_054 | 20 | 1.67 | combinatorial_graph | 1.26 | 1.327 | 1.309 |  |
| 7 | pop_056 | 107 | 7.93 | parametric_daqp | 6.74 | 1.177 | 1.163 |  |
| 7 | pop_065 | 1089 | 157.21 | combinatorial | 126.98 | 1.238 | 1.288 |  |
| 7 | pop_068 | 86 | 7.02 | parametric_daqp | 5.95 | 1.179 | 1.176 |  |
| 7 | pop_073 | 23 | 2.24 | combinatorial_graph | 1.69 | 1.324 | 1.295 |  |
| 7 | pop_080 | 273 | 26.61 | graph | 23.08 | 1.153 | 1.212 |  |
| 7 | pop_085 | 121 | 9.47 | parametric_daqp | 8.10 | 1.169 | 1.150 |  |
| 7 | pop_086 | 3625 | 924.69 | combinatorial | 605.74 | 1.527 | 1.290 |  |
| 7 | pop_097 | 20574 | 5810.53 | parametric_daqp | 4805.23 | 1.209 | 1.310 |  |
| 8 | masses4_N2 | 327 | 91.01 | parametric_daqp | 61.46 | 1.481 | 2.340 |  |
| 8 | masses4_N3 | 2947 | 2030.91 | parametric_daqp | 899.62 | 2.258 | 4.217 |  |
| 8 | masses4_N4 | 16312 | 20977.98 | parametric_daqp | 7880.98 | 2.662 | 8.862 |  |
| 8 | pop_007 | 12235 | 4588.63 | parametric_daqp | 3008.36 | 1.525 | 3.495 |  |
| 8 | pop_010 | 36 | 8.35 | parametric_daqp | 5.08 | 1.643 | 1.571 |  |
| 8 | pop_016 | 384 | 74.84 | combinatorial_graph | 56.91 | 1.315 | 1.286 |  |
| 8 | pop_027 | 1472 | 319.31 | combinatorial | 270.07 | 1.182 | 1.223 |  |
| 8 | pop_028 | 82116 | 27348.03 | parametric_daqp | 24304.96 | 1.125 | 1.171 |  |
| 8 | pop_032 | 178950 | 59768.41 | parametric_daqp | 54087.48 | 1.105 | 1.208 |  |
| 8 | pop_061 | 33587 | 7849.09 | parametric_daqp | 6327.53 | 1.240 | 1.247 |  |
| 8 | pop_069 | 26 | 4.16 | combinatorial_graph | 3.25 | 1.279 | 1.240 |  |
| 8 | pop_076 | 15006 | 4128.07 | combinatorial | 3082.83 | 1.339 | 1.390 |  |
| 8 | pop_090 | 438 | 92.47 | combinatorial_graph | 70.91 | 1.304 | 1.284 |  |
| 8 | pop_098 | 269 | 50.94 | combinatorial_graph | 39.39 | 1.293 | 1.277 |  |
| 8 | rand_t8_x5_m15_s0 | 1035 | 141.91 | parametric_daqp | 110.93 | 1.279 | 1.163 |  |
| 8 | rand_t8_x5_m20_s0 | 4111 | 672.25 | graph | 560.41 | 1.200 | 1.209 |  |
| 8 | rand_t8_x5_m20_s1 | 8737 | 1781.69 | combinatorial_graph | 1325.43 | 1.344 | 1.393 |  |
| 8 | rand_t8_x6_m18_s0 | 9378 | 1561.09 | combinatorial_graph | 1166.76 | 1.338 | 1.359 |  |
| 8 | rand_t8_x6_m24_s0 | 32475 | 7495.34 | combinatorial_graph | 5611.01 | 1.336 | 1.358 |  |
| 10 | masses5_N2 | 373 | 205.54 | parametric_daqp | 130.96 | 1.569 | 2.906 |  |
| 10 | masses5_N3 | 5266 | 9670.21 | parametric_daqp | 3013.71 | 3.209 | 10.182 |  |
| 10 | rand_t10_x5_m15_s0 | 1859 | 417.82 | parametric_daqp | 347.22 | 1.203 | 1.212 |  |
| 10 | rand_t10_x5_m20_s0 | 6124 | 1682.99 | parametric_daqp | 1351.06 | 1.246 | 1.237 |  |
| 10 | rand_t10_x5_m20_s1 | 14848 | 4296.25 | combinatorial_graph | 3033.60 | 1.416 | 1.404 |  |
| 10 | rand_t10_x6_m18_s0 | 15312 | 3605.70 | parametric_daqp | 2924.19 | 1.233 | 1.229 |  |
| 10 | rand_t10_x6_m24_s0 | 71219 | 23699.77 | parametric_daqp | 18907.01 | 1.253 | 1.260 |  |
| 12 | masses6_N2 | 383 | 332.01 | parametric_daqp | 219.91 | 1.510 | 3.314 |  |
| 12 | rand_t12_x5_m15_s0 | 1012 | 323.98 | graph | 279.91 | 1.157 | 1.156 |  |
| 12 | rand_t12_x5_m20_s0 | 6143 | 3131.47 | parametric_daqp | 2107.62 | 1.486 | 1.805 |  |
| 14 | masses7_N2 | 393 | 683.28 | parametric_daqp | 444.68 | 1.537 | 2.536 |  |
| 14 | rand_t14_x5_m15_s0 | 2470 | 1325.68 | graph | 1190.82 | 1.113 | 1.117 |  |
| 16 | masses8_N2 | 399 | 1030.88 | parametric_daqp | 729.54 | 1.413 | 2.183 |  |
| 16 | rand_t16_x5_m15_s0 | 1876 | 1403.06 | graph | 1226.04 | 1.144 | 1.145 |  |

### All problems
| params | problem | regions | mplcp ms | best other | best other ms | mplcp / best | previous ratio | A/A |
|---:|---|---:|---:|---|---:|---:|---:|---:|
| 2 | di_N10 | 11 | 0.15 | geometric | 0.27 | 0.567 | 0.548 | 1.013 |
| 2 | di_N12 | 11 | 0.17 | geometric | 0.32 | 0.547 | 0.534 | 1.006 |
| 2 | di_N15 | 11 | 0.22 | geometric | 0.45 | 0.487 | 0.472 | 1.014 |
| 2 | di_N20 | 11 | 0.46 | parametric_daqp | 0.77 | 0.591 | 0.442 | 0.996 |
| 2 | di_N25 | 11 | 0.62 | parametric_daqp | 0.99 | 0.625 | 0.471 | 0.997 |
| 2 | di_N30 | 11 | 0.65 | parametric_daqp | 1.26 | 0.516 | 0.514 | 1.003 |
| 2 | di_N4 | 9 | 0.08 | geometric | 0.13 | 0.585 | 0.589 | 1.013 |
| 2 | di_N6 | 11 | 0.11 | geometric | 0.20 | 0.550 | 0.542 | 1.009 |
| 2 | di_N8 | 11 | 0.13 | parametric_daqp | 0.39 | 0.328 | 0.329 | 1.000 |
| 2 | di_doc_N3 | 7 | 0.06 | geometric | 0.11 | 0.582 | 0.587 | 1.000 |
| 2 | dinv_N4 | 9 | 0.07 | graph | 0.29 | 0.241 | 0.235 | 1.000 |
| 2 | dinv_N6 | 11 | 0.11 | parametric_daqp | 0.32 | 0.341 | 0.344 | 1.000 |
| 2 | dinv_N8 | 11 | 0.13 | geometric | 0.21 | 0.605 | 0.592 | 1.000 |
| 3 | sys3x2_N3 | 211 | 3.92 | geometric | 6.44 | 0.609 | 0.611 | 0.999 |
| 3 | sys3x2_N4 | 503 | 10.84 | geometric | 17.66 | 0.614 | 0.611 | 1.000 |
| 3 | sys3x2_N5 | 973 | 22.94 | geometric | 42.59 | 0.539 | 0.535 | 1.011 |
| 3 | sys3x2_N6 | 1503 | 38.80 | geometric | 80.03 | 0.485 | 0.471 | 1.003 |
| 3 | sys3x2_N7 | 2095 | 58.12 | geometric | 160.16 | 0.363 | 0.459 | 0.987 |
| 4 | masses2_N10 | 6381 | 397.77 | geometric | 1046.68 | 0.380 | 0.372 | 0.996 |
| 4 | masses2_N12 | 12779 | 1018.87 | geometric | 3684.09 | 0.277 | 0.313 | 0.998 |
| 4 | masses2_N4 | 307 | 13.42 | graph | 16.14 | 0.832 | 0.812 | 0.999 |
| 4 | masses2_N5 | 493 | 23.62 | graph | 37.56 | 0.629 | 0.608 | 1.009 |
| 4 | masses2_N6 | 901 | 47.73 | geometric | 77.78 | 0.614 | 0.576 | 0.999 |
| 4 | masses2_N7 | 1641 | 93.78 | geometric | 163.88 | 0.572 | 0.549 | 1.030 |
| 4 | masses2_N8 | 2837 | 162.67 | geometric | 322.84 | 0.504 | 0.500 | 1.007 |
| 4 | pop_009 | 63 | 2.23 | parametric_daqp | 1.62 | 1.377 | 1.353 | 1.005 |
| 4 | pop_014 | 19 | 0.55 | parametric_daqp | 0.40 | 1.363 | 1.347 | 1.000 |
| 4 | pop_019 | 6857 | 2369.80 | graph | 4680.07 | 0.506 | 0.560 | 1.001 |
| 4 | pop_023 | 4729 | 186.67 | graph | 243.90 | 0.765 | 0.868 | 0.993 |
| 4 | pop_043 | 899 | 38.36 | graph | 42.39 | 0.905 | 0.888 | 0.994 |
| 4 | pop_048 | 46 | 29.02 | combinatorial | 7.76 | 3.740 | 3.720 | 0.999 |
| 4 | pop_059 | 53 | 1.84 | parametric_daqp | 1.30 | 1.417 | 1.415 | 1.004 |
| 4 | pop_077 | 3915 | 134.29 | graph | 171.66 | 0.782 | 0.806 | 0.996 |
| 5 | pop_001 | 3529 | 16727.40 | parametric_daqp | 18112.51 | 0.924 | 1.031 | 0.998 |
| 5 | pop_008 | 21295 | 1604.37 | graph | 1880.89 | 0.853 | 0.864 | 1.001 |
| 5 | pop_012 | 1047 | 66.24 | graph | 64.25 | 1.031 | 1.025 | 0.994 |
| 5 | pop_040 | 2007 | 164.98 | graph | 263.67 | 0.626 | 0.653 | 1.032 |
| 5 | pop_057 | 34 | 2.98 | graph | 163.96 | 0.018 | 0.018 | 1.001 |
| 5 | pop_062 | 31727 | 3024.86 | parametric_daqp | 3984.22 | 0.759 | 0.763 | 1.003 |
| 5 | pop_070 | 703 | 55.09 | graph | 798.02 | 0.069 | 0.069 | 0.998 |
| 5 | pop_075 | 119 | 4.74 | graph | 5.17 | 0.917 | 0.921 | 1.003 |
| 5 | pop_083 | 13266 | 1183.75 | parametric_daqp | 1110.59 | 1.066 | 1.219 | 1.000 |
| 5 | pop_088 | 62318 | 5100.71 | parametric_daqp | 12281.69 | 0.415 | 0.551 | 0.999 |
| 5 | pop_096 | 258 | 37.71 | combinatorial_graph | 20.72 | 1.820 | 1.858 | 1.001 |
| 6 | masses3_N2 | 169 | 20.96 | parametric_daqp | 15.91 | 1.318 | 1.728 | 0.985 |
| 6 | masses3_N3 | 955 | 176.54 | parametric_daqp | 144.98 | 1.218 | 2.155 | 1.002 |
| 6 | masses3_N4 | 3643 | 947.00 | parametric_daqp | 906.70 | 1.044 | 2.688 | 1.001 |
| 6 | masses3_N5 | 10436 | 4825.43 | graph | 4516.38 | 1.068 | 4.764 | 1.000 |
| 6 | masses3_N6 | 22143 | 15553.84 | graph | 15739.07 | 0.988 | 3.715 | 1.002 |
| 6 | masses3_N7 | 38326 | 38741.51 | graph | 54336.70 | 0.713 | 3.236 | 0.998 |
| 6 | pop_011 | 397 | 32.83 | graph | 25.62 | 1.281 | 1.310 | 0.999 |
| 6 | pop_017 | 4142 | 532.70 | parametric_daqp | 420.23 | 1.268 | 1.255 | 0.999 |
| 6 | pop_018 | 20800 | 2595.48 | parametric_daqp | 2863.50 | 0.906 | 0.916 | 1.007 |
| 6 | pop_025 | 5441 | 754.52 | parametric_daqp | 674.21 | 1.119 | 1.191 | 0.999 |
| 6 | pop_046 | 24 | 2.06 | graph | 1.15 | 1.794 | 1.874 | 1.002 |
| 6 | pop_049 | 9341 | 1341.76 | graph | 1207.84 | 1.111 | 1.123 | 0.999 |
| 6 | pop_053 | 2441 | 261.90 | graph | 213.54 | 1.226 | 1.223 | 1.010 |
| 6 | pop_081 | 4 | 0.16 | parametric_daqp | 0.08 | 2.000 | 1.902 | 1.006 |
| 6 | pop_084 | 665 | 47.84 | graph | 39.71 | 1.205 | 1.242 | 0.999 |
| 6 | rand_t6_x5_m20_s0 | 1774 | 171.77 | graph | 145.26 | 1.183 | 1.165 | 1.010 |
| 6 | rand_t6_x5_m20_s1 | 5062 | 555.12 | combinatorial_graph | 466.56 | 1.190 | 1.225 | 1.003 |
| 6 | rand_t6_x6_m24_s0 | 17134 | 2716.56 | combinatorial_graph | 2039.18 | 1.332 | 1.169 | 0.979 |
| 7 | pop_004 | 247 | 23.08 | parametric_daqp | 21.62 | 1.068 | 1.056 | 1.001 |
| 7 | pop_005 | 10289 | 2348.97 | graph | 2089.09 | 1.124 | 2.168 | 0.998 |
| 7 | pop_021 | 679 | 113.54 | combinatorial_graph | 67.66 | 1.678 | 1.647 | 1.009 |
| 7 | pop_030 | 286 | 24.02 | graph | 21.55 | 1.115 | 1.121 | 0.998 |
| 7 | pop_037 | 18615 | 3256.20 | parametric_daqp | 2332.59 | 1.396 | 1.369 | 1.005 |
| 7 | pop_054 | 20 | 1.67 | combinatorial_graph | 1.26 | 1.327 | 1.309 | 1.007 |
| 7 | pop_056 | 107 | 7.93 | parametric_daqp | 6.74 | 1.177 | 1.163 | 0.998 |
| 7 | pop_065 | 1089 | 157.21 | combinatorial | 126.98 | 1.238 | 1.288 | 1.000 |
| 7 | pop_068 | 86 | 7.02 | parametric_daqp | 5.95 | 1.179 | 1.176 | 1.002 |
| 7 | pop_073 | 23 | 2.24 | combinatorial_graph | 1.69 | 1.324 | 1.295 | 1.005 |
| 7 | pop_080 | 273 | 26.61 | graph | 23.08 | 1.153 | 1.212 | 1.003 |
| 7 | pop_085 | 121 | 9.47 | parametric_daqp | 8.10 | 1.169 | 1.150 | 1.001 |
| 7 | pop_086 | 3625 | 924.69 | combinatorial | 605.74 | 1.527 | 1.290 | 1.004 |
| 7 | pop_091 | 145018 | 35372.79 | parametric_daqp | 37170.18 | 0.952 | 1.084 | 1.000 |
| 7 | pop_097 | 20574 | 5810.53 | parametric_daqp | 4805.23 | 1.209 | 1.310 | 1.001 |
| 8 | masses4_N2 | 327 | 91.01 | parametric_daqp | 61.46 | 1.481 | 2.340 | 1.005 |
| 8 | masses4_N3 | 2947 | 2030.91 | parametric_daqp | 899.62 | 2.258 | 4.217 | 1.004 |
| 8 | masses4_N4 | 16312 | 20977.98 | parametric_daqp | 7880.98 | 2.662 | 8.862 | 1.004 |
| 8 | pop_007 | 12235 | 4588.63 | parametric_daqp | 3008.36 | 1.525 | 3.495 | 0.996 |
| 8 | pop_010 | 36 | 8.35 | parametric_daqp | 5.08 | 1.643 | 1.571 | 0.996 |
| 8 | pop_016 | 384 | 74.84 | combinatorial_graph | 56.91 | 1.315 | 1.286 | 1.003 |
| 8 | pop_027 | 1472 | 319.31 | combinatorial | 270.07 | 1.182 | 1.223 | 1.000 |
| 8 | pop_028 | 82116 | 27348.03 | parametric_daqp | 24304.96 | 1.125 | 1.171 | 0.998 |
| 8 | pop_032 | 178950 | 59768.41 | parametric_daqp | 54087.48 | 1.105 | 1.208 | 1.000 |
| 8 | pop_047 | 48144 | 13308.43 | parametric_daqp | 15547.83 | 0.856 | 0.874 | 1.016 |
| 8 | pop_061 | 33587 | 7849.09 | parametric_daqp | 6327.53 | 1.240 | 1.247 | 0.995 |
| 8 | pop_069 | 26 | 4.16 | combinatorial_graph | 3.25 | 1.279 | 1.240 | 1.006 |
| 8 | pop_076 | 15006 | 4128.07 | combinatorial | 3082.83 | 1.339 | 1.390 | 0.991 |
| 8 | pop_090 | 438 | 92.47 | combinatorial_graph | 70.91 | 1.304 | 1.284 | 1.000 |
| 8 | pop_098 | 269 | 50.94 | combinatorial_graph | 39.39 | 1.293 | 1.277 | 1.001 |
| 8 | rand_t8_x5_m15_s0 | 1035 | 141.91 | parametric_daqp | 110.93 | 1.279 | 1.163 | 1.007 |
| 8 | rand_t8_x5_m20_s0 | 4111 | 672.25 | graph | 560.41 | 1.200 | 1.209 | 0.997 |
| 8 | rand_t8_x5_m20_s1 | 8737 | 1781.69 | combinatorial_graph | 1325.43 | 1.344 | 1.393 | 1.003 |
| 8 | rand_t8_x6_m18_s0 | 9378 | 1561.09 | combinatorial_graph | 1166.76 | 1.338 | 1.359 | 0.995 |
| 8 | rand_t8_x6_m24_s0 | 32475 | 7495.34 | combinatorial_graph | 5611.01 | 1.336 | 1.358 | 1.000 |
| 10 | masses5_N2 | 373 | 205.54 | parametric_daqp | 130.96 | 1.569 | 2.906 | 1.000 |
| 10 | masses5_N3 | 5266 | 9670.21 | parametric_daqp | 3013.71 | 3.209 | 10.182 | 1.002 |
| 10 | rand_t10_x5_m15_s0 | 1859 | 417.82 | parametric_daqp | 347.22 | 1.203 | 1.212 | 0.997 |
| 10 | rand_t10_x5_m20_s0 | 6124 | 1682.99 | parametric_daqp | 1351.06 | 1.246 | 1.237 | 1.001 |
| 10 | rand_t10_x5_m20_s1 | 14848 | 4296.25 | combinatorial_graph | 3033.60 | 1.416 | 1.404 | 1.002 |
| 10 | rand_t10_x6_m18_s0 | 15312 | 3605.70 | parametric_daqp | 2924.19 | 1.233 | 1.229 | 1.000 |
| 10 | rand_t10_x6_m24_s0 | 71219 | 23699.77 | parametric_daqp | 18907.01 | 1.253 | 1.260 | 0.999 |
| 12 | masses6_N2 | 383 | 332.01 | parametric_daqp | 219.91 | 1.510 | 3.314 | 1.001 |
| 12 | rand_t12_x5_m15_s0 | 1012 | 323.98 | graph | 279.91 | 1.157 | 1.156 | 0.999 |
| 12 | rand_t12_x5_m20_s0 | 6143 | 3131.47 | parametric_daqp | 2107.62 | 1.486 | 1.805 | 0.999 |
| 14 | masses7_N2 | 393 | 683.28 | parametric_daqp | 444.68 | 1.537 | 2.536 | 1.002 |
| 14 | rand_t14_x5_m15_s0 | 2470 | 1325.68 | graph | 1190.82 | 1.113 | 1.117 | 0.997 |
| 16 | masses8_N2 | 399 | 1030.88 | parametric_daqp | 729.54 | 1.413 | 2.183 | 1.001 |
| 16 | rand_t16_x5_m15_s0 | 1876 | 1403.06 | graph | 1226.04 | 1.144 | 1.145 | 0.995 |
