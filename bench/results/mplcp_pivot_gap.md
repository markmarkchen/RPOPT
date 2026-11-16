# mplcp: pivots in the undecided band decided from the original rows (branch big-cost, on main 1cced96)

## What changed (src/algorithms/mplcp.rs)

- `refined_dependency(p, zj, c)`: Gram solve plus 2 refinement steps (as `refined_multipliers`), and rho = r'Q^-1 r / M_cc for the final residual r = a_c - sum mu_j a_j, formed from the row vectors, not from a Gram difference. rho is the scale-free squared distance of a_c from the span of the active rows, i.e. the scale-free pivot.
- A pivot measure in (MEASURE_ZERO 1e-9, MEASURE_NONZERO 1e-6] is decided by rho on both branches of `adjacent`: rho <= 1e-19 zero (rank-1 Case 2, even when the tableau pivot is above PTOL), rho >= 1e-18 nonzero (toggle, with the child factored from its basis, never pivoted on the tableau entry).
- Shared functions: `classify_pivot`, `classify_gap`, `classify_gap_rows(p, zj, var)` (no tableau needed, for the walk), `classify_multipliers`.
- After a pivot is decided zero from the rows, the multipliers of that call all come from the recomputed values, the tableau symmetry check is skipped for that call, and row c may carry z-column entries up to 1e-5 (tableau rounding reached 1e-4 on masses3_N7). Recomputed multipliers above 1e-12 are real (before: only above 1e-6; the band of recomputed values 1e-12..1e-6 held 9121 and 8082 calls on the two big problems, all of them real).
- A member LP of the rank-1 enumeration that ends undecided sends the facet to the walk (`chebyshev_ball_status`, cherry-picked from 3e1770a).
- Switches: `MPLCP_PIVDD=0` (pivot decision off), `MPLCP_ZCOL=0`, `MPLCP_BANDREAL=0`, `MPLCP_TRIGEXT=1` (see below, default off).

## Threshold evidence (table 3v of lp_census)

rho of the gap-band calls falls in two clusters with an empty interval between them: structural zeros at 1e-21 and below (down to exact 0; 1576 calls on pop_005, 23405 on masses4_N5, 9495 on masses5_N4), small real pivots at 1e-18 to 1e-12 (4437 and 4275 calls on the two big problems; nothing in decades 18 and 19 on masses4_N5 and in 16 to 20 on masses5_N4). Thresholds 1e-19 and 1e-18 lie in the empty interval.

## Audit (every gap call run both ways, `MPLCP_MU_AUDIT=1`; pivot rule on / off)

Mismatches (the enumeration has a set the walk lacks, or the reverse with a real positive multiplier): masses3_N4 1 / 1, masses4_N3 1 / 1, rand_t12 2 / 2, pop_005 2 / 1, others 0. Five of the six exist with the rule off, in main's refined path. Classification of all six facets (`facet_check`, `pair_check`, exact dependence test): none has a walk-only set that is a genuine region bordering the facet. Walk-only sets are dependent sets (5), non-regions (more than 30), or genuine regions that are in the final solve through other facets and touch this facet only in a piece of dimension below d-1 (7). The pop_005 facet earlier called "P- empty" has P- = {2} (mu +4.3), its member already seen; the walk's 19 extra sets are not regions. masses4_N3: [20,35,51] (enumeration only) builds no region; P- = {26, 35} is the same from the tableau and the recomputed values (nu 16.7 and 15.9), j=51 recomputes to 4e-24 (tableau 2.85e-9).

## Region sets against main (158 problems, `RPOPT_CANON=1`)

Identical on 157. pop_028 gains [1,16,22,29,46] (radius 1.2e-8) and [14,16,19,22,46] (3.9e-8): both regions whose interior the QP labels with a neighbouring set ([1,16,22,28,29], [14,16,19,22,28]), that is two near-duplicate labels (overlaps of the known kind, see mplcp_dup_test.md). masses5_N4: identical. masses4_N5 (raw indices): loses [1,4,14,55,58] (boundary sliver, QP infeasible), gains [6,7,21,64,79] (radius 1.8e-7, QP agrees: genuine) and five boundary slivers of radius 1.1e-8 to 4.8e-8 ([1,8,36,52,75], [4,5,26,57,87], [4,6,24,55,58], [4,29,36,55,58], [9,14,29,55,58]; QP infeasible at their centres), the kind main already outputs.

Two earlier versions lost genuine regions and were fixed before this gate: skipping the tableau symmetry check on every recomputed call lost masses3_N7 [1,4,5,6,17,49,61]; recomputing all multipliers whenever any tableau entry lies in (1e-12, 1e-9] (the reviewer's third finding) lost masses5_N4 [7,34,36,69] (radius 7.6e-8). The second is left off (`MPLCP_TRIGEXT=1`); the reviewer's scale-dependence of tableau zeros is therefore not fixed.

## Speed (plain release solve, 1 CPU, main then new on the same node, loaded: 71 to 83)

| problem | main (s) | new (s) | ratio |
|---|---:|---:|---:|
| masses5_N4 | 351.7 | 161.1 | 2.18 |
| masses4_N5 | 282.5 | 144.2 | 1.96 |
| pop_028 | 24.0 | 24.0 | 1.00 |

Walk share of the solve after the change: 14% and 28% (was 59% and 61%); what is left is the symmetry-mismatch class (3682 and 6379 calls, 14% and 19% of the solve), nonzero pivots (1% and 9%), and the multi-row classes.

## Scale test (one constraint row scaled by 1e-6 and 1e6, `bench` problems masses4_N3, masses5_N3, pop_005, rand_t12)

Scaling by 1e6: sets unchanged on all four. Scaling by 1e-6: thousands of sets change (masses4_N3 row 21: 1598 removed, 1525 added; masses5_N3 row 35: 2604 / 2467; rand_t12 row 4: 4251 / 3054), identically on main 1cced96 and on this branch with the rule on or off. So the defect is not in the pivot decision; most differing sets do not contain the scaled row, which points at the preprocessing or a tolerance on the row norm, not at the neighbour search.

## Gate on the rebased branch (big-cost 483f2d4 on main d832d8f)

Region sets, all 158 problems against main (the sets of the main binary, `RPOPT_CANON=1`): identical on 157; pop_028 gains the two near-duplicate labels described above. masses5_N4 and masses4_N5 are not in the 158; their differences are listed above.

Audit verdict, counting only genuine regions that border the facet (contact radius above the walk's 1e-9) and keeping the other classes visible. Six mismatch facets (masses3_N4 1, masses4_N3 1, rand_t12 2, pop_005 2):

| class of the walk-only or enumeration-only set | sets |
|---|---:|
| dependent set (contains c and every row with recomputed nu above 1e-12) | 5 |
| no full-dimensional region from the active set | 30 or more |
| genuine region in the final solve, contact with the facet below 1e-9 | 6 |
| genuine region bordering the facet (rand_t12 [1,11,15,16,17], contact radius 1.66e-9, in the final solve) | 1 |

So: one negligible miss in six facets, no region lost. audit-search accepted the nonzero side after the switch comparison below.

Nonzero side of the gap decision: with `MPLCP_GAPNZ=0` (pivots decided nonzero left to the old rule) the sorted active sets are identical on masses5_N4 and masses4_N5, and the solve is 22% and 7% slower (laptop: 87.3 s and 77.3 s with the decision, 106.7 s and 82.4 s without). The five boundary slivers masses4_N5 gains are present with the switch off too.

Forced calls (a pivot decided zero from the rows) with a z-column entry of row c above 1e-8, counted per entry: masses5_N4 5536, masses4_N5 28684, masses3_N6 3450, pop_005 82, masses4_N3 0, rand_t12 0. The tableau of those calls is noisy (the cuts read lambda from it); a fresh `tableau_for` for them is a follow-up.

Paired timing, one node (a244, load 94 to 101), new against main d832d8f; masses5_N4 and masses4_N5 as one task per (alternation, pair) with two solves each, 3 alternations, A/A pair main against main; the others 5 alternations, one solve each of new, main, main again in shuffled order:

| problem | main (s) | new / main (median) | A/A (median) |
|---|---:|---:|---:|
| masses5_N4 | 351.9 | 0.460 | 0.998 |
| masses4_N5 | 286.5 | 0.510 | 1.002 |
| masses3_N6 | 11.48 | 0.914 | 1.000 |
| pop_005 | 2.07 | 0.954 | 0.995 |
| masses4_N3 | 1.59 | 0.999 | 0.997 |
| rand_t12_x5_m20_s0 | 1.90 | 1.004 | 1.003 |

Suite (`ci_build.sh c2cost_T full`, 158 tests): 155 passed, 0 failed, 3 ignored.

## Open

- Finding 3 of the review (tableau zeros not relative to the accepted members): the extension that recomputes whenever a tableau entry lies in (1e-12, 1e-9] loses masses5_N4 [7,34,36,69]; it stays off. masses3_N7 [1,4,5,6,17,49,61] needs the symmetry check kept. Both are genuine thin regions that main reaches through walk entries below any nonzero rule (walk-fix: raw 4.7e-10, scale-free 1.8e-12 on masses3_N7); one defect, still to be traced.
- The symmetry-mismatch class (3682 and 6379 calls, 14% and 19% of the solve on the two big problems) and the |pivot| M_jj rule for vanishing multiplier rows.
- The row-scaling defect (above) is a main defect.

Against the fresh main b57d52d sets of mplcp-audit (no `RPOPT_CANON`, `~/rpopt/mplcp-audit/gate_lp/main`): 155 problems compared, 1 differing (pop_028, the same two labels), 3 problems without a main file.
