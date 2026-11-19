size = theta/x/m: parameters, variables, constraints (rows of A x <= b + F theta that the solvers see after loading; `results/problem_sizes.md` lists every problem with the raw count and the regions).

### Same algorithm, single thread

| problem | size (theta/x/m) | algorithm | PPOPT | Rust (1 thread) | speedup | regions (PPOPT / Rust) |
|---|---|---|---|---|---|---|
| di_N10 | 2/10/42 | combinatorial | >120.00 s | 7.67 s | >16x | - / 11 |
| di_N10 | 2/10/42 | graph | 254.1 ms | 267 us | 951x | 11 / 11 |
| di_N10 | 2/10/42 | geometric | 85.9 ms | 139 us | 619x | 11 / 11 |
| di_N10 | 2/10/42 | combinatorial_graph | 223.1 ms | 360 us | 621x | 11 / 11 |
| di_N4 | 2/4/18 | combinatorial | 1.14 s | 479 us | 2,381x | 9 / 9 |
| di_N4 | 2/4/18 | graph | 187.1 ms | 111 us | 1,684x | 9 / 9 |
| di_N4 | 2/4/18 | geometric | 217.0 ms | 61 us | 3,543x | 9 / 9 |
| di_N4 | 2/4/18 | combinatorial_graph | 198.4 ms | 108 us | 1,832x | 9 / 9 |
| di_N6 | 2/6/26 | combinatorial | 27.60 s | 10.5 ms | 2,633x | 11 / 11 |
| di_N6 | 2/6/26 | graph | 330.4 ms | 186 us | 1,778x | 11 / 11 |
| di_N6 | 2/6/26 | geometric | 86.7 ms | 98 us | 887x | 11 / 11 |
| di_N6 | 2/6/26 | combinatorial_graph | 300.8 ms | 266 us | 1,133x | 11 / 11 |
| di_N8 | 2/8/34 | combinatorial | >120.00 s | 272.7 ms | >440x | - / 11 |
| di_N8 | 2/8/34 | graph | 204.9 ms | 223 us | 919x | 11 / 11 |
| di_N8 | 2/8/34 | geometric | 66.2 ms | 114 us | 582x | 11 / 11 |
| di_N8 | 2/8/34 | combinatorial_graph | 170.5 ms | 302 us | 564x | 11 / 11 |
| di_doc_N3 | 2/9/20 | combinatorial | 115.0 ms | 231 us | 499x | 7 / 7 |
| di_doc_N3 | 2/9/20 | graph | 51.7 ms | 141 us | 366x | 7 / 7 |
| di_doc_N3 | 2/9/20 | geometric | 26.6 ms | 52 us | 508x | 7 / 7 |
| di_doc_N3 | 2/9/20 | combinatorial_graph | 39.4 ms | 116 us | 340x | 7 / 7 |
| dinv_N4 | 2/4/16 | combinatorial | 340.9 ms | 399 us | 855x | 9 / 9 |
| dinv_N4 | 2/4/16 | graph | 94.8 ms | 128 us | 737x | 9 / 9 |
| dinv_N4 | 2/4/16 | geometric | 33.1 ms | 55 us | 603x | 9 / 9 |
| dinv_N4 | 2/4/16 | combinatorial_graph | 61.5 ms | 109 us | 563x | 9 / 9 |
| dinv_N6 | 2/6/24 | combinatorial | 8.35 s | 8.9 ms | 933x | 11 / 11 |
| dinv_N6 | 2/6/24 | graph | 184.4 ms | 251 us | 734x | 11 / 11 |
| dinv_N6 | 2/6/24 | geometric | 64.0 ms | 94 us | 684x | 11 / 11 |
| dinv_N6 | 2/6/24 | combinatorial_graph | 144.5 ms | 260 us | 557x | 11 / 11 |
| dinv_N8 | 2/8/32 | combinatorial | >120.00 s | 228.5 ms | >525x | - / 11 |
| dinv_N8 | 2/8/32 | graph | 209.7 ms | 269 us | 778x | 11 / 11 |
| dinv_N8 | 2/8/32 | geometric | 68.5 ms | 108 us | 634x | 11 / 11 |
| dinv_N8 | 2/8/32 | combinatorial_graph | 174.8 ms | 298 us | 586x | 11 / 11 |
| masses2_N4 | 4/4/40 | combinatorial | 7.76 s | 15.2 ms | 511x | 307 / 307 |
| masses2_N4 | 4/4/40 | graph | 4.24 s | 7.9 ms | 536x | 307 / 307 |
| masses2_N4 | 4/4/40 | geometric | 4.42 s | 11.2 ms | 393x | 307 / 307 |
| masses2_N4 | 4/4/40 | combinatorial_graph | 4.92 s | 13.8 ms | 357x | 307 / 307 |
| masses2_N6 | 4/6/60 | combinatorial | >120.00 s | 337.3 ms | >356x | - / 901 |
| masses2_N6 | 4/6/60 | graph | 28.00 s | 52.5 ms | 534x | 901 / 901 |
| masses2_N6 | 4/6/60 | geometric | 21.63 s | 39.4 ms | 550x | 901 / 901 |
| masses2_N6 | 4/6/60 | combinatorial_graph | 41.10 s | 136.4 ms | 301x | 901 / 901 |
| sys3x2_N3 | 3/6/30 | combinatorial | 14.11 s | 21.8 ms | 646x | 211 / 211 |
| sys3x2_N3 | 3/6/30 | graph | 2.31 s | 3.4 ms | 683x | 211 / 211 |
| sys3x2_N3 | 3/6/30 | geometric | 1.58 s | 2.8 ms | 574x | 211 / 211 |
| sys3x2_N3 | 3/6/30 | combinatorial_graph | 2.56 s | 6.3 ms | 408x | 211 / 211 |
| sys3x2_N5 | 3/10/50 | combinatorial | >120.00 s | 8.06 s | >15x | - / 973 |
| sys3x2_N5 | 3/10/50 | graph | 57.83 s | 103.7 ms | 558x | 973 / 973 |
| sys3x2_N5 | 3/10/50 | geometric | 15.50 s | 21.4 ms | 726x | 973 / 973 |
| sys3x2_N5 | 3/10/50 | combinatorial_graph | 85.40 s | 336.7 ms | 254x | 973 / 973 |

### Parallel algorithms

| problem | size (theta/x/m) | algorithm | PPOPT (processes) | Rust (threads) | speedup | regions (PPOPT / Rust) |
|---|---|---|---|---|---|---|
| di_N10 | 2/10/42 | combinatorial_parallel | >120.00 s | 1.62 s | >74x | - / 11 |
| di_N4 | 2/4/18 | combinatorial_parallel | 320.7 ms | 452 us | 709x | 9 / 9 |
| di_N6 | 2/6/26 | combinatorial_parallel | 5.36 s | 3.3 ms | 1,623x | 11 / 11 |
| di_N8 | 2/8/34 | combinatorial_parallel | 67.05 s | 60.0 ms | 1,118x | 11 / 11 |
| di_doc_N3 | 2/9/20 | combinatorial_parallel | 103.0 ms | 251 us | 410x | 7 / 7 |
| dinv_N4 | 2/4/16 | combinatorial_parallel | 158.2 ms | 420 us | 376x | 9 / 9 |
| dinv_N6 | 2/6/24 | combinatorial_parallel | 1.94 s | 3.2 ms | 613x | 11 / 11 |
| dinv_N8 | 2/8/32 | combinatorial_parallel | 49.65 s | 85.9 ms | 578x | 11 / 11 |
| masses2_N4 | 4/4/40 | combinatorial_parallel | 1.93 s | 4.3 ms | 452x | 307 / 307 |
| masses2_N6 | 4/6/60 | combinatorial_parallel | 66.99 s | 70.8 ms | 946x | 901 / 901 |
| sys3x2_N3 | 3/6/30 | combinatorial_parallel | 3.37 s | 5.9 ms | 570x | 211 / 211 |
| sys3x2_N5 | 3/10/50 | combinatorial_parallel | >120.00 s | 1.58 s | >76x | - / 973 |

### Fastest serial algorithm per problem

| problem | size (theta/x/m) | fastest PPOPT | fastest Rust (1 thread) | speedup |
|---|---|---|---|---|
| di_N10 | 2/10/42 | geometric 85.9 ms | mplcp 79 us | 1,091x |
| di_N4 | 2/4/18 | graph 187.1 ms | mplcp 36 us | 5,126x |
| di_N6 | 2/6/26 | geometric 86.7 ms | mplcp 54 us | 1,604x |
| di_N8 | 2/8/34 | geometric 66.2 ms | mplcp 65 us | 1,024x |
| di_doc_N3 | 2/9/20 | geometric 26.6 ms | mplcp 32 us | 838x |
| dinv_N4 | 2/4/16 | geometric 33.1 ms | mplcp 33 us | 1,009x |
| dinv_N6 | 2/6/24 | geometric 64.0 ms | mplcp 55 us | 1,173x |
| dinv_N8 | 2/8/32 | geometric 68.5 ms | mplcp 65 us | 1,050x |
| masses2_N4 | 4/4/40 | graph 4.24 s | graph_exp 7.7 ms | 553x |
| masses2_N6 | 4/6/60 | geometric 21.63 s | mplcp 32.7 ms | 662x |
| sys3x2_N3 | 3/6/30 | geometric 1.58 s | mplcp 1.7 ms | 939x |
| sys3x2_N5 | 3/10/50 | geometric 15.50 s | mplcp 11.4 ms | 1,358x |

### Point location

| problem | size (theta/x/m) | PPOPT PointLocation (numba) | Rust sequential (PPOPT algorithm) | Rust fastest method |
|---|---|---|---|---|
| di_N10 | 2/10/42 | 6,776 ns | 98 ns | hash table (limit 16x old16) 11 ns |
| di_N4 | 2/4/18 | 13,763 ns | 87 ns | hash table (limit 64 MB) 11 ns |
| di_N6 | 2/6/26 | 13,727 ns | 97 ns | hash table (limit 8 MB) 11 ns |
| di_N8 | 2/8/34 | 6,328 ns | 98 ns | hash table (limit 8 MB) 11 ns |
| di_doc_N3 | 2/9/20 | 6,939 ns | 70 ns | hash table (limit 1 MB) 11 ns |
| dinv_N4 | 2/4/16 | 6,791 ns | 77 ns | hash table (limit 1 MB) 10 ns |
| dinv_N6 | 2/6/24 | 6,974 ns | 96 ns | hash table (limit 64x old16) 11 ns |
| dinv_N8 | 2/8/32 | 6,826 ns | 100 ns | hash table (limit 16x old16) 11 ns |
| masses2_N4 | 4/4/40 | 22,426 ns | 7533 ns | hash table (limit 64 MB) 37 ns |
| masses2_N6 | 4/6/60 | 68,711 ns | 20593 ns | hash table (limit 16x old16) 39 ns |
| sys3x2_N3 | 3/6/30 | 16,799 ns | 3302 ns | hash table (limit 8 MB) 23 ns |
| sys3x2_N5 | 3/10/50 | 63,876 ns | 14712 ns | hash table (limit 64x old16) 23 ns |

Same algorithm, single thread: measured on 43 runs, min 254x (sys3x2_N5/combinatorial_graph), median 619x, max 3,543x; 43 of 43 at 200x or more.
Same algorithm, single thread, PPOPT runs stopped at the time limit (lower bounds): di_N10/combinatorial >16x, di_N8/combinatorial >440x, dinv_N8/combinatorial >525x, masses2_N6/combinatorial >356x, sys3x2_N5/combinatorial >15x.
Parallel vs parallel: measured on 10 runs, min 376x (dinv_N4/combinatorial_parallel), median 613x, max 1,623x; 10 of 10 at 200x or more.
Parallel vs parallel, PPOPT runs stopped at the time limit (lower bounds): di_N10/combinatorial_parallel >74x, sys3x2_N5/combinatorial_parallel >76x.
Fastest serial PPOPT algorithm vs fastest serial Rust algorithm: 553x to 5,126x.
