# mplcp region build: profile, speedups and rejected ideas

Branch `mplcp-build-speed` (merged with main 1ee2e20), baseline main f017013 (the source of `src/` is unchanged
between f017013 and 1ee2e20). All timings are serial (1 thread), on Negishi (AMD EPYC 7763), one single-CPU SLURM
array task per (problem, alternation); inside a task the binaries under comparison run once each (reps as listed)
in a shuffled order on the same node, and the ratio is taken inside the task. The reported ratio of a problem is the
median over its tasks. Release profile for every timed binary. Loads at task start: median 55, range 20 to 110
(r1, 29 nodes); 38 (29 to 85) for r3 and 47 (29 to 78) for r2. A/A: the same main binary twice in the same tasks.

## What the build costs (profile)

Timers inside `build_region` (job 45868726, node a053, load 87, ratios only; the 3 to 4 parameter split is a laptop run
of 2,000 regions, 8 us per region):

| parameters | where the build time goes |
|---|---|
| 3 (sys3x2_N7) | clip 44%, Chebyshev LP 27%, KKT rows 9%, row preparation 10%, output rows (`remove_duplicate_rows`, selection) 14% |
| 4 (masses2_N8) | clip 62%, Chebyshev LP 21%, rows 15% (output stage 14% before the change) |
| 6, clip succeeds | 1.1 ms per region (masses3_N4), law and Chebyshev under 1% |
| 6, clip fails (the 600-vertex guard; 14% to 25% of regions on masses3_N4 and pop_017) | first clip about 39 ms, the retry inside the shared builder another 39 ms, the LP path 0.07 ms. The retry was 46% of the whole build on masses3_N4 and 48% on pop_017, and it never succeeded (0 successes in every run) |
| 7 to 10 | the clip returns at once (nt > 6); the LP path is 90% of the build, of which `lp_flags` is 77 to 85%; the rows, normalisation and Chebyshev LP were done twice (8 to 10% of the build) |

Bounding-box counters (census problems, per region, deterministic; instrumented run, job 45874968):

| params | problem | rows | rows live | bbox LPs | dropped by the box | kept by a box vertex (of which no ray hits them) | kept by the projection test | per-row LP kept / dropped |
|---|---|---:|---:|---:|---:|---:|---:|---|
| 6 | masses3_N3 | 51.4 | 47.4 | 12 | 12.8 | 15.8 (8.7) | 0.12 | 1.1 / 17.6 |
| 6 | pop_017 | 36.0 | 32.5 | 12 | 8.1 | 13.5 (5.7) | 0.11 | 0.8 / 10.0 |
| 6 | rand_t6_x5_m20_s0 | 36.0 | 31.0 | 12 | 9.1 | 13.6 (6.1) | 0.12 | 1.5 / 6.7 |
| 7 | pop_005 | 62.0 | 57.2 | 14 | 22.3 | 14.8 (6.5) | 0.09 | 1.1 / 19.0 |
| 7 | pop_021 | 33.0 | 32.7 | 14 | 8.0 | 19.1 (10.5) | 0.18 | 1.6 / 3.9 |
| 8 | masses4_N3 | 67.3 | 64.2 | 16 | 16.7 | 23.2 (14.1) | 0.11 | 2.1 / 22.2 |
| 8 | pop_027 | 50.0 | 49.1 | 16 | 12.0 | 22.7 (12.2) | 0.07 | 4.1 / 10.2 |
| 8 | rand_t8_x5_m20_s0 | 38.0 | 36.0 | 16 | 10.8 | 20.0 (10.3) | 0.05 | 1.8 / 3.3 |
| 10 | rand_t10_x5_m15_s0 | 38.0 | 35.0 | 20 | 5.8 | 27.9 (15.8) | 0.05 | 1.1 / 0.1 |
| 10 | rand_t10_x5_m20_s0 | 43.0 | 40.0 | 20 | 9.1 | 27.9 (15.9) | 0.05 | 2.0 / 0.9 |

Answer to "what is the bounding box used for": only to screen rows in `lp_flags` (drop a row whose maximum over the
box is below its right-hand side; keep a row that touches one of the 2n box vertices). It is not part of the region.
Without it, the rows it decides would need an LP each: 5 to 22 dropped rows plus 6 to 16 kept rows that no ray
certifies, per region, against 12 to 20 box LPs. So it pays, and the timing below confirms it.

## Kept changes

Each is its own commit. Row identity is checked with `region_hash` (active set, facet rows rounded at 1e-9, sorted by
active set) against main f017013 on the 42-problem standard set (23 bench problems, the 12 census problems at 6 to 10
parameters, 7 generated random problems at 3 to 5 parameters), and for graph and geometric on the 23 bench problems.

### 1. No double build on the LP path (0ff754b)
At 6 or more parameters `build_region` computed the KKT rows, the normalisation and the Chebyshev LP, found that the
clip did not apply, and then called the shared builder, which did all of it again and tried the clip again. It now
hands the prepared rows and the centre to the redundancy step (`mpqp_utils::gen_cr_from_prepared`, with the clip
disabled), and `complete_primal_law` runs after the radius test (it only fills output fields).
Hash (mplcp): identical to main on all 42 problems (job array 45872345). Graph and geometric were hashed on the tree that adds item 2 (23 bench problems each, identical).
Effect: 6 parameters 1.49x (masses3_N3), 1.75x (masses3_N4), 1.88x (pop_017), 0.99x (rand_t6, which rarely fails the clip);
7 to 10 parameters 1.01x to 1.03x; 2 to 5 parameters 1.00x.

### 2. `remove_duplicate_rows` by pairwise comparison (9ddbb4a)
It hashed a `Vec<u64>` per row with SipHash. Pairwise comparison with early exit keeps the same rows (first occurrence
wins). 0.7 us less per region at 3 to 4 parameters.
Hash: identical on all 42 problems and on graph and geometric (23 each). Effect: 2 parameters 1.03x (1.006 to 1.06),
3 parameters 1.02x, 4 parameters 1.02x, 5 parameters 1.00x, 7 to 8 parameters 1.01x to 1.02x.

### 3. Vertex clipping only up to 5 parameters, in mplcp's builder (f4597fe) and in the shared builder (6b490e9)
At 6 parameters the clip was the whole cost: hundreds of vertices per region, and a failure at the guard cost twice as
much again. The LP path takes 0.1 ms. Sweep (job 45869468, 9 problems, cap = 3, 4, 5, 6 and vertex guards 40 to 300):
at 5 parameters the clip is 10% faster than the LP path (0.167 against 0.198 s on rand_t5_x5_m20_s0, 0.671 against
0.750 s on rand_t5_x6_m24_s0), at 4 parameters 25% faster, at 6 parameters 1.2x to 10x slower. Lower vertex guards were
worse than the LP path. Cap: 5.
Hash: same rows as main on 3 to 5 parameters (all cap values give identical hashes there). At 6 parameters 3 of
6 problems differ for mplcp: masses3_N3 15,449 to 15,444 facet rows, masses3_N4 55,776 to 55,765, rand_t6_x6_m24_s0 231,385 to 231,384
(that one from the knob sweep, job 45869468); pop_017 and the two rand_t6_x5 problems are identical. For graph and geometric (shared builder): masses3_N3 graph 15,452 and geometric 15,451 to
15,444 (now equal to mplcp's, and graph and geometric agree with each other: hash 6ea64c05), pop_017 geometric 58,941 to 58,942,
rand_t6_x6 graph 231,385 to 231,384.
Explanation (diagnostic run comparing the clip's flags with the LP path's on masses3_N3, `CLIPDIFF`): every differing row sits in a
sliver region. One case: Chebyshev radius 2.0e-7, and the clip kept a row whose hyperplane is 0.0179 away from the
region (the clip's vertex tolerance is meaningless on a sliver of that aspect ratio); the others have gaps of 5e-9 and 9e-10
to the hyperplane (kept by the clip's 1e-9 vertex test, dropped by the LP's feasibility test). The LP path's answer is
the correct one, and all three solvers now agree on masses3_N3. The region count and the active sets are unchanged.
Effect (mplcp): 6 parameters 2.4x (masses3_N3), 7.2x (masses3_N4), 11.7x (pop_017), 1.63x (rand_t6), other sizes 1.00x.
Effect on graph and geometric at 6 parameters: table below.

### Per-problem table, mplcp (job array 45874969, 215 tasks)
Ratios are paired medians (above 1: faster). "main / final" is the product of the three. For masses3_N4 and pop_017
(seconds per solve) every pair was timed in its own task with one repetition after a warm-up.

| params | problem | regions | main ms | no double build, no re-clip | pairwise duplicate rows | clip up to 5 | main / final (chained) | final ms |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 2 | di_N10 | 11 | 0.166 | 1.000 | 1.025 | 1.006 | 1.031 | 0.161 |
| 2 | di_N12 | 11 | 0.196 | 1.000 | 1.031 | 0.995 | 1.026 | 0.191 |
| 2 | di_N15 | 11 | 0.252 | 0.992 | 1.016 | 1.012 | 1.021 | 0.247 |
| 2 | di_N20 | 11 | 0.366 | 1.000 | 1.022 | 1.000 | 1.022 | 0.358 |
| 2 | di_N25 | 11 | 0.481 | 1.004 | 1.006 | 0.996 | 1.006 | 0.478 |
| 2 | di_N30 | 11 | 0.668 | 0.997 | 1.012 | 1.004 | 1.013 | 0.659 |
| 2 | di_N4 | 9 | 0.078 | 0.987 | 1.053 | 1.014 | 1.054 | 0.074 |
| 2 | di_N6 | 11 | 0.116 | 1.009 | 1.044 | 0.991 | 1.043 | 0.111 |
| 2 | di_N8 | 11 | 0.138 | 1.007 | 1.023 | 0.993 | 1.022 | 0.135 |
| 2 | di_doc_N3 | 7 | 0.067 | 0.985 | 1.062 | 1.000 | 1.047 | 0.064 |
| 2 | dinv_N4 | 9 | 0.071 | 1.000 | 1.044 | 1.015 | 1.060 | 0.067 |
| 2 | dinv_N6 | 11 | 0.119 | 1.017 | 1.027 | 1.000 | 1.045 | 0.114 |
| 2 | dinv_N8 | 11 | 0.137 | 0.986 | 1.030 | 1.015 | 1.030 | 0.133 |
| 3 | rand_t3_x5_m20_s0 | 123 | 3.239 | 1.002 | 1.033 | 1.001 | 1.036 | 3.126 |
| 3 | sys3x2_N3 | 211 | 3.958 | 0.994 | 1.027 | 1.003 | 1.024 | 3.866 |
| 3 | sys3x2_N4 | 503 | 11.077 | 0.996 | 1.031 | 0.993 | 1.019 | 10.868 |
| 3 | sys3x2_N5 | 973 | 23.602 | 1.002 | 1.024 | 0.999 | 1.026 | 23.003 |
| 3 | sys3x2_N6 | 1503 | 39.395 | 1.002 | 1.013 | 1.001 | 1.016 | 38.767 |
| 3 | sys3x2_N7 | 2095 | 58.168 | 1.003 | 1.011 | 0.997 | 1.011 | 57.551 |
| 4 | masses2_N4 | 307 | 17.893 | 1.002 | 1.013 | 1.002 | 1.017 | 17.594 |
| 4 | masses2_N5 | 493 | 30.397 | 0.999 | 1.017 | 1.002 | 1.019 | 29.841 |
| 4 | masses2_N6 | 901 | 61.436 | 1.002 | 1.015 | 0.996 | 1.013 | 60.659 |
| 4 | masses2_N7 | 1641 | 118.241 | 1.006 | 1.012 | 0.989 | 1.006 | 117.481 |
| 4 | masses2_N8 | 2837 | 223.731 | 1.008 | 1.004 | 1.004 | 1.017 | 220.035 |
| 4 | rand_t4_x5_m20_s0 | 797 | 39.701 | 1.000 | 1.064 | 0.981 | 1.044 | 38.026 |
| 4 | rand_t4_x5_m20_s1 | 484 | 28.474 | 0.995 | 1.021 | 1.006 | 1.022 | 27.854 |
| 5 | rand_t5_x5_m20_s0 | 1423 | 145.577 | 0.997 | 1.002 | 1.017 | 1.016 | 143.289 |
| 5 | rand_t5_x5_m20_s1 | 1136 | 141.144 | 0.973 | 1.004 | 1.008 | 0.985 | 143.235 |
| 5 | rand_t5_x5_m20_s2 | 2301 | 181.164 | 1.013 | 1.004 | 1.020 | 1.038 | 174.539 |
| 5 | rand_t5_x6_m24_s0 | 5036 | 582.279 | 1.000 | 1.005 | 1.009 | 1.014 | 574.272 |
| 6 | masses3_N3 | 955 | 1607.885 | 1.485 | 0.993 | 2.395 | 3.532 | 455.254 |
| 6 | masses3_N4 | 3643 | 44900.499 | 1.752 | 0.980 | 7.187 | 12.340 | 3638.743 |
| 6 | pop_017 | 4142 | 17601.934 | 1.876 | 0.983 | 11.653 | 21.477 | 819.555 |
| 6 | rand_t6_x5_m20_s0 | 1774 | 439.033 | 0.990 | 1.029 | 1.633 | 1.664 | 263.902 |
| 7 | pop_005 | 10298 | 8163.840 | 1.001 | 1.016 | 0.995 | 1.012 | 8069.306 |
| 7 | pop_021 | 679 | 203.449 | 1.015 | 1.021 | 0.996 | 1.032 | 197.228 |
| 8 | masses4_N3 | 2892 | 5067.640 | 1.015 | 1.005 | 0.996 | 1.016 | 4989.379 |
| 8 | pop_027 | 1472 | 580.364 | 1.022 | 1.009 | 0.993 | 1.025 | 566.410 |
| 8 | rand_t8_x5_m20_s0 | 4111 | 1069.582 | 1.020 | 1.022 | 0.997 | 1.039 | 1029.390 |
| 10 | masses5_N2 | 370 | 594.759 | 1.024 | 1.002 | 1.000 | 1.025 | 580.118 |
| 10 | rand_t10_x5_m15_s0 | 1859 | 642.846 | 1.033 | 1.002 | 1.001 | 1.036 | 620.356 |
| 10 | rand_t10_x5_m20_s0 | 6124 | 2896.145 | 1.032 | 0.992 | 1.005 | 1.029 | 2815.871 |
| params | problems | no double build | pairwise duplicate rows | clip up to 5 | main / final |
|---:|---:|---|---|---|---|
| 2 | 13 | 0.999 (0.985 to 1.017) | 1.030 (1.006 to 1.062) | 1.003 (0.991 to 1.015) | 1.032 (1.006 to 1.060) |
| 3 | 6 | 1.000 (0.994 to 1.003) | 1.023 (1.011 to 1.033) | 0.999 (0.993 to 1.003) | 1.022 (1.011 to 1.036) |
| 4 | 7 | 1.002 (0.995 to 1.008) | 1.021 (1.004 to 1.064) | 0.997 (0.981 to 1.006) | 1.020 (1.006 to 1.044) |
| 5 | 4 | 0.996 (0.973 to 1.013) | 1.004 (1.002 to 1.005) | 1.014 (1.008 to 1.020) | 1.013 (0.985 to 1.038) |
| 6 | 4 | 1.483 (0.990 to 1.876) | 0.996 (0.980 to 1.029) | 4.254 (1.633 to 11.653) | 6.282 (1.664 to 21.477) |
| 7 | 2 | 1.008 (1.001 to 1.015) | 1.018 (1.016 to 1.021) | 0.996 (0.995 to 0.996) | 1.022 (1.012 to 1.032) |
| 8 | 3 | 1.019 (1.015 to 1.022) | 1.012 (1.005 to 1.022) | 0.995 (0.993 to 0.997) | 1.026 (1.016 to 1.039) |
| 10 | 3 | 1.030 (1.024 to 1.033) | 0.998 (0.992 to 1.002) | 1.002 (1.000 to 1.005) | 1.030 (1.025 to 1.036) |

A/A, same main binary twice (job array 45874969): median ratio 1.000 over the 42 problems (0.984 to 1.035); on the 27
problems of 4 ms or more 0.994 to 1.008 (geometric mean 1.001).

### graph and geometric (shared builder, 6b490e9 against main), job arrays 45872531 and 45876225

### base / c4
| problem | regions | tasks | base ms | c4 ms | ratio | min | max | nodes |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| masses2_N8.geometric | 2837 | 4 | 325.637 | 323.350 | 1.007 | 0.996 | 1.014 | a002 |
| masses2_N8.graph | 2837 | 4 | 732.585 | 730.859 | 0.997 | 0.992 | 1.013 | a002 |
| masses3_N3.geometric | 955 | 4 | 1505.091 | 848.832 | 1.777 | 1.764 | 1.781 | a043,a048,a055 |
| masses3_N3.graph | 955 | 4 | 5082.512 | 165.318 | 30.743 | 30.539 | 30.966 | a043 |
| pop_017.geometric | 4139 | 4 | 22991.991 | 5292.857 | 4.349 | 4.326 | 4.365 | a045 |
| pop_017.graph | 4143 | 4 | 14171.953 | 439.121 | 32.255 | 31.345 | 32.354 | a033,a059 |
| rand_t6_x5_m20_s0.geometric | 1774 | 4 | 854.938 | 649.238 | 1.321 | 1.307 | 1.325 | a043 |
| rand_t6_x5_m20_s0.graph | 1774 | 4 | 361.538 | 148.909 | 2.423 | 2.403 | 2.465 | a002,a017,a043 |
| rand_t6_x5_m20_s1.geometric | 5063 | 4 | 3491.142 | 1597.934 | 2.202 | 2.163 | 2.231 | a052 |
| rand_t6_x5_m20_s1.graph | 5063 | 4 | 2497.160 | 518.928 | 4.815 | 4.622 | 5.003 | a052 |
| rand_t6_x6_m24_s0.geometric | 17131 | 4 | 33594.663 | 30617.586 | 1.099 | 1.072 | 1.114 | a033 |
| rand_t6_x6_m24_s0.graph | 17134 | 4 | 4984.573 | 2742.495 | 1.820 | 1.789 | 1.847 | a033,a052 |
| rand_t8_x5_m20_s0.geometric | 4111 | 4 | 2784.421 | 2859.573 | 0.979 | 0.974 | 1.000 | a045 |
| rand_t8_x5_m20_s0.graph | 4111 | 4 | 560.234 | 552.773 | 1.011 | 0.996 | 1.017 | a045 |
all problems: n=14 median 1.799 min 0.979 max 32.255 geomean 2.559
problems of 4 ms or more: n=14 median 1.799 min 0.979 max 32.255 geomean 2.559
### base / base2
| problem | regions | tasks | base ms | base2 ms | ratio | min | max | nodes |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| masses2_N8.geometric | 2837 | 4 | 325.637 | 325.111 | 1.003 | 0.995 | 1.023 | a002 |
| masses2_N8.graph | 2837 | 4 | 732.585 | 734.704 | 1.000 | 0.979 | 1.003 | a002 |
| masses3_N3.geometric | 955 | 4 | 1505.091 | 1503.144 | 1.003 | 0.999 | 1.005 | a043,a048,a055 |
| masses3_N3.graph | 955 | 4 | 5082.512 | 5082.146 | 1.000 | 0.997 | 1.003 | a043 |
| pop_017.geometric | 4139 | 4 | 22937.281 | 22979.266 | 0.997 | 0.994 | 1.005 | a045 |
| pop_017.graph | 4143 | 4 | 14186.921 | 14184.087 | 0.999 | 0.999 | 1.001 | a045,a059 |
| rand_t6_x5_m20_s0.geometric | 1774 | 4 | 854.938 | 858.320 | 0.995 | 0.992 | 1.006 | a043 |
| rand_t6_x5_m20_s0.graph | 1774 | 4 | 361.538 | 363.882 | 0.996 | 0.972 | 1.004 | a002,a017,a043 |
| rand_t6_x5_m20_s1.geometric | 5063 | 4 | 3491.142 | 3600.979 | 0.969 | 0.964 | 1.030 | a052 |
| rand_t6_x5_m20_s1.graph | 5063 | 4 | 2497.160 | 2524.462 | 0.986 | 0.984 | 0.994 | a052 |
| rand_t6_x6_m24_s0.geometric | 17131 | 4 | 33049.094 | 31955.815 | 1.032 | 0.928 | 1.068 | a033 |
| rand_t6_x6_m24_s0.graph | 17134 | 4 | 4984.573 | 4934.256 | 1.004 | 1.000 | 1.030 | a033,a052 |
| rand_t8_x5_m20_s0.geometric | 4111 | 4 | 2784.421 | 2802.324 | 0.997 | 0.985 | 1.016 | a045 |
| rand_t8_x5_m20_s0.graph | 4111 | 4 | 560.234 | 561.224 | 0.998 | 0.967 | 1.001 | a045 |
all problems: n=14 median 0.999 min 0.969 max 1.032 geomean 0.998
problems of 4 ms or more: n=14 median 0.999 min 0.969 max 1.032 geomean 0.998

## Rejected or not done

| idea | result |
|---|---|
| Rays from the Chebyshev centre instead of the 2n box LPs (certain keeps by first hit, LP for the rest), job 45874072/45874073 (r3, 12 problems at 6 to 10 parameters, 4 tasks each, against the cap-5 build) | Rays then the box only for undecided rows: 0.92x (8% slower, the m^2 n ray cost exceeds the LPs saved), rows identical. Rays and no box at all: median 1.006 (0.92x pop_005 to 1.19x rand_t10_x5_m15_s0), rows differ on 4 of 11 problems (masses3_N3 15,444 to 15,442, masses4_N3 73,042 to 72,998, pop_005 163,599 to 163,589, pop_017 58,974 to 58,973; all rows within 6e-10 of touching a sliver region). Not shipped. Patch kept in this work log only. |
| Lower vertex guard at 6 parameters (40, 80, 150, 300) | Slower than the LP path for every problem and rows differ more; subsumed by the cap |
| Linear list instead of `FxHashMap` in `clip_n` | no change (clip 4.82 to 4.93 ms per 2,000 regions on a laptop), reverted |
| Margin accept (skip the Chebyshev LP when the pushed hint has margin above 2e-8), counter only (job 45874968) | share of hints that would be accepted: 71% at 3 parameters, 57% at 4, 27% to 34% at 5, 19% to 31% at 6, 23% to 39% at 7 to 10 (7% on masses4_N3). The accepted centre would not be the Chebyshev centre (clip order and hint chain change), so rows could flip at tolerance edges; not implemented. Chebyshev LP is 21 to 27% of the build at 3 to 4 parameters and 5 to 8% at 7 to 10 |
| Flag transfer across toggles | waits for the neighbour index of `mplcp-case2-fix`; not built |
| Region rows from the tableau (task 39) | not built: the Chebyshev and clip stages need the same rows anyway and the KKT rows are 9% of the 3-parameter build; no counter justified it |
| Weak-row counter (rows kept with fewer than nt tight vertices) | fewer than 2,000 occurrences per run (the counter prints every 2,000), so rare; not measured further |
| Lifting the clip cap above 6 | already rejected before this work |

### Timing of the rejected ray variants (r3, ratios c3 / variant, above 1 means the variant is faster)

### c3 / x@LPTAU=1000
| problem | regions | tasks | c3 ms | x@LPTAU=1000 ms | ratio | min | max | nodes |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| masses3_N3 | 955 | 4 | 453.269 | 460.341 | 0.984 | 0.981 | 0.992 | a045 |
| masses4_N3 | 2892 | 4 | 4958.358 | 4887.939 | 1.014 | 1.011 | 1.019 | a047 |
| masses5_N2 | 370 | 4 | 577.176 | 546.639 | 1.059 | 1.051 | 1.083 | a043 |
| pop_005 | 10298 | 4 | 8209.262 | 8924.296 | 0.918 | 0.916 | 0.930 | a047 |
| pop_017 | 4142 | 4 | 804.909 | 807.202 | 0.997 | 0.990 | 1.007 | a045 |
| pop_021 | 679 | 4 | 196.560 | 202.301 | 0.977 | 0.966 | 0.980 | a047 |
| pop_027 | 1472 | 4 | 559.585 | 586.249 | 0.955 | 0.947 | 0.956 | a047 |
| rand_t10_x5_m15_s0 | 1859 | 4 | 629.045 | 529.647 | 1.189 | 1.185 | 1.277 | a002,a043 |
| rand_t10_x5_m20_s0 | 6124 | 4 | 2807.161 | 2539.223 | 1.105 | 1.094 | 1.121 | a009,a017 |
| rand_t6_x5_m20_s0 | 1774 | 4 | 261.866 | 250.856 | 1.046 | 1.041 | 1.048 | a045 |
| rand_t6_x6_m24_s0 | 17134 | 4 | 3241.968 | 3323.260 | 0.977 | 0.972 | 0.980 | a014,a045 |
| rand_t8_x5_m20_s0 | 4111 | 4 | 1048.402 | 1015.420 | 1.032 | 1.018 | 1.036 | a043,a053,a055 |
all problems: n=12 median 1.006 min 0.918 max 1.189 geomean 1.019
problems of 4 ms or more: n=12 median 1.006 min 0.918 max 1.189 geomean 1.019
### c3 / x@LPTAU=0
all problems: n=12 median 0.919 min 0.880 max 0.969 geomean 0.917
### c3 / x@LPTAU=1
all problems: n=12 median 0.920 min 0.880 max 0.968 geomean 0.919
### c3 / x@NORAYS=1
all problems: n=12 median 1.001 min 0.990 max 1.076 geomean 1.005

A control build of the same tree with the rays disabled (`x@NORAYS=1`) matched c3 within 0.99x to 1.01x on 11 of 12
problems and 1.076x on rand_t6_x5_m20_s0 (code layout noise of a rebuilt binary).

## Gates

* `cargo test --profile gate`, merged tree 590243a (this branch after merging main 1ee2e20), one task per test
  (array 45872996, merged test binary `all` plus unit tests and `mplcp_rows`, `mplcp_tableau_law`): 137 passed,
  0 failed, 3 ignored (140 tests listed). Same totals as main.
* Both builds compile: default and `--features lp_census` (`cargo check` on the final tree).
* Row identity: see each item. mplcp 40 of 42 identical for the final tree (the 2 that differ are in item 3).
* Serial/parallel order gate (`mplcp_bench 2,8`, the shared-builder tree 6b490e9, 31 problems from the bench set to
  rand_t6_x5_m20_s0; array 45876933): `order=true` on all 31 (parallel region order equals serial at 2 and 8 threads);
  `same=true` (signature equal to geometric) on 30. rand_t5_x6_m24_s0 prints `same=false` on main f017013 and on every
  tree of this branch alike (jobs 45876968), so it predates this work.

