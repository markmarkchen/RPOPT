# Entry-facet skip in `mplcp`: A/B timing on Negishi

Baseline: main at 5e00b38 (lexico-feasible start, twin filter, tableau law). Candidate: the same plus the entry-facet skip (merge of 478051a). Binary: `mplcp_bench`, release build, `NOGEO=1`.

## Machine and load

| Job | Purpose | Node | CPU | Load average at start / end |
|---|---|---|---|---|
| 45863406 | build and gate | a108 | AMD EPYC 7763, 2 x 64 cores | 48.9 / 50.9 |
| 45863586 | serial A/B | a033 | same | 24.8 / 29.2 |
| 45863588 | 16-thread A/B | a045 | same | 55.8 / 52.3 |
| 45863589 | A/A (serial, then 16 threads) | a092 | same | 70.5 / 70.9 |

Each job asked for 16 CPUs and 31 GB on a shared node (`rustc 1.92.0`), so the nodes carried other users' work at the loads above. The three timing jobs ran concurrently on different nodes. Base and candidate alternate inside each job: 4 alternations for the A/B jobs and for the serial A/A, 3 for the 16-thread A/A, each with 7 repetitions per problem. The per-problem value is the median over the alternations. The A/A comparison times two copies of the baseline binary the same way.

## Gate

All 23 problems print `same=true order=true` for the candidate (geometric signature equal to the geometric solver, parallel region order equal to serial at 2 and 8 threads); job 45863406.

## Per-problem medians (ms) and ratio base/candidate (above 1 means the candidate is faster)

| Problem | Regions | Serial base | Serial cand | Ratio | 16-thread base | 16-thread cand | Ratio |
|---|---:|---:|---:|---:|---:|---:|---:|
| di_doc_N3 | 7 | 0.076 | 0.072 | 1.070 | 0.101 | 0.089 | 1.123 |
| di_N4 | 9 | 0.085 | 0.083 | 1.030 | 0.103 | 0.088 | 1.164 |
| di_N6 | 11 | 0.131 | 0.123 | 1.061 | 0.169 | 0.184 | 0.916 |
| di_N8 | 11 | 0.153 | 0.148 | 1.034 | 0.191 | 0.193 | 0.987 |
| di_N10 | 11 | 0.181 | 0.174 | 1.043 | 0.245 | 0.196 | 1.253 |
| dinv_N4 | 9 | 0.080 | 0.077 | 1.039 | 0.116 | 0.088 | 1.318 |
| dinv_N6 | 11 | 0.130 | 0.125 | 1.040 | 0.140 | 0.134 | 1.045 |
| dinv_N8 | 11 | 0.152 | 0.150 | 1.013 | 0.204 | 0.218 | 0.934 |
| di_N12 | 11 | 0.217 | 0.207 | 1.048 | 0.359 | 0.331 | 1.088 |
| di_N15 | 11 | 0.273 | 0.263 | 1.038 | 0.386 | 0.365 | 1.055 |
| di_N20 | 11 | 0.392 | 0.385 | 1.020 | 0.542 | 0.560 | 0.968 |
| di_N25 | 11 | 0.510 | 0.496 | 1.026 | 0.702 | 0.751 | 0.935 |
| di_N30 | 11 | 0.696 | 0.677 | 1.027 | 0.915 | 0.885 | 1.035 |
| sys3x2_N3 | 211 | 4.021 | 3.939 | 1.021 | 2.686 | 2.500 | 1.074 |
| sys3x2_N4 | 503 | 11.191 | 10.990 | 1.018 | 6.810 | 6.454 | 1.055 |
| sys3x2_N5 | 973 | 23.961 | 23.309 | 1.028 | 12.376 | 11.089 | 1.116 |
| sys3x2_N6 | 1503 | 40.055 | 38.988 | 1.027 | 18.709 | 20.658 | 0.906 |
| sys3x2_N7 | 2095 | 58.709 | 56.867 | 1.032 | 28.976 | 31.471 | 0.921 |
| masses2_N4 | 307 | 18.069 | 17.813 | 1.014 | 6.168 | 6.486 | 0.951 |
| masses2_N5 | 493 | 30.830 | 30.256 | 1.019 | 9.396 | 9.166 | 1.025 |
| masses2_N6 | 901 | 61.893 | 61.939 | 0.999 | 16.547 | 15.455 | 1.071 |
| masses2_N7 | 1641 | 116.676 | 112.157 | 1.040 | 28.533 | 27.973 | 1.020 |
| masses2_N8 | 2837 | 205.279 | 202.437 | 1.014 | 53.198 | 50.800 | 1.047 |

Serial columns come from job 45863586, 16-thread columns from job 45863588.

## Summary

| Comparison | Median ratio | Min | Max | Median over the 10 problems of 4 ms or more |
|---|---:|---:|---:|---:|
| A/B serial (45863586) | 1.028 | 0.999 | 1.070 | 1.020 (0.999 to 1.040) |
| A/B serial, replicate inside the 16-thread job (45863588) | 1.020 | 0.964 | 1.159 | not computed |
| A/B 16 threads (45863588) | 1.045 | 0.906 | 1.318 | 1.036 (0.906 to 1.116) |
| A/A serial (45863589) | 1.004 | 0.958 | 1.233 | 1.001 (0.990 to 1.011) |
| A/A 16 threads (45863589) | 1.024 | 0.693 | 1.548 | not computed |

## Noise floor

On the ten problems of 4 ms or more, the serial A/A ratios stay within 0.990 to 1.011 (median 1.001), so serial differences above about 1% on those problems are real. The thirteen sub-millisecond `di` and `dinv` problems are noisy even serially (A/A ratios up to 1.23), so only the larger problems carry weight. At 16 threads the noise on a shared node is large: A/A ratios range from 0.69 to 1.55, and single problems swing by 30% or more. No per-problem 16-thread ratio is trustworthy, and the 16-thread median (1.045 against 1.024 for A/A) cannot be told apart from noise.

## Conclusion

Serial: the skip is a real gain of about 2% (median 1.020 to 1.028; 1.020 over the larger problems, none below 0.999, against an A/A band of 0.990 to 1.011). At 16 threads the measurement is inconclusive on these shared nodes.
