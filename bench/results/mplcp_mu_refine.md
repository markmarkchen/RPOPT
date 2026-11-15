# Gap-band multipliers decided from the original data

Branch `walkclass` (worktree agent-a98de2306d1951999), head 16391df, on main 107aa72 plus `case2-mu-guard`. Negishi, 1-CPU tasks, release builds, census
solves with the audit switches of `lp_census` (`MPLCP_MU_AUDIT=1`, `MPLCP_NO_REFINE=1`).

## What was added

`refined_multipliers(p, zj, c)` recomputes the dependency a_c = sum_j mu_j a_j of a Case-2 call over the active rows `zj` from the original data: the
Gram system M_ZZ mu = M_Zc equilibrated to a unit diagonal (the rows of A differ in scale by 1e3 and more, and the library LU judged the raw Gram matrix
singular on a quarter of the calls), then two steps of iterative refinement whose residual r = a_c - sum_j mu_j a_j is formed from the rows of A themselves
and whose correction solves M_ZZ delta = (A_Z Q^-1) r. In `rank1_case2` a call that meets a multiplier in the band (nu between 1e-9 and 1e-6) now
uses the refined values for its classification: refined nu <= 1e-12 is zero, refined nu >= 1e-6 is nonzero, in between goes to the walk.
It needs the rows of A, so it is off with equality rows. Safeguard (Lean Finding 14): when no multiplier is positive but a positive tableau multiplier
(above 1e-12 in nu) was decided zero, the same recomputation must show exact zeros, otherwise the call goes to the walk.

## Measure: the refined nu of the gap-band entries (table 3t, audit runs)

| problem | gap-band calls | band entries | refined nu <= 1e-12 | refined >= 1e-6 | still between | calls fully decided | us per recomputation |
|---|---:|---:|---:|---:|---:|---:|---:|
| masses4_N3 | 394 | 409 | 409 | 0 | 0 | 100% | 0.8 |
| masses5_N3 | 724 | 762 | 762 | 0 | 0 | 100% | 0.7 |
| masses4_N4 | 4,846 | 5,610 | 5,502 | 0 | 108 | 98% | 1.0 |
| pop_005 | 1,832 | 2,316 | 2,316 | 0 | 0 | 100% | 1.4 |
| rand_t12_x5_m20_s0 | 376 | 418 | 418 | 0 | 0 | 100% | 1.1 |
| masses3_N6 | 5,056 | 7,173 | 6,991 | 0 | 182 | 96% | 1.4 |
| masses3_N7 | 8,802 | 13,647 | 13,249 | 0 | 398 | 96% | 1.6 |

The refined values of the noise entries are 1e-22 and below (decade "below 1e-14" holds all of them); the large entries agree with the tableau in
sign and to 1e-6 relative on all 711 to 30,887 compared per problem (0 disagreements; the sign convention is the tableau's mu = -T[i,kk]). The entries
that stay between are small real multipliers at nu = 1e-7 to 1e-6 (108 on masses4_N4, 182 on masses3_N6, 398 on masses3_N7), all left to the walk.
The tableau value of the same entries was 1e-9 to 1e-8: the gap band is rounding of exact zeros, as expected.

## Audit: the refined enumeration against the walk on every gap-band call

Neighbour sets compared as sorted active sets, known sets (Seen) removed from the walk's side (the walk does not drop every known set).

| problem | audited | same | walk has extra sets, all linearly dependent | walk wandered (no real positive multiplier) | walk has other extra sets (a real positive multiplier exists) | enumeration has a set the walk lacks | walk incomplete | walk calls left |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| masses4_N3 | 394 | 289 | 91 | 5 | 0 | 1 | 8 | 635 |
| masses5_N3 | 724 | 614 | 69 | 14 | 5 | 0 | 22 | 1,194 |
| masses4_N4 | 4,846 | 3,984 | 580 | 32 | 16 | 0 | 75 | 9,613 |
| pop_005 | 1,832 | 1,579 | 124 | 11 | 1 | 8 | 2 | 3,573 |
| rand_t12_x5_m20_s0 | 376 | 261 | 65 | 0 | 2 | 0 | 48 | 418 |
| masses3_N6 | 5,056 | 3,907 | 741 | 77 | 28 | 0 | 54 | 9,187 |
| masses3_N7 | 8,802 | 6,823 | 1,168 | 166 | 67 | 2 | 94 | 16,552 |

(walk calls left is the audit run's count, where the walk answers every call; with the refined rule on, 241 remain on masses4_N3.)

* 69% to 86% of the audited calls give the same set (masses4_N3 73%, masses5_N3 85%, masses4_N4 82%, pop_005 86%, rand_t12 69%, masses3_N6 77%, masses3_N7 78%). Where they differ the walk is the one with more: the extra sets are linearly dependent rows (the walk
  removes a row whose multiplier is zero, or descends from such a set: not an active set of any region), or the walk wandered through further bases
  because a positive noise multiplier made it think there was a neighbour although every real multiplier is negative (the P- of the exact dependency is empty).
  In 8 calls of pop_005 and 1 to 2 of masses4_N3 and masses3_N7 the enumeration has a set the walk lacks: in the sampled cases the walk's lexicographic
  tests were distorted by a member with a multiplier of 1e-22.
* The calls that are neither the same nor explained (walk has other extra sets while a real positive multiplier exists: 0 to 67 per problem) were not explained
  one by one. What decides is the region set below.

## Empty-P- safeguard on 132 problems (those with Case-2 calls), table 3u

499,268 times P- was empty while a positive tableau multiplier was decided zero (pop_032 174,468, rand_t10_x6_m24_s0 67,870, masses3_N7 47,408 ...); the
recomputation showed exact zeros every time except 4 (di_doc_N3), which went to the walk. With the guard at every positive nu above 0 the 2 to 5 parameter
problems were 4% slower (geometric mean 0.954); restricted to nu above 1e-12 the cost is 1% (0.989).

## Gate (against `walkclass` b1e3861, which equals the multiplier rule of mplcp_mu_guard.md)

* **Region sets** (158 problems, head 8352a15, before the 1e-12 restriction; 51 problems again on 16391df): sorted active sets identical on 149 of 158; rows only
  (1e-9) differ on seven; **masses3_N6 and masses3_N7 have one region more** (22,144 and 38,327). The extra regions (`extra_regions`): masses3_N6
  [2,3,4,20,31] radius 1.2e-5 lies inside [2,3,4,20], QP active set at its centre [2,3,4,20]; masses3_N7 [5,7,8,9,10,64,97] radius 1.1e-5 inside
  [5,7,8,9,10,64], QP active set [5,7,8,9,10,64]. Overlapping slivers (a weakly active constraint added to a smaller region), the kind `base` had on masses5_N3 and
  that the wider zero band also created.
* **Tests:** 138 of 141 results in at the time of writing, 0 failed, 3 ignored (the full 147 are still running on Negishi).
* **Timing** (51 problems, `base` against refined, A/A pair; ratio above 1 means refined is faster): all 1.004, masses 1.023, not masses 0.999, 2 to 5 parameters 0.989,
  6 to 16 parameters 1.026, A/A 0.999 to 1.002.

| problem | params | base ms | base/refined | A/A |
|---|---:|---:|---:|---:|
| masses4_N3 | 8 | 2,124 | 1.385 | 0.999 |
| rand_t12_x5_m20_s0 | 12 | 3,073 | 1.240 | 1.003 |
| pop_005 | 7 | 2,316 | 1.056 | 1.003 |
| masses3_N4 | 6 | 933 | 1.024 | 0.996 |
| masses5_N2 to masses8_N2 | 10 to 16 | 203 to 1,010 | 0.995 to 0.996 | 0.994 to 1.001 |
| masses2_N4 to N8 | 4 | 12 to 150 | 0.984 to 0.992 | 0.995 to 1.003 |

Census (clean, refinement on): remaining walk share of the solve 1% (masses4_N3, masses5_N3), 6% (masses4_N4, pop_005), 0% (rand_t12), 22% and 25% on masses3_N6 and N7
(the band entries that stay between, and ties).

## Verdict

The recomputation does what was hoped: every gap-band entry on the five problems is rounding of an exact zero (1e-22 and below), and a gap-band call costs 1 us to
decide instead of 0.8 to 3 ms of walk. It is exact on the region sets of 149 of 158 problems and costs one overlapping sliver on masses3_N6 and masses3_N7, so by the
rule set for this change (zero mismatches or each one explained) it is not ready to be the default: the sliver is explained as a kind (a candidate with a dependent
active set that the region builder accepts), not one by one, and the audit still has calls where the walk returns sets the enumeration does not. The gain on top of
the multiplier rule is 1.39x on masses4_N3, 1.24x on rand_t12, 1.06x on pop_005 and nothing elsewhere in the timing set. The refined rule is off by default (`set_refine_multipliers(true)`, `MPLCP_REFINE=1` in the examples turns it on); the empty-P- safeguard and the comment fixes are on.


# Update: the refined rule on top of the cold retry (branch `refine-on`, on coldretry-final 6136b44 and main c5964f2)

With the cold retry in the builder the picture changes: the regions the refined rule seemed to add on masses3_N6 (and the other problems of the first gate) are now found by the cold retry itself.
Region sets of the same binary with the rule off (`MPLCP_REFINE=0`) and on, 158 problems (`RPOPT_CANON=1`): **identical on 157; masses3_N7 gains one set, [5,7,8,9,10,64,97]**.

Verdict on that set (`pair_check`): lexico-feasible (3 zero-theta rows, first entries 1.0), LICQ margin 2.5e-9 (nearly dependent), Chebyshev radius 7.1e-8, tight residual 1.06e-3, the QP has no
solution at the centre, and none at 16 points inside the region at half the radius (0 of 16 feasible); the centre lies in no other region. Main already outputs a region of the same kind,
[3,4,5,7,8,43,97] (radius 1.7e-7, LICQ margin 1.9e-8, 0 of 16 interior points feasible). So it is an **edge sliver on the boundary of the feasible set**, neither an overlap nor a genuine region.
For comparison, masses3_N6 [2,3,4,20,31] (radius 1.7e-7): 16 of 16 interior points feasible with exactly this active set: genuine, and found with the rule off as well.

Gate of the rule being on by default: region sets as above; suite (ci tag c2cost_P, 149 tests): 144 passed, 0 failed, 3 ignored with 147 of 149 results in at the time of writing (the two slow `pl_wnn` tests pending);
paired timing, `base` = the same tree with the rule off, new = on, 3 alternations, A/A pair:

| problem | regions | base ms | tasks | base/new | A/A |
|---|---:|---:|---:|---:|---:|
| masses2_N4 | 307 | 11.8 | 3 | 0.994 | 1.000 |
| masses2_N5 | 493 | 20.2 | 3 | 0.998 | 1.004 |
| masses2_N6 | 901 | 40.3 | 3 | 0.999 | 0.998 |
| masses2_N7 | 1641 | 79.1 | 3 | 1.035 | 0.996 |
| masses2_N8 | 2837 | 145.2 | 3 | 1.010 | 1.009 |
| masses3_N3 | 955 | 155.0 | 3 | 1.006 | 0.996 |
| masses3_N4 | 3643 | 895.9 | 3 | 1.025 | 1.002 |
| masses4_N3 | 2953 | 2042.0 | 3 | 1.334 | 1.001 |
| masses5_N2 | 373 | 191.2 | 3 | 1.000 | 0.998 |
| masses6_N2 | 383 | 299.5 | 3 | 0.994 | 0.999 |
| masses7_N2 | 393 | 563.2 | 3 | 0.998 | 0.999 |
| masses8_N2 | 399 | 761.1 | 3 | 1.003 | 1.000 |
| pop_005 | 10289 | 2115.0 | 3 | 1.058 | 1.000 |
| pop_017 | 4142 | 441.5 | 3 | 1.002 | 0.999 |
| rand_t10_x5_m20_s0 | 6124 | 1392.3 | 3 | 0.998 | 1.000 |
| rand_t12_x5_m20_s0 | 6143 | 2560.7 | 3 | 1.285 | 1.001 |
| rand_t8_x5_m20_s0 | 4111 | 578.8 | 3 | 0.997 | 0.999 |
all 17 base/new 1.039 (17)  A/A 1.000
masses (12) base/new 1.030 (12)  A/A 1.000
rand/pop (5) base/new 1.063 (5)  A/A 1.000

Remaining walk work is unchanged where the rule does not apply (nu in the true gap 1e-7 to 1e-6, ties, nonzero pivots). The switch: `set_refine_multipliers(false)`, `MPLCP_REFINE=0` in the examples.

## Lean's reading of the cold retry (Finding 21)

A drop in `build_region` now needs two LP runs (warm, then cold) to agree that the radius is at most 1e-8, but it is not certified; a checked dual y with sum y_k b_k <= 1e-8 (`chebyshev_radius_le`) would certify it
(possible follow-up). "Claimed implies built or proven empty" overstates it: the right statement is "built, or decided empty by the cold LP", and the 1e-8 threshold drops slivers by design.
