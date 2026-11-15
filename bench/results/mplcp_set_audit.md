# mplcp set audit: genuine regions lost or gained between c345cdc and main a3ebf60

Measurement only, no solver code changed. Date 2026-11-15, Negishi, 1-CPU array tasks.

## Method

- **Builds (release, one solve each):** old = c345cdc, new = main a3ebf60 (it contains the multiplier rule of 59b2afb). Each tree has its own target directory; an earlier shared target directory silently reused the old tree's library for the new build, so both binaries were checked to differ (`cmp`). Solve examples: `examples/audit_solve.rs` (sorted active sets, one `[i, j, ...]` per line).
- **Problems:** 153 problems: the 49-problem set plus the large set (the 159 problem names of the earlier hash sweeps) minus pop_002, pop_024, pop_033, pop_036, pop_051, pop_095 (each above 2 GB). The 158 of the brief would add those six; they are not included.
- **Not run, by the coordinator's instruction:** the walk-only oracle on unfixed main (it loses the same thin regions). The oracle columns below come from `mplcp-case2-cost`: its fix build (Chebyshev cold retry in `build_region`, 1f080db) and the same build with the walk only (`set_rank1(false)`), files `fix.<problem>.txt` and `fixwalk.<problem>.txt`. Those exist for 8 of the 10 problems with differences; masses4_N5 and masses5_N4 were not part of that sweep ("n/a").
- **Not run as a separate build:** 169cff5 (twin snap, `across_fallback`); its sets were not needed once the comparison old against main was made.
- **Classification** (`examples/set_audit.rs`, on the ten problems whose old and new set files differ; the other 143 problems have byte-identical sorted set files). Every line of every set file is parsed strictly (a bad token aborts). Each set is rebuilt into a region (`gen_cr_from_active_set`), then:
  - (a) genuine and lost: in old only, QP active set at the Chebyshev centre equals the set (`tight_diagnostic` agrees), the centre lies in no region of the new solve, and it overlaps no other old region;
  - (b) the same for sets in new only (genuine, gained);
  - (c) spurious: QP disagrees or is infeasible at the centre, or the region contains the centre of another region of its own solve;
  - (d) exchanged duplicate: the centre lies in a region of the other solve that is itself not in the common set list (the same region under another active set);
  - undecided: set cannot be rebuilt and the solver dump has no entry.
- **Sets that cannot be rebuilt** from their own active set (`gen_cr_from_active_set` returns none, or its Chebyshev LP fails): judged through the solver's own region, dumped as (radius, centre) by `audit_solve` with `AUDIT_DUMP`. The QP active set at that centre decides: equal to the set means genuine (marked "unbuildable"), a set in the other solve's list means duplicate, otherwise spurious. Radius zero is spurious.
- Run-to-run consistency: the 20 dump re-solves (old and new on the ten problems) reproduce the sorted set files of the first solves byte for byte.
- Weaker verdicts: a set that cannot be rebuilt ("unbuildable") has no region to test against the other solve's regions, so its (a) or (b) verdict rests on the QP set at the solver's own centre only.

## Result

- 143 of 153 problems: old and new set files are identical. The ten with differences are all masses problems (masses3_N5, N6, N7, masses4_N2, N3, N4, N5, masses5_N2, N3, N4).
- Total genuine regions in old and absent in new, (a): **355** (32 of them present in the fix build, 4 absent from it, 319 on masses4_N5 and masses5_N4 where no fix sets exist).
- Genuine regions gained by main, (b): see table; they are the regions the skip fix and the multiplier rule added.
- (a) radii range from 1.0e-08 to 4.2e-05; lex-infeasible among (a): 0.

### Old c345cdc against main a3ebf60

| problem | old regions | main regions | only old | only main | (a) lost | (b) gained | (c) spurious (old-only / main-only) | (d) duplicate | undecided |
|---|---|---|---|---|---|---|---|---|---|
| masses3_N5 | 10437 | 10436 | 2 | 1 | 2 | 0 | 1 (0 / 1) | 0 | 0 |
| masses3_N6 | 22145 | 22143 | 3 | 1 | 3 | 1 | 0 (0 / 0) | 0 | 0 |
| masses3_N7 | 38330 | 38326 | 6 | 2 | 5 | 2 | 1 (1 / 0) | 0 | 0 |
| masses4_N2 | 325 | 327 | 0 | 2 | 0 | 0 | 0 (0 / 0) | 2 | 0 |
| masses4_N3 | 2891 | 2947 | 6 | 62 | 6 | 35 | 26 (0 / 26) | 1 | 0 |
| masses4_N4 | 16050 | 16312 | 27 | 289 | 18 | 135 | 156 (9 / 147) | 7 | 0 |
| masses4_N5 | 66922 | 67963 | 257 | 1298 | 183 | 460 | 886 (74 / 812) | 26 | 0 |
| masses5_N2 | 370 | 373 | 0 | 3 | 0 | 2 | 1 (0 / 1) | 0 | 0 |
| masses5_N3 | 5060 | 5266 | 3 | 209 | 2 | 141 | 66 (1 / 65) | 3 | 0 |
| masses5_N4 | 42484 | 44065 | 150 | 1731 | 136 | 1091 | 619 (13 / 606) | 31 | 4 |
| total | | | 454 | 3598 | 355 | 1867 | 1756 | 70 | 4 |

(c) is split into old-only / new-only in parentheses. "undecided" rows are counted separately and are not inside (c).

### Old c345cdc against the fix build (mplcp-case2-cost, 1f080db)

Only 8 problems have fix sets. The remaining (a) are genuine regions of c345cdc that even the fix build lacks.

| problem | old regions | fix regions | only old | only fix | (a) lost | (b) gained | (c) spurious (old-only / fix-only) | (d) duplicate | undecided |
|---|---|---|---|---|---|---|---|---|---|
| masses3_N5 | 10437 | 10444 | 0 | 7 | 0 | 4 | 3 (0 / 3) | 0 | 0 |
| masses3_N6 | 22145 | 22152 | 1 | 8 | 1 | 8 | 0 (0 / 0) | 0 | 0 |
| masses3_N7 | 38330 | 38343 | 3 | 16 | 2 | 14 | 1 (1 / 0) | 0 | 2 |
| masses4_N2 | 325 | 327 | 0 | 2 | 0 | 0 | 0 (0 / 0) | 0 | 2 |
| masses4_N3 | 2891 | 2953 | 0 | 62 | 0 | 1 | 0 (0 / 0) | 0 | 61 |
| masses4_N4 | 16050 | 16342 | 7 | 299 | 1 | 25 | 9 (6 / 3) | 0 | 271 |
| masses5_N2 | 370 | 373 | 0 | 3 | 0 | 0 | 0 (0 / 0) | 0 | 3 |
| masses5_N3 | 5060 | 5273 | 0 | 213 | 0 | 12 | 6 (0 / 6) | 0 | 195 |
| total | | | 11 | 610 | 4 | 64 | 19 | 0 | 534 |

Genuine regions in c345cdc that the fix build lacks (a), in full:

| problem | active set | radius | lex feasible | lex margin (min signed first entry) | fix has it | fix walk-only has it |
|---|---|---|---|---|---|---|
| masses3_N6 | [0, 4, 8, 9, 35, 77] | 1.608e-8 | true | 1.0000000000000002 | NO | NO |
| masses3_N7 | [4, 8, 12, 13, 22, 43] | 1.640e-8 | true | 1.0 | NO | NO |
| masses3_N7 | [5, 6, 30, 37, 49, 51, 84] | 1.248e-8 | true | 0.9999999999688641 | NO | NO |
| masses4_N4 | [1, 9, 53, 56] | 1.031e-8 | true | 6.748565110505297 | NO | NO |

Many "undecided" rows in this table are sets of the fix build for which no solver dump was made (only old and a3ebf60 were dumped). They are fix-only sets that the rebuild cannot judge, so (b) is a lower bound here.

## Losses common to old and main

The old-against-main comparison cannot see a genuine region that both lack. Comparing the fix build's sets with the union of old and main on the eight problems that have fix sets, the fix has sets in neither: masses3_N5 6, masses3_N6 7, masses3_N7 14, masses4_N4 10, masses5_N3 4, and none on masses4_N2, masses4_N3, masses5_N2 (41 in all). For example [37, 46, 53] on masses5_N3 (radius 3.9e-8) is absent from c345cdc as well as from main, so it was not a loss of the multiplier rule against c345cdc. These 41 were not run through `pair_check`; they are the fix build's additions (the fix and fix walk-only sets are equal on seven of the eight problems; on masses3_N7 they differ by one set).

## Reading the result

- Main a3ebf60 loses genuine regions that c345cdc has, but only thin ones (radius 1e-8 to 4e-5, all lexico-feasible). 32 of the 36 that could be checked against the fix build return with it; 4 (masses3_N6 1, masses3_N7 2, masses4_N4 1) are lacking in the fix and the fix walk-only builds as well.
- masses4_N5 and masses5_N4 (183 and 136 (a)) have no fix sets, so which of those return is open.
- The large counts of (b) and (c) are sets the sampled coverage checks could not judge; the radii of (c) were not tabulated. (c) sets whose QP at the solver's centre disagrees or is infeasible, and sets the rebuild cannot make full-dimensional, are not evidence of an error in the region set by themselves.
- Timing: main is 3.3 times faster in total than c345cdc on these 153 solves (single runs on shared nodes; the masses problems 3.3 to 4.6 times, the pop problems about equal).

## Timings of the two solves (one 1-CPU task each, same node type)

Total over 153 problems: old 3941 s, new 1214 s (ratio 3.25).

| problem | old s | new s | old / new |
|---|---|---|---|
| masses3_N7 | 185.6 | 40.3 | 4.61 |
| masses4_N4 | 70.1 | 21.2 | 3.30 |
| masses3_N6 | 59.8 | 15.9 | 3.76 |
| masses5_N3 | 31.9 | 9.3 | 3.41 |
| pop_001 | 19.1 | 17.2 | 1.11 |
| pop_028 | 29.2 | 28.9 | 1.01 |
| pop_047 | 14.0 | 13.2 | 1.06 |
| pop_091 | 37.4 | 36.8 | 1.02 |
| masses4_N5 | 1681.3 | 449.5 | 3.74 |
| masses5_N4 | 1601.8 | 404.7 | 3.96 |

The walk-only times were not measured here (see Method).

## (a) in full: genuine regions of c345cdc that main a3ebf60 lacks

Columns: radius of the Chebyshev ball of the rebuilt region, lex margin = smallest signed first significant lexicographic entry over the rows with a vanishing theta part (positive means lexico-feasible), and the verdict of the fix build and of the fix walk-only build for the same set.

| problem | active set | radius | lex feasible | lex margin (min signed first entry) | fix has it | fix walk-only has it |
|---|---|---|---|---|---|---|
| masses3_N5 | [2, 3, 17, 38, 63] | 3.333e-8 | true | 0.9999999991345032 | yes | yes |
| masses3_N5 | [7, 21, 31, 51] | 3.122e-7 | true | 1.0000000043653308 | yes | yes |
| masses3_N6 | [0, 4, 8, 9, 35, 77] | 1.608e-8 | true | 1.0000000000000002 | NO | NO |
| masses3_N6 | [2, 3, 4, 20, 31] | 1.700e-7 | true | 0.9999999984767347 | yes | yes |
| masses3_N6 | [5, 7, 27, 39, 59, 80] | 4.394e-6 | true | 5.311713793774059 | yes | yes |
| masses3_N7 | [0, 6, 9, 10, 49, 79, 91] | 2.700e-7 | true | 0.9999999999541056 | yes | yes |
| masses3_N7 | [3, 4, 8, 13, 22, 43, 89] | 5.809e-8 | true | 0.9999999988493046 | yes | yes |
| masses3_N7 | [3, 6, 7, 8, 43, 76] | 1.140e-7 | true | 1.0 | yes | yes |
| masses3_N7 | [4, 8, 12, 13, 22, 43] | 1.640e-8 | true | 1.0 | NO | NO |
| masses3_N7 | [5, 6, 30, 37, 49, 51, 84] | 1.248e-8 | true | 0.9999999999688641 | NO | NO |
| masses4_N3 | [2, 19, 22] | 1.033e-8 | true | 1.0 | yes | yes |
| masses4_N3 | [7, 31] | 5.761e-8 | true | 8.811973553632093 | yes | yes |
| masses4_N3 | [12, 34, 40] | 1.136e-8 | true | 5.60107892138702 | yes | yes |
| masses4_N3 | [14, 23] | 5.762e-8 | true | 8.811973553632093 | yes | yes |
| masses4_N3 | [19, 38, 40] | 6.525e-8 | true | 6.257972572348081 | yes | yes |
| masses4_N3 | [27, 34, 41] | 3.095e-7 | true | 5.601078921388364 | yes | yes |
| masses4_N4 | [1, 9, 53, 56] | 1.031e-8 | true | 6.748565110505297 | NO | NO |
| masses4_N4 | [2, 3, 14, 31] | 4.451e-8 | true | 1.0 | yes | yes |
| masses4_N4 | [2, 14, 48] | 6.587e-8 | true | 1.0 | yes | yes |
| masses4_N4 | [2, 20, 29] | 3.941e-7 | true | 1.0000000018698438 | yes | yes |
| masses4_N4 | [2, 21, 52] | 3.621e-8 | true | 1.0 | yes | yes |
| masses4_N4 | [2, 28, 37, 62] | 5.948e-7 | true | 0.9999999994926165 | yes | yes |
| masses4_N4 | [3, 10, 43] | 1.767e-6 | true | 1.0 | yes | yes |
| masses4_N4 | [5, 7, 21, 43] | 5.496e-8 | true | 1.0 | yes | yes |
| masses4_N4 | [5, 37, 44] | 4.074e-7 | true | 1.0 | yes | yes |
| masses4_N4 | [6, 7, 21, 27] | 1.022e-7 | true | 1.0 | yes | yes |
| masses4_N4 | [7, 12, 37, 39] | 3.962e-8 | true | 5.661584096407994 | yes | yes |
| masses4_N4 | [7, 14, 26, 64] | 1.877e-8 | true | 5.311603448528331 | yes | yes |
| masses4_N4 | [7, 14, 59] | 5.514e-7 | true | 1.0 | yes | yes |
| masses4_N4 | [13, 37, 59] | 1.999e-7 | true | 5.661584096407993 | yes | yes |
| masses4_N4 | [17, 45, 61] (set not rebuildable, judged at the solver's centre) | 0.000e0 | - | - | yes | yes |
| masses4_N4 | [20, 29, 60] | 2.198e-8 | true | 5.661584096407993 | yes | yes |
| masses4_N4 | [21, 39, 55] | 1.975e-8 | true | 6.362710237771011 | yes | yes |
| masses4_N4 | [21, 43] | 2.516e-7 | true | 6.36271023777101 | yes | yes |
| masses4_N5 | [0, 1, 2, 71, 78] | 2.552e-8 | true | 0.9999999999999999 | n/a | n/a |
| masses4_N5 | [0, 1, 65, 87] | 1.412e-8 | true | 6.238264495789212 | n/a | n/a |
| masses4_N5 | [0, 7, 33, 63, 82] | 1.006e-8 | true | 7.058727027448185 | n/a | n/a |
| masses4_N5 | [0, 28, 55, 61, 86] | 3.445e-8 | true | 5.800840143449477 | n/a | n/a |
| masses4_N5 | [0, 34, 50, 69, 78] | 1.736e-8 | true | 5.800840143463574 | n/a | n/a |
| masses4_N5 | [0, 39, 45, 86] | 2.956e-8 | true | 5.661584096407993 | n/a | n/a |
| masses4_N5 | [1, 2, 27, 86] | 5.570e-7 | true | 0.9999999978790949 | n/a | n/a |
| masses4_N5 | [1, 3, 9, 12, 70] | 3.313e-8 | true | 1.000000000047491 | n/a | n/a |
| masses4_N5 | [1, 4, 8, 30, 55] | 5.619e-7 | true | 0.9999999999663715 | n/a | n/a |
| masses4_N5 | [1, 4, 11, 55, 58] | 1.031e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [1, 4, 11, 55, 86] | 5.048e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [1, 4, 13, 55, 58] | 1.114e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [1, 4, 31, 50, 81] | 1.852e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [1, 4, 31, 66, 81] | 2.695e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [1, 4, 31, 78, 87] | 5.258e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [1, 8, 9, 16, 78] | 1.375e-7 | true | 0.9999999977244338 | n/a | n/a |
| masses4_N5 | [1, 30, 55, 73] | 1.535e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [1, 31, 78, 87] | 5.097e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [1, 47, 66, 76, 78] | 5.043e-6 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 3, 4, 16, 37] | 1.022e-7 | true | 0.9999999957563163 | n/a | n/a |
| masses4_N5 | [2, 3, 4, 21, 39] | 1.226e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 3, 23, 42, 76] | 6.587e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 3, 23, 86] | 2.093e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 4, 19, 63, 84] | 8.565e-8 | true | 0.9999999999334825 | n/a | n/a |
| masses4_N5 | [2, 4, 19, 67] | 1.326e-7 | true | 0.9999999999519946 | n/a | n/a |
| masses4_N5 | [2, 4, 23, 86] | 9.688e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 5, 29, 71, 74] | 1.156e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 8, 23, 26] | 1.033e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 9, 20, 36, 58] | 4.451e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 9, 21, 31, 81] | 7.807e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 9, 23, 76] | 5.995e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 9, 35, 47, 64] | 3.327e-7 | true | 0.9999999981636972 | n/a | n/a |
| masses4_N5 | [2, 19, 63, 65] | 6.725e-8 | true | 6.593132815549325 | n/a | n/a |
| masses4_N5 | [2, 21, 31, 65] | 1.126e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 23, 26] | 1.033e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 27, 55] | 6.531e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 28, 43, 77, 87] | 1.984e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 30, 39, 65] | 3.840e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [2, 39, 46, 86] | 1.024e-6 | true | 1.0 | n/a | n/a |
| masses4_N5 | [3, 4, 12, 38, 41] | 3.560e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [3, 4, 15, 37, 86] | 6.902e-6 | true | 1.0000000000402867 | n/a | n/a |
| masses4_N5 | [3, 4, 16, 29, 87] | 3.267e-8 | true | 5.582176376570715 | n/a | n/a |
| masses4_N5 | [3, 4, 16, 50] | 6.520e-8 | true | 6.362710237771011 | n/a | n/a |
| masses4_N5 | [3, 4, 16, 57, 71] | 3.597e-8 | true | 6.362710237771011 | n/a | n/a |
| masses4_N5 | [3, 4, 19, 30] | 4.188e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [3, 4, 22, 31, 81] | 2.036e-7 | true | 5.661584096407993 | n/a | n/a |
| masses4_N5 | [3, 4, 23, 29, 70] | 1.770e-8 | true | 5.582176376541611 | n/a | n/a |
| masses4_N5 | [3, 4, 23, 36, 45] | 7.717e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [3, 4, 23, 42] | 6.520e-8 | true | 6.362710237771011 | n/a | n/a |
| masses4_N5 | [3, 4, 23, 86] | 3.170e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [3, 4, 27, 45, 87] | 1.338e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [3, 6, 19, 79, 80] | 2.331e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [3, 6, 39, 57, 73] | 3.335e-8 | true | 5.661584096390193 | n/a | n/a |
| masses4_N5 | [3, 7, 9, 15, 39] | 6.652e-8 | true | 0.9999999999999999 | n/a | n/a |
| masses4_N5 | [3, 9, 14, 39, 73] | 9.907e-8 | true | 1.0000000022862325 | n/a | n/a |
| masses4_N5 | [3, 9, 15, 39, 73] | 2.036e-7 | true | 5.661584096407992 | n/a | n/a |
| masses4_N5 | [3, 9, 16, 71, 78] | 4.772e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [3, 14, 41, 55, 85] | 4.843e-7 | true | 5.698408945703704 | n/a | n/a |
| masses4_N5 | [3, 15, 39, 73] | 2.017e-7 | true | 5.66158409640799 | n/a | n/a |
| masses4_N5 | [3, 16, 29, 71] | 1.022e-7 | true | 5.582176376570715 | n/a | n/a |
| masses4_N5 | [3, 16, 71, 78] | 4.755e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [3, 23, 29, 82] | 2.537e-8 | true | 5.582176376271485 | n/a | n/a |
| masses4_N5 | [3, 23, 41] | 1.774e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [3, 27, 45, 87] | 1.332e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [4, 5, 6, 66] | 3.465e-5 | true | 0.9999999999994666 | n/a | n/a |
| masses4_N5 | [4, 6, 8, 39, 42] | 2.411e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [4, 6, 28, 70, 71] | 1.946e-7 | true | 5.311603448535932 | n/a | n/a |
| masses4_N5 | [4, 6, 37, 63, 75] | 3.561e-8 | true | 5.582176376568954 | n/a | n/a |
| masses4_N5 | [4, 6, 39, 42, 86] | 2.151e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [4, 7, 8, 16, 74] | 1.900e-7 | true | 0.9999999997060702 | n/a | n/a |
| masses4_N5 | [4, 7, 8, 19, 79] | 3.342e-6 | true | 0.9999999998797895 | n/a | n/a |
| masses4_N5 | [4, 7, 8, 39, 46] | 1.076e-5 | true | 0.9999999999948681 | n/a | n/a |
| masses4_N5 | [4, 7, 35, 36, 62] | 4.240e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [4, 8, 16, 41, 87] | 1.591e-7 | true | 6.238264495797921 | n/a | n/a |
| masses4_N5 | [4, 8, 22, 31, 69] | 2.124e-7 | true | 0.9999999996231848 | n/a | n/a |
| masses4_N5 | [4, 8, 31, 37, 75] | 4.460e-8 | true | 0.9999999995736253 | n/a | n/a |
| masses4_N5 | [4, 13, 28, 55, 82] (set not rebuildable, judged at the solver's centre) | 0.000e0 | - | - | n/a | n/a |
| masses4_N5 | [4, 13, 28, 70, 71] | 2.489e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [4, 15, 39, 61] | 4.144e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [4, 16, 28, 56, 87] | 1.371e-8 | true | 6.238264495797921 | n/a | n/a |
| masses4_N5 | [4, 16, 37, 46, 77] | 1.740e-8 | true | 0.9999999991946671 | n/a | n/a |
| masses4_N5 | [4, 16, 50] | 5.567e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [4, 16, 50, 78] | 1.016e-8 | true | 5.800840143347159 | n/a | n/a |
| masses4_N5 | [4, 22, 25, 53] | 1.399e-6 | true | 1.0 | n/a | n/a |
| masses4_N5 | [4, 22, 31, 69] | 1.961e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [4, 23, 29, 70] | 3.318e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [4, 23, 29, 82] | 4.182e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [4, 23, 41] | 1.638e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [4, 25, 39, 86] | 1.609e-6 | true | 1.000000000127737 | n/a | n/a |
| masses4_N5 | [4, 27, 28, 71, 73] | 2.088e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [4, 30, 39, 49, 77] | 8.420e-8 | true | 1.0000000009059544 | n/a | n/a |
| masses4_N5 | [4, 30, 46, 55, 77] | 4.174e-7 | true | 5.6010789181462135 | n/a | n/a |
| masses4_N5 | [4, 31, 50, 52, 78] | 6.936e-7 | true | 0.9999999970065651 | n/a | n/a |
| masses4_N5 | [5, 6, 7, 63, 86] | 2.552e-8 | true | 0.9999999999999999 | n/a | n/a |
| masses4_N5 | [5, 8, 31, 81] | 2.720e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [5, 30, 55, 73] | 1.255e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [6, 7, 8, 23, 77] | 1.864e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [6, 7, 16] | 5.995e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [6, 7, 19, 61] | 1.097e-6 | true | 0.9999999978445161 | n/a | n/a |
| masses4_N5 | [6, 7, 22, 62, 76] | 1.800e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [6, 8, 28, 55, 73] | 1.365e-8 | true | 0.9999999999999999 | n/a | n/a |
| masses4_N5 | [6, 9, 19, 46, 76] | 5.851e-8 | true | 5.513727790676057 | n/a | n/a |
| masses4_N5 | [6, 9, 23, 57] | 8.329e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [6, 9, 23, 64, 79] | 1.178e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [6, 19, 63, 81] | 4.167e-8 | true | 6.685333000263199 | n/a | n/a |
| masses4_N5 | [6, 20, 48, 63, 86] | 2.556e-8 | true | 5.800840143450841 | n/a | n/a |
| masses4_N5 | [6, 23] | 5.995e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [6, 29, 55, 58, 76] | 1.110e-8 | true | 5.51372779160738 | n/a | n/a |
| masses4_N5 | [6, 39, 74] | 5.721e-6 | true | 1.0 | n/a | n/a |
| masses4_N5 | [6, 49, 63, 82] | 1.174e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [7, 8, 15, 37, 77] | 1.613e-6 | true | 1.0 | n/a | n/a |
| masses4_N5 | [7, 8, 27, 77, 87] | 4.466e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [7, 9, 12, 57, 71] | 6.725e-8 | true | 6.593132815549325 | n/a | n/a |
| masses4_N5 | [7, 9, 15, 39, 61] | 3.343e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [7, 9, 16, 34] | 1.033e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [7, 14, 39, 57] | 1.126e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [7, 15, 39, 73] | 5.513e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [7, 16, 58] | 6.699e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [7, 16, 65, 77] | 4.609e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [7, 27, 57, 87] | 8.942e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [7, 30, 40, 56, 86] | 1.553e-6 | true | 0.9999999991835212 | n/a | n/a |
| masses4_N5 | [7, 31, 38, 69, 78] | 1.900e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [7, 31, 45, 87] | 6.105e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [8, 9, 16, 50] | 6.520e-8 | true | 6.362710237771011 | n/a | n/a |
| masses4_N5 | [8, 9, 16, 75] | 2.614e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [8, 9, 18, 31, 64] | 4.637e-7 | true | 5.661584096407993 | n/a | n/a |
| masses4_N5 | [8, 9, 22, 31, 78] | 1.494e-7 | true | 5.661584096407992 | n/a | n/a |
| masses4_N5 | [8, 9, 22, 50, 87] | 5.364e-8 | true | 0.9999999987281509 | n/a | n/a |
| masses4_N5 | [8, 9, 23, 29, 60] | 1.022e-7 | true | 1.0000000016111932 | n/a | n/a |
| masses4_N5 | [8, 9, 23, 37, 63] | 1.022e-7 | true | 5.582176376570715 | n/a | n/a |
| masses4_N5 | [8, 9, 23, 66] | 9.658e-8 | true | 0.9999999976793349 | n/a | n/a |
| masses4_N5 | [8, 9, 31, 45, 87] | 6.456e-7 | true | 5.661584096407993 | n/a | n/a |
| masses4_N5 | [8, 9, 35, 63, 65] | 7.983e-7 | true | 0.9999999981702423 | n/a | n/a |
| masses4_N5 | [8, 11, 39, 79] | 9.771e-8 | true | 5.6615840964080055 | n/a | n/a |
| masses4_N5 | [8, 12, 87] | 2.573e-6 | true | 1.0000000000840399 | n/a | n/a |
| masses4_N5 | [8, 16, 28, 84, 87] | 4.171e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [8, 16, 41, 87] | 1.591e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [8, 16, 87] | 1.586e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [8, 21, 37, 47] | 5.341e-7 | true | 5.582176375585726 | n/a | n/a |
| masses4_N5 | [8, 22, 31, 65] | 1.126e-7 | true | 5.661584096407992 | n/a | n/a |
| masses4_N5 | [8, 22, 31, 69] | 2.124e-7 | true | 5.661584096407992 | n/a | n/a |
| masses4_N5 | [8, 22, 31, 87] | 7.593e-8 | true | 5.661584096407992 | n/a | n/a |
| masses4_N5 | [8, 23, 79, 84, 86] | 7.711e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [9, 12, 38, 41] | 3.560e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [9, 12, 41, 87] | 3.083e-6 | true | 1.0000000002983558 | n/a | n/a |
| masses4_N5 | [9, 12, 45, 54] | 3.346e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [9, 12, 55, 71] | 1.119e-8 | true | 6.862126216417948 | n/a | n/a |
| masses4_N5 | [9, 14, 29, 62, 71] | 2.102e-6 | true | 1.0 | n/a | n/a |
| masses4_N5 | [9, 14, 46, 55, 73] | 4.645e-8 | true | 6.063580772107751 | n/a | n/a |
| masses4_N5 | [9, 15, 39, 58] | 4.332e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [9, 15, 39, 61] | 1.961e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [9, 15, 39, 61, 64] | 1.262e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [9, 15, 39, 73] | 1.811e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [9, 16, 41, 87] | 1.588e-7 | true | 1.0000000042360417 | n/a | n/a |
| masses4_N5 | [9, 16, 57, 71] | 3.419e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [9, 16, 71, 76, 78] | 4.531e-8 | true | 5.513727790908888 | n/a | n/a |
| masses4_N5 | [9, 18, 31, 66, 78] | 1.365e-8 | true | 5.661584096407716 | n/a | n/a |
| masses4_N5 | [9, 23, 45, 76] | 1.486e-7 | true | 1.0 | n/a | n/a |
| masses4_N5 | [9, 23, 63, 82] | 2.535e-8 | true | 1.0 | n/a | n/a |
| masses4_N5 | [9, 27, 57, 73, 87] | 2.051e-7 | true | 6.238264495788826 | n/a | n/a |
| masses4_N5 | [9, 31, 38, 76] | 1.424e-6 | true | 1.0 | n/a | n/a |
| masses4_N5 | [9, 31, 38, 78, 85] | 3.934e-7 | true | 1.0000000017372688 | n/a | n/a |
| masses4_N5 | [9, 35, 36, 48, 79] | 7.665e-8 | true | 5.311603448535607 | n/a | n/a |
| masses4_N5 | [9, 35, 47, 82] | 2.511e-7 | true | 6.8621262164178916 | n/a | n/a |
| masses4_N5 | [15, 28, 44, 61, 86] | 3.036e-7 | true | 5.800840143449932 | n/a | n/a |
| masses4_N5 | [15, 28, 61, 86] | 5.282e-7 | true | 5.800840143449932 | n/a | n/a |
| masses4_N5 | [15, 45, 70, 86] | 3.542e-7 | true | 5.800840143449022 | n/a | n/a |
| masses4_N5 | [16, 29, 87] | 2.353e-8 | true | 5.582176376570715 | n/a | n/a |
| masses4_N5 | [16, 50, 62] | 1.492e-8 | true | 6.362710237771012 | n/a | n/a |
| masses4_N5 | [16, 62] | 1.328e-7 | true | 6.362710237771012 | n/a | n/a |
| masses4_N5 | [16, 69, 78, 87] | 1.457e-8 | true | 5.800840143463574 | n/a | n/a |
| masses4_N5 | [21, 30, 55, 68, 77] | 5.806e-8 | true | 5.601078922402998 | n/a | n/a |
| masses4_N5 | [21, 30, 61, 86] | 1.172e-6 | true | 5.601078921391601 | n/a | n/a |
| masses4_N5 | [21, 37, 63, 65, 79] | 1.247e-7 | true | 6.23826449578921 | n/a | n/a |
| masses4_N5 | [22, 31, 78] | 8.817e-8 | true | 5.661584096407992 | n/a | n/a |
| masses4_N5 | [22, 36, 52, 69, 78] | 3.036e-7 | true | 5.800840143449932 | n/a | n/a |
| masses4_N5 | [23, 45] | 5.032e-8 | true | 6.362710237771012 | n/a | n/a |
| masses4_N5 | [23, 63, 70] | 2.256e-8 | true | 6.36271023777051 | n/a | n/a |
| masses4_N5 | [23, 70] | 1.328e-7 | true | 6.362710237771012 | n/a | n/a |
| masses4_N5 | [30, 39, 61, 86] | 2.860e-8 | true | 5.661584096407994 | n/a | n/a |
| masses4_N5 | [31, 38, 48, 78] | 3.851e-7 | true | 5.601078921388364 | n/a | n/a |
| masses4_N5 | [33, 35, 47] | 1.177e-7 | true | 6.862126216417938 | n/a | n/a |
| masses4_N5 | [39, 49, 65, 79] | 2.775e-7 | true | 5.661584096526513 | n/a | n/a |
| masses5_N3 | [2, 15, 49] | 2.154e-6 | true | 0.9999999999831087 | yes | yes |
| masses5_N3 | [2, 19, 55] | 1.037e-7 | true | 1.0 | yes | yes |
| masses5_N4 | [1, 3, 14, 71] | 8.358e-8 | true | 1.0 | n/a | n/a |
| masses5_N4 | [1, 14, 71] | 7.930e-8 | true | 1.0 | n/a | n/a |
| masses5_N4 | [1, 36, 48, 70] | 2.852e-7 | true | 1.0 | n/a | n/a |
| masses5_N4 | [2, 3, 14, 78] | 1.040e-7 | true | 0.9999999976902277 | n/a | n/a |
| masses5_N4 | [2, 3, 17, 28] | 4.188e-7 | true | 0.9999999997151137 | n/a | n/a |
| masses5_N4 | [2, 7, 39, 50] | 7.665e-7 | true | 0.9999999998351009 | n/a | n/a |
| masses5_N4 | [2, 7, 39, 66] | 8.863e-7 | true | 0.999999999910595 | n/a | n/a |
| masses5_N4 | [2, 12, 37, 67] | 3.359e-8 | true | 5.570400345525949 | n/a | n/a |
| masses5_N4 | [2, 16, 59, 70] | 1.256e-8 | true | 1.0 | n/a | n/a |
| masses5_N4 | [2, 38, 44, 70] | 2.449e-8 | true | 5.601015953364342 | n/a | n/a |
| masses5_N4 | [3, 5, 26, 79] (set not rebuildable, judged at the solver's centre) | 0.000e0 | - | - | n/a | n/a |
| masses5_N4 | [3, 6, 14, 67] | 2.502e-8 | true | 1.0 | n/a | n/a |
| masses5_N4 | [3, 8, 42, 58] (set not rebuildable, judged at the solver's centre) | 0.000e0 | - | - | n/a | n/a |
| masses5_N4 | [3, 8, 59, 62] | 1.160e-7 | true | 6.418954056862276 | n/a | n/a |
| masses5_N4 | [3, 14, 36, 40] | 4.797e-8 | true | 1.0000000015569648 | n/a | n/a |
| masses5_N4 | [3, 14, 57] | 1.012e-7 | true | 1.0000000026590419 | n/a | n/a |
| masses5_N4 | [3, 14, 62] | 8.564e-8 | true | 1.0000000014764587 | n/a | n/a |
| masses5_N4 | [3, 17, 36, 69] | 7.473e-8 | true | 1.0 | n/a | n/a |
| masses5_N4 | [3, 20, 49, 56] | 1.680e-7 | true | 6.063236586967301 | n/a | n/a |
| masses5_N4 | [3, 21, 55] | 1.419e-7 | true | 1.0 | n/a | n/a |
| masses5_N4 | [3, 21, 66] | 2.051e-7 | true | 1.0 | n/a | n/a |
| masses5_N4 | [3, 23, 48, 57] | 1.724e-8 | true | 0.9999999960609853 | n/a | n/a |
| masses5_N4 | [3, 25, 27, 79] | 7.591e-8 | true | 1.0 | n/a | n/a |
| masses5_N4 | [3, 30, 37, 60] | 2.683e-8 | true | 5.570400345524831 | n/a | n/a |
| masses5_N4 | [3, 30, 61, 77] | 1.277e-7 | true | 5.661584040260836 | n/a | n/a |
| masses5_N4 | [3, 39, 62, 66] | 2.129e-8 | true | 0.9999999998117927 | n/a | n/a |
| masses5_N4 | [3, 41, 44, 58] | 3.003e-8 | true | 7.6849875154147185 | n/a | n/a |
| masses5_N4 | [3, 54, 68] | 1.043e-6 | true | 1.000000000465415 | n/a | n/a |
| masses5_N4 | [4, 45, 49] | 1.075e-6 | true | 0.9999999999779463 | n/a | n/a |
| masses5_N4 | [5, 7, 21, 46] | 5.239e-8 | true | 1.0 | n/a | n/a |
| masses5_N4 | [5, 18, 47, 68] | 3.509e-8 | true | 1.0 | n/a | n/a |
| masses5_N4 | [5, 37, 48, 71] | 4.720e-8 | true | 1.0 | n/a | n/a |
| masses5_N4 | [6, 7, 14, 62] | 1.413e-7 | true | 1.0000000002633134 | n/a | n/a |
| masses5_N4 | [6, 11, 33, 71] | 2.297e-8 | true | 1.0 | n/a | n/a |
| masses5_N4 | [6, 20, 69, 71] | 2.545e-8 | true | 1.0 | n/a | n/a |
| masses5_N4 | [6, 21, 60] | 5.714e-7 | true | 0.999999999988435 | n/a | n/a |
| masses5_N4 | [6, 21, 68] | 1.040e-7 | true | 0.9999999981206334 | n/a | n/a |
| masses5_N4 | [6, 24, 38] | 4.325e-7 | true | 1.0 | n/a | n/a |
| masses5_N4 | [6, 30, 37, 60] | 1.060e-8 | true | 5.57040034552665 | n/a | n/a |
| masses5_N4 | [6, 30, 58, 60] | 2.231e-8 | true | 1.0 | n/a | n/a |
| masses5_N4 | [6, 39, 42] | 9.572e-8 | true | 1.0 | n/a | n/a |
| masses5_N4 | [6, 41, 57, 68] | 6.669e-7 | true | 7.028444788259119 | n/a | n/a |
| masses5_N4 | [6, 54, 67] | 1.073e-8 | true | 0.9999999999933178 | n/a | n/a |
| masses5_N4 | [7, 12, 59, 61] | 3.037e-7 | true | 1.0000000006626415 | n/a | n/a |
| masses5_N4 | [7, 13, 59, 65] | 2.389e-7 | true | 6.06323658696492 | n/a | n/a |
| masses5_N4 | [7, 14, 56] | 1.274e-7 | true | 1.0 | n/a | n/a |
| masses5_N4 | [7, 14, 57] | 1.012e-7 | true | 0.999999997340958 | n/a | n/a |
| masses5_N4 | [7, 20, 64, 76] | 2.463e-6 | true | 0.999999998802042 | n/a | n/a |
| masses5_N4 | [7, 23, 25, 58] | 1.888e-7 | true | 1.0 | n/a | n/a |
| masses5_N4 | [7, 29, 45, 76] | 1.590e-5 | true | 1.0000000000015021 | n/a | n/a |
| masses5_N4 | [7, 29, 59] | 6.213e-7 | true | 5.601015953364342 | n/a | n/a |
| masses5_N4 | [7, 30, 40, 76] | 4.434e-8 | true | 1.0000000003338265 | n/a | n/a |
| masses5_N4 | [7, 30, 41, 77] | 1.794e-8 | true | 5.661584040260845 | n/a | n/a |
| masses5_N4 | [7, 30, 51] | 2.234e-7 | true | 1.0000000006800818 | n/a | n/a |
| masses5_N4 | [7, 30, 56] | 1.675e-7 | true | 1.0000000001871727 | n/a | n/a |
| masses5_N4 | [7, 30, 61, 77] | 1.025e-7 | true | 5.661584040260837 | n/a | n/a |
| masses5_N4 | [7, 32, 49, 69] | 2.032e-7 | true | 1.0 | n/a | n/a |
| masses5_N4 | [7, 37, 43, 50] | 7.945e-7 | true | 1.0000000001265 | n/a | n/a |
| masses5_N4 | [7, 39, 42, 45] | 9.480e-8 | true | 5.661584040260817 | n/a | n/a |
| masses5_N4 | [7, 44, 77] | 2.107e-7 | true | 1.0000000000919747 | n/a | n/a |
| masses5_N4 | [7, 49, 58, 66] | 5.953e-7 | true | 1.0 | n/a | n/a |
| masses5_N4 | [8, 47, 61, 78] | 2.648e-6 | true | 6.581152814176837 | n/a | n/a |
| masses5_N4 | [9, 47, 53, 75] | 3.009e-7 | true | 6.549545899499208 | n/a | n/a |
| masses5_N4 | [9, 54, 60, 76] | 1.844e-8 | true | 6.747559207258746 | n/a | n/a |
| masses5_N4 | [10, 61, 65, 77] | 1.562e-6 | true | 6.54954589940462 | n/a | n/a |
| masses5_N4 | [12, 37, 48, 76] | 8.394e-8 | true | 5.57040034552595 | n/a | n/a |
| masses5_N4 | [12, 41, 62, 79] | 1.115e-8 | true | 6.063580554736306 | n/a | n/a |
| masses5_N4 | [12, 58, 61, 62] | 1.780e-7 | true | 6.063580554736306 | n/a | n/a |
| masses5_N4 | [13, 38, 46, 77] | 3.915e-8 | true | 5.601015953364342 | n/a | n/a |
| masses5_N4 | [13, 42, 68] | 2.242e-8 | true | 6.06323658696492 | n/a | n/a |
| masses5_N4 | [13, 46, 59, 70] | 8.281e-8 | true | 6.06323658696492 | n/a | n/a |
| masses5_N4 | [14, 27, 65, 78] | 9.745e-8 | true | 5.58204811159757 | n/a | n/a |
| masses5_N4 | [14, 46, 57] | 1.916e-8 | true | 6.064264397605659 | n/a | n/a |
| masses5_N4 | [14, 46, 57, 65] | 2.106e-8 | true | 6.064264397605656 | n/a | n/a |
| masses5_N4 | [14, 51, 67, 70] | 4.634e-8 | true | 6.06426439760589 | n/a | n/a |
| masses5_N4 | [14, 52, 67] | 1.420e-8 | true | 6.064264397605316 | n/a | n/a |
| masses5_N4 | [14, 66, 70, 78] | 3.121e-8 | true | 6.064264397605888 | n/a | n/a |
| masses5_N4 | [16, 25, 65] | 1.550e-8 | true | 6.5495458990335464 | n/a | n/a |
| masses5_N4 | [16, 35, 64] | 1.214e-7 | true | 5.3116033659490824 | n/a | n/a |
| masses5_N4 | [17, 49, 75] | 3.598e-7 | true | 6.549545899499208 | n/a | n/a |
| masses5_N4 | [18, 49, 56, 67] (set not rebuildable, judged at the solver's centre) | 0.000e0 | - | - | n/a | n/a |
| masses5_N4 | [19, 44, 71] | 3.565e-8 | true | 6.581152813974768 | n/a | n/a |
| masses5_N4 | [19, 48, 74] (set not rebuildable, judged at the solver's centre) | 0.000e0 | - | - | n/a | n/a |
| masses5_N4 | [20, 25, 45] | 5.108e-8 | true | 6.063236587254836 | n/a | n/a |
| masses5_N4 | [20, 36, 48] | 5.844e-8 | true | 5.5820481077117465 | n/a | n/a |
| masses5_N4 | [20, 67, 79] | 1.096e-7 | true | 6.063236586964242 | n/a | n/a |
| masses5_N4 | [21, 23, 60, 76] | 1.505e-8 | true | 6.064264397605886 | n/a | n/a |
| masses5_N4 | [21, 28] | 2.062e-8 | true | 5.570400345524831 | n/a | n/a |
| masses5_N4 | [21, 28, 75] | 2.062e-8 | true | 5.570400345524831 | n/a | n/a |
| masses5_N4 | [21, 40, 41, 77] | 4.770e-8 | true | 6.064264397605891 | n/a | n/a |
| masses5_N4 | [21, 41, 60, 77] | 4.634e-8 | true | 6.06426439760589 | n/a | n/a |
| masses5_N4 | [21, 47, 56] | 1.916e-8 | true | 6.06426439760566 | n/a | n/a |
| masses5_N4 | [21, 49, 60] | 1.457e-8 | true | 6.064264397605892 | n/a | n/a |
| masses5_N4 | [21, 68, 75] | 9.883e-8 | true | 6.064264397605891 | n/a | n/a |
| masses5_N4 | [22, 28, 58, 75] (set not rebuildable, judged at the solver's centre) | 0.000e0 | - | - | n/a | n/a |
| masses5_N4 | [22, 30, 57] | 1.070e-8 | true | 5.661584040260853 | n/a | n/a |
| masses5_N4 | [22, 49, 58, 75] | 2.247e-8 | true | 6.5495458990335464 | n/a | n/a |
| masses5_N4 | [23, 39, 45, 65] | 6.407e-8 | true | 5.661584040260853 | n/a | n/a |
| masses5_N4 | [23, 39, 76] | 3.487e-8 | true | 5.661584040260853 | n/a | n/a |
| masses5_N4 | [23, 41, 57, 76] | 1.434e-5 | true | 6.968316110449905 | n/a | n/a |
| masses5_N4 | [26, 30, 72] | 2.741e-8 | true | 5.3116033680779084 | n/a | n/a |
| masses5_N4 | [26, 45, 69, 78] | 4.537e-8 | true | 5.311603365949084 | n/a | n/a |
| masses5_N4 | [27, 39, 61, 65] | 1.519e-8 | true | 5.582048111563899 | n/a | n/a |
| masses5_N4 | [27, 78] | 4.215e-5 | true | 5.582048111563899 | n/a | n/a |
| masses5_N4 | [28, 38, 66, 75] | 2.416e-8 | true | 5.570400345525949 | n/a | n/a |
| masses5_N4 | [28, 51, 59, 75] | 2.075e-8 | true | 6.549545899964869 | n/a | n/a |
| masses5_N4 | [28, 56, 59] | 4.203e-8 | true | 6.86210550923972 | n/a | n/a |
| masses5_N4 | [29, 54, 60, 76] | 1.871e-8 | true | 5.601015953364342 | n/a | n/a |
| masses5_N4 | [29, 54, 76] | 3.602e-8 | true | 5.601015953364342 | n/a | n/a |
| masses5_N4 | [29, 59, 70] | 5.214e-7 | true | 5.601015953364342 | n/a | n/a |
| masses5_N4 | [29, 59, 76] | 3.163e-7 | true | 5.601015953364342 | n/a | n/a |
| masses5_N4 | [30, 37, 65] | 2.329e-8 | true | 5.57040034552665 | n/a | n/a |
| masses5_N4 | [30, 40, 56] | 9.303e-8 | true | 5.661584040260853 | n/a | n/a |
| masses5_N4 | [30, 46, 65] | 6.746e-8 | true | 5.661584040260853 | n/a | n/a |
| masses5_N4 | [30, 46, 78] | 2.386e-8 | true | 5.6615840402613635 | n/a | n/a |
| masses5_N4 | [30, 55, 73] | 3.652e-8 | true | 5.6615840402608715 | n/a | n/a |
| masses5_N4 | [30, 61, 77] | 1.207e-7 | true | 5.661584040260853 | n/a | n/a |
| masses5_N4 | [31, 39, 47] | 1.070e-8 | true | 5.661584040260853 | n/a | n/a |
| masses5_N4 | [31, 47, 79] | 1.581e-6 | true | 7.374365988349854 | n/a | n/a |
| masses5_N4 | [34, 36, 69] | 7.579e-8 | true | 5.582048111562472 | n/a | n/a |
| masses5_N4 | [36, 39, 47] | 8.937e-8 | true | 5.661584040260853 | n/a | n/a |
| masses5_N4 | [36, 42, 79] | 1.813e-7 | true | 5.582048111563899 | n/a | n/a |
| masses5_N4 | [36, 47, 62, 64] | 1.130e-8 | true | 6.4189540568636865 | n/a | n/a |
| masses5_N4 | [37, 49] | 2.193e-7 | true | 5.5704003448582675 | n/a | n/a |
| masses5_N4 | [37, 49, 76] | 1.159e-7 | true | 5.5704003439172665 | n/a | n/a |
| masses5_N4 | [37, 56, 60, 69] | 1.543e-6 | true | 5.570400345525949 | n/a | n/a |
| masses5_N4 | [38, 49, 60] | 5.214e-7 | true | 5.601015953364342 | n/a | n/a |
| masses5_N4 | [39, 45, 63] | 3.652e-8 | true | 5.6615840402608715 | n/a | n/a |
| masses5_N4 | [39, 62, 66] | 1.690e-8 | true | 5.661584040260852 | n/a | n/a |
| masses5_N4 | [39, 63, 65] | 6.248e-8 | true | 5.661584040260876 | n/a | n/a |
| masses5_N4 | [40, 49, 73] | 7.372e-7 | true | 6.740853958191735 | n/a | n/a |
| masses5_N4 | [46, 49, 72] | 1.527e-7 | true | 6.418954056863754 | n/a | n/a |
| masses5_N4 | [54, 56, 76] | 2.579e-6 | true | 6.968316110454907 | n/a | n/a |
| masses5_N4 | [54, 65, 69] | 5.276e-8 | true | 6.549545899397344 | n/a | n/a |
| masses5_N4 | [54, 69, 71] | 1.452e-7 | true | 6.581152814120287 | n/a | n/a |
| masses5_N4 | [54, 69, 76] | 2.590e-7 | true | 6.968316110491287 | n/a | n/a |

## Round 2: coldretry-final 6136b44

Build: 6136b44 (main b98b621 with the dedupe, the box-chain check and the cold retry), release, own target directory on scratch, plus `audit_solve` and `set_audit` (the only files added). One solve each on the ten problems whose old and a3ebf60 sets differ, with solver dumps (`AUDIT_DUMP`); old c345cdc is not re-solved, its sets and dumps are those of round 1. masses4_N5 and masses5_N4 ran with 2 CPUs and 2500 MB; the others with 1 CPU and 1900 MB. The other 143 problems were not re-solved with 6136b44, so nothing is claimed about them.

`set_audit` now splits the spurious class (c) by reason, tested in this order:

- c4: the set cannot be rebuilt and the solver's region has radius zero or no dump entry;
- c1: the region's centre lies inside another region of its own solve (an overlap); the depth is the distance from that centre to the other region's boundary, given with both radii;
- c3: the QP is infeasible at the centre;
- c2: the QP returns another set, which is not in the other solve's list (a QP set in the other solve's list is the duplicate class d).

Sets that cannot be rebuilt are judged at the centre of the solver's own region, and now also take part in the overlap test. An unbuildable region of the same solve is invisible to c1 as the overlapped region, so c1 is a lower bound.

| problem | old c345cdc s | main a3ebf60 s | 6136b44 s |
|---|---|---|---|
| masses3_N5 | 21.8 | 5.0 | 5.0 |
| masses3_N6 | 59.8 | 15.9 | 16.0 |
| masses3_N7 | 185.6 | 40.3 | 39.2 |
| masses4_N2 | 0.2 | 0.1 | 0.1 |
| masses4_N3 | 4.1 | 2.0 | 2.2 |
| masses4_N4 | 70.1 | 21.2 | 23.6 |
| masses4_N5 | 1681.3 | 449.5 | 466.6 |
| masses5_N2 | 0.4 | 0.2 | 0.2 |
| masses5_N3 | 31.9 | 9.3 | 10.5 |
| masses5_N4 | 1601.8 | 404.7 | 443.0 |

Single runs on shared nodes; masses4_N5 and masses5_N4 ran on 2-CPU tasks of the same node type.

### 1. Which of the 355 lost regions come back

(a) of round 1 are the rows of `class3` (old against a3ebf60, re-classified with this `set_audit`); a set comes back if it is in the 6136b44 sets of that problem.

| problem | (a) lost by a3ebf60 | back in 6136b44 | still missing from 6136b44 |
|---|---|---|---|
| masses3_N5 | 2 | 2 | 0 |
| masses3_N6 | 3 | 2 | 1 |
| masses3_N7 | 5 | 3 | 2 |
| masses4_N2 | 0 | 0 | 0 |
| masses4_N3 | 6 | 6 | 0 |
| masses4_N4 | 18 | 17 | 1 |
| masses4_N5 | 183 | 150 | 33 |
| masses5_N2 | 0 | 0 | 0 |
| masses5_N3 | 2 | 2 | 0 |
| masses5_N4 | 136 | 116 | 20 |
| total | 355 | 298 | 57 |

(A set missing from 6136b44 can still be covered by it as a duplicate; the class tables below classify old against 6136b44 directly.)

The four sets that the fix build also lacked:

| problem | set | in 6136b44 | class in old against 6136b44 |
|---|---|---|---|
| masses3_N6 | [0, 4, 8, 9, 35, 77] | NO | a_lost |
| masses3_N7 | [4, 8, 12, 13, 22, 43] | NO | a_lost |
| masses3_N7 | [5, 6, 30, 37, 49, 51, 84] | NO | a_lost |
| masses4_N4 | [1, 9, 53, 56] | NO | a_lost |

Old against 6136b44, classes per problem:

| problem | only old | only 6136b44 | (a) | (b) | (d) | c1 | c2 | c3 | c4 |
|---|---|---|---|---|---|---|---|---|---|
| masses3_N5 | 0 | 7 | 0 | 4 | 0 | 0 | 0 | 3 | 0 |
| masses3_N6 | 1 | 8 | 1 | 8 | 0 | 0 | 0 | 0 | 0 |
| masses3_N7 | 3 | 16 | 2 | 16 | 0 | 0 | 0 | 1 | 0 |
| masses4_N2 | 0 | 2 | 0 | 0 | 2 | 0 | 0 | 0 | 0 |
| masses4_N3 | 0 | 62 | 0 | 34 | 1 | 2 | 18 | 7 | 0 |
| masses4_N4 | 7 | 299 | 1 | 143 | 4 | 3 | 48 | 107 | 0 |
| masses4_N5 | 77 | 1400 | 33 | 538 | 22 | 38 | 95 | 749 | 2 |
| masses5_N2 | 0 | 3 | 0 | 2 | 0 | 0 | 1 | 0 | 0 |
| masses5_N3 | 0 | 213 | 0 | 145 | 2 | 4 | 20 | 42 | 0 |
| masses5_N4 | 26 | 1778 | 20 | 1133 | 28 | 15 | 150 | 452 | 6 |
| total | 114 | 3788 | 57 | 2023 | 59 | 62 | 332 | 1361 | 8 |

Genuine regions of c345cdc that 6136b44 still lacks (a), in full:

| problem | active set | radius | lex margin (min signed first entry) | also lost by a3ebf60 |
|---|---|---|---|---|
| masses3_N6 | [0, 4, 8, 9, 35, 77] | 1.61e-08 | 1.0000000000000002 | yes |
| masses3_N7 | [4, 8, 12, 13, 22, 43] | 1.64e-08 | 1.0 | yes |
| masses3_N7 | [5, 6, 30, 37, 49, 51, 84] | 1.25e-08 | 0.9999999999688641 | yes |
| masses4_N4 | [1, 9, 53, 56] | 1.03e-08 | 6.748565110505297 | yes |
| masses4_N5 | [0, 1, 2, 71, 78] | 2.55e-08 | 0.9999999999999999 | yes |
| masses4_N5 | [0, 28, 55, 61, 86] | 3.45e-08 | 5.800840143449477 | yes |
| masses4_N5 | [0, 7, 33, 63, 82] | 1.01e-08 | 7.058727027448185 | yes |
| masses4_N5 | [1, 4, 11, 55, 58] | 1.03e-08 | 1.0 | yes |
| masses4_N5 | [1, 4, 11, 55, 86] | 5.05e-08 | 1.0 | yes |
| masses4_N5 | [1, 4, 13, 55, 58] | 1.11e-08 | 1.0 | yes |
| masses4_N5 | [1, 4, 31, 50, 81] | 1.85e-08 | 1.0 | yes |
| masses4_N5 | [16, 29, 87] | 2.35e-08 | 5.582176376570715 | yes |
| masses4_N5 | [2, 19, 63, 65] | 6.72e-08 | 6.593132815549325 | yes |
| masses4_N5 | [2, 4, 23, 86] | 9.69e-08 | 1.0 | yes |
| masses4_N5 | [2, 5, 29, 71, 74] | 1.16e-08 | 1.0 | yes |
| masses4_N5 | [22, 36, 52, 69, 78] | 3.04e-07 | 5.800840143449932 | yes |
| masses4_N5 | [3, 16, 71, 78] | 4.75e-08 | 1.0 | yes |
| masses4_N5 | [3, 4, 19, 30] | 4.19e-07 | 1.0 | yes |
| masses4_N5 | [3, 4, 23, 42] | 6.52e-08 | 6.362710237771011 | yes |
| masses4_N5 | [4, 5, 6, 66] | 3.47e-05 | 0.9999999999994666 | yes |
| masses4_N5 | [4, 6, 39, 42, 86] | 2.15e-08 | 1.0 | yes |
| masses4_N5 | [4, 6, 8, 39, 42] | 2.41e-08 | 1.0 | yes |
| masses4_N5 | [4, 7, 8, 16, 74] | 1.90e-07 | 0.9999999997060702 | yes |
| masses4_N5 | [5, 6, 7, 63, 86] | 2.55e-08 | 0.9999999999999999 | yes |
| masses4_N5 | [6, 20, 48, 63, 86] | 2.56e-08 | 5.800840143450841 | yes |
| masses4_N5 | [6, 29, 55, 58, 76] | 1.11e-08 | 5.51372779160738 | yes |
| masses4_N5 | [6, 49, 63, 82] | 1.17e-08 | 1.0 | yes |
| masses4_N5 | [6, 8, 28, 55, 73] | 1.36e-08 | 0.9999999999999999 | yes |
| masses4_N5 | [7, 15, 39, 73] | 5.51e-08 | 1.0 | yes |
| masses4_N5 | [7, 16, 65, 77] | 4.61e-08 | 1.0 | yes |
| masses4_N5 | [8, 12, 87] | 2.57e-06 | 1.0000000000840399 | yes |
| masses4_N5 | [8, 16, 28, 84, 87] | 4.17e-08 | 1.0 | yes |
| masses4_N5 | [8, 22, 31, 69] | 2.12e-07 | 5.661584096407992 | yes |
| masses4_N5 | [9, 15, 39, 61, 64] | 1.26e-07 | 1.0 | yes |
| masses4_N5 | [9, 15, 39, 61] | 1.96e-07 | 1.0 | yes |
| masses4_N5 | [9, 15, 39, 73] | 1.81e-07 | 1.0 | yes |
| masses4_N5 | [9, 18, 31, 66, 78] | 1.36e-08 | 5.661584096407716 | yes |
| masses5_N4 | [1, 3, 14, 71] | 8.36e-08 | 1.0 | yes |
| masses5_N4 | [14, 46, 57, 65] | 2.11e-08 | 6.064264397605656 | yes |
| masses5_N4 | [2, 12, 37, 67] | 3.36e-08 | 5.570400345525949 | yes |
| masses5_N4 | [20, 25, 45] | 5.11e-08 | 6.063236587254836 | yes |
| masses5_N4 | [21, 23, 60, 76] | 1.50e-08 | 6.064264397605886 | yes |
| masses5_N4 | [21, 28, 75] | 2.06e-08 | 5.570400345524831 | yes |
| masses5_N4 | [21, 28] | 2.06e-08 | 5.570400345524831 | yes |
| masses5_N4 | [22, 49, 58, 75] | 2.25e-08 | 6.5495458990335464 | yes |
| masses5_N4 | [28, 51, 59, 75] | 2.08e-08 | 6.549545899964869 | yes |
| masses5_N4 | [28, 56, 59] | 4.20e-08 | 6.86210550923972 | yes |
| masses5_N4 | [29, 59, 76] | 3.16e-07 | 5.601015953364342 | yes |
| masses5_N4 | [3, 14, 36, 40] | 4.80e-08 | 1.0000000015569648 | yes |
| masses5_N4 | [3, 8, 42, 58] (unbuildable, judged at the solver centre) | 7.34e-07 | n/a (unbuildable) | yes |
| masses5_N4 | [37, 49] | 2.19e-07 | 5.5704003448582675 | yes |
| masses5_N4 | [54, 69, 76] | 2.59e-07 | 6.968316110491287 | yes |
| masses5_N4 | [6, 24, 38] | 4.32e-07 | 1.0 | yes |
| masses5_N4 | [6, 30, 37, 60] | 1.06e-08 | 5.57040034552665 | yes |
| masses5_N4 | [7, 14, 57] | 1.01e-07 | 0.999999997340958 | yes |
| masses5_N4 | [7, 30, 51] | 2.23e-07 | 1.0000000006800818 | yes |
| masses5_N4 | [7, 30, 61, 77] | 1.02e-07 | 5.661584040260837 | yes |

### 2. The spurious class split by reason

Old against main a3ebf60 (round 1 data, re-classified):

| problem | only old | only a3ebf60 | (a) | (b) | (d) | c1 | c2 | c3 | c4 |
|---|---|---|---|---|---|---|---|---|---|
| masses3_N5 | 2 | 1 | 2 | 0 | 0 | 0 | 0 | 1 | 0 |
| masses3_N6 | 3 | 1 | 3 | 1 | 0 | 0 | 0 | 0 | 0 |
| masses3_N7 | 6 | 2 | 5 | 2 | 0 | 0 | 0 | 1 | 0 |
| masses4_N2 | 0 | 2 | 0 | 0 | 2 | 0 | 0 | 0 | 0 |
| masses4_N3 | 6 | 62 | 6 | 34 | 1 | 2 | 18 | 7 | 0 |
| masses4_N4 | 27 | 289 | 18 | 135 | 5 | 3 | 48 | 107 | 0 |
| masses4_N5 | 257 | 1298 | 183 | 455 | 24 | 41 | 94 | 757 | 1 |
| masses5_N2 | 0 | 3 | 0 | 2 | 0 | 0 | 1 | 0 | 0 |
| masses5_N3 | 3 | 209 | 2 | 141 | 3 | 4 | 20 | 42 | 0 |
| masses5_N4 | 150 | 1731 | 136 | 1091 | 28 | 15 | 150 | 455 | 6 |
| total | 454 | 3598 | 355 | 1861 | 63 | 65 | 331 | 1370 | 7 |

| class | side | count | radius (min / median / p90 / max) |
|---|---|---|---|
| c1 | old-only | 6 | 1.1e-08 / 5.0e-08 / 1.8e-07 / 1.8e-07 |
| c1 | a3ebf60-only | 59 | 1.1e-08 / 3.7e-08 / 3.3e-07 / 4.5e-07 |
| c2 | old-only | 0 | |
| c2 | a3ebf60-only | 331 | 1.0e-08 / 4.5e-08 / 5.8e-08 / 1.3e-07 |
| c3 | old-only | 86 | 0.0e+00 / 1.2e-08 / 2.6e-08 / 1.5e-07 |
| c3 | a3ebf60-only | 1284 | 0.0e+00 / 4.0e-08 / 4.9e-08 / 2.4e-07 |
| c4 | old-only | 0 | |
| c4 | a3ebf60-only | 7 | 0.0e+00 / 0.0e+00 / 0.0e+00 / 0.0e+00 |

c1 overlaps, old against a3ebf60:

65 rows (6 old-only, 59 a3ebf60-only). Deepest overlap depth: 3.45e-07; rows whose depth exceeds half of the smaller of the two radii: 34.

| problem | side | set | its radius | overlapped region | depth of the centre below that region's boundary | that region's radius |
|---|---|---|---|---|---|---|
| masses4_N5 | a3ebf60-only | [8, 13, 28, 57, 87] | 4.05e-07 | [0, 8, 28, 57, 87] | 3.45e-07 | 4.06e-07 |
| masses4_N5 | a3ebf60-only | [7, 8, 13, 56, 87] | 3.44e-07 | [8, 13, 28, 56, 87] | 3.18e-07 | 5.83e-07 |
| masses4_N5 | a3ebf60-only | [2, 3, 20, 64, 79] | 3.44e-07 | [3, 20, 36, 64, 79] | 3.18e-07 | 5.82e-07 |
| masses4_N5 | a3ebf60-only | [2, 3, 20, 65, 79] | 4.50e-07 | [1, 2, 3, 65, 79] | 2.42e-07 | 5.80e-07 |
| masses4_N5 | a3ebf60-only | [7, 8, 13, 57, 87] | 4.50e-07 | [6, 7, 8, 57, 87] | 2.42e-07 | 5.80e-07 |
| masses4_N5 | a3ebf60-only | [3, 6, 20, 64, 79] | 3.33e-07 | [6, 7, 20, 64, 79] | 2.36e-07 | 5.85e-07 |
| masses5_N4 | a3ebf60-only | [1, 35, 64] | 1.21e-07 | [18, 35, 64] | 1.20e-07 | 1.21e-07 |
| masses4_N5 | old-only | [5, 9, 31, 44, 81] | 1.08e-07 | [9, 17, 31, 44, 81] | 1.08e-07 | 1.08e-07 |
| masses5_N4 | a3ebf60-only | [11, 41, 69, 78] | 1.19e-07 | [12, 41, 69, 78] | 7.02e-08 | 3.12e-07 |
| masses4_N5 | a3ebf60-only | [3, 5, 42, 44, 87] | 6.86e-08 | [3, 7, 42, 44, 87] | 6.16e-08 | 1.75e-07 |
| masses5_N4 | a3ebf60-only | [1, 31, 69, 71] | 8.50e-08 | [4, 31, 69, 71] | 6.03e-08 | 8.49e-08 |
| masses5_N4 | a3ebf60-only | [3, 4, 31, 48] | 1.35e-07 | [3, 4, 5, 48] | 5.08e-08 | 7.24e-07 |

Old against 6136b44:

| class | side | count | radius (min / median / p90 / max) |
|---|---|---|---|
| c1 | old-only | 1 | 1.1e-08 / 1.1e-08 / 1.1e-08 / 1.1e-08 |
| c1 | 6136b44-only | 61 | 4.4e-09 / 3.5e-08 / 1.3e-07 / 4.5e-07 |
| c2 | old-only | 0 | |
| c2 | 6136b44-only | 332 | 1.0e-08 / 4.5e-08 / 5.8e-08 / 1.3e-07 |
| c3 | old-only | 55 | 1.0e-08 / 1.2e-08 / 2.5e-08 / 1.5e-07 |
| c3 | 6136b44-only | 1306 | 0.0e+00 / 4.0e-08 / 4.9e-08 / 2.4e-07 |
| c4 | old-only | 0 | |
| c4 | 6136b44-only | 8 | 0.0e+00 / 0.0e+00 / 0.0e+00 / 0.0e+00 |

c1 overlaps, old against 6136b44:

62 rows (1 old-only, 61 6136b44-only). Deepest overlap depth: 3.45e-07; rows whose depth exceeds half of the smaller of the two radii: 33.

| problem | side | set | its radius | overlapped region | depth of the centre below that region's boundary | that region's radius |
|---|---|---|---|---|---|---|
| masses4_N5 | 6136b44-only | [8, 13, 28, 57, 87] | 4.05e-07 | [0, 8, 28, 57, 87] | 3.45e-07 | 4.06e-07 |
| masses4_N5 | 6136b44-only | [7, 8, 13, 56, 87] | 3.44e-07 | [8, 13, 28, 56, 87] | 3.18e-07 | 5.83e-07 |
| masses4_N5 | 6136b44-only | [2, 3, 20, 64, 79] | 3.44e-07 | [3, 20, 36, 64, 79] | 3.18e-07 | 5.82e-07 |
| masses4_N5 | 6136b44-only | [2, 3, 20, 65, 79] | 4.50e-07 | [1, 2, 3, 65, 79] | 2.42e-07 | 5.80e-07 |
| masses4_N5 | 6136b44-only | [7, 8, 13, 57, 87] | 4.50e-07 | [6, 7, 8, 57, 87] | 2.42e-07 | 5.80e-07 |
| masses4_N5 | 6136b44-only | [3, 6, 20, 64, 79] | 3.33e-07 | [6, 7, 20, 64, 79] | 2.36e-07 | 5.85e-07 |
| masses5_N4 | 6136b44-only | [1, 35, 64] | 1.21e-07 | [18, 35, 64] | 1.20e-07 | 1.21e-07 |
| masses5_N4 | 6136b44-only | [11, 41, 69, 78] | 1.19e-07 | [12, 41, 69, 78] | 7.02e-08 | 3.12e-07 |
| masses4_N5 | 6136b44-only | [3, 5, 42, 44, 87] | 6.86e-08 | [3, 7, 42, 44, 87] | 6.16e-08 | 1.75e-07 |
| masses5_N4 | 6136b44-only | [1, 31, 69, 71] | 8.50e-08 | [4, 31, 69, 71] | 6.03e-08 | 8.49e-08 |
| masses5_N4 | 6136b44-only | [3, 4, 31, 48] | 1.35e-07 | [3, 4, 5, 48] | 5.08e-08 | 7.24e-07 |
| masses4_N3 | 6136b44-only | [4, 25, 47] | 4.88e-08 | [10, 25, 47] | 4.83e-08 | 4.89e-08 |

### 3. The 41 fix-only sets

Sets of the fix build (1f080db) that are in neither old nor a3ebf60. Verdict = class in old against 6136b44, where they are new-only if 6136b44 has them.

| problem | fix-only sets | in 6136b44 | b gained | other class | not in 6136b44 |
|---|---|---|---|---|---|
| masses3_N5 | 6 | 6 | 4 | c3_qp_infeasible 2 | 0 |
| masses3_N6 | 7 | 7 | 7 | - | 0 |
| masses3_N7 | 14 | 14 | 14 | - | 0 |
| masses4_N2 | 0 | | | | |
| masses4_N3 | 0 | | | | |
| masses4_N4 | 10 | 10 | 8 | c3_qp_infeasible 2 | 0 |
| masses5_N2 | 0 | | | | |
| masses5_N3 | 4 | 4 | 4 | - | 0 |
| total | 41 | 41 | 37 | | 0 |


### Reading round 2

- **Lost regions:** 298 of the 355 (a) of a3ebf60 are in the 6136b44 sets (masses4_N5 150 of 183, masses5_N4 116 of 136, masses3_N5 2 of 2, masses3_N6 2 of 3, masses3_N7 3 of 5, masses4_N3 6 of 6, masses4_N4 17 of 18, masses5_N3 2 of 2). 57 are still missing, and old against 6136b44 classifies exactly these 57 as (a): 33 on masses4_N5, 20 on masses5_N4, 4 on the small problems (the four sets of the fix build, all still absent). No region is newly lost relative to a3ebf60. All 57 have radius 1.0e-8 to 3.5e-5 (two above 1e-6: masses4_N5 [4, 5, 6, 66] and [8, 12, 87]) and are lexico-feasible.
- **The four sets** masses3_N6 [0, 4, 8, 9, 35, 77], masses3_N7 [4, 8, 12, 13, 22, 43] and [5, 6, 30, 37, 49, 51, 84], masses4_N4 [1, 9, 53, 56] are still missing, each classified (a): genuine, QP agrees at the centre, centre in no 6136b44 region.
- **c1 (overlap):** 62 rows against 6136b44 (1 old-only, 61 6136b44-only) and 65 against a3ebf60 (6 old-only, 59 a3ebf60-only). The overlapped region is typically one index away from the row's set and about as large (for example masses4_N5 [8, 13, 28, 57, 87], radius 4.05e-7, centre 3.45e-7 inside [0, 8, 28, 57, 87], radius 4.06e-7), so the pairs are near-duplicates of one region emitted under two sets. Overlap depth is at most 3.5e-7 and exceeds half of the smaller radius in about half of the rows. Old c345cdc has such pairs too (6 old-only rows against a3ebf60), so overlaps are not new with the recent changes, and they are not removed by the cold retry (the count did not fall). c1 is a lower bound: a region that cannot be rebuilt cannot be the overlapped one.
- **c2 and c3:** every one is thin. Radii against 6136b44: c2 (6136b44-only) 1.0e-8 to 1.3e-7; c3 old-only 1.0e-8 to 1.5e-7, 6136b44-only up to 2.4e-7 (a few below 1e-12, kept as c3 because the solver radius was positive). c3 is the largest class (1,361 rows against 6136b44), consistent with the QP's answer flipping at the centres of regions of radius 1e-8 to 1e-7. c4 (radius zero or no dump entry) is 8 rows.
- **The 41 fix-only sets:** all 41 are in 6136b44. 37 are (b) genuine gained (masses3_N5 4 of 6, masses3_N6 7, masses3_N7 14, masses4_N4 8 of 10, masses5_N3 4); the other 4 are c3 (QP infeasible at the centre, masses3_N5 2 and masses4_N4 2).
- **Time:** 6136b44 is 0 to 13 percent slower than a3ebf60 on these ten problems (masses4_N5 467 s against 450 s, masses5_N4 443 s against 405 s, single runs).
- **Not covered:** the other 143 problems (identical sets for old and a3ebf60) were not solved with 6136b44.
