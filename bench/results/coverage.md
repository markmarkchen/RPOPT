Cells: PPOPT time / Rust time (speedup). Coverage: share of the feasible parameter set (uniform samples
of Θ inside the final partition, 100000 samples) lying in regions built so far. "all regions": every region of
the final partition built; "total": the whole solve, including the search for further regions.

| problem | algorithm | regions | 50% | 90% | 99% | 100% | all regions | total |
|---|---|---|---|---|---|---|---|---|
| di_N10 | combinatorial | 11 | 7.81 s / 6.9 ms (1,133x) | >600.00 s / 8.60 s (>70x) | >600.00 s / 8.60 s (>70x) | >600.00 s / 8.60 s (>70x) | >600.00 s / 8.60 s (>70x) | >600.00 s / 8.62 s (>70x) |
| di_N10 | combinatorial_graph | 11 | 168.5 ms / 31 us (5,519x) | 266.1 ms / 104 us (2,564x) | 999.1 ms / 162 us (6,159x) | 1.60 s / 196 us (8,158x) | 1.60 s / 196 us (8,158x) | 2.16 s / 362 us (5,946x) |
| di_N10 | geometric | 11 | 34.2 ms / 30 us (1,144x) | 80.2 ms / 80 us (1,000x) | 113.4 ms / 128 us (884x) | 123.1 ms / 146 us (845x) | 123.1 ms / 146 us (845x) | 126.3 ms / 166 us (759x) |
| di_N10 | graph | 11 | 62.3 ms / 109 us (572x) | 286.8 ms / 149 us (1,929x) | 721.7 ms / 190 us (3,792x) | 821.1 ms / 213 us (3,853x) | 821.1 ms / 213 us (3,853x) | 1.02 s / 272 us (3,745x) |
| di_N4 | combinatorial | 9 | 411.9 ms / 232 us (1,777x) | 848.8 ms / 509 us (1,667x) | 848.8 ms / 509 us (1,667x) | 848.8 ms / 509 us (1,667x) | 848.8 ms / 509 us (1,667x) | 849.2 ms / 514 us (1,652x) |
| di_N4 | combinatorial_graph | 9 | 44.9 ms / 16 us (2,721x) | 80.7 ms / 51 us (1,580x) | 91.8 ms / 70 us (1,310x) | 91.8 ms / 70 us (1,310x) | 91.8 ms / 70 us (1,310x) | 112.8 ms / 118 us (954x) |
| di_N4 | geometric | 9 | 34.8 ms / 17 us (2,045x) | 79.2 ms / 55 us (1,444x) | 154.7 ms / 68 us (2,287x) | 154.7 ms / 68 us (2,287x) | 154.7 ms / 68 us (2,287x) | 161.5 ms / 74 us (2,176x) |
| di_N4 | graph | 9 | 104.2 ms / 43 us (2,424x) | 127.6 ms / 67 us (1,900x) | 289.4 ms / 84 us (3,433x) | 289.4 ms / 84 us (3,433x) | 289.4 ms / 84 us (3,433x) | 351.2 ms / 118 us (2,963x) |
| di_N6 | combinatorial | 11 | 1.63 s / 936 us (1,740x) | 14.41 s / 12.1 ms (1,190x) | 14.41 s / 12.1 ms (1,190x) | 14.41 s / 12.1 ms (1,190x) | 14.41 s / 12.1 ms (1,190x) | 14.41 s / 12.2 ms (1,183x) |
| di_N6 | combinatorial_graph | 11 | 38.1 ms / 20 us (1,861x) | 82.2 ms / 70 us (1,174x) | 114.7 ms / 111 us (1,032x) | 149.8 ms / 132 us (1,134x) | 149.8 ms / 132 us (1,134x) | 537.6 ms / 275 us (1,955x) |
| di_N6 | geometric | 11 | 19.6 ms / 22 us (906x) | 47.9 ms / 71 us (679x) | 70.4 ms / 94 us (747x) | 76.9 ms / 112 us (687x) | 76.9 ms / 112 us (687x) | 80.0 ms / 121 us (662x) |
| di_N6 | graph | 11 | 43.9 ms / 56 us (777x) | 130.6 ms / 86 us (1,515x) | 393.3 ms / 119 us (3,294x) | 601.6 ms / 139 us (4,316x) | 601.6 ms / 139 us (4,316x) | 752.1 ms / 194 us (3,875x) |
| di_N8 | combinatorial | 11 | 3.21 s / 2.8 ms (1,151x) | 501.96 s / 321.2 ms (1,563x) | 501.96 s / 321.2 ms (1,563x) | 501.96 s / 321.2 ms (1,563x) | 501.96 s / 321.2 ms (1,563x) | 501.96 s / 322.9 ms (1,554x) |
| di_N8 | combinatorial_graph | 11 | 147.9 ms / 26 us (5,714x) | 228.1 ms / 86 us (2,638x) | 247.4 ms / 138 us (1,796x) | 254.3 ms / 165 us (1,544x) | 254.3 ms / 165 us (1,544x) | 315.0 ms / 318 us (989x) |
| di_N8 | geometric | 11 | 40.8 ms / 25 us (1,647x) | 98.3 ms / 67 us (1,458x) | 118.1 ms / 107 us (1,103x) | 127.3 ms / 121 us (1,051x) | 127.3 ms / 121 us (1,051x) | 129.0 ms / 136 us (946x) |
| di_N8 | graph | 11 | 30.3 ms / 80 us (379x) | 70.5 ms / 115 us (615x) | 130.1 ms / 152 us (853x) | 167.5 ms / 174 us (962x) | 167.5 ms / 174 us (962x) | 333.8 ms / 230 us (1,449x) |
| di_doc_N3 | combinatorial | 7 | 125.6 ms / 139 us (905x) | 206.2 ms / 266 us (774x) | 206.2 ms / 266 us (774x) | 206.2 ms / 266 us (774x) | 206.2 ms / 266 us (774x) | 206.6 ms / 268 us (771x) |
| di_doc_N3 | combinatorial_graph | 7 | 49.7 ms / 19 us (2,580x) | 117.7 ms / 46 us (2,533x) | 192.3 ms / 61 us (3,143x) | 192.3 ms / 61 us (3,143x) | 192.3 ms / 61 us (3,143x) | 329.5 ms / 95 us (3,453x) |
| di_doc_N3 | geometric | 7 | 70.7 ms / 27 us (2,625x) | 97.6 ms / 46 us (2,118x) | 104.8 ms / 54 us (1,942x) | 104.8 ms / 54 us (1,942x) | 104.8 ms / 54 us (1,942x) | 106.5 ms / 60 us (1,771x) |
| di_doc_N3 | graph | 7 | 58.0 ms / 70 us (833x) | 87.7 ms / 88 us (995x) | 144.2 ms / 104 us (1,388x) | 144.2 ms / 104 us (1,388x) | 144.2 ms / 104 us (1,388x) | 200.1 ms / 139 us (1,438x) |
| masses2_N4 | combinatorial | 307 | 6.04 s / 10.7 ms (563x) | 8.92 s / 20.5 ms (435x) | 8.92 s / 20.5 ms (435x) | 9.95 s / 24.6 ms (405x) | 9.95 s / 24.6 ms (405x) | 9.95 s / 24.7 ms (404x) |
| masses2_N4 | combinatorial_graph | 307 | 1.54 s / 4.1 ms (376x) | 2.48 s / 13.9 ms (179x) | 5.06 s / 15.4 ms (329x) | 5.06 s / 16.6 ms (305x) | 5.29 s / 19.1 ms (277x) | 6.07 s / 20.1 ms (302x) |
| masses2_N4 | geometric | 307 | 1.52 s / 6.3 ms (240x) | 4.29 s / 16.7 ms (257x) | 5.09 s / 17.7 ms (288x) | 5.30 s / 19.3 ms (275x) | 5.30 s / 19.3 ms (275x) | 5.51 s / 20.0 ms (276x) |
| masses2_N4 | graph | 307 | 1.83 s / 5.5 ms (331x) | 3.01 s / 9.7 ms (311x) | 3.26 s / 10.6 ms (309x) | 4.67 s / 13.6 ms (345x) | 5.57 s / 14.8 ms (375x) | 5.66 s / 15.0 ms (377x) |
| masses2_N6 | combinatorial | 901 | 307.01 s / 203.1 ms (1,512x) | 431.47 s / 358.2 ms (1,204x) | 455.58 s / 418.3 ms (1,089x) | 468.20 s / 444.2 ms (1,054x) | 468.20 s / 444.2 ms (1,054x) | 468.20 s / 446.0 ms (1,050x) |
| masses2_N6 | combinatorial_graph | 901 | 18.91 s / 33.1 ms (572x) | 267.07 s / 118.5 ms (2,254x) | 288.71 s / 127.7 ms (2,261x) | 309.20 s / 140.8 ms (2,197x) | 328.34 s / 151.0 ms (2,174x) | 332.15 s / 170.6 ms (1,947x) |
| masses2_N6 | geometric | 901 | 52.44 s / 39.1 ms (1,342x) | 89.94 s / 72.9 ms (1,233x) | 93.82 s / 76.0 ms (1,234x) | 95.28 s / 80.5 ms (1,184x) | 95.60 s / 82.3 ms (1,162x) | 96.50 s / 82.6 ms (1,168x) |
| masses2_N6 | graph | 901 | 51.22 s / 22.8 ms (2,247x) | 87.85 s / 44.2 ms (1,990x) | 95.86 s / 48.8 ms (1,965x) | 118.79 s / 51.3 ms (2,317x) | 139.42 s / 71.5 ms (1,949x) | 140.91 s / 74.1 ms (1,902x) |
| sys3x2_N3 | combinatorial | 211 | 31.74 s / 17.8 ms (1,785x) | 37.74 s / 25.4 ms (1,488x) | 37.74 s / 25.4 ms (1,488x) | 37.99 s / 26.4 ms (1,439x) | 37.99 s / 26.4 ms (1,439x) | 37.99 s / 26.6 ms (1,431x) |
| sys3x2_N3 | combinatorial_graph | 211 | 6.38 s / 2.2 ms (2,867x) | 11.77 s / 6.9 ms (1,701x) | 15.62 s / 6.9 ms (2,257x) | 16.91 s / 6.9 ms (2,443x) | 16.91 s / 6.9 ms (2,443x) | 27.86 s / 7.8 ms (3,560x) |
| sys3x2_N3 | geometric | 211 | 2.37 s / 2.3 ms (1,034x) | 5.36 s / 5.7 ms (935x) | 5.36 s / 5.7 ms (935x) | 5.36 s / 5.7 ms (935x) | 5.36 s / 5.7 ms (935x) | 5.44 s / 5.9 ms (929x) |
| sys3x2_N3 | graph | 211 | 3.73 s / 3.2 ms (1,170x) | 3.95 s / 3.2 ms (1,222x) | 4.53 s / 3.8 ms (1,182x) | 4.94 s / 4.0 ms (1,231x) | 5.79 s / 4.8 ms (1,205x) | 6.09 s / 4.9 ms (1,239x) |
| sys3x2_N5 | combinatorial | 973 | >600.00 s / 8.40 s (>71x) | >600.00 s / 9.76 s (>61x) | >600.00 s / 9.76 s (>61x) | >600.00 s / 9.83 s (>61x) | >600.00 s / 9.83 s (>61x) | >600.00 s / 9.85 s (>61x) |
| sys3x2_N5 | combinatorial_graph | 973 | 98.79 s / 69.0 ms (1,431x) | 112.01 s / 287.3 ms (390x) | 122.62 s / 287.3 ms (427x) | 144.65 s / 287.3 ms (503x) | 178.63 s / 287.3 ms (622x) | 255.81 s / 347.7 ms (736x) |
| sys3x2_N5 | geometric | 973 | 25.68 s / 18.6 ms (1,378x) | 87.99 s / 42.7 ms (2,062x) | 87.99 s / 42.7 ms (2,062x) | 87.99 s / 42.7 ms (2,062x) | 88.35 s / 42.9 ms (2,061x) | 88.53 s / 43.0 ms (2,056x) |
| sys3x2_N5 | graph | 973 | 217.28 s / 40.8 ms (5,325x) | 237.26 s / 41.2 ms (5,753x) | 243.02 s / 61.8 ms (3,930x) | 254.10 s / 64.3 ms (3,950x) | 280.48 s / 100.3 ms (2,797x) | 291.55 s / 106.8 ms (2,730x) |

50%: speedup min 240x (masses2_N4/geometric), median 1,378x, max 5,714x over 35 runs; 35 at 200x or more.
90%: speedup min 179x (masses2_N4/combinatorial_graph), median 1,458x, max 5,753x over 34 runs; 33 at 200x or more.
99%: speedup min 288x (masses2_N4/geometric), median 1,388x, max 6,159x over 34 runs; 34 at 200x or more.
100%: speedup min 275x (masses2_N4/geometric), median 1,388x, max 8,158x over 34 runs; 34 at 200x or more.
all regions: speedup min 275x (masses2_N4/geometric), median 1,388x, max 8,158x over 34 runs; 34 at 200x or more.
total: speedup min 276x (masses2_N4/geometric), median 1,438x, max 5,946x over 34 runs; 34 at 200x or more.
PPOPT runs stopped at the time limit: di_N10/combinatorial (stopped at 600.00 s, 9 of 11 regions built); sys3x2_N5/combinatorial (stopped at 600.00 s, 286 of 973 regions built).
