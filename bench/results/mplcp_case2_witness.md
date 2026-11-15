# mplcp pivot_facet: witness-based nonemptiness, and three fixes from the Lean round

Branch `mplcp-case2-p3b`, on main b5c8f6d (phase 3 merged), merged with main 89d7ecb. Commits:

| commit | content |
|---|---|
| b1d7065 | witness-accepted nonempty sub-facets (first version: the skipped LP is run again when a leaf needs its centre) |
| cba1392 | the witness is the hint of a leaf below a witness-accepted piece |
| 0091136, 84ba098 | the cold split of `adjacent` and its revert |
| 305612c | the `Across` candidate is the lexicographic Lemke basis at the probe |
| 4352072 | a stopped walk also probes the sub-facets it did not finish |
| 52d4ffc, 7b6a659 | the comment and the results text on the walk bound |

Timings: serial, 1 CPU, release profile, Negishi (AMD EPYC 7763); one array task per (problem, alternation) with
the binaries under comparison run once each in a shuffled order on the same node, ratio taken inside the task,
median over 4 to 5 tasks per problem (each pair in its own task above 5 s). Ratios above 1 mean the second binary
is faster. "main" = b5c8f6d.

## Witness-based nonemptiness

A sub-facet's Chebyshev LP only decides whether the piece is nonempty. A non-leaf Case-2 branch is now accepted
without it when a known point clears every row of the piece by 1e-8 (the rows are unit vectors, so that is a
distance, and it certifies a Chebyshev radius above the LP's 1e-9 acceptance threshold). Candidates in order:
(a) a branch with no theta-dependent cut equals its parent polytope and inherits its witness; (b) the parent's
witness (its Chebyshev centre when its LP ran); (c) the region centre projected onto the facet (the old hint).
Emptiness still needs the LP, and a leaf branch (alpha leaves) always runs its LP, because its centre is the hint of
the returned neighbour.

The first version (b1d7065) computed the skipped LP again when a returned leaf below a witness-accepted piece
needed a centre. Nearly every accepted piece had such a leaf (24,710 accepted, 23,423 recomputed on
rand_t12_x5_m20_s0), so it saved almost nothing: LPs per region 48.4 to 48.1, and the timing was 1.00 to 1.02
(table below). The second version (cba1392) uses the witness as that hint. Region hashes stay identical to main on
all 49 problems of the standard set (23 bench, 7 generated at 3 to 5 parameters, 12 census at 6 to 10, 7
`problems_highdim` at 12 to 16) for the first version, the version with the witness as hint, and the final build
(job arrays 45884143, 45885647 and 45886342 on Negishi; the hint did not change a row anywhere).

Counters (`lp_census` table 3f, one run per problem; "before" = main, "after" = final build):

| params | problem | LP/region before | LP/region after | pivot_facet Chebyshev LPs/region before / after | branch checks answered by a witness | probes found / failed on stopped walks | budget stops |
|---:|---|---:|---:|---|---|---|---:|
| 3 | sys3x2_N7 | 1.1 | 1.0 | 0.0 / 0.0 | 24 (24%) | 0 / 0 | 0 |
| 4 | masses2_N8 | 3.7 | 3.0 | 2.7 / 2.0 | 1933 (25%) | 0 / 0 | 0 |
| 5 | rand_t5_x5_m20_s2 | 8.6 | 7.2 | 7.6 / 6.2 | 3308 (19%) | 0 / 0 | 0 |
| 6 | masses3_N3 | 51.8 | 49.0 | 20.5 / 17.6 | 2715 (14%) | 0 / 16 | 3 |
| 6 | pop_017 | 37.2 | 34.2 | 13.3 / 10.3 | 12375 (22%) | 0 / 0 | 0 |
| 7 | pop_005 | 56.8 | 54.7 | 21.8 / 19.7 | 21884 (10%) | 0 / 1010 | 99 |
| 7 | pop_021 | 39.3 | 35.5 | 18.8 / 15.0 | 2579 (20%) | 0 / 0 | 0 |
| 8 | masses4_N3 | 92.9 | 87.8 | 51.6 / 46.5 | 14756 (10%) | 10 / 303 | 26 |
| 8 | rand_t8_x5_m20_s0 | 32.3 | 29.3 | 10.2 / 7.2 | 12343 (30%) | 0 / 0 | 0 |
| 10 | masses5_N2 | 65.2 | 60.7 | 31.6 / 27.2 | 1651 (14%) | 4 / 81 | 8 |
| 12 | masses6_N2 | 72.9 | 68.1 | 35.5 / 30.7 | 1820 (13%) | 7 / 139 | 10 |
| 12 | rand_t12_x5_m15_s0 | 34.3 | 31.9 | 8.3 / 5.9 | 2419 (29%) | 0 / 0 | 0 |
| 12 | rand_t12_x5_m20_s0 | 48.4 | 44.3 | 21.3 / 17.2 | 24710 (19%) | 778 / 673 | 117 |
| 14 | masses7_N2 | 74.0 | 69.4 | 33.5 / 28.8 | 1804 (14%) | 3 / 65 | 5 |
| 14 | rand_t14_x5_m15_s0 | 40.8 | 38.1 | 10.9 / 8.2 | 6871 (25%) | 16 / 0 | 1 |
| 16 | masses8_N2 | 78.1 | 73.1 | 33.8 / 28.8 | 1963 (15%) | 0 / 99 | 8 |
| 16 | rand_t16_x5_m15_s0 | 44.4 | 41.7 | 10.1 / 7.3 | 5241 (28%) | 0 / 0 | 0 |

The witness answers 10% to 30% of the branch checks (25% on masses2_N8, 19% on rand_t12_x5_m20_s0, 30% on
rand_t8_x5_m20_s0) and cuts the LPs per region by 4% to 19% (rand_t8_x5_m20_s0 9.3%, rand_t12_x5_m20_s0 8.5%, masses4_N3 5.5%): the pieces that
need an LP are the ones the parent's centre does not lie in, and the cuts of a branch split its parent so that
the centre lies in at most one of them.

Time, final build against main (job array 45887281; A/A = main twice):

| params | problems | final | A/A |
|---:|---:|---|---|
| 2 | 13 | 0.995 (0.975 to 1.009) | 1.003 (0.994 to 1.015) |
| 3 | 6 | 1.003 (0.988 to 1.034) | 0.995 (0.983 to 1.001) |
| 4 | 7 | 1.020 (1.009 to 1.044) | 1.000 (0.993 to 1.004) |
| 5 | 4 | 1.026 (1.015 to 1.042) | 1.001 (0.999 to 1.002) |
| 6 | 4 | 1.024 (1.014 to 1.037) | 0.999 (0.995 to 1.004) |
| 7 | 2 | 1.021 (0.998 to 1.045) | 1.002 (1.001 to 1.003) |
| 8 | 3 | 1.036 (1.019 to 1.052) | 0.998 (0.996 to 1.000) |
| 10 | 3 | 1.050 (1.039 to 1.063) | 0.999 (0.996 to 1.002) |
| 12 | 3 | 1.061 (1.050 to 1.072) | 1.002 (0.999 to 1.006) |
| 14 | 2 | 1.064 (1.062 to 1.065) | 0.998 (0.998 to 0.998) |
| 16 | 2 | 1.059 (1.057 to 1.061) | 1.000 (0.998 to 1.002) |

For the first version (witness, recomputed centre, plus the cold split below; job array 45884145) the ratio
by parameter count was 0.99 to 1.02: 2 parameters 0.997, 4 parameters 0.989, 6 parameters 1.004, 12 parameters 1.015,
16 parameters 1.020. The gain comes from using the witness as the hint.

## The 1% to 2% at 2 to 5 parameters

The cold, out-of-line `adjacent_case2` (0091136) was 1.001 to 1.011 faster than the build before it by parameter
count (2 to 5 parameters 1.001 to 1.005, inside the A/A band 0.985 to 1.009), and the phase-3 integration build is
still 1.6% faster than it at 2 to 5 parameters (job array 45885506, 24 problems, A/A 1.00). The split does not resolve
the regression and was reverted (84ba098). The witness gain (1.00 at 2 to 3 parameters, 1.02 at 4 to 6) covers
it from 4 parameters on; at 2 to 3 parameters the final build is level with main (0.995 and 1.003).

## Fixes from the Lean round (Finding 10)

* **Across candidate (305612c).** The QP at the probe point returns an optimal active set, which can be a
  lexico-infeasible twin or rank-deficient, and `drop_lex_infeasible` is gone. The candidate is now `lex_lemke` at the
  probe, as in `lex_feasible_start`, and is rejected when the tableau of that basis does not exist or is not
  lexico-feasible.
* **Probes of the unfinished frames (4352072).** When the budget or a cycle stops a facet walk, the frames not
  yet walked (and the one in progress) are probed like the whole facet, at their Chebyshev centre or witness
  (at most 16 probes per facet, duplicates dropped). The cost is a few `lex_lemke` calls on the facets whose walk is stopped;
  solve times are unchanged (table above). Probes found / failed are in the counter table: 778 / 673
  on rand_t12_x5_m20_s0 (117 stopped walks), 0 / 1010 on pop_005 (99), 10 / 303 on masses4_N3 (26). A probe fails when the
  point is outside the feasible set (a facet on the boundary of the parameter set) or its basis is rank-deficient.
  At the default budget the regions are the same with or without the probes (hashes identical), so this is a safety
  margin, as expected.
* **Comment (52d4ffc).** The proved bound on the walk is the number of reachable label states, about n 2^n
  (`chain_length_bound`); `walk_no_repeat` needs a lexico-feasible start and in general finiteness comes from
  `walk_repeat_returns` plus the cycle check.

Coverage (`lexcheck`, 20,000 samples, final build; job array 45886343): no gap and no lexico-infeasible region on any
problem; overlaps only on pop_005 (191 sampled points, the known stray regions, as before).

```
f3 masses3_N3 regions 955 lex_infeasible 0 feasible_samples 5459 overlaps 0 gaps 0
f3 masses3_N4 regions 3643 lex_infeasible 0 feasible_samples 4971 overlaps 0 gaps 0
f3 masses4_N3 regions 2891 lex_infeasible 0 feasible_samples 3351 overlaps 0 gaps 0
f3 masses5_N2 regions 370 lex_infeasible 0 feasible_samples 2848 overlaps 0 gaps 0
f3 masses6_N2 regions 380 lex_infeasible 0 feasible_samples 1847 overlaps 0 gaps 0
f3 masses7_N2 regions 390 lex_infeasible 0 feasible_samples 1258 overlaps 0 gaps 0
f3 masses8_N2 regions 395 lex_infeasible 0 feasible_samples 861 overlaps 0 gaps 0
f3 pop_005 regions 10297 lex_infeasible 0 feasible_samples 6992 overlaps 191 gaps 0
f3 pop_017 regions 4142 lex_infeasible 0 feasible_samples 3193 overlaps 0 gaps 0
f3 rand_t12_x5_m20_s0 regions 6143 lex_infeasible 0 feasible_samples 8653 overlaps 0 gaps 0
f3 rand_t14_x5_m15_s0 regions 2470 lex_infeasible 0 feasible_samples 9989 overlaps 0 gaps 0
```

## Per problem, final build against main

| params | problem | regions | main ms | final | A/A |
|---:|---|---|---:|---:|---:|
| 2 | di_N10 | 11 / 11 | 0.152 | 0.987 | 1.006 |
| 2 | di_N12 | 11 / 11 | 0.178 | 0.989 | 0.994 |
| 2 | di_N15 | 11 / 11 | 0.225 | 1.004 | 0.996 |
| 2 | di_N20 | 11 / 11 | 0.319 | 0.997 | 1.006 |
| 2 | di_N25 | 11 / 11 | 0.411 | 1.005 | 0.998 |
| 2 | di_N30 | 11 / 11 | 0.556 | 0.996 | 1.004 |
| 2 | di_N4 | 9 / 9 | 0.077 | 0.975 | 1.000 |
| 2 | di_N6 | 11 / 11 | 0.111 | 1.009 | 1.000 |
| 2 | di_N8 | 11 / 11 | 0.131 | 1.000 | 1.008 |
| 2 | di_doc_N3 | 7 / 7 | 0.063 | 0.984 | 1.000 |
| 2 | dinv_N4 | 9 / 9 | 0.068 | 1.000 | 1.015 |
| 2 | dinv_N6 | 11 / 11 | 0.109 | 0.982 | 1.000 |
| 2 | dinv_N8 | 11 / 11 | 0.129 | 1.008 | 1.008 |
| 3 | rand_t3_x5_m20_s0 | 123 / 123 | 3.271 | 1.034 | 1.001 |
| 3 | sys3x2_N3 | 211 / 211 | 3.748 | 0.996 | 0.994 |
| 3 | sys3x2_N4 | 503 / 503 | 10.970 | 0.999 | 0.997 |
| 3 | sys3x2_N5 | 973 / 973 | 21.922 | 0.988 | 0.999 |
| 3 | sys3x2_N6 | 1503 / 1503 | 36.030 | 0.989 | 0.996 |
| 3 | sys3x2_N7 | 2095 / 2095 | 56.544 | 1.012 | 0.983 |
| 4 | masses2_N4 | 307 / 307 | 17.915 | 1.022 | 1.004 |
| 4 | masses2_N5 | 493 / 493 | 30.052 | 1.018 | 1.003 |
| 4 | masses2_N6 | 901 / 901 | 59.735 | 1.012 | 1.001 |
| 4 | masses2_N7 | 1641 / 1641 | 116.753 | 1.012 | 0.998 |
| 4 | masses2_N8 | 2837 / 2837 | 196.946 | 1.009 | 0.993 |
| 4 | rand_t4_x5_m20_s0 | 797 / 797 | 38.530 | 1.044 | 1.004 |
| 4 | rand_t4_x5_m20_s1 | 484 / 484 | 28.920 | 1.023 | 1.000 |
| 5 | rand_t5_x5_m20_s0 | 1423 / 1423 | 143.659 | 1.042 | 1.001 |
| 5 | rand_t5_x5_m20_s1 | 1136 / 1136 | 143.233 | 1.030 | 0.999 |
| 5 | rand_t5_x5_m20_s2 | 2301 / 2301 | 174.285 | 1.016 | 1.002 |
| 5 | rand_t5_x6_m24_s0 | 5036 / 5036 | 576.553 | 1.015 | 1.001 |
| 6 | masses3_N3 | 955 / 955 | 379.106 | 1.016 | 1.001 |
| 6 | masses3_N4 | 3643 / 3643 | 2611.314 | 1.014 | 1.004 |
| 6 | pop_017 | 4142 / 4142 | 791.590 | 1.037 | 0.996 |
| 6 | rand_t6_x5_m20_s0 | 1774 / 1774 | 248.085 | 1.029 | 0.995 |
| 7 | pop_005 | 10297 / 10297 | 5384.514 | 0.998 | 1.001 |
| 7 | pop_021 | 679 / 679 | 175.343 | 1.045 | 1.003 |
| 8 | masses4_N3 | 2891 / 2891 | 4316.819 | 1.019 | 0.996 |
| 8 | pop_027 | 1472 / 1472 | 564.621 | 1.037 | 1.000 |
| 8 | rand_t8_x5_m20_s0 | 4111 / 4111 | 1042.861 | 1.052 | 0.999 |
| 10 | masses5_N2 | 370 / 370 | 512.793 | 1.039 | 0.996 |
| 10 | rand_t10_x5_m15_s0 | 1859 / 1859 | 617.817 | 1.049 | 1.000 |
| 10 | rand_t10_x5_m20_s0 | 6124 / 6124 | 2839.709 | 1.063 | 1.002 |
| 12 | masses6_N2 | 380 / 380 | 968.513 | 1.072 | 1.002 |
| 12 | rand_t12_x5_m15_s0 | 1012 / 1012 | 499.216 | 1.063 | 1.006 |
| 12 | rand_t12_x5_m20_s0 | 6143 / 6143 | 5806.604 | 1.050 | 0.999 |
| 14 | masses7_N2 | 390 / 390 | 1645.528 | 1.065 | 0.998 |
| 14 | rand_t14_x5_m15_s0 | 2470 / 2470 | 2050.119 | 1.062 | 0.998 |
| 16 | masses8_N2 | 395 / 395 | 2289.509 | 1.061 | 1.002 |
| 16 | rand_t16_x5_m15_s0 | 1876 / 1876 | 2023.954 | 1.057 | 0.998 |

## Tests and gates

* `cargo test --profile gate` on the final tree (cba1392 plus the Lean-round commits, tree `f3`), one 1-CPU task per
  test (array 45886341, 144 tests listed): 141 passed, 0 failed, 3 ignored.
* Both builds compile: default and `--features lp_census` (`cargo check` with `--tests --examples`).
* Region hashes (49 problems) and `lexcheck` coverage: see above.
