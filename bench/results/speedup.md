### Same algorithm, single thread

| problem | algorithm | PPOPT | Rust (1 thread) | speedup | regions (PPOPT / Rust) |
|---|---|---|---|---|---|
| di_N10 | combinatorial | >14,400.00 s | 6.01 s | >2,398x | - / 11 |
| di_N10 | graph | 330.9 ms | 220 us | 1,505x | 11 / 11 |
| di_N10 | geometric | 112.7 ms | 111 us | 1,014x | 11 / 11 |
| di_N10 | combinatorial_graph | 296.8 ms | 291 us | 1,020x | 11 / 11 |
| di_N4 | combinatorial | 621.0 ms | 374 us | 1,659x | 9 / 9 |
| di_N4 | graph | 187.5 ms | 92 us | 2,031x | 9 / 9 |
| di_N4 | geometric | 89.6 ms | 50 us | 1,783x | 9 / 9 |
| di_N4 | combinatorial_graph | 112.0 ms | 89 us | 1,259x | 9 / 9 |
| di_N6 | combinatorial | 14.46 s | 8.7 ms | 1,665x | 11 / 11 |
| di_N6 | graph | 395.6 ms | 154 us | 2,577x | 11 / 11 |
| di_N6 | geometric | 181.6 ms | 80 us | 2,257x | 11 / 11 |
| di_N6 | combinatorial_graph | 600.5 ms | 222 us | 2,703x | 11 / 11 |
| di_N8 | combinatorial | >300.00 s | 223.5 ms | >1,342x | - / 11 |
| di_N8 | graph | 285.6 ms | 184 us | 1,555x | 11 / 11 |
| di_N8 | geometric | 126.5 ms | 92 us | 1,377x | 11 / 11 |
| di_N8 | combinatorial_graph | 267.4 ms | 256 us | 1,046x | 11 / 11 |
| di_doc_N3 | combinatorial | 174.6 ms | 197 us | 887x | 7 / 7 |
| di_doc_N3 | graph | 73.0 ms | 127 us | 575x | 7 / 7 |
| di_doc_N3 | geometric | 43.0 ms | 45 us | 962x | 7 / 7 |
| di_doc_N3 | combinatorial_graph | 100.5 ms | 81 us | 1,241x | 7 / 7 |
| masses2_N4 | combinatorial | 9.04 s | 12.8 ms | 705x | 307 / 307 |
| masses2_N4 | graph | 4.91 s | 6.6 ms | 740x | 307 / 307 |
| masses2_N4 | geometric | 5.06 s | 9.0 ms | 559x | 307 / 307 |
| masses2_N4 | combinatorial_graph | 5.58 s | 11.1 ms | 502x | 307 / 307 |
| masses2_N6 | combinatorial | >300.00 s | 286.1 ms | >1,049x | - / 901 |
| masses2_N6 | graph | 39.95 s | 44.4 ms | 899x | 901 / 901 |
| masses2_N6 | geometric | 24.30 s | 33.0 ms | 736x | 901 / 901 |
| masses2_N6 | combinatorial_graph | 46.96 s | 116.2 ms | 404x | 901 / 901 |
| sys3x2_N3 | combinatorial | 18.04 s | 18.0 ms | 1,001x | 211 / 211 |
| sys3x2_N3 | graph | 3.09 s | 2.9 ms | 1,065x | 211 / 211 |
| sys3x2_N3 | geometric | 2.19 s | 2.3 ms | 951x | 211 / 211 |
| sys3x2_N3 | combinatorial_graph | 3.43 s | 5.4 ms | 634x | 211 / 211 |
| sys3x2_N5 | combinatorial | >14,400.00 s | 6.53 s | >2,206x | - / 973 |
| sys3x2_N5 | graph | 77.54 s | 90.3 ms | 859x | 973 / 973 |
| sys3x2_N5 | geometric | 18.63 s | 17.5 ms | 1,062x | 973 / 973 |
| sys3x2_N5 | combinatorial_graph | 114.23 s | 293.7 ms | 389x | 973 / 973 |

### Parallel algorithms

| problem | algorithm | PPOPT (processes) | Rust (threads) | speedup | regions (PPOPT / Rust) |
|---|---|---|---|---|---|
| di_N10 | combinatorial_parallel | >300.00 s | 571.5 ms | >525x | - / 11 |
| di_N4 | combinatorial_parallel | 1.18 s | 558 us | 2,110x | 9 / 9 |
| di_N6 | combinatorial_parallel | 4.72 s | 3.7 ms | 1,286x | 11 / 11 |
| di_N8 | combinatorial_parallel | 66.57 s | 33.5 ms | 1,984x | 11 / 11 |
| di_doc_N3 | combinatorial_parallel | 722.8 ms | 320 us | 2,260x | 7 / 7 |
| masses2_N4 | combinatorial_parallel | 6.76 s | 3.4 ms | 1,960x | 307 / 307 |
| masses2_N6 | combinatorial_parallel | 196.24 s | 34.1 ms | 5,755x | 901 / 901 |
| sys3x2_N3 | combinatorial_parallel | 12.74 s | 5.1 ms | 2,497x | 211 / 211 |
| sys3x2_N5 | combinatorial_parallel | >300.00 s | 628.1 ms | >478x | - / 973 |

### Fastest serial algorithm per problem

| problem | fastest PPOPT | fastest Rust (1 thread) | speedup |
|---|---|---|---|
| di_N10 | geometric 112.7 ms | mplcp 63 us | 1,798x |
| di_N4 | geometric 89.6 ms | mplcp 30 us | 3,036x |
| di_N6 | geometric 181.6 ms | mplcp 44 us | 4,167x |
| di_N8 | geometric 126.5 ms | mplcp 53 us | 2,400x |
| di_doc_N3 | geometric 43.0 ms | mplcp 27 us | 1,589x |
| masses2_N4 | graph 4.91 s | graph_exp 6.5 ms | 755x |
| masses2_N6 | geometric 24.30 s | mplcp 27.5 ms | 884x |
| sys3x2_N3 | geometric 2.19 s | mplcp 1.4 ms | 1,576x |
| sys3x2_N5 | geometric 18.63 s | mplcp 9.5 ms | 1,954x |

### Point location

| problem | PPOPT PointLocation (numba) | Rust sequential (PPOPT algorithm) | Rust fastest method |
|---|---|---|---|
| di_N10 | 5,723 ns | 90 ns | hash table (limit 64 MB) 9 ns |
| di_N4 | 5,788 ns | 77 ns | hash table (limit 64 MB) 9 ns |
| di_N6 | 6,088 ns | 88 ns | hash table (limit 64 MB) 9 ns |
| di_N8 | 5,646 ns | 91 ns | hash table (limit 64 MB) 9 ns |
| di_doc_N3 | 5,335 ns | 61 ns | hash table (limit 64 MB) 9 ns |
| dinv_N4 | - | 70 ns | hash table (limit 64 MB) 8 ns |
| dinv_N6 | - | 90 ns | hash table (limit 64 MB) 9 ns |
| dinv_N8 | - | 86 ns | hash table (limit 64 MB) 9 ns |
| masses2_N4 | 23,215 ns | 5095 ns | hash table (limit 64x old16) 31 ns |
| masses2_N6 | 63,909 ns | 14345 ns | hash table (limit 16x old16) 32 ns |
| sys3x2_N3 | 17,911 ns | 2279 ns | hash table (limit 64x old16) 19 ns |
| sys3x2_N5 | 68,451 ns | 10166 ns | hash table (limit 64x old16) 19 ns |

Same algorithm, single thread: measured on 32 runs, min 389x (sys3x2_N5/combinatorial_graph), median 1,020x, max 2,703x; 32 of 32 at 200x or more.
Same algorithm, single thread, PPOPT runs stopped at the time limit (lower bounds): di_N10/combinatorial >2,398x, di_N8/combinatorial >1,342x, masses2_N6/combinatorial >1,049x, sys3x2_N5/combinatorial >2,206x.
Parallel vs parallel: measured on 7 runs, min 1,286x (di_N6/combinatorial_parallel), median 2,110x, max 5,755x; 7 of 7 at 200x or more.
Parallel vs parallel, PPOPT runs stopped at the time limit (lower bounds): di_N10/combinatorial_parallel >525x, sys3x2_N5/combinatorial_parallel >478x.
Fastest serial PPOPT algorithm vs fastest serial Rust algorithm: 755x to 4,167x.
