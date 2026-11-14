# Large benchmark (busy machine)

All timings were taken on a shared machine with other jobs running ("busy machine"): treat them as noisy, ratios within a row as more reliable than absolute values. `T/O` = per-solve cap reached.


## MPC: problems, solve times, correctness

| problem | theta | x | constraints | regions | combinatorial | geometric | graph | combinatorial_graph | ahmadi | mplcp | checks (g=gaps, o=overlaps, w=wrong law, 2000 samples) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| masses2_N8 | 4 | 8 | 80 | 2837 | - | 223.9 ms | 365.2 ms | 1.12 s | 443.5 ms | 214.6 ms | PL mismatches uniform 0, traj 16 |
| masses2_N10 | 4 | 10 | 100 | 6381 | - | 698.4 ms | 2.18 s | 7.33 s | 3.29 s | 1.09 s | ok |
| masses2_N12 | 4 | 12 | 120 | 12779 | - | 2.23 s | 20.74 s | 87.99 s | 6.27 s | 2.04 s | ok |
| masses3_N2 | 6 | 2 | 28 | 169 | 9.5 ms | 20.2 ms | 10.3 ms | 10.1 ms | 13.7 ms | 19.5 ms | PL mismatches uniform 0, traj 8 |
| masses3_N3 | 6 | 3 | 42 | 955 | - | 160.3 ms | 88.1 ms | 91.3 ms | 135.1 ms | 163.7 ms | PL mismatches uniform 0, traj 31 |
| masses3_N4 | 6 | 4 | 56 | 3638/3642/3643 | - | 1.29 s | 550.6 ms | 671.0 ms | 1.10 s | 965.4 ms | active sets differ: combinatorial_graph,mplcp; PL mismatches uniform 0, traj 9 |
| masses3_N5 | 6 | 5 | 70 | 10410/10438/10440 | - | 6.78 s | 2.52 s | 3.67 s | 16.46 s | 4.01 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| masses3_N6 | 6 | 6 | 84 | 22020/22138/22154/22168 | - | 26.53 s | 9.75 s | 28.99 s | T/O | 9.48 s | active sets differ: graph,combinatorial_graph,mplcp |
| masses3_N7 | 6 | 7 | 98 | 37937/38354/38397/38469 | - | 58.95 s | 30.39 s | 53.82 s | T/O | 23.80 s | active sets differ: graph,combinatorial_graph,mplcp |
| masses4_N2 | 8 | 2 | 34 | 325/327 | 44.5 ms | 121.7 ms | 51.8 ms | 47.9 ms | 66.6 ms | 94.6 ms | active sets differ: combinatorial_graph |
| masses4_N3 | 8 | 3 | 52 | 2888/2889/2890/2891/2953 | - | 2.62 s | 963.6 ms | 808.4 ms | 1.04 s | 1.64 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| masses4_N4 | 8 | 4 | 70 | 16021/16056/16057/16071/16317 | - | 46.75 s | 9.64 s | 12.91 s | 11.43 s | 17.65 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| masses4_N5 | 8 | 5 | 88 | - | - | T/O | T/O | T/O | T/O | T/O | ok |
| masses5_N2 | 10 | 2 | 36 | 369/373 | 112.6 ms | 367.5 ms | 136.3 ms | 117.0 ms | 135.8 ms | 216.0 ms | active sets differ: combinatorial_graph |
| masses5_N3 | 10 | 3 | 58 | 5059/5070/5074/5075/5267 | - | 23.25 s | 5.13 s | 4.71 s | 5.87 s | 11.66 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| masses5_N4 | 10 | 4 | 80 | 42696 | - | T/O | 109.54 s | T/O | T/O | T/O | samples: graph g0/o0/w2; verify 1/2000 |
| sys3x2_N7 | 3 | 14 | 66 | 2095 | - | 347.9 ms | 4.09 s | 13.42 s | 305.8 ms | 84.5 ms | ok |
| sys3x2_N9 | 3 | 18 | 74 | 3745 | - | 283.8 ms | 38.93 s | T/O | 724.8 ms | 200.3 ms | samples: geometric g0/o0/w1; graph g0/o0/w1; mplcp g0/o0/w1 |

## random: problems, solve times, correctness

| problem | theta | x | constraints | regions | combinatorial | geometric | graph | combinatorial_graph | ahmadi | mplcp | checks (g=gaps, o=overlaps, w=wrong law, 2000 samples) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| rand_t6_x5_m20_s0 | 6 | 5 | 24 | 1774 | 86.0 ms | 167.2 ms | 76.8 ms | 84.5 ms | 110.9 ms | 126.9 ms | ok |
| rand_t6_x5_m20_s1 | 6 | 5 | 20 | 5063 | 254.1 ms | 611.1 ms | 272.3 ms | 254.0 ms | 297.0 ms | 402.8 ms | ok |
| rand_t6_x6_m24_s0 | 6 | 6 | 24 | 17131/17133/17134/17135 | - | 4.36 s | 1.53 s | 1.67 s | 1.44 s | 1.72 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| rand_t8_x5_m15_s0 | 8 | 5 | 19 | 1035 | 65.3 ms | 180.4 ms | 176.2 ms | 204.7 ms | 223.8 ms | 299.7 ms | ok |
| rand_t8_x5_m20_s0 | 8 | 5 | 22 | 4111 | 309.9 ms | 744.2 ms | 321.4 ms | 321.8 ms | 385.2 ms | 572.0 ms | ok |
| rand_t8_x5_m20_s1 | 8 | 5 | 20 | 8736/8737 | - | 1.67 s | 792.9 ms | 657.8 ms | 727.4 ms | 1.34 s | active sets differ: graph,combinatorial_graph,ahmadi |
| rand_t8_x6_m18_s0 | 8 | 6 | 18 | 9377/9378 | - | 1.97 s | 724.0 ms | 644.4 ms | 737.7 ms | 1.26 s | active sets differ: graph,ahmadi |
| rand_t8_x6_m24_s0 | 8 | 6 | 24 | 32474/32476 | - | 27.20 s | 8.09 s | 5.14 s | 5.62 s | 6.53 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| rand_t10_x5_m15_s0 | 10 | 5 | 18 | 1859 | 229.9 ms | 671.4 ms | 345.2 ms | 342.3 ms | 366.8 ms | 524.4 ms | ok |
| rand_t10_x5_m20_s0 | 10 | 5 | 23 | 6124 | - | 2.34 s | 912.2 ms | 932.8 ms | 1.72 s | 2.54 s | ok |
| rand_t10_x5_m20_s1 | 10 | 5 | 20 | 14843/14849 | - | 6.76 s | 2.36 s | 1.93 s | 2.00 s | 4.14 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| rand_t10_x6_m18_s0 | 10 | 6 | 22 | 15312/15313 | - | 6.02 s | 2.05 s | 2.18 s | 2.25 s | 3.62 s | active sets differ: graph,combinatorial_graph,ahmadi |
| rand_t10_x6_m24_s0 | 10 | 6 | 28 | 71216/71219 | - | T/O | 19.56 s | 18.13 s | 14.59 s | 31.63 s | active sets differ: combinatorial_graph,mplcp |

## POP: problems, solve times, correctness

| problem | theta | x | constraints | regions | combinatorial | geometric | graph | combinatorial_graph | ahmadi | mplcp | checks (g=gaps, o=overlaps, w=wrong law, 2000 samples) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| pop_001 | 5 | 7 | 26 | 0/4624/4696/4708 | - | 0.0 ms | 2.13 s | 2.11 s | 3.73 s | 0.0 ms | active sets differ: graph,combinatorial_graph,ahmadi; samples: geometric g940/o0/w0; graph g0/o609/w2; combinatorial_graph g0/o609/w2; ahmadi g0/o609/w2; mplcp g940/o0/w0; verify 935/2000 |
| pop_002 | 7 | 8 | 77 | - | - | T/O | T/O | T/O | T/O | T/O | ok |
| pop_003 | 1 | 6 | 31 | 29 | 15.5 ms | 0.1 ms | 0.5 ms | 0.6 ms | 0.9 ms | 0.1 ms | ok |
| pop_004 | 7 | 4 | 18 | 247 | 11.6 ms | 195.2 ms | 179.0 ms | 119.9 ms | 154.0 ms | 152.2 ms | PL mismatches uniform 0, traj 1 |
| pop_005 | 7 | 6 | 48 | 10258/10287/10289/10540 | - | 7.78 s | 2.85 s | 5.35 s | 3.15 s | 2.68 s | active sets differ: graph,combinatorial_graph,mplcp; samples: combinatorial_graph g0/o0/w3 |
| pop_006 | 3 | 8 | 30 | 2373/2375 | 1.10 s | 92.9 ms | 162.4 ms | 160.7 ms | 98.9 ms | 50.0 ms | active sets differ: mplcp; samples: combinatorial g0/o0/w1; geometric g0/o0/w1; graph g0/o0/w1; combinatorial_graph g0/o0/w1; mplcp g0/o0/w1 |
| pop_007 | 8 | 4 | 46 | 12231/12234/12293/12328 | - | 3.51 s | 1.76 s | 1.77 s | 2.45 s | 6.60 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp; samples: geometric g0/o0/w1; graph g0/o0/w1; combinatorial_graph g0/o0/w1; ahmadi g0/o0/w1; mplcp g0/o0/w1 |
| pop_008 | 5 | 6 | 59 | 21293/21295 | - | 7.53 s | 1.54 s | 4.04 s | 4.68 s | 2.93 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| pop_009 | 4 | 2 | 18 | 63 | 0.8 ms | 1.6 ms | 0.9 ms | 0.9 ms | 1.3 ms | 1.6 ms | ok |
| pop_010 | 8 | 1 | 37 | 36/37 | 2.4 ms | 6.9 ms | 2.8 ms | 2.5 ms | 3.9 ms | 10.4 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp; samples: combinatorial g0/o33/w0; graph g0/o33/w0; combinatorial_graph g0/o33/w0; ahmadi g0/o33/w0 |
| pop_011 | 6 | 4 | 26 | 392/396/408/411 | 15.1 ms | 26.5 ms | 14.0 ms | 19.8 ms | 24.4 ms | 19.5 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp; samples: combinatorial g0/o10/w0; geometric g7/o0/w0; graph g0/o10/w0; combinatorial_graph g0/o10/w0; ahmadi g0/o10/w0 |
| pop_012 | 5 | 4 | 26 | 1047 | 38.0 ms | 59.0 ms | 32.4 ms | 38.7 ms | 52.5 ms | 48.1 ms | ok |
| pop_013 | 3 | 7 | 20 | 391 | 27.8 ms | 7.5 ms | 5.3 ms | 9.0 ms | 8.0 ms | 4.2 ms | ok |
| pop_014 | 4 | 2 | 14 | 19 | 0.2 ms | 0.3 ms | 0.2 ms | 0.2 ms | 0.3 ms | 0.3 ms | ok |
| pop_015 | 3 | 8 | 62 | 243 | - | 8.0 ms | 5.7 ms | 10.3 ms | 12.3 ms | 6.7 ms | ok |
| pop_016 | 8 | 2 | 29 | 384 | 30.6 ms | 69.2 ms | 33.1 ms | 31.3 ms | 39.1 ms | 80.9 ms | ok |
| pop_017 | 6 | 4 | 29 | 4139/4142/4143 | 221.5 ms | 802.1 ms | 425.4 ms | 394.0 ms | 548.9 ms | 608.8 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp |
| pop_018 | 6 | 7 | 28 | 20799/20800 | - | 5.62 s | 2.27 s | 2.35 s | 2.15 s | 1.96 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| pop_019 | 4 | 8 | 60 | 0/9634/9654 | - | 0.0 ms | 473.2 ms | 7.00 s | T/O | 0.0 ms | active sets differ: graph,combinatorial_graph; samples: geometric g1996/o0/w0; graph g0/o1542/w0; combinatorial_graph g0/o1542/w0; mplcp g1996/o0/w0; verify 1998/2000 |
| pop_020 | 1 | 3 | 12 | 5 | 0.1 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | ok |
| pop_021 | 7 | 3 | 20 | 679 | 37.3 ms | 130.6 ms | 67.5 ms | 49.6 ms | 59.9 ms | 125.1 ms | ok |
| pop_022 | 2 | 4 | 19 | 2 | 0.2 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | ok |
| pop_023 | 4 | 6 | 32 | 4729 | 419.4 ms | 335.7 ms | 178.0 ms | 231.8 ms | 228.4 ms | 176.7 ms | ok |
| pop_024 | 6 | 8 | 46 | 428841 | - | T/O | T/O | T/O | T/O | 63.18 s | samples: mplcp g2/o0/w0 |
| pop_025 | 6 | 4 | 31 | 5440/5441 | 334.7 ms | 736.2 ms | 320.9 ms | 677.7 ms | 1.19 s | 1.48 s | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp |
| pop_026 | 1 | 6 | 31 | 16 | 7.2 ms | 0.1 ms | 0.3 ms | 0.2 ms | 0.5 ms | 0.1 ms | ok |
| pop_027 | 8 | 3 | 34 | 1471/1472 | 142.0 ms | 290.0 ms | 145.1 ms | 146.4 ms | 422.5 ms | 341.7 ms | active sets differ: graph |
| pop_028 | 8 | 5 | 54 | 82116 | - | T/O | 27.70 s | 24.43 s | 22.14 s | 32.70 s | active sets differ: combinatorial_graph |
| pop_029 | 2 | 6 | 35 | 24/27/30 | 12.2 ms | 0.3 ms | 0.3 ms | 0.7 ms | 0.5 ms | 0.2 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp; samples: combinatorial g0/o37/w0; geometric g37/o0/w0; graph g0/o37/w0; combinatorial_graph g0/o37/w0; ahmadi g0/o37/w0 |
| pop_030 | 7 | 8 | 37 | 286 | 1.11 s | 51.2 ms | 30.7 ms | 54.1 ms | 45.5 ms | 34.0 ms | ok |
| pop_031 | 2 | 1 | 9 | 0/6 | 0.0 ms | 0.1 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.1 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi; samples: combinatorial g0/o625/w0; geometric g1973/o0/w0; graph g0/o625/w0; combinatorial_graph g0/o625/w0; ahmadi g0/o625/w0; mplcp g1973/o0/w0; PL mismatches uniform 31367, traj 31864 |
| pop_032 | 8 | 6 | 42 | 178943/178945/178950/178965 | - | T/O | 84.36 s | 34.05 s | 41.81 s | 80.08 s | active sets differ: combinatorial_graph,ahmadi,mplcp |
| pop_033 | 8 | 8 | 64 | - | - | T/O | T/O | T/O | T/O | T/O | ok |
| pop_034 | 1 | 5 | 18 | 8/9/10 | 1.4 ms | 0.0 ms | 0.1 ms | 0.1 ms | 0.1 ms | 0.0 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp |
| pop_035 | 3 | 5 | 21 | 77 | 2.3 ms | 1.3 ms | 0.7 ms | 0.9 ms | 1.1 ms | 0.8 ms | ok |
| pop_036 | 8 | 8 | 48 | - | - | T/O | T/O | T/O | T/O | T/O | ok |
| pop_037 | 7 | 5 | 24 | 18613/18615 | - | 5.68 s | 1.64 s | 1.35 s | 1.60 s | 2.61 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| pop_038 | 1 | 7 | 27 | 7 | 38.6 ms | 0.0 ms | 0.1 ms | 0.0 ms | 0.1 ms | 0.0 ms | ok |
| pop_039 | 1 | 1 | 4 | 1 | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | ok |
| pop_040 | 5 | 7 | 49 | 1996/2006/2058/2060 | - | 205.5 ms | 159.9 ms | 293.5 ms | 1.13 s | 275.9 ms | active sets differ: graph,combinatorial_graph,ahmadi,mplcp; samples: geometric g2/o0/w0; graph g0/o2/w0; combinatorial_graph g0/o2/w0; ahmadi g0/o2/w0 |
| pop_041 | 3 | 6 | 27 | 522/523 | 73.8 ms | 16.1 ms | 9.3 ms | 23.0 ms | 18.5 ms | 9.2 ms | active sets differ: mplcp; samples: mplcp g2/o0/w0 |
| pop_042 | 1 | 1 | 6 | 2 | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | ok |
| pop_043 | 4 | 4 | 26 | 899 | 20.1 ms | 37.6 ms | 17.6 ms | 23.5 ms | 35.9 ms | 31.0 ms | ok |
| pop_044 | 1 | 6 | 25 | 10 | 9.8 ms | 0.0 ms | 0.1 ms | 0.1 ms | 0.1 ms | 0.0 ms | ok |
| pop_045 | 3 | 4 | 31 | 55/61/62 | 3.1 ms | 0.9 ms | 0.7 ms | 1.4 ms | 1.4 ms | 1.1 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp; samples: combinatorial g0/o10/w0; geometric g10/o0/w0; graph g0/o6/w0; combinatorial_graph g0/o10/w0; ahmadi g0/o10/w0 |
| pop_046 | 6 | 3 | 11 | 19/24/33 | 0.6 ms | 0.9 ms | 0.6 ms | 0.7 ms | 0.7 ms | 1.0 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp; samples: combinatorial g0/o443/w0; geometric g417/o0/w0; graph g0/o443/w0; combinatorial_graph g0/o443/w0; ahmadi g0/o443/w0 |
| pop_047 | 8 | 7 | 41 | 48025/48137/48143/48144/48152 | - | 43.71 s | 10.24 s | 10.21 s | 24.01 s | 13.05 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| pop_048 | 4 | 3 | 22 | 46/56/57 | 1.1 ms | 1.3 ms | 1.0 ms | 1.0 ms | 2.4 ms | 1.4 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp |
| pop_049 | 6 | 5 | 39 | 9340/9341 | - | 1.68 s | 663.8 ms | 907.1 ms | 1.16 s | 1.11 s | active sets differ: graph |
| pop_050 | 2 | 2 | 9 | 18 | 0.0 ms | 0.1 ms | 0.1 ms | 0.1 ms | 0.1 ms | 0.1 ms | ok |
| pop_051 | 7 | 7 | 56 | - | - | T/O | T/O | T/O | T/O | T/O | ok |
| pop_052 | 1 | 1 | 5 | 0/4 | 0.0 ms | 0.1 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.1 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi; samples: combinatorial g0/o1814/w0; geometric g2000/o0/w0; graph g0/o1814/w0; combinatorial_graph g0/o1814/w0; ahmadi g0/o1814/w0; mplcp g2000/o0/w0 |
| pop_053 | 6 | 5 | 33 | 2441 | 150.8 ms | 262.8 ms | 137.6 ms | 244.9 ms | 512.1 ms | 518.5 ms | ok |
| pop_054 | 7 | 1 | 21 | 20 | 0.5 ms | 1.3 ms | 0.7 ms | 0.6 ms | 0.8 ms | 1.6 ms | ok |
| pop_055 | 2 | 3 | 12 | 7 | 0.1 ms | 0.0 ms | 0.1 ms | 0.0 ms | 0.0 ms | 0.0 ms | ok |
| pop_056 | 7 | 3 | 16 | 107 | 3.6 ms | 7.6 ms | 3.9 ms | 3.7 ms | 4.7 ms | 5.6 ms | ok |
| pop_057 | 5 | 2 | 15 | 34 | 0.5 ms | 1.1 ms | 0.6 ms | 0.6 ms | 0.9 ms | 0.9 ms | ok |
| pop_058 | 2 | 2 | 9 | 8 | 0.0 ms | 0.0 ms | 0.1 ms | 0.0 ms | 0.0 ms | 0.0 ms | ok |
| pop_059 | 4 | 2 | 16 | 51/53/54 | 0.5 ms | 1.0 ms | 0.6 ms | 0.5 ms | 0.8 ms | 1.1 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp |
| pop_060 | 3 | 8 | 42 | 2625/2627 | - | 119.5 ms | 131.4 ms | 192.6 ms | 138.5 ms | 89.8 ms | active sets differ: mplcp |
| pop_061 | 8 | 5 | 38 | 33568/33586/33587 | - | 23.07 s | 7.11 s | 7.33 s | 7.87 s | 13.59 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| pop_062 | 5 | 8 | 30 | 31726/31727 | - | 17.86 s | 3.01 s | 6.45 s | 5.35 s | 3.27 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| pop_063 | 1 | 6 | 33 | 17 | 14.5 ms | 0.1 ms | 0.2 ms | 0.3 ms | 0.4 ms | 0.1 ms | ok |
| pop_064 | 1 | 2 | 10 | 2 | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | ok |
| pop_065 | 7 | 3 | 30 | 1089 | 68.1 ms | 269.0 ms | 120.8 ms | 144.5 ms | 193.7 ms | 323.6 ms | ok |
| pop_066 | 2 | 6 | 26 | 93 | 2.9 ms | 1.3 ms | 1.0 ms | 1.7 ms | 1.6 ms | 0.6 ms | ok |
| pop_067 | 1 | 2 | 8 | 5 | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | ok |
| pop_068 | 7 | 3 | 14 | 86 | 3.1 ms | 10.0 ms | 3.5 ms | 3.6 ms | 4.4 ms | 5.3 ms | ok |
| pop_069 | 8 | 1 | 26 | 26 | 1.6 ms | 3.8 ms | 1.9 ms | 1.8 ms | 2.3 ms | 4.3 ms | ok |
| pop_070 | 5 | 4 | 37 | 701/703 | 39.1 ms | 53.7 ms | 30.6 ms | 48.4 ms | 56.3 ms | 53.4 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp |
| pop_071 | 2 | 2 | 10 | 0/7 | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi; samples: combinatorial g0/o1588/w0; geometric g2000/o0/w0; graph g0/o1588/w0; combinatorial_graph g0/o1588/w0; ahmadi g0/o1588/w0; mplcp g2000/o0/w0 |
| pop_072 | 1 | 4 | 16 | 9 | 0.2 ms | 0.0 ms | 0.1 ms | 0.0 ms | 0.1 ms | 0.0 ms | ok |
| pop_073 | 7 | 1 | 23 | 23 | 0.7 ms | 1.7 ms | 0.9 ms | 0.8 ms | 1.1 ms | 2.3 ms | ok |
| pop_074 | 3 | 4 | 18 | 79 | 1.3 ms | 1.0 ms | 0.6 ms | 1.0 ms | 1.3 ms | 0.8 ms | ok |
| pop_075 | 5 | 5 | 19 | 119 | 4.3 ms | 4.1 ms | 2.2 ms | 3.1 ms | 3.4 ms | 2.5 ms | ok |
| pop_076 | 8 | 5 | 34 | 13473/15007/19903/19907 | - | 5.47 s | 2.51 s | 2.01 s | 3.17 s | 6.11 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp; samples: geometric g314/o0/w0; graph g0/o725/w0; combinatorial_graph g0/o725/w0; ahmadi g0/o725/w0 |
| pop_077 | 4 | 7 | 40 | 3914/3915 | 336.8 ms | 484.3 ms | 175.4 ms | 271.8 ms | 287.4 ms | 194.4 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp; samples: geometric g0/o0/w1; ahmadi g0/o0/w1; mplcp g0/o0/w1 |
| pop_078 | 3 | 8 | 30 | 705/706 | 238.2 ms | 25.3 ms | 28.6 ms | 92.7 ms | 41.0 ms | 11.8 ms | active sets differ: graph |
| pop_079 | 3 | 8 | 61 | 126 | - | 4.2 ms | 5.4 ms | 9.9 ms | 11.0 ms | 3.4 ms | ok |
| pop_080 | 7 | 3 | 25 | 273 | 12.2 ms | 22.4 ms | 12.0 ms | 12.3 ms | 18.6 ms | 22.4 ms | ok |
| pop_081 | 6 | 1 | 5 | 2/4/6 | 0.0 ms | 0.1 ms | 0.1 ms | 0.0 ms | 0.0 ms | 0.0 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp; samples: combinatorial g0/o1556/w0; graph g0/o1556/w0; combinatorial_graph g0/o1556/w0; ahmadi g0/o1556/w0; mplcp g1556/o0/w0 |
| pop_082 | 2 | 6 | 30 | 76 | 9.4 ms | 0.9 ms | 0.6 ms | 0.9 ms | 1.1 ms | 0.5 ms | ok |
| pop_083 | 5 | 5 | 30 | 13265/13266 | - | 3.97 s | 1.15 s | 959.4 ms | 2.30 s | 2.30 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| pop_084 | 6 | 5 | 24 | 665 | 26.8 ms | 81.0 ms | 49.2 ms | 40.4 ms | 43.2 ms | 56.9 ms | ok |
| pop_085 | 7 | 3 | 16 | 121 | 4.1 ms | 11.1 ms | 4.8 ms | 4.6 ms | 5.8 ms | 7.0 ms | ok |
| pop_086 | 7 | 3 | 40 | 3625 | - | 1.18 s | 645.7 ms | 638.5 ms | 740.2 ms | 1.32 s | ok |
| pop_087 | 2 | 7 | 29 | 128/148/172 | 84.5 ms | 2.9 ms | 4.1 ms | 8.3 ms | 43.7 ms | 1.3 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp |
| pop_088 | 5 | 8 | 39 | 62314/62316/62318 | - | 39.45 s | 15.91 s | 16.77 s | 50.92 s | 6.66 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| pop_089 | 1 | 2 | 9 | 3 | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | ok |
| pop_090 | 8 | 2 | 37 | 438 | 36.3 ms | 105.5 ms | 54.7 ms | 56.5 ms | 76.9 ms | 152.3 ms | ok |
| pop_091 | 7 | 6 | 42 | 145019/145020/145022 | - | T/O | 27.72 s | 29.41 s | 53.75 s | 35.66 s | active sets differ: combinatorial_graph,mplcp |
| pop_092 | 3 | 3 | 9 | 12 | 0.1 ms | 0.1 ms | 0.1 ms | 0.1 ms | 0.1 ms | 0.1 ms | ok |
| pop_093 | 3 | 1 | 9 | 7/8 | 0.0 ms | 0.1 ms | 0.1 ms | 0.0 ms | 0.0 ms | 0.1 ms | active sets differ: combinatorial,graph,combinatorial_graph,ahmadi,mplcp; samples: combinatorial g0/o33/w0; graph g0/o33/w0; combinatorial_graph g0/o33/w0; ahmadi g0/o33/w0 |
| pop_094 | 2 | 2 | 10 | 10 | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.1 ms | 0.0 ms | ok |
| pop_095 | 8 | 8 | 48 | - | - | T/O | T/O | T/O | T/O | T/O | ok |
| pop_096 | 5 | 5 | 20 | 257/258 | 10.5 ms | 9.4 ms | 6.8 ms | 8.9 ms | 9.0 ms | 6.8 ms | active sets differ: mplcp |
| pop_097 | 7 | 4 | 44 | 20570/20571/20574/20582 | - | 14.48 s | 5.21 s | 5.15 s | 10.92 s | 15.02 s | active sets differ: graph,combinatorial_graph,ahmadi,mplcp |
| pop_098 | 8 | 2 | 32 | 269 | 21.0 ms | 48.6 ms | 22.5 ms | 22.5 ms | 32.2 ms | 54.5 ms | ok |
| pop_099 | 3 | 7 | 27 | 458 | 96.2 ms | 13.5 ms | 10.7 ms | 17.0 ms | 13.8 ms | 7.2 ms | ok |
| pop_100 | 1 | 2 | 9 | 4 | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | 0.0 ms | ok |

## Point location, ns per query (uniform / closed-loop trajectory), busy machine

Trajectory mode: closed-loop simulation x+ = A x + B u with the explicit law (MPC), a random walk in theta (POP, random). 1000 trajectories of up to 100 steps. Slow methods are timed on a prefix of the queries. `-` = method not built.


### MPC

| problem | regions | sequential (PPOPT) | sequential (early exit) | binary search tree | multiway tree | descriptor function | hash table | YANN | linear machine |
|---|---|---|---|---|---|---|---|---|---|
| masses2_N8 | 2837 | 46990 / 41883 | 20498 / 5637 | 394 / 284 | 269 / 235 | 18421 / 7334 | 86 / 74 | 47058 / 47915 | - |
| masses2_N10 | 6381 | 146807 / 103417 | 51331 / 13135 | - | - | 47387 / 17865 | 91 / 86 | 105956 / 106685 | - |
| masses2_N12 | 12779 | 215507 / 187725 | 107423 / 25815 | - | - | - | - | 217474 / 213729 | - |
| masses3_N2 | 169 | 6949 / 6552 | 807 / 230 | 249 / 289 | 233 / 259 | 680 / 427 | 155 / 113 | 7293 / 7559 | - |
| masses3_N3 | 955 | 35944 / 37425 | 6670 / 1077 | 664 / 435 | 532 / 367 | 4788 / 2406 | 210 / 161 | 40728 / 34120 | - |
| masses3_N4 | 3643 | 123109 / 116770 | 28787 / 7238 | - | - | 21461 / 9683 | 253 / 183 | 119239 / 117546 | - |
| masses3_N5 | 10440 | 361670 / 350807 | 98694 / 32611 | - | - | - | - | 330991 / 438370 | - |
| masses3_N6 | 22168 | 715465 / 673315 | 244325 / 94358 | - | - | - | - | 738567 / 798498 | - |
| masses3_N7 | 38469 | 1321778 / 1168301 | 415427 / 130926 | - | - | - | - | 1635677 / 2040890 | - |
| masses4_N2 | 327 | 14175 / 11976 | 1016 / 241 | 958 / 607 | 714 / 368 | 1148 / 593 | 282 / 94 | 14723 / 14733 | - |
| masses4_N3 | 2953 | 101445 / 96649 | 15545 / 967 | - | - | - | - | 128805 / 129143 | - |
| masses4_N4 | 16317 | 1240038 / 1188928 | 104533 / 7518 | - | - | - | - | 668061 / 675111 | - |
| masses5_N2 | 373 | 59711 / 54858 | 3314 / 259 | - | - | - | - | 57418 / 62586 | - |
| masses5_N3 | 5267 | 425028 / 477618 | 44910 / 2915 | - | - | - | - | 498322 / 427881 | - |
| masses5_N4 | 42696 | 18200323 / 14594736 | 4270846 / 481719 | - | - | - | - | 23494951 / 23146905 | - |
| sys3x2_N7 | 2095 | 30976 / 16969 | 8009 / 2385 | 188 / 158 | 155 / 135 | 7223 / 4101 | 47 / 32 | 21816 / 21986 | - |
| sys3x2_N9 | 3745 | 50560 / 33432 | 16941 / 854 | - | - | 16774 / 5237 | 53 / 40 | 42820 / 42591 | - |

### random

| problem | regions | sequential (PPOPT) | sequential (early exit) | binary search tree | multiway tree | descriptor function | hash table | YANN | linear machine |
|---|---|---|---|---|---|---|---|---|---|
| rand_t6_x5_m20_s0 | 1774 | 60954 / 55092 | 20036 / 3016 | 419 / 330 | 337 / 281 | 13523 / 4468 | 166 / 135 | 53150 / 53131 | - |
| rand_t6_x5_m20_s1 | 5063 | 175511 / 153587 | 102639 / 16114 | - | - | 53650 / 13901 | 189 / 174 | 151504 / 148222 | - |
| rand_t6_x6_m24_s0 | 17135 | 581195 / 525503 | 192464 / 75987 | - | - | 149797 / 57539 | 260 / 168 | 545437 / 584179 | - |
| rand_t8_x5_m15_s0 | 1035 | 90558 / 85279 | 25576 / 5748 | 1648 / 793 | 859 / 580 | 19836 / 7519 | 1867 / 495 | 102834 / 104356 | - |
| rand_t8_x5_m20_s0 | 4111 | 211449 / 205074 | 65430 / 5825 | - | - | - | - | 152559 / 147558 | - |
| rand_t8_x5_m20_s1 | 8737 | 241929 / 199639 | 105676 / 3262 | - | - | 82290 / 14132 | 785 / 396 | 240851 / 245190 | - |
| rand_t8_x6_m18_s0 | 9378 | 273805 / 254744 | 84470 / 24886 | - | - | 67174 / 30496 | 877 / 230 | 299224 / 294868 | - |
| rand_t8_x6_m24_s0 | 32476 | 1749596 / 1627908 | 651141 / 127301 | - | - | - | - | 1140914 / 1192888 | - |
| rand_t10_x5_m15_s0 | 1859 | 103742 / 97761 | 18727 / 2639 | - | - | - | - | 96892 / 95605 | - |
| rand_t10_x5_m20_s0 | 6124 | 383681 / 357130 | 77784 / 14378 | - | - | - | - | 362300 / 366118 | - |
| rand_t10_x5_m20_s1 | 14849 | 822046 / 696790 | 206476 / 37911 | - | - | - | - | 731868 / 738348 | - |
| rand_t10_x6_m18_s0 | 15313 | 834407 / 1062434 | 250090 / 86098 | - | - | - | - | 843245 / 871277 | - |
| rand_t10_x6_m24_s0 | 71219 | 3569365 / 3346059 | 1722666 / 222671 | - | - | - | - | 3647503 / 3582423 | - |

### POP

| problem | regions | sequential (PPOPT) | sequential (early exit) | binary search tree | multiway tree | descriptor function | hash table | YANN | linear machine |
|---|---|---|---|---|---|---|---|---|---|
| pop_003 | 29 | 128 / 68 | 25 / 17 | 31 / 19 | 35 / 29 | 48 / 57 | 7 / 7 | 745 / 90 | 31 / 31 |
| pop_004 | 247 | 111269 / 85331 | 16246 / 5205 | 1415 / 1417 | 890 / 860 | 3321 / 1878 | 761 / 166 | 22380 / 44073 | - |
| pop_005 | 10540 | 461749 / 420359 | 80280 / 8955 | - | - | 61884 / 25485 | 424 / 177 | 401642 / 401699 | - |
| pop_006 | 2375 | 21092 / 19757 | 1632 / 999 | 188 / 152 | 136 / 120 | 3868 / 3564 | 50 / 34 | 28694 / 27028 | - |
| pop_007 | 12328 | 398214 / 370404 | 94049 / 19832 | - | - | - | - | 470792 / 511472 | - |
| pop_008 | 21295 | 896705 / 788964 | 33218 / 21799 | - | - | - | - | 820993 / 525213 | - |
| pop_009 | 63 | 1156 / 1087 | 227 / 90 | 126 / 121 | 118 / 115 | 220 / 148 | 57 / 50 | 1047 / 1009 | - |
| pop_010 | 37 | 2177 / 3132 | 463 / 111 | - | - | - | - | 3320 / 2427 | - |
| pop_011 | 411 | 12638 / 12476 | 1139 / 762 | 429 / 368 | 341 / 296 | 1247 / 1116 | 131 / 89 | 13537 / 12873 | - |
| pop_012 | 1047 | 24825 / 25069 | 3352 / 788 | 329 / 236 | 264 / 206 | 3445 / 2013 | 142 / 94 | 24291 / 24148 | - |
| pop_013 | 391 | 3542 / 3266 | 1350 / 539 | 128 / 118 | 92 / 108 | 1177 / 745 | 28 / 29 | 4003 / 4017 | - |
| pop_014 | 19 | 334 / 320 | 52 / 42 | 101 / 90 | 81 / 74 | 67 / 60 | 37 / 29 | 299 / 322 | - |
| pop_015 | 243 | 2054 / 1971 | 486 / 380 | 136 / 113 | 107 / 99 | 552 / 527 | 34 / 28 | 2227 / 2201 | - |
| pop_016 | 384 | 15299 / 15359 | 1342 / 160 | 1183 / 587 | 696 / 400 | 1626 / 678 | 620 / 94 | 46200 / 31565 | - |
| pop_017 | 4143 | 212316 / 130521 | 33563 / 3525 | - | - | 35076 / 14880 | 279 / 210 | 131345 / 127370 | - |
| pop_018 | 20800 | 690717 / 693886 | 271821 / 36070 | - | - | - | - | 607144 / 602731 | - |
| pop_020 | 5 | 20 / 16 | 13 / 7 | 11 / 9 | 12 / 11 | 20 / 13 | 7 / 7 | 24 / 24 | 9 / 9 |
| pop_021 | 679 | 34921 / 32814 | 7115 / 1409 | 1417 / 745 | 1069 / 539 | 5054 / 2007 | 510 / 255 | 33888 / 32189 | - |
| pop_022 | 2 | 16 / 16 | 6 / 6 | 9 / 8 | 10 / 9 | 12 / 12 | 10 / 9 | 19 / 18 | 10 / 9 |
| pop_023 | 4729 | 76960 / 64836 | 17506 / 6844 | - | - | 18708 / 10490 | 83 / 62 | 72308 / 72152 | - |
| pop_024 | 428841 | 42067791 / 13724166 | 8869423 / 3287322 | - | - | - | - | 15597597 / 15085790 | - |
| pop_025 | 5441 | 612781 / 563920 | 147431 / 13132 | - | - | 151309 / 30635 | 369 / 181 | 393359 / 452661 | - |
| pop_026 | 16 | 42 / 41 | 26 / 22 | 15 / 14 | 23 / 23 | 39 / 37 | 6 / 6 | 50 / 49 | 19 / 19 |
| pop_027 | 1472 | 61134 / 71335 | 13859 / 495 | - | - | - | - | 68683 / 113030 | - |
| pop_028 | 82116 | 2561511 / 2608003 | 164959 / 99855 | - | - | - | - | 3872814 / 3901590 | - |
| pop_029 | 30 | 124 / 118 | 30 / 21 | 43 / 30 | 49 / 44 | 50 / 44 | 13 / 12 | 154 / 151 | 39 / 35 |
| pop_030 | 286 | 10063 / 10628 | 411 / 320 | 278 / 248 | 254 / 445 | 1455 / 1134 | 130 / 103 | 16897 / 18987 | - |
| pop_031 | 6 | 44 / 40 | 18 / 14 | 22 / 18 | 19 / 17 | 28 / 24 | 16 / 13 | 50 / 49 | 21 / 40 |
| pop_032 | 178965 | 5682390 / 4949752 | 4134752 / 742793 | - | - | - | - | 6882244 / 6935273 | - |
| pop_034 | 10 | 23 / 21 | 10 / 6 | 12 / 11 | 14 / 15 | 18 / 14 | 6 / 7 | 28 / 27 | 12 / 12 |
| pop_035 | 77 | 708 / 699 | 138 / 117 | 121 / 100 | 96 / 97 | 187 / 172 | 34 / 26 | 780 / 796 | - |
| pop_037 | 18615 | 1253794 / 1102051 | 341007 / 40371 | - | - | 229977 / 93664 | 799 / 401 | 1318081 / 1358185 | - |
| pop_038 | 7 | 24 / 19 | 15 / 7 | 21 / 11 | 20 / 13 | 24 / 16 | 7 / 7 | 33 / 31 | 12 / 11 |
| pop_039 | 1 | 7 / 7 | 4 / 4 | 5 / 5 | 5 / 6 | 9 / 9 | 6 / 7 | 9 / 9 | 6 / 6 |
| pop_040 | 2060 | 42323 / 39570 | 6244 / 1576 | 418 / 291 | 383 / 236 | 7441 / 4104 | 180 / 93 | 42740 / 42593 | - |
| pop_041 | 523 | 7677 / 8094 | 2023 / 1097 | 234 / 163 | 143 / 128 | 2305 / 1428 | 49 / 39 | 8277 / 8076 | - |
| pop_042 | 2 | 9 / 9 | 4 / 4 | 5 / 5 | 6 / 6 | 9 / 9 | 6 / 6 | 12 / 11 | 7 / 6 |
| pop_043 | 899 | 15722 / 14490 | 4794 / 1895 | 237 / 190 | 202 / 174 | 4138 / 2462 | 75 / 66 | 14455 / 14646 | - |
| pop_044 | 10 | 30 / 29 | 16 / 11 | 18 / 13 | 21 / 17 | 28 / 24 | 6 / 6 | 36 / 36 | 13 / 13 |
| pop_045 | 62 | 512 / 518 | 81 / 71 | 95 / 77 | 82 / 74 | 128 / 115 | 32 / 27 | 572 / 564 | - |
| pop_046 | 33 | 679 / 747 | 94 / 66 | 127 / 111 | 121 / 114 | 245 / 101 | 87 / 62 | 733 / 725 | - |
| pop_047 | 48152 | 2150160 / 2188183 | 354717 / 78164 | - | - | - | - | 3093775 / 2945754 | - |
| pop_048 | 57 | 827 / 783 | 210 / 70 | 101 / 92 | 94 / 87 | 236 / 130 | 45 / 45 | 769 / 771 | 110 / 114 |
| pop_049 | 9341 | 515039 / 485210 | 131750 / 32677 | - | - | 76225 / 25805 | 300 / 167 | 409054 / 468102 | - |
| pop_050 | 18 | 82 / 85 | 10 / 9 | 26 / 20 | 32 / 32 | 24 / 27 | 13 / 12 | 117 / 118 | 30 / 30 |
| pop_052 | 4 | 33 / 17 | 9 / 8 | 9 / 7 | 12 / 9 | 16 / 14 | 7 / 7 | 19 / 20 | 9 / 9 |
| pop_053 | 2441 | 130818 / 120988 | 40981 / 8282 | 483 / 314 | 438 / 302 | 16147 / 7881 | 2294 / 293 | 113440 / 106636 | - |
| pop_054 | 20 | 905 / 1139 | 182 / 121 | 170 / 137 | 143 / 134 | 164 / 154 | 158 / 105 | 1234 / 1155 | 147 / 141 |
| pop_055 | 7 | 46 / 42 | 26 / 17 | 38 / 21 | 33 / 24 | 32 / 21 | 22 / 13 | 50 / 49 | 23 / 18 |
| pop_056 | 107 | 4653 / 4536 | 454 / 230 | 393 / 265 | 270 / 230 | 454 / 351 | 207 / 119 | 4587 / 4678 | - |
| pop_057 | 34 | 998 / 1305 | 178 / 79 | 126 / 135 | 126 / 124 | 179 / 165 | 72 / 62 | 1228 / 1339 | - |
| pop_058 | 8 | 62 / 56 | 31 / 16 | 55 / 31 | 54 / 38 | 38 / 24 | 29 / 16 | 63 / 63 | - |
| pop_059 | 54 | 910 / 1190 | 145 / 55 | 136 / 141 | 113 / 124 | 174 / 193 | 66 / 46 | 1193 / 1209 | - |
| pop_060 | 2627 | 23689 / 27148 | 4567 / 2647 | 194 / 202 | 170 / 152 | 6243 / 8358 | 47 / 35 | 57811 / 57531 | - |
| pop_061 | 33587 | 1561256 / 1702405 | 72349 / 44757 | - | - | - | - | 2184404 / 2186439 | - |
| pop_062 | 31727 | 2107272 / 1858501 | 828999 / 148788 | - | - | - | - | 2015613 / 2089468 | - |
| pop_063 | 17 | 46 / 45 | 27 / 21 | 27 / 18 | 32 / 28 | 39 / 33 | 7 / 7 | 57 / 58 | 20 / 21 |
| pop_064 | 2 | 9 / 9 | 4 / 4 | 5 / 5 | 7 / 7 | 9 / 9 | 6 / 6 | 12 / 12 | 7 / 7 |
| pop_065 | 1089 | 55350 / 52791 | 6794 / 1657 | 688 / 505 | 452 / 362 | 5348 / 3003 | 278 / 175 | 53260 / 53990 | - |
| pop_066 | 93 | 475 / 719 | 82 / 65 | 85 / 69 | 69 / 60 | 154 / 142 | 23 / 17 | 989 / 829 | 122 / 135 |
| pop_067 | 5 | 27 / 17 | 16 / 9 | 21 / 11 | 20 / 12 | 23 / 15 | 7 / 8 | 25 / 26 | 9 / 9 |
| pop_068 | 86 | 5842 / 5830 | 1099 / 216 | 206 / 403 | 190 / 230 | 836 / 671 | 439 / 152 | 9241 / 10028 | - |
| pop_069 | 26 | 972 / 857 | 131 / 94 | 162 / 277 | 111 / 187 | 136 / 85 | 74 / 71 | 1596 / 969 | 60 / 88 |
| pop_070 | 703 | 25482 / 26299 | 3424 / 2142 | 497 / 259 | 536 / 247 | 3582 / 3073 | 98 / 76 | 26757 / 25879 | - |
| pop_072 | 9 | 30 / 27 | 20 / 13 | 22 / 14 | 22 / 17 | 29 / 21 | 7 / 8 | 34 / 33 | 14 / 14 |
| pop_073 | 23 | 1162 / 2415 | 759 / 450 | 225 / 276 | 209 / 205 | 281 / 163 | 524 / 124 | 1550 / 1745 | 200 / 145 |
| pop_074 | 79 | 748 / 1554 | 37 / 30 | 269 / 74 | 88 / 86 | 128 / 117 | 34 / 26 | 1672 / 1852 | - |
| pop_075 | 119 | 3245 / 4093 | 585 / 188 | 190 / 177 | 172 / 161 | 909 / 338 | 90 / 66 | 2461 / 2382 | - |
| pop_076 | 19907 | 500665 / 777775 | 122093 / 9052 | - | - | - | - | 1167078 / 1161508 | - |
| pop_077 | 3915 | 89837 / 86149 | 17145 / 9939 | - | - | 15524 / 14255 | 93 / 60 | 62345 / 99914 | - |
| pop_078 | 706 | 9211 / 9531 | 1761 / 242 | 155 / 140 | 180 / 111 | 2339 / 1348 | 42 / 36 | 10832 / 10411 | - |
| pop_079 | 126 | 1376 / 3088 | 658 / 531 | 292 / 104 | 156 / 90 | 720 / 436 | 38 / 30 | 3054 / 1924 | - |
| pop_080 | 273 | 12026 / 11664 | 3436 / 632 | 580 / 312 | 434 / 288 | 2130 / 1110 | 251 / 113 | 17320 / 18378 | - |
| pop_081 | 6 | 140 / 137 | 51 / 43 | 53 / 48 | 52 / 47 | 55 / 48 | 53 / 43 | 181 / 134 | 44 / 46 |
| pop_082 | 76 | 424 / 391 | 120 / 99 | 81 / 53 | 65 / 57 | 170 / 150 | 24 / 18 | 478 / 773 | - |
| pop_083 | 13266 | 313655 / 492443 | 142777 / 43465 | - | - | 130649 / 55693 | 236 / 150 | 401373 / 332864 | - |
| pop_084 | 665 | 33513 / 34948 | 2666 / 1706 | 572 / 427 | 577 / 355 | 2721 / 1954 | 149 / 110 | 20496 / 19168 | - |
| pop_085 | 121 | 6473 / 7411 | 1427 / 255 | 287 / 373 | 294 / 267 | 778 / 446 | 259 / 107 | 7366 / 8136 | - |
| pop_086 | 3625 | 186075 / 168642 | 50114 / 2809 | - | - | 32162 / 10075 | 609 / 419 | 200983 / 206847 | - |
| pop_087 | 172 | 614 / 986 | 118 / 83 | 77 / 59 | 75 / 67 | 348 / 186 | 20 / 15 | 1278 / 1276 | - |
| pop_088 | 62318 | 2878709 / 2257704 | 1473851 / 180935 | - | - | - | - | 2518647 / 1948928 | - |
| pop_089 | 3 | 11 / 11 | 8 / 8 | 9 / 8 | 9 / 8 | 14 / 13 | 6 / 6 | 16 / 16 | 7 / 7 |
| pop_090 | 438 | 14755 / 15987 | 1490 / 449 | 701 / 465 | 428 / 304 | 1530 / 1279 | 882 / 274 | 27736 / 27949 | - |
| pop_091 | 145022 | 10947363 / 5972724 | 4845166 / 1496566 | - | - | - | - | 6607995 / 6430502 | - |
| pop_092 | 12 | 138 / 132 | 40 / 28 | 49 / 41 | 65 / 53 | 51 / 40 | 27 / 49 | 137 / 133 | - |
| pop_093 | 8 | 83 / 89 | 42 / 33 | 72 / 46 | 55 / 45 | 40 / 32 | 28 / 22 | 88 / 83 | 33 / 26 |
| pop_094 | 10 | 55 / 52 | 23 / 9 | 38 / 23 | 36 / 27 | 34 / 23 | 15 / 11 | 66 / 64 | 25 / 21 |
| pop_096 | 258 | 5096 / 7580 | 660 / 210 | 199 / 190 | 177 / 172 | 968 / 756 | 82 / 50 | 8255 / 7882 | - |
| pop_097 | 20582 | 2131392 / 1032859 | 348808 / 63596 | - | - | - | - | 1096534 / 1079103 | - |
| pop_098 | 269 | 12883 / 9986 | 1033 / 329 | - | - | - | - | 11773 / 11732 | - |
| pop_099 | 458 | 3929 / 3670 | 538 / 262 | 145 / 123 | 127 / 117 | 800 / 660 | 37 / 28 | 4267 / 4306 | - |
| pop_100 | 4 | 13 / 14 | 4 / 4 | 7 / 8 | 8 / 12 | 11 / 10 | 6 / 6 | 19 / 18 | 7 / 8 |

Geometric mean over the POP problems (ns per query, uniform / trajectory):

| sequential (PPOPT) | sequential (early exit) | binary search tree | multiway tree | descriptor function | hash table | YANN | linear machine |
|---|---|---|---|---|---|---|---|
| 4499 / 4407 | 1028 / 416 | 93 / 73 | 82 / 68 | 433 / 300 | 67 / 41 | 5124 / 4966 | 21 / 21 |

## Point location storage (floats) and build time (ms), problems with the most regions

| problem | regions | sequential (PPOPT) | sequential (early exit) | binary search tree | multiway tree | descriptor function | hash table | YANN | linear machine |
|---|---|---|---|---|---|---|---|---|---|
| pop_024 | 428841 | 39388300 / 97 | 39388300 / 354 | - | - | - | - | 63832238 / 3328 | - |
| pop_032 | 178965 | 29782269 / 57 | 29782269 / 42 | - | - | - | - | 39624245 / 700 | - |
| pop_091 | 145022 | 18604280 / 48 | 18604280 / 37 | - | - | - | - | 25710359 / 586 | - |
| pop_028 | 82116 | 14440626 / 23 | 14440626 / 14 | - | - | - | - | 18217963 / 291 | - |
| rand_t10_x6_m24_s0 | 71219 | 18917943 / 31 | 18917943 / 26 | - | - | - | - | 23689617 / 457 | - |
| pop_088 | 62318 | 3868602 / 9 | 3868602 / 9 | - | - | - | - | 6921989 / 99 | - |
| pop_047 | 48152 | 7587945 / 18 | 7587945 / 10 | - | - | - | - | 10668714 / 151 | - |
| masses5_N4 | 42696 | 15955445 / 20 | 15955445 / 26 | - | - | - | - | 17876766 / 5196 | - |
| masses3_N7 | 38469 | 3720073 / 8 | 3720073 / 8 | - | - | - | - | 5616924 / 84 | - |
| pop_061 | 33587 | 5936571 / 16 | 5936571 / 8 | - | - | - | - | 7480700 / 129 | - |
| rand_t8_x6_m24_s0 | 32476 | 5230467 / 15 | 5230467 / 8 | - | - | - | - | 7016538 / 109 | - |
| pop_062 | 31727 | 1978920 / 10 | 1978920 / 4 | - | - | - | - | 3533495 / 45 | - |

## Not built / skipped

- masses3_N2: linear machine infeasible (176 multi-neighbour facets)
- masses3_N3: linear machine infeasible (1040 multi-neighbour facets)
- masses4_N2: linear machine infeasible (397 multi-neighbour facets)
- pop_004: linear machine infeasible (170 multi-neighbour facets)
- pop_009: linear machine infeasible (26 multi-neighbour facets)
- pop_011: linear machine infeasible (337 multi-neighbour facets)
- pop_012: linear machine infeasible (920 multi-neighbour facets)
- pop_013: linear machine infeasible (42 multi-neighbour facets)
- pop_014: linear machine infeasible (6 multi-neighbour facets)
- pop_015: linear machine infeasible (0 multi-neighbour facets)
- pop_016: linear machine infeasible (1593 multi-neighbour facets)
- pop_021: linear machine infeasible (2646 multi-neighbour facets)
- pop_030: linear machine infeasible (0 multi-neighbour facets)
- pop_035: linear machine infeasible (6 multi-neighbour facets)
- pop_041: linear machine infeasible (91 multi-neighbour facets)
- pop_043: linear machine infeasible (910 multi-neighbour facets)
- pop_045: linear machine infeasible (18 multi-neighbour facets)
- pop_046: linear machine infeasible (1 multi-neighbour facets)
- pop_056: linear machine infeasible (170 multi-neighbour facets)
- pop_057: linear machine infeasible (5 multi-neighbour facets)
- pop_058: linear machine infeasible (2 multi-neighbour facets)
- pop_059: linear machine infeasible (36 multi-neighbour facets)
- pop_065: linear machine infeasible (2962 multi-neighbour facets)
- pop_068: linear machine infeasible (71 multi-neighbour facets)
- pop_070: linear machine infeasible (1248 multi-neighbour facets)
- pop_074: linear machine infeasible (12 multi-neighbour facets)
- pop_075: linear machine infeasible (17 multi-neighbour facets)
- pop_078: linear machine infeasible (60 multi-neighbour facets)
- pop_079: linear machine infeasible (0 multi-neighbour facets)
- pop_080: linear machine infeasible (314 multi-neighbour facets)
- pop_082: linear machine infeasible (0 multi-neighbour facets)
- pop_084: linear machine infeasible (860 multi-neighbour facets)
- pop_085: linear machine infeasible (139 multi-neighbour facets)
- pop_087: linear machine infeasible (5 multi-neighbour facets)
- pop_090: linear machine infeasible (1197 multi-neighbour facets)
- pop_092: linear machine infeasible (1 multi-neighbour facets)
- pop_096: linear machine infeasible (140 multi-neighbour facets)
- pop_099: linear machine infeasible (87 multi-neighbour facets)
- rand_t8_x5_m15_s0: linear machine infeasible (997 multi-neighbour facets)

Timed-out solves: masses3_N6/ahmadi, masses3_N7/ahmadi, masses4_N5/geometric, masses4_N5/graph, masses4_N5/combinatorial_graph, masses4_N5/ahmadi, masses4_N5/mplcp, masses5_N4/geometric, masses5_N4/combinatorial_graph, masses5_N4/ahmadi, masses5_N4/mplcp, pop_002/geometric, pop_002/graph, pop_002/combinatorial_graph, pop_002/ahmadi, pop_002/mplcp, pop_019/ahmadi, pop_024/geometric, pop_024/graph, pop_024/combinatorial_graph, pop_024/ahmadi, pop_028/geometric, pop_032/geometric, pop_033/geometric, pop_033/graph, pop_033/combinatorial_graph, pop_033/ahmadi, pop_033/mplcp, pop_036/geometric, pop_036/graph, pop_036/combinatorial_graph, pop_036/ahmadi, pop_036/mplcp, pop_051/geometric, pop_051/graph, pop_051/combinatorial_graph, pop_051/ahmadi, pop_051/mplcp, pop_091/geometric, pop_095/geometric, pop_095/graph, pop_095/combinatorial_graph, pop_095/ahmadi, pop_095/mplcp, rand_t10_x6_m24_s0/geometric, sys3x2_N9/combinatorial_graph

## Summary by algorithm

| algorithm | solved | timed out | geometric-mean time vs graph (common problems) |
|---|---|---|---|
| combinatorial | 79 | 0 | 1.57 |
| geometric | 119 | 12 | 1.02 |
| graph | 124 | 7 | 1.00 |
| combinatorial_graph | 122 | 9 | 1.03 |
| ahmadi | 120 | 11 | 1.17 |
| mplcp | 124 | 7 | 0.76 |
