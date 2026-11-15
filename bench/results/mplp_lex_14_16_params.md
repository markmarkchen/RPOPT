# mplp_lex at 14 and 16 parameters, and an LP failure that dropped regions

Round 8, the last check of the standing goal on mpLPs. The set `bench/problems_lp_highdim` grows by seven problems
(`bench/gen_highdim_lp.py`, `gen_random --lp`): oscillating masses with 7 and 8 masses (14 and 16 parameters), inf-norm
and 1-norm at N=1 (`masses8one_N1` is left out: `mplp_lex` needs over 100 s on the laptop), and four random mpLPs
(`randlp_t14_x4_m30`, `randlp_t14_x5_m50`, `randlp_t16_x4_m30`, `randlp_t16_x6_m50`).

## Step 1: every exact solver once (Negishi, build 45890871, array 45890872, 1 CPU, 600 s timeout, 1,900 MB)

A run with 0 regions, gaps or errors counts as a failure. Times are from this run (the walk before the changes below);
out of memory is the child killed at 1.9 GB.

| problem | parameters | regions | mplp_lex ms | graph | geometric | combinatorial_graph | combinatorial | mplcp_enum |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| masses7inf_N1 | 14 | 56 | 83 | 5,511 | 2,083 | 92 | 87 | 819 |
| masses7one_N1 | 14 | 131,012 | 172,192 | timeout | timeout | out of memory | out of memory | timeout |
| masses8inf_N1 | 16 | 60 | 176 | 13,358 | 3,591 | 184 | 182 | 1,241 |
| randlp_t14_x4_m30_s0 | 14 | 274 | 224 | 4,682 | 500 | 137 | 183 | 4,497 |
| randlp_t14_x5_m50_s0 | 14 | 745 | 797 | 38,412 | 2,394 | 938 | 10,830 | timeout |
| randlp_t16_x4_m30_s0 | 16 | 365 | 362 | 6,466 | 886 | 266 | 313 | 4,108 |
| randlp_t16_x6_m50_s0 | 16 | 2,046 | 5,538 | 148,830 | 10,175 | 3,588 | out of memory | timeout |

`mplp_lex` lost three of these: randlp_t14_x4 (1.63x), randlp_t16_x4 (1.36x) and randlp_t16_x6 (1.54x) against `combinatorial_graph`.

## Finding 1: the Chebyshev LP of the region builder fails on some nonempty systems

On masses8inf_N1 every exact solver (`mplp_lex`, `combinatorial`, `combinatorial_graph`, `geometric`, `mplcp_enum`) returned 59 regions and left 17 of 2,000
sampled parameters uncovered; only `graph` returned 60 and no gap. The missing region has basis (30, 48, 49): exact arithmetic
(`bench/exact_regions.py`) gives all multipliers positive (smallest 0.707) and a Chebyshev radius of 1.60, so it is a genuine fat region. The
shared builder rejected it: the Chebyshev LP of its 95-row system (16 parameters, so 17 LP variables) returns no solution, warm or cold, and solves
after removing any one of 40 rows or when the right hand side is relaxed by 1e-9 (`examples/gen_one.rs`).

Fix (`mpqp_utils.rs`, `chebyshev_ball_from_or_cold`): when the warm LP fails, retry cold, then with the rows in reverse order, then with the right hand
side relaxed by 1e-9 relative. The retries run only after a failure, so they change nothing that solved before.

Effect: masses8inf_N1 now has 60 regions and 0 gaps for all solvers. The same failure had dropped genuine regions elsewhere: masses5inf_N2 had 4,190
regions and now 4,191 (the extra region, basis (11, 33, 49, 65, 77, 79), has exact radius 7.8e-8 and positive multipliers), masses6one_N1 32,763 and now
32,767 (= 2^15 - 1), masses7one_N1 130,978 and now 131,012. Region hashes on the other 44 mpLPs are identical to before; the builder gate also holds on
the other solvers: `graph`, `combinatorial_graph`, `combinatorial` identical on 18 mpLPs, `mplcp`, `graph`, `geometric` identical on 5 mpQPs;
`geometric` on sys3x2inf_N3 returns 499 regions instead of 490 (0 gaps either way), the 9 regions it had lost to the same failure (the earlier
note that blamed the bounding-box skip for this difference was wrong: the skip only changed which LPs failed).
The underlying fragility is in the dense LP for more than 16 variables; this retry works around it and does not fix the solver.

## Finding 2: nothing beyond a facet

At 5 or more parameters a facet attempt fails 66% of the time on randlp_t16_x6 (36,144 attempts, 12,449 successes), many of them (the speedup below shows how many) facets on the
boundary of the feasible parameter set, where the LP path then spends a Chebyshev LP and a lexicographic solve to find the program infeasible. The
direct crossing now recognises these exactly: if no inequality row can leave the basis (every T_jp <= 0), the row j satisfies A_j x >= A_j x_B > b_j + F_j theta for every x
the basic rows allow, so the program is infeasible beyond the facet and the facet has no neighbour. The facet is skipped without an LP (the LP path skips it through
`Fail::Infeasible`). These skips do not enter the success rates of the gate: counting them made the gate open on the sliver MPC problems and cost
2.5% on masses3inf_N3.

## Checks and timing of the final code

- Region hashes: the default path equals the walk on all 53 mpLPs; the enumeration forced on equals the walk on 9 of them (the ones it can run);
  the 46 old mpLPs equal the previous hashes except masses5inf_N2 and masses6one_N1 (recovered regions, above).
- Serial A/B on Negishi (build 45893737, array 45893738): base is the code before this round (main 799a7cc, region counts below show the recovered regions),
  23 high-dimensional problems three times and 8 small ones five times, base / candidate / base again shuffled, 1 CPU. The 22 other
  `bench/problems_lp` problems have nt <= 4, where neither change applies (direct crossing is off); they were in the earlier 53-problem A/B of this round (0.97 to 1.02 with the version that let these skips enter the success rates) and are not slowed by these changes.

| problem | cand/base median | range | A/A median | A/A range | cand ms | regions cand/base | gaps cand/base |
|---|---:|---|---:|---|---:|---|---|
| diinf_N2 | 1.006 | 0.988-1.035 | 1.011 | 0.986-1.029 | 0.4 | 20/20 | 0/0 |
| diinf_N3 | 1.002 | 0.993-1.032 | 1.000 | 0.995-1.011 | 0.9 | 31/31 | 0/0 |
| diinf_N4 | 1.003 | 1.000-1.006 | 0.997 | 0.991-1.010 | 1.9 | 39/39 | 0/0 |
| diinf_N5 | 1.004 | 0.999-1.008 | 1.000 | 0.998-1.014 | 3.9 | 47/47 | 0/0 |
| dione_N2 | 1.008 | 0.995-1.026 | 1.009 | 1.000-1.021 | 0.3 | 20/20 | 0/0 |
| dione_N3 | 1.007 | 0.995-1.011 | 1.004 | 0.999-1.009 | 0.9 | 30/30 | 0/0 |
| dione_N4 | 1.001 | 0.997-1.018 | 1.004 | 0.993-1.009 | 1.9 | 43/43 | 0/0 |
| masses3inf_N2 | 0.997 | 0.994-1.013 | 0.998 | 0.983-1.012 | 183.6 | 740/740 | 0/0 |
| masses3inf_N3 | 1.002 | 0.996-1.013 | 0.998 | 0.998-1.011 | 3586.5 | 10684/10684 | 0/0 |
| masses3one_N2 | 0.989 | 0.988-0.994 | 0.996 | 0.988-1.003 | 3354.2 | 19076/19076 | 0/0 |
| masses4inf_N2 | 1.003 | 1.002-1.003 | 1.001 | 1.001-1.001 | 1897.4 | 1974/1974 | 0/0 |
| masses4one_N1 | 0.966 | 0.943-0.980 | 0.998 | 0.973-1.021 | 416.5 | 2048/2048 | 0/0 |
| masses5inf_N1 | 0.997 | 0.992-1.006 | 1.001 | 1.000-1.002 | 18.3 | 48/48 | 0/0 |
| masses5inf_N2 | 1.009 | 1.008-1.017 | 1.002 | 1.000-1.006 | 12828.6 | 4191/4190 | 0/0 |
| masses5one_N1 | 0.964 | 0.960-0.966 | 0.993 | 0.992-1.015 | 3126.8 | 8190/8190 | 0/0 |
| masses6inf_N1 | 0.996 | 0.992-1.001 | 1.000 | 0.988-1.009 | 34.7 | 52/52 | 0/0 |
| masses6one_N1 | 0.975 | 0.961-0.975 | 0.990 | 0.989-0.992 | 21974.0 | 32767/32763 | 0/0 |
| masses7inf_N1 | 1.001 | 0.978-1.003 | 1.005 | 0.979-1.008 | 82.6 | 56/56 | 0/0 |
| masses7one_N1 | 0.970 | 0.969-0.975 | 0.996 | 0.995-0.997 | 163272.6 | 131012/130978 | 0/0 |
| masses8inf_N1 | 1.095 | 1.073-1.111 | 1.001 | 0.976-1.005 | 178.7 | 60/59 | 0/17 |
| randlp_t10_x6_m50_s0 | 0.843 | 0.840-0.844 | 1.001 | 0.999-1.006 | 6524.1 | 12105/12105 | 0/0 |
| randlp_t12_x5_m40_s0 | 0.914 | 0.911-0.923 | 0.994 | 0.994-1.001 | 6.5 | 29/29 | 0/0 |
| randlp_t14_x4_m30_s0 | 0.594 | 0.593-0.597 | 1.000 | 0.999-1.008 | 133.6 | 274/274 | 0/0 |
| randlp_t14_x5_m50_s0 | 0.802 | 0.798-0.814 | 1.002 | 0.987-1.025 | 656.0 | 745/745 | 0/0 |
| randlp_t16_x4_m30_s0 | 0.716 | 0.709-0.718 | 1.000 | 0.984-1.005 | 264.6 | 365/365 | 0/0 |
| randlp_t16_x6_m50_s0 | 0.497 | 0.495-0.501 | 1.024 | 1.015-1.034 | 2928.8 | 2046/2046 | 0/0 |
| randlp_t5_x5_m40_s0 | 1.001 | 0.964-1.010 | 0.991 | 0.957-0.997 | 48.1 | 562/562 | 0/0 |
| randlp_t5_x5_m40_s1 | 0.963 | 0.952-0.982 | 1.003 | 0.995-1.005 | 132.3 | 1563/1563 | 0/0 |
| randlp_t6_x5_m40_s1 | 0.916 | 0.913-0.929 | 0.992 | 0.990-1.002 | 272.0 | 1818/1818 | 0/0 |
| randlp_t8_x5_m40_s0 | 0.807 | 0.802-0.807 | 0.997 | 0.996-1.006 | 104.4 | 557/557 | 0/0 |
| sys3x2inf_N2 | 1.004 | 0.999-1.006 | 0.998 | 0.992-1.005 | 7.0 | 152/152 | 0/0 |

Everything within 0.96 to 1.01 of base except the problems that gain (random mpLPs 0.50 to 0.96, the 1-norm MPC problems 0.96 to 0.98) and masses8inf_N1
(1.095: it now builds the 60th region that the base lost, which is also why its gaps fall from 17 to 0).

## Against the best other solver (paired: all solvers in the same 1-CPU task, 5 alternations, shuffled)

| problem | n | lex ms | combinatorial_graph ms | combinatorial ms | lex / cg (range) | lex / combinatorial (range) | valid |
|---|---:|---:|---:|---:|---|---|---|
| masses7inf_N1 | 5 | 82.4 | 92.0 | 87.4 | 0.904 (0.871-0.913) | 0.947 (0.926-0.954) | yes |
| masses8inf_N1 | 5 | 179.0 | 187.5 | 184.3 | 0.954 (0.930-0.962) | 0.975 (0.965-0.978) | yes |
| randlp_t14_x4_m30_s0 | 5 | 133.7 | 136.9 | 182.9 | 0.981 (0.934-0.983) | 0.732 (0.728-0.733) | yes |
| randlp_t16_x4_m30_s0 | 5 | 262.5 | 264.1 | 313.2 | 0.995 (0.986-0.997) | 0.839 (0.826-0.845) | yes |
| randlp_t16_x6_m50_s0 | 5 | 2858.2 | 3576.6 | n/a (out of memory) | 0.798 (0.775-0.803) | - | yes |

`mplp_lex` is fastest on all five, by 2 to 20% (randlp_t16_x4: 0.995 against `combinatorial_graph`, a tie within 0.5%), and on masses7one_N1 it is the only
solver that finishes. With the earlier tables: `mplp_lex` is now the fastest valid exact solver on every mpLP of the benchmark: the 30 of
`bench/problems_lp`, and the 23 of `bench/problems_lp_highdim` (6 to 16 parameters).
