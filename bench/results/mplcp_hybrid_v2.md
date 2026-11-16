# Case-2 hybrid v2: closure of the facet basis (off by default, not gated)

**Status.** Closed, with one stated reopening condition (see the conclusion). The code is on branch `case2-hybrid-v2` (review fixes f1b032f, vanishing-row inheritance a33ac46, per-reason walk counters 6d1d9b1, and the commit that adds this note and removes the rejected `RPOPT_HYNEAR` switch); it is off by default and was never merged into main's behaviour. The census below was measured on tree 6d1d9b1 (main 1cced96 plus big-cost 6945a9b); it predates main's exact pivot decision (308d384) and the reason-5 fix (KKT law on a symmetry mismatch, mplcp-case2-cost). The hybrid is switched on by `mplcp::set_case2_hybrid(true)` (`RPOPT_HYBRID=1` in the examples); `RPOPT_HYAUDIT=1` (census build) runs the walk beside it and compares. Nothing here was gated. Raw census results are in `~/rpopt/mplcp-regbuild/wk_prof/` on Negishi; the candidate and audit logs of the local runs were not kept.

## What v2 is

For a Case-2 facet that `rank1_case2` refused, the neighbours are the closure of the facet's basis A under single moves inside the facet (hyperplane g.theta + b = 0), instead of the walk (`hybrid_facet`, `hy_node` in `mplcp.rs`):

* Move (a): a row that vanishes on the facet and has a nonzero pivot is toggled (principal pivot of the row and its complement).
* Move (b): a slack row c that vanishes and has a zero pivot is a dependent constraint, a_c = sum mu_j a_j; the nodes are B + {c} - {j} for every j with mu_j decidedly positive (the column is read once; both swap pivots are checked).
* Every node is expanded, whether or not it is seen or empty, because a stepping stone has a region of lower dimension. Seen, lex and emptiness filters gate emission only. The closure has at most 32 nodes.
* Placement: after `rank1_case2` returned None and before the walk, for nt of at least 2. Anything the closure cannot settle (an undecided pivot, multiplier or vanishing row, more than 32 nodes, nothing emitted and no node built) returns the facet to the walk unchanged.
* Scale-free decisions: pivots by the pivot measure (zero at most 1e-9, nonzero above 1e-6, between is undecided; multiplier rows use |pivot| M_jj), multipliers by MU_ZERO and MU_NONZERO as in `rank1_case2`. At the facet's own basis the vanishing rows are `adjacent`'s `zrows`; at derived nodes the rows are measured with the three-way test (exact at most 1e-9 of the row scale, undecided up to 1e-6), except that across move (a) the vanishing rows are inherited from the parent (commit a33ac46, unaudited).
* Seen lookup by key XOR through `SeenSet::find`; no index vector per candidate.
* Lex gate: a node is emitted only if `lex_feasible_rel` holds (theta part vanishing relative to max(1, |rho_0|)).
* Own DAQP state: `daqp_empty_impl(.., adaptive = false, ..)` has no pause window. Rows are shifted inwards by 1e-8 (the build's cut; `RPOPT_HYSHIFT` changes it). The DAQP point of the shifted system is handed on as the build hint.
* Rejected records: `Nbr::Rej` is recorded in Seen only when the system relaxed outward by 1e-7 is also certified empty (the tableau law errs by up to 3.4e-8 relative). Shift-only empties are dropped from the facet and not recorded.
* The candidate's own tableau is handed on (`Cand.pretab`): the law comes from it, it becomes the child's tableau, and past REFACTOR (32) pivots from the last refactorisation it is dropped and rebuilt from the active set. Both solve loops carry the depth.
* Census switches: `RPOPT_HYAUDIT=1` also runs the walk on every facet the closure answered and compares the fresh sets (counters `HY_AUD_*`, `W` lines in the `RPOPT_HYBLOG` file); `RPOPT_HYBLOG=<file>` writes `F` (facet basis), `B` (band rows at the basis), `N` (node verdicts) and `R` (build outcomes) lines.

## Why v1 lost 23 regions (cause table)

All 23 sets lost by v1 (17 `a_lost` and 6 `a_lost_unbuildable`, masses3_N5 to N7, masses4_N3, N4, masses5_N3) were generated as candidates, so no candidate was missing.

| cause | sets | evidence |
|---|---|---|
| the absolute 1e-9 pivot tolerance in `tab_to_basis` rejected a swap | 19 | facets with one vanishing slack row (Z = 1, pv = 1); the enter-first order has a pivot of raw size 1e-16 to 1e-28 (measure 5e-16 to 1e-11), the leave-first order has a fine first step and a second pivot of raw size 1e-10 to 1e-15 whose scale-free measure is 2.3e-5 to 4.9e-1, at least 23 times MEASURE_NONZERO |
| tableau-law noise in the build, the thin region is rejected on first arrival | 4 | masses3_N6 [2,3,6,10,29,83], masses3_N7 [1,4,5,6,17,49,61] and [8,9,10,11,58,79,91], masses5_N3 [28,29]; all four build from the active-set law (law None), radii 1.55e-8, 9.0e-7, 1.3e-7, 3.5e-7 (job 45938618, `pair_check`), so the shared retry with law None in `build_cand` cures them and v2 adds none |

The first cause was fixed in v2 (scale-free measures, the facet goes to the walk when a pivot is undecided). All 19 of the first class were single-row facets that `rank1_case2` handles; the hybrid is placed after rank-1 and does not see them any more.

## Counterexample and unit test

min 1/2 |x - (10,10)|^2 subject to x1 <= th1, x2 <= th1, x1 + x2 <= th2 on 0 <= th1 <= 1, 0 <= th2 <= 4. Region {1,2} meets region {3} on th2 = 2 th1, where a_3 = a_1 + a_2 and the two multipliers are equal on the whole facet (a tie, so `rank1_case2` refuses). The neighbour {3} is two removals away from {1,2}; the closure reaches it through {1,3} or {2,3} and then toggles the multiplier that vanishes on the facet. Unit test `hybrid_finds_the_two_removal_neighbour`: hybrid on and off give the same 2 regions, one with active set {2} (row index 2), and the hybrid answered at least one facet. It passes (`RUST_TEST_THREADS=1 cargo test --release --lib hybrid_finds`). It flips the global switch, so the suite must also be run once with the hybrid off everywhere.

## Facets reaching the hybrid before and after the big-cost merge (hybrid code held fixed)

| problem | reaching the hybrid, f1b032f (main 1cced96) | after the merge of big-cost 6945a9b | answered after the merge |
|---|---|---|---|
| masses3_N4 | 133 | 25 | 9 |
| masses4_N3 | 241 | 54 | 54 |

On masses3_N4 before the merge, 107 of the 133 facets bailed on a pivot in the undecided band (105 of them a slack toggle) and 17 on an undecided multiplier; after the merge 16 bail on a pivot. Final region sets with the hybrid on and off are identical on masses3_N3, masses3_N4 (3643 regions) and masses4_N3 (2953).

## Per-facet audit on the answered facets (merged tree, `RPOPT_HYAUDIT=1`)

| problem | answered | same fresh sets | hybrid-only | walk-only, finished walk | walk-only, unfinished walk |
|---|---|---|---|---|---|
| masses3_N4 | 9 | 6 | 2 | 1 | 0 |
| masses4_N3 | 54 | 41 | 11 | 2 | 0 |

Lex drops, shift-only drops and zero-result nets were 0; Rejected records 4 and 9; DAQP points handed on 2 and 11; node histogram 7 facets with 3 nodes and 2 with 4 on masses3_N4, 36 and 18 on masses4_N3.

The three walk-only sets:

* masses4_N3 [24] from [0,24,51] and [32] from [3,32,43]: the closure generated [24,51] (DAQP-empty) and [0,24] (built) but not [24], because after the first toggle the second removed row does not vanish at the derived node when re-measured. Neither set is a region of main (not among its 2953), so nothing is lost. The only band row at the basis is the member con 24 (con 32) itself, a multiplier row at res/sc 2.8e-8.
* masses3_N4 [2] from [1,2,10,55]: the closure generated [1,2,55] and [1,2,10] (both seen) but not [2], which needs three removals. [2] is a region of main and is found by the hybrid run as well, through another facet, so the final set is unchanged; it is not reached on its own facet. The band rows at the basis include the members 1 and 2 (res/sc 1.0e-9 and 2.5e-9) and the non-members 3, 7, 44, 47, 50, 53 (2e-8 to 6.7e-7).

Band rows at the facet's own basis (the vanishing-row test in the 1e-9 to 1e-6 band): 224 rows on masses3_N4 (gi positive in 143, negative in 81), 866 on masses4_N3 (507 and 359); the row scale is set by gradients of about 1e6, so the band is a distance of 1e-9 to 1e-6 in parameter units; they are not rows of small natural scale.

## Defects still open if anyone reopens v2

* Vanishing rows must be inherited across a toggle (move (a)) and not re-measured; this is coded in a33ac46 but has not been audited.
* Near-vanishing rows need a distance rule tied to the build cut (about 1e-7 in parameter units); the full 1e-6 band blows the 32-node cap (masses3_N4: 0 of 25 facets answered, masses4_N3: 19 of 54) and was dropped. The distance rule was not implemented.
* Items A to C of the review (exact `classify_pivot` for slack-row pivots, multipliers through the refined dependency, `tableau_for` for kids decided by rho) are not implemented; D (walk-fix's `vanishes_on_facet`) is not used.
* The 13 hybrid-only sets (2 on masses3_N4, 11 on masses4_N3) have no build outcome: none had a build line, so each was claimed by another route first.
* `lex_feasible_rel` uses the unit max(1, |rho_0|); the natural row unit is not settled.
* The `RPOPT_HYNEAR` full-band switch was rejected and removed. The shift of 1e-8 for the emptiness filter was not studied against main's regions (masses3_N6 [2,3,6,10,29,83] has radius 1.55e-8).

## Walk-time census, hybrid off (first census)

Tree 6d1d9b1 (main 1cced96 plus big-cost 6945a9b), build job 45952995, node a265 for all four tasks (1 CPU each), node load not recorded. Census clocks (two clock reads per lap) inflate the times against release timings. Walk = time inside `pivot_facet` after `rank1_case2` returned None (the sum of `R1Nanos0..5`); rank-1 part = Case 2 minus walk.

| problem | solve ms | neighbour search ms (share) | Case 2 ms (share) | walk ms (share) | rank-1 part ms (share) | Case-2 facets | reaching the walk |
|---|---|---|---|---|---|---|---|
| masses3_N6 | 10,837 | 3,210 (29.6%) | 3,060 (28.2%) | 1,716 (15.8%) | 1,344 (12.4%) | 165,225 | 1,558 |
| masses5_N3 | 7,717 | 762 (9.9%) | 696 (9.0%) | 7 (0.1%) | 689 (8.9%) | 100,553 | 105 |
| masses4_N5 | 200,370 | 109,750 (54.8%) | 105,441 (52.6%) | 97,377 (48.6%) | 8,064 (4.0%) | 769,125 | 23,718 |
| masses5_N4 | 315,231 | 193,846 (61.5%) | 189,329 (60.1%) | 180,356 (57.2%) | 8,974 (2.8%) | 735,390 | 18,730 |

Walk facets and walk time by the reason `rank1_case2` returned None (share of the solve):

| reason | masses3_N6 | masses5_N3 | masses4_N5 | masses5_N4 |
|---|---|---|---|---|
| 1 several rows or pv != zrows | 295 facets, 9.5 ms (0.1%) | 105, 7.0 ms (0.1%) | 2,230, 78.5 ms (0.0%) | 935, 34.2 ms (0.0%) |
| 3 nonzero pivot | 202, 943.7 ms (8.7%) | 0 | 4,959, 23,894 ms (11.9%) | 4,488, 43,725 ms (13.9%) |
| 4 z column in row c | 866, 651.3 ms (6.0%) | 0 | 6,447, 58,832 ms (29.4%) | 1,623, 20,458 ms (6.5%) |
| 5 symmetry mismatch | 13, 101.4 ms (0.9%) | 0 | 975, 2,129 ms (1.1%) | 1,083, 671 ms (0.2%) |
| 6 multiplier band | 182, 10.2 ms (0.1%) | 0 | 9,107, 12,445 ms (6.2%) | 10,601, 115,469 ms (36.6%) |
| 2, 7, 8, 9 | 0 | 0 | 0 | 0 |
| walk outside reasons 5 and 6 | 1,605 ms (14.8%) | 7.0 ms (0.1%) | 82,805 ms (41.3%) | 64,217 ms (20.4%) |

This census predates main's exact pivot decision (308d384, which acts on reason 3), the relaxed z-column test (reason 4) and the reason-5 fix, so the totals are stale (the lead measured 14% and 28% on big-cost 22c9e57 for masses5_N4 and masses4_N5).

## Conclusion

v2 is closed, with one stated reopening condition. The only facets where the closure does what rank-1 cannot are those rank-1 declines for reason 1 (several vanishing rows, or pv != zrows) and for the tie reasons 7 and 8. On 6d1d9b1 they carry 0.0% to 0.1% of the solve on masses3_N6, masses5_N3, masses4_N5 and masses5_N4 (reasons 7 and 8 do not occur; reason 1 depends only on the count of vanishing rows, so main's exact pivot decision cannot change it). That census predates main's exact pivot decision and the reason-5 fix; the lead will read reasons 1, 7 and 8 from mplcp-case2-cost's per-reason walk table on main after the reason-5 merge. If they exceed 5% of the solve on masses4_N5 or masses5_N4, v2 is reopened from the tagged branch in this order: inherit vanishing rows across a toggle (a33ac46, to be audited), the 1e-7 distance rule for near-vanishing rows, the shared exact decision (items A to C), then the staged gate. Walk time in the single-row classes (reasons 3, 4, 5, 6) does not count toward this condition. The walk totals and the split over reasons 3 to 6 in the census above were measured on 6d1d9b1 and are stale; current figures are mplcp-case2-cost's.

## What was not done

* No cluster gate: no `set_audit` of v2 against main (so no a_lost figure for v2 and no check for new c1_overlap or d_dup), no suite run with the hybrid off everywhere, no paired timing. There is no speed claim for v2.
* Items A to C and the distance rule are not implemented; the inheritance commit a33ac46 is unaudited.
* The 13 hybrid-only sets have no build outcome; [2] on masses3_N4 is not reached on its own facet.
* The shift study (1e-8, 1e-9, 0) and the lex unit are open.
