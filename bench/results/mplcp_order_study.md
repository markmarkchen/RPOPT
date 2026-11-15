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

## What makes a masses8_N2 walk cost milliseconds (main ddf0aa8, one census solve each, lp_census build)
"rel" = region reached through a relative-test toggle (basis-only hand-over, tableau rebuilt from the active set).
Per-call means; "LPs" are all LP calls inside the walk call, at the LP time per call shown. All walk calls were refused
by reason 6 except where stated. Node a005 load 84 (default) and a007 load 56 (guard off), so ms are indicative.

masses8_N2, default: 399 regions, solve 2588 ms, 764 walk calls, 1343 ms. 148 rel regions take 1273 ms of region
expansion (8.60 ms each) against 175 ms for the 251 others (0.70 ms each).

| class (rel, finds new regions, completes) | calls | ms | ms/call | pivots | branches | leaves | Chebyshev LPs | LP us | empty | budget hit | Across probes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rel, new, budget hit | 15 | 985 | 65.7 | 63.3 | 485 | 17.5 | 404 | 134 | 90% | 100% | 8.9 |
| rel, new, completes | 8 | 177 | 22.1 | 26.2 | 165 | 5.6 | 144 | 146 | 86% | 0% | 0 |
| not rel, new, budget hit | 2 | 92 | 46.2 | 67.5 | 442 | 19.0 | 384 | 112 | 91% | 100% | 6.5 |
| not rel, new, completes | 3 | 9 | 2.9 | 2.7 | 23 | 1.3 | 16 | 68 | n/a | 0% | 0 |
| rel, no new region | 449 | 58 | 0.13 | 2.0 | 1.0 | 0 | 1.0 | 118 | 10% | 0% | 0 |
| not rel, no new region | 251 | 20 | 0.08 | 2.0 | 1.0 | 0 | 0.9 | 77 | 0% | 0% | 0 |
| reasons 5 and 1 | 36 | 2 | 0.05 | 1.5 | 0.5 | 0 | 0.4 | 92 | n/a | 0% | 0 |

- The cost is bimodal. 25 walks that find new regions take 1254 ms (93% of walk time, 48% of the solve), 22 to 66 ms
  each; the other 739 walks (3 cheap new-region walks and 736 without) cost 0.05 to 0.13 ms each. The mean of 1.8 ms
  hides those few.
- A 66 ms walk is 63 pivots (the 2n budget, hit in 17 of the 25), 485 sub-facet branches, 404 Chebyshev LPs at 134 us
  (82% of the walk) and 90% of those LPs find the sub-facet empty. The Across fallback after the budget costs 0.12 ms
  per walk and `lex_lemke` 2.2 ms in total, so the fallback is not the cost.
- 15 of the 17 budget-hit walks and 23 of the 25 expensive walks start in rel regions; the rel regions are where the
  rank-1 refusal happens (their tableau is rebuilt from the active set, and the recomputed multipliers sit at 1e-10 to 1e-8).
- masses7_N2 shows the same on a smaller scale: 699 walk calls, 253 ms; the 11 expensive walks are 10 in rel regions (8 completing at
  11.5 ms, 2 budget-hit at 35 ms) and 1 not rel (budget-hit, 39 ms); 140 rel regions take 229 ms (1.64 ms each) against 101 ms for 253 others.

With `RPOPT_NO_R6=1` (same binary): masses8_N2 36 walk calls (all reasons 5 and 1, 1.8 ms in total), solve 1207 ms
(0.47x), rel regions 0.26 ms each (38 ms); masses7_N2 28 walk calls (1.2 ms), solve 801 ms against 1072 ms (0.75x),
rel regions 0.18 ms each. So the guard removes every expensive walk on both problems.

## The neighbour search without walks: how much of it rediscovers known regions (main ddf0aa8, release build, one solve each)
Scratch build (no lp_census; two `Instant::now` per `adjacent` call, about 50 ns). Per `adjacent` call: its class
(none = no vanishing row; fast = Case-1 toggle; rank-1 by outcome: kind 0 the multiplier set P- is empty, kind 1 every
member A + {c} - {j} was already in Seen so nothing is returned, kind 2 members tested by witness or LP), whether the
returned neighbour was already known (fast: its toggle active set is in Seen), and the time. "pre" is the time up to
the zero-row decision, which every class pays (zero-row loop, three allocations gam, zrows, rowsc). Node a014/a017,
load 50 to 80, so the us/call are indicative. rand_t12 spends 1.4 s of its 4.3 s in the reason-6 walks (not in the table).

| problem | solve ms | adjacent calls | none: calls, us, share | fast: calls, known, us, share | rank-1 kind 1 (all known): calls, us, share | rank-1 kind 2 (tested): calls, us, share | rank-1 kind 0 share | pre us |
|---|---:|---:|---|---|---|---|---:|---:|
| rand_t8_x5_m20_s0 | 711 | 58.3k | 21.4k, 0.31, 0.9% | 28.6k, 89%, 0.69, 2.8% | 29.6k, 2.46, 10.3% | 1.2k, 13.5, 2.3% | 0.7% | 0.31 to 0.38 |
| rand_t10_x5_m20_s0 | 1890 | 181k | 66.4k, 0.36, 1.3% | 49.9k, 98%, 0.84, 2.2% | 51.7k, 3.45, 9.4% | 5.2k, 18.2, 5.0% | 0.5% | 0.36 to 0.47 |
| pop_017 | 563 | 54.1k | 3.6k, 0.37, 0.2% | 21.3k, 86%, 0.80, 3.0% | 19.0k, 2.40, 8.1% | 7.6k, 9.9, 13.4% | 0.6% | 0.37 to 0.45 |
| rand_t12_x5_m20_s0 | 4277 | 233k | 118.5k, 0.41, 1.1% | 47.1k, 90%, 0.89, 1.0% | 61.1k, 4.32, 6.2% | 1.3k, 28.5, 0.9% | 0.1% | 0.41 to 0.52 |

- Adjacent time (all classes) is 17% (rand_t8), 18% (rand_t10), 25% (pop_017) of the solve; the rest of
  `expand_candidates` (hints, candidate vectors, the Seen claims) is 1.7% to 3.6%.
- Rediscovery dominates the counts: 55% (rand_t8), 56% (rand_t10), 48% (pop_017), 44% (rand_t12) of all `adjacent`
  calls are rank-1 calls with every member known, or fast calls to a known toggle. The rank-1 all-known calls alone cost
  6% to 10% of the solve (2.4 to 4.3 us each, against 0.3 to 0.5 us for the pre-decision part); the known fast calls
  1% to 3%.
- Lever (a), know before searching: the pre-decision part stays (the zero row must be found first), so it can only
  remove the post-decision work of known calls. For the rank-1 all-known calls that is 2.0 to 3.9 us of 2.4 to 4.3 us.
  If the member keys are tested in O(1) with allocation-free active-set keys, the achievable floor is about 1 us per
  call (pre-decision 0.4 us plus the O(n) hypothesis checks), a saving of about 55% to 70% of those calls: 6% (rand_t8),
  6% (rand_t10), 5% (pop_017), 4% (rand_t12) of the solve. For fast calls nothing is left in `adjacent` (0.3 to 0.4 us
  after the decision, no Seen test there); the known ones cost the downstream `seen_claim` and Dup handling, within the
  1.7% to 3.6% above.
- Lever (b), pooling the 33 to 42 allocations per rank-1 call: at about 25 ns each that is about 0.8 to 1.0 us of the 2.4
  to 4.3 us of an all-known call, a third of it: 3% to 4% (rand_t8, rand_t10, pop_017) and 2% (rand_t12) of the solve.
  It overlaps lever (a), so the two together are about the (a) figure, not the sum. Kind-2 calls (9.9 to 28 us) are
  dominated by witnesses and LPs, not allocations; pop_017 is the problem where those matter (13% of the solve).
- The 1.2x to 1.3x loss to `parametric_daqp` is 20% to 30% of the solve; these levers close 4% to 6% of it.
