# Time profile of mplcp against the other exact mpQP solvers

**Status.** The `mplcp` rows are from main 1feb98b (dedupe, the box-chain check, the cold retry and the refined gap-band rule), with the per-region counters; they **predate the neighbour pre-check**, which is not in main yet, and will be rerun once more when it lands. `mplcp`'s vertex-clip work budget (`mplcp_clip_budget.md`) is not in main either: the `mplcp` rows are taken with the budget off. The other five solvers are final.

## How it was measured

* One census solve per (solver, problem) on Negishi, one CPU, gate profile (`--features lp_census`, `examples/lp_census.rs --pair`), median of up to 3 solves. Census times include two clock reads per LP call, so the **milliseconds are inflated against the release timings** (most on small problems). Use the shares to compare phases and the ms only within a table. The `mplcp` rows have the census-only per-region bookkeeping taken out of the solve time.
* Cached baseline: `~/rpopt/mplcp-regbuild/profile_cache/<solver>.<problem>.json` (the census child's JSON, key `profile`); the `mplcp` rows are in `profile_main/` with the per-region records (`mplcp.<problem>.regions`). Ten (solver, problem) pairs come from older reports instead of a rerun, as asked: `parametric_daqp` on rand_t8, rand_t12, pop_017, pop_005, masses3_N4, masses4_N3 and `graph` on rand_t8, rand_t12, pop_017, masses4_N3 (`~/rpopt/mplcp-build-speed/cen2_out/d1.*` and `m1.*`, an older tree; shares rounded to 1%). Their candidate tests are in the 'neighbour search / candidate tests' column (the reports' 'ms in the tests' for `parametric_daqp`; none for `graph`). The 'source' column says which row is which.
* Three runs did not finish and are recorded as failures: `geometric` on pop_002 and `combinatorial` on pop_002 (1500 s limit) and `geometric` on masses5_N3 (killed at 8 GB).
* The region counts differ between solvers: on masses5_N3 `mplcp` finds 5,273 regions, `graph`, `combinatorial_graph` and `combinatorial` 5,075 to 5,077 and `parametric_daqp` 3,503; on pop_002 `parametric_daqp` 317,175 against 321,026. `mplcp` follows the lexicographic perturbation and splits at degenerate vertices; `parametric_daqp` finds fewer regions on degenerate problems. The solvers do not solve the same number of regions, so compare the ms per region as well as the ms.

## Phases

| phase | what it counts |
|---|---|
| neighbour search / candidate tests | `mplcp`: the whole neighbour search (`expand_candidates`; the Case-2 walk and rank-1 part is given separately). `parametric_daqp`: the active-set tests (DAQP). The others: LP and QP time of the feasibility and neighbour purposes (candidate tests, facet steps). |
| row generation | `mplcp`: the tableau law and the KKT-form rows with normalisation. The others: construction time that is not an LP (rows, clipping, deduplication). |
| bounding-box LPs | `mplcp`: the whole `boxlp::box_chain` call. The others: LP time of the bbox purpose. |
| per-row redundancy tests | `mplcp`: projection tests and per-row LPs. The others: LP time of the redundancy purpose. |
| Chebyshev LP | `mplcp`: the Chebyshev stage of the build. The others: LP time of the chebyshev purpose. |
| other construction | `mplcp`: the rest of the region build (clipping at 5 or fewer parameters, assembly, tightness check); for masses5_N3 this also holds the shared-builder part (47%), because the fallback guard switched the chain off after about 2,500 of the regions. The others: LP time inside construction that has another purpose. |
| tableau update | `mplcp` only. |
| rest | everything else (candidate generation, bookkeeping, exploration), by difference. |

The phases partition the solve; the construction of the non-`mplcp` solvers can overlap a little with candidate tests (feasibility LPs inside a build), so a small 'rest' can come out slightly off.

## sys3x2_N5
| solver | regions | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 973 | 25.3 | 8.4 (33%), Case 2 1.5 (rank-1 0.0) | 3.0 (12%) | 0.0 (0%) | 0.0 (0%) | 2.8 (11%) | 7.8 (31%) | 1.3 (5%) | 2.1 (8%) | census (this tree) |
| graph | 973 | 195.3 | 20.2 (10%) | 15.3 (8%) | 0.0 (0%) | 0.0 (0%) | 2.1 (1%) | 0.0 (0%) | - | 157.7 (81%) | census (this tree) |
| combinatorial_graph | 973 | 632.3 | 65.8 (10%) | 15.4 (2%) | 0.0 (0%) | 0.0 (0%) | 2.2 (0%) | 0.0 (0%) | - | 548.9 (87%) | census (this tree) |
| parametric_daqp | 973 | 424.0 | 291.8 (69%) | 17.3 (4%) | 0.0 (0%) | 0.0 (0%) | 3.1 (1%) | 0.0 (0%) | - | 111.8 (26%) | census (this tree) |
| geometric | 973 | 39.9 | 10.9 (27%) | 15.2 (38%) | 0.0 (0%) | 0.0 (0%) | 3.0 (7%) | 0.0 (0%) | - | 10.8 (27%) | census (this tree) |
| combinatorial | 973 | 15735.0 | 5068.6 (32%) | 59.3 (0%) | 0.0 (0%) | 0.0 (0%) | 13.8 (0%) | 0.0 (0%) | - | 10593.3 (67%) | census (this tree) |
## rand_t8_x5_m20_s0
| solver | regions | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 4,111 | 683.5 | 187.6 (27%), Case 2 121.3 (rank-1 0.0) | 14.4 (2%) | 301.6 (44%) | 79.2 (12%) | 33.7 (5%) | 46.4 (7%) | 7.4 (1%) | 13.3 (2%) | census (this tree) |
| graph | 4,111 | 555.3 | 11.1 (2%) | 55.5 (10%) | 360.9 (65%) | 66.6 (12%) | 27.8 (5%) | 0.0 (0%) | - | 33.3 (6%) | cached report m1.graph.rand_t8_x5_m20_s0.md (shares rounded to 1%) |
| combinatorial_graph | 4,111 | 572.3 | 20.3 (4%) | 57.2 (10%) | 362.4 (63%) | 66.7 (12%) | 29.8 (5%) | 0.0 (0%) | - | 36.0 (6%) | census (this tree) |
| parametric_daqp | 4,105 | 572.9 | 28.0 (5%) | 51.6 (9%) | 366.7 (64%) | 68.7 (12%) | 40.1 (7%) | 0.0 (0%) | - | 17.8 (3%) | cached report d1.parametric_daqp.rand_t8_x5_m20_s0.md (shares rounded to 1%) |
| geometric | 4,111 | 2516.4 | 1163.0 (46%) | 64.8 (3%) | 368.7 (15%) | 67.2 (3%) | 42.2 (2%) | 0.0 (0%) | - | 810.5 (32%) | census (this tree) |
| combinatorial | 4,111 | 573.9 | 14.8 (3%) | 60.6 (11%) | 364.3 (63%) | 67.2 (12%) | 42.8 (7%) | 0.0 (0%) | - | 24.2 (4%) | census (this tree) |
## rand_t12_x5_m20_s0
| solver | regions | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 6,143 | 2211.4 | 557.1 (25%), Case 2 365.4 (rank-1 3.4) | 29.2 (1%) | 1251.8 (57%) | 107.2 (5%) | 105.2 (5%) | 129.4 (6%) | 12.0 (1%) | 19.6 (1%) | census (this tree) |
| graph | 6,143 | 2143.4 | 21.4 (1%) | 128.6 (6%) | 1757.6 (82%) | 85.7 (4%) | 85.7 (4%) | 0.0 (0%) | - | 64.3 (3%) | cached report m1.graph.rand_t12_x5_m20_s0.md (shares rounded to 1%) |
| combinatorial_graph | 6,143 | 2212.1 | 50.9 (2%) | 153.3 (7%) | 1776.1 (80%) | 75.9 (3%) | 96.7 (4%) | 0.0 (0%) | - | 59.2 (3%) | census (this tree) |
| parametric_daqp | 6,061 | 2163.6 | 53.0 (2%) | 151.5 (7%) | 1730.9 (80%) | 64.9 (3%) | 129.8 (6%) | 0.0 (0%) | - | 33.5 (2%) | cached report d1.parametric_daqp.rand_t12_x5_m20_s0.md (shares rounded to 1%) |
| geometric | 6,143 | 48046.4 | 28156.3 (59%) | 168.4 (0%) | 1787.9 (4%) | 76.9 (0%) | 129.5 (0%) | 0.0 (0%) | - | 17727.4 (37%) | census (this tree) |
| combinatorial | 6,143 | 2207.1 | 55.9 (3%) | 154.7 (7%) | 1754.3 (79%) | 75.4 (3%) | 123.6 (6%) | 0.0 (0%) | - | 43.3 (2%) | census (this tree) |
## pop_017
| solver | regions | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 4,142 | 545.4 | 188.9 (35%), Case 2 145.2 (rank-1 1.1) | 17.7 (3%) | 145.2 (27%) | 92.9 (17%) | 23.6 (4%) | 56.5 (10%) | 7.6 (1%) | 12.9 (2%) | census (this tree) |
| graph | 4,143 | 444.7 | 26.7 (6%) | 66.7 (15%) | 182.3 (41%) | 84.5 (19%) | 22.2 (5%) | 0.0 (0%) | - | 62.3 (14%) | cached report m1.graph.pop_017.md (shares rounded to 1%) |
| combinatorial_graph | 4,143 | 483.1 | 56.5 (12%) | 72.3 (15%) | 191.4 (40%) | 88.0 (18%) | 22.7 (5%) | 0.0 (0%) | - | 52.1 (11%) | census (this tree) |
| parametric_daqp | 4,107 | 447.9 | 67.0 (15%) | 67.2 (15%) | 179.2 (40%) | 80.6 (18%) | 31.4 (7%) | 0.0 (0%) | - | 22.6 (5%) | cached report d1.parametric_daqp.pop_017.md (shares rounded to 1%) |
| geometric | 4,139 | 4977.9 | 850.3 (17%) | 71.8 (1%) | 182.1 (4%) | 83.1 (2%) | 30.7 (1%) | 0.0 (0%) | - | 3759.9 (76%) | census (this tree) |
| combinatorial | 4,143 | 462.5 | 23.4 (5%) | 80.1 (17%) | 182.0 (39%) | 83.0 (18%) | 48.2 (10%) | 0.0 (0%) | - | 45.7 (10%) | census (this tree) |
## pop_005
| solver | regions | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 10,289 | 2190.0 | 591.8 (27%), Case 2 419.6 (rank-1 140.3) | 64.0 (3%) | 584.9 (27%) | 622.4 (28%) | 90.7 (4%) | 188.3 (9%) | 23.7 (1%) | 24.3 (1%) | census (this tree) |
| graph | 10,287 | 2008.7 | 173.7 (9%) | 258.6 (13%) | 672.6 (33%) | 545.3 (27%) | 75.6 (4%) | 0.0 (0%) | - | 282.8 (14%) | census (this tree) |
| combinatorial_graph | 10,289 | 3496.6 | 1055.3 (30%) | 261.6 (7%) | 673.6 (19%) | 546.0 (16%) | 76.1 (2%) | 0.0 (0%) | - | 884.0 (25%) | census (this tree) |
| parametric_daqp | 10,273 | 2851.3 | 1096.0 (38%) | 256.6 (9%) | 655.8 (23%) | 541.7 (19%) | 114.1 (4%) | 0.0 (0%) | - | 187.1 (7%) | cached report d1.parametric_daqp.pop_005.md (shares rounded to 1%) |
| geometric | 10,289 | 5335.9 | 1055.9 (20%) | 311.8 (6%) | 739.0 (14%) | 593.0 (11%) | 125.2 (2%) | 0.0 (0%) | - | 2511.0 (47%) | census (this tree) |
| combinatorial | 10,289 | 4544.0 | 1146.5 (25%) | 505.4 (11%) | 674.8 (15%) | 550.1 (12%) | 538.7 (12%) | 0.0 (0%) | - | 1128.6 (25%) | census (this tree) |
## masses3_N4
| solver | regions | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 3,643 | 933.4 | 214.3 (23%), Case 2 164.6 (rank-1 9.5) | 35.0 (4%) | 233.6 (25%) | 298.6 (32%) | 51.2 (5%) | 89.7 (10%) | 3.6 (0%) | 7.4 (1%) | census (this tree) |
| graph | 3,642 | 1022.0 | 66.5 (7%) | 91.4 (9%) | 348.8 (34%) | 318.6 (31%) | 76.2 (7%) | 0.0 (0%) | - | 120.5 (12%) | census (this tree) |
| combinatorial_graph | 3,642 | 1333.7 | 247.5 (19%) | 98.0 (7%) | 366.0 (27%) | 329.9 (25%) | 92.4 (7%) | 0.0 (0%) | - | 200.0 (15%) | census (this tree) |
| parametric_daqp | 3,431 | 974.3 | 241.0 (25%) | 77.9 (8%) | 282.5 (29%) | 253.3 (26%) | 58.5 (6%) | 0.0 (0%) | - | 61.0 (6%) | cached report d1.parametric_daqp.masses3_N4.md (shares rounded to 1%) |
| geometric | 3,642 | 331870.6 | 168199.9 (51%) | 110.3 (0%) | 311.4 (0%) | 272.8 (0%) | 341.6 (0%) | 0.0 (0%) | - | 162634.5 (49%) | census (this tree) |
| combinatorial | 3,642 | 1079.4 | 89.0 (8%) | 118.1 (11%) | 307.3 (28%) | 273.9 (25%) | 173.0 (16%) | 0.0 (0%) | - | 118.0 (11%) | census (this tree) |
## masses4_N3
| solver | regions | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 2,953 | 1586.3 | 282.0 (18%), Case 2 222.3 (rank-1 8.8) | 31.0 (2%) | 529.0 (33%) | 390.7 (25%) | 109.6 (7%) | 235.0 (15%) | 2.4 (0%) | 6.7 (0%) | census (this tree) |
| graph | 2,891 | 1651.0 | 49.5 (3%) | 99.1 (6%) | 726.4 (44%) | 528.3 (32%) | 165.1 (10%) | 16.5 (1%) | - | 66.0 (4%) | cached report m1.graph.masses4_N3.md (shares rounded to 1%) |
| combinatorial_graph | 2,891 | 1822.8 | 84.3 (5%) | 106.7 (6%) | 790.1 (43%) | 519.1 (28%) | 255.3 (14%) | 0.0 (0%) | - | 67.4 (4%) | census (this tree) |
| parametric_daqp | 2,291 | 938.5 | 59.0 (6%) | 65.7 (7%) | 441.1 (47%) | 253.4 (27%) | 84.5 (9%) | 0.0 (0%) | - | 34.9 (4%) | cached report d1.parametric_daqp.masses4_N3.md (shares rounded to 1%) |
| geometric | 2,891 | 232574.0 | 144184.6 (62%) | 124.1 (0%) | 579.9 (0%) | 367.0 (0%) | 1068.9 (0%) | 0.0 (0%) | - | 86249.5 (37%) | census (this tree) |
| combinatorial | 2,891 | 1304.6 | 27.1 (2%) | 86.6 (7%) | 570.8 (44%) | 358.0 (27%) | 222.2 (17%) | 0.0 (0%) | - | 39.8 (3%) | census (this tree) |
## masses5_N3
| solver | regions | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 5,273 | 7972.8 | 1009.6 (13%), Case 2 818.8 (rank-1 56.9) | 96.3 (1%) | 1014.8 (13%) | 630.8 (8%) | 474.9 (6%) | 4729.9 (59%) | 2.9 (0%) | 13.6 (0%) | census (this tree) |
| graph | 5,075 | 8410.8 | 107.9 (1%) | 296.3 (4%) | 3438.2 (41%) | 3225.9 (38%) | 1093.8 (13%) | 0.0 (0%) | - | 248.8 (3%) | census (this tree) |
| combinatorial_graph | 5,076 | 8669.9 | 212.5 (2%) | 302.1 (3%) | 3695.2 (43%) | 2460.6 (28%) | 1851.8 (21%) | 0.0 (0%) | - | 147.6 (2%) | census (this tree) |
| parametric_daqp | 3,503 | 3123.7 | 104.5 (3%) | 151.6 (5%) | 1582.5 (51%) | 921.5 (29%) | 308.1 (10%) | 0.0 (0%) | - | 55.5 (2%) | census (this tree) |
| geometric | - | - | - | - | - | - | - | - | - | - | killed at the memory limit (up to 8 GB) |
| combinatorial | 5,077 | 5534.5 | 57.9 (1%) | 220.4 (4%) | 2495.7 (45%) | 1596.5 (29%) | 1084.0 (20%) | 0.0 (0%) | - | 80.1 (1%) | census (this tree) |
## pop_002
| solver | regions | solve ms | neighbour search / candidate tests | row generation | bounding-box LPs | per-row redundancy tests | Chebyshev LP | other construction | tableau update | rest | source |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| mplcp | 321,026 | 73591.5 | 9675.0 (13%), Case 2 2682.3 (rank-1 140.2) | 2208.1 (3%) | 19410.2 (26%) | 26088.7 (35%) | 4238.0 (6%) | 5971.1 (8%) | 5225.0 (7%) | 775.6 (1%) | census (this tree) |
| graph | 321,026 | 78441.7 | 3456.3 (4%) | 8870.8 (11%) | 30146.1 (38%) | 24388.1 (31%) | 4239.7 (5%) | 0.0 (0%) | - | 7340.6 (9%) | census (this tree) |
| combinatorial_graph | 321,026 | 160716.5 | 57211.5 (36%) | 9339.2 (6%) | 31113.3 (19%) | 24892.4 (15%) | 4503.8 (3%) | 0.0 (0%) | - | 33656.3 (21%) | census (this tree) |
| parametric_daqp | 317,175 | 127718.6 | 51331.6 (40%) | 9187.1 (7%) | 29532.9 (23%) | 23274.8 (18%) | 6228.5 (5%) | 0.0 (0%) | - | 8163.7 (6%) | census (this tree) |
| geometric | - | - | - | - | - | - | - | - | - | - | no result within 1500 s (rc=124) |
| combinatorial | - | - | - | - | - | - | - | - | - | - | no result within 1500 s (rc=124) |
## Summary table
| problem | solver | solve ms | largest phase | its share | LPs of the build (box + per-row + Chebyshev) |
|---|---|---:|---|---:|---:|
| sys3x2_N5 | mplcp | 25.3 | neighbour search / candidate tests | 33% | 11% |
| sys3x2_N5 | graph | 195.3 | rest | 81% | 1% |
| sys3x2_N5 | combinatorial_graph | 632.3 | rest | 87% | 0% |
| sys3x2_N5 | parametric_daqp | 424.0 | neighbour search / candidate tests | 69% | 1% |
| sys3x2_N5 | geometric | 39.9 | row generation | 38% | 7% |
| sys3x2_N5 | combinatorial | 15735.0 | rest | 67% | 0% |
| rand_t8_x5_m20_s0 | mplcp | 683.5 | bounding-box LPs | 44% | 61% |
| rand_t8_x5_m20_s0 | graph | 555.3 | bounding-box LPs | 65% | 82% |
| rand_t8_x5_m20_s0 | combinatorial_graph | 572.3 | bounding-box LPs | 63% | 80% |
| rand_t8_x5_m20_s0 | parametric_daqp | 572.9 | bounding-box LPs | 64% | 83% |
| rand_t8_x5_m20_s0 | geometric | 2516.4 | neighbour search / candidate tests | 46% | 19% |
| rand_t8_x5_m20_s0 | combinatorial | 573.9 | bounding-box LPs | 63% | 83% |
| rand_t12_x5_m20_s0 | mplcp | 2211.4 | bounding-box LPs | 57% | 66% |
| rand_t12_x5_m20_s0 | graph | 2143.4 | bounding-box LPs | 82% | 90% |
| rand_t12_x5_m20_s0 | combinatorial_graph | 2212.1 | bounding-box LPs | 80% | 88% |
| rand_t12_x5_m20_s0 | parametric_daqp | 2163.6 | bounding-box LPs | 80% | 89% |
| rand_t12_x5_m20_s0 | geometric | 48046.4 | neighbour search / candidate tests | 59% | 4% |
| rand_t12_x5_m20_s0 | combinatorial | 2207.1 | bounding-box LPs | 79% | 88% |
| pop_017 | mplcp | 545.4 | neighbour search / candidate tests | 35% | 48% |
| pop_017 | graph | 444.7 | bounding-box LPs | 41% | 65% |
| pop_017 | combinatorial_graph | 483.1 | bounding-box LPs | 40% | 63% |
| pop_017 | parametric_daqp | 447.9 | bounding-box LPs | 40% | 65% |
| pop_017 | geometric | 4977.9 | rest | 76% | 6% |
| pop_017 | combinatorial | 462.5 | bounding-box LPs | 39% | 68% |
| pop_005 | mplcp | 2190.0 | per-row redundancy tests | 28% | 59% |
| pop_005 | graph | 2008.7 | bounding-box LPs | 33% | 64% |
| pop_005 | combinatorial_graph | 3496.6 | neighbour search / candidate tests | 30% | 37% |
| pop_005 | parametric_daqp | 2851.3 | neighbour search / candidate tests | 38% | 46% |
| pop_005 | geometric | 5335.9 | rest | 47% | 27% |
| pop_005 | combinatorial | 4544.0 | neighbour search / candidate tests | 25% | 39% |
| masses3_N4 | mplcp | 933.4 | per-row redundancy tests | 32% | 63% |
| masses3_N4 | graph | 1022.0 | bounding-box LPs | 34% | 73% |
| masses3_N4 | combinatorial_graph | 1333.7 | bounding-box LPs | 27% | 59% |
| masses3_N4 | parametric_daqp | 974.3 | bounding-box LPs | 29% | 61% |
| masses3_N4 | geometric | 331870.6 | neighbour search / candidate tests | 51% | 0% |
| masses3_N4 | combinatorial | 1079.4 | bounding-box LPs | 28% | 70% |
| masses4_N3 | mplcp | 1586.3 | bounding-box LPs | 33% | 65% |
| masses4_N3 | graph | 1651.0 | bounding-box LPs | 44% | 86% |
| masses4_N3 | combinatorial_graph | 1822.8 | bounding-box LPs | 43% | 86% |
| masses4_N3 | parametric_daqp | 938.5 | bounding-box LPs | 47% | 83% |
| masses4_N3 | geometric | 232574.0 | neighbour search / candidate tests | 62% | 1% |
| masses4_N3 | combinatorial | 1304.6 | bounding-box LPs | 44% | 88% |
| masses5_N3 | mplcp | 7972.8 | other construction | 59% | 27% |
| masses5_N3 | graph | 8410.8 | bounding-box LPs | 41% | 92% |
| masses5_N3 | combinatorial_graph | 8669.9 | bounding-box LPs | 43% | 92% |
| masses5_N3 | parametric_daqp | 3123.7 | bounding-box LPs | 51% | 90% |
| masses5_N3 | combinatorial | 5534.5 | bounding-box LPs | 45% | 94% |
| pop_002 | mplcp | 73591.5 | per-row redundancy tests | 35% | 68% |
| pop_002 | graph | 78441.7 | bounding-box LPs | 38% | 75% |
| pop_002 | combinatorial_graph | 160716.5 | neighbour search / candidate tests | 36% | 38% |
| pop_002 | parametric_daqp | 127718.6 | neighbour search / candidate tests | 40% | 46% |

## Reading

* **On rand_t8 and rand_t12 the bounding-box LPs are the largest phase of every solver**: 63% to 82% of the solve for `graph`, `combinatorial_graph`, `parametric_daqp` and `combinatorial` (they share one region builder), 44% and 57% for `mplcp` after the chain. On masses5_N3 `mplcp` shows only 13% in the box LPs and 59% in 'other construction' because the fallback guard switched the chain off (the shared-builder part is not split).
* **`mplcp`'s other large cost is the neighbour search**: 25% to 35% on rand_t8, rand_t12, pop_017 and pop_005, 18% to 23% on the masses problems, 13% on pop_002. Since the refined gap-band rule it is mostly Case-2 walks (rand_t12: 365 ms of 557 ms), and the rank-1 share has become small. The other solvers pay 1% to 9% for candidate tests (`parametric_daqp`'s DAQP tests: 40% on pop_002, 5% on rand_t8).
* **Row generation is small** (0% to 13% in all rows except `geometric` on sys3x2_N5, 38%).
* **`geometric` is far slower than the rest from 6 parameters up** (4.5x on rand_t8, 22x on rand_t12, 140x to 360x on masses4_N3 and masses3_N4).
* **pop_005**: with the dedupe and the chain in main, `mplcp`'s build is no longer twice `graph`'s. Per region both take 62.0 rows into the redundancy step and 20.0 per-row LPs; box 585 ms against 673 ms, per-row tests 622 against 545 (+14%, census noise included), Chebyshev 91 against 76. The 2x in the provisional version came from the tree before the dedupe. No further census.

## Would a region be cheaper to search the mplcp way or the graph-family way?

Per region popped by `mplcp` (counters in the same census solve, `mplcp.<problem>.regions`): the search time `mplcp` spent (Case-1 facets, rank-1 facets, walks, from per-region counter deltas) and the number of one-index changes A + {i}, A - {i} that are not yet in `Seen` at pop time. The graph-family price is that count times `parametric_daqp`'s measured mean microseconds per candidate test on the same problem (column 'daqp us/test'; mean over all its tests including the cheap LICQ-violated ones). `bench/regionsearch_report.py` makes the tables.

| problem | regions popped | daqp us/test | source | mplcp search ms | priced as DAQP tests ms | regions where mplcp is cheaper | per-region ideal choice ms | saving, share of solve | per-facet-type ideal ms | saving, share of solve |
|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|
| sys3x2_N5 | 973 | 0.52 | census | 8.4 | 23.1 | 96% | 8.1 | 1.4% | 2.9 | 21.8% |
| rand_t8_x5_m20_s0 | 4111 | 0.78 | cached report | 187.9 | 47.6 | 0% | 47.6 | 20.5% | 32.4 | 22.8% |
| rand_t12_x5_m20_s0 | 6143 | 0.93 | cached report | 559.5 | 93.3 | 0% | 93.3 | 21.1% | 67.7 | 22.2% |
| pop_017 | 4142 | 1.04 | cached report | 189.3 | 96.9 | 16% | 93.7 | 17.5% | 24.4 | 30.2% |
| pop_005 | 10289 | 1.66 | cached report | 591.0 | 712.9 | 84% | 425.5 | 7.6% | 73.2 | 23.6% |
| masses3_N4 | 3643 | 1.07 | cached report | 214.6 | 199.5 | 59% | 154.6 | 6.4% | 20.7 | 20.8% |
| masses4_N3 | 2953 | 0.68 | cached report | 282.2 | 94.0 | 7% | 93.0 | 11.9% | 22.0 | 16.4% |
| masses5_N3 | 5273 | 0.71 | census | 1010.0 | 193.0 | 1% | 192.7 | 10.3% | 58.3 | 11.9% |
| pop_002 | 321026 | 2.31 | census | 9700.0 | 37472.0 | 99% | 9561.9 | 0.2% | 2487.4 | 9.8% |

Reading. `mplcp`'s search is cheaper than the graph family's tests in almost every region only on the small problem (96% of regions on sys3x2_N5) and on pop_002 (99%, where `parametric_daqp` pays 2.3 us per test on 321,026 regions) and on pop_005 (84%, a tie in total: 591 ms against 713 ms); on rand_t8 and rand_t12 it is cheaper in no region, on masses4_N3 and masses5_N3 in 7% and 1%. The ceiling of an ideal per-region choice is 0.2% (pop_002) to 21% (rand_t8, rand_t12) of the solve; an ideal per-facet-type choice 9.8% to 30%, but see the caveats below.

'regions where mplcp is cheaper' compares, region by region, `mplcp`'s search time with unseen one-index changes x us per test. 'per-region ideal choice' takes the cheaper side in every region; 'saving' is the whole search time minus that, as a share of the solve (a ceiling, assuming the choice is free).

By facet type, `mplcp`'s time against DAQP tests of the same candidates (the candidates that the search returned and that were fresh):

| problem | Case-1 facets: mplcp ms | DAQP ms (same candidates) | cheaper | rank-1: mplcp ms | DAQP ms | cheaper | walks: mplcp ms | DAQP ms | cheaper | search time not in these (none, overhead) ms |
|---|---:|---:|---|---:|---:|---|---:|---:|---|---:|
| sys3x2_N5 | 4.5 | 0.5 | DAQP | 0.0 | 0.0 | mplcp | 1.5 | 0.0 | DAQP | 2.4 |
| rand_t8_x5_m20_s0 | 36.7 | 2.6 | DAQP | 0.0 | 0.7 | mplcp | 121.3 | 0.0 | DAQP | 29.8 |
| rand_t12_x5_m20_s0 | 129.4 | 4.5 | DAQP | 3.5 | 1.1 | DAQP | 363.5 | 0.0 | DAQP | 63.1 |
| pop_017 | 22.7 | 3.1 | DAQP | 1.1 | 1.2 | mplcp | 144.1 | 0.0 | DAQP | 21.3 |
| pop_005 | 113.6 | 13.8 | DAQP | 139.5 | 3.1 | DAQP | 279.2 | 0.7 | DAQP | 58.7 |
| masses3_N4 | 31.0 | 1.5 | DAQP | 9.5 | 2.7 | DAQP | 155.1 | 0.1 | DAQP | 19.1 |
| masses4_N3 | 38.9 | 0.7 | DAQP | 8.8 | 1.6 | DAQP | 213.5 | 0.3 | DAQP | 21.0 |
| masses5_N3 | 134.2 | 0.8 | DAQP | 56.9 | 5.1 | DAQP | 761.9 | 0.7 | DAQP | 57.0 |
| pop_002 | 5202.2 | 671.2 | DAQP | 140.2 | 72.5 | DAQP | 2542.0 | 0.8 | DAQP | 1815.5 |

Caveats. (1) The per-facet-type comparison prices only the fresh candidates a facet returned, while `mplcp`'s time also covers the facets whose neighbours were already known, which the graph family never looks at because it enumerates one-index changes instead of facets; the 'DAQP ms' columns are therefore lower bounds on the graph-family side for those facets and the per-facet ceilings are optimistic. (2) The DAQP price per test comes from `parametric_daqp`'s whole census, not from `mplcp`'s candidates (a candidate that mplcp returns is a neighbour, which is a nonempty test, dearer than the mean). (3) 'search time not in these' is the search time outside the Case-1, rank-1 and walk counters (facets without a zero row, bookkeeping).

## Task 86: the small-problem outliers (one census each, mplcp against combinatorial; `~/rpopt/mplcp-regbuild/small_out2/`)

| problem | parameters | mplcp ms (regions) | combinatorial ms (regions) | what costs in mplcp |
|---|---:|---:|---:|---|
| pop_048 | 4 | 29.2 (46) | 7.8 (57) | **two vertex clips fail at the 600-vertex guard and take 25.7 ms, 88% of the solve (about 12.8 ms each)**; the other 44 clips take 1.1 ms in all; fixed by the clip work budget, 8.8x faster (`mplcp_clip_budget.md`) |
| pop_081 | 6 | 0.28 (4) | 0.20 (6) | no hot spot: neighbour search 0.157 ms (56%), build 0.067 ms (24%), initial region 0.046 ms (16%, 17 QPs and a Chebyshev LP); with 4 regions the fixed costs of the pivoting method (start, tableau, one `adjacent` call per facet) are larger than combinatorial's enumeration |

(The 2-parameter case di_N10 was dropped, as asked.)
