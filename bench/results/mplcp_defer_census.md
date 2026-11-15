# mplcp: deferring Case-2 facets, reverse crossings, active-set bit sets and the hyperplane index (counters only)

Scratch builds on the pre-check tree (main 59b2afb plus the flat Seen table), release profile, Negishi, 1 CPU, one solve
each, no change in what the solver decides except in the deferral variant, which changes the order. The `solve_ms` of
these builds include the cost of the counters and are not used; all times below are the summed times of the `adjacent`
calls (the per-call us include only the call, not the counters). Node load 50 to 100.

## 1. Deferral: every Case-2 facet to a second queue, processed after the main queue is empty

Pass 1 is the usual breadth-first search, but a facet that is neither "no vanishing row" nor a Case-1 toggle is not
searched: its (region, facet, centre) is queued. Pass 2 pops these, rebuilds the region's tableau and searches the
facet (rank-1 or walk). Main queue order is the plain FIFO (no Hilbert key or facet ordering here). "tested" = rank-1
calls that tested members by witness or LP; "all known" = rank-1 calls whose members were all in Seen; "stub" = the
pass-1 call that only decides to defer (0.5 to 1.2 us, and the pre-decision work is paid again in pass 2).

| problem | order | tested calls (with a new region) | tested ms | walk calls (with new) | walk ms | all known ms | stub ms | regions | region sets |
|---|---|---|---:|---|---:|---:|---:|---:|---|
| masses4_N3 | BFS | 6022 (2298) | 97 | 635 (121) | 638 | 44 | | 2947 | |
| | deferred | 4023 (158) | 54 | 637 (111) | 621 | 47 | 36 | 2952 | differ |
| masses5_N3 | BFS | 13075 (6744) | 427 | 1194 (141) | 2488 | 162 | | 5266 | |
| | deferred | 6388 (172) | 185 | 1190 (116) | 2562 | 177 | 113 | 5267 | differ |
| masses4_N4 | BFS | 47597 (15103) | 947 | 9613 (1311) | 8660 | 262 | | 16312 | |
| | deferred | 35656 (1213) | 645 | 9429 (1031) | 9716 | 265 | 226 | 16295 | differ |
| rand_t12_x5_m20_s0 | BFS | 1387 (1191) | 45 | 418 (180) | 667 | 124 | | 6143 | |
| | deferred | 243 (0) | 8 | 314 (47) | 225 | 127 | 42 | 6143 | identical |
| rand_t10_x5_m20_s0 | BFS | 5162 (4817) | 144 | 0 | 0 | 123 | | 6124 | |
| | deferred | 328 (0) | 7 | 0 | 0 | 96 | 35 | 6124 | identical |
| pop_017 | BFS | 7644 (1173) | 72 | 835 (17) | 7 | 26 | | 4142 | |
| | deferred | 6746 (33) | 53 | 841 (16) | 7 | 27 | 17 | 4142 | identical |
| rand_t8_x5_m20_s0 | BFS | 1233 (838) | 19 | 0 | 0 | 36 | | 4111 | |
| | deferred | 434 (0) | 5 | 0 | 0 | 36 | 17 | 4111 | identical |

- Space-filling first does what the idea predicts: the tested Case-2 calls that find a new region fall from 838 to 15,103
  per problem to 0 to 1,213 (rand_t8, rand_t10, rand_t12: to 0), because the Case-1 facets have already filled the space.
- The saving is small, though. The tested calls that remain still run their witness or LP tests (13 to 33 us each),
  so their ms fall by 1% to 3% of the solve, except rand_t10 (144 to 7 ms) and the rand_t12 walks (667 to 225 ms, the
  walks that find new regions fall from 180 to 47). The masses4_N4 walks get dearer (8.7 s to 9.7 s) and the walks of
  the masses problems stay: 635, 1194 and 9613 walk calls in BFS against 637, 1190 and 9429 deferred, with the same
  cost per call; they are the rank-1 cases refused for the multiplier gap band and still find new regions in one call of five to one of
  eight.
- Net of the stub calls, before any tableau rebuild: masses4_N3 -22 ms, masses5_N3 -40 ms, masses4_N4 +983 ms, rand_t12
  -433 ms, rand_t10 -128 ms, pop_017 0, rand_t8 +3 ms. The measured variant rebuilds the tableau once per deferred facet
  (the item is one facet); grouped by region it would be one rebuild per region, which still costs about as much as the
  gain on rand_t10.
- Region sets: identical to BFS on the rand and pop problems; on masses4_N3 (2947 to 2952), masses5_N3 (5266 to 5267)
  and masses4_N4 (16312 to 16295) the order changes the degenerate splits (not checked for gaps).
- Memory: a queued (region, facet, centre) is 48 bytes plus the centre. Peak 37,915 items (4.2 MB) on masses4_N3,
  97,991 (12.5 MB) on masses5_N3, 176,551 (19.8 MB) on masses4_N4, 69,551 (10.0 MB) on rand_t12. Grouped by region
  (a region index and a facet bit set) it would be below 1 MB.
- Verdict: deferral by itself is not worth building. It helps only where the Case-2 facets are what finds the regions
  (rand_t10 about 4%, rand_t12 up to 10% before rebuild costs), costs time on masses4_N4, and changes the region sets of
  the degenerate masses problems.

## 2. The all-candidates-known test, and the active-set bit sets

For every rank-1 facet the scan lists the known regions A' with c in A' and |A xor A'| = 2 and |A'| = |A| (the swaps
A + {c} - {j}), over all regions known at that time (bit sets of the active sets, XOR and popcount).

| problem | rank-1 all-known facets | sum of |P-| | known swaps found | of which j in P- | j not in P- | known A + {c} (|xor| = 1) |
|---|---:|---:|---:|---:|---:|---:|
| rand_t8_x5_m20_s0 | 29,646 | 38,787 | 55,059 | 38,787 | 16,272 | 0 |
| rand_t10_x5_m20_s0 | 51,685 | 74,117 | 106,738 | 74,117 | 32,621 | 0 |
| rand_t12_x5_m20_s0 | 61,755 | 90,729 | 131,731 | 90,729 | 41,002 | 0 |
| pop_017 | 19,009 | 32,861 | 59,136 | 32,815 | 26,321 | 0 |
| masses4_N3 | 22,100 | 31,082 | 39,169 | 26,578 | 12,591 | 0 |

- On the rand problems the members in P- are exactly the known swaps with j in P- (sum |P-| equals the count), so "every
  member of the dependency is a known region" is exactly what the scan finds. On pop_017 and masses the count is a little
  below sum |P-| (32,815 of 32,861; 26,578 of 31,082) because some members were seen as rejected (empty) sets and are not
  regions.
- 24% to 45% of the known |xor| = 2 sets are swaps whose j is not in P- (multiplier of the wrong sign or zero): they
  contain c and differ from A by two indices, but do not border the facet. The bit-set scan alone over-counts; the sign of
  the multiplier decides. The scan over all regions is O(regions) per facet; it was used for the census only.

## 3. Facet-hyperplane index (a known region holding the opposite row, normalised to 1e-7)

Share of the facets with 0, 1 or 2 or more other regions that have the same hyperplane with the opposite orientation.

| class | masses4_N3 | masses5_N3 | masses4_N4 | rand_t8 | rand_t10 | rand_t12 | pop_017 |
|---|---|---|---|---|---|---|---|
| Case-1 facet: exactly one | 76% | 71% | 76% | 89% | 98% | 90% | 77% |
| Case-1 facet: none | 22% | 26% | 21% | 11% | 2% | 10% | 12% |
| rank-1 all known: one | 38% | 38% | 28% | 0.1% | 0.03% | 0.03% | 33% |
| rank-1 all known: 2 or more | 60% | 51% | 70% | 100% | 100% | 100% | 67% |

At 6 to 16 parameters the hyperplane index names the neighbour of a Case-1 facet in 71% to 98% of the cases, and the
unknown ones (no match) are the facets that create a region; on a rank-1 facet the same hyperplane belongs to several
regions (2 or more matches in 51% to 100%), so it cannot identify the neighbour there.

## 4. Reverse crossing of Case-1 facets (done-mark)

Known Case-1 neighbours found from S, by whether the region R is still queued (the mark would save R's later call) and
by the Lean condition on S's vanishing rows (single vanishing row; every other vanishing row inert; not inert):

| problem | single row, R queued | single row, R popped | others inert | not inert | ceiling of the saving (queued calls x 0.8 us) |
|---|---:|---:|---:|---:|---:|
| masses4_N3 | 5,444 | 7,691 | 0 | 0 | 0.2% |
| masses5_N3 | 11,855 | 20,680 | 0 | 0 | about 0.1% |
| masses4_N4 | 37,265 | 55,778 | 0 | 0 | about 0.1% |
| pop_017 | 9,163 | 9,184 | 0 | 1 | 1.4% |
| rand_t8 | 12,644 | 12,644 | 0 | 0 | 1.5% |
| rand_t10 | 24,498 | 24,498 | 0 | 0 | 1.2% |
| rand_t12 | 21,120 | 21,120 | 0 | 0 | 0.6% |

Exactly half of the known Case-1 crossings leave R still queued (the other half are the second visit of the pair), and
the Lean condition holds on all but one of them. The saving is at most the cost of the queued calls, 0.1% to 1.4% of the
solve, before the bookkeeping to mark an item and to match the facet by hyperplane at the pop.

## 5. Index-first lookup (naming the neighbour by row index) and the all-swaps test, on the merged pre-check tree

Counters only (scratch build, release profile, Negishi node a017 at load 70 to 75, one census solve each; the plain
solve time is the same tree without counters, run before it on the same node). A facet from row i of E has the
neighbour A xor {i}: the row is mapped to its constraint through the region's `lambda_set`, `regular_set` and `omega_set`
(rows are lambda, regular, omega in that order; the mapping was available on every region of every problem), and a lambda
row removes the constraint, a regular row adds it, an omega row (parameter-set boundary) has no neighbour.

| problem | plain ms | facets | omega rows (boundary, class none) | A xor {i} a known built region | of those: single vanishing row and Case-1 fast | index-known but not single or not fast | index set equals the toggle set |
|---|---:|---:|---:|---:|---:|---:|---|
| rand_t8_x5_m20_s0 | 562 | 86,346 | 21,432 | 25,288 | 25,288 | 0 | 25,288 of 25,288 |
| rand_t12_x5_m20_s0 | 2458 | 235,103 | 118,289 | 42,240 | 42,240 | 0 | 42,240 of 42,240 |
| pop_017 | 449 | 55,862 | 3,578 | 18,330 | 18,329 | 1 | 18,347 of 18,347 |
| masses4_N3 | 1949 | 73,623 | 17,255 | 11,135 | 11,088 | 47 | 13,135 of 13,135 |
| pop_002 | 72,659 | 3,929,854 | 376,956 | 3,022,287 | 3,019,536 | 2,751 | 3,024,576 of 3,024,576 |

- The index rule agrees with the tableau on every known Case-1 neighbour (0 disagreements, 0 misses, and no Case-1
  facet whose neighbour was unknown to the tableau but known to the index). The only exceptions are index-known facets
  that are degenerate (more than one vanishing row, a rank-1 or walk facet): 47, 1, 0, 0 and 2,751 of them (0.4%, 0.01%,
  0%, 0% and 0.09% of the index-known facets). Skipping those by index alone would drop their neighbours, so an exact
  skip needs the "single vanishing row" fact. The stored region cannot give it: `remove_duplicate_rows` drops the rows that
  coincide, so the builder would have to record which rows had duplicates or near-duplicates.
- Facets settled this way (omega boundary plus index-known single Case-1) and the time the `adjacent` calls spend on them
  today, as a share of the plain solve (ceiling, before the cost of the index lookups themselves, about 50 ns each):
  rand_t8 46,720 facets, 24.5 ms, 4.4%; rand_t12 160,529 facets, 91 ms, 3.7%; pop_017 21,907 facets, 14.3 ms, 3.2%;
  masses4_N3 28,343 facets, 25.4 ms, 1.3%; pop_002 3,396,492 facets, 4,177 ms, 5.7%.
- Regions whose every facet is settled (their tableau would never be resolved): rand_t8 19, rand_t12 53, pop_017 166,
  masses4_N3 37 (0.02 to 0.3 ms of resolve in all four), pop_002 133,239 of 321,026 (41%), which spend 3,254 ms of the
  6,551 ms of tableau resolve time (4.5% of the plain solve).
  The rank-1 and walk facets keep every other region's tableau needed.

## 6. All swaps A + {c} - {j}, for every j in A, known (no dependency needed)

For each rank-1 facet (class: P- empty, all known, tested) with entering row c, every swap over all inequality indices j in
A is looked up (known = built or rejected); if all are known, no dependency is needed.

| problem | rank-1 facets | all swaps known | share | `adjacent` time of those facets | share of the plain solve | mean share of swaps known |
|---|---:|---:|---:|---:|---:|---:|
| rand_t8_x5_m20_s0 | 36,354 | 807 | 2.2% | 1.2 ms | 0.2% | 0.40 |
| rand_t12_x5_m20_s0 | 69,133 | 2,562 | 3.7% | 7.2 ms | 0.3% | 0.48 |
| pop_017 | 30,799 | 9,130 | 30% | 11.9 ms | 2.7% | 0.71 |
| masses4_N3 | 40,023 | 11,149 | 28% | 22.4 ms | 1.1% | 0.62 |
| pop_002 | 229,034 | 42,303 | 18% | 131 ms | 0.2% | 0.82 |

The test needs |A| key lookups per facet (20 to 40, about 0.4 to 1.5 us), against 1.2 to 4.3 us for the whole all-known
rank-1 call, so the net saving is a fraction of the time shown: at most about 2% of the solve on pop_017 and 1% on
masses4_N3, 0.3% or less elsewhere.
