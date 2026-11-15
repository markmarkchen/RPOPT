# mplcp: work budget on the vertex clip (2 to 5 parameters)

Branch `worktree-agent-a8095c6d163dbc0a4` (code commit d42a11f), merged with main 8033548's parent. Negishi, one CPU per task, SLURM arrays 45927457 (census), 45927807 (gate), 45927808 (timing).

## The problem

`mplcp_build::clip_n` cuts the parameter box with the region's rows and keeps the vertices. Its cost is quadratic in the vertex count (edge pairs times duplicate scans). When the polytope has more than 600 vertices the clip gives up, and the region goes to the LP path, which takes 0.1 to 0.3 ms. A clip that fails at the guard has already spent 12 to 170 ms. Census over the 60 problems at 2 to 5 parameters that mplcp finishes (`lp_census --pair`, time in `clip_flags` split by outcome):

| problem | parameters | regions | solve ms (census) | ms in failed clips | failed clips |
|---|---:|---:|---:|---:|---:|
| pop_001 | 5 | 3,529 | 16,772 | 16,363 (98%) | 98 |
| pop_048 | 4 | 46 | 28.5 | 25.0 (88%) | 2 |
| pop_019 | 4 | 6,857 | 2,397 | 1,824 (76%) | 10 |
| pop_096 | 5 | 258 | 39.2 | 26.5 (68%) | 1 |
| pop_057 | 5 | 34 | 4.1 | 1.5 (37%) | 2 |

## The work counter and the budget

`clip_n` counts work units: vertices times one row per cut, plus one unit per edge pair examined and the length of the vertex list for every new vertex (the duplicate scan). One unit is about 1.5 ns (pop_001: a failed clip is 167 ms and 1e8 units). Histogram of the work of every clip, by number of bits (bucket b = work in [2^(b-1), 2^b)), summed over the 60 problems:

```
bucket b = work in [2^(b-1), 2^b)
nt 2
 ok   [(5, 81.0), (6, 252.0), (7, 120.0), (8, 6.0)]
 fail []
nt 3
 ok   [(6, 18.0), (7, 1313.0), (8, 6971.0), (9, 5181.0), (10, 983.0), (11, 202.0), (12, 19.0), (13, 4.0)]
 fail [(8, 1.0)]
nt 4
 ok   [(7, 1.0), (8, 122.0), (9, 3797.0), (10, 15515.0), (11, 11176.0), (12, 4509.0), (13, 871.0), (14, 111.0), (15, 8.0), (16, 1.0), (17, 3.0), (19, 1.0), (22, 1.0)]
 fail [(19, 2.0), (20, 1.0), (21, 1.0), (23, 3.0), (24, 1.0), (25, 1.0), (27, 3.0)]
nt 5
 ok   [(7, 3.0), (8, 2.0), (9, 12.0), (10, 209.0), (11, 4645.0), (12, 47232.0), (13, 60615.0), (14, 18118.0), (15, 4384.0), (16, 823.0), (17, 120.0), (18, 17.0), (19, 1.0)]
 fail [(12, 1.0), (18, 6.0), (19, 16.0), (20, 9.0), (21, 8.0), (22, 10.0), (23, 5.0), (24, 8.0), (25, 11.0), (26, 7.0), (27, 30.0)]
```

Successful clips: 3 of about 200,000 needed 2^18 units or more (nt 4: one in bucket 19, one in 22; nt 5: one in 19). Failed clips are almost all in buckets 19 to 27 (2^18 to 2^27 units). The default budget is therefore **2^18 = 262,144 units (about 0.4 ms)**: a clip that exceeds it aborts and the region takes the LP path. `mplcp::set_clip_budget(u64::MAX)` or `RPOPT_CLIP_BUDGET=<n>` in `region_hash` and `lexcheck` switches it off.

## Gate

* `lexcheck`, budget off against default, on 60 problems at 2 to 5 parameters (6 are heavy POP problems left out): region counts, lexico-infeasible counts (0), sampled gaps and overlaps, the sorted-active-set hash **and the rounded-row hash** are identical on all 60.
* Suite on the tree: see the report; 

## Paired timing (regbuild_ab: modes main (budget off), new (default budget), main again as the A/A pair, shuffled, 3 alternations per problem; ratio inside the task, median over tasks; above 1 = faster)

| problem | regions | tasks | main ms | new ms | speed-up median | min | max | A/A median | A/A min | A/A max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| masses2_N8 | 2837 | 3 | 145.7 | 144.7 | 0.996 | 0.996 | 1.007 | 1.006 | 0.979 | 1.017 |
| pop_001 | 3529 | 3 | 16711.9 | 388.0 | 43.087 | 42.957 | 43.239 | 1.000 | 0.999 | 1.000 |
| pop_019 | 6857 | 3 | 2449.8 | 560.9 | 4.368 | 4.308 | 4.413 | 0.991 | 0.990 | 1.010 |
| pop_040 | 2007 | 3 | 147.8 | 141.7 | 1.040 | 0.861 | 1.043 | 0.968 | 0.881 | 0.988 |
| pop_048 | 46 | 3 | 28.5 | 3.7 | 7.656 | 7.384 | 7.699 | 1.001 | 0.996 | 1.016 |
| pop_057 | 34 | 3 | 3.0 | 3.0 | 0.982 | 0.894 | 0.999 | 1.004 | 0.948 | 1.007 |
| pop_096 | 258 | 3 | 39.0 | 11.4 | 3.435 | 3.239 | 3.446 | 1.010 | 1.010 | 1.012 |

pop_040 and pop_057 are inside their A/A bands (their A/A minimum is 0.88 and 0.95); masses2_N8 is unchanged. Problems at 4 and 5 parameters that clip a lot (rand_t4 and rand_t5 problems) are not on the cluster's problem directories, so the check uses the POP problems and masses2_N8.

## Not changed

The shared builder (`mpqp_utils::clip_n`, used by `graph`, `combinatorial_graph` and `geometric`) has the same 600-vertex guard and the same cost on a failed clip; its census was not taken here.
