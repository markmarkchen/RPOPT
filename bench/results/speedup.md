### Same algorithm, single thread

| problem | algorithm | PPOPT | Rust (1 thread) | speedup | regions (PPOPT / Rust) |
|---|---|---|---|---|---|
| di_N10 | combinatorial | >14,400.00 s | 6.14 s | >2,344x | - / 11 |
| di_N10 | graph | 330.9 ms | 218 us | 1,516x | 11 / 11 |
| di_N10 | geometric | 112.7 ms | 113 us | 997x | 11 / 11 |
| di_N10 | combinatorial_graph | 296.8 ms | 286 us | 1,039x | 11 / 11 |
| di_N4 | combinatorial | 621.0 ms | 381 us | 1,629x | 9 / 9 |
| di_N4 | graph | 187.5 ms | 92 us | 2,048x | 9 / 9 |
| di_N4 | geometric | 89.6 ms | 51 us | 1,766x | 9 / 9 |
| di_N4 | combinatorial_graph | 112.0 ms | 89 us | 1,260x | 9 / 9 |
| di_N6 | combinatorial | 14.46 s | 8.7 ms | 1,656x | 11 / 11 |
| di_N6 | graph | 395.6 ms | 152 us | 2,608x | 11 / 11 |
| di_N6 | geometric | 181.6 ms | 81 us | 2,252x | 11 / 11 |
| di_N6 | combinatorial_graph | 600.5 ms | 218 us | 2,751x | 11 / 11 |
| di_N8 | combinatorial | >300.00 s | 226.6 ms | >1,324x | - / 11 |
| di_N8 | graph | 285.6 ms | 181 us | 1,577x | 11 / 11 |
| di_N8 | geometric | 126.5 ms | 92 us | 1,374x | 11 / 11 |
| di_N8 | combinatorial_graph | 267.4 ms | 250 us | 1,068x | 11 / 11 |
| di_doc_N3 | combinatorial | 174.6 ms | 191 us | 913x | 7 / 7 |
| di_doc_N3 | graph | 73.0 ms | 122 us | 600x | 7 / 7 |
| di_doc_N3 | geometric | 43.0 ms | 44 us | 977x | 7 / 7 |
| di_doc_N3 | combinatorial_graph | 100.5 ms | 97 us | 1,034x | 7 / 7 |
| masses2_N4 | combinatorial | 9.04 s | 12.8 ms | 705x | 307 / 307 |
| masses2_N4 | graph | 4.91 s | 6.6 ms | 740x | 307 / 307 |
| masses2_N4 | geometric | 5.06 s | 9.1 ms | 554x | 307 / 307 |
| masses2_N4 | combinatorial_graph | 5.58 s | 11.2 ms | 499x | 307 / 307 |
| masses2_N6 | combinatorial | >300.00 s | 286.4 ms | >1,048x | - / 901 |
| masses2_N6 | graph | 39.95 s | 43.9 ms | 909x | 901 / 901 |
| masses2_N6 | geometric | 24.30 s | 33.1 ms | 734x | 901 / 901 |
| masses2_N6 | combinatorial_graph | 46.96 s | 115.0 ms | 408x | 901 / 901 |
| sys3x2_N3 | combinatorial | 18.04 s | 18.0 ms | 1,003x | 211 / 211 |
| sys3x2_N3 | graph | 3.09 s | 2.8 ms | 1,090x | 211 / 211 |
| sys3x2_N3 | geometric | 2.19 s | 2.3 ms | 960x | 211 / 211 |
| sys3x2_N3 | combinatorial_graph | 3.43 s | 5.3 ms | 648x | 211 / 211 |
| sys3x2_N5 | combinatorial | >14,400.00 s | 6.54 s | >2,203x | - / 973 |
| sys3x2_N5 | graph | 77.54 s | 85.3 ms | 908x | 973 / 973 |
| sys3x2_N5 | geometric | 18.63 s | 17.6 ms | 1,061x | 973 / 973 |
| sys3x2_N5 | combinatorial_graph | 114.23 s | 270.9 ms | 422x | 973 / 973 |

### Parallel algorithms

| problem | algorithm | PPOPT (processes) | Rust (threads) | speedup | regions (PPOPT / Rust) |
|---|---|---|---|---|---|
| di_N10 | combinatorial_parallel | >300.00 s | 582.4 ms | >515x | - / 11 |
| di_N4 | combinatorial_parallel | 1.18 s | 662 us | 1,779x | 9 / 9 |
| di_N6 | combinatorial_parallel | 4.72 s | 3.4 ms | 1,387x | 11 / 11 |
| di_N8 | combinatorial_parallel | 66.57 s | 32.5 ms | 2,047x | 11 / 11 |
| di_doc_N3 | combinatorial_parallel | 722.8 ms | 307 us | 2,352x | 7 / 7 |
| masses2_N4 | combinatorial_parallel | 6.76 s | 3.3 ms | 2,063x | 307 / 307 |
| masses2_N6 | combinatorial_parallel | 196.24 s | 33.3 ms | 5,889x | 901 / 901 |
| sys3x2_N3 | combinatorial_parallel | 12.74 s | 4.9 ms | 2,621x | 211 / 211 |
| sys3x2_N5 | combinatorial_parallel | >300.00 s | 636.5 ms | >471x | - / 973 |

### Fastest serial algorithm per problem

| problem | fastest PPOPT | fastest Rust (1 thread) | speedup |
|---|---|---|---|
| di_N10 | geometric 112.7 ms | mplcp 65 us | 1,739x |
| di_N4 | geometric 89.6 ms | mplcp 30 us | 2,977x |
| di_N6 | geometric 181.6 ms | mplcp 45 us | 4,062x |
| di_N8 | geometric 126.5 ms | mplcp 54 us | 2,330x |
| di_doc_N3 | geometric 43.0 ms | mplcp 27 us | 1,606x |
| masses2_N4 | graph 4.91 s | graph_exp 6.5 ms | 753x |
| masses2_N6 | geometric 24.30 s | mplcp 27.5 ms | 884x |
| sys3x2_N3 | geometric 2.19 s | mplcp 1.4 ms | 1,522x |
| sys3x2_N5 | geometric 18.63 s | mplcp 9.7 ms | 1,920x |

### Point location

| problem | PPOPT PointLocation (numba) | Rust sequential (PPOPT algorithm) | Rust fastest method |
|---|---|---|---|
| di_N10 | 5,723 ns | 86 ns | hash table (limit 8 MB) 9 ns |
| di_N4 | 5,788 ns | 77 ns | hash table (limit 8 MB) 9 ns |
| di_N6 | 6,088 ns | 86 ns | hash table (limit 64 MB) 9 ns |
| di_N8 | 5,646 ns | 87 ns | hash table (limit 64 MB) 9 ns |
| di_doc_N3 | 5,335 ns | 60 ns | hash table (limit 64 MB) 9 ns |
| dinv_N4 | - | 68 ns | hash table (limit 64 MB) 8 ns |
| dinv_N6 | - | 87 ns | hash table (limit 64 MB) 9 ns |
| dinv_N8 | - | 87 ns | hash table (limit 8 MB) 9 ns |
| masses2_N4 | 23,215 ns | 5089 ns | hash table (limit 64 MB) 32 ns |
| masses2_N6 | 63,909 ns | 14314 ns | hash table (limit 16x old16) 33 ns |
| sys3x2_N3 | 17,911 ns | 2273 ns | hash table (limit 16x old16) 20 ns |
| sys3x2_N5 | 68,451 ns | 10180 ns | hash table (limit 64 MB) 19 ns |

Same algorithm, single thread: measured on 32 runs, min 408x (masses2_N6/combinatorial_graph), median 1,034x, max 2,751x; 32 of 32 at 200x or more.
Same algorithm, single thread, PPOPT runs stopped at the time limit (lower bounds): di_N10/combinatorial >2,344x, di_N8/combinatorial >1,324x, masses2_N6/combinatorial >1,048x, sys3x2_N5/combinatorial >2,203x.
Parallel vs parallel: measured on 7 runs, min 1,387x (di_N6/combinatorial_parallel), median 2,063x, max 5,889x; 7 of 7 at 200x or more.
Parallel vs parallel, PPOPT runs stopped at the time limit (lower bounds): di_N10/combinatorial_parallel >515x, sys3x2_N5/combinatorial_parallel >471x.
Fastest serial PPOPT algorithm vs fastest serial Rust algorithm: 753x to 4,062x.
