# mplcp: scale-free "nonzero pivot" test in the Case-1 fast path

Candidate: branch `worktree-agent-a2cd0480ac0fec818` (solver change 17f22f4, tree 196e78f for the measured binaries) against main
c345cdc. Release profile, Negishi (AMD EPYC 7763), serial, 1 CPU per task, 111 problems of 2 to 16 parameters (the set of
`mplcp_goal_status.md`), minus `masses5_N4`, `masses4_N5` and six large POP problems, which are reported apart.

## The change

`adjacent` decided "the pivot is nonzero" with `tve > PTOL` (1e-10), and `rank1_case2` decided "the pivot is zero" with an
absolute pivot and an absolute 1e-8 column test. On badly scaled problems (the diagonal of M = A Q^-1 A' runs from 5e-13 to 0.55
on the masses family) a small but genuinely nonzero pivot was called zero, so a Case-1 facet fell to the Case-2 walk. Both tests now use one
scale-free measure, `pivot_measure` = the larger of |T[c,kk]| / M_cc and |T[i,kk]| / sqrt(M_ii M_cc) over the other slack rows.
Exact rank-1 facets have measure up to 4e-10 and every other pivot has at least 1.7e-5 on the problems measured, so the
thresholds are 1e-9 (zero, rank-1) and 1e-6 (nonzero, fast path). Calls between them go to the walk and are counted by
`PivotGap`; calls taken by the new rule are counted by `FastRel`. The Toggle's sign tests are unchanged.

## Tests

`cargo test --profile gate` on the pre-merge tree (job 45895563, one 1-CPU task per test): 144 passed, 0 failed, 3 ignored
(147 tests), the same as main. The merge of main 76c5070 changed only `mplp_lex.rs` and `mpqp_utils.rs`.

## Regions (hash and coverage from one solve)

`lexcheck`: one solve of each version per problem, then 3000 uniform samples of the parameter box (a feasible sample in no
region is a gap, in two regions an overlap), and the count of lexico-infeasible regions. The candidate has **0 gaps, 0 overlaps
and 0 lexico-infeasible regions on all 111 problems**; base has none either. 93 problems have the same region hash as main, 18
differ:

| problem | params | base regions | cand regions | same hash | cand gaps | overlaps | lex-infeasible |
|---|---:|---:|---:|---|---:|---:|---:|
| pop_001 | 5 | 3529 | 3529 (+0) | no | 0 | 0 | 0 |
| pop_088 | 5 | 62318 | 62318 (+0) | no | 0 | 0 | 0 |
| masses3_N3 | 6 | 955 | 955 (+0) | no | 0 | 0 | 0 |
| masses3_N4 | 6 | 3643 | 3643 (+0) | no | 0 | 0 | 0 |
| masses3_N5 | 6 | 10437 | 10436 (-1) | no | 0 | 0 | 0 |
| masses3_N6 | 6 | 22145 | 22142 (-3) | no | 0 | 0 | 0 |
| masses3_N7 | 6 | 38330 | 38329 (-1) | no | 0 | 0 | 0 |
| rand_t6_x6_m24_s0 | 6 | 17134 | 17134 (+0) | no | 0 | 0 | 0 |
| masses4_N3 | 8 | 2891 | 2896 (+5) | no | 0 | 0 | 0 |
| masses4_N4 | 8 | 16050 | 16068 (+18) | no | 0 | 0 | 0 |
| pop_007 | 8 | 12235 | 12235 (+0) | no | 0 | 0 | 0 |
| pop_028 | 8 | 82116 | 82116 (+0) | no | 0 | 0 | 0 |
| pop_047 | 8 | 48144 | 48143 (-1) | no | 0 | 0 | 0 |
| masses5_N2 | 10 | 370 | 369 (-1) | no | 0 | 0 | 0 |
| masses5_N3 | 10 | 5060 | 5084 (+24) | no | 0 | 0 | 0 |
| masses6_N2 | 12 | 380 | 379 (-1) | no | 0 | 0 | 0 |
| masses7_N2 | 14 | 390 | 389 (-1) | no | 0 | 0 | 0 |
| masses8_N2 | 16 | 395 | 395 (+0) | no | 0 | 0 | 0 |
identical 93 differ 18


Direction of the change. An increase of the region count (masses4_N3 +5, masses4_N4 +18, masses5_N3 +24) is a finer
lexicographic split: a sliver that main merged into a neighbour is a region of its own. A decrease (masses3_N5 -1, masses3_N6 -3,
masses3_N7 -1, pop_047 -1, masses5_N2 -1, masses6_N2 -1, masses7_N2 -1) is a merge of such a sliver. A change of the hash at an
equal count (pop_001, pop_088, masses3_N3, masses3_N4, rand_t6_x6_m24_s0, pop_007, pop_028, masses8_N2) is a different sliver
boundary at the 1e-9 rounding of the hash. In every case the evidence is the coverage line above: the regions of the candidate
cover the sampled feasible set and no sampled point is inside two of them. The sliver-level differences are the same kind as
the earlier sliver differences between tolerances (`mplcp_stray_regions.md`); the lexicographic disjointness argument does
not decide which of the two partitions of a sliver is the better one, only that both are valid.

## Serial time, paired against main

One task per (problem, alternation); in a task the candidate, main, and a second copy of main (A/A) each run once in shuffled order
(`alg_time`: one warm-up solve, then 1 timed solve above 5 s, 3 from 1 to 5 s, 7 from 4 ms to 1 s, 51 from 1 to 4 ms, 301
below). Alternations: 3 from 1 s, 5 from 50 ms, 7 below. Ratio = candidate / main, median over the alternations of a problem;
the groups show the geometric mean of the problems' ratios. A/A median 1.000, range 0.982 to 1.016 over the 111 problems.

| group | problems | geomean cand/base | median | best | worst | above 1.05 | A/A median (range) |
|---|---:|---:|---:|---|---|---:|---|
| all | 111 | 0.969 | 1.002 | masses5_N3 0.433 | masses8_N2 1.480 | 1 | 1.000 (0.982 to 1.016) |
| masses, all | 21 | 0.832 | 0.860 | masses5_N3 0.433 | masses8_N2 1.480 | 1 | 1.000 (0.982 to 1.006) |
| masses 2-5 params | 7 | 1.012 | 1.012 | masses2_N12 1.008 | masses2_N6 1.022 | 0 | 1.000 (0.993 to 1.005) |
| masses 6+ params | 14 | 0.754 | 0.751 | masses5_N3 0.433 | masses8_N2 1.480 | 1 | 1.000 (0.982 to 1.006) |
| not masses, all | 90 | 1.005 | 1.002 | pop_007 0.986 | sys3x2_N7 1.033 | 0 | 1.000 (0.984 to 1.016) |
| not masses, 2-3 params | 18 | 1.013 | 1.013 | sys3x2_N6 0.995 | sys3x2_N7 1.033 | 0 | 1.000 (0.989 to 1.016) |
| not masses, 4-5 params | 19 | 1.001 | 0.999 | pop_043 0.991 | pop_062 1.013 | 0 | 1.000 (0.992 to 1.009) |
| not masses, 6-8 params | 44 | 1.004 | 1.002 | pop_007 0.986 | rand_t6_x5_m20_s1 1.030 | 0 | 0.999 (0.984 to 1.013) |
| not masses, 10-16 params | 9 | 1.000 | 1.001 | rand_t12_x5_m20_s0 0.990 | rand_t16_x5_m15_s0 1.005 | 0 | 0.999 (0.995 to 1.006) |

The masses family is reported apart from the other problems. At 2 to 5 parameters masses is unchanged (ratio 1.012) and so are the
POP, random, di, dinv and sys3x2 problems (geometric mean 1.005, no problem above 1.05 among them). The gain is on masses from 6
parameters: geometric mean 0.754, with masses5_N3 at 0.433 (30.6 s to 13.3 s), masses4_N4 0.559 (69.8 s to 39.1 s) and masses3_N5
0.624 (21.5 s to 13.4 s). One regression: **masses8_N2 is 1.48x** (1.65 s to 2.44 s, 3 of 3 alternations, A/A 0.998); the
neighbours masses7_N2 (0.84) and masses6_N2 (0.96) gain. Where the 0.8 s go on masses8_N2 is in the section below.

All problems:

| params | problem | regions | base ms | cand ms | cand/base | A/A | alternations |
|---:|---|---:|---:|---:|---:|---:|---:|
| 2 | di_N10 | 11 | 0.15 | 0.15 | 1.021 | 1.000 | 7 |
| 2 | di_N12 | 11 | 0.17 | 0.18 | 1.011 | 0.989 | 7 |
| 2 | di_N15 | 11 | 0.22 | 0.23 | 1.018 | 1.005 | 7 |
| 2 | di_N20 | 11 | 0.34 | 0.34 | 1.000 | 0.997 | 7 |
| 2 | di_N25 | 11 | 0.45 | 0.45 | 1.009 | 1.007 | 7 |
| 2 | di_N30 | 11 | 0.65 | 0.66 | 1.025 | 1.008 | 7 |
| 2 | di_N4 | 9 | 0.07 | 0.07 | 1.027 | 1.000 | 7 |
| 2 | di_N6 | 11 | 0.11 | 0.11 | 1.009 | 1.000 | 7 |
| 2 | di_N8 | 11 | 0.12 | 0.13 | 1.024 | 1.000 | 7 |
| 2 | di_doc_N3 | 7 | 0.06 | 0.06 | 1.016 | 1.016 | 7 |
| 2 | dinv_N4 | 9 | 0.07 | 0.07 | 1.014 | 1.014 | 7 |
| 2 | dinv_N6 | 11 | 0.11 | 0.11 | 1.009 | 1.000 | 7 |
| 2 | dinv_N8 | 11 | 0.12 | 0.13 | 1.024 | 1.008 | 7 |
| 3 | sys3x2_N3 | 211 | 3.96 | 3.97 | 1.000 | 0.998 | 7 |
| 3 | sys3x2_N4 | 503 | 11.46 | 11.08 | 0.998 | 1.000 | 7 |
| 3 | sys3x2_N5 | 973 | 23.09 | 23.20 | 0.999 | 0.997 | 7 |
| 3 | sys3x2_N6 | 1503 | 38.82 | 38.35 | 0.995 | 1.003 | 7 |
| 3 | sys3x2_N7 | 2095 | 58.67 | 60.20 | 1.033 | 1.003 | 5 |
| 4 | masses2_N10 | 6381 | 386.69 | 393.06 | 1.013 | 1.000 | 5 |
| 4 | masses2_N12 | 12779 | 1013.72 | 1024.62 | 1.008 | 0.993 | 3 |
| 4 | masses2_N4 | 307 | 13.25 | 13.31 | 1.011 | 0.997 | 7 |
| 4 | masses2_N5 | 493 | 22.88 | 23.20 | 1.013 | 1.002 | 7 |
| 4 | masses2_N6 | 901 | 47.97 | 49.82 | 1.022 | 1.000 | 7 |
| 4 | masses2_N7 | 1641 | 87.05 | 87.15 | 1.008 | 1.005 | 5 |
| 4 | masses2_N8 | 2837 | 152.70 | 154.63 | 1.012 | 1.003 | 5 |
| 4 | pop_009 | 63 | 2.20 | 2.22 | 1.007 | 1.004 | 7 |
| 4 | pop_014 | 19 | 0.53 | 0.54 | 1.006 | 0.998 | 7 |
| 4 | pop_019 | 6857 | 2650.47 | 2654.95 | 1.001 | 0.997 | 3 |
| 4 | pop_023 | 4729 | 199.54 | 199.97 | 1.002 | 0.996 | 5 |
| 4 | pop_043 | 899 | 38.46 | 38.00 | 0.991 | 0.992 | 7 |
| 4 | pop_048 | 46 | 28.82 | 28.66 | 0.996 | 1.000 | 7 |
| 4 | pop_059 | 53 | 1.85 | 1.85 | 0.998 | 0.999 | 7 |
| 4 | pop_077 | 3915 | 135.40 | 136.82 | 1.010 | 1.005 | 5 |
| 5 | pop_001 | 3529 | 18606.78 | 18598.67 | 0.999 | 1.000 | 3 |
| 5 | pop_008 | 21295 | 1671.86 | 1675.22 | 1.007 | 1.007 | 3 |
| 5 | pop_012 | 1047 | 66.20 | 66.62 | 1.002 | 0.998 | 5 |
| 5 | pop_040 | 2007 | 172.83 | 172.41 | 0.995 | 1.009 | 5 |
| 5 | pop_057 | 34 | 2.97 | 2.94 | 0.992 | 1.001 | 7 |
| 5 | pop_062 | 31727 | 3052.91 | 3126.90 | 1.013 | 1.005 | 3 |
| 5 | pop_070 | 703 | 54.82 | 54.89 | 1.003 | 1.003 | 5 |
| 5 | pop_075 | 119 | 4.76 | 4.74 | 0.998 | 0.997 | 7 |
| 5 | pop_083 | 13266 | 1240.93 | 1237.68 | 0.998 | 0.998 | 3 |
| 5 | pop_088 | 62318 | 5384.27 | 5383.61 | 0.999 | 1.006 | 3 |
| 5 | pop_096 | 258 | 38.35 | 38.18 | 0.995 | 0.998 | 7 |
| 6 | masses3_N2 | 169 | 26.75 | 24.66 | 0.922 | 1.001 | 7 |
| 6 | masses3_N3 | 955 | 441.65 | 331.91 | 0.753 | 0.982 | 5 |
| 6 | masses3_N4 | 3643 | 2495.45 | 1699.33 | 0.681 | 0.996 | 3 |
| 6 | masses3_N5 | 10436/10437/10437 | 21499.21 | 13419.00 | 0.624 | 1.002 | 3 |
| 6 | masses3_N6 | 22145/22142 | 58250.30 | 43699.24 | 0.749 | 1.000 | 3 |
| 6 | masses3_N7 | 38329/38330/38330 | 168497.32 | 123088.03 | 0.730 | 1.000 | 3 |
| 6 | pop_011 | 397 | 34.13 | 34.13 | 1.005 | 0.997 | 7 |
| 6 | pop_017 | 4142 | 539.04 | 541.74 | 1.006 | 1.000 | 5 |
| 6 | pop_018 | 20800 | 2797.83 | 2767.44 | 0.997 | 0.984 | 3 |
| 6 | pop_025 | 5441 | 771.86 | 773.19 | 0.998 | 0.991 | 5 |
| 6 | pop_046 | 24 | 2.17 | 2.17 | 1.001 | 0.998 | 7 |
| 6 | pop_049 | 9341 | 1372.18 | 1378.29 | 1.004 | 1.000 | 3 |
| 6 | pop_053 | 2441 | 256.76 | 261.33 | 1.003 | 0.994 | 5 |
| 6 | pop_081 | 4 | 0.16 | 0.16 | 0.987 | 0.994 | 7 |
| 6 | pop_084 | 665 | 47.83 | 48.17 | 1.001 | 1.000 | 7 |
| 6 | rand_t6_x5_m20_s0 | 1774 | 172.09 | 173.72 | 1.007 | 1.000 | 5 |
| 6 | rand_t6_x5_m20_s1 | 5062 | 569.10 | 583.31 | 1.030 | 1.010 | 5 |
| 6 | rand_t6_x6_m24_s0 | 17134 | 2444.64 | 2455.74 | 1.001 | 0.999 | 3 |
| 7 | pop_004 | 247 | 23.00 | 22.98 | 0.998 | 1.001 | 7 |
| 7 | pop_005 | 10289 | 4586.66 | 4607.64 | 1.006 | 1.004 | 3 |
| 7 | pop_021 | 679 | 101.38 | 101.38 | 1.001 | 1.000 | 5 |
| 7 | pop_030 | 286 | 24.18 | 24.32 | 1.002 | 1.001 | 7 |
| 7 | pop_037 | 18615 | 3137.78 | 3132.78 | 0.999 | 1.003 | 3 |
| 7 | pop_054 | 20 | 1.66 | 1.68 | 1.011 | 0.998 | 7 |
| 7 | pop_056 | 107 | 7.82 | 7.82 | 1.002 | 0.999 | 7 |
| 7 | pop_065 | 1089 | 165.29 | 166.19 | 1.011 | 0.999 | 5 |
| 7 | pop_068 | 86 | 7.02 | 7.02 | 0.996 | 0.996 | 7 |
| 7 | pop_073 | 23 | 2.20 | 2.24 | 1.016 | 0.997 | 7 |
| 7 | pop_080 | 273 | 26.57 | 26.65 | 1.000 | 1.006 | 7 |
| 7 | pop_085 | 121 | 9.39 | 9.37 | 0.997 | 0.999 | 7 |
| 7 | pop_086 | 3625 | 799.99 | 824.01 | 1.018 | 1.013 | 5 |
| 7 | pop_091 | 145018 | 35683.88 | 35652.48 | 0.999 | 0.998 | 3 |
| 7 | pop_097 | 20574 | 6361.59 | 6475.49 | 1.018 | 1.009 | 3 |
| 8 | masses4_N2 | 325 | 144.24 | 125.07 | 0.860 | 0.995 | 5 |
| 8 | masses4_N3 | 2891/2896 | 3821.89 | 2362.32 | 0.618 | 1.002 | 3 |
| 8 | masses4_N4 | 16050/16068/16050 | 69833.82 | 39050.46 | 0.559 | 1.002 | 3 |
| 8 | pop_007 | 12235 | 12289.53 | 12193.06 | 0.986 | 0.999 | 3 |
| 8 | pop_010 | 36 | 8.00 | 8.16 | 1.017 | 1.001 | 7 |
| 8 | pop_016 | 384 | 73.64 | 74.39 | 1.012 | 1.004 | 5 |
| 8 | pop_027 | 1472 | 320.70 | 319.89 | 1.006 | 0.995 | 5 |
| 8 | pop_028 | 82116 | 27388.21 | 27543.01 | 1.006 | 1.000 | 3 |
| 8 | pop_032 | 178950 | 64759.95 | 64688.00 | 0.997 | 0.994 | 3 |
| 8 | pop_047 | 48143/48144/48144 | 15319.56 | 14963.01 | 0.995 | 0.996 | 3 |
| 8 | pop_061 | 33587 | 7886.52 | 7915.85 | 1.004 | 1.010 | 3 |
| 8 | pop_069 | 26 | 4.31 | 4.47 | 1.016 | 0.997 | 7 |
| 8 | pop_076 | 15006 | 4261.93 | 4264.85 | 1.002 | 1.002 | 3 |
| 8 | pop_090 | 438 | 90.30 | 91.59 | 1.017 | 1.002 | 5 |
| 8 | pop_098 | 269 | 50.85 | 51.69 | 1.014 | 0.994 | 5 |
| 8 | rand_t8_x5_m15_s0 | 1035 | 129.51 | 129.04 | 0.991 | 0.997 | 5 |
| 8 | rand_t8_x5_m20_s0 | 4111 | 656.63 | 658.43 | 1.003 | 1.002 | 5 |
| 8 | rand_t8_x5_m20_s1 | 8737 | 1868.93 | 1864.73 | 1.001 | 0.994 | 3 |
| 8 | rand_t8_x6_m18_s0 | 9378 | 1671.94 | 1634.12 | 0.989 | 1.003 | 3 |
| 8 | rand_t8_x6_m24_s0 | 32475 | 7554.18 | 7540.06 | 0.998 | 1.000 | 3 |
| 10 | masses5_N2 | 370/369 | 381.04 | 293.25 | 0.770 | 0.998 | 5 |
| 10 | masses5_N3 | 5060/5084 | 30620.30 | 13262.44 | 0.433 | 0.999 | 3 |
| 10 | rand_t10_x5_m15_s0 | 1859 | 422.05 | 421.04 | 0.999 | 0.997 | 5 |
| 10 | rand_t10_x5_m20_s0 | 6124 | 1704.70 | 1708.86 | 1.003 | 1.001 | 3 |
| 10 | rand_t10_x5_m20_s1 | 14848 | 4281.87 | 4259.56 | 0.997 | 0.998 | 3 |
| 10 | rand_t10_x6_m18_s0 | 15312 | 3611.61 | 3620.89 | 1.004 | 1.001 | 3 |
| 10 | rand_t10_x6_m24_s0 | 71219 | 23692.49 | 23806.08 | 1.002 | 0.999 | 3 |
| 12 | masses6_N2 | 380/379/380 | 728.76 | 700.07 | 0.961 | 1.006 | 5 |
| 12 | rand_t12_x5_m15_s0 | 1012 | 323.80 | 324.75 | 1.001 | 0.999 | 5 |
| 12 | rand_t12_x5_m20_s0 | 6143 | 3791.24 | 3758.10 | 0.990 | 1.006 | 3 |
| 14 | masses7_N2 | 390/389 | 1145.33 | 960.09 | 0.838 | 0.997 | 3 |
| 14 | rand_t14_x5_m15_s0 | 2470 | 1344.77 | 1342.47 | 0.998 | 1.001 | 3 |
| 16 | masses8_N2 | 395 | 1649.05 | 2435.37 | 1.480 | 0.998 | 3 |
| 16 | rand_t16_x5_m15_s0 | 1876 | 1428.98 | 1422.96 | 1.005 | 0.995 | 3 |

## masses8_N2

The census (`lp_census`, one solve each) of main and the candidate gives the same LP count (27049), the same Case-2 calls (8354),
the same answers by the rank-1 enumeration (7606) and the same hand-offs to the walk (748). `PivotGap` is 0 (no call has a measure
between 1e-9 and 1e-6, so the gap is clean here too) and `FastRel` is 338 (masses7_N2: 324, also 0 in the gap). So the new rule
takes 338 facets that main took through another route, with unchanged LP and walk counts.
Variants of the candidate on masses8_N2 (alg_time, 3 solves, 3 alternations, ms): main 1596, candidate 2343, new fast path
switched off (rank-1 new) 1948, new rank-1 test switched off (fast path new) 2658, both switched off 1804. The fast path rule
is the larger part (0.4 to 0.85 s); the rank-1 test adds about 0.15 s; and the candidate source with both switched off is
still 13% slower than main, which points at code layout (the function `adjacent` grew) as well. The extra time is not in the
LP or walk counters, and the one-shot census run of the candidate (2381 ms) is not slower than that of main (2541 ms), so
this is not explained and is not claimed to be. The absolute cost is 0.8 s on a 1.6 s problem, 16 parameters, where `mplcp` is
already 2.2x slower than `parametric_daqp`.

## masses5_N4 and masses4_N5 against graph

Candidate `mplcp` and `graph`, one solve each, then 2000 coverage samples (`lexcheck`):

| problem | solver | regions | solve s | gaps | overlaps | lexico-infeasible | MaxRSS |
|---|---|---:|---:|---:|---:|---:|---:|
| masses5_N4 | mplcp (candidate) | 42554 | 491.6 | 0 | 0 | 0 | 1.12 GB |
| masses5_N4 | graph | 42696 | 152.9 | 0 | 1 | 0 | 0.43 GB |
| masses4_N5 | mplcp (candidate) | 66986 | 797.1 | 0 | 0 | 0 | 1.52 GB |
| masses4_N5 | graph | 67330 | 140.0 | 0 | 0 | 0 | 0.43 GB |

`mplcp` is 3.2x (masses5_N4) and 5.7x (masses4_N5) slower than `graph` on these two. The candidate cut masses5_N4 from about 1800 s
on f234e64 to 492 s. `graph` returns 142 and 344 more regions than `mplcp` (the same kind of sliver difference), and on
masses5_N4 `graph` has one sampled overlap.

## Reproduction

`bench/results/` has no per-task logs; the arrays were Negishi jobs 45895899 (hash and coverage), 45895900 (paired timing),
45895901 (masses), 45897253 and 45897343 (census of the candidate and of main) and 45897389 (variants). Run
`examples/lexcheck` (`LEX_ALG=graph` for the other solver), `examples/alg_time`, `examples/lp_census --features lp_census`.
