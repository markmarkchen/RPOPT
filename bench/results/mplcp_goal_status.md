# mplcp against the best other solver, 111 problems (main f234e64)

Measurement only, no solver change. Tree f234e64 (rank-1 enumeration, witness hints, certificate index), release profile,
Negishi (AMD EPYC 7763), serial, 1 CPU per task.

**Problems.** Every problem of `bench/problems`, `bench/problems_large` and `bench/problems_highdim` with 4 or more
parameters, plus the 23-problem set of the small end (di, dinv, sys3x2, masses2_N4 to N8), 111 in all, minus
`masses5_N4`, `masses4_N5` and six large POP problems (`pop_002`, `pop_024`, `pop_033`, `pop_036`, `pop_051`, `pop_095`),
which `mplcp-case2-fix` measures. **Solvers:** `mplcp`, `graph`, `combinatorial_graph`, `geometric`, `combinatorial`,
`parametric_daqp`.

**Validity (screening, job arrays 45892068 and 45893417).** Each (solver, problem) was solved once, cold, in its own task
(limits 15 minutes and 1.9 GB) with a sampled coverage check (5,000 uniform samples of the parameter box; a feasible sample in no
region is a gap). Of 666 runs: 646 completed with regions and no gap; no run returned 0 regions; one has a gap
(`graph` on `pop_048`: 56 regions, 1 sampled gap, so `graph` is not a valid reference there); 15 were killed by the 1.9 GB
limit (`combinatorial` on di_N12, N15, N20, N25, N30, masses2_N10, masses2_N12, masses3_N7, sys3x2_N6, sys3x2_N7; `geometric` on masses3_N4,
masses3_N5, masses3_N7, masses4_N4, masses5_N3) and 4 hit the 15 minute limit (`geometric` on masses3_N6, pop_028, pop_032, pop_091).
Killed and timed-out runs are not valid references.

**Timing (job array 45894359).** Per problem the reference is the fastest valid other solver of the screening, plus the
next two if they were within 1.5x of it. One task per (problem, alternation), 3 alternations; in a task `mplcp`, those solvers
and a second copy of `mplcp` (the A/A pair) each run once in a shuffled order (`alg_time`: one warm-up solve, then 1 timed solve
above 5 s, 3 from 1 to 5 s, 7 from 4 ms to 1 s, 51 and 301 below). The reported ratio is `mplcp` time over the time of the
best reference, the median over the three paired tasks (the reference is the one with the lowest median time). Below 1:
`mplcp` is the fastest. A/A over the 111 problems: median 1.000, range 0.955 to 1.037.

| params | problems | mplcp / best other, geometric mean | worst problem (ratio, winner) | problems where mplcp is not fastest |
|---:|---:|---:|---|---:|
| 2 | 13 | 0.462 | dinv_N8 (0.59, geometric) | 0 |
| 3 | 5 | 0.533 | sys3x2_N3 (0.61, geometric) | 0 |
| 4 | 15 | 0.792 | pop_048 (3.72, combinatorial) | 4 |
| 5 | 11 | 0.514 | pop_096 (1.86, combinatorial_graph) | 4 |
| 6 | 18 | 1.672 | masses3_N5 (4.76, graph) | 17 |
| 7 | 15 | 1.287 | pop_005 (2.17, graph) | 15 |
| 8 | 20 | 1.599 | masses4_N4 (8.86, parametric_daqp) | 19 |
| 10 | 7 | 1.921 | masses5_N3 (10.18, parametric_daqp) | 7 |
| 12 | 3 | 1.905 | masses6_N2 (3.31, parametric_daqp) | 3 |
| 14 | 2 | 1.683 | masses7_N2 (2.54, parametric_daqp) | 2 |
| 16 | 2 | 1.581 | masses8_N2 (2.18, parametric_daqp) | 2 |
all 111 problems: geometric mean 1.064, median 1.208, worst 10.182, mplcp fastest on 38
A/A (mplcp against a second copy, same task): median 1.000, range 0.955 to 1.037, over 111 problems
### Problems where mplcp is not fastest (ratio above 1)
| params | problem | regions | mplcp ms | best other | best other ms | mplcp / best | other solvers (ratio mplcp / solver) |
|---:|---|---:|---:|---|---:|---:|---|
| 4 | pop_009 | 63 | 2.20 | parametric_daqp | 1.62 | 1.353 | [('parametric_daqp', 1.353)] |
| 4 | pop_014 | 19 | 0.54 | parametric_daqp | 0.40 | 1.347 | [('parametric_daqp', 1.347), ('combinatorial', 1.002)] |
| 4 | pop_048 | 46 | 28.82 | combinatorial | 7.76 | 3.720 | [('combinatorial', 3.72)] |
| 4 | pop_059 | 53 | 1.88 | parametric_daqp | 1.30 | 1.415 | [('parametric_daqp', 1.415), ('combinatorial_graph', 1.393), ('combinatorial', 1.234)] |
| 5 | pop_001 | 3529 | 18680.88 | parametric_daqp | 18112.51 | 1.031 | [('parametric_daqp', 1.031), ('combinatorial', 0.729), ('geometric', 0.68)] |
| 5 | pop_012 | 1047 | 66.13 | graph | 64.25 | 1.025 | [('graph', 1.025), ('parametric_daqp', 0.979), ('combinatorial_graph', 0.893)] |
| 5 | pop_083 | 13266 | 1361.84 | parametric_daqp | 1110.59 | 1.219 | [('parametric_daqp', 1.219), ('combinatorial_graph', 1.112), ('combinatorial', 1.094)] |
| 5 | pop_096 | 258 | 39.08 | combinatorial_graph | 20.72 | 1.858 | [('combinatorial_graph', 1.858)] |
| 6 | masses3_N2 | 169 | 27.26 | parametric_daqp | 15.91 | 1.728 | [('parametric_daqp', 1.728), ('graph', 1.583), ('combinatorial', 1.525)] |
| 6 | masses3_N3 | 955 | 312.56 | parametric_daqp | 144.98 | 2.155 | [('parametric_daqp', 2.155), ('graph', 1.895), ('combinatorial', 1.874)] |
| 6 | masses3_N4 | 3643 | 2416.31 | parametric_daqp | 906.70 | 2.688 | [('parametric_daqp', 2.688), ('graph', 2.41), ('combinatorial', 2.246)] |
| 6 | masses3_N5 | 10437 | 21500.01 | graph | 4516.38 | 4.764 | [('graph', 4.764), ('parametric_daqp', 4.491), ('combinatorial', 3.377)] |
| 6 | masses3_N6 | 22145 | 58469.65 | graph | 15739.07 | 3.715 | [('graph', 3.715), ('parametric_daqp', 2.747)] |
| 6 | masses3_N7 | 38330 | 184966.65 | graph | 54336.70 | 3.236 | [('graph', 3.236)] |
| 6 | pop_011 | 397 | 33.55 | graph | 25.62 | 1.310 | [('graph', 1.31), ('parametric_daqp', 1.095), ('combinatorial', 1.036)] |
| 6 | pop_017 | 4142 | 527.71 | parametric_daqp | 420.23 | 1.255 | [('parametric_daqp', 1.255), ('graph', 1.192), ('combinatorial', 1.145)] |
| 6 | pop_025 | 5441 | 801.72 | parametric_daqp | 674.21 | 1.191 | [('parametric_daqp', 1.191), ('combinatorial_graph', 1.153), ('combinatorial', 1.085)] |
| 6 | pop_046 | 24 | 2.15 | graph | 1.15 | 1.874 | [('graph', 1.874), ('combinatorial_graph', 1.661), ('combinatorial', 1.637)] |
| 6 | pop_049 | 9341 | 1356.43 | graph | 1207.84 | 1.123 | [('graph', 1.123), ('parametric_daqp', 0.973), ('combinatorial_graph', 0.85)] |
| 6 | pop_053 | 2441 | 261.21 | graph | 213.54 | 1.223 | [('graph', 1.223), ('parametric_daqp', 1.048), ('combinatorial_graph', 0.972)] |
| 6 | pop_081 | 4 | 0.16 | parametric_daqp | 0.08 | 1.902 | [('parametric_daqp', 1.902)] |
| 6 | pop_084 | 665 | 49.15 | graph | 39.71 | 1.242 | [('graph', 1.242), ('parametric_daqp', 1.215), ('combinatorial', 1.089)] |
| 6 | rand_t6_x5_m20_s0 | 1774 | 169.92 | graph | 145.26 | 1.165 | [('graph', 1.165), ('parametric_daqp', 1.141), ('combinatorial_graph', 1.054)] |
| 6 | rand_t6_x5_m20_s1 | 5062 | 571.53 | combinatorial_graph | 466.56 | 1.225 | [('combinatorial_graph', 1.225), ('parametric_daqp', 1.158), ('graph', 1.127)] |
| 6 | rand_t6_x6_m24_s0 | 17134 | 2372.33 | combinatorial_graph | 2039.18 | 1.169 | [('combinatorial_graph', 1.169), ('parametric_daqp', 1.158), ('combinatorial', 1.035)] |
| 7 | pop_004 | 247 | 22.83 | parametric_daqp | 21.62 | 1.056 | [('parametric_daqp', 1.056), ('combinatorial', 1.012), ('combinatorial_graph', 0.987)] |
| 7 | pop_005 | 10289 | 4530.67 | graph | 2089.09 | 2.168 | [('graph', 2.168), ('parametric_daqp', 1.676)] |
| 7 | pop_021 | 679 | 108.97 | combinatorial_graph | 67.66 | 1.647 | [('combinatorial_graph', 1.647), ('parametric_daqp', 1.577), ('combinatorial', 1.543)] |
| 7 | pop_030 | 286 | 24.16 | graph | 21.55 | 1.121 | [('graph', 1.121)] |
| 7 | pop_037 | 18615 | 3178.45 | parametric_daqp | 2332.59 | 1.369 | [('parametric_daqp', 1.369), ('combinatorial_graph', 1.362), ('combinatorial', 1.243)] |
| 7 | pop_054 | 20 | 1.64 | combinatorial_graph | 1.26 | 1.309 | [('combinatorial_graph', 1.309), ('parametric_daqp', 1.297), ('graph', 1.11)] |
| 7 | pop_056 | 107 | 7.83 | parametric_daqp | 6.74 | 1.163 | [('parametric_daqp', 1.163), ('combinatorial_graph', 1.116), ('combinatorial', 1.112)] |
| 7 | pop_065 | 1089 | 163.38 | combinatorial | 126.98 | 1.288 | [('combinatorial', 1.288), ('graph', 1.267), ('combinatorial_graph', 1.26)] |
| 7 | pop_068 | 86 | 6.99 | parametric_daqp | 5.95 | 1.176 | [('parametric_daqp', 1.176), ('combinatorial_graph', 1.136), ('combinatorial', 1.101)] |
| 7 | pop_073 | 23 | 2.19 | combinatorial_graph | 1.69 | 1.295 | [('combinatorial_graph', 1.295), ('parametric_daqp', 1.289), ('combinatorial', 1.257)] |
| 7 | pop_080 | 273 | 28.09 | graph | 23.08 | 1.212 | [('graph', 1.212), ('combinatorial_graph', 1.155), ('parametric_daqp', 1.15)] |
| 7 | pop_085 | 121 | 9.32 | parametric_daqp | 8.10 | 1.150 | [('parametric_daqp', 1.15), ('combinatorial_graph', 1.157), ('graph', 1.15)] |
| 7 | pop_086 | 3625 | 780.33 | combinatorial | 605.74 | 1.290 | [('combinatorial', 1.29), ('parametric_daqp', 1.269), ('graph', 1.198)] |
| 7 | pop_091 | 145018 | 40285.36 | parametric_daqp | 37170.18 | 1.084 | [('parametric_daqp', 1.084), ('combinatorial', 1.034), ('combinatorial_graph', 1.0)] |
| 7 | pop_097 | 20574 | 6293.06 | parametric_daqp | 4805.23 | 1.310 | [('parametric_daqp', 1.31), ('combinatorial_graph', 1.287), ('combinatorial', 1.279)] |
| 8 | masses4_N2 | 325 | 143.83 | parametric_daqp | 61.46 | 2.340 | [('parametric_daqp', 2.34), ('combinatorial', 1.893), ('graph', 1.757)] |
| 8 | masses4_N3 | 2891 | 3795.06 | parametric_daqp | 899.62 | 4.217 | [('parametric_daqp', 4.217), ('combinatorial', 2.877)] |
| 8 | masses4_N4 | 16050 | 70023.64 | parametric_daqp | 7880.98 | 8.862 | [('parametric_daqp', 8.862)] |
| 8 | pop_007 | 12235 | 10515.23 | parametric_daqp | 3008.36 | 3.495 | [('parametric_daqp', 3.495), ('combinatorial', 3.357), ('combinatorial_graph', 3.293)] |
| 8 | pop_010 | 36 | 7.98 | parametric_daqp | 5.08 | 1.571 | [('parametric_daqp', 1.571), ('combinatorial_graph', 1.561), ('combinatorial', 1.533)] |
| 8 | pop_016 | 384 | 73.28 | combinatorial_graph | 56.91 | 1.286 | [('combinatorial_graph', 1.286), ('parametric_daqp', 1.265), ('combinatorial', 1.26)] |
| 8 | pop_027 | 1472 | 335.57 | combinatorial | 270.07 | 1.223 | [('combinatorial', 1.223), ('combinatorial_graph', 1.216), ('parametric_daqp', 1.214)] |
| 8 | pop_028 | 82116 | 28428.60 | parametric_daqp | 24304.96 | 1.171 | [('parametric_daqp', 1.171), ('combinatorial', 1.062), ('combinatorial_graph', 1.069)] |
| 8 | pop_032 | 178950 | 65515.58 | parametric_daqp | 54087.48 | 1.208 | [('parametric_daqp', 1.208), ('combinatorial_graph', 1.122), ('combinatorial', 1.033)] |
| 8 | pop_061 | 33587 | 7903.38 | parametric_daqp | 6327.53 | 1.247 | [('parametric_daqp', 1.247), ('combinatorial', 1.22), ('combinatorial_graph', 1.195)] |
| 8 | pop_069 | 26 | 4.04 | combinatorial_graph | 3.25 | 1.240 | [('combinatorial_graph', 1.24), ('parametric_daqp', 1.232), ('combinatorial', 1.217)] |
| 8 | pop_076 | 15006 | 4285.88 | combinatorial | 3082.83 | 1.390 | [('combinatorial', 1.39), ('graph', 1.31), ('combinatorial_graph', 1.26)] |
| 8 | pop_090 | 438 | 91.12 | combinatorial_graph | 70.91 | 1.284 | [('combinatorial_graph', 1.284), ('combinatorial', 1.262), ('graph', 1.201)] |
| 8 | pop_098 | 269 | 50.33 | combinatorial_graph | 39.39 | 1.277 | [('combinatorial_graph', 1.277), ('parametric_daqp', 1.275), ('combinatorial', 1.258)] |
| 8 | rand_t8_x5_m15_s0 | 1035 | 129.33 | parametric_daqp | 110.93 | 1.163 | [('parametric_daqp', 1.163), ('graph', 1.161), ('combinatorial', 1.099)] |
| 8 | rand_t8_x5_m20_s0 | 4111 | 677.27 | graph | 560.41 | 1.209 | [('graph', 1.209), ('parametric_daqp', 1.209), ('combinatorial_graph', 1.182)] |
| 8 | rand_t8_x5_m20_s1 | 8737 | 1844.51 | combinatorial_graph | 1325.43 | 1.393 | [('combinatorial_graph', 1.393), ('parametric_daqp', 1.37), ('combinatorial', 1.333)] |
| 8 | rand_t8_x6_m18_s0 | 9378 | 1583.32 | combinatorial_graph | 1166.76 | 1.359 | [('combinatorial_graph', 1.359), ('parametric_daqp', 1.292), ('graph', 1.179)] |
| 8 | rand_t8_x6_m24_s0 | 32475 | 7611.73 | combinatorial_graph | 5611.01 | 1.358 | [('combinatorial_graph', 1.358), ('parametric_daqp', 1.294), ('combinatorial', 1.216)] |
| 10 | masses5_N2 | 370 | 379.25 | parametric_daqp | 130.96 | 2.906 | [('parametric_daqp', 2.906), ('combinatorial', 2.141)] |
| 10 | masses5_N3 | 5060 | 30658.38 | parametric_daqp | 3013.71 | 10.182 | [('parametric_daqp', 10.182)] |
| 10 | rand_t10_x5_m15_s0 | 1859 | 421.51 | parametric_daqp | 347.22 | 1.212 | [('parametric_daqp', 1.212), ('graph', 1.207), ('combinatorial_graph', 1.2)] |
| 10 | rand_t10_x5_m20_s0 | 6124 | 1681.02 | parametric_daqp | 1351.06 | 1.237 | [('parametric_daqp', 1.237), ('graph', 1.242), ('combinatorial', 1.239)] |
| 10 | rand_t10_x5_m20_s1 | 14848 | 4257.78 | combinatorial_graph | 3033.60 | 1.404 | [('combinatorial_graph', 1.404), ('combinatorial', 1.366), ('parametric_daqp', 1.36)] |
| 10 | rand_t10_x6_m18_s0 | 15312 | 3597.10 | parametric_daqp | 2924.19 | 1.229 | [('parametric_daqp', 1.229), ('combinatorial', 1.224), ('combinatorial_graph', 1.2)] |
| 10 | rand_t10_x6_m24_s0 | 71219 | 23719.90 | parametric_daqp | 18907.01 | 1.260 | [('parametric_daqp', 1.26), ('combinatorial', 1.239), ('combinatorial_graph', 1.242)] |
| 12 | masses6_N2 | 380 | 728.87 | parametric_daqp | 219.91 | 3.314 | [('parametric_daqp', 3.314), ('combinatorial', 2.297)] |
| 12 | rand_t12_x5_m15_s0 | 1012 | 323.51 | graph | 279.91 | 1.156 | [('graph', 1.156), ('combinatorial_graph', 1.114), ('combinatorial', 1.082)] |
| 12 | rand_t12_x5_m20_s0 | 6143 | 3803.37 | parametric_daqp | 2107.62 | 1.805 | [('parametric_daqp', 1.805), ('graph', 1.797), ('combinatorial', 1.757)] |
| 14 | masses7_N2 | 390 | 1125.33 | parametric_daqp | 444.68 | 2.536 | [('parametric_daqp', 2.536), ('combinatorial', 1.837)] |
| 14 | rand_t14_x5_m15_s0 | 2470 | 1333.51 | graph | 1190.82 | 1.117 | [('graph', 1.117), ('parametric_daqp', 1.127), ('combinatorial_graph', 1.098)] |
| 16 | masses8_N2 | 395 | 1592.38 | parametric_daqp | 729.54 | 2.183 | [('parametric_daqp', 2.183), ('combinatorial', 1.6), ('graph', 1.463)] |
| 16 | rand_t16_x5_m15_s0 | 1876 | 1410.53 | graph | 1226.04 | 1.145 | [('graph', 1.145), ('parametric_daqp', 1.139), ('combinatorial', 1.119)] |
### All problems
| params | problem | regions | mplcp ms | best other | best other ms | mplcp / best | A/A | tasks |
|---:|---|---:|---:|---|---:|---:|---:|---:|
| 2 | di_N10 | 11 | 0.15 | geometric | 0.27 | 0.548 | 0.993 | 3 |
| 2 | di_N12 | 11 | 0.17 | geometric | 0.32 | 0.534 | 1.012 | 3 |
| 2 | di_N15 | 11 | 0.21 | geometric | 0.45 | 0.472 | 0.995 | 3 |
| 2 | di_N20 | 11 | 0.34 | parametric_daqp | 0.77 | 0.442 | 1.021 | 3 |
| 2 | di_N25 | 11 | 0.47 | parametric_daqp | 0.99 | 0.471 | 1.015 | 3 |
| 2 | di_N30 | 11 | 0.65 | parametric_daqp | 1.26 | 0.514 | 1.012 | 3 |
| 2 | di_N4 | 9 | 0.08 | geometric | 0.13 | 0.589 | 0.987 | 3 |
| 2 | di_N6 | 11 | 0.11 | geometric | 0.20 | 0.542 | 1.000 | 3 |
| 2 | di_N8 | 11 | 0.13 | parametric_daqp | 0.39 | 0.329 | 1.000 | 3 |
| 2 | di_doc_N3 | 7 | 0.06 | geometric | 0.11 | 0.587 | 1.000 | 3 |
| 2 | dinv_N4 | 9 | 0.07 | graph | 0.29 | 0.235 | 0.986 | 3 |
| 2 | dinv_N6 | 11 | 0.11 | parametric_daqp | 0.32 | 0.344 | 1.000 | 3 |
| 2 | dinv_N8 | 11 | 0.13 | geometric | 0.21 | 0.592 | 0.984 | 3 |
| 3 | sys3x2_N3 | 211 | 3.90 | geometric | 6.44 | 0.611 | 1.002 | 3 |
| 3 | sys3x2_N4 | 503 | 10.78 | geometric | 17.66 | 0.611 | 1.004 | 3 |
| 3 | sys3x2_N5 | 973 | 22.71 | geometric | 42.59 | 0.535 | 1.002 | 3 |
| 3 | sys3x2_N6 | 1503 | 37.68 | geometric | 80.03 | 0.471 | 1.001 | 3 |
| 3 | sys3x2_N7 | 2095 | 78.56 | geometric | 160.16 | 0.459 | 0.978 | 3 |
| 4 | masses2_N10 | 6381 | 388.65 | geometric | 1046.68 | 0.372 | 0.997 | 3 |
| 4 | masses2_N12 | 12779 | 1236.15 | geometric | 3684.09 | 0.313 | 1.010 | 3 |
| 4 | masses2_N4 | 307 | 13.13 | graph | 16.14 | 0.812 | 1.002 | 3 |
| 4 | masses2_N5 | 493 | 22.83 | graph | 37.56 | 0.608 | 1.002 | 3 |
| 4 | masses2_N6 | 901 | 44.76 | geometric | 77.78 | 0.576 | 0.996 | 3 |
| 4 | masses2_N7 | 1641 | 90.05 | geometric | 163.88 | 0.549 | 1.037 | 3 |
| 4 | masses2_N8 | 2837 | 161.16 | geometric | 322.84 | 0.500 | 0.998 | 3 |
| 4 | pop_009 | 63 | 2.20 | parametric_daqp | 1.62 | 1.353 | 1.000 | 3 |
| 4 | pop_014 | 19 | 0.54 | parametric_daqp | 0.40 | 1.347 | 1.000 | 3 |
| 4 | pop_019 | 6857 | 2666.31 | graph | 4680.07 | 0.560 | 1.011 | 3 |
| 4 | pop_023 | 4729 | 210.81 | graph | 243.90 | 0.868 | 0.955 | 3 |
| 4 | pop_043 | 899 | 37.67 | graph | 42.39 | 0.888 | 0.999 | 3 |
| 4 | pop_048 | 46 | 28.82 | combinatorial | 7.76 | 3.720 | 1.000 | 3 |
| 4 | pop_059 | 53 | 1.88 | parametric_daqp | 1.30 | 1.415 | 1.008 | 3 |
| 4 | pop_077 | 3915 | 138.30 | graph | 171.66 | 0.806 | 1.000 | 3 |
| 5 | pop_001 | 3529 | 18680.88 | parametric_daqp | 18112.51 | 1.031 | 0.999 | 3 |
| 5 | pop_008 | 21295 | 1629.33 | graph | 1880.89 | 0.864 | 0.995 | 3 |
| 5 | pop_012 | 1047 | 66.13 | graph | 64.25 | 1.025 | 0.997 | 3 |
| 5 | pop_040 | 2007 | 172.05 | graph | 263.67 | 0.653 | 1.001 | 3 |
| 5 | pop_057 | 34 | 2.99 | graph | 163.96 | 0.018 | 1.001 | 3 |
| 5 | pop_062 | 31727 | 3038.36 | parametric_daqp | 3984.22 | 0.763 | 1.000 | 3 |
| 5 | pop_070 | 703 | 55.00 | graph | 798.02 | 0.069 | 1.000 | 3 |
| 5 | pop_075 | 119 | 4.76 | graph | 5.17 | 0.921 | 0.991 | 3 |
| 5 | pop_083 | 13266 | 1361.84 | parametric_daqp | 1110.59 | 1.219 | 1.007 | 3 |
| 5 | pop_088 | 62318 | 6761.68 | parametric_daqp | 12281.69 | 0.551 | 1.005 | 3 |
| 5 | pop_096 | 258 | 39.08 | combinatorial_graph | 20.72 | 1.858 | 1.000 | 3 |
| 6 | masses3_N2 | 169 | 27.26 | parametric_daqp | 15.91 | 1.728 | 0.995 | 3 |
| 6 | masses3_N3 | 955 | 312.56 | parametric_daqp | 144.98 | 2.155 | 0.998 | 3 |
| 6 | masses3_N4 | 3643 | 2416.31 | parametric_daqp | 906.70 | 2.688 | 0.998 | 3 |
| 6 | masses3_N5 | 10437 | 21500.01 | graph | 4516.38 | 4.764 | 1.001 | 3 |
| 6 | masses3_N6 | 22145 | 58469.65 | graph | 15739.07 | 3.715 | 0.998 | 3 |
| 6 | masses3_N7 | 38330 | 184966.65 | graph | 54336.70 | 3.236 | 1.002 | 3 |
| 6 | pop_011 | 397 | 33.55 | graph | 25.62 | 1.310 | 0.998 | 3 |
| 6 | pop_017 | 4142 | 527.71 | parametric_daqp | 420.23 | 1.255 | 1.001 | 3 |
| 6 | pop_018 | 20800 | 2621.72 | parametric_daqp | 2863.50 | 0.916 | 1.002 | 3 |
| 6 | pop_025 | 5441 | 801.72 | parametric_daqp | 674.21 | 1.191 | 0.995 | 3 |
| 6 | pop_046 | 24 | 2.15 | graph | 1.15 | 1.874 | 1.002 | 3 |
| 6 | pop_049 | 9341 | 1356.43 | graph | 1207.84 | 1.123 | 0.996 | 3 |
| 6 | pop_053 | 2441 | 261.21 | graph | 213.54 | 1.223 | 0.996 | 3 |
| 6 | pop_081 | 4 | 0.16 | parametric_daqp | 0.08 | 1.902 | 0.994 | 3 |
| 6 | pop_084 | 665 | 49.15 | graph | 39.71 | 1.242 | 1.007 | 3 |
| 6 | rand_t6_x5_m20_s0 | 1774 | 169.92 | graph | 145.26 | 1.165 | 0.988 | 3 |
| 6 | rand_t6_x5_m20_s1 | 5062 | 571.53 | combinatorial_graph | 466.56 | 1.225 | 1.002 | 3 |
| 6 | rand_t6_x6_m24_s0 | 17134 | 2372.33 | combinatorial_graph | 2039.18 | 1.169 | 0.997 | 3 |
| 7 | pop_004 | 247 | 22.83 | parametric_daqp | 21.62 | 1.056 | 1.002 | 3 |
| 7 | pop_005 | 10289 | 4530.67 | graph | 2089.09 | 2.168 | 1.001 | 3 |
| 7 | pop_021 | 679 | 108.97 | combinatorial_graph | 67.66 | 1.647 | 0.989 | 3 |
| 7 | pop_030 | 286 | 24.16 | graph | 21.55 | 1.121 | 1.006 | 3 |
| 7 | pop_037 | 18615 | 3178.45 | parametric_daqp | 2332.59 | 1.369 | 0.997 | 3 |
| 7 | pop_054 | 20 | 1.64 | combinatorial_graph | 1.26 | 1.309 | 1.004 | 3 |
| 7 | pop_056 | 107 | 7.83 | parametric_daqp | 6.74 | 1.163 | 1.002 | 3 |
| 7 | pop_065 | 1089 | 163.38 | combinatorial | 126.98 | 1.288 | 0.996 | 3 |
| 7 | pop_068 | 86 | 6.99 | parametric_daqp | 5.95 | 1.176 | 0.994 | 3 |
| 7 | pop_073 | 23 | 2.19 | combinatorial_graph | 1.69 | 1.295 | 0.997 | 3 |
| 7 | pop_080 | 273 | 28.09 | graph | 23.08 | 1.212 | 1.002 | 3 |
| 7 | pop_085 | 121 | 9.32 | parametric_daqp | 8.10 | 1.150 | 1.001 | 3 |
| 7 | pop_086 | 3625 | 780.33 | combinatorial | 605.74 | 1.290 | 1.003 | 3 |
| 7 | pop_091 | 145018 | 40285.36 | parametric_daqp | 37170.18 | 1.084 | 0.990 | 3 |
| 7 | pop_097 | 20574 | 6293.06 | parametric_daqp | 4805.23 | 1.310 | 1.003 | 3 |
| 8 | masses4_N2 | 325 | 143.83 | parametric_daqp | 61.46 | 2.340 | 1.005 | 3 |
| 8 | masses4_N3 | 2891 | 3795.06 | parametric_daqp | 899.62 | 4.217 | 0.997 | 3 |
| 8 | masses4_N4 | 16050 | 70023.64 | parametric_daqp | 7880.98 | 8.862 | 1.008 | 3 |
| 8 | pop_007 | 12235 | 10515.23 | parametric_daqp | 3008.36 | 3.495 | 0.997 | 3 |
| 8 | pop_010 | 36 | 7.98 | parametric_daqp | 5.08 | 1.571 | 0.997 | 3 |
| 8 | pop_016 | 384 | 73.28 | combinatorial_graph | 56.91 | 1.286 | 1.000 | 3 |
| 8 | pop_027 | 1472 | 335.57 | combinatorial | 270.07 | 1.223 | 1.002 | 3 |
| 8 | pop_028 | 82116 | 28428.60 | parametric_daqp | 24304.96 | 1.171 | 0.978 | 3 |
| 8 | pop_032 | 178950 | 65515.58 | parametric_daqp | 54087.48 | 1.208 | 0.997 | 3 |
| 8 | pop_047 | 48144 | 13633.94 | parametric_daqp | 15547.83 | 0.874 | 0.979 | 3 |
| 8 | pop_061 | 33587 | 7903.38 | parametric_daqp | 6327.53 | 1.247 | 0.995 | 3 |
| 8 | pop_069 | 26 | 4.04 | combinatorial_graph | 3.25 | 1.240 | 1.002 | 3 |
| 8 | pop_076 | 15006 | 4285.88 | combinatorial | 3082.83 | 1.390 | 1.005 | 3 |
| 8 | pop_090 | 438 | 91.12 | combinatorial_graph | 70.91 | 1.284 | 1.000 | 3 |
| 8 | pop_098 | 269 | 50.33 | combinatorial_graph | 39.39 | 1.277 | 0.997 | 3 |
| 8 | rand_t8_x5_m15_s0 | 1035 | 129.33 | parametric_daqp | 110.93 | 1.163 | 1.001 | 3 |
| 8 | rand_t8_x5_m20_s0 | 4111 | 677.27 | graph | 560.41 | 1.209 | 1.013 | 3 |
| 8 | rand_t8_x5_m20_s1 | 8737 | 1844.51 | combinatorial_graph | 1325.43 | 1.393 | 1.007 | 3 |
| 8 | rand_t8_x6_m18_s0 | 9378 | 1583.32 | combinatorial_graph | 1166.76 | 1.359 | 1.002 | 3 |
| 8 | rand_t8_x6_m24_s0 | 32475 | 7611.73 | combinatorial_graph | 5611.01 | 1.358 | 0.997 | 3 |
| 10 | masses5_N2 | 370 | 379.25 | parametric_daqp | 130.96 | 2.906 | 1.000 | 3 |
| 10 | masses5_N3 | 5060 | 30658.38 | parametric_daqp | 3013.71 | 10.182 | 0.998 | 3 |
| 10 | rand_t10_x5_m15_s0 | 1859 | 421.51 | parametric_daqp | 347.22 | 1.212 | 1.011 | 3 |
| 10 | rand_t10_x5_m20_s0 | 6124 | 1681.02 | parametric_daqp | 1351.06 | 1.237 | 1.005 | 3 |
| 10 | rand_t10_x5_m20_s1 | 14848 | 4257.78 | combinatorial_graph | 3033.60 | 1.404 | 1.000 | 3 |
| 10 | rand_t10_x6_m18_s0 | 15312 | 3597.10 | parametric_daqp | 2924.19 | 1.229 | 1.000 | 3 |
| 10 | rand_t10_x6_m24_s0 | 71219 | 23719.90 | parametric_daqp | 18907.01 | 1.260 | 0.996 | 3 |
| 12 | masses6_N2 | 380 | 728.87 | parametric_daqp | 219.91 | 3.314 | 1.001 | 3 |
| 12 | rand_t12_x5_m15_s0 | 1012 | 323.51 | graph | 279.91 | 1.156 | 1.000 | 3 |
| 12 | rand_t12_x5_m20_s0 | 6143 | 3803.37 | parametric_daqp | 2107.62 | 1.805 | 1.003 | 3 |
| 14 | masses7_N2 | 390 | 1125.33 | parametric_daqp | 444.68 | 2.536 | 1.001 | 3 |
| 14 | rand_t14_x5_m15_s0 | 2470 | 1333.51 | graph | 1190.82 | 1.117 | 1.002 | 3 |
| 16 | masses8_N2 | 395 | 1592.38 | parametric_daqp | 729.54 | 2.183 | 1.005 | 3 |
| 16 | rand_t16_x5_m15_s0 | 1876 | 1410.53 | graph | 1226.04 | 1.145 | 1.007 | 3 |
