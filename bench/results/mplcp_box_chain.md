# mplcp: chained bounding-box LPs in the LP-path region builder (6 or more parameters)

Branch `worktree-agent-a8095c6d163dbc0a4`, merged with main 59b2afb (commit 8622d44). Negishi, serial, one CPU per task, job arrays 45914185 (gate) and 45914186 (timing). Release build for timing, gate profile for the gate.

## What changed

Above 5 parameters `mplcp_build::build_region` no longer calls the shared `gen_cr_from_prepared`; it runs `lp_path` (a copy of the shared redundancy step) whose 2n bounding-box LPs are replaced by `boxlp::box_chain`:

* the first LP starts at a vertex of the parent's box that lies on the shared (entry) facet and is feasible in the child (the parent's box points are kept with the queued region, `Item::boxpts`, 2n * n * 8 bytes: 1.0 KB at 8 parameters, 2.3 KB at 12, against about 53 KB for a queued tableau); without one (no entry facet, tableau rebuilt beyond the budget, parallel path) the engine's first LP from the centre gives the vertex;
* the other 2n - 1 objectives (all maxima, then all minima reversed) are a primal simplex from the previous optimum, with a rank-one update of the basis inverse W, of R = A W (so the ratio test reads one column), of the multipliers and of the slacks;
* every optimum is verified (recomputed vertex feasible within 1e-8, c a nonnegative combination of the basis rows); a failure sends that objective and the restart of the chain to the engine; Bland's rule after 8 zero-step pivots;
* a fallback guard switches the chain off for the rest of the solve when, after 100 regions, more than 20% of the objectives fell back (it switches off on masses5_N3 after about 2,500 of 3,500 regions; no other problem below);
* `mplcp::set_box_chain(false)` or `RPOPT_BOXCHAIN=0` (in `region_hash`, `lexcheck`) restores the shared builder; the shared builder is unchanged.

The box only screens rows (reject by the exact box, keep by an optimal vertex); the projection test and the per-row LPs are exact, so a different optimal vertex at a tie changes which stage decides a row, not the verdict, except for rows that touch the region only through a degenerate vertex.

## Gate (lexcheck: one solve gives regions, lexico-infeasible count, 1000 coverage samples, row hash, active-set hash, facet rows)

64 problems with 6 or more parameters (the standard and large sets; six memory-heavy POP problems, masses4_N5 and masses5_N4 left out, rand_t12/t14/t16 added), mode off (shared builder) against mode on.

| problem | regions | lex-infeasible | overlaps | gaps | active-set hash equal | facet rows (shared builder) | facet rows (chain) | difference |
|---|---:|---:|---:|---:|---|---:|---:|---:|
| masses3_N2 | 169 | 0/0 | 0/0 | 0/0 | yes | 3064 | 3065 | +1 (0.033%) |
| masses3_N3 | 955 | 0/0 | 0/0 | 0/0 | yes | 15444 | 15446 | +2 (0.013%) |
| masses3_N4 | 3643 | 0/0 | 0/0 | 0/0 | yes | 55765 | 55781 | +16 (0.029%) |
| masses3_N5 | 10436 | 0/0 | 0/0 | 0/0 | yes | 154937 | 154963 | +26 (0.017%) |
| masses3_N6 | 22143 | 0/0 | 0/0 | 0/0 | yes | 317537 | 317580 | +43 (0.014%) |
| masses3_N7 | 38326 | 0/0 | 0/0 | 0/0 | yes | 537287 | 537355 | +68 (0.013%) |
| masses4_N2 | 327 | 0/0 | 0/0 | 0/0 | yes | 9336 | 9338 | +2 (0.021%) |
| masses4_N3 | 2947 | 0/0 | 0/0 | 0/0 | yes | 74638 | 74672 | +34 (0.046%) |
| masses4_N4 | 16312 | 0/0 | 0/0 | 0/0 | yes | 378686 | 378839 | +153 (0.040%) |
| masses5_N2 | 373 | 0/0 | 0/0 | 0/0 | yes | 15888 | 15897 | +9 (0.057%) |
| masses5_N3 | 5266 | 0/0 | 0/0 | 0/0 | yes | 203035 | 203135 | +100 (0.049%) |
| pop_004 | 247 | 0/0 | 0/0 | 0/0 | yes | 4897 | 4897 | +0 (0.000%) |
| pop_005 | 10289 | 0/0 | 0/0 | 0/0 | yes | 163345 | 163346 | +1 (0.001%) |
| pop_007 | 12235 | 0/0 | 0/0 | 0/0 | yes | 262808 | 262817 | +9 (0.003%) |
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
| pop_032 | 178950 | 0/0 | 0/0 | 0/0 | yes | 3309204 | 3309224 | +20 (0.001%) |
| pop_037 | 18615 | 0/0 | 0/0 | 0/0 | yes | 295652 | 295653 | +1 (0.000%) |
| pop_046 | 24 | 0/0 | 0/0 | 0/0 | yes | 324 | 324 | +0 (0.000%) |
| pop_047 | 48144 | 0/0 | 0/0 | 0/0 | yes | 843241 | 843286 | +45 (0.005%) |
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
| pop_091 | 145018 | 0/0 | 0/0 | 0/0 | yes | 2325460 | 2325476 | +16 (0.001%) |
| pop_097 | 20574 | 0/0 | 0/0 | 0/0 | yes | 404198 | 404206 | +8 (0.002%) |
| pop_098 | 269 | 0/0 | 0/0 | 0/0 | yes | 7170 | 7170 | +0 (0.000%) |
| rand_t10_x5_m15_s0 | 1859 | 0/0 | 0/0 | 0/0 | yes | 53809 | 53811 | +2 (0.004%) |
| rand_t10_x5_m20_s0 | 6124 | 0/0 | 0/0 | 0/0 | yes | 183036 | 183039 | +3 (0.002%) |
| rand_t10_x5_m20_s1 | 14848 | 0/0 | 0/0 | 0/0 | yes | 366578 | 366603 | +25 (0.007%) |
| rand_t10_x6_m18_s0 | 15312 | 0/0 | 0/0 | 0/0 | yes | 364129 | 364201 | +72 (0.020%) |
| rand_t10_x6_m24_s0 | 71219 | 0/0 | 0/0 | 0/0 | yes | 1719804 | 1720189 | +385 (0.022%) |
| rand_t12_x5_m15_s0 | 1012 | 0/0 | 0/0 | 0/0 | yes | 37751 | 37754 | +3 (0.008%) |
| rand_t12_x5_m20_s0 | 6143 | 0/0 | 0/0 | 0/0 | yes | 239957 | 240022 | +65 (0.027%) |
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


The sorted active sets, the region counts, the lexico-infeasible counts (all 0) and the sampled gaps and overlaps are identical on all 64 problems. The rounded-row hash differs on most of the problems with 8 or more parameters because the chain keeps slightly more facet rows (at most +0.06%) than the shared builder: the vertex test keeps a row when an optimal box vertex lies on it within 1e-9, and the two methods can pick different optimal vertices at a degenerate tie. Rows that touch the region only through such a vertex are redundant by the facet definition; this already happened between the clip and the LP path (`mplcp_build_speed.md`, item 3). An experiment that also counted every row in the basis at any visited vertex as touching raised the difference (masses4_N3 +205 rows) and slowed rand_t8 by 5% to 10%, so it was reverted (fe867f4).

## Timing (regbuild_ab: one task per (problem, alternation), modes off, on, off (A/A) and parametric_daqp, shuffled, 5 alternations; ratio inside the task, median over tasks; above 1 = chain faster)

| problem | regions | tasks | off ms | on/off speed-up (median) | min | max | A/A median | A/A min | A/A max | daqp ms | on ms | on/daqp |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| di_N10 | 11 | 5 | 0.1 | 1.051 | 0.958 | 1.097 | 1.027 | 0.975 | 1.071 | - | - | - |
| masses2_N6 | 901 | 5 | 39.3 | 1.000 | 0.997 | 1.004 | 0.999 | 0.997 | 1.002 | - | - | - |
| masses3_N3 | 955 | 5 | 167.9 | 1.085 | 1.064 | 1.110 | 1.001 | 0.983 | 1.014 | - | - | - |
| masses3_N4 | 3643 | 5 | 933.8 | 1.078 | 1.054 | 1.083 | 1.003 | 1.000 | 1.005 | - | - | - |
| masses4_N3 | 2947 | 5 | 2117.9 | 1.031 | 0.856 | 1.045 | 0.999 | 0.941 | 1.046 | 921.6 | 2168.1 | 0.43 |
| masses5_N3 | 3503 | 5 | 8831.5 | 1.003 | 0.983 | 1.012 | 0.999 | 0.984 | 1.015 | 3045.8 | 8804.9 | 0.35 |
| pop_005 | 10289 | 5 | 2265.4 | 1.077 | 1.074 | 1.079 | 1.004 | 1.001 | 1.008 | - | - | - |
| pop_017 | 4142 | 5 | 487.4 | 1.069 | 1.057 | 1.118 | 1.005 | 0.994 | 1.047 | 408.0 | 459.0 | 0.89 |
| pop_021 | 679 | 5 | 96.3 | 1.041 | 1.040 | 1.044 | 1.000 | 0.999 | 1.001 | - | - | - |
| pop_027 | 1472 | 5 | 309.6 | 1.000 | 0.990 | 1.006 | 0.989 | 0.989 | 1.009 | - | - | - |
| rand_t10_x5_m15_s0 | 1859 | 5 | 405.8 | 1.250 | 1.248 | 1.266 | 0.994 | 0.993 | 0.996 | - | - | - |
| rand_t10_x5_m20_s0 | 6124 | 5 | 1635.8 | 1.182 | 1.180 | 1.192 | 0.999 | 0.992 | 1.008 | 1344.4 | 1383.2 | 0.97 |
| rand_t12_x5_m15_s0 | 1012 | 5 | 325.7 | 1.430 | 1.392 | 1.484 | 1.007 | 0.996 | 1.035 | - | - | - |
| rand_t12_x5_m20_s0 | 6061 | 5 | 3122.3 | 1.111 | 1.066 | 1.203 | 1.003 | 0.974 | 1.080 | 2105.2 | 2810.9 | 0.75 |
| rand_t14_x5_m15_s0 | 2470 | 5 | 1315.5 | 1.841 | 1.826 | 1.855 | 1.005 | 0.978 | 1.007 | - | - | - |
| rand_t16_x5_m15_s0 | 1876 | 5 | 1391.1 | 2.003 | 1.981 | 2.032 | 1.004 | 1.001 | 1.014 | - | - | - |
| rand_t6_x5_m20_s0 | 1774 | 5 | 159.1 | 1.080 | 1.052 | 1.083 | 0.989 | 0.986 | 0.999 | - | - | - |
| rand_t6_x6_m24_s0 | 17134 | 5 | 2199.3 | 1.085 | 1.066 | 1.094 | 1.001 | 0.983 | 1.018 | - | - | - |
| rand_t8_x5_m20_s0 | 4111 | 5 | 668.4 | 1.077 | 0.956 | 1.096 | 1.000 | 0.982 | 1.062 | 565.8 | 669.8 | 0.84 |
| rand_t8_x6_m24_s0 | 32475 | 5 | 6962.9 | 1.123 | 1.117 | 1.149 | 0.998 | 0.996 | 1.002 | - | - | - |

Medians over 5 tasks. 'on/daqp' column: parametric_daqp ms / chain ms (below 1: parametric_daqp faster). Node load 55 to 68 at task start. The 2 to 5 parameter problems (`di_N10`, `masses2_N6`) take the same code path in both modes: their ratios are the noise floor.

Acceptance rule (rand_t8, rand_t10, rand_t12 each at least 5%): met, 1.08x, 1.18x, 1.11x; rand_t10_x5_m15 1.25x, rand_t12_x5_m15 1.43x, rand_t14 1.84x, rand_t16 2.00x. No problem is slower beyond its A/A band: `pop_027` 1.00x, `masses5_N3` 1.00x (the guard turns the chain off), `masses4_N3` 1.03x with 2.7 fallback objectives per region (16% of 2n). An earlier run on main 0312c9f (91633fb) had `pop_027` at 0.93x and `pop_005` at 0.975x; with the merged main (59b2afb) both are at 1.00x and 1.08x.

## Fallback statistics (one chained solve per problem)

| problem | regions built by chain | fallback objectives per region | share of 2n objectives | engine LPs per region | switched off |
|---|---:|---:|---:|---:|---|
| masses3_N3 | 954 | 0.91 | - | 1.90 | False |
| masses3_N4 | 3642 | 1.22 | - | 2.27 | False |
| masses4_N3 | 2946 | 2.65 | - | 4.74 | False |
| masses5_N3 | 2523 | 4.00 | - | 7.45 | True |
| pop_005 | 10294 | 0.04 | - | 0.59 | False |
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

mplcp with the chain against parametric_daqp: rand_t8 670 ms against 566 ms (1.18x slower), rand_t10 1383 against 1344 (1.03x), pop_017 459 against 408 (1.13x), rand_t12 2811 against 2105 (1.34x), masses4_N3 2168 against 922 (2.35x), masses5_N3 8805 against 3046 (2.9x). The chain closes most of the gap on rand_t10 and does not flip any of the six.

## Census background

`bench/results/mplcp_regbuild_study.md` (studies 1 to 11): why this was chosen over a Clarkson-style builder, a cheaper box, parent hints and tableau rows.
