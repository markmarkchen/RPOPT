# mplcp rank-1 enumeration: a scale-free multiplier rule in place of the small-multiplier guard

Branch `case2-mu-guard` (worktree agent-a98de2306d1951999), built on main ddf0aa8 (skip fix merged), gated against it. Negishi, 1-CPU tasks,
release builds, `base` = ddf0aa8, `A` = this branch (0d26a3f), 158 problems for hashes (the 51 timing problems of 2 to 16 parameters plus the
large set without pop_002, pop_024, pop_033, pop_036, pop_051, pop_095, masses4_N5, masses5_N4).

## The problem

`rank1_case2` handled the Case-2 facets with one vanishing slack row and a zero pivot, but refused (sent to the walk) every call with an
active-row multiplier m with PTOL < m <= 1e-7 max(mu_max, 1) ("reason 6"), and every call whose symmetry check
|T[c,w_j] - mu_j| <= 1e-8 (1 + |mu_j|) failed ("reason 5"). On main ddf0aa8 these refusals were 88% to 99.8% of all walk time
(`mplcp-order`, `mplcp_order_study.md`). Both tests are absolute: a multiplier is a ratio of the scales of two constraints, a_c = sum mu_j a_j.

## The rule

In the scale of the dependency, mu_j is measured in units u_j = sqrt(M_cc / M_jj) (M the pLCP matrix, c the vanishing row, j the active
row; the same normalisation as `pivot_measure`), nu_j = |mu_j| / u_j.

* nu_j <= `MU_ZERO` = 1e-9: zero (rounding of the tableau), j is not in P-.
* nu_j > `MU_NONZERO` = 1e-6, mu_j > 0: j is in P-, the member A + {c} - {j} is a candidate, as before.
* in between: undecided, the walk runs (as for the pivot measure, same thresholds).
* the symmetry test is relative to the same scale: |T[c,w_j] - mu_j| <= 1e-8 (u_j + |mu_j|).

Measured nu (masses4_N3, rank-1 calls, 3s census): the multipliers the old guard refused fall in two clusters, 1,900 entries at
nu = 1e-1 to 1e-4 (refused only because mu_max was large) and 750 at nu = 1e-8 to 1e-13 (rounding of an exact zero), nothing between
1e-5 and 1e-8. In exact arithmetic any nonzero mu_j makes A + {c} - {j} independent (Lean `hinj_of_dependency`), so the first cluster
is safe to accept; the second is a zero. The member basis is then built by `tableau_for` (relative pivot floor, ddf0aa8), which also decides
whether it is numerically sound.

## Counters (3s table of `lp_census`, one census solve each)

| problem | Case-2 calls | answered by the enumeration | handed to the walk (before: ddf0aa8) | members the old guard refused, now in P- | refused, now zero | in the gap | symmetry refusals now passed |
|---|---:|---:|---:|---:|---:|---:|---:|
| masses4_N3 | 40,658 | 40,023 | 635 (3,517) | 1,844 | 1,547 | 394 | 304 |
| masses5_N3 | 100,403 | 99,209 | 1,194 (14,478) | 13,019 | 2,380 | 724 | 572 |
| masses3_N4 | 29,302 | 28,805 | 497 (2,627) | 280 | 1,747 | 366 | 131 |
| pop_005 | 55,758 | 52,185 | 3,573 (5,184) | 7 | 3,387 | 1,832 | 31 |
| rand_t12_x5_m20_s0 | 69,551 | 69,133 | 418 (1,192) | 0 | 1,037 | 376 | 0 |

The members columns count multipliers, not calls. The walks that remain are the gap band (nu between 1e-9 and 1e-6: 394 to 1,832
entries) and the cases of pivot measure and multiple rows. On masses4_N3 the remaining walks are 27% of the solve (894 us each).

## Gate

* **Tests:** 144 passed, 0 failed, 3 ignored (147 tests, 1-CPU array). The audit counter of the facet certificate (`certificate_audit`) now counts
  certified facets instead of their candidates: with the rank-1 enumeration a certified facet whose neighbours are all known returns no
  candidate, which left `certificate_changes_no_region` vacuous ("no facet was ever certified"); the test and its non-vacuity check are unchanged.
* **Region hashes** (rows at 1e-9 and sorted active sets), `A` against `base`: identical on 147 of 158. The 11 that differ:
  * rows only (same regions, same sorted active sets): masses3_N4, masses3_N5, masses3_N6, masses3_N7, pop_005, pop_007, pop_028, pop_032,
    pop_047 (rounding at 1e-9; at 1e-6 the hash is identical on masses3_N4, N5, N6, pop_007, pop_028, pop_032, pop_047, and differs on
    masses3_N7 and pop_005 by a single rounding step);
  * masses4_N4: same 16,312 regions and sets, 2 facet rows more (378,686 against 378,684);
  * masses5_N3: 5,266 regions against 5,268. The two regions only `base` has, [2, 15, 49] (Chebyshev radius 1.2e-5) and [37, 46, 53]
    (radius 6e-8), are slivers inside the regions [2, 15] and [37, 46] of both builds: the QP at their centres has the active sets [2, 15] and
    [37, 46] (`extra_regions`), so `base` had two overlapping slivers that `A` does not produce.
* **Coverage** (`lexcheck 3000`, base and A, the 11 problems above): 0 lexico-infeasible regions, 0 overlaps, 0 gaps on all 22 runs
  (330 to 3,000 feasible samples each; pop_032 56).
* **ResolveNone:** 0 on the five census problems (regions popped without a tableau, table 3j; `tableau_for` refusals 140 to 683 per run, all rescued by the second pivot order or refused in `candidate_law`, as on main).
* **Paired timing** (51 problems, 4 to 5 tasks each; a task runs `base`, `A` and a copy of `base` once, shuffled; median ratio over the tasks,
  ratio above 1 means `A` is faster; A/A is `base` against its copy):

| group (problems) | base/A | A/A |
|---|---:|---:|
| all (51) | 1.072 | 1.002 |
| masses (12) | 1.298 | 1.018 |
| not masses (39) | 1.010 | 0.997 |
| 2 to 5 parameters (30) | 1.002 | 1.001 |
| 6 to 16 parameters (21) | 1.180 | 1.003 |

| problem | params | regions | base ms | tasks | base/A | A/A |
|---|---:|---:|---:|---:|---:|---:|
| di_N10 | 2 | 11 | 0.1 | 5 | 1.023 | 1.008 |
| di_N12 | 2 | 11 | 0.2 | 5 | 1.006 | 1.006 |
| di_N15 | 2 | 11 | 0.2 | 5 | 1.010 | 1.005 |
| di_N20 | 2 | 11 | 0.3 | 5 | 1.004 | 1.009 |
| di_N25 | 2 | 11 | 0.4 | 5 | 0.992 | 0.986 |
| di_N30 | 2 | 11 | 0.5 | 5 | 0.998 | 1.000 |
| di_N4 | 2 | 9 | 0.1 | 5 | 0.985 | 1.000 |
| di_N6 | 2 | 11 | 0.1 | 5 | 1.011 | 0.990 |
| di_N8 | 2 | 11 | 0.1 | 5 | 1.027 | 1.000 |
| di_doc_N3 | 2 | 7 | 0.1 | 5 | 1.000 | 1.000 |
| dinv_N4 | 2 | 9 | 0.1 | 5 | 1.017 | 1.000 |
| dinv_N6 | 2 | 11 | 0.1 | 5 | 1.021 | 0.980 |
| dinv_N8 | 2 | 11 | 0.1 | 5 | 1.000 | 1.000 |
| masses2_N4 | 4 | 307 | 12.2 | 5 | 0.999 | 1.004 |
| masses2_N5 | 4 | 493 | 20.7 | 5 | 1.002 | 1.002 |
| masses2_N6 | 4 | 901 | 44.6 | 5 | 0.998 | 1.011 |
| masses2_N7 | 4 | 1641 | 81.6 | 5 | 0.992 | 0.999 |
| masses2_N8 | 4 | 2837 | 163.0 | 5 | 1.003 | 1.002 |
| masses3_N3 | 6 | 955 | 206.9 | 4 | 1.193 | 1.003 |
| masses3_N4 | 6 | 3643 | 1237.9 | 4 | 1.294 | 1.000 |
| masses4_N3 | 8 | 2947 | 3614.4 | 4 | 1.792 | 1.209 |
| masses5_N2 | 10 | 373 | 288.5 | 4 | 1.401 | 1.004 |
| masses6_N2 | 12 | 383 | 684.5 | 4 | 2.067 | 1.002 |
| masses7_N2 | 14 | 393 | 898.5 | 4 | 1.344 | 0.998 |
| masses8_N2 | 16 | 399 | 2201.8 | 4 | 2.145 | 1.001 |
| pop_005 | 7 | 10289 | 3230.9 | 4 | 1.398 | 0.999 |
| pop_017 | 6 | 4142 | 514.0 | 4 | 0.996 | 0.985 |
| pop_021 | 7 | 679 | 98.2 | 4 | 1.005 | 0.995 |
| pop_027 | 8 | 1472 | 311.2 | 4 | 0.999 | 1.001 |
| rand_t10_x5_m15_s0 | 10 | 1859 | 426.0 | 4 | 0.990 | 0.990 |
| rand_t10_x5_m20_s0 | 10 | 6124 | 1817.6 | 4 | 1.032 | 1.019 |
| rand_t12_x5_m15_s0 | 12 | 1012 | 329.8 | 4 | 1.004 | 1.000 |
| rand_t12_x5_m20_s0 | 12 | 6143 | 4202.4 | 4 | 1.009 | 0.896 |
| rand_t14_x5_m15_s0 | 14 | 2470 | 1399.0 | 4 | 0.954 | 0.987 |
| rand_t16_x5_m15_s0 | 16 | 1876 | 1400.4 | 4 | 1.007 | 0.997 |
| rand_t3_x5_m20_s0 | 3 | 123 | 2.3 | 4 | 0.940 | 1.007 |
| rand_t4_x5_m20_s0 | 4 | 797 | 33.1 | 4 | 0.973 | 1.013 |
| rand_t4_x5_m20_s1 | 4 | 484 | 16.8 | 4 | 0.987 | 1.006 |
| rand_t5_x5_m20_s0 | 5 | 1423 | 172.3 | 4 | 1.040 | 1.019 |
| rand_t5_x5_m20_s1 | 5 | 1136 | 84.8 | 4 | 0.997 | 1.003 |
| rand_t5_x5_m20_s2 | 5 | 2301 | 136.0 | 4 | 1.007 | 0.992 |
| rand_t5_x6_m24_s0 | 5 | 5036 | 507.1 | 4 | 1.000 | 1.013 |
| rand_t6_x5_m20_s0 | 6 | 1774 | 167.1 | 4 | 0.995 | 1.007 |
| rand_t6_x5_m20_s1 | 6 | 5062 | 531.7 | 4 | 1.002 | 0.999 |
| rand_t6_x6_m24_s0 | 6 | 17134 | 2201.5 | 4 | 1.002 | 0.997 |
| rand_t8_x5_m20_s0 | 8 | 4111 | 662.4 | 4 | 1.007 | 1.002 |
| sys3x2_N3 | 3 | 211 | 3.4 | 5 | 1.019 | 1.002 |
| sys3x2_N4 | 3 | 503 | 9.5 | 5 | 1.001 | 0.997 |
| sys3x2_N5 | 3 | 973 | 20.5 | 5 | 0.998 | 1.003 |
| sys3x2_N6 | 3 | 1503 | 35.6 | 5 | 1.012 | 1.005 |
| sys3x2_N7 | 3 | 2095 | 52.6 | 5 | 1.001 | 0.976 |

masses6_N2 and masses8_N2 (2.1x) and masses4_N3 (1.8x, its A/A is 1.209 in this run) are the largest gains, as `mplcp-order` predicted from `RPOPT_NO_R6=1` (masses8_N2 0.47x, masses7_N2 0.75x); pop_005 1.40x, rand_t12 1.01x.

## A wider zero band was tried and not kept

Raising `MU_ZERO` to 1e-7 (the gap band mostly gone: masses4_N3 walks 635 to 241) is faster still (masses 1.326, 6 to 16 parameters
1.217, masses4_N3 1.99x, rand_t12 1.49x), but it changes the regions: masses3_N6 22,144 and masses3_N7 38,327 (one more each, sets differ from `base`
and from the 1e-9 rule), masses5_N3 5,266. Accepting every gap multiplier as a member (no walk) gives the same extra region on masses3_N6. A
multiplier at 1e-8 relative is an exact zero in most calls but not all: where lambda_j is also near zero, j decides the piece of its neighbours,
and dropping it adds a candidate. So the band stays with the walk, which decides it as before. The extra regions were not examined further.
