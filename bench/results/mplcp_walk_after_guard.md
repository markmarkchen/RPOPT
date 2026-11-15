# What the Case-2 walk costs after the multiplier rule

Main 107aa72 plus the scale-free multiplier rule (`case2-mu-guard`, `bench/results/mplcp_mu_guard.md`), branch `walkclass` (adds the refusal reason to the
walk-class census, table 3n of `lp_census`). One clean census solve per problem on Negishi (release, 1 CPU, nodes a434 and a443 at load 50 to 60),
timings include the census laps (63 ns each); the class time excludes census work.

Rank-1 calls cost 4.3 to 7.1 us each and are 6% to 10% of the solve. The walk calls that remain:

| problem | solve ms | rank-1 calls, share | walk calls, share of solve | walk us/call |
|---|---:|---|---|---:|
| masses4_N3 | 2,090 | 40,023, 9% | 635, 26% | 868 |
| masses5_N3 | 8,995 | 99,209, 8% | 1,194, 24% | 1,819 |
| masses4_N4 | 21,321 | 186,253, 6% | 9,613, 37% | 817 |
| pop_005 | 2,478 | 52,185, 9% | 3,573, 12% | 81 |
| rand_t12_x5_m20_s0 | 3,162 | 69,133, 10% | 418, 18% | 1,376 |

## Walk calls by class (share of the solve; calls in brackets)

| problem | multiplier in the gap band (nu 1e-9 to 1e-6) | pivot measure in the gap (1e-9 to 1e-6) | symmetry mismatch | nonzero pivot | 2 vanishing z rows | 3 or more rows | other |
|---|---|---|---|---|---|---|---|
| masses4_N3 | 26% (394) | 0% (177) | 0% (10) | 0 | 0% (36) | 0% (18) | 0 |
| masses5_N3 | 24% (724) | 0% (331) | 0% (34) | 0 | 0% (63) | 0% (42) | 0 |
| masses4_N4 | 33% (4,846) | 3% (3,569) | 1% (849) | 0% (9) | 0% (174) | 0% (166) | 0 |
| pop_005 | 7% (1,832) | 4% (1,576) | 0% (160) | 0% (5) | 0 | 0 | 0 |
| rand_t12_x5_m20_s0 | 18% (376) | 0% (42) | 0 | 0 | 0 | 0 | 0 |

* The gap-band class is what is left: one vanishing slack row, a zero pivot, a multiplier with nu between 1e-9 and 1e-6. It is 62% of the
  walk calls on masses4_N3 and 90% on rand_t12, and 24% to 33% of the solve on the three masses problems, 18% on rand_t12, 7% on pop_005.
  Each such walk costs 0.8 to 3.0 ms on the masses problems and rand_t12 (95 us on pop_005), against 4 to 7 us for a rank-1 call.
* Within those walks the Chebyshev LPs are 40% (masses4_N3), 66% (masses5_N3), 56% (masses4_N4), 38% (pop_005) and 89% (rand_t12) of the
  class time, DAQP 48%, 27%, 31%, 26% and 3%.
* The pivot-in-gap class (pivot measure between 1e-9 and 1e-6) is 3% to 4% of the solve on masses4_N4 and pop_005, nothing elsewhere. The symmetry
  mismatch is at most 1%, a nonzero pivot, several vanishing z rows and three or more rows are 0%.

## Widening the zero band to cover the gap band is not an option

Treating nu <= 1e-7 as zero (the gap band mostly gone: masses4_N3 walks 635 to 241) adds one region to masses3_N6 (22,143 to 22,144): the active set
[2, 3, 4, 20, 31], Chebyshev radius 1.2e-5, lies inside the region [2, 3, 4, 20] and the QP at its centre has the active set [2, 3, 4, 20] (`extra_regions`).
It is an overlapping sliver of the kind `base` produced on masses5_N3, not a gap that the walk missed. Accepting the gap multipliers as members gives the
same region. So the band keeps going to the walk, and the walk is the only thing that decides it.

## What this says about the remaining levers

* The hybrid (candidate testing in place of the walk, as `parametric_daqp` does) has a target of 24% to 33% of the solve on masses4_N3, masses5_N3 and
  masses4_N4, 18% on rand_t12 and 7% on pop_005, essentially all in one class (zero pivot, gap-band multiplier).
* Lever 2 (phase-1 empty filter) applies to the Chebyshev LPs inside those walks: 40% to 89% of the class time. It paid on masses (about 8% to
  15% of the solve before the multiplier rule) and not on rand_t12, where phase 1 takes as many pivots as the full LP. With the gap-band walks
  at 24% to 33% of the solve on masses, a 45% saving of the LP share (40% to 66%) is 4% to 10% of the solve on those problems.
