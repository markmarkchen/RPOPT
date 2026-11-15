# mplcp Case-2 cost by component (census, one solve per problem)

Branch worktree-agent-a98de2306d1951999 (commit f5f13b1 on main a8d003f), `lp_census` table 3l, release build, Negishi
(AMD EPYC 7763), one 1-CPU task per problem (array 45900659), nodes a243/a244 under load 50 to 65. The census adds
about 40 ns per lap (two clock reads and counter updates); laps per call are in the full tables (`bench/results` source:
`~/rpopt/mplcp-case2-cost/out/*.md` on Negishi). Allocations come from a counting wrapper around mimalloc.
Census-only work (basis_seen, rank-1 probe, tested map, hint check) is taken out. Case numbering as in `mplcp-order`:
2 = rank-1 enumeration, 3 = walk (`pivot_facet`). A rank-1 attempt that fails is counted as walk time ("rank-1 try").

## Per Case-2 call (us, allocations) and share of the solve

| problem | rank-1 calls | rank-1 us (al) | rank-1 share | walk calls | walk us (al) | walk share |
|---|---:|---:|---:|---:|---:|---:|
| masses3_N4 | 26,676 | 4.7 (38) | 8% | 2,624 | 232 (841) | 39% |
| masses4_N3 | 31,221 | 4.7 (36) | 6% | 4,233 | 217 (565) | 39% |
| masses5_N3 | 57,042 | 6.9 (42) | 3% | 15,829 | 434 (640) | 52% |
| pop_005 | 50,574 | 4.2 (33) | 5% | 5,184 | 368 (1238) | 47% |
| rand_t12_x5_m20_s0 | 68,359 | 4.7 (37) | 8% | 1,192 | 1065 (1035) | 33% |

## Walk calls, by component (us per walk call, share of the solve)

| component | masses3_N4 | masses4_N3 | masses5_N3 | pop_005 | rand_t12 |
|---|---:|---:|---:|---:|---:|
| Chebyshev LPs | 33 (6%) | 102 (18%) | 272 (33%) | 33 (4%) | 883 (28%) |
| DAQP emptiness tests | 50 (8%) | 21 (4%) | 40 (5%) | 115 (15%) | 38 (1%) |
| Case-2 Gamma tests (lex and cut rows) | 120 (20%) | 68 (12%) | 88 (11%) | 178 (23%) | 94 (3%) |
| witness tests | 6 (1%) | 4 (1%) | 4 (1%) | 13 (2%) | 6 (0%) |
| walk setup (work tableau, ef/ff) | 4.7 | 6.4 | 10.8 | 5.4 | 9.4 |
| prelude (P, Z sets) | 3.8 | 3.4 | 3.9 | 4.5 | 5.5 |
| pivots | 2.4 | 1.9 | 2.6 | 3.1 | 3.6 |
| tableau clones | 2.4 | 2.2 | 2.5 | 3.6 | 3.9 |
| cycle key, Seen, leaf output, stack, post | 2.3 | 2.2 | 2.1 | 3.4 | 9.1 |
| rest | 5.1 | 4.0 | 6.2 | 8.2 | 11.8 |

* Rank-1 calls are cheap (4 to 7 us, 33 to 42 allocations, 3% to 8% of the solve): entry tests 0.6 to 2.6 us, Seen lookups 0.7,
  facet polytope 0.1 to 0.6, LP and DAQP almost never.
* Walk calls are 5 to 12% of the Case-2 calls and 33% to 52% of the solve. Pivots, tableau clones and stack handling are
  each below 1% of the solve (pivot 2 to 4 us per walk call, 2 to 3 pivots per call).
* Sub-facet tests (LP + DAQP + witness) are 18% to 41% of the solve (see table); the Gamma tests are 3% to 23%, and
  they are the one non-test component that matters. They allocate 400 to 1,080 times per walk call (about 40 per branch pair
  loop): per (l, j) pair the code builds `g`, `v`, a normalised row, recomputes `rfull(j, k) / a[j]` and `rfull(l, k) / a[l]`
  (a branchy closure), and clones `ef` per branch.

## Proposed cuts (not built yet)

1. Gamma tests: precompute per iteration the scaled rows r_j = rfull(j, .) / a[j] (and their max over the first 1 + d1
   entries) once for all j in P, then g = r_j - r_l is one subtraction per pair; stage the cut rows in a scratch buffer and
   clone `ef` only for a branch that survives the lexicographic tests. Same values (the same divisions, the same
   subtractions), so the same decisions. Expected: Gamma 3x to 5x cheaper, solve -2% (rand_t12) to -17% (pop_005);
   masses3_N4 -15%, masses4_N3 -9%, masses5_N3 -8%.
2. Look up the leaf in `Seen` before the Gamma tests: a known leaf and a lex-cut branch both `continue` with no output, so
   the order does not change the neighbours (counters PfLexCut / LeafSkip shift). Leaf branches skipped were 31% to 52% of
   leaf branches; expected a further -1% to -5%.
3. Not mine to cut, noted: Chebyshev LPs (18% to 33% on masses4_N3, masses5_N3, rand_t12) and DAQP tests (up to 15% on
   pop_005, 25 tests per walk call) are the remaining walk cost. Fewer calls is an exploration-order question (`mplcp-order`);
   a cheaper LP is `lp-speed`'s. Rank-1 entry allocations (zrows_all, pm, mu, lam, nb) could be pooled for about -1% to -3%.
