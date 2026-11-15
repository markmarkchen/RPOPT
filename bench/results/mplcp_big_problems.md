# mplcp on masses5_N4 and masses4_N5: where the time goes (main 1feb98b plus merge-regbuild 8b7bed4, branch big-cost)

One solve each, release build for the plain time, `lp_census --highdim` (lp_census profile, 2 CPUs, 2500M, `RAYON_NUM_THREADS=1`) for the split. Raw census reports: `mplcp_census_masses5_N4.md`, `mplcp_census_masses4_N5.md`.

| problem | regions | plain solve (s, node load 75 to 79) | census solve (s) | a3ebf60 (s) | graph (stored, s) | ratio to graph |
|---|---:|---:|---:|---:|---:|---:|
| masses5_N4 | 44236 | 349.9 | 347.4 | 405 | 153 | 2.3 |
| masses4_N5 | 68247 | 291.4 | 282.5 | 450 | 140 | 2.1 |

## Phase split (lp_census profile)

| | masses5_N4 | masses4_N5 |
|---|---:|---:|
| neighbour search (cand) | 63% | 66% |
| of which walks (Case 2, rank-1 enumeration failed or declined) | 59% | 61% |
| of which rank-1 enumeration | 2% | 2% |
| region construction (build) | 37% | 33% |
| of which Chebyshev LP (centre, cold retry) | 19% | 15% |
| of which redundancy LPs / bounding-box LPs | 8% / 8% | 9% / 7% |
| tableau updates and rebuilds | 0.05% | 0.14% |

Box chain: the fallback guard switched the chain off after 4253 of 44236 regions (masses5_N4; 17019 fallbacks, 29642 engine calls) and after 13084 of 68247 (masses4_N5; 41873 fallbacks, 70930 engine calls). So the chain serves 10% and 19% of the regions; the rest use the shared builder.

## The walks

Case-2 calls per region: 16.0 and 10.6. The rank-1 enumeration answers 706207 of 735947 (masses5_N4) and 723180 of 769797 (masses4_N5); the rest, 29740 and 46617 calls (4% and 6%), go to the walk and cost 6.9 ms and 3.7 ms each.

| walk class (calls, share of solve) | masses5_N4 | masses4_N5 |
|---|---|---|
| one slack, pivot in the gap | 14475, 23% | 28226, 42% |
| one slack, zero pivot, multiplier in the gap band | 10600, 31% | 9105, 4% |
| one slack, zero pivot, symmetry mismatch | 3679, 5% | 6377, 10% |
| one slack, nonzero pivot | 51, 0% | 679, 4% |
| two z / three or more | 935, 0% | 2230, 0% |

Inside the walks: Chebyshev LPs 46% (masses5_N4) and 29% (masses4_N5) of the solve, DAQP tests 10% and 25%.

## Candidate fixes and ceilings (Amdahl, from the shares above)

1. Decide the "pivot in the gap" and "multiplier in the gap band" cases without the walk, by recomputing the pivot or multiplier in double-double from the original data (exact structural zeros come out near 1e-30, real small values stay far above) and the symmetry test likewise. Needs the same audit as the refined rule (walk against enumeration, 0 mismatches). Ceiling if all the walk classes vanish: 2.4x (masses5_N4) and 2.5x (masses4_N5); the pivot-in-gap class alone gives 1.3x and 1.7x, the multiplier gap band alone 1.45x and 1.04x.
2. Cheaper tests inside the walks that remain (shared DAQP row build, warm Chebyshev): bounded by the 55% and 54% they hold, but earlier measurements put each of those at a few percent.
3. Box chain guard: with the chain on for all regions the redundancy and bounding-box LPs (16% of the solve on both) would shrink; ceiling at most 1.19x, probably well under it since the guard exists because the chain falls back on those regions. Not exact in ties, needs a region-set gate. Not built.
4. Per-row LPs (under 2%) and the tableau budget (rebuilds of 17101 tableaux cost 0.05% to 0.14% of the solve): nothing to gain.
