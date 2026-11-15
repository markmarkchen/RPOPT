# Thin regions lost by the hint-started Chebyshev LP, and the cold retry

Branch `walkclass` (worktree agent-a98de2306d1951999), code change in `mplcp_build.rs` (commit 4a93b04), tools and counters around it (head f3196c1 plus
1f080db). Negishi, release builds, 1-CPU tasks.

## The defect

`build_region` starts its Chebyshev LP at the hint, a point on the facet the candidate was reached through, so on the boundary of the new region
(slack -6e-15 on masses5_N3). On a thin region that warm-started LP can stop at radius 0.0 or fail: masses5_N3 [2,15,49] (true radius 2.2e-6) came back with
radius 0.0 and [37,46,53] (3.9e-8) with `None`. The candidate is dropped and its active set stays claimed in `Seen`, so no other facet ever builds it. Which
facet reaches a set first (and with which hint) therefore decides whether a thin region survives; the old guard, the walk and the new multiplier rule all lost
regions this way, in different places. The multiplier rule was not the cause (a knob test restoring the old guard brought the two regions back only because the walk
happened to hand the builder a different hint).

Fix: if the warm LP gives radius <= 1e-8 or fails, a cold `chebyshev_ball` decides, with the same 1e-8 acceptance threshold. Counter `BuildColdRetry`.

## Gate

Three builds on the 158 problems (the 51 timing problems plus the large set without pop_002, pop_024, pop_033, pop_036, pop_051, pop_095, masses4_N5, masses5_N4):
`base` = the multiplier rule at b1e3861, `fix` = base + the cold retry, `oracle` = fix with `set_rank1(false)` (the Case-2 walk only; it uses the same builder).

* **fix against its walk-only oracle:** identical sorted active sets on 157 of 158 problems; masses3_N7 has one set more with the fix ([5,8,9,10,58,79,91]), which is genuine (radius 2.3e-7, QP active set = the set, centre in no other region): the walk-only oracle misses it.
* **base against fix:** identical on 146; the fix adds regions on 12 problems and loses none (base-only sets: 0 on all 12):

| problem | base | fix | walk-only oracle |
|---|---:|---:|---:|
| masses3_N5 | 10,436 | 10,444 | 10,444 |
| masses3_N6 | 22,143 | 22,152 | 22,152 |
| masses3_N7 | 38,326 | 38,343 | 38,342 |
| masses4_N3 | 2,947 | 2,953 | 2,953 |
| masses4_N4 | 16,312 | 16,342 | 16,342 |
| masses5_N3 | 5,266 | 5,273 | 5,273 |
| pop_076 | 15,006 | 15,007 | 15,007 |
| pop_091 | 145,018 | 145,022 | 145,022 |
| rand_t10_x5_m20_s1 | 14,848 | 14,849 | 14,849 |
| rand_t10_x6_m24_s0 | 71,219 | 71,221 | 71,221 |
| rand_t6_x5_m20_s1 | 5,062 | 5,063 | 5,063 |
| rand_t8_x6_m24_s0 | 32,475 | 32,476 | 32,476 |

(For comparison, ddf0aa8 had 5,268 on masses5_N3, 2,947 on masses4_N3.)

## Classification of the 87 added regions (`pair_check`: lexico-feasibility, tightness at the centre, the QP's active set at the centre, which solved regions contain the centre)

* **Genuine, 77:** lexico-feasible, the QP at the Chebyshev centre returns exactly the set, the centre lies in no other region of the solve. Radii 1e-8 to 6e-2,
  mostly 1e-7 to 1e-5. Includes the two lost ones ([2,15,49] radius 2.2e-6, [37,46,53] 3.9e-8), [5,8,39], [7,36,38], [2,19,55], [5,15,27] (exists only through the tableau-law
  path: `pair_check` cannot rebuild it from the active set alone, the solve's own region has radius 1.0e-7, tight residual 2.5e-8, QP agrees) and, on masses3_N6, [2,3,4,20,31] (radius 1.7e-7).
* **Doubtful, 10, all at radius 1e-8 to 4e-8 (the acceptance edge):**
  * the QP is infeasible at the centre (boundary of the feasible set): masses3_N5 [4,13,25,36,69] and [9,19,31,42,63]; masses4_N4 [5,18,45,64], [7,14,32], [11,37,43,68], [12,44,53,55];
  * overlap, the centre lies inside another region whose active set the QP returns: masses5_N3 [9,36,39] (inside [0,36,39], radius 1.1e-8, a known sliver at the edge, of the kind main
    already has), masses4_N4 [6,21,28] (QP [7,21,28]), masses4_N4 [7,12,37,39] (also inside [0,7,37,39]), rand_t10_x6_m24_s0 [4,5,7,10,11,15] (also inside [5,10,11,13,15,16]).
* A QP-at-the-centre guard on thin accepts was tried and dropped: the QP flips at the centres of regions of radius 1e-8 to 1e-6 and removes genuine ones (masses5_N3 5,127 to 5,205 regions against 5,273,
  masses3_N4 3,633 to 3,640 against 3,643).

## Tests and timing

Timing (51 problems, 4 to 5 tasks, A/A pair; `base` against `fix` as built at a32ed49, whose builder is the same): all 0.990, masses 0.982, not masses 0.993, 2 to 5 parameters 0.992,
6 to 16 parameters 0.987, A/A 0.998 to 1.000. The fix costs 1% to 2%: the empty-P- safeguard of the same build (about 1%) and the extra regions it finds (masses4_N3 +6 regions, masses4_N4 +30).
Suite (ci tag c2cost_M, 147 tests, pre-rebase tree): 144 passed, 0 failed, 3 ignored.

## Known edge slivers

The 10 doubtful regions are at the acceptance edge (radius 1e-8 to 4e-8). Six have no QP solution at their centre (masses3_N5 [4,13,25,36,69], [9,19,31,42,63]; masses4_N4 [5,18,45,64], [7,14,32],
[11,37,43,68], [12,44,53,55]), four overlap an existing region (masses5_N3 [9,36,39] inside [0,36,39]; masses4_N4 [6,21,28] and [7,12,37,39]; rand_t10_x6_m24_s0 [4,5,7,10,11,15]). They are of the kind main
already outputs (masses3_N7 [3,4,5,7,8,43,97], radius 1.7e-7, QP infeasible at its centre). The cold retry does not create the class, it stops dropping thin candidates; a local overlap test
against known regions differing in at most two constraints is the next item.

## Rebased tree (main 685c144 with the box chain and the twin snap underneath; branch `coldretry-final`)

* Region sets, `base` = 685c144, `fix` = base + the merged branch, `oracle` = fix with the walk only: identical in every number to the pre-rebase run (same 12 problems with the same region counts
  and the same set hashes for base, fix and oracle; fix against oracle: 157 of 158 identical, masses3_N7 one genuine region more). The box chain and the twin snap change no region set on these 158 problems.
* Timing (51 problems, `base` against `fix`, A/A pair): all 0.993, masses 0.979, not masses 0.997, 2 to 5 parameters 0.995, 6 to 16 parameters 0.989, A/A 0.999 to 1.000.
* Suite (ci tag c2cost_N, 147 tests): 143 passed, 0 failed, 3 ignored with 145 of 147 results in; the two slow `pl_wnn` tests (random_mpqp_theta6 and theta8_b) were still running when this was written. The
  pre-rebase tree passed 144/0/3.
