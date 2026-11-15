# mplcp: chained bounding-box LPs in the LP-path region builder (6 or more parameters)

Branch `worktree-agent-a8095c6d163dbc0a4` (code commit 2914cb7; timings and gates below are from 91633fb, the same algorithm without the statistics call), on main 0312c9f. Negishi, serial, one CPU per task, job arrays 45911129 (gate) and 45911130 (timing). Release build for timing, gate profile for the gate.

## What changed

Above 5 parameters `mplcp_build::build_region` no longer calls the shared `gen_cr_from_prepared`; it runs `lp_path` (a copy of the shared redundancy step) whose 2n bounding-box LPs are replaced by `boxlp::box_chain`:

* the first LP starts at a vertex of the parent's box that lies on the shared (entry) facet and is feasible in the child (the parent's box points are kept with the queued region, `Item::boxpts`, 2n * n * 8 bytes: 1.0 KB at 8 parameters, 2.3 KB at 12, against about 53 KB for a queued tableau); without one (no entry facet, tableau rebuilt beyond the budget, parallel path) the engine's first LP from the centre gives the vertex;
* the other 2n - 1 objectives (all maxima, then all minima reversed) are a primal simplex from the previous optimum, with a rank-one update of the basis inverse W, of R = A W (so the ratio test reads one column), of the multipliers and of the slacks;
* every optimum is verified (recomputed vertex feasible within 1e-8, c a nonnegative combination of the basis rows); a failure sends that objective and the restart of the chain to the engine; Bland's rule after 8 zero-step pivots;
* a fallback guard switches the chain off for the rest of the solve when, after 100 regions, more than 20% of the objectives fell back (never triggered on the problems below);
* `mplcp::set_box_chain(false)` or `RPOPT_BOXCHAIN=0` (in `region_hash`, `lexcheck`) restores the shared builder; the shared builder is unchanged.

The box only screens rows (reject by the exact box, keep by an optimal vertex); the projection test and the per-row LPs are exact, so a different optimal vertex at a tie changes which stage decides a row, not the verdict, except for rows that touch the region only through a degenerate vertex.

## Gate (lexcheck: one solve gives regions, lexico-infeasible count, 1000 coverage samples, row hash, active-set hash, facet rows)

64 problems with 6 or more parameters (the standard and large sets; six memory-heavy POP problems, masses4_N5 and masses5_N4 left out, rand_t12/t14/t16 added), mode off (shared builder) against mode on.

| problem | regions | lex-infeasible | overlaps | gaps | active-set hash equal | facet rows (shared builder) | facet rows (chain) | difference |
|---|---:|---:|---:|---:|---|---:|---:|---:|
| masses3_N2 | 169 | 0/0 | 0/0 | 0/0 | yes | 3064 | 3065 | +1 (0.033%) |
| masses3_N3 | 955 | 0/0 | 0/0 | 0/0 | yes | 15444 | 15446 | +2 (0.013%) |
| masses3_N4 | 3643 | 0/0 | 0/0 | 0/0 | yes | 55765 | 55775 | +10 (0.018%) |
| masses3_N5 | 10436 | 0/0 | 0/0 | 0/0 | yes | 154937 | 154962 | +25 (0.016%) |
| masses3_N6 | 22143 | 0/0 | 0/0 | 0/0 | yes | 317537 | 317584 | +47 (0.015%) |
| masses3_N7 | 38326 | 0/0 | 0/0 | 0/0 | yes | 537287 | 537353 | +66 (0.012%) |
| masses4_N2 | 327 | 0/0 | 0/0 | 0/0 | yes | 9336 | 9338 | +2 (0.021%) |
| masses4_N3 | 2947 | 0/0 | 0/0 | 0/0 | yes | 74638 | 74678 | +40 (0.054%) |
| masses4_N4 | 16312 | 0/0 | 0/0 | 0/0 | yes | 378684 | 378826 | +142 (0.037%) |
| masses5_N2 | 373 | 0/0 | 0/0 | 0/0 | yes | 15888 | 15897 | +9 (0.057%) |
| masses5_N3 | 5268 | 0/0 | 0/0 | 0/0 | yes | 203098 | 203189 | +91 (0.045%) |
| pop_004 | 247 | 0/0 | 0/0 | 0/0 | yes | 4897 | 4897 | +0 (0.000%) |
| pop_005 | 10289 | 0/0 | 0/0 | 0/0 | yes | 163345 | 163346 | +1 (0.001%) |
| pop_007 | 12235 | 0/0 | 0/0 | 0/0 | yes | 262808 | 262816 | +8 (0.003%) |
| pop_010 | 36 | 0/0 | 0/0 | 0/0 | yes | 1356 | 1356 | +0 (0.000%) |
| pop_011 | 397 | 0/0 | 0/0 | 0/0 | yes | 5139 | 5139 | +0 (0.000%) |
| pop_016 | 384 | 0/0 | 0/0 | 0/0 | yes | 10031 | 10031 | +0 (0.000%) |
| pop_017 | 4142 | 0/0 | 0/0 | 0/0 | yes | 58974 | 58975 | +1 (0.002%) |
| pop_018 | 20800 | 0/0 | 0/0 | 0/0 | yes | 268896 | 268901 | +5 (0.002%) |
| pop_021 | 679 | 0/0 | 0/0 | 0/0 | yes | 13670 | 13670 | +0 (0.000%) |
| pop_025 | 5441 | 0/0 | 0/0 | 0/0 | yes | 77758 | 77759 | +1 (0.001%) |
| pop_027 | 1472 | 0/0 | 0/0 | 0/0 | yes | 37797 | 37797 | +0 (0.000%) |
| pop_028 | 82116 | 0/0 | 0/0 | 0/0 | yes | 1604516 | 1604584 | +68 (0.004%) |
| pop_030 | 286 | 0/0 | 0/0 | 0/0 | yes | 3852 | 3852 | +0 (0.000%) |
| pop_032 | 178950 | 0/0 | 0/0 | 0/0 | yes | 3309204 | 3309226 | +22 (0.001%) |
| pop_037 | 18615 | 0/0 | 0/0 | 0/0 | yes | 295652 | 295653 | +1 (0.000%) |
| pop_046 | 24 | 0/0 | 0/0 | 0/0 | yes | 324 | 324 | +0 (0.000%) |
| pop_047 | 48144 | 0/0 | 0/0 | 0/0 | yes | 843241 | 843289 | +48 (0.006%) |
| pop_049 | 9341 | 0/0 | 0/0 | 0/0 | yes | 132112 | 132112 | +0 (0.000%) |
| pop_053 | 2441 | 0/0 | 0/0 | 0/0 | yes | 33179 | 33179 | +0 (0.000%) |
| pop_054 | 20 | 0/0 | 0/0 | 0/0 | yes | 329 | 329 | +0 (0.000%) |
| pop_056 | 107 | 0/0 | 0/0 | 0/0 | yes | 1861 | 1861 | +0 (0.000%) |
| pop_061 | 33587 | 0/0 | 0/0 | 0/0 | yes | 659982 | 660028 | +46 (0.007%) |
| pop_065 | 1089 | 0/0 | 0/0 | 0/0 | yes | 21813 | 21813 | +0 (0.000%) |
| pop_068 | 86 | 0/0 | 0/0 | 0/0 | yes | 1484 | 1484 | +0 (0.000%) |
| pop_069 | 26 | 0/0 | 0/0 | 0/0 | yes | 615 | 615 | +0 (0.000%) |
| pop_073 | 23 | 0/0 | 0/0 | 0/0 | yes | 406 | 406 | +0 (0.000%) |
| pop_076 | 15006 | 0/0 | 0/0 | 0/0 | yes | 261641 | 261641 | +0 (0.000%) |
| pop_080 | 273 | 0/0 | 0/0 | 0/0 | yes | 4692 | 4692 | +0 (0.000%) |
| pop_081 | 4 | 0/0 | 0/0 | 0/0 | yes | 54 | 54 | +0 (0.000%) |
| pop_084 | 665 | 0/0 | 0/0 | 0/0 | yes | 9091 | 9092 | +1 (0.011%) |
| pop_085 | 121 | 0/0 | 0/0 | 0/0 | yes | 1969 | 1969 | +0 (0.000%) |
| pop_086 | 3625 | 0/0 | 0/0 | 0/0 | yes | 69717 | 69717 | +0 (0.000%) |
| pop_090 | 438 | 0/0 | 0/0 | 0/0 | yes | 10254 | 10254 | +0 (0.000%) |
| pop_091 | 145018 | 0/0 | 0/0 | 0/0 | yes | 2325460 | 2325475 | +15 (0.001%) |
| pop_097 | 20574 | 0/0 | 0/0 | 0/0 | yes | 404198 | 404202 | +4 (0.001%) |
| pop_098 | 269 | 0/0 | 0/0 | 0/0 | yes | 7170 | 7170 | +0 (0.000%) |
| rand_t10_x5_m15_s0 | 1859 | 0/0 | 0/0 | 0/0 | yes | 53809 | 53811 | +2 (0.004%) |
| rand_t10_x5_m20_s0 | 6124 | 0/0 | 0/0 | 0/0 | yes | 183036 | 183039 | +3 (0.002%) |
| rand_t10_x5_m20_s1 | 14848 | 0/0 | 0/0 | 0/0 | yes | 366578 | 366603 | +25 (0.007%) |
| rand_t10_x6_m18_s0 | 15312 | 0/0 | 0/0 | 0/0 | yes | 364129 | 364205 | +76 (0.021%) |
| rand_t10_x6_m24_s0 | 71219 | 0/0 | 0/0 | 0/0 | yes | 1719804 | 1720185 | +381 (0.022%) |
| rand_t12_x5_m15_s0 | 1012 | 0/0 | 0/0 | 0/0 | yes | 37751 | 37754 | +3 (0.008%) |
| rand_t12_x5_m20_s0 | 6143 | 0/0 | 0/0 | 0/0 | yes | 239957 | 240025 | +68 (0.028%) |
| rand_t14_x5_m15_s0 | 2470 | 0/0 | 0/0 | 0/0 | yes | 104462 | 104464 | +2 (0.002%) |
| rand_t16_x5_m15_s0 | 1876 | 0/0 | 0/0 | 0/0 | yes | 87768 | 87769 | +1 (0.001%) |
| rand_t6_x5_m20_s0 | 1774 | 0/0 | 0/0 | 0/0 | yes | 26522 | 26523 | +1 (0.004%) |
| rand_t6_x5_m20_s1 | 5062 | 0/0 | 0/0 | 0/0 | yes | 67327 | 67328 | +1 (0.001%) |
| rand_t6_x6_m24_s0 | 17134 | 0/0 | 0/0 | 0/0 | yes | 231384 | 231392 | +8 (0.003%) |
| rand_t8_x5_m15_s0 | 1035 | 0/0 | 0/0 | 0/0 | yes | 22690 | 22690 | +0 (0.000%) |
| rand_t8_x5_m20_s0 | 4111 | 0/0 | 0/0 | 0/0 | yes | 89618 | 89622 | +4 (0.004%) |
| rand_t8_x5_m20_s1 | 8737 | 0/0 | 0/0 | 0/0 | yes | 151189 | 151196 | +7 (0.005%) |
| rand_t8_x6_m18_s0 | 9378 | 0/0 | 0/0 | 0/0 | yes | 165893 | 165896 | +3 (0.002%) |
| rand_t8_x6_m24_s0 | 32475 | 0/0 | 0/0 | 0/0 | yes | 581181 | 581199 | +18 (0.003%) |
same regions, lex, coverage and active-set hash on 64 of 64 problems


The sorted active sets, the region counts, the lexico-infeasible counts (all 0) and the sampled gaps and overlaps are identical on all 64 problems. The rounded-row hash differs on 40 of them because the chain keeps slightly more facet rows (at most +0.06%, 1 to 381 rows) than the shared builder: the vertex test keeps a row when an optimal box vertex lies on it within 1e-9, and the two methods can pick different optimal vertices at a degenerate tie. Rows that touch the region only through such a vertex are redundant by the facet definition; this already happened between the clip and the LP path (`mplcp_build_speed.md`, item 3). An experiment that also counted every row in the basis at any visited vertex as touching raised the difference (masses4_N3 +205 rows) and slowed rand_t8 by 5% to 10%, so it was reverted (fe867f4).

## Timing (regbuild_ab: one task per (problem, alternation), modes off, on, off (A/A) and parametric_daqp, shuffled, 5 alternations; ratio inside the task, median over tasks; above 1 = chain faster)

| problem | regions | tasks | off ms | on/off speed-up (median) | min | max | A/A median | A/A min | A/A max | daqp ms | on ms | on/daqp |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| di_N10 | 11 | 5 | 0.2 | 1.065 | 1.037 | 1.083 | 1.077 | 1.015 | 1.103 | - | - | - |
| masses2_N6 | 901 | 5 | 39.4 | 0.994 | 0.987 | 1.006 | 1.002 | 0.988 | 1.004 | - | - | - |
| masses3_N3 | 955 | 5 | 203.7 | 1.104 | 1.069 | 1.135 | 1.021 | 0.973 | 1.029 | - | - | - |
| masses3_N4 | 3643 | 5 | 1234.1 | 1.031 | 0.967 | 1.071 | 0.982 | 0.975 | 1.009 | - | - | - |
| masses4_N3 | 2947 | 5 | 2905.3 | 1.024 | 0.959 | 1.072 | 1.009 | 0.955 | 1.058 | 905.5 | 2856.3 | 0.32 |
| pop_005 | 10289 | 5 | 3396.2 | 0.975 | 0.814 | 1.078 | 0.995 | 0.828 | 1.050 | - | - | - |
| pop_017 | 4142 | 5 | 495.6 | 1.036 | 1.023 | 1.041 | 1.002 | 0.821 | 1.122 | 417.9 | 476.3 | 0.88 |
| pop_021 | 679 | 5 | 103.2 | 0.980 | 0.973 | 1.020 | 0.993 | 0.987 | 1.020 | - | - | - |
| pop_027 | 1472 | 5 | 324.8 | 0.933 | 0.875 | 0.943 | 0.992 | 0.963 | 1.008 | - | - | - |
| rand_t10_x5_m15_s0 | 1859 | 5 | 414.6 | 1.245 | 1.242 | 1.262 | 1.007 | 0.999 | 1.011 | - | - | - |
| rand_t10_x5_m20_s0 | 6124 | 5 | 1674.3 | 1.146 | 1.144 | 1.172 | 1.000 | 0.996 | 1.005 | 1359.6 | 1461.5 | 0.93 |
| rand_t12_x5_m15_s0 | 1012 | 5 | 325.1 | 1.335 | 1.306 | 1.368 | 1.007 | 0.977 | 1.019 | - | - | - |
| rand_t12_x5_m20_s0 | 6061 | 5 | 3690.8 | 1.149 | 1.071 | 1.186 | 0.980 | 0.886 | 1.029 | 2069.4 | 3152.2 | 0.66 |
| rand_t14_x5_m15_s0 | 2470 | 5 | 1327.1 | 1.801 | 1.791 | 1.827 | 1.003 | 0.995 | 1.010 | - | - | - |
| rand_t16_x5_m15_s0 | 1876 | 5 | 1390.0 | 1.969 | 1.930 | 1.971 | 1.003 | 0.996 | 1.004 | - | - | - |
| rand_t6_x5_m20_s0 | 1774 | 5 | 162.9 | 1.053 | 1.025 | 1.083 | 0.996 | 0.988 | 1.006 | - | - | - |
| rand_t6_x6_m24_s0 | 17134 | 5 | 2222.6 | 1.062 | 1.057 | 1.075 | 0.992 | 0.988 | 1.005 | - | - | - |
| rand_t8_x5_m20_s0 | 4111 | 5 | 644.3 | 1.084 | 1.061 | 1.103 | 0.999 | 0.992 | 1.014 | 544.5 | 602.3 | 0.90 |
| rand_t8_x6_m24_s0 | 32475 | 5 | 7080.1 | 1.130 | 1.121 | 1.140 | 1.003 | 1.001 | 1.004 | - | - | - |

Medians over 5 tasks. 'on/daqp' column: parametric_daqp ms / chain ms (below 1: parametric_daqp faster). Node load 55 to 68 at task start. The 2 to 5 parameter problems (`di_N10`, `masses2_N6`) take the same code path in both modes: their ratios are the noise floor.

Acceptance rule (rand_t8, rand_t10, rand_t12 each at least 5%): met, 1.08x, 1.15x, 1.15x; rand_t10_x5_m15 1.25x, rand_t12_x5_m15 1.34x, rand_t14 1.80x, rand_t16 1.97x. Regressions beyond the A/A band: `pop_027` 0.93x in this run (0.97x in a one-alternation repeat, 1.03x in a run of a variant), `pop_005` 0.975x and `pop_021` 0.98x are at the edge of the noise (A/A min 0.83 for pop_005). `masses4_N3` 1.02x with 2.6 fallback objectives per region (16% of 2n).

## Fallback statistics (one chained solve per problem, 2914cb7)

| problem | regions built by chain | fallback objectives per region | share of 2n objectives | engine LPs per region | switched off |
|---|---:|---:|---:|---:|---|
| masses3_N3 | 954 | 0.91 | - | 1.90 | False |
| masses3_N4 | 3642 | 1.21 | - | 2.27 | False |
| masses4_N3 | 2946 | 2.64 | - | 4.73 | False |
| pop_005 | 10296 | 0.04 | - | 0.59 | False |
| pop_017 | 4141 | 0.21 | - | 0.74 | False |
| pop_021 | 678 | 0.00 | - | 1.11 | False |
| pop_027 | 1471 | 0.00 | - | 0.79 | False |
| rand_t10_x5_m15_s0 | 1858 | 0.08 | - | 0.83 | False |
| rand_t10_x5_m20_s0 | 6123 | 0.06 | - | 1.03 | False |
| rand_t12_x5_m15_s0 | 1011 | 0.27 | - | 0.64 | False |
| rand_t12_x5_m20_s0 | 6142 | 0.48 | - | 1.00 | False |
| rand_t14_x5_m15_s0 | 2469 | 0.14 | - | 0.54 | False |
| rand_t16_x5_m15_s0 | 1875 | 0.02 | - | 1.95 | False |
| rand_t6_x5_m20_s0 | 1773 | 0.08 | - | 0.37 | False |
| rand_t6_x6_m24_s0 | 17133 | 0.09 | - | 0.89 | False |
| rand_t8_x5_m20_s0 | 4110 | 0.15 | - | 0.48 | False |
| rand_t8_x6_m24_s0 | 32474 | 0.16 | - | 1.08 | False |

## Against parametric_daqp (same task, same node)

rand_t8 544 ms against 602 ms (mplcp now 1.11x slower, was about 1.2x), rand_t10 1360 against 1461 (1.07x), pop_017 418 against 476 (1.14x), rand_t12 2069 against 3152 (1.52x), masses4_N3 906 against 2856 (3.2x). The chain does not flip any of the five.

## Census background

`bench/results/mplcp_regbuild_study.md` (studies 1 to 11): why this was chosen over a Clarkson-style builder, a cheaper box, parent hints and tableau rows.
