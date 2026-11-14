| problem | algorithm | regions | PPOPT LPs | QPs | rank tests | Rust time | PPOPT lower bound | speedup lower bound |
|---|---|---|---|---|---|---|---|---|
| di_N10 | combinatorial | 11 | 19,051,665 | 0 | 6,975,162 | 29.74 s | 2.55 h | 308x |
| di_N10 | combinatorial_graph | 11 | 1,181 | 1 | 972 | 0.74 ms | 575.5 ms | 779x |
| di_N10 | combinatorial_graph_parallel | 11 | 1,181 | 1 | 972 | 0.66 ms | 575.5 ms | 876x |
| di_N10 | combinatorial_parallel | 11 | 19,051,665 | 0 | 6,975,162 | 2.82 s | 2.55 h | 3,251x |
| di_N10 | geometric | 11 | 539 | 25 | 10 | 0.18 ms | 258.6 ms | 1,468x |
| di_N10 | geometric_parallel | 11 | 539 | 25 | 10 | 0.47 ms | 258.6 ms | 550x |
| di_N10 | graph | 11 | 699 | 100 | 233 | 0.39 ms | 342.0 ms | 878x |
| di_N10 | graph_exp | 11 | 699 | 100 | 239 | 0.38 ms | 342.1 ms | 891x |
| di_N10 | graph_parallel | 11 | 699 | 100 | 239 | 0.74 ms | 342.1 ms | 464x |
| di_N4 | combinatorial | 9 | 2,503 | 0 | 878 | 1.10 ms | 898.6 ms | 818x |
| di_N4 | combinatorial_graph | 9 | 409 | 1 | 345 | 0.17 ms | 149.1 ms | 878x |
| di_N4 | combinatorial_graph_parallel | 9 | 409 | 1 | 345 | 0.41 ms | 149.1 ms | 366x |
| di_N4 | combinatorial_parallel | 9 | 2,503 | 0 | 878 | 0.99 ms | 898.6 ms | 907x |
| di_N4 | geometric | 9 | 231 | 21 | 8 | 0.08 ms | 82.7 ms | 1,081x |
| di_N4 | geometric_parallel | 9 | 231 | 21 | 8 | 0.34 ms | 82.7 ms | 244x |
| di_N4 | graph | 9 | 309 | 91 | 139 | 0.15 ms | 113.7 ms | 770x |
| di_N4 | graph_exp | 9 | 309 | 91 | 143 | 0.14 ms | 113.7 ms | 786x |
| di_N4 | graph_parallel | 9 | 309 | 91 | 143 | 0.47 ms | 113.7 ms | 242x |
| di_N6 | combinatorial | 11 | 50,171 | 0 | 17,766 | 32.79 ms | 19.59 s | 597x |
| di_N6 | combinatorial_graph | 11 | 829 | 1 | 796 | 0.44 ms | 328.7 ms | 743x |
| di_N6 | combinatorial_graph_parallel | 11 | 829 | 1 | 796 | 0.62 ms | 328.7 ms | 533x |
| di_N6 | combinatorial_parallel | 11 | 50,171 | 0 | 17,766 | 7.32 ms | 19.59 s | 2,676x |
| di_N6 | geometric | 11 | 363 | 25 | 10 | 0.12 ms | 141.4 ms | 1,134x |
| di_N6 | geometric_parallel | 11 | 363 | 25 | 10 | 0.44 ms | 141.4 ms | 321x |
| di_N6 | graph | 11 | 523 | 91 | 233 | 0.27 ms | 207.6 ms | 771x |
| di_N6 | graph_exp | 11 | 523 | 91 | 239 | 0.26 ms | 207.7 ms | 785x |
| di_N6 | graph_parallel | 11 | 523 | 91 | 239 | 0.66 ms | 207.7 ms | 316x |
| di_N8 | combinatorial | 11 | 992,375 | 0 | 355,164 | 957.64 ms | 433.25 s | 452x |
| di_N8 | combinatorial_graph | 11 | 1,005 | 1 | 884 | 0.57 ms | 443.2 ms | 774x |
| di_N8 | combinatorial_graph_parallel | 11 | 1,005 | 1 | 884 | 0.68 ms | 443.2 ms | 654x |
| di_N8 | combinatorial_parallel | 11 | 992,375 | 0 | 355,164 | 102.29 ms | 433.25 s | 4,236x |
| di_N8 | geometric | 11 | 451 | 25 | 10 | 0.14 ms | 196.8 ms | 1,368x |
| di_N8 | geometric_parallel | 11 | 451 | 25 | 10 | 0.42 ms | 196.8 ms | 465x |
| di_N8 | graph | 11 | 611 | 100 | 233 | 0.32 ms | 271.6 ms | 840x |
| di_N8 | graph_exp | 11 | 611 | 100 | 239 | 0.32 ms | 271.6 ms | 852x |
| di_N8 | graph_parallel | 11 | 611 | 100 | 239 | 0.66 ms | 271.6 ms | 410x |
| di_doc_N3 | combinatorial | 7 | 629 | 0 | 212 | 0.48 ms | 213.2 ms | 449x |
| di_doc_N3 | combinatorial_graph | 7 | 217 | 1 | 148 | 0.12 ms | 74.3 ms | 624x |
| di_doc_N3 | combinatorial_graph_parallel | 7 | 217 | 1 | 148 | 0.27 ms | 74.3 ms | 275x |
| di_doc_N3 | combinatorial_parallel | 7 | 629 | 0 | 212 | 0.48 ms | 213.2 ms | 443x |
| di_doc_N3 | geometric | 7 | 155 | 15 | 6 | 0.06 ms | 52.6 ms | 879x |
| di_doc_N3 | geometric_parallel | 7 | 155 | 15 | 6 | 0.26 ms | 52.6 ms | 204x |
| di_doc_N3 | graph | 7 | 164 | 100 | 51 | 0.15 ms | 59.1 ms | 406x |
| di_doc_N3 | graph_exp | 7 | 167 | 100 | 55 | 0.14 ms | 60.2 ms | 419x |
| di_doc_N3 | graph_parallel | 7 | 167 | 100 | 55 | 0.34 ms | 60.2 ms | 179x |
| masses2_N4 | combinatorial | 307 | 32,887 | 0 | 7,543 | 29.60 ms | 12.39 s | 419x |
| masses2_N4 | combinatorial_graph | 307 | 22,355 | 1 | 18,886 | 22.58 ms | 8.59 s | 380x |
| masses2_N4 | combinatorial_graph_parallel | 307 | 22,355 | 1 | 18,886 | 4.95 ms | 8.59 s | 1,734x |
| masses2_N4 | combinatorial_parallel | 307 | 32,887 | 0 | 7,543 | 4.76 ms | 12.39 s | 2,601x |
| masses2_N4 | geometric | 307 | 16,945 | 889 | 308 | 15.56 ms | 6.37 s | 409x |
| masses2_N4 | geometric_parallel | 307 | 16,945 | 1,189 | 609 | 3.49 ms | 6.38 s | 1,828x |
| masses2_N4 | graph | 307 | 15,825 | 100 | 2,514 | 12.81 ms | 5.95 s | 465x |
| masses2_N4 | graph_exp | 307 | 16,009 | 100 | 3,437 | 12.79 ms | 6.03 s | 472x |
| masses2_N4 | graph_parallel | 307 | 15,955 | 100 | 3,094 | 3.54 ms | 6.01 s | 1,695x |
| masses2_N6 | combinatorial | 901 | 628,283 | 0 | 202,147 | 936.29 ms | 309.94 s | 331x |
| masses2_N6 | combinatorial_graph | 901 | 160,189 | 1 | 187,489 | 249.81 ms | 80.28 s | 321x |
| masses2_N6 | combinatorial_graph_parallel | 901 | 160,189 | 1 | 187,489 | 38.05 ms | 80.28 s | 2,110x |
| masses2_N6 | combinatorial_parallel | 901 | 628,283 | 0 | 202,147 | 88.41 ms | 309.94 s | 3,506x |
| masses2_N6 | geometric | 901 | 65,867 | 2,237 | 908 | 62.79 ms | 32.38 s | 516x |
| masses2_N6 | geometric_parallel | 901 | 65,867 | 3,557 | 2,228 | 10.20 ms | 32.44 s | 3,180x |
| masses2_N6 | graph | 901 | 73,145 | 100 | 17,362 | 75.24 ms | 36.03 s | 479x |
| masses2_N6 | graph_exp | 901 | 76,041 | 100 | 25,283 | 75.38 ms | 37.52 s | 498x |
| masses2_N6 | graph_parallel | 901 | 74,600 | 100 | 21,972 | 17.03 ms | 36.79 s | 2,160x |
| sys3x2_N3 | combinatorial | 211 | 70,191 | 0 | 22,030 | 52.26 ms | 27.91 s | 534x |
| sys3x2_N3 | combinatorial_graph | 211 | 14,735 | 1 | 13,140 | 11.10 ms | 5.95 s | 535x |
| sys3x2_N3 | combinatorial_graph_parallel | 211 | 14,735 | 1 | 13,140 | 3.50 ms | 5.95 s | 1,700x |
| sys3x2_N3 | combinatorial_parallel | 211 | 70,191 | 0 | 22,030 | 9.60 ms | 27.91 s | 2,908x |
| sys3x2_N3 | geometric | 211 | 8,353 | 307 | 210 | 4.72 ms | 3.31 s | 702x |
| sys3x2_N3 | geometric_parallel | 211 | 8,353 | 604 | 507 | 1.96 ms | 3.32 s | 1,692x |
| sys3x2_N3 | graph | 211 | 9,021 | 100 | 2,202 | 4.92 ms | 3.58 s | 728x |
| sys3x2_N3 | graph_exp | 211 | 9,531 | 100 | 3,127 | 5.40 ms | 3.80 s | 703x |
| sys3x2_N3 | graph_parallel | 211 | 9,345 | 100 | 2,791 | 2.60 ms | 3.72 s | 1,429x |
| sys3x2_N5 | combinatorial | 973 | 15,780,881 | 0 | 5,300,246 | 23.58 s | 1.95 h | 298x |
| sys3x2_N5 | combinatorial_graph | 973 | 370,095 | 1 | 562,400 | 629.13 ms | 168.45 s | 268x |
| sys3x2_N5 | combinatorial_graph_parallel | 973 | 370,095 | 1 | 562,400 | 111.82 ms | 168.45 s | 1,506x |
| sys3x2_N5 | combinatorial_parallel | 973 | 15,780,881 | 0 | 5,300,246 | 2.22 s | 1.95 h | 3,161x |
| sys3x2_N5 | geometric | 973 | 55,277 | 1,197 | 972 | 37.62 ms | 24.47 s | 650x |
| sys3x2_N5 | geometric_parallel | 973 | 55,277 | 2,758 | 2,533 | 7.83 ms | 24.55 s | 3,137x |
| sys3x2_N5 | graph | 973 | 107,479 | 100 | 58,917 | 143.41 ms | 47.99 s | 335x |
| sys3x2_N5 | graph_exp | 973 | 128,549 | 100 | 92,111 | 169.84 ms | 57.59 s | 339x |
| sys3x2_N5 | graph_parallel | 973 | 119,931 | 100 | 78,727 | 56.66 ms | 53.67 s | 947x |

Single-thread, same algorithm: speedup lower bound min 268x (sys3x2_N5/combinatorial_graph), median 624x, max 1,468x over 45 runs; 45 of 45 runs at 200x or more.

| problem | fastest PPOPT algorithm (lower bound) | fastest Rust algorithm (1 thread) | speedup lower bound |
|---|---|---|---|
| di_N10 | geometric 258.6 ms | geometric 0.18 ms | 1,468x |
| di_N4 | geometric 82.7 ms | geometric 0.08 ms | 1,081x |
| di_N6 | geometric 141.4 ms | geometric 0.12 ms | 1,134x |
| di_N8 | geometric 196.8 ms | geometric 0.14 ms | 1,368x |
| di_doc_N3 | geometric 52.6 ms | geometric 0.06 ms | 879x |
| masses2_N4 | graph 5952.5 ms | graph_exp 12.79 ms | 465x |
| masses2_N6 | geometric 32382.2 ms | geometric 62.79 ms | 516x |
| sys3x2_N3 | geometric 3309.3 ms | geometric 4.72 ms | 702x |
| sys3x2_N5 | geometric 24470.0 ms | geometric 37.62 ms | 650x |
