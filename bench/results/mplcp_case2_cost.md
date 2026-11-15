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


# Update on main e2d4655: cut 1 gated, cut 2 dropped, census of what is left

The first census above is on a8d003f. Since then the masses8_N2 fix (6fa4a27) moved most `masses` walk calls to rank-1 or the
fast path, so the numbers below are the ones to use. All runs: Negishi, one 1-CPU task per solve or per (problem, alternation),
release builds, builds `base` = main e2d4655, `A` = base + cut 1, `B` = A + cut 2 (Seen before the Gamma tests).

## Gates

* **Tests:** `A` and `B` 144 passed, 0 failed, 3 ignored (147 tests as a 1-CPU array, the same as main).
* **Region hashes:** `region_hash` (rows at 1e-9, and sorted active sets) of `base`, `A` and `B`, 158 problems (the 51 timing problems
  of 2 to 16 parameters plus the large set without the memory hogs pop_002, pop_024, pop_033, pop_036, pop_051, pop_095, masses4_N5,
  masses5_N4): `A` and `B` identical to `base` on 158 of 158. No coverage run needed.
* **Paired timing:** 51 problems, 4 to 5 tasks each, every task runs `base`, `A`, `B` and a second copy of `base` once, shuffled; the
  ratio is taken inside the task and the median over the tasks is shown. `base/A` above 1 means `A` is faster. A/A is `base` against
  its copy. Geometric means of the per-problem medians:

| group (problems) | base/A | base/B | A/B | A/A |
|---|---:|---:|---:|---:|
| all (51) | 1.012 | 1.012 | 0.998 | 1.000 |
| masses (12) | 1.037 | 1.035 | 0.999 | 0.997 |
| not masses (39) | 1.005 | 1.004 | 0.997 | 1.001 |
| 2 to 5 parameters (30) | 0.996 | 0.996 | 0.998 | 1.000 |
| 6 to 16 parameters (21) | 1.037 | 1.034 | 0.998 | 0.999 |

Per problem:

| problem | params | regions | base ms | tasks | base/A | base/B | A/B | A/A (base/base#2) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| di_N10 | 2 | 11 | 0.1 | 5 | 0.978 | 0.985 | 1.000 | 0.993 |
| di_N12 | 2 | 11 | 0.2 | 5 | 1.000 | 1.000 | 1.000 | 1.000 |
| di_N15 | 2 | 11 | 0.2 | 5 | 0.990 | 0.995 | 0.995 | 1.015 |
| di_N20 | 2 | 11 | 0.3 | 5 | 0.986 | 0.993 | 1.003 | 0.986 |
| di_N25 | 2 | 11 | 0.4 | 5 | 0.979 | 0.967 | 0.988 | 1.005 |
| di_N30 | 2 | 11 | 0.6 | 5 | 0.984 | 0.995 | 1.002 | 0.995 |
| di_N4 | 2 | 9 | 0.1 | 5 | 1.016 | 1.000 | 0.985 | 1.000 |
| di_N6 | 2 | 11 | 0.1 | 5 | 0.991 | 1.000 | 1.009 | 1.000 |
| di_N8 | 2 | 11 | 0.1 | 5 | 0.993 | 0.986 | 0.993 | 1.000 |
| di_doc_N3 | 2 | 7 | 0.1 | 5 | 0.984 | 1.000 | 1.000 | 1.033 |
| dinv_N4 | 2 | 9 | 0.1 | 5 | 1.000 | 1.000 | 0.983 | 1.000 |
| dinv_N6 | 2 | 11 | 0.1 | 5 | 1.011 | 0.969 | 0.980 | 1.011 |
| dinv_N8 | 2 | 11 | 0.1 | 5 | 1.000 | 1.000 | 0.991 | 1.008 |
| masses2_N4 | 4 | 307 | 12.0 | 5 | 0.999 | 0.997 | 1.000 | 0.997 |
| masses2_N5 | 4 | 493 | 20.6 | 5 | 1.006 | 1.005 | 0.999 | 0.996 |
| masses2_N6 | 4 | 901 | 41.6 | 5 | 0.990 | 0.982 | 0.994 | 0.998 |
| masses2_N7 | 4 | 1641 | 82.5 | 5 | 0.995 | 0.993 | 1.005 | 0.992 |
| masses2_N8 | 4 | 2837 | 167.9 | 5 | 0.974 | 0.983 | 1.005 | 0.990 |
| masses3_N3 | 6 | 955 | 245.7 | 4 | 1.123 | 1.115 | 0.989 | 0.999 |
| masses3_N4 | 6 | 3643 | 1480.6 | 4 | 1.237 | 1.229 | 0.993 | 1.001 |
| masses4_N3 | 8 | 2891 | 1935.6 | 4 | 1.079 | 1.077 | 1.002 | 0.995 |
| masses5_N2 | 10 | 369 | 263.2 | 4 | 1.033 | 1.034 | 1.002 | 1.000 |
| masses6_N2 | 12 | 379 | 427.9 | 4 | 1.019 | 1.018 | 1.002 | 0.996 |
| masses7_N2 | 14 | 389 | 778.4 | 4 | 1.011 | 1.009 | 1.000 | 0.996 |
| masses8_N2 | 16 | 395 | 1165.7 | 4 | 1.006 | 1.004 | 0.998 | 0.999 |
| pop_005 | 7 | 10289 | 3931.0 | 4 | 1.265 | 1.255 | 0.992 | 1.002 |
| pop_017 | 6 | 4142 | 491.5 | 4 | 1.006 | 1.004 | 0.999 | 1.003 |
| pop_021 | 7 | 679 | 97.5 | 4 | 1.003 | 1.005 | 1.003 | 1.003 |
| pop_027 | 8 | 1472 | 313.8 | 4 | 0.999 | 1.001 | 0.999 | 1.000 |
| rand_t10_x5_m15_s0 | 10 | 1859 | 437.0 | 4 | 1.008 | 1.000 | 0.994 | 1.002 |
| rand_t10_x5_m20_s0 | 10 | 6124 | 1632.0 | 4 | 1.000 | 0.999 | 0.998 | 1.001 |
| rand_t12_x5_m15_s0 | 12 | 1012 | 322.6 | 4 | 0.999 | 0.996 | 0.996 | 0.989 |
| rand_t12_x5_m20_s0 | 12 | 6143 | 3723.3 | 4 | 1.016 | 1.018 | 1.002 | 0.998 |
| rand_t14_x5_m15_s0 | 14 | 2470 | 1326.6 | 4 | 1.002 | 1.000 | 0.999 | 1.002 |
| rand_t16_x5_m15_s0 | 16 | 1876 | 1393.9 | 4 | 1.001 | 0.997 | 0.995 | 1.006 |
| rand_t3_x5_m20_s0 | 3 | 123 | 2.1 | 4 | 1.001 | 0.995 | 0.994 | 0.989 |
| rand_t4_x5_m20_s0 | 4 | 797 | 31.0 | 4 | 0.991 | 0.994 | 1.003 | 1.002 |
| rand_t4_x5_m20_s1 | 4 | 484 | 17.3 | 4 | 0.998 | 0.985 | 0.995 | 0.980 |
| rand_t5_x5_m20_s0 | 5 | 1423 | 157.7 | 4 | 0.999 | 0.999 | 0.998 | 0.997 |
| rand_t5_x5_m20_s1 | 5 | 1136 | 81.8 | 4 | 1.004 | 1.002 | 0.997 | 1.002 |
| rand_t5_x5_m20_s2 | 5 | 2301 | 126.5 | 4 | 1.014 | 0.998 | 0.993 | 0.989 |
| rand_t5_x6_m24_s0 | 5 | 5036 | 466.4 | 4 | 1.004 | 1.002 | 0.992 | 0.998 |
| rand_t6_x5_m20_s0 | 6 | 1774 | 162.2 | 4 | 1.006 | 1.000 | 0.999 | 0.996 |
| rand_t6_x5_m20_s1 | 6 | 5062 | 536.5 | 4 | 1.009 | 1.009 | 0.996 | 1.000 |
| rand_t6_x6_m24_s0 | 6 | 17134 | 2222.1 | 4 | 1.008 | 1.003 | 0.994 | 1.001 |
| rand_t8_x5_m20_s0 | 8 | 4111 | 641.4 | 4 | 0.993 | 0.999 | 1.008 | 1.000 |
| sys3x2_N3 | 3 | 211 | 3.4 | 5 | 1.009 | 1.003 | 0.990 | 1.009 |
| sys3x2_N4 | 3 | 503 | 9.4 | 5 | 1.003 | 1.003 | 0.999 | 0.997 |
| sys3x2_N5 | 3 | 973 | 20.2 | 5 | 1.002 | 1.011 | 1.005 | 1.002 |
| sys3x2_N6 | 3 | 1503 | 38.9 | 5 | 0.981 | 1.037 | 1.037 | 1.025 |
| sys3x2_N7 | 3 | 2095 | 60.6 | 5 | 0.993 | 0.995 | 0.997 | 0.986 |

* Cut 1 gives 1.27x on pop_005, 1.24x on masses3_N4, 1.12x on masses3_N3 and 1.08x on masses4_N3 (A/A 0.995 to 1.001); about 1.0 on the
  rest (rand_t12 1.016, masses5_N2 1.03). 2 to 5 parameters: 0.996, inside the noise of those sub-millisecond to 100 ms problems.
* Cut 2 (Seen lookup before the Gamma tests) changes nothing measurable (A/B 0.998 overall, 0.999 on masses). With the Gamma tests
  already cheap it saves nothing, so it is reverted on the branch and not proposed for merge.

## Walk cost on e2d4655 (clean census, one solve, release; component shares of the solve)

Walk calls (case 3) are now 36% (pop_005), 24% to 42% (masses) and 32% (rand_t12) of the solve; rank-1 calls 3% to 9%.

| problem | walk calls | walk us/call | walk share | Chebyshev LP us/call (share) | DAQP us/call (share) | witness us | Gamma us | pivots, clones, stack, setup, prelude us |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| masses3_N4 | 2,627 | 142 | 28% | 33 (6%) | 57 (11%) | 14 | 13 | 14 |
| masses4_N3 | 3,517 | 129 | 24% | 91 (17%) | 3.7 (1%) | 6 | 6 | 15 |
| masses5_N3 | 14,478 | 316 | 42% | 240 (32%) | 26 (3%) | 11 | 10 | 22 |
| pop_005 | 5,184 | 242 | 36% | 34 (5%) | 127 (19%) | 28 | 22 | 19 |
| rand_t12_x5_m20_s0 | 1,192 | 1,023 | 32% | 898 (28%) | 36 (1%) | 22 | 20 | 28 |

Cut 1 took the Gamma tests from 68 to 178 us per walk call (first census, a8d003f) to 6 to 22 us. Everything that is not an LP or a DAQP
test costs 14 to 28 us per walk call.

## Walk Chebyshev LPs against the parent piece (probe run, `CHEB_PROBE=1`, extra solves not in the timings)

| problem | walk LPs | rows of the piece | rows added to the parent: 0 / 1 / 2 / 3+ | cold pivots | parent point violates (all / added rows) | empty verdicts: share of LPs, of LP time |
|---|---:|---:|---|---:|---|---|
| masses3_N4 | 14,629 | 68 | 660 / 1,113 / 1,108 / 7,844 | 12 | 7.4 / 6.6 | 63%, 69% |
| masses4_N3 | 26,664 | 89 | 1,107 / 2,387 / 1,519 / 17,080 | 21.5 | 10.5 / 9.1 | 76%, 80% |
| masses5_N3 | 102,722 | 153 | 4,613 / 11,342 / 6,274 / 73,283 | 40 | 12.5 / 10.5 | 74%, 81% |
| pop_005 | 17,699 | 73 | 509 / 1,400 / 1,564 / 12,898 | 20 | 3.8 / 2.6 | 28%, 33% |
| rand_t12_x5_m20_s0 | 32,143 | 81 | 223 / 2,190 / 1,450 / 27,199 | 60 | 4.8 / 4.4 | 70%, 75% |

* A child piece is not the parent plus one or two rows: it adds the lexicographic cut rows of the other members of P, 3 or more in
  73% to 85% of the LPs (about 9 to 18 rows on average), and the parent's point violates 3 to 12 of them (it always violates at least one: the LP only
  runs when no witness clears the piece). The parent's optimal point is therefore not feasible for the child, and a dual simplex from the
  parent basis needs at least one pivot per violated cut row (3 to 10 pivots) against 12 to 60 for the cold LP. A parent point exists for
  73% to 97% of the LPs; the parent basis itself is not stored.
* Primal start from the parent point (the existing `lp_point_from` shift): pivots 22 against 24.5 cold (masses4_N3), 33 against 41
  (masses5_N3), time -10%. Same decision and radius on 100% (99% on rand_t12). Not worth building.

By outcome, cold Chebyshev LP against a phase-1 feasibility LP on the rows shifted by 1e-9 ||a|| (an exact test of radius above 1e-9);
pivots, then us per LP:

| problem | outcome | share of LPs | cold | shifted start | phase 1 |
|---|---|---:|---:|---:|---:|
| masses3_N4 | empty | 63% | 16.2, 9.0 | 15.0, 8.2 | 7.2, 3.1 |
| masses3_N4 | nonempty | 37% | 16.7, 6.9 | 14.9, 5.5 | 7.2, 2.3 |
| masses4_N3 | empty | 76% | 24.5, 15.5 | 22.3, 13.8 | 10.4, 5.8 |
| masses4_N3 | nonempty | 24% | 30.5, 11.9 | 30.1, 10.1 | 10.3, 3.7 |
| masses5_N3 | empty | 74% | 40.9, 40.0 | 33.2, 32.6 | 17.2, 13.9 |
| masses5_N3 | nonempty | 26% | 49.8, 26.6 | 45.5, 21.6 | 14.6, 6.8 |
| pop_005 | empty | 28% | 23.5, 12.6 | 19.3, 9.7 | 12.1, 5.4 |
| pop_005 | nonempty | 72% | 21.2, 10.0 | 17.5, 7.5 | 10.2, 4.0 |
| rand_t12_x5_m20_s0 | empty | 70% | 65.8, 38.2 | 57.0, 33.3 | 66.2, 33.8 |
| rand_t12_x5_m20_s0 | nonempty | 30% | 55.1, 28.5 | 52.5, 26.3 | 69.6, 29.7 |

* Leaf against non-leaf: leaf LPs are 3% to 14% of the walk LPs (594 of 22,000 on masses4_N3, 2,384 of 95,500 on masses5_N3) and
  86% to 96% of the LP time is in non-leaf branches, which only need nonemptiness (the leaf centre is the hint).
* A phase-1 pre-test kept as a filter (an empty verdict from phase 1 ends the test, anything else runs the Chebyshev LP as now) saves
  about 60% of an empty LP and costs about 30% of a nonempty LP: net about -45% of the walk LP time on masses4_N3, -48% on masses5_N3,
  -34% on masses3_N4, a loss on pop_005 (72% nonempty) and rand_t12 (phase 1 as long as the full LP at d1 = 10.6). Expected gain on the
  solve: masses5_N3 about 15%, masses4_N3 about 8%, masses3_N4 about 2%, pop_005 and rand_t12 none. Phase 1 agreed with the Chebyshev
  verdict on 100% of LPs (94% on rand_t12, where thin pieces sit at the 1e-9 limit), so an exact filter needs a certificate (Farkas, as the
  DAQP filter has) or the LP as a fallback for borderline cases. Using the phase-1 point as the witness of a nonempty branch does not work: it
  sits on the boundary of the shifted polytope, so it fails the 1e-8 margin test of the children.

## Rows per sub-facet test (4), repeated hyperplanes (5), DAQP per-test cost (6)

| problem | rows per piece | non-redundant rows (exact, every 8th walk LP) | walks | on a hyperplane walked before | from the other side | share of walk time | DAQP tests | build us | solve us | certificate us |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses3_N4 | 123 | 67 of 93 | 2,627 | 63% | 11% | 79% | 48,226 | 1.26 | 1.81 | 0.06 |
| masses4_N3 | 108 | 95 of 108 | 3,517 | 65% | 14% | 52% | 28,191 | 0.14 | 0.36 | 0.01 |
| masses5_N3 | 175 | 128 of 165 | 14,478 | 62% | 10% | 79% | 117,499 | 0.85 | 2.09 | 0.02 |
| pop_005 | 113 | 18 of 78 | 5,184 | 86% | 4% | 95% | 147,412 | 1.68 | 2.58 | 0.08 |
| rand_t12_x5_m20_s0 | 84 | 57 of 84 | 1,192 | 79% | 4% | 96% | 32,654 | 0.14 | 1.29 | 0.01 |

(The two row columns come from different samples: "rows per piece" counts every piece that reaches the DAQP or LP stage, the exact
column samples the walk LPs.)

* **Rows:** on pop_005 77% of the rows of a piece are redundant (18 of 78), on rand_t12 31%, masses3_N4 28%, masses5_N3 22%, masses4_N3
  12%. The redundant rows are mostly the facet polytope's (the same for every branch of a facet), so they could be removed once per
  walk call; DAQP (19% of the solve on pop_005), the witness test and the LP all scale with the row count.
* **Hyperplanes:** 62% to 86% of the walks are on a hyperplane that an earlier walk already handled, but only 4% to 14% were walked from
  the other side. The repeats are mostly other facets of other regions on the same hyperplane (same side), whose pieces differ; no
  direct reuse is visible.
* **DAQP per test** (4.3 us on pop_005): about 40% building the rows (a norm and a column-major copy of all rows, although the rows of the
  facet polytope and the cut rows are unit rows already), 60% the solve, 2% the certificate check. On masses4_N3 and rand_t12 the build is
  0.14 us (short rows) and the solve dominates.

## Levers ranked by expected gain on the solve (nothing below is built)

1. Per-facet removal of the redundant rows of the facet polytope: on pop_005 DAQP + witness + LP are 28% of the solve and 77% of the rows
   are redundant; expected 10% to 14% on pop_005, less elsewhere (22% to 31% redundant). Needs one redundancy pass per walk call
   (m LPs, or the region's clip vertices); exact (same polytope), though the Chebyshev centre can pick another of equal radius.
2. A phase-1 empty-verdict filter before the Chebyshev LP, with a certificate: masses5_N3 about 15%, masses4_N3 about 8%.
3. Incremental DAQP row build (append the cut rows to the facet's normalised rows): about 40% of the DAQP time, 5% to 8% on pop_005
   and masses3_N4; dividing by ||row|| (1 up to rounding) changes the bits of the DAQP input but not the certificate test.
4. Warm primal start from the parent point: 10% of the LP time, about 1% to 3% of the solve. A dual simplex from the parent basis needs 3
   to 10 pivots against 12 to 60 but is a new solver path with a stored basis, comparable in gain to the phase-1 filter.
