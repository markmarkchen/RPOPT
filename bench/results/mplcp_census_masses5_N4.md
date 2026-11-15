### Overview

| problem | algorithm | regions | solve ms | LPs | LPs/region | QPs | LP time / solve | build kept | build dup | build empty |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | mplcp | 44236 | 348074.0 | 3961416 | 89.6 | 18 | 81% | 44244 | 383788 | 0 |

### Share of LP count (and of LP time) by purpose

| problem | algorithm | redundancy | feasibility | neighbour | chebyshev | bbox | init | other |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | mplcp | 51% (10%) | - | 25% (57%) | 4% (24%) | 21% (10%) | 0% (0%) | 0% (0%) |

### Share of total solve time by purpose (LP+QP time) and Amdahl bound 1/(1-f)

| problem | algorithm | LP+QP / solve | redundancy | feasibility | neighbour | chebyshev | bbox | init | other |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | mplcp | 81% | 8% (x1.09) | - | 46% (x1.85) | 19% (x1.24) | 8% (x1.08) | 0% (x1.00) | 0% (x1.00) |

### Region construction (inside gen_cr_*): share of solve time and Amdahl bound if its LPs are cut

| problem | algorithm | construction / solve | of which LP | non-LP part | x if LP -50% | x if LP -80% | x if LP -100% | x if construction free |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | mplcp | 17% | 16% | 1% | 1.09 | 1.14 | 1.19 | 1.20 |

### QP solves by purpose

| problem | algorithm | QPs | by purpose |
|---|---|---:|---|
| masses5_N4 | mplcp | 18 | init 18 (16 infeasible, 0.2 ms) |

### Redundancy removal (rows entering, kept, decided without an LP, tested by LP)

| problem | algorithm | region builds | rows in | rows in / build | kept / build | no LP | LP | LP rows infeasible (redundant) |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | mplcp | 44244 | 4634397 | 104.7 | 34.2 | 2630904 (57%) | 2003493 (43%) | 1853346 |

### Combinatorial candidates

| problem | algorithm | generated | pruned by list | rank deficient | accepted by witness | tested by LP | LP infeasible |
|---|---|---:|---:|---:|---:|---:|---:|

### High-dimension 1: cost per region

| problem | algorithm | regions | solve ms | us/region | LP/region | QP/region | LP+QP share | facets/region | build ms/region |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | mplcp | 44236 | 348074 | 7869 | 89.6 | 0.0 | 81% | 34.2 | 4.22 |

### High-dimension 2: LP calls per region by purpose (time share of solve in brackets)

| problem | algorithm | redundancy | feasibility | neighbour | chebyshev | bbox | init | other |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | mplcp | 45.3 (8%) | - | 22.1 (46%) | 3.2 (19%) | 18.7 (8%) | 0.0 (0%) | 0.3 (0%) |

### High-dimension 3: mplcp neighbour search (per `adjacent` call) and time split

| problem | regions | adjacent/region | no zero row | Case 1 fast | Case 2 | us/call none | us/call fast | us/call Case 2 | Case 2 share of adjacent time | neighbour search | region build | tableau update | rest |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | 44236 | 33.9 | 27% | 24% | 49% | 1.1 | 1.6 | 289.9 | 100% | 63% | 37% | 0% | 0% |

### High-dimension 3b: mplcp candidates that end at a known region

| problem | candidates/region | fast returned | fast already seen | slow returned | slow already seen | adjacent calls with candidates | all seen (fast) | all seen (slow) | clip rows sharing a hyperplane/region |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | 11.3 | 228775 | 94% | 271155 | 62% | 10.0/region | 94% | 65% | 0.0 |

### High-dimension 3c: mplcp `pivot_facet` (degenerate slow path)

| problem | calls/region | Case-1 iters | Case-2 iters | branches | cut by lex_nonneg | Chebyshev LPs | empty facet | repeated basis | depth 0/1/2/3/4/5/6/7+ | depth>6 returns | iteration-limit exits |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|
| masses5_N4 | 3.7 | 32897 | 102769 | 1787170 | 2% | 976961 | 178% | 1% | 29740/34531/5036/4952/4811/6026/6086/72581 | 0 | 0 |

### High-dimension 3d: mplcp facet certificate (RPOPT_CERT=1 skips, 2 audits; asked only for Case-2 facets)

| problem | regions | Case-2 facets/region | certified | search skipped | audit: search also found a new region | check us/facet | check share of solve |
|---|---:|---:|---:|---:|---:|---:|---:|

### High-dimension 3e: mplcp Case-2 leaf pruning, sub-facet hints and walk limits

| problem | regions | LP/region | Chebyshev LPs/region in pivot_facet | leaf branches skipped | leaves kept | leaves with own centre | old hint inside / outside the leaf sub-facet | cycle stops | budget stops | fallbacks found / failed |
|---|---:|---:|---:|---:|---:|---:|---|---:|---:|---|
| masses5_N4 | 44236 | 89.6 | 22.1 | 6018 (15%) | 33357 | 68232 | 7117 / 62050 | 0 | 506 | 576 / 5746 |

### High-dimension 3f: mplcp sub-facet witnesses (non-leaf Case-2 branches accepted without a Chebyshev LP)

| problem | regions | Chebyshev LPs/region in pivot_facet | branches accepted by a witness | of which piece = parent | candidates tried, none cleared | LPs found empty |
|---|---:|---:|---:|---:|---:|---:|
| masses5_N4 | 44236 | 22.1 | 37471 (4%) | 8612 | 1667047 | 1737511 |

### High-dimension 3g: mplcp Case-2 calls by class (rank-1 census)

| problem | Case-2 calls | one slack row, zero pivot, slack columns zero | one slack row, zero pivot, slack columns not zero | one slack row, other | one multiplier row | two rows | three or more rows | time share of the same classes (ms) |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| masses5_N4 | 735947 | 26591 (4%) | 2214 (0%) | 0 (0%) | 0 (0%) | 476 (0%) | 459 (0%) | 177823 / 26958 / 0 / 0 / 13 / 14 |

### High-dimension 3h: mplcp rank-1 calls (one slack row, zero pivot, slack columns zero): |P-|, the candidate set S, depth of the current walk

| problem | calls | |P-| = 0 / 1 / 2 / 3 / 4 / 5 / 6 / 7+ | members of S | already in Seen | calls with all of S in Seen | unseen members with an empty sub-facet | unseen with a nonempty sub-facet | calls with a structural tie | current walk depth 0 / 1 / 2 / 3-5 / 6+ |
|---|---:|---|---:|---:|---:|---:|---:|---:|---|
| masses5_N4 | 26591 | 5213 / 7681 / 7257 / 3664 / 2776 / 0 / 0 / 0 | 44291 | 32853 (74%) | 13459 (51%) | 9095 | 2343 | 0 | 5161 / 19849 / 527 / 379 / 675 |

### High-dimension 3i: mplcp rank-1 enumeration of Case 2

| problem | Case-2 calls | answered by the enumeration | handed to the walk | candidates emitted | members dropped as known | emitted on a witness | sub-facet LPs | members with an empty sub-facet | neighbour LPs/region | distinct active sets tested | repeated tests | of which empty | fast path by relative pivot | pivot measure in the gap | fast path demoted to rank-1 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | 735947 | 706207 | 29740 | 64336 | 875589 | 3414 | 168169 | 159941 | 22.1 | 126217 | 98060 | 96146 | 131330 | 11008 | 0 |

### High-dimension 3j: parametric_daqp active-set tests and region builds

| problem | regions | solve ms | tests/region | LICQ violated | empty | nonempty | empty before any DAQP solve | DAQP solves (stage 1 / stage 2) | LP fallbacks | ms in the tests | of which DAQP | of which LDL | ms in gen_cr_from_kkt | us per test | us per DAQP solve | other ms |
|---|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|

### High-dimension 3k: mplcp sub-facet emptiness tests answered by DAQP

| problem | regions | solve ms | DAQP empty / region | sent on to the LP / region | share answered by DAQP | us per DAQP test | LPs / region |
|---|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | 44236 | 348074 | 21.35 | 4.86 | 81% | 31.49 | 22.1 |

### High-dimension 3l: mplcp Case-2 cost by component (rank-1 = case 2, walk = case 3)

Per `adjacent` call that ended in the case. `us` is microseconds per call, `al` allocations per call. The census overhead (basis_seen, rank-1 probe, tested map, hint check) is taken out of the total and of the rest. Timer cost per lap is in the last column; every lap costs two clock reads and counter updates, so components with many laps are inflated by about that much per lap.

**masses5_N4** (44236 regions, solve 348074 ms, 16.0 Case-2 calls/region, lap cost 63 ns)

| case | component | us/call | al/call | KB/call | laps/call | share of solve |
|---|---|---:|---:|---:|---:|---:|
| rank-1 (2) | **total** (706207 calls, census removed) | 11.58 | 58.6 | 11.5 | - | 2% |
| rank-1 (2) | pre | 1.10 | 4.0 | 2.0 | 1.0 | 0% |
| rank-1 (2) | gate | 0.30 | 1.0 | 0.0 | 1.0 | 0% |
| rank-1 (2) | rank-1 entry/members/lambda | 1.73 | 17.7 | 1.6 | 0.8 | 0% |
| rank-1 (2) | rank-1 facet polytope | 0.95 | 23.0 | 3.5 | 0.2 | 0% |
| rank-1 (2) | rank-1 member cuts | 0.17 | 2.4 | 2.9 | 0.3 | 0% |
| rank-1 (2) | rank-1 output | 0.02 | 0.3 | 0.0 | 0.1 | 0% |
| rank-1 (2) | Seen lookups (leaf) | 1.26 | 8.2 | 0.3 | 1.6 | 0% |
| rank-1 (2) | witness tests | 0.03 | 0.0 | 0.0 | 0.3 | 0% |
| rank-1 (2) | DAQP tests | 0.88 | 0.0 | 0.0 | 0.3 | 0% |
| rank-1 (2) | Chebyshev LPs | 4.43 | 1.3 | 1.1 | 0.2 | 1% |
| rank-1 (2) | rest (untimed code, timer cost) | 0.72 | 0.7 | 0.1 | - | 0% |
| walk (3) | **total** (29740 calls, census removed) | 6877.95 | 663.1 | 5269.0 | - | 59% |
| walk (3) | pre | 1.14 | 4.0 | 2.0 | 1.0 | 0% |
| walk (3) | gate | 0.32 | 1.0 | 0.0 | 1.0 | 0% |
| walk (3) | rank-1 try (failed) | 1.06 | 6.6 | 0.3 | 1.0 | 0% |
| walk (3) | walk setup | 13.04 | 101.8 | 15.0 | 2.0 | 0% |
| walk (3) | walk prelude (P, Z sets) | 10.10 | 44.5 | 16.6 | 6.6 | 0% |
| walk (3) | Case-1 lex choice | 0.10 | 0.4 | 0.3 | 1.1 | 0% |
| walk (3) | Case-2 Gamma tests | 68.07 | 32.4 | 59.5 | 63.5 | 1% |
| walk (3) | pivots | 9.37 | 5.9 | 7.8 | 5.9 | 0% |
| walk (3) | tableau clones | 15.23 | 23.8 | 282.6 | 4.8 | 0% |
| walk (3) | cycle key | 0.79 | 11.5 | 7.2 | 5.9 | 0% |
| walk (3) | Seen lookups (leaf) | 102.27 | 239.5 | 4007.6 | 58.6 | 1% |
| walk (3) | witness tests | 35.59 | 1.0 | 0.1 | 57.0 | 0% |
| walk (3) | DAQP tests | 1211.89 | 0.0 | 0.0 | 57.2 | 10% |
| walk (3) | Chebyshev LPs | 5387.55 | 170.5 | 853.5 | 27.2 | 46% |
| walk (3) | leaf output | 0.65 | 5.2 | 0.5 | 2.3 | 0% |
| walk (3) | stack handling | 1.88 | 4.1 | 6.0 | 17.2 | 0% |
| walk (3) | post-walk | 0.98 | 2.9 | 0.3 | 2.0 | 0% |
| walk (3) | rest (untimed code, timer cost) | 17.93 | 8.3 | 9.7 | - | 0% |


### High-dimension 3m: mplcp walk Chebyshev LPs against the parent piece (warm-start potential)

| problem | walk LPs | polytope rows (child) | rows added: 0 / 1 / 2 / 3+ | cold pivots/LP | with a parent point | child rows the point violates (all / added) | point inside child | cold pivots on those | shifted-start pivots/LP | shifted us/LP | cold us/LP | same decision and radius |
|---|---:|---:|---|---:|---:|---|---:|---:|---:|---:|---:|---:|

### High-dimension 3n: mplcp walk calls by class of vanishing rows (time without census work)

| problem | class | walk calls | share of calls | us/call | share of solve | LP share of class time | DAQP share | Gamma share |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | 1 slack, pivot in gap | 14475 | 49% | 5480 | 23% | 69% | 25% | 1% |
| masses5_N4 | 1 slack, nonzero pivot | 51 | 0% | 23635 | 0% | 56% | 35% | 3% |
| masses5_N4 | 2 z | 476 | 2% | 27 | 0% | 0% | 0% | 0% |
| masses5_N4 | 3 or more | 459 | 2% | 31 | 0% | 0% | 0% | 0% |
| masses5_N4 | 1 slack, zero pivot: symmetry mismatch | 3679 | 12% | 4415 | 5% | 69% | 24% | 2% |
| masses5_N4 | 1 slack, zero pivot: multiplier in the gap band | 10600 | 36% | 10165 | 31% | 87% | 11% | 1% |

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
| masses5_N4 | 735947 | 706207 | 29740 | 135667 | 28295 | 5472 | 25140 | 5919 |

### High-dimension 3j: mplcp tableau rebuilds refused

| problem | regions | `tableau_for` refusals | of which pivot / M_ii above 1e-6 | regions popped without a tableau (neighbour search skipped) | `candidate_law`: fresh tableau refused | `candidate_law`: one-pivot step below 1e-9| bases accepted in the second pivot order | tableaux rebuilt for popped children (tableau budget) | entries set to zero as rounding noise |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | 44236 | 18777 | 0 | 0 | 18032 | 0 | 0 | 17101 | 0 |

### High-dimension 3t: gap-band multipliers recomputed from the original data (Gram solve plus 2 refinement steps)

| problem | gap-band calls | entries in the band | refined nu <= 1e-12 (noise) | refined nu >= 1e-6 (real) | still between | calls with every entry decided | us per recomputation | large entries compared | disagreeing | tableau zeros not zero when refined | refined nu decade of the band entries (1e0 ... 1e-14, below) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|

### High-dimension 3u: refined multipliers in the enumeration, and the audit against the walk

| problem | band entries decided zero | decided real | still between (walk) | audited calls | same neighbour sets | walk has extra sets, all linearly dependent (no region) | walk has other extra sets, no real positive multiplier (walk wandered on a noise multiplier) | walk has other extra sets, a real positive multiplier exists | enumeration has a set the walk lacks | walk incomplete | enumeration undecided (back to the walk) | P- empty with a positive multiplier decided zero | of those handed to the walk | walk calls left |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | 14540 | 0 | 10600 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17168 | 0 | 29740 |

### High-dimension 4: vertex clipping (`clip_flags`) outcomes per call

| problem | algorithm | clip calls/region | ok | nt outside 2..=6 | more than 128 hyperplanes | infinite box | more than 600 vertices | too few vertices | vertex violates a row | redundancy LPs/region |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses5_N4 | mplcp | 1.00 | 0% | 100% | 0% | 0% | 0% | 0% | 0% | 45.3 |

### High-dimension 5: candidates and facet steps per region

| problem | algorithm | regions | candidates tried/region | duplicates/region | neighbour LP+QP/region | feasibility LP/region |
|---|---|---:|---:|---:|---:|---:|
| masses5_N4 | mplcp | 44236 | 2.5 | 8.7 | 22.1 | 0.0 |
