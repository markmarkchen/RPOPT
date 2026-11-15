# Time profile of mplcp against the other exact mpQP solvers

**Status: provisional for `mplcp`.** The `mplcp` rows come from main 685c144 plus the box chain, before the cold-retry merge; the other five solvers are final. The `mplcp` rows are to be rerun once that merge lands (`lp_census --pair <problem.json> mplcp` into the cache, then `bench/time_profile.py`).

## How it was measured

* One census solve per (solver, problem) on Negishi, one CPU, gate profile (`--features lp_census`, `examples/lp_census.rs --pair`), median of up to 3 solves. Census times include two clock reads per LP call, so the **milliseconds are inflated against the release timings** (more on small problems). Use the shares to compare the phases and the ms only within a table.
* Cached baseline: `~/rpopt/mplcp-regbuild/profile_cache/<solver>.<problem>.json` (the census child's JSON, key `profile`). Ten (solver, problem) pairs were taken from older reports instead of being rerun, as asked: `parametric_daqp` on rand_t8, rand_t12, pop_017, pop_005, masses3_N4, masses4_N3 and `graph` on rand_t8, rand_t12, pop_017, masses4_N3 (`~/rpopt/mplcp-build-speed/cen2_out/d1.*` and `m1.*`, an older tree; their shares are rounded to 1%). The 'source' column says which.
* Three runs did not finish: `geometric` on pop_002 and `combinatorial` on pop_002 (1500 s limit) and `geometric` on masses5_N3 (killed at 8 GB).

## Phases

| phase | what it counts |
|---|---|
| neighbour search / candidate tests | `mplcp`: the whole neighbour search (`expand_candidates`; the Case-2 walk and rank-1 part is given separately). `parametric_daqp`: the active-set tests (DAQP). The others: LP and QP time of the feasibility and neighbour purposes (candidate tests, facet steps). |
| row generation | `mplcp`: the tableau law and the KKT-form rows with normalisation. The others: construction time that is not an LP (rows, clipping, deduplication). |
| bounding-box LPs | `mplcp`: the whole `boxlp::box_chain` call. The others: LP time of the bbox purpose. |
| per-row redundancy tests | `mplcp`: projection tests and per-row LPs. The others: LP time of the redundancy purpose. |
| Chebyshev LP | `mplcp`: the Chebyshev stage of the build. The others: LP time of the chebyshev purpose. |
| other construction | `mplcp`: the rest of the region build (clipping at 5 or fewer parameters, assembly, tightness check); for masses5_N3 this also holds the shared-builder part, because the fallback guard switched the chain off after about 2,500 of 3,500 regions. The others: LP time inside construction that has another purpose. |
| tableau update | `mplcp` only. |
| rest | everything else (candidate generation, bookkeeping, exploration), by difference. |

The phases partition the solve; the construction of the non-`mplcp` solvers can overlap a little with candidate tests (feasibility LPs inside a build), so a small 'rest' can come out slightly off.

## sys3x2_N5
| solver | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 31.0 | 10.2 (33%), Case 2 1.6 (rank-1 0.0) | 4.1 (13%) | 0.0 (0%) | 0.0 (0%) | 3.6 (12%) | 9.4 (30%) | 1.8 (6%) | 1.9 (6%) | census (this tree) |
| graph | 195.3 | 20.2 (10%) | 15.3 (8%) | 0.0 (0%) | 0.0 (0%) | 2.1 (1%) | 0.0 (0%) | - | 157.7 (81%) | census (this tree) |
| combinatorial_graph | 632.3 | 65.8 (10%) | 15.4 (2%) | 0.0 (0%) | 0.0 (0%) | 2.2 (0%) | 0.0 (0%) | - | 548.9 (87%) | census (this tree) |
| parametric_daqp | 398.6 | 299.5 (75%) | 15.6 (4%) | 0.0 (0%) | 0.0 (0%) | 3.0 (1%) | 0.0 (0%) | - | 80.6 (20%) | census (this tree) |
| geometric | 39.9 | 10.9 (27%) | 15.2 (38%) | 0.0 (0%) | 0.0 (0%) | 3.0 (7%) | 0.0 (0%) | - | 10.8 (27%) | census (this tree) |
| combinatorial | 15735.0 | 5068.6 (32%) | 59.3 (0%) | 0.0 (0%) | 0.0 (0%) | 13.8 (0%) | 0.0 (0%) | - | 10593.3 (67%) | census (this tree) |
## rand_t8_x5_m20_s0
| solver | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 625.3 | 159.0 (25%), Case 2 105.7 (rank-1 0.0) | 12.3 (2%) | 291.3 (47%) | 78.8 (13%) | 31.9 (5%) | 41.8 (7%) | 4.0 (1%) | 6.3 (1%) | census (this tree) |
| graph | 555.3 | 11.1 (2%) | 55.5 (10%) | 360.9 (65%) | 66.6 (12%) | 27.8 (5%) | 0.0 (0%) | - | 33.3 (6%) | cached report m1.graph.rand_t8_x5_m20_s0.md (shares rounded to 1%) |
| combinatorial_graph | 572.3 | 20.3 (4%) | 57.2 (10%) | 362.4 (63%) | 66.7 (12%) | 29.8 (5%) | 0.0 (0%) | - | 36.0 (6%) | census (this tree) |
| parametric_daqp | 572.9 | 0.0 (0%) | 51.6 (9%) | 366.7 (64%) | 68.7 (12%) | 40.1 (7%) | 0.0 (0%) | - | 45.8 (8%) | cached report d1.parametric_daqp.rand_t8_x5_m20_s0.md (shares rounded to 1%) |
| geometric | 2516.4 | 1163.0 (46%) | 64.8 (3%) | 368.7 (15%) | 67.2 (3%) | 42.2 (2%) | 0.0 (0%) | - | 810.5 (32%) | census (this tree) |
| combinatorial | 573.9 | 14.8 (3%) | 60.6 (11%) | 364.3 (63%) | 67.2 (12%) | 42.8 (7%) | 0.0 (0%) | - | 24.2 (4%) | census (this tree) |
## rand_t12_x5_m20_s0
| solver | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 2678.6 | 1098.5 (41%), Case 2 925.4 (rank-1 592.4) | 27.6 (1%) | 1212.7 (45%) | 105.9 (4%) | 101.2 (4%) | 113.9 (4%) | 5.4 (0%) | 13.5 (1%) | census (this tree) |
| graph | 2143.4 | 21.4 (1%) | 128.6 (6%) | 1757.6 (82%) | 85.7 (4%) | 85.7 (4%) | 0.0 (0%) | - | 64.3 (3%) | cached report m1.graph.rand_t12_x5_m20_s0.md (shares rounded to 1%) |
| combinatorial_graph | 2212.1 | 50.9 (2%) | 153.3 (7%) | 1776.1 (80%) | 75.9 (3%) | 96.7 (4%) | 0.0 (0%) | - | 59.2 (3%) | census (this tree) |
| parametric_daqp | 2163.6 | 0.0 (0%) | 151.5 (7%) | 1730.9 (80%) | 64.9 (3%) | 129.8 (6%) | 0.0 (0%) | - | 86.5 (4%) | cached report d1.parametric_daqp.rand_t12_x5_m20_s0.md (shares rounded to 1%) |
| geometric | 48046.4 | 28156.3 (59%) | 168.4 (0%) | 1787.9 (4%) | 76.9 (0%) | 129.5 (0%) | 0.0 (0%) | - | 17727.4 (37%) | census (this tree) |
| combinatorial | 2207.1 | 55.9 (3%) | 154.7 (7%) | 1754.3 (79%) | 75.4 (3%) | 123.6 (6%) | 0.0 (0%) | - | 43.3 (2%) | census (this tree) |
## pop_017
| solver | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 527.1 | 176.8 (34%), Case 2 140.2 (rank-1 8.5) | 14.8 (3%) | 152.2 (29%) | 97.7 (19%) | 24.0 (5%) | 49.3 (9%) | 4.2 (1%) | 8.0 (2%) | census (this tree) |
| graph | 444.7 | 26.7 (6%) | 66.7 (15%) | 182.3 (41%) | 84.5 (19%) | 22.2 (5%) | 0.0 (0%) | - | 62.3 (14%) | cached report m1.graph.pop_017.md (shares rounded to 1%) |
| combinatorial_graph | 483.1 | 56.5 (12%) | 72.3 (15%) | 191.4 (40%) | 88.0 (18%) | 22.7 (5%) | 0.0 (0%) | - | 52.1 (11%) | census (this tree) |
| parametric_daqp | 447.9 | 0.0 (0%) | 67.2 (15%) | 179.2 (40%) | 80.6 (18%) | 31.4 (7%) | 0.0 (0%) | - | 89.6 (20%) | cached report d1.parametric_daqp.pop_017.md (shares rounded to 1%) |
| geometric | 4977.9 | 850.3 (17%) | 71.8 (1%) | 182.1 (4%) | 83.1 (2%) | 30.7 (1%) | 0.0 (0%) | - | 3759.9 (76%) | census (this tree) |
| combinatorial | 462.5 | 23.4 (5%) | 80.1 (17%) | 182.0 (39%) | 83.0 (18%) | 48.2 (10%) | 0.0 (0%) | - | 45.7 (10%) | census (this tree) |
## pop_005
| solver | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 4348.7 | 1307.9 (30%), Case 2 1011.3 (rank-1 558.7) | 120.8 (3%) | 1198.7 (28%) | 1181.3 (27%) | 157.2 (4%) | 322.8 (7%) | 30.7 (1%) | 29.3 (1%) | census (this tree) |
| graph | 2008.7 | 173.7 (9%) | 258.6 (13%) | 672.6 (33%) | 545.3 (27%) | 75.6 (4%) | 0.0 (0%) | - | 282.8 (14%) | census (this tree) |
| combinatorial_graph | 3496.6 | 1055.3 (30%) | 261.6 (7%) | 673.6 (19%) | 546.0 (16%) | 76.1 (2%) | 0.0 (0%) | - | 884.0 (25%) | census (this tree) |
| parametric_daqp | 2851.3 | 0.0 (0%) | 256.6 (9%) | 655.8 (23%) | 541.7 (19%) | 114.1 (4%) | 0.0 (0%) | - | 1283.1 (45%) | cached report d1.parametric_daqp.pop_005.md (shares rounded to 1%) |
| geometric | 5335.9 | 1055.9 (20%) | 311.8 (6%) | 739.0 (14%) | 593.0 (11%) | 125.2 (2%) | 0.0 (0%) | - | 2511.0 (47%) | census (this tree) |
| combinatorial | 4544.0 | 1146.5 (25%) | 505.4 (11%) | 674.8 (15%) | 550.1 (12%) | 538.7 (12%) | 0.0 (0%) | - | 1128.6 (25%) | census (this tree) |
## masses3_N4
| solver | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 920.2 | 239.2 (26%), Case 2 193.9 (rank-1 44.0) | 34.0 (4%) | 224.2 (24%) | 297.2 (32%) | 51.1 (6%) | 65.4 (7%) | 3.0 (0%) | 6.1 (1%) | census (this tree) |
| graph | 1022.0 | 66.5 (7%) | 91.4 (9%) | 348.8 (34%) | 318.6 (31%) | 76.2 (7%) | 0.0 (0%) | - | 120.5 (12%) | census (this tree) |
| combinatorial_graph | 1333.7 | 247.5 (19%) | 98.0 (7%) | 366.0 (27%) | 329.9 (25%) | 92.4 (7%) | 0.0 (0%) | - | 200.0 (15%) | census (this tree) |
| parametric_daqp | 974.3 | 0.0 (0%) | 77.9 (8%) | 282.5 (29%) | 253.3 (26%) | 58.5 (6%) | 0.0 (0%) | - | 302.0 (31%) | cached report d1.parametric_daqp.masses3_N4.md (shares rounded to 1%) |
| geometric | 331870.6 | 168199.9 (51%) | 110.3 (0%) | 311.4 (0%) | 272.8 (0%) | 341.6 (0%) | 0.0 (0%) | - | 162634.5 (49%) | census (this tree) |
| combinatorial | 1079.4 | 89.0 (8%) | 118.1 (11%) | 307.3 (28%) | 273.9 (25%) | 173.0 (16%) | 0.0 (0%) | - | 118.0 (11%) | census (this tree) |
## masses4_N3
| solver | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 2003.4 | 806.9 (40%), Case 2 747.7 (rank-1 543.2) | 35.0 (2%) | 517.0 (26%) | 391.5 (20%) | 90.5 (5%) | 153.5 (8%) | 2.6 (0%) | 6.3 (0%) | census (this tree) |
| graph | 1651.0 | 49.5 (3%) | 99.1 (6%) | 726.4 (44%) | 528.3 (32%) | 165.1 (10%) | 16.5 (1%) | - | 66.0 (4%) | cached report m1.graph.masses4_N3.md (shares rounded to 1%) |
| combinatorial_graph | 1822.8 | 84.3 (5%) | 106.7 (6%) | 790.1 (43%) | 519.1 (28%) | 255.3 (14%) | 0.0 (0%) | - | 67.4 (4%) | census (this tree) |
| parametric_daqp | 938.5 | 0.0 (0%) | 65.7 (7%) | 441.1 (47%) | 253.4 (27%) | 84.5 (9%) | 0.0 (0%) | - | 93.8 (10%) | cached report d1.parametric_daqp.masses4_N3.md (shares rounded to 1%) |
| geometric | 232574.0 | 144184.6 (62%) | 124.1 (0%) | 579.9 (0%) | 367.0 (0%) | 1068.9 (0%) | 0.0 (0%) | - | 86249.5 (37%) | census (this tree) |
| combinatorial | 1304.6 | 27.1 (2%) | 86.6 (7%) | 570.8 (44%) | 358.0 (27%) | 222.2 (17%) | 0.0 (0%) | - | 39.8 (3%) | census (this tree) |
## masses5_N3
| solver | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 9005.0 | 3175.6 (35%), Case 2 2962.9 (rank-1 2217.6) | 101.3 (1%) | 1217.2 (14%) | 812.0 (9%) | 467.5 (5%) | 3210.3 (36%) | 3.0 (0%) | 18.1 (0%) | census (this tree) |
| graph | 8410.8 | 107.9 (1%) | 296.3 (4%) | 3438.2 (41%) | 3225.9 (38%) | 1093.8 (13%) | 0.0 (0%) | - | 248.8 (3%) | census (this tree) |
| combinatorial_graph | 8669.9 | 212.5 (2%) | 302.1 (3%) | 3695.2 (43%) | 2460.6 (28%) | 1851.8 (21%) | 0.0 (0%) | - | 147.6 (2%) | census (this tree) |
| parametric_daqp | 3097.6 | 103.3 (3%) | 144.3 (5%) | 1580.8 (51%) | 922.6 (30%) | 307.7 (10%) | 0.0 (0%) | - | 38.9 (1%) | census (this tree) |
| geometric | - | - | - | - | - | - | - | - | - | killed at the memory limit (up to 8 GB) |
| combinatorial | 5534.5 | 57.9 (1%) | 220.4 (4%) | 2495.7 (45%) | 1596.5 (29%) | 1084.0 (20%) | 0.0 (0%) | - | 80.1 (1%) | census (this tree) |
## pop_002
| solver | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 79620.2 | 10658.6 (13%), Case 2 2797.9 (rank-1 155.2) | 2749.1 (3%) | 18599.0 (23%) | 25961.4 (33%) | 4716.5 (6%) | 6491.3 (8%) | 9397.2 (12%) | 1047.1 (1%) | census (this tree) |
| graph | 78441.7 | 3456.3 (4%) | 8870.8 (11%) | 30146.1 (38%) | 24388.1 (31%) | 4239.7 (5%) | 0.0 (0%) | - | 7340.6 (9%) | census (this tree) |
| combinatorial_graph | 160716.5 | 57211.5 (36%) | 9339.2 (6%) | 31113.3 (19%) | 24892.4 (15%) | 4503.8 (3%) | 0.0 (0%) | - | 33656.3 (21%) | census (this tree) |
| parametric_daqp | 125320.9 | 51035.5 (41%) | 8837.4 (7%) | 29626.2 (24%) | 23469.6 (19%) | 6219.0 (5%) | 0.0 (0%) | - | 6133.0 (5%) | census (this tree) |
| geometric | - | - | - | - | - | - | - | - | - | no result within 1500 s (rc=124) |
| combinatorial | - | - | - | - | - | - | - | - | - | no result within 1500 s (rc=124) |
## Summary table
| problem | solver | solve ms | largest phase | its share | LPs of the build (box + per-row + Chebyshev) |
|---|---|---:|---|---:|---:|
| sys3x2_N5 | mplcp | 31.0 | neighbour search / candidate tests | 33% | 12% |
| sys3x2_N5 | graph | 195.3 | rest | 81% | 1% |
| sys3x2_N5 | combinatorial_graph | 632.3 | rest | 87% | 0% |
| sys3x2_N5 | parametric_daqp | 398.6 | neighbour search / candidate tests | 75% | 1% |
| sys3x2_N5 | geometric | 39.9 | row generation | 38% | 7% |
| sys3x2_N5 | combinatorial | 15735.0 | rest | 67% | 0% |
| rand_t8_x5_m20_s0 | mplcp | 625.3 | bounding-box LPs | 47% | 64% |
| rand_t8_x5_m20_s0 | graph | 555.3 | bounding-box LPs | 65% | 82% |
| rand_t8_x5_m20_s0 | combinatorial_graph | 572.3 | bounding-box LPs | 63% | 80% |
| rand_t8_x5_m20_s0 | parametric_daqp | 572.9 | bounding-box LPs | 64% | 83% |
| rand_t8_x5_m20_s0 | geometric | 2516.4 | neighbour search / candidate tests | 46% | 19% |
| rand_t8_x5_m20_s0 | combinatorial | 573.9 | bounding-box LPs | 63% | 83% |
| rand_t12_x5_m20_s0 | mplcp | 2678.6 | bounding-box LPs | 45% | 53% |
| rand_t12_x5_m20_s0 | graph | 2143.4 | bounding-box LPs | 82% | 90% |
| rand_t12_x5_m20_s0 | combinatorial_graph | 2212.1 | bounding-box LPs | 80% | 88% |
| rand_t12_x5_m20_s0 | parametric_daqp | 2163.6 | bounding-box LPs | 80% | 89% |
| rand_t12_x5_m20_s0 | geometric | 48046.4 | neighbour search / candidate tests | 59% | 4% |
| rand_t12_x5_m20_s0 | combinatorial | 2207.1 | bounding-box LPs | 79% | 88% |
| pop_017 | mplcp | 527.1 | neighbour search / candidate tests | 34% | 52% |
| pop_017 | graph | 444.7 | bounding-box LPs | 41% | 65% |
| pop_017 | combinatorial_graph | 483.1 | bounding-box LPs | 40% | 63% |
| pop_017 | parametric_daqp | 447.9 | bounding-box LPs | 40% | 65% |
| pop_017 | geometric | 4977.9 | rest | 76% | 6% |
| pop_017 | combinatorial | 462.5 | bounding-box LPs | 39% | 68% |
| pop_005 | mplcp | 4348.7 | neighbour search / candidate tests | 30% | 58% |
| pop_005 | graph | 2008.7 | bounding-box LPs | 33% | 64% |
| pop_005 | combinatorial_graph | 3496.6 | neighbour search / candidate tests | 30% | 37% |
| pop_005 | parametric_daqp | 2851.3 | rest | 45% | 46% |
| pop_005 | geometric | 5335.9 | rest | 47% | 27% |
| pop_005 | combinatorial | 4544.0 | neighbour search / candidate tests | 25% | 39% |
| masses3_N4 | mplcp | 920.2 | per-row redundancy tests | 32% | 62% |
| masses3_N4 | graph | 1022.0 | bounding-box LPs | 34% | 73% |
| masses3_N4 | combinatorial_graph | 1333.7 | bounding-box LPs | 27% | 59% |
| masses3_N4 | parametric_daqp | 974.3 | rest | 31% | 61% |
| masses3_N4 | geometric | 331870.6 | neighbour search / candidate tests | 51% | 0% |
| masses3_N4 | combinatorial | 1079.4 | bounding-box LPs | 28% | 70% |
| masses4_N3 | mplcp | 2003.4 | neighbour search / candidate tests | 40% | 50% |
| masses4_N3 | graph | 1651.0 | bounding-box LPs | 44% | 86% |
| masses4_N3 | combinatorial_graph | 1822.8 | bounding-box LPs | 43% | 86% |
| masses4_N3 | parametric_daqp | 938.5 | bounding-box LPs | 47% | 83% |
| masses4_N3 | geometric | 232574.0 | neighbour search / candidate tests | 62% | 1% |
| masses4_N3 | combinatorial | 1304.6 | bounding-box LPs | 44% | 88% |
| masses5_N3 | mplcp | 9005.0 | other construction | 36% | 28% |
| masses5_N3 | graph | 8410.8 | bounding-box LPs | 41% | 92% |
| masses5_N3 | combinatorial_graph | 8669.9 | bounding-box LPs | 43% | 92% |
| masses5_N3 | parametric_daqp | 3097.6 | bounding-box LPs | 51% | 91% |
| masses5_N3 | combinatorial | 5534.5 | bounding-box LPs | 45% | 94% |
| pop_002 | mplcp | 79620.2 | per-row redundancy tests | 33% | 62% |
| pop_002 | graph | 78441.7 | bounding-box LPs | 38% | 75% |
| pop_002 | combinatorial_graph | 160716.5 | neighbour search / candidate tests | 36% | 38% |
| pop_002 | parametric_daqp | 125320.9 | neighbour search / candidate tests | 41% | 47% |

## Reading

* **From about 8 parameters up, the bounding-box LPs are the largest phase of every solver except `mplcp`**: 63% to 82% of the solve for `graph`, `combinatorial_graph`, `parametric_daqp` and `combinatorial` on rand_t8 and rand_t12 (they share one region builder). `mplcp` has them at 45% to 47% there, even after the chain, because its own neighbour search (25% to 41%) comes on top.
* **`mplcp`'s other large cost is the neighbour search**, 25% to 41% on the 8 to 12 parameter problems, mostly Case 2 (rank-1 is 54% of the neighbour search on rand_t12 and 67% on masses4_N3). The other solvers pay 1% to 9% for candidate tests (`parametric_daqp`'s DAQP tests: 41% on pop_002, 75% on sys3x2_N5).
* **Row generation is small** (0% to 13% in all rows except `geometric` on sys3x2_N5, 38%).
* **`geometric` is far slower than the rest from 6 parameters on** (4.5x on rand_t8, 22x on rand_t12, 140x to 360x on masses4_N3 and masses3_N4); its time is in the neighbour search and in bookkeeping outside the LPs.
* On the small problem (sys3x2_N5) the solve is dominated by the neighbour search and the construction non-LP part for `mplcp`, and by non-LP candidate handling ('rest') for `graph` and `combinatorial_graph`.

## Task 86: the small-problem outliers (one census each, mplcp against combinatorial; `~/rpopt/mplcp-regbuild/small_out2/`)

| problem | parameters | mplcp ms (regions) | combinatorial ms (regions) | what costs in mplcp |
|---|---:|---:|---:|---|
| pop_048 | 4 | 29.2 (46) | 7.8 (57) | **two vertex clips fail at the 600-vertex guard and take 25.7 ms, 88% of the solve (about 12.8 ms each)**; the other 44 clips take 1.1 ms in all (0.025 ms each); the fallback after a failed clip is cheap (0.3 LPs per region) |
| pop_081 | 6 | 0.28 (4) | 0.20 (6) | no hot spot: neighbour search 0.157 ms (56%), build 0.067 ms (24%), initial region 0.046 ms (16%, 17 QPs and a Chebyshev LP); with 4 regions the fixed costs of the pivoting method (start, tableau, one `adjacent` call per facet) are larger than combinatorial's enumeration |
| di_N10 | 2 | 0.20 (11) | 17,540 | no outlier in this census: build 0.084 ms (42%), neighbour search 0.051 ms (26%), initial region 0.032 ms (16%), setup 0.004 ms |

* pop_048: the clip's cost is quadratic in the vertex count, so a clip that will exceed the guard burns about 12 ms before it gives up, where the LP path takes about 0.1 to 0.3 ms per region. A work budget on the clip (abort when the edge pairs tested exceed a few hundred thousand, or the vertex count passes about 150 at 4 parameters) would bring pop_048 from 29 ms towards 4 ms if the 2 failing regions then cost the LP path's 0.1 to 0.3 ms each (not measured). The earlier sweep of lower vertex guards (`mplcp_build_speed.md`, item 3) was at 6 parameters and found them slower there; 4 parameters was not swept. Not changed here.
* pop_081 and the 2-parameter 1.05x: one census cannot attribute a 5% difference between two measurements (10 microseconds on di_N10). If it matters, the way to find it is a paired A/B of the two commits on di_N10 at release scale. Not done.
