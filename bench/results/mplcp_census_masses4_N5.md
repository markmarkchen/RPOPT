### Overview

| problem | algorithm | regions | solve ms | LPs | LPs/region | QPs | LP time / solve | build kept | build dup | build empty |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | mplcp | 68247 | 282490.1 | 5345466 | 78.3 | 312 | 59% | 68387 | 490180 | 0 |

### Share of LP count (and of LP time) by purpose

| problem | algorithm | redundancy | feasibility | neighbour | chebyshev | bbox | init | other |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | mplcp | 62% (15%) | - | 16% (48%) | 3% (25%) | 18% (11%) | 0% (0%) | 1% (0%) |

### Share of total solve time by purpose (LP+QP time) and Amdahl bound 1/(1-f)

| problem | algorithm | LP+QP / solve | redundancy | feasibility | neighbour | chebyshev | bbox | init | other |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | mplcp | 59% | 9% (x1.10) | - | 28% (x1.40) | 15% (x1.18) | 7% (x1.07) | 0% (x1.00) | 0% (x1.00) |

### Region construction (inside gen_cr_*): share of solve time and Amdahl bound if its LPs are cut

| problem | algorithm | construction / solve | of which LP | non-LP part | x if LP -50% | x if LP -80% | x if LP -100% | x if construction free |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | mplcp | 17% | 16% | 2% | 1.08 | 1.14 | 1.19 | 1.21 |

### QP solves by purpose

| problem | algorithm | QPs | by purpose |
|---|---|---:|---|
| masses4_N5 | mplcp | 312 | init 312 (286 infeasible, 3.2 ms) |

### Redundancy removal (rows entering, kept, decided without an LP, tested by LP)

| problem | algorithm | region builds | rows in | rows in / build | kept / build | no LP | LP | LP rows infeasible (redundant) |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | mplcp | 68387 | 6989325 | 102.2 | 22.2 | 3672173 (53%) | 3317152 (47%) | 3210496 |

### Combinatorial candidates

| problem | algorithm | generated | pruned by list | rank deficient | accepted by witness | tested by LP | LP infeasible |
|---|---|---:|---:|---:|---:|---:|---:|

### High-dimension 1: cost per region

| problem | algorithm | regions | solve ms | us/region | LP/region | QP/region | LP+QP share | facets/region | build ms/region |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | mplcp | 68247 | 282490 | 4139 | 78.3 | 0.0 | 59% | 22.2 | 2.09 |

### High-dimension 2: LP calls per region by purpose (time share of solve in brackets)

| problem | algorithm | redundancy | feasibility | neighbour | chebyshev | bbox | init | other |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | mplcp | 48.6 (9%) | - | 12.8 (28%) | 2.4 (15%) | 14.0 (7%) | 0.0 (0%) | 0.5 (0%) |

### High-dimension 3: mplcp neighbour search (per `adjacent` call) and time split

| problem | regions | adjacent/region | no zero row | Case 1 fast | Case 2 | us/call none | us/call fast | us/call Case 2 | Case 2 share of adjacent time | neighbour search | region build | tableau update | rest |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | 68247 | 21.8 | 18% | 30% | 52% | 1.0 | 1.4 | 232.7 | 99% | 66% | 33% | 0% | 0% |

### High-dimension 3b: mplcp candidates that end at a known region

| problem | candidates/region | fast returned | fast already seen | slow returned | slow already seen | adjacent calls with candidates | all seen (fast) | all seen (slow) | clip rows sharing a hyperplane/region |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | 9.3 | 344635 | 92% | 292216 | 59% | 7.9/region | 92% | 61% | 0.0 |

### High-dimension 3c: mplcp `pivot_facet` (degenerate slow path)

| problem | calls/region | Case-1 iters | Case-2 iters | branches | cut by lex_nonneg | Chebyshev LPs | empty facet | repeated basis | depth 0/1/2/3/4/5/6/7+ | depth>6 returns | iteration-limit exits |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|
| masses4_N5 | 4.2 | 57571 | 185657 | 3879916 | 3% | 872719 | 437% | 2% | 46617/54520/7822/7650/8481/12338/11983/140506 | 0 | 0 |

### High-dimension 3d: mplcp facet certificate (RPOPT_CERT=1 skips, 2 audits; asked only for Case-2 facets)

| problem | regions | Case-2 facets/region | certified | search skipped | audit: search also found a new region | check us/facet | check share of solve |
|---|---:|---:|---:|---:|---:|---:|---:|

### High-dimension 3e: mplcp Case-2 leaf pruning, sub-facet hints and walk limits

| problem | regions | LP/region | Chebyshev LPs/region in pivot_facet | leaf branches skipped | leaves kept | leaves with own centre | old hint inside / outside the leaf sub-facet | cycle stops | budget stops | fallbacks found / failed |
|---|---:|---:|---:|---:|---:|---:|---|---:|---:|---|
| masses4_N5 | 68247 | 78.3 | 12.8 | 13019 (17%) | 64803 | 114180 | 14952 / 101458 | 0 | 835 | 1792 / 7147 |

### High-dimension 3f: mplcp sub-facet witnesses (non-leaf Case-2 branches accepted without a Chebyshev LP)

| problem | regions | Chebyshev LPs/region in pivot_facet | branches accepted by a witness | of which piece = parent | candidates tried, none cleared | LPs found empty |
|---|---:|---:|---:|---:|---:|---:|
| masses4_N5 | 68247 | 12.8 | 89149 (9%) | 14469 | 3605166 | 3810907 |

### High-dimension 3g: mplcp Case-2 calls by class (rank-1 census)

| problem | Case-2 calls | one slack row, zero pivot, slack columns zero | one slack row, zero pivot, slack columns not zero | one slack row, other | one multiplier row | two rows | three or more rows | time share of the same classes (ms) |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| masses4_N5 | 769797 | 36348 (5%) | 8039 (1%) | 0 (0%) | 0 (0%) | 786 (0%) | 1444 (0%) | 100350 / 71387 / 0 / 0 / 19 / 45 |

### High-dimension 3h: mplcp rank-1 calls (one slack row, zero pivot, slack columns zero): |P-|, the candidate set S, depth of the current walk

| problem | calls | |P-| = 0 / 1 / 2 / 3 / 4 / 5 / 6 / 7+ | members of S | already in Seen | calls with all of S in Seen | unseen members with an empty sub-facet | unseen with a nonempty sub-facet | calls with a structural tie | current walk depth 0 / 1 / 2 / 3-5 / 6+ |
|---|---:|---|---:|---:|---:|---:|---:|---:|---|
| masses4_N5 | 36348 | 7083 / 7359 / 7161 / 6007 / 4640 / 4098 / 0 / 0 | 78752 | 43819 (56%) | 11144 (31%) | 29814 | 5119 | 0 | 7459 / 26390 / 1050 / 444 / 1005 |

### High-dimension 3i: mplcp rank-1 enumeration of Case 2

| problem | Case-2 calls | answered by the enumeration | handed to the walk | candidates emitted | members dropped as known | emitted on a witness | sub-facet LPs | members with an empty sub-facet | neighbour LPs/region | distinct active sets tested | repeated tests | of which empty | fast path by relative pivot | pivot measure in the gap | fast path demoted to rank-1 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | 769797 | 723180 | 46617 | 64181 | 773607 | 7436 | 162059 | 334938 | 12.8 | 213391 | 185728 | 183565 | 102686 | 14204 | 0 |

### High-dimension 3j: parametric_daqp active-set tests and region builds

| problem | regions | solve ms | tests/region | LICQ violated | empty | nonempty | empty before any DAQP solve | DAQP solves (stage 1 / stage 2) | LP fallbacks | ms in the tests | of which DAQP | of which LDL | ms in gen_cr_from_kkt | us per test | us per DAQP solve | other ms |
|---|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|

### High-dimension 3k: mplcp sub-facet emptiness tests answered by DAQP

| problem | regions | solve ms | DAQP empty / region | sent on to the LP / region | share answered by DAQP | us per DAQP test | LPs / region |
|---|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | 68247 | 282490 | 46.73 | 6.31 | 88% | 19.91 | 12.8 |

### High-dimension 3l: mplcp Case-2 cost by component (rank-1 = case 2, walk = case 3)

Per `adjacent` call that ended in the case. `us` is microseconds per call, `al` allocations per call. The census overhead (basis_seen, rank-1 probe, tested map, hint check) is taken out of the total and of the rest. Timer cost per lap is in the last column; every lap costs two clock reads and counter updates, so components with many laps are inflated by about that much per lap.

**masses4_N5** (68247 regions, solve 282490 ms, 10.6 Case-2 calls/region, lap cost 63 ns)

| case | component | us/call | al/call | KB/call | laps/call | share of solve |
|---|---|---:|---:|---:|---:|---:|
| rank-1 (2) | **total** (723180 calls, census removed) | 9.48 | 59.6 | 9.4 | - | 2% |
| rank-1 (2) | pre | 0.98 | 4.0 | 2.2 | 1.0 | 0% |
| rank-1 (2) | gate | 0.28 | 1.0 | 0.0 | 1.0 | 0% |
| rank-1 (2) | rank-1 entry/members/lambda | 1.05 | 13.9 | 0.8 | 0.8 | 0% |
| rank-1 (2) | rank-1 facet polytope | 0.69 | 21.8 | 2.5 | 0.3 | 0% |
| rank-1 (2) | rank-1 member cuts | 0.25 | 4.8 | 2.7 | 0.6 | 0% |
| rank-1 (2) | rank-1 output | 0.02 | 0.2 | 0.0 | 0.1 | 0% |
| rank-1 (2) | Seen lookups (leaf) | 1.41 | 11.3 | 0.5 | 1.6 | 0% |
| rank-1 (2) | witness tests | 0.05 | 0.0 | 0.0 | 0.6 | 0% |
| rank-1 (2) | DAQP tests | 2.03 | 0.0 | 0.0 | 0.5 | 1% |
| rank-1 (2) | Chebyshev LPs | 1.85 | 1.2 | 0.6 | 0.2 | 0% |
| rank-1 (2) | rest (untimed code, timer cost) | 0.87 | 1.4 | 0.1 | - | 0% |
| walk (3) | **total** (46617 calls, census removed) | 3675.82 | 694.8 | 6607.9 | - | 61% |
| walk (3) | pre | 1.01 | 4.0 | 2.2 | 1.0 | 0% |
| walk (3) | gate | 0.29 | 1.0 | 0.0 | 1.0 | 0% |
| walk (3) | rank-1 try (failed) | 0.92 | 4.9 | 0.3 | 1.0 | 0% |
| walk (3) | walk setup | 9.65 | 68.8 | 7.5 | 2.0 | 0% |
| walk (3) | walk prelude (P, Z sets) | 11.29 | 50.9 | 20.3 | 7.5 | 0% |
| walk (3) | Case-1 lex choice | 0.18 | 1.0 | 0.7 | 1.2 | 0% |
| walk (3) | Case-2 Gamma tests | 99.47 | 40.0 | 81.2 | 87.2 | 2% |
| walk (3) | pivots | 11.62 | 6.6 | 9.6 | 6.6 | 0% |
| walk (3) | tableau clones | 17.15 | 27.1 | 376.3 | 5.4 | 0% |
| walk (3) | cycle key | 0.95 | 12.8 | 8.9 | 6.6 | 0% |
| walk (3) | Seen lookups (leaf) | 135.30 | 334.2 | 5588.4 | 80.9 | 2% |
| walk (3) | witness tests | 79.73 | 1.6 | 0.1 | 78.9 | 1% |
| walk (3) | DAQP tests | 1521.15 | 0.0 | 0.0 | 78.7 | 25% |
| walk (3) | Chebyshev LPs | 1762.22 | 119.1 | 494.6 | 15.2 | 29% |
| walk (3) | leaf output | 0.75 | 6.6 | 0.6 | 2.5 | 0% |
| walk (3) | stack handling | 2.10 | 4.5 | 6.6 | 19.8 | 0% |
| walk (3) | post-walk | 0.60 | 3.1 | 0.3 | 2.0 | 0% |
| walk (3) | rest (untimed code, timer cost) | 21.44 | 8.4 | 10.5 | - | 0% |


### High-dimension 3m: mplcp walk Chebyshev LPs against the parent piece (warm-start potential)

| problem | walk LPs | polytope rows (child) | rows added: 0 / 1 / 2 / 3+ | cold pivots/LP | with a parent point | child rows the point violates (all / added) | point inside child | cold pivots on those | shifted-start pivots/LP | shifted us/LP | cold us/LP | same decision and radius |
|---|---:|---:|---|---:|---:|---|---:|---:|---:|---:|---:|---:|

### High-dimension 3n: mplcp walk calls by class of vanishing rows (time without census work)

| problem | class | walk calls | share of calls | us/call | share of solve | LP share of class time | DAQP share | Gamma share |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | 1 slack, pivot in gap | 28226 | 61% | 4196 | 42% | 48% | 42% | 3% |
| masses4_N5 | 1 slack, nonzero pivot | 679 | 1% | 18009 | 4% | 26% | 60% | 4% |
| masses4_N5 | 2 z | 786 | 2% | 24 | 0% | 0% | 0% | 0% |
| masses4_N5 | 3 or more | 1444 | 3% | 31 | 0% | 0% | 0% | 0% |
| masses4_N5 | 1 slack, zero pivot: symmetry mismatch | 6377 | 14% | 4430 | 10% | 42% | 45% | 3% |
| masses4_N5 | 1 slack, zero pivot: multiplier in the gap band | 9105 | 20% | 1360 | 4% | 83% | 12% | 1% |

### High-dimension 3o: walk Chebyshev LPs, leaf against non-leaf, and the phase-1 feasibility LP for non-leaf branches

| problem | leaf LPs | leaf us/LP | leaf pivots/LP | non-leaf LPs | non-leaf us/LP | non-leaf pivots/LP | non-leaf share of LP time | phase-1 pivots/LP | phase-1 us/LP | phase-1 same verdict |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|

### High-dimension 3q: walk Chebyshev LPs by outcome, cold against shifted start from the parent point and against the phase-1 feasibility LP

| problem | outcome | LPs | share of LPs | share of LP time | cold pivots/LP | cold us/LP | shifted pivots/LP | shifted us/LP | phase-1 pivots/LP | phase-1 us/LP |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|

### High-dimension 3p: region-box prefilter on the pieces that reach the DAQP / LP stage, exact row redundancy, repeated hyperplanes, DAQP cost split

| problem | tests | box proves empty | of those: DAQP empty / LP empty / LP nonempty (must be 0) | rows/piece | rows redundant over the box | cut rows/piece | redundant cut rows | non-redundant rows (exact, sample of walk LPs) | walks | walks on a hyperplane walked before | from the other side | share of walk time in repeats | DAQP tests | build us | solve us | certificate us |
|---|---:|---:|---|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|

### High-dimension 3s: rank-1 multipliers under the scale-free rule (nu = |mu_j| sqrt(M_jj / M_cc), zero below 1e-9, nonzero above 1e-6)

| problem | Case-2 calls | answered by the enumeration | handed to the walk | members the old guard refused, now in P- | refused by the old guard, now zero | tiny (<= PTOL) now in P- | in the gap | mismatch refusals now passed |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | 769797 | 723180 | 46617 | 80213 | 61335 | 1457 | 46569 | 8933 |

### High-dimension 3j: mplcp tableau rebuilds refused

| problem | regions | `tableau_for` refusals | of which pivot / M_ii above 1e-6 | regions popped without a tableau (neighbour search skipped) | `candidate_law`: fresh tableau refused | `candidate_law`: one-pivot step below 1e-9| bases accepted in the second pivot order | tableaux rebuilt for popped children (tableau budget) | entries set to zero as rounding noise |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | 68247 | 24285 | 0 | 0 | 23109 | 0 | 0 | 27805 | 0 |

### High-dimension 3t: gap-band multipliers recomputed from the original data (Gram solve plus 2 refinement steps)

| problem | gap-band calls | entries in the band | refined nu <= 1e-12 (noise) | refined nu >= 1e-6 (real) | still between | calls with every entry decided | us per recomputation | large entries compared | disagreeing | tableau zeros not zero when refined | refined nu decade of the band entries (1e0 ... 1e-14, below) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|

### High-dimension 3u: refined multipliers in the enumeration, and the audit against the walk

| problem | band entries decided zero | decided real | still between (walk) | audited calls | same neighbour sets | walk has extra sets, all linearly dependent (no region) | walk has other extra sets, no real positive multiplier (walk wandered on a noise multiplier) | walk has other extra sets, a real positive multiplier exists | enumeration has a set the walk lacks | walk incomplete | enumeration undecided (back to the walk) | P- empty with a positive multiplier decided zero | of those handed to the walk | walk calls left |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | 37464 | 0 | 9105 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 34398 | 0 | 46617 |

### High-dimension 4: vertex clipping (`clip_flags`) outcomes per call

| problem | algorithm | clip calls/region | ok | nt outside 2..=6 | more than 128 hyperplanes | infinite box | more than 600 vertices | too few vertices | vertex violates a row | redundancy LPs/region |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses4_N5 | mplcp | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 48.6 |

### High-dimension 5: candidates and facet steps per region

| problem | algorithm | regions | candidates tried/region | duplicates/region | neighbour LP+QP/region | feasibility LP/region |
|---|---|---:|---:|---:|---:|---:|
| masses4_N5 | mplcp | 68247 | 2.0 | 7.2 | 12.8 | 0.0 |
