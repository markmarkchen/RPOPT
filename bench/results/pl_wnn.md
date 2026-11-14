# Weighted nearest-neighbour point location (busy machine)

Generated from `pl_wnn_bench_busy_machine.txt`. Timings are medians of 5 interleaved repetitions of single-thread loops of 100,000 queries on a shared machine (load average 10 to 160), so ratios are more reliable than absolute values. Hash table limit 8 MB. Bytes include the fallback table for `wnn` (the hash table it shares). "tr" is the closed-loop trajectory (MPC problems only), "walk" a random walk with steps of 2% of the box.

Status: `Theory` = certified power diagram from the mpLP value function; `Lp` = certified from the facet-adjacency LP; `NotFacetToFacet` = some facet has two or more neighbours, so no power diagram exists and every query goes to the exact fallback (the shared hash table), which makes the `WNN` column equal the hash table column plus one branch. The fallback column counts queries that needed the hash table although the partition is a power diagram (uniform queries outside the feasible set are answered `None` from the domain planes and are not counted). "Duplicate pieces" are regions whose affine value-function piece equals another region's (degenerate mpLP): their sites coincide, so the nearest neighbour can be the twin and the answer comes from a neighbour test or the fallback.

The adjacency LP alone (`WNN_LP_ONLY=1`) does not certify any of the mpLP partitions tried: the adjacency graph from `PartitionGeometry` has 2 (diinf_N4, diinf_N8) or 8 (dione_N4) connected components, so the relative gauge of the components is free and the solution fails the vertex certificate. On `di_N10` the graph is connected and the LP is certified.

## Power-diagram status

| problem | params | regions | status | facets with 2+ neighbours | duplicate pieces | fallback % (inside, uniform) | fallback % (inside, walk) | regions scored / total (uniform) |
|---|---|---|---|---|---|---|---|---|
| file:di_N4 | 2 | 9 | Lp | 0 | 0 | 0.000 | 0.007 | 5.7 / 9 |
| file:di_N6 | 2 | 11 | Lp | 0 | 0 | 0.000 | 0.007 | 7.1 / 11 |
| file:di_N8 | 2 | 11 | Lp | 0 | 0 | 0.000 | 0.009 | 7.1 / 11 |
| file:di_N10 | 2 | 11 | Lp | 0 | 0 | 0.000 | 0.009 | 7.1 / 11 |
| file:di_doc_N3 | 2 | 7 | Lp | 0 | 0 | 0.000 | 0.000 | 7.0 / 7 |
| file:dinv_N4 | 2 | 9 | Lp | 0 | 0 | 0.000 | 0.000 | 6.2 / 9 |
| file:dinv_N6 | 2 | 11 | Lp | 0 | 0 | 0.000 | 0.007 | 9.8 / 11 |
| file:dinv_N8 | 2 | 11 | Lp | 0 | 0 | 0.000 | 0.000 | 9.8 / 11 |
| file:masses2_N4 | 4 | 307 | NotFacetToFacet | 118 | 0 | 100.000 | 100.000 | n/a |
| file:masses2_N6 | 4 | 901 | NotFacetToFacet | 286 | 0 | 100.000 | 100.000 | n/a |
| file:sys3x2_N3 | 3 | 211 | NotFacetToFacet | 6 | 0 | 100.000 | 100.000 | n/a |
| file:sys3x2_N5 | 3 | 973 | NotFacetToFacet | 22 | 0 | 100.000 | 100.000 | n/a |
| lpfile:diinf_N4 | 2 | 39 | Theory | 0 | 3 | 0.000 | 0.000 | 20.4 / 39 |
| lpfile:diinf_N8 | 2 | 62 | Theory | 0 | 0 | 0.000 | 0.007 | 44.6 / 62 |
| lpfile:dione_N4 | 2 | 43 | Theory | 0 | 1 | 0.000 | 0.000 | 28.4 / 43 |
| lpfile:dione_N8 | 2 | 93 | Theory | 0 | 1 | 0.000 | 0.007 | 53.1 / 93 |
| lpfile:masses2inf_N3 | 4 | 1098 | Theory | 0 | 0 | 0.000 | 0.000 | 491.1 / 1098 |
| lpfile:masses2inf_N5 | 4 | 7602 | Theory | 0 | 0 | 0.000 | 0.000 | 2597.6 / 7602 |
| lpfile:masses2one_N4 | 4 | 10974 | Theory | 0 | 0 | 0.000 | 0.000 | 2629.7 / 10974 |
| lpfile:sys3x2inf_N3 | 3 | 504 | Theory | 0 | 214 | 14.019 | 13.263 | 69.4 / 504 |
| lpfile:sys3x2inf_N5 | 3 | 2757 | Theory | 0 | 705 | 13.015 | 11.644 | 83.5 / 2757 |
| lpfile:sys3x2one_N4 | 3 | 2316 | Theory | 0 | 168 | 0.000 | 0.000 | 156.7 / 2316 |
| qp:8:2:100:1 | 2 | 293 | NotFacetToFacet | 20 | 0 | 100.000 | 100.000 | n/a |
| qp:8:3:100:1 | 3 | 1225 | NotFacetToFacet | 100 | 0 | NaN | 100.000 | n/a |
| qp:8:3:100:2 | 3 | 2236 | NotFacetToFacet | 1325 | 0 | 100.000 | 100.000 | n/a |
| qp:5:4:25:1 | 4 | 906 | NotFacetToFacet | 1204 | 0 | 100.000 | 100.000 | n/a |
| qp:4:4:14:1 | 4 | 320 | NotFacetToFacet | 500 | 0 | 100.000 | 100.000 | n/a |
| qp:8:4:60:1 | 4 | 4639 | NotFacetToFacet | 726 | 0 | NaN | 100.000 | n/a |
| qp:5:5:20:1 | 5 | 1136 | NotFacetToFacet | 2081 | 0 | 100.000 | 100.000 | n/a |
| qp:6:5:40:1 | 5 | 7919 | NotFacetToFacet | 18655 | 0 | 100.000 | 100.000 | n/a |
| qp:4:6:14:1 | 6 | 841 | NotFacetToFacet | 2119 | 0 | 100.000 | 100.000 | n/a |
| qp:3:6:14:1 | 6 | 182 | NotFacetToFacet | 304 | 0 | 100.000 | 100.000 | n/a |
| qp:6:6:40:2 | 6 | 4297 | NotFacetToFacet | 4168 | 0 | 100.000 | 100.000 | n/a |
| qp:3:8:10:1 | 8 | 123 | NotFacetToFacet | 213 | 0 | 100.000 | 100.000 | n/a |
| qp:4:8:14:1 | 8 | 1204 | NotFacetToFacet | 5412 | 0 | 100.000 | 100.000 | n/a |
| mass:2:4 | 4 | 307 | NotFacetToFacet | 118 | 0 | 100.000 | 100.000 | n/a |
| mass:3:3 | 6 | 955 | NotFacetToFacet | 1040 | 0 | 100.000 | 100.000 | n/a |

## ns per query

| problem | params | regions | mode | sequential | BST | opt tree | hash 8MB | walk | graph | WNN |
|---|---|---|---|---|---|---|---|---|---|---|
| file:di_N4 | 2 | 9 | uniform | 29.7 | 35.6 | 39.4 | 9.7 | 61.2 | 58.9 | 46.3 |
| file:di_N4 | 2 | 9 | trajectory | 8.8 | 16.0 | 20.5 | 4.8 | 8.5 | 7.6 | 23.7 |
| file:di_N4 | 2 | 9 | walk | 12.1 | 19.8 | 23.9 | 9.9 | 13.4 | 10.6 | 28.4 |
| file:di_N6 | 2 | 11 | uniform | 32.6 | 42.5 | 38.8 | 9.6 | 62.5 | 59.9 | 49.7 |
| file:di_N6 | 2 | 11 | trajectory | 8.8 | 15.9 | 20.2 | 4.8 | 9.2 | 7.6 | 25.6 |
| file:di_N6 | 2 | 11 | walk | 17.9 | 20.8 | 23.0 | 9.6 | 13.4 | 10.5 | 30.1 |
| file:di_N8 | 2 | 11 | uniform | 31.2 | 42.7 | 38.9 | 9.6 | 61.9 | 60.3 | 49.8 |
| file:di_N8 | 2 | 11 | trajectory | 7.1 | 16.2 | 20.2 | 4.8 | 9.2 | 7.6 | 26.7 |
| file:di_N8 | 2 | 11 | walk | 16.2 | 20.7 | 23.0 | 9.5 | 13.8 | 10.3 | 30.0 |
| file:di_N10 | 2 | 11 | uniform | 31.5 | 43.0 | 39.0 | 9.6 | 61.6 | 59.9 | 50.0 |
| file:di_N10 | 2 | 11 | trajectory | 7.1 | 16.3 | 20.2 | 4.8 | 9.3 | 7.6 | 26.8 |
| file:di_N10 | 2 | 11 | walk | 16.3 | 20.7 | 23.1 | 9.7 | 13.5 | 10.4 | 30.3 |
| file:di_doc_N3 | 2 | 7 | uniform | 30.4 | 35.8 | 33.2 | 9.7 | 55.9 | 52.8 | 34.0 |
| file:di_doc_N3 | 2 | 7 | trajectory | 23.8 | 26.1 | 26.9 | 8.7 | 41.3 | 33.1 | 27.8 |
| file:di_doc_N3 | 2 | 7 | walk | 11.7 | 19.3 | 19.9 | 9.2 | 13.0 | 10.3 | 21.1 |
| file:dinv_N4 | 2 | 9 | uniform | 16.9 | 19.3 | 24.5 | 8.6 | 30.3 | 23.8 | 42.4 |
| file:dinv_N4 | 2 | 9 | trajectory | 10.6 | 15.0 | 18.8 | 4.8 | 8.0 | 7.4 | 24.0 |
| file:dinv_N4 | 2 | 9 | walk | 9.0 | 16.1 | 20.5 | 7.1 | 9.0 | 8.0 | 26.7 |
| file:dinv_N6 | 2 | 11 | uniform | 31.8 | 43.9 | 40.8 | 9.9 | 64.0 | 61.5 | 52.4 |
| file:dinv_N6 | 2 | 11 | trajectory | 15.7 | 16.6 | 21.0 | 5.0 | 8.9 | 8.0 | 33.4 |
| file:dinv_N6 | 2 | 11 | walk | 22.7 | 22.7 | 23.8 | 9.9 | 14.1 | 10.8 | 35.1 |
| file:dinv_N8 | 2 | 11 | uniform | 32.0 | 43.5 | 40.0 | 9.8 | 63.3 | 62.2 | 52.4 |
| file:dinv_N8 | 2 | 11 | trajectory | 15.8 | 16.7 | 21.0 | 4.9 | 9.4 | 7.9 | 33.2 |
| file:dinv_N8 | 2 | 11 | walk | 22.9 | 22.6 | 23.7 | 9.7 | 14.3 | 10.7 | 34.9 |
| file:masses2_N4 | 4 | 307 | uniform | 1348.8 | 209.7 | 189.8 | 37.2 | 635.2 | 206.3 | 39.6 |
| file:masses2_N4 | 4 | 307 | trajectory | 386.2 | 203.9 | 174.5 | 17.6 | 125.6 | 56.5 | 20.0 |
| file:masses2_N4 | 4 | 307 | walk | 538.9 | 195.7 | 185.7 | 20.4 | 84.1 | 47.9 | 23.6 |
| file:masses2_N6 | 4 | 901 | uniform | 5525.1 | 265.9 | 232.8 | 43.6 | 857.4 | 270.1 | 46.2 |
| file:masses2_N6 | 4 | 901 | trajectory | 1546.4 | 235.2 | 219.5 | 23.7 | 153.0 | 59.6 | 25.8 |
| file:masses2_N6 | 4 | 901 | walk | 1713.1 | 213.0 | 197.4 | 27.7 | 91.8 | 45.6 | 30.9 |
| file:sys3x2_N3 | 3 | 211 | uniform | 604.5 | 129.0 | 139.9 | 21.8 | 297.7 | 167.1 | 25.8 |
| file:sys3x2_N3 | 3 | 211 | trajectory | 263.1 | 102.5 | 112.6 | 12.6 | 40.0 | 25.4 | 15.0 |
| file:sys3x2_N3 | 3 | 211 | walk | 402.9 | 121.6 | 124.2 | 15.8 | 51.8 | 30.0 | 17.2 |
| file:sys3x2_N5 | 3 | 973 | uniform | 3325.5 | 167.6 | 164.6 | 25.2 | 517.3 | 292.1 | 28.0 |
| file:sys3x2_N5 | 3 | 973 | trajectory | 1017.1 | 151.0 | 143.5 | 12.0 | 49.8 | 31.4 | 14.1 |
| file:sys3x2_N5 | 3 | 973 | walk | 1489.6 | 151.6 | 152.2 | 21.2 | 76.7 | 41.1 | 24.3 |
| lpfile:diinf_N4 | 2 | 39 | uniform | 142.9 | 70.2 | 64.9 | 10.1 | 139.4 | 87.2 | 127.4 |
| lpfile:diinf_N4 | 2 | 39 | walk | 102.5 | 56.3 | 52.6 | 11.1 | 29.5 | 21.2 | 114.5 |
| lpfile:diinf_N8 | 2 | 62 | uniform | 204.5 | 76.7 | 75.5 | 10.2 | 231.5 | 125.7 | 169.2 |
| lpfile:diinf_N8 | 2 | 62 | walk | 94.3 | 67.6 | 65.4 | 11.2 | 47.2 | 33.0 | 160.2 |
| lpfile:dione_N4 | 2 | 43 | uniform | 139.0 | 67.2 | 64.5 | 9.4 | 89.8 | 57.5 | 137.4 |
| lpfile:dione_N4 | 2 | 43 | walk | 37.9 | 46.3 | 46.4 | 10.3 | 22.6 | 17.7 | 122.0 |
| lpfile:dione_N8 | 2 | 93 | uniform | 219.0 | 80.8 | 77.2 | 10.9 | 88.6 | 48.4 | 246.5 |
| lpfile:dione_N8 | 2 | 93 | walk | 141.2 | 63.8 | 63.2 | 12.4 | 33.0 | 24.0 | 236.6 |
| lpfile:masses2inf_N3 | 4 | 1098 | uniform | 11163.2 | 226.2 | 204.3 | 45.7 | 1109.3 | 226.9 | 3204.1 |
| lpfile:masses2inf_N3 | 4 | 1098 | walk | 2466.2 | 192.1 | 189.6 | 29.1 | 131.1 | 58.9 | 2801.3 |
| lpfile:masses2inf_N5 | 4 | 7602 | uniform | 92673.7 | 349.8 | 299.9 | 81.6 | 2413.4 | 385.8 | 13630.7 |
| lpfile:masses2inf_N5 | 4 | 7602 | walk | 27369.9 | 267.3 | 258.3 | 77.8 | 230.5 | 91.0 | 14831.9 |
| lpfile:masses2one_N4 | 4 | 10974 | uniform | 126043.6 | 342.8 | 276.7 | 77.8 | 1873.4 | 375.5 | 18834.6 |
| lpfile:masses2one_N4 | 4 | 10974 | walk | 38894.0 | 229.0 | 208.8 | 61.4 | 126.8 | 48.7 | 19625.0 |
| lpfile:sys3x2inf_N3 | 3 | 504 | uniform | 2025.6 | 130.1 | 124.4 | 21.2 | 259.6 | 103.8 | 562.4 |
| lpfile:sys3x2inf_N3 | 3 | 504 | walk | 695.2 | 113.8 | 111.3 | 16.5 | 52.6 | 29.9 | 478.5 |
| lpfile:sys3x2inf_N5 | 3 | 2757 | uniform | 14166.3 | 181.2 | 162.8 | 28.5 | 306.1 | 132.3 | 1008.8 |
| lpfile:sys3x2inf_N5 | 3 | 2757 | walk | 5628.4 | 150.6 | 143.0 | 24.9 | 76.5 | 38.6 | 1080.1 |
| lpfile:sys3x2one_N4 | 3 | 2316 | uniform | 15445.7 | 168.8 | 152.2 | 27.2 | 700.5 | 274.1 | 1880.1 |
| lpfile:sys3x2one_N4 | 3 | 2316 | walk | 3766.3 | 141.6 | 133.3 | 22.6 | 65.6 | 33.8 | 1714.8 |
| qp:8:2:100:1 | 2 | 293 | uniform | 960.2 | 91.6 | 87.0 | 4.7 | 176.2 | 147.9 | 6.3 |
| qp:8:2:100:1 | 2 | 293 | walk | 343.0 | 75.6 | 73.4 | 8.0 | 80.7 | 58.7 | 9.8 |
| qp:8:3:100:1 | 3 | 1225 | uniform | 12189.6 | 165.6 | 140.2 | 5.6 | 426.1 | 223.3 | 7.2 |
| qp:8:3:100:1 | 3 | 1225 | walk | 1937.0 | 160.2 | 125.3 | 9.1 | 25.9 | 19.6 | 10.9 |
| qp:8:3:100:2 | 3 | 2236 | uniform | 18396.0 | 130.1 | 135.0 | 6.9 | 522.4 | 278.6 | 7.9 |
| qp:8:3:100:2 | 3 | 2236 | walk | 2803.7 | 174.6 | 158.2 | 26.3 | 142.0 | 60.0 | 29.5 |
| qp:5:4:25:1 | 4 | 906 | uniform | 4170.8 | 191.4 | 188.5 | 22.2 | 221.0 | 114.0 | 23.9 |
| qp:5:4:25:1 | 4 | 906 | walk | 89.2 | 184.6 | 188.2 | 15.4 | 42.9 | 28.5 | 17.5 |
| qp:4:4:14:1 | 4 | 320 | uniform | 954.0 | 228.8 | 197.5 | 31.6 | 492.2 | 179.8 | 34.6 |
| qp:4:4:14:1 | 4 | 320 | walk | 862.0 | 171.5 | 164.8 | 18.7 | 50.2 | 29.8 | 19.6 |
| qp:8:4:60:1 | 4 | 4639 | uniform | 56711.4 | 288.7 | 254.3 | 6.7 | 733.4 | 256.4 | 8.4 |
| qp:8:4:60:1 | 4 | 4639 | walk | 5867.2 | 232.1 | 199.7 | 26.6 | 47.9 | 31.7 | 28.2 |
| qp:5:5:20:1 | 5 | 1136 | uniform | 10830.7 | 1115.4 | 282.0 | 20.7 | 268.9 | 162.4 | 20.5 |
| qp:5:5:20:1 | 5 | 1136 | walk | 477.8 | 734.3 | 226.5 | 18.5 | 37.9 | 31.1 | 20.1 |
| qp:6:5:40:1 | 5 | 7919 | uniform | 28843.3 | 16063.6 | 726.0 | 76.0 | 1063.7 | 390.4 | 77.6 |
| qp:6:5:40:1 | 5 | 7919 | walk | 7387.9 | 3941.9 | 431.3 | 50.2 | 103.6 | 57.7 | 50.2 |
| qp:4:6:14:1 | 6 | 841 | uniform | 10259.0 | 5943.5 | 765.0 | 97.4 | 1064.5 | 420.3 | 95.6 |
| qp:4:6:14:1 | 6 | 841 | walk | 948.9 | 1165.4 | 340.4 | 59.8 | 114.3 | 65.4 | 60.7 |
| qp:3:6:14:1 | 6 | 182 | uniform | 327.0 | 248.5 | 245.7 | 45.5 | 650.0 | 223.2 | 49.4 |
| qp:3:6:14:1 | 6 | 182 | walk | 157.8 | 194.3 | 209.2 | 24.1 | 66.9 | 46.4 | 25.1 |
| qp:6:6:40:2 | 6 | 4297 | uniform | 63885.7 | 35423.9 | 1065.2 | 11.1 | 726.3 | 226.9 | 11.4 |
| qp:6:6:40:2 | 6 | 4297 | walk | 4255.6 | 3581.4 | 468.2 | 221.3 | 309.0 | 113.2 | 223.4 |
| qp:3:8:10:1 | 8 | 123 | uniform | 316.6 | 314.4 | 259.1 | 74.9 | 635.0 | 202.9 | 76.7 |
| qp:3:8:10:1 | 8 | 123 | walk | 139.1 | 240.1 | 203.5 | 48.5 | 57.0 | 38.1 | 48.2 |
| qp:4:8:14:1 | 8 | 1204 | uniform | 5152.4 | 4732.3 | 2663.4 | 177.2 | 1174.3 | 403.4 | 178.4 |
| qp:4:8:14:1 | 8 | 1204 | walk | 960.4 | 2070.7 | 1146.6 | 107.5 | 81.3 | 48.5 | 108.3 |
| mass:2:4 | 4 | 307 | uniform | 1222.5 | 188.6 | 172.1 | 34.1 | 599.0 | 189.2 | 35.2 |
| mass:2:4 | 4 | 307 | trajectory | 365.6 | 192.6 | 161.3 | 16.1 | 116.8 | 53.4 | 17.7 |
| mass:2:4 | 4 | 307 | walk | 637.0 | 180.5 | 165.1 | 18.3 | 77.6 | 44.8 | 19.9 |
| mass:3:3 | 6 | 955 | uniform | 8627.7 | 5552.7 | 748.8 | 224.9 | 1938.1 | 788.7 | 179.6 |
| mass:3:3 | 6 | 955 | trajectory | 1136.6 | 1352.1 | 447.7 | 99.6 | 562.7 | 252.4 | 144.0 |
| mass:3:3 | 6 | 955 | walk | 362.2 | 1768.6 | 386.1 | 79.6 | 255.4 | 162.5 | 80.4 |

## Build time (s) and storage (KiB)

| problem | BST s | opt tree s | hash s | WNN s (excl. hash) | BST KiB | opt tree KiB | hash KiB | WNN KiB incl. hash |
|---|---|---|---|---|---|---|---|---|
| file:di_N4 | 0.00 | 0.00 | 0.01 | 0.00 | 1 | 2 | 25 | 27 |
| file:di_N6 | 0.00 | 0.00 | 0.01 | 0.00 | 2 | 2 | 30 | 32 |
| file:di_N8 | 0.00 | 0.00 | 0.01 | 0.00 | 2 | 2 | 30 | 32 |
| file:di_N10 | 0.00 | 0.00 | 0.01 | 0.00 | 2 | 2 | 30 | 32 |
| file:di_doc_N3 | 0.00 | 0.00 | 0.06 | 0.00 | 1 | 1 | 19 | 20 |
| file:dinv_N4 | 0.00 | 0.00 | 0.01 | 0.00 | 1 | 2 | 25 | 27 |
| file:dinv_N6 | 0.00 | 0.00 | 0.04 | 0.00 | 2 | 2 | 30 | 32 |
| file:dinv_N8 | 0.00 | 0.00 | 0.10 | 0.00 | 2 | 2 | 30 | 32 |
| file:masses2_N4 | 0.18 | 3.07 | 0.78 | 0.00 | 744 | 738 | 3244 | 3379 |
| file:masses2_N6 | 1.28 | 6.38 | 3.28 | 0.00 | 2337 | 3640 | 5922 | 6296 |
| file:sys3x2_N3 | 0.02 | 0.26 | 0.27 | 0.00 | 143 | 218 | 1396 | 1449 |
| file:sys3x2_N5 | 0.43 | 1.48 | 1.38 | 0.00 | 921 | 1168 | 6144 | 6380 |
| lpfile:diinf_N4 | 0.00 | 0.00 | 0.02 | 0.00 | 11 | 11 | 114 | 127 |
| lpfile:diinf_N8 | 0.00 | 0.01 | 0.04 | 0.00 | 22 | 23 | 204 | 230 |
| lpfile:dione_N4 | 0.00 | 0.00 | 0.02 | 0.00 | 9 | 10 | 113 | 123 |
| lpfile:dione_N8 | 0.00 | 0.01 | 0.05 | 0.00 | 20 | 22 | 268 | 292 |
| lpfile:masses2inf_N3 | 0.28 | 1.97 | 6.91 | 0.07 | 1112 | 1616 | 6677 | 7405 |
| lpfile:masses2inf_N5 | 16.84 | 19.51 | 15.96 | 2.11 | 10025 | 14327 | 7509 | 12872 |
| lpfile:masses2one_N4 | 8.89 | 18.89 | 11.64 | 5.28 | 11704 | 15170 | 7221 | 13413 |
| lpfile:sys3x2inf_N3 | 0.04 | 0.20 | 0.65 | 0.00 | 279 | 312 | 3264 | 3510 |
| lpfile:sys3x2inf_N5 | 1.02 | 1.61 | 6.26 | 0.09 | 1597 | 1943 | 7365 | 8694 |
| lpfile:sys3x2one_N4 | 0.40 | 1.10 | 4.39 | 0.09 | 1030 | 1240 | 8090 | 8913 |
| qp:8:2:100:1 | 0.01 | 0.09 | 0.12 | 0.00 | 66 | 95 | 620 | 659 |
| qp:8:3:100:1 | 1.18 | 2.32 | 1.26 | 0.00 | 1994 | 1723 | 6328 | 6645 |
| qp:8:3:100:2 | 1.11 | 2.39 | 4.07 | 0.00 | 1957 | 2002 | 5578 | 6147 |
| qp:5:4:25:1 | 3.85 | 10.90 | 1.02 | 0.00 | 8388 | 6522 | 5146 | 5516 |
| qp:4:4:14:1 | 1.15 | 2.60 | 0.93 | 0.00 | 4975 | 1549 | 4341 | 4478 |
| qp:8:4:60:1 | 95.27 | 30.05 | 13.50 | 0.00 | 36202 | 9214 | 5258 | 7180 |
| qp:5:5:20:1 | 21.35 | 30.02 | 2.15 | 0.00 | 34874 | 5492 | 5690 | 6344 |
| qp:6:5:40:1 | 115.07 | 30.01 | 14.59 | 0.00 | 37545 | 7634 | 6396 | 11320 |
| qp:4:6:14:1 | 19.31 | 30.01 | 39.52 | 0.00 | 33962 | 3325 | 7691 | 8426 |
| qp:3:6:14:1 | 0.42 | 12.33 | 1.37 | 0.00 | 2116 | 3614 | 3911 | 4071 |
| qp:6:6:40:2 | 46.31 | 30.00 | 56.38 | 0.00 | 38389 | 4676 | 8027 | 11697 |
| qp:3:8:10:1 | 1.18 | 30.00 | 6.88 | 0.00 | 5337 | 1478 | 7765 | 7952 |
| qp:4:8:14:1 | 30.72 | 30.00 | 86.10 | 0.00 | 35315 | 2428 | 4281 | 6237 |
| mass:2:4 | 0.15 | 1.16 | 0.62 | 0.00 | 742 | 785 | 3244 | 3379 |
| mass:3:3 | 23.28 | 30.01 | 38.71 | 0.00 | 33995 | 2210 | 7279 | 8226 |
