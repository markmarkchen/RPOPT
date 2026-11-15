# mplcp exploration order: does it drive the Case-2 cost?

Data: Negishi, 1-CPU tasks, scratch-instrumented builds (no trace code in the repo), from the
`worktree-agent-a2cd0480ac0fec818` g/ directory. Builds: base = main c345cdc, cand = 17f22f4 (first scale-free commit),
cand3 = 04d4ca9 (fix). Case numbers: 0 none, 1 fast, 2 rank-1, 3 walk, 4 certificate skip. "All-known" = every returned
neighbour was already in Seen (calls that returned nothing are counted as not all-known).

## Canonical order A/B (candidate list of each region sorted by active set; median of 3 tasks, ms)
| problem | base | base+C | cand | cand+C | cand3 | cand3+C |
|---|---:|---:|---:|---:|---:|---:|
| masses8_N2 | 1598 | 1813 | 2351 | 2675 | 1191 | 1354 |
| masses4_N3 | 3806 | 3911 | 2358 | 2416 | 1978 | 2099 |
| masses5_N3 | 30544 | 32656 | 13218 | 14012 | 11653 | 12798 |
| masses4_N4 | 69.7 s | 71.7 s | 39.1 s | n/a | 39.3 s | 39.3 s |

Canonical order makes every build 0% to 14% slower and leaves the cand/base ratio unchanged on masses8_N2
(1.47 without, 1.48 with). Order is not the cause of the masses8_N2 swing.

## Trace, one solve each (walk = case 3; ms)
| problem | build | walk calls | walk ms | all-known walk calls | all-known walk ms | share of walk ms | share of solve |
|---|---|---:|---:|---:|---:|---:|---:|
| masses8_N2 | base | 530 | 653.5 | 288 | 98.6 | 15% | 5.4% |
| masses8_N2 | cand | 748 | 1505.6 | 530 | 83.8 | 6% | 3.1% |
| masses8_N2 | cand3 | 308 | 208.2 | 226 | 32.8 | 16% | 2.3% |
| masses4_N3 | base | 6069 | 2325 | 4346 | 502 | 22% | 12.8% |
| masses4_N3 | cand3 | 3517 | 595 | 2514 | 74.7 | 13% | 3.6% |
| masses5_N3 | base | 21173 | 25424 | 15409 | 2802 | 11% | 8.6% |
| masses5_N3 | cand3 | 14478 | 5915 | 10134 | 568.7 | 10% | 4.5% |
| masses4_N4 | base | 49903 | 55458 | 37528 | 8055 | 15% | 11.6% |
| masses4_N4 | cand3 | 32534 | 26303 | 23527 | 1023.5 | 4% | 2.6% |

Fast all-known calls cost under 0.5% of the solve. Rank-1 calls never return an all-known set in this trace.

First divergence of the discovery order (masses8_N2, 395 regions, identical region sets): base vs cand and base vs cand3
at region 11; cand vs cand3 at region 201. masses4_N3: base vs cand at region 12, cand vs cand3 at region 282.

## Outcome classes of walk calls (case 3), one census solve each (lp_census build, so ms carry the counter overhead)
Classes: new = no returned neighbour was known; empty = no neighbour returned; allknown = every neighbour known;
mixed = known plus new. "tests/call" = `PfBranches` per walk call (sub-facet tests), "LPs/call" = all LP calls inside
the walk call, "cheb empty" = share of the walk's Chebyshev LPs that found the sub-facet empty. cand3 = 04d4ca9, which
is in main e2d4655. Node a007 (a005 and a007 for base), load 60 to 90 from other jobs, so ms are indicative.

| problem | build | class | calls | ms | share of solve | tests/call | LPs/call | cheb empty |
|---|---|---|---:|---:|---:|---:|---:|---:|
| masses4_N3 | cand3 | new | 219 | 240 | 11% | 48 | 46 | 91% |
| masses4_N3 | cand3 | empty | 470 | 15 | 0.7% | 1.0 | 1.1 | 96% |
| masses4_N3 | cand3 | allknown | 2514 | 87 | 4.0% | 1.7 | 1.7 | 16% |
| masses4_N3 | cand3 | mixed | 314 | 271 | 12.6% | 31 | 30 | 85% |
| masses5_N3 | cand3 | new | 793 | 1587 | 12% | 36 | 34 | 92% |
| masses5_N3 | cand3 | empty | 1936 | 51 | 0.4% | 0.3 | 0.3 | 96% |
| masses5_N3 | cand3 | allknown | 10134 | 622 | 4.8% | 1.7 | 1.7 | 14% |
| masses5_N3 | cand3 | mixed | 1615 | 3754 | 29% | 45 | 42 | 87% |
| masses4_N4 | cand3 | new | 2217 | 8217 | 21% | 87 | 83 | 94% |
| masses4_N4 | cand3 | empty | 3744 | 311 | 0.8% | 2.6 | 2.6 | 97% |
| masses4_N4 | cand3 | allknown | 23527 | 1174 | 2.9% | 2.2 | 2.4 | 34% |
| masses4_N4 | cand3 | mixed | 3046 | 16573 | 41% | 128 | 121 | 93% |
| pop_005 | cand3 | new | 457 | 1964 | 40% | 234 | 219 | 93% |
| pop_005 | cand3 | empty | 1497 | 146 | 2.9% | 6.3 | 6.1 | 95% |
| pop_005 | cand3 | allknown | 3018 | 131 | 2.6% | 3.0 | 3.7 | 62% |
| pop_005 | cand3 | mixed | 212 | 334 | 6.7% | 90 | 85 | 90% |
| rand_t12_x5_m20_s0 | cand3 | new | 32 | 176 | 4.0% | 138 | 129 | 72% |
| rand_t12_x5_m20_s0 | cand3 | empty | 19 | 0.6 | 0.0% | 0 | 0.5 | n/a |
| rand_t12_x5_m20_s0 | cand3 | allknown | 821 | 99 | 2.2% | 2.8 | 2.9 | 36% |
| rand_t12_x5_m20_s0 | cand3 | mixed | 320 | 1175 | 26% | 90 | 81 | 71% |

Empty walks (the boundary case) cost 0% to 3% of the solve, with about 1 to 6 tests each; none of them lies on a row of
the parameter-set constraints (a_t, b_t) within 1e-7, so there is no facet-of-Theta shortcut to take. On main before
04d4ca9 (c345cdc) the empty class was 2.3% (masses4_N3), 1.2% (masses4_N4) and 1% (masses5_N3) of the solve.
The time is in the new and mixed classes (walks that find a new region): 24% to 62% of the solve on the masses family
and 47% on pop_005, with 71% to 94% of their Chebyshev LPs proving a sub-facet empty.

Correction: that "cheb empty" column counted the sub-facets that the DAQP certificate decided as well as the LPs
that ran, so it overstates the LP verdicts. The next section separates them.

## Cheap emptiness proofs in front of the sub-facet LP (walk_one, scratch build on main 054eb58, one solve each)
Outcomes of the sub-facet tests that reach the DAQP check: DAQP-certified empty, LP empty, LP nonempty. Two exact
proofs are counted on every test: the box proof (the box of the root facet F, 2*d1 LPs per Case-2 call, computed
lazily; a new cut row whose minimum over the box exceeds its rhs proves the piece empty) and the pair proof (two
rows nearly opposite with b_i + b_j < 0). Node a005 at load 92, so ms are inflated, ratios are indicative.

| problem | DAQP empty | LP empty | LP nonempty | LP-empty share of LPs run | box decides LP-empty | box decides DAQP-empty | pair adds to box (LP-empty) |
|---|---:|---:|---:|---:|---:|---:|---:|
| masses4_N3 | 1800 | 15918 | 5390 | 75% | 43% | 36% | 4% |
| masses5_N3 | 17668 | 67358 | 24649 | 73% | 19% | 28% | 7% |
| masses4_N4 | 262749 | 274191 | 64506 | 81% | 42% | 49% | 6% |
| pop_005 | 117157 | 4608 | 11763 | 28% | 0% | 52% | 18% |
| rand_t12_x5_m20_s0 | 627 | 21479 | 9445 | 69% | 25% | 71% | 1% |

Cost: a box test is about 1 us; the root box costs about 60 us (14 to 22 LPs) per Case-2 call that reaches an LP
test, 2795 of 3517 walk calls on masses4_N3. Time shares of the solve: LP-empty tests 9% (masses4_N3), 15%
(masses5_N3), 11% (masses4_N4), 1% (pop_005), 19% (rand_t12); DAQP 0.7%, 2.8%, 7.4%, 12%, 1%. The root box costs 6.8%
(masses4_N3), 11% (masses5_N3), 2.3% (masses4_N4), 1.5% (pop_005), 7.2% (rand_t12) of the solve, as much as or more
than the LP time it saves, except on masses4_N4 (net about 2% to 4%). The pair proof is too weak and too costly.

## Why the walks happen: rank1_case2 refuses on a small multiplier (scratch build on main 054eb58)
Each walk call (case 3) is classed by the vanishing rows (k, slack or z), the scale-free pivot measure of each
vanishing slack row, and the reason `rank1_case2` returned None (reason numbers are the order of its `return None`).
One census solve each, node a025 or a032 (load 80 to 100), so ms are inflated.

| problem | walk calls | walk ms | k=1 slack row, zero pivot (rank-1 target) | refused by reason 6 (small multiplier) | reason 5 (mu mismatch) | reason 3 (pivot measure above 1e-9) |
|---|---:|---:|---:|---:|---:|---:|
| masses4_N3 | 3517 | 571 | 3516 | 99.0% of walk ms | 1.0% | 0.0% |
| masses5_N3 | 14478 | 5634 | 14476 | 88.3% | 11.7% | 0.0% |
| masses4_N4 | 32534 | 22827 | 32393 | 96.2% | 2.2% | 1.6% |
| pop_005 | 5184 | 1902 | 5184 | 91.9% | 0.1% | 8.0% |
| rand_t12_x5_m20_s0 | 1192 | 1427 | 1192 | 99.8% | 0 | 0.2% |

Reason 6 is the test `m > PTOL && m <= 1e-7 * max(mu_max, 1)` on a multiplier m = |T[i,kk]| of an active row. The
offending multipliers are small: by walk time the decade of m is 1e-10 (about 60%), 1e-9 (about 30%), 1e-8 and 1e-7
(under 10%), against mu_max between 1e-3 and 1e6. PTOL is 1e-10 and MEASURE_ZERO is 1e-9, so these multipliers are
at the rounding level of the pivot test that already passed. Everything else (k >= 2, a z row vanishing, a nonzero
pivot) is under 1% of the calls on all five problems.

Experiment: the same binary with the reason-6 refusal switched off (`RPOPT_NO_R6=1`, such multipliers join P-), pairs
run in shuffled order, 3 alternations per problem, same node per task. Solve ms, default / switched, medians:

| problem | default | switched | ratio | regions default / switched | walk calls default / switched | region-set hash |
|---|---:|---:|---:|---:|---:|---|
| masses4_N3 | 2115 | 1516 | 0.72 | 2891 / 2891 | 3517 / 51 | identical |
| masses5_N3 | 12318 | 7257 | 0.59 | 5067 / 5068 | 14478 / 87 | one extra region [37,46,53] |
| masses4_N4 | 36833 | 13978 | 0.38 | 16054 / 16054 | 32534 / 1983 | identical |
| pop_005 | 4229 | 2407 | 0.57 | 10289 / 10289 | 5184 / 1674 | identical |
| rand_t12_x5_m20_s0 | 4311 | 2894 | 0.67 | 6143 / 6143 | 1192 / 42 | identical |

`lexcheck 3000` on the switched variant: overlaps 0 and gaps 0 on all five problems (masses5_N3: 5068 regions).
Region counts on masses5_N3 differ between builds anyway (5060, 5067, 5084, 5268), all with 0 gaps and 0 overlaps.
