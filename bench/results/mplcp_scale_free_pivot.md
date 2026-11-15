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
0.624 (21.5 s to 13.4 s). One regression in this first version (51d9c62), fixed below by 04d4ca9: **masses8_N2 is 1.48x** (1.65 s to 2.44 s, 3 of 3 alternations, A/A 0.998); the
neighbours masses7_N2 (0.84) and masses6_N2 (0.96) gain. Where the 0.8 s went on masses8_N2, and the fix, are in the section below.

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

## masses8_N2 and the follow-up fix (04d4ca9)

The first version (51d9c62) made masses8_N2 1.48x slower than main (1.65 s to 2.44 s). The census run of main for this comparison was
first built wrongly (the "main" binary carried the candidate's code), so the first reading of "identical counts" was wrong. With the
correct builds, on masses8_N2 main against 51d9c62: Case-2 calls 6337 against 8354, adjacent calls per region 61.4 against 81.6, LPs
23654 against 27049, regions 395 in both, QPs 1 in both. In a release build the 338 relative-pivot calls per solve are all taken
(none blocked), so the fast path takes facets that main sent to the rank-1 enumeration, and the pivoted child tableau behind a
sub-1e-10 pivot makes the later neighbour search more expensive. Variants (alg_time, ms): main 1596, 51d9c62 2343, fast-path rule
off 1948, rank-1 test old 2658, both off 1804.

Fix (04d4ca9): a toggle taken through the relative test returns `Nbr::Z` (the basis only), so the child is factored from its active
set and no pivoted tableau or entry hyperplane is kept. Single-solve check, ms: masses8_N2 2350 to 1371 (main 1593), masses7_N2 945 to 910
(main 1134), masses5_N3 13231 to 12549 (main 30756). Folded into the same commit: the Lean finding 16 gap. With a single
vanishing slack row the measure now also demotes a fast-path call with `tve > PTOL` and measure at most 1e-9 to rank-1
(counter `FastDemoted`). `FastDemoted` is 0 on masses8_N2, masses4_N3, masses5_N3 and masses4_N4, so the demotion does not
fire there (the census also gives `FastRel` 270, 2835, 8744 and 19979, and `PivotGap` 0, 3, 4 and 687 for the same four).

### Gate of 04d4ca9 (head of the branch before the merges of main a8d003f: 04d4ca9)

Same method as above, binaries built once (bincache key `src-6c9f69dd2fe4-lock-049f3bfd-ex-1a391d25ae8a`), jobs 45899228 (build),
hash and coverage on all 111 problems, paired timing on the masses family plus the 18 problems that changed in 51d9c62 (111 tasks)
and on 13 control problems of 2 to 16 parameters (55 tasks). In a task the fix, 51d9c62, main and a copy of main (A/A) run once each
in shuffled order.

| group | problems | fix / main | previous (51d9c62) / main | best | worst | A/A median (range) |
|---|---:|---:|---:|---|---|---|
| masses, all | 21 | 0.772 | 0.833 | masses5_N3 0.378 | masses2_N10 1.043 | 1.000 (0.992 to 1.012) |
| masses 2-5 params | 7 | 1.033 | 1.015 | masses2_N4 1.019 | masses2_N10 1.043 | 1.001 (0.996 to 1.012) |
| masses 6+ params | 14 | 0.667 | 0.755 | masses5_N3 0.378 | masses3_N2 0.925 | 1.000 (0.992 to 1.006) |
| changed non-masses problems (hash differs in 51d9c62) | 6 | 1.002 | 1.004 | pop_007 0.988 | rand_t6_x6_m24_s0 1.010 | 1.000 (0.997 to 1.011) |
| 13 control problems, 2-16 params | 13 | 1.010 | 1.003 | pop_043 0.999 | di_N10 1.034 | 1.001 (0.997 to 1.006) |

All timed problems:

| params | problem | regions (main / fix) | main ms | 51d9c62 ms | fix ms | fix / main | 51d9c62 / main | A/A | alternations |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|
| 2 | di_N10 | 11 / 11 | 0.15 | 0.15 | 0.15 | 1.034 | 1.027 | 1.000 | 7 |
| 3 | sys3x2_N5 | 973 / 973 | 22.89 | 22.89 | 23.57 | 1.027 | 0.997 | 1.001 | 7 |
| 4 | masses2_N10 | 6381 / 6381 | 407.94 | 413.25 | 427.47 | 1.043 | 1.013 | 1.001 | 5 |
| 4 | masses2_N12 | 12779 / 12779 | 967.51 | 977.20 | 998.91 | 1.036 | 1.010 | 0.996 | 3 |
| 4 | masses2_N4 | 307 / 307 | 13.16 | 13.35 | 13.42 | 1.019 | 1.012 | 1.000 | 7 |
| 4 | masses2_N5 | 493 / 493 | 22.84 | 23.05 | 23.56 | 1.028 | 1.018 | 1.001 | 7 |
| 4 | masses2_N6 | 901 / 901 | 44.09 | 44.64 | 45.50 | 1.033 | 1.013 | 1.002 | 7 |
| 4 | masses2_N7 | 1641 / 1641 | 85.32 | 86.30 | 88.40 | 1.043 | 1.029 | 1.012 | 5 |
| 4 | masses2_N8 | 2837 / 2837 | 162.26 | 163.15 | 166.70 | 1.032 | 1.012 | 1.003 | 5 |
| 4 | pop_019 | 6857 / 6857 | 2506.15 | 2494.48 | 2523.18 | 1.007 | 0.995 | 1.002 | 3 |
| 4 | pop_043 | 899 / 899 | 38.98 | 39.04 | 39.01 | 0.999 | 0.995 | 1.000 | 7 |
| 5 | pop_001 | 3529 / 3529 | 18789.30 | 18671.43 | 18758.16 | 1.003 | 0.999 | 0.999 | 3 |
| 5 | pop_088 | 62318 / 62318 | 5364.04 | 5381.04 | 5406.38 | 1.008 | 1.003 | 0.997 | 3 |
| 6 | masses3_N2 | 169 / 169 | 26.80 | 24.59 | 24.67 | 0.925 | 0.921 | 1.000 | 7 |
| 6 | masses3_N3 | 955 / 955 | 305.82 | 230.19 | 232.23 | 0.761 | 0.754 | 1.000 | 5 |
| 6 | masses3_N4 | 3643 / 3643 | 2390.20 | 1640.95 | 1667.54 | 0.698 | 0.685 | 1.003 | 3 |
| 6 | masses3_N5 | 10437 / 10436 | 21527.48 | 13422.98 | 14046.39 | 0.652 | 0.624 | 1.000 | 3 |
| 6 | masses3_N6 | 22145 / 22143 | 58639.13 | 43850.26 | 46597.03 | 0.795 | 0.748 | 1.005 | 3 |
| 6 | masses3_N7 | 38330 / 38327 | 169813.21 | 123279.83 | 129185.85 | 0.761 | 0.729 | 0.995 | 3 |
| 6 | pop_017 | 4142 / 4142 | 526.88 | 528.75 | 529.49 | 1.005 | 1.002 | 1.001 | 5 |
| 6 | pop_049 | 9341 / 9341 | 1343.38 | 1352.45 | 1357.91 | 1.011 | 1.007 | 1.003 | 3 |
| 6 | rand_t6_x6_m24_s0 | 17134 / 17134 | 2386.44 | 2410.23 | 2416.94 | 1.010 | 1.010 | 1.011 | 3 |
| 7 | pop_005 | 10289 / 10289 | 4517.70 | 4530.21 | 4579.37 | 1.013 | 1.002 | 0.997 | 3 |
| 7 | pop_037 | 18615 / 18615 | 3133.30 | 3123.55 | 3145.44 | 1.004 | 0.997 | 0.999 | 3 |
| 8 | masses4_N2 | 325 / 325 | 147.91 | 128.00 | 118.67 | 0.805 | 0.867 | 1.000 | 5 |
| 8 | masses4_N3 | 2891 / 2891 | 3875.08 | 2401.67 | 1989.42 | 0.517 | 0.621 | 1.002 | 3 |
| 8 | masses4_N4 | 16050 / 16054 | 69887.69 | 39205.97 | 38216.32 | 0.548 | 0.562 | 1.000 | 3 |
| 8 | pop_007 | 12235 / 12235 | 10718.86 | 10856.78 | 10678.72 | 0.988 | 1.020 | 1.003 | 3 |
| 8 | pop_028 | 82116 / 82116 | 28008.29 | 28331.07 | 28310.81 | 1.007 | 1.006 | 0.999 | 3 |
| 8 | pop_047 | 48144 / 48144 | 13673.94 | 13529.98 | 13653.28 | 0.998 | 0.986 | 1.000 | 3 |
| 8 | pop_076 | 15006 / 15006 | 4266.17 | 4253.24 | 4304.40 | 1.007 | 0.995 | 1.006 | 3 |
| 8 | rand_t8_x5_m20_s0 | 4111 / 4111 | 659.07 | 662.49 | 664.22 | 1.007 | 1.008 | 1.004 | 5 |
| 10 | masses5_N2 | 370 / 369 | 391.46 | 301.11 | 274.01 | 0.703 | 0.771 | 0.996 | 5 |
| 10 | masses5_N3 | 5060 / 5067 | 30896.04 | 13602.30 | 11618.18 | 0.378 | 0.438 | 0.998 | 3 |
| 10 | rand_t10_x5_m20_s0 | 6124 / 6124 | 1673.39 | 1683.82 | 1702.23 | 1.009 | 1.006 | 1.003 | 3 |
| 12 | masses6_N2 | 380 / 379 | 736.15 | 704.84 | 414.75 | 0.568 | 0.955 | 0.992 | 5 |
| 14 | masses7_N2 | 390 / 389 | 1128.95 | 949.90 | 787.31 | 0.693 | 0.841 | 1.001 | 3 |
| 14 | rand_t14_x5_m15_s0 | 2470 / 2470 | 1325.64 | 1325.69 | 1329.02 | 1.003 | 0.999 | 0.999 | 3 |
| 16 | masses8_N2 | 395 / 395 | 1597.55 | 2334.51 | 1185.62 | 0.743 | 1.466 | 1.006 | 3 |
| 16 | rand_t16_x5_m15_s0 | 1876 / 1876 | 1396.33 | 1403.84 | 1403.67 | 1.005 | 1.002 | 1.004 | 3 |

Regions of the fix against main and against 51d9c62 (only problems whose hash differs from either):

| problem | params | main regions | 51d9c62 regions | fix regions | hash vs main | hash vs 51d9c62 | fix gaps | overlaps | lex-infeasible |
|---|---:|---:|---:|---:|---|---|---:|---:|---:|
| pop_001 | 5 | 3529 | 3529 | 3529 | differs | differs | 0 | 0 | 0 |
| pop_088 | 5 | 62318 | 62318 | 62318 | same | differs | 0 | 0 | 0 |
| masses3_N3 | 6 | 955 | 955 | 955 | differs | differs | 0 | 0 | 0 |
| masses3_N4 | 6 | 3643 | 3643 | 3643 | differs | differs | 0 | 0 | 0 |
| masses3_N5 | 6 | 10437 | 10436 | 10436 | differs | differs | 0 | 0 | 0 |
| masses3_N6 | 6 | 22145 | 22142 | 22143 | differs | differs | 0 | 0 | 0 |
| masses3_N7 | 6 | 38330 | 38329 | 38327 | differs | differs | 0 | 0 | 0 |
| rand_t6_x6_m24_s0 | 6 | 17134 | 17134 | 17134 | differs | differs | 0 | 0 | 0 |
| masses4_N3 | 8 | 2891 | 2896 | 2891 | differs | differs | 0 | 0 | 0 |
| masses4_N4 | 8 | 16050 | 16068 | 16054 | differs | differs | 0 | 0 | 0 |
| pop_007 | 8 | 12235 | 12235 | 12235 | differs | same | 0 | 0 | 0 |
| pop_028 | 8 | 82116 | 82116 | 82116 | differs | same | 0 | 0 | 0 |
| pop_047 | 8 | 48144 | 48143 | 48144 | differs | differs | 0 | 0 | 0 |
| masses5_N2 | 10 | 370 | 369 | 369 | differs | same | 0 | 0 | 0 |
| masses5_N3 | 10 | 5060 | 5084 | 5067 | differs | differs | 0 | 0 | 0 |
| masses6_N2 | 12 | 380 | 379 | 379 | differs | same | 0 | 0 | 0 |
| masses7_N2 | 14 | 390 | 389 | 389 | differs | same | 0 | 0 | 0 |
| masses8_N2 | 16 | 395 | 395 | 395 | differs | same | 0 | 0 | 0 |
fix identical to main on 94 of 111, identical to 51d9c62 on 99 of 111, problems with a gap, overlap or lex-infeasible region: 0


The fix has 0 gaps, 0 overlaps and 0 lexico-infeasible regions on all 111 problems. It is hash-identical to main on 94 problems and
to 51d9c62 on 99. The masses2 to masses5 problems of 2 to 5 parameters cost 1.9 to 4.3 percent more than main (geometric mean 1.033,
51d9c62 was 1.015): the extra measure evaluation on every single-row fast-path call, which the demotion check needs, is O(n) and
measurable at these sizes. The 13 control problems are unchanged within the A/A range (geometric mean 1.010, di_N10 at 0.15 ms is the
largest, 1.034).

### Short-circuit of the measure (78a0f07)

The O(n) column loop of `pivot_measure` is skipped when the O(1) pivot part alone fixes the decision (pivot part above 1e-9 for a
call with `tve > PTOL`, above 1e-6 for `tve <= PTOL`; `rank1_case2` tests the pivot part first; the census build always computes
the full measure so `PivotGap` stays exact). Paired against the unchanged fix (job 45900820, short-circuit, fix and a copy of the fix
per task, 3 to 7 alternations): the 7 masses problems of 2 to 5 parameters 0.996 (A/A 1.005), all 11 problems 0.995
(masses8_N2 1.004, di_N10 0.987, pop_043 0.992, rand_t8_x5_m20_s0 0.994; the A/A of single problems ranges from 0.970 to 1.060,
the largest on masses2_N8 and masses2_N12). The hash is identical on all 11 and the coverage is 0 gaps, 0 overlaps, 0 lexico-infeasible
regions. So the loop accounts for at most about 0.5 percent of the 1.9 to 4.3 percent that the 2 to 5 parameter masses problems
cost against main; the rest is not explained by it (code layout of `adjacent` is the likely remainder, not measured).

## masses5_N4 and masses4_N5 against graph

Candidate `mplcp` and `graph`, one solve each, then 2000 coverage samples (`lexcheck`):

| problem | solver | regions | solve s | gaps | overlaps | lexico-infeasible | MaxRSS |
|---|---|---:|---:|---:|---:|---:|---:|
| masses5_N4 | mplcp (candidate) | 42554 | 491.6 | 0 | 0 | 0 | 1.12 GB |
| masses5_N4 | graph | 42696 | 152.9 | 0 | 1 | 0 | 0.43 GB |
| masses4_N5 | mplcp (candidate) | 66986 | 797.1 | 0 | 0 | 0 | 1.52 GB |
| masses4_N5 | graph | 67330 | 140.0 | 0 | 0 | 0 | 0.43 GB |

`mplcp` is 3.2x (masses5_N4) and 5.7x (masses4_N5) slower than `graph` on these two. The candidate cut masses5_N4 from about 1800 s
on f234e64 to 492 s. `graph` returns 142 and 344 more regions than `mplcp` (the same kind of sliver difference). **Finding about `graph`:** on
masses5_N4 it has one sampled overlap (two of its regions contain the same feasible sample); not ours to fix.

## Reproduction

`bench/results/` has no per-task logs; the arrays were Negishi jobs 45895899 (hash and coverage), 45895900 (paired timing),
45895901 (masses), 45897253 and 45897343 (census of the candidate and of main) and 45897389 (variants). Run
`examples/lexcheck` (`LEX_ALG=graph` for the other solver), `examples/alg_time`, `examples/lp_census --features lp_census`.
