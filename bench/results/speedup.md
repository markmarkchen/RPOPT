### Same algorithm, single thread

| problem | algorithm | PPOPT | Rust (1 thread) | speedup | regions (PPOPT / Rust) |
|---|---|---|---|---|---|
| di_N10 | combinatorial | >300.00 s | 9.29 s | >32x | - / 11 |
| di_N10 | graph | 330.9 ms | 273 us | 1,214x | 11 / 11 |
| di_N10 | geometric | 112.7 ms | 167 us | 676x | 11 / 11 |
| di_N10 | combinatorial_graph | 296.8 ms | 379 us | 784x | 11 / 11 |
| di_N4 | combinatorial | 621.0 ms | 582 us | 1,068x | 9 / 9 |
| di_N4 | graph | 187.5 ms | 120 us | 1,561x | 9 / 9 |
| di_N4 | geometric | 89.6 ms | 73 us | 1,222x | 9 / 9 |
| di_N4 | combinatorial_graph | 112.0 ms | 119 us | 941x | 9 / 9 |
| di_N6 | combinatorial | 14.46 s | 13.1 ms | 1,103x | 11 / 11 |
| di_N6 | graph | 395.6 ms | 191 us | 2,072x | 11 / 11 |
| di_N6 | geometric | 181.6 ms | 119 us | 1,531x | 11 / 11 |
| di_N6 | combinatorial_graph | 600.5 ms | 270 us | 2,224x | 11 / 11 |
| di_N8 | combinatorial | >300.00 s | 362.8 ms | >827x | - / 11 |
| di_N8 | graph | 285.6 ms | 227 us | 1,257x | 11 / 11 |
| di_N8 | geometric | 126.5 ms | 135 us | 940x | 11 / 11 |
| di_N8 | combinatorial_graph | 267.4 ms | 318 us | 840x | 11 / 11 |
| di_doc_N3 | combinatorial | 174.6 ms | 276 us | 633x | 7 / 7 |
| di_doc_N3 | graph | 73.0 ms | 138 us | 530x | 7 / 7 |
| di_doc_N3 | geometric | 43.0 ms | 59 us | 728x | 7 / 7 |
| di_doc_N3 | combinatorial_graph | 100.5 ms | 95 us | 1,059x | 7 / 7 |
| masses2_N4 | combinatorial | 9.04 s | 25.0 ms | 362x | 307 / 307 |
| masses2_N4 | graph | 4.91 s | 14.8 ms | 333x | 307 / 307 |
| masses2_N4 | geometric | 5.06 s | 19.4 ms | 261x | 307 / 307 |
| masses2_N4 | combinatorial_graph | 5.58 s | 20.2 ms | 277x | 307 / 307 |
| masses2_N6 | combinatorial | >300.00 s | 452.2 ms | >663x | - / 901 |
| masses2_N6 | graph | 39.95 s | 71.5 ms | 558x | 901 / 901 |
| masses2_N6 | geometric | 24.30 s | 74.4 ms | 327x | 901 / 901 |
| masses2_N6 | combinatorial_graph | 46.96 s | 151.0 ms | 311x | 901 / 901 |
| sys3x2_N3 | combinatorial | 18.04 s | 27.2 ms | 662x | 211 / 211 |
| sys3x2_N3 | graph | 3.09 s | 4.7 ms | 661x | 211 / 211 |
| sys3x2_N3 | geometric | 2.19 s | 5.6 ms | 393x | 211 / 211 |
| sys3x2_N3 | combinatorial_graph | 3.43 s | 7.6 ms | 451x | 211 / 211 |
| sys3x2_N5 | combinatorial | >300.00 s | 10.03 s | >30x | - / 973 |
| sys3x2_N5 | graph | 77.54 s | 103.0 ms | 753x | 973 / 973 |
| sys3x2_N5 | geometric | 18.63 s | 41.7 ms | 447x | 973 / 973 |
| sys3x2_N5 | combinatorial_graph | 114.23 s | 342.3 ms | 334x | 973 / 973 |

### Parallel algorithms

| problem | algorithm | PPOPT (processes) | Rust (threads) | speedup | regions (PPOPT / Rust) |
|---|---|---|---|---|---|
| di_N10 | combinatorial_parallel | >300.00 s | 1.53 s | >197x | - / 11 |
| di_N4 | combinatorial_parallel | 1.18 s | 827 us | 1,424x | 9 / 9 |
| di_N6 | combinatorial_parallel | 4.72 s | 6.1 ms | 779x | 11 / 11 |
| di_N8 | combinatorial_parallel | 66.57 s | 99.2 ms | 671x | 11 / 11 |
| di_doc_N3 | combinatorial_parallel | 722.8 ms | 295 us | 2,451x | 7 / 7 |
| masses2_N4 | combinatorial_parallel | 6.76 s | 5.4 ms | 1,242x | 307 / 307 |
| masses2_N6 | combinatorial_parallel | 196.24 s | 69.2 ms | 2,837x | 901 / 901 |
| sys3x2_N3 | combinatorial_parallel | 12.74 s | 8.8 ms | 1,450x | 211 / 211 |
| sys3x2_N5 | combinatorial_parallel | >300.00 s | 1.50 s | >199x | - / 973 |

### Fastest serial algorithm per problem

| problem | fastest PPOPT | fastest Rust (1 thread) | speedup |
|---|---|---|---|
| di_N10 | geometric 112.7 ms | geometric 167 us | 676x |
| di_N4 | geometric 89.6 ms | geometric 73 us | 1,222x |
| di_N6 | geometric 181.6 ms | geometric 119 us | 1,531x |
| di_N8 | geometric 126.5 ms | geometric 135 us | 940x |
| di_doc_N3 | geometric 43.0 ms | geometric 59 us | 728x |
| masses2_N4 | graph 4.91 s | graph_exp 14.7 ms | 335x |
| masses2_N6 | geometric 24.30 s | graph_exp 68.5 ms | 355x |
| sys3x2_N3 | geometric 2.19 s | graph 4.7 ms | 470x |
| sys3x2_N5 | geometric 18.63 s | geometric 41.7 ms | 447x |

### Point location

| problem | PPOPT PointLocation (numba) | Rust sequential (PPOPT algorithm) | Rust fastest method |
|---|---|---|---|
| di_N10 | 5,723 ns | 75 ns | hash table 23 ns |
| di_N4 | 5,788 ns | 67 ns | hash table 23 ns |
| di_N6 | 6,088 ns | 79 ns | hash table 23 ns |
| di_N8 | 5,646 ns | 76 ns | hash table 22 ns |
| di_doc_N3 | 5,335 ns | 54 ns | hash table 22 ns |
| masses2_N4 | 23,215 ns | 4893 ns | hash table 67 ns |
| masses2_N6 | 63,909 ns | 14113 ns | hash table 83 ns |
| sys3x2_N3 | 17,911 ns | 1988 ns | hash table 39 ns |
| sys3x2_N5 | 68,451 ns | 9286 ns | hash table 51 ns |

Same algorithm, single thread: measured on 32 runs, min 261x (masses2_N4/geometric), median 728x, max 2,224x; 32 of 32 at 200x or more.
Same algorithm, single thread, PPOPT runs stopped at the time limit (lower bounds): di_N10/combinatorial >32x, di_N8/combinatorial >827x, masses2_N6/combinatorial >663x, sys3x2_N5/combinatorial >30x.
Parallel vs parallel: measured on 7 runs, min 671x (di_N8/combinatorial_parallel), median 1,424x, max 2,837x; 7 of 7 at 200x or more.
Parallel vs parallel, PPOPT runs stopped at the time limit (lower bounds): di_N10/combinatorial_parallel >197x, sys3x2_N5/combinatorial_parallel >199x.
Fastest serial PPOPT algorithm vs fastest serial Rust algorithm: 335x to 1,531x.
