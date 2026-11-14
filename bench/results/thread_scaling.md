| problem | algorithm | serial (1 thread, serial code) | 1 thr | 2 thr | 4 thr | 8 thr | 12 thr | 16 thr | speedup 16 vs 1 | speedup 16 vs serial | regions ok |
|---|---|---|---|---|---|---|---|---|---|---|---|
| di_N8 | combinatorial_parallel | 957.6 ms | 966.9 | 504.7 | 263.6 | 146.1 | 107.7 | 102.0 | 9.5x | 9.4x | yes |
| di_N8 | graph_parallel | 0.3 ms | 0.4 | 0.3 | 0.3 | 0.5 | 0.8 | 0.7 | 0.6x | 0.5x | yes |
| di_N8 | combinatorial_graph_parallel | 0.6 ms | 0.6 | 0.5 | 0.3 | 0.5 | 0.8 | 0.7 | 1.0x | 0.9x | yes |
| di_N8 | geometric_parallel | 0.1 ms | 0.2 | 0.2 | 0.2 | 0.3 | 0.5 | 0.4 | 0.5x | 0.3x | yes |
| masses2_N4 | combinatorial_parallel | 29.6 ms | 29.4 | 15.4 | 8.1 | 5.3 | 4.6 | 4.8 | 6.2x | 6.2x | yes |
| masses2_N4 | graph_parallel | 12.8 ms | 13.5 | 7.4 | 4.5 | 3.4 | 3.5 | 3.5 | 3.9x | 3.7x | yes |
| masses2_N4 | combinatorial_graph_parallel | 22.6 ms | 23.0 | 12.4 | 7.2 | 4.9 | 5.0 | 4.9 | 4.7x | 4.6x | yes |
| masses2_N4 | geometric_parallel | 15.6 ms | 15.5 | 8.3 | 4.8 | 3.6 | 3.9 | 3.5 | 4.5x | 4.5x | yes |
| masses2_N6 | combinatorial_parallel | 936.3 ms | 935.8 | 483.9 | 248.9 | 132.3 | 97.1 | 89.9 | 10.4x | 10.4x | yes |
| masses2_N6 | graph_parallel | 75.2 ms | 94.3 | 50.4 | 29.0 | 19.1 | 16.9 | 16.9 | 5.6x | 4.4x | yes |
| masses2_N6 | combinatorial_graph_parallel | 249.8 ms | 253.0 | 137.0 | 76.3 | 47.5 | 39.3 | 38.2 | 6.6x | 6.5x | yes |
| masses2_N6 | geometric_parallel | 62.8 ms | 65.3 | 34.4 | 18.8 | 11.7 | 10.4 | 10.0 | 6.5x | 6.3x | yes |
| sys3x2_N3 | combinatorial_parallel | 52.3 ms | 51.6 | 27.2 | 14.5 | 10.5 | 9.2 | 9.4 | 5.5x | 5.5x | yes |
| sys3x2_N3 | graph_parallel | 4.9 ms | 5.8 | 3.4 | 2.2 | 2.3 | 2.9 | 2.7 | 2.1x | 1.8x | yes |
| sys3x2_N3 | combinatorial_graph_parallel | 11.1 ms | 11.4 | 6.2 | 3.7 | 3.0 | 3.7 | 3.5 | 3.2x | 3.1x | yes |
| sys3x2_N3 | geometric_parallel | 4.7 ms | 4.8 | 2.9 | 1.8 | 2.0 | 2.8 | 2.2 | 2.2x | 2.1x | yes |
| sys3x2_N5 | combinatorial_parallel | 23580.0 ms | 23721.8 | 12491.1 | 6357.6 | 3386.4 | 2485.0 | 2309.9 | 10.3x | 10.2x | yes |
| sys3x2_N5 | graph_parallel | 143.4 ms | 351.6 | 191.1 | 105.5 | 68.7 | 57.4 | 55.7 | 6.3x | 2.6x | yes |
| sys3x2_N5 | combinatorial_graph_parallel | 629.1 ms | 646.4 | 357.9 | 209.5 | 137.8 | 113.8 | 110.2 | 5.9x | 5.7x | yes |
| sys3x2_N5 | geometric_parallel | 37.6 ms | 39.9 | 22.0 | 11.9 | 9.2 | 9.8 | 8.3 | 4.8x | 4.5x | yes |

Times in ms. "regions ok": same region count as the serial algorithm and no sampled gaps, overlaps or wrong laws at every thread count.
