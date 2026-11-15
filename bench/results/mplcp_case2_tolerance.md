# mplcp: the Case-2 sign test of `pivot_facet` (phase 1)

Main at f017013 against the fix, measured on Negishi (AMD EPYC 7763; every check a 1-CPU task or an 8-CPU build).
Raw per-task outputs were kept on the cluster; the numbers below are copied from them.

## The bug and the change

`pivot_facet` Case 2 compares the candidate leaving rows l and j through the lexico-inequality
Γ = a_l row_j − a_j row_l ⪰ 0. Γ scales with a_l a_j, so for two rows with a near PTOL (1e-10) the whole vector sits
at 1e-20 to 1e-10, below the absolute ZTOL (1e-9) that decides "zero" and the lexicographic sign. Both directions read
as "zero, so nonnegative" and both twins are accepted. On pop_001 at the parent active set `[3,5,11,14,15,18]`, facet
k=9, the twin rows 4 and 23 have a ≈ 1.18e-10; the lexico-infeasible basis `[3,4,5,11,14,18,20]` was returned next
to the good `[3,5,11,14,18,20,23]`, and `drop_lex_infeasible` hid it at the end.

The fix (`ppopt_rs/src/algorithms/mplcp.rs`):

- Case 2 works on the scale-free vector row_j/a_j − row_l/a_l. The zero test of the θ part uses
  `ZTOL * (1 + largest scaled entry)`, and the lexicographic sign (`lex_nonneg_rel`) skips entries below
  `ZTOL * (largest entry of the vector)`. The facet-cut rows are unchanged in meaning (positive scaling).
- `in_z` in `pivot_facet` now uses the tolerance `adjacent` used to find the vanishing rows Z, a relative one
  (`ZTOL * sc`, with `sc` the row's magnitude computed in `adjacent` and passed down per variable in `Facet::sc`).
  Before, `adjacent` could find a vanishing row that `in_z` did not, so P ∩ Z was empty at the first iteration and
  Case 2 took a finite α step off the facet.
- `drop_lex_infeasible` is removed. `count_lex_infeasible` (pub) is the check used by the tests and `examples/lexcheck.rs`.
- The parallel path (`pivot_solve_par`) shares `expand_candidates` and `adjacent`, so it has the same change.
  Its order gate (parallel region order equals serial) passes; it was not timed.

## pop_001: 3529 against 3535

Main returns 3535 regions, the fix 3529 with the filter removed. The six regions only main has:

| active set | Chebyshev radius | lexico-infeasible | where its Chebyshev centre lies in the fixed solve |
|---|---:|---|---|
| `[0,3,4,8,10,12,16]` | 1.5e-5 | no | inside `[0,3,8,10,12,16,23]` |
| `[3,4,5,12,14,17,20]` | 4.5e-8 | no | inside `[3,5,12,14,17,20,23]` |
| `[3,4,5,8,12,14,17]` | 9.5e-8 | no | inside `[3,5,8,12,14,17,23]` |
| `[3,4,7,8,11,15,16]` | 6.3e-7 | no | inside `[3,7,8,11,15,16,23]` |
| `[4,5,7,10,15,16,21]` | 1.1e-3 | yes | inside `[5,7,10,15,16,21,23]` |
| `[4,5,8,10,12,17,20]` | 1.4e-6 | no | inside `[5,8,10,12,17,20,23]` |

All six are the twin of an existing region with row 4 in place of row 23: the same parameter set counted twice.
The end filter let all six through (only the fifth is lexico-infeasible by the basis check; the others are
lexico-feasible on their own but cover a parameter set that the 23-twin already covers). So the six were not real
regions, and the fix loses nothing.
(`examples/extra_regions.rs` produced the table.)

Other evidence for the fixed solver on pop_001: 0 lexico-infeasible regions, and in 20,000 sampled parameters (9,018
feasible) none lies in no region and none strictly inside two. The other RPOPT solvers do not give a comparable
reference: `geometric` returns 3531 regions with a different representative of each dependent-row class and
`graph` / `combinatorial_graph` return 4696 / 4708 (they count degenerate splits differently), so their active sets
cannot be diffed against `mplcp`'s.

## Region signatures against main (`examples/region_hash.rs`, hash of E and f rounded to 1e-9)

135 of the 143 problems of `bench/problems` and `bench/problems_large` finished for both main and the fix; 117 are
identical, 18 differ.

| problem | main regions | fixed | explanation |
|---|---:|---:|---|
| pop_001 | 3535 | 3529 | the six twin regions above |
| masses3_N5 | 10440 | 10441 | +1 |
| masses3_N6 | 22154 | 22151 | -3 |
| masses3_N7 | 38371 | 38370 | -1 |
| masses4_N4 | 16043 | 16044 | +1 |
| masses5_N3 | 5052 | 5054 | +2 |
| pop_005 | 10298 | 10297 | -1 |
| pop_007 | 12250 | 12254 | +4 |
| pop_032 | 178995 | 178965 | -30 |
| pop_047 | 48207 | 48146 | -61 |
| masses3_N4, masses4_N3, pop_008, pop_028, pop_083, pop_091, pop_097, rand_t6_x6_m24_s0 | same count | same count | hash differs only |

Not investigated one by one: the count changes other than pop_001 (the coverage check below shows no lexico-infeasible
region and no new gap on any of them). Where the count is the same and only the hash differs, masses4_N3 shows the
cause: the active sets are identical and one region (`[1,23,50]`) keeps 27 facet rows instead of 28 (a borderline
redundant row of the vertex clip, which depends on the tableau path that produced the law). The other seven were not
checked this way. The 8 problems that did not finish for the fix (masses4_N5, masses5_N4, pop_002, pop_024, pop_033,
pop_036, pop_051, pop_095) did not finish for main either (six of them) or ran out of memory (see below).

## Coverage and lexico-feasibility of the fixed solver (`examples/lexcheck.rs`, 4000 samples per problem)

135 problems, 1,097,907 regions in total, 335,374 feasible samples:

- **lexico-infeasible regions: 0 on every problem.** The entry-facet skip (proved for lexico-feasible parents) therefore
  only sees lexico-feasible parents; removing `drop_lex_infeasible` changes nothing on these problems.
- Gaps: pop_081 only (3070 of 4000). It returns 2 regions on main as well (the zero-region failure
  of the open item list, not caused by this change).
- Samples strictly inside two regions: pop_011 119 of 3682, pop_005 70 of 1385, pop_007 23 of 612, none elsewhere.
  **These overlaps are on main too** (`examples/overlap_pairs.rs` on main: pop_011 119, pop_005 70, pop_007 7). In each
  pair both regions are lexico-feasible, with Chebyshev radius 0.6 to 5, and the two laws disagree; the direct QP
  solve agrees with the smaller region. So the larger region (for example `[2,5,6,11,28]` on pop_007, created late in
  the walk) is a stray region with a wrong facet set. It is a separate pre-existing defect; the fix did not cause it
  (pop_011 is identical to main, pop_005 differs by one region, pop_007 has 4 more regions and 16 more overlapping
  samples than main, so on pop_007 the fix moved but did not remove it). Open: its cause is not known.

## `depth > 6` guard (`--features lp_census`, `bench/results/lp_census_highdim.md` table 3c)

Returns by the guard, main against the fix (one run each, counts are exact):

| problem | main | fixed |
|---|---:|---:|
| pop_005 | 11279 | 13061 |
| masses4_N3 | 1158 | 999 |
| masses3_N4 | 628 | 550 |
| masses5_N2 | 254 | 236 |
| masses3_N3 | 97 | 85 |
| pop_017 | 0 | 0 |
| pop_001 | 6 | 1 |

The guard still fires on pop_005, masses4_N3, masses3_N4, masses5_N2 and masses3_N3. pop_017 shows 0 in both
builds in this run. The change is small except on pop_001. Repeated bases (a branch reaching a basis already reached
in the same `adjacent` call) before and after: pop_005 4% / 6%, masses4_N3 0% / 1%, masses3_N4 0% / 0%, masses5_N2 1% / 1%,
masses3_N3 0% / 1%, pop_017 0% / 0%. Depth histograms (calls at depth 0/1/2/3/4/5/6/7+) fixed build: pop_005
55802/43355/1459/2606/4453/7591/11885/16812; masses3_N4 30393/30528/2975/509/411/475/501/669; masses3_N3
8465/7883/575/97/91/93/100/114. The guard was not changed. The count of first-iteration Case-2 steps (the effect of
the `in_z` alignment on its own) was not measured.

## Tests

`cargo test`: 139 passed, 0 failed, 3 ignored (main had 137 passed; the two new tests are
`mplcp_pop_001_twin_rows_lex_feasible` and the unit test `lex_nonneg_rel_is_scale_free`).

## Serial timing, main against the fixed build (1 CPU per task, release binaries, `NOGEO`)

Ratio > 1 means the fixed build is faster. Each task runs the binaries one after another (rotated order) on one node and the
ratio is taken inside the task; the table gives the median over the tasks of a problem (4 to 5 alternations;
the three largest problems run one pair per task).

| group | main / fixed, median (min to max) | A/A (main / copy of main) |
|---|---|---|
| 13 problems of 4 ms or more | 1.043 (0.965 to 1.106) | 1.003 (0.948 to 1.145) |
| all 27 small and medium problems | 1.047 (0.965 to 1.184) | 1.003 (0.948 to 1.145) |
| masses3_N4 (21 s) | 1.452 | 1.001 |
| pop_001 (19 s) | 1.583 | 0.998 |
| pop_017 (18 s) | 1.016 | 1.002 |

The correctness change is not slower, and is much faster on masses3_N4 and pop_001 (probably the aligned `in_z`, which
removes first-iteration Case-2 steps, and the end filter's tableau rebuild per region; the split was not measured).
Speed was not chased.
Per-problem ratios: raw records `ab/res` and `ab/res2` under `~/rpopt/worktree-agent-a2cd0480ac0fec818/` on Negishi.

## Memory of the six problems that did not finish at 2 GB

`lexcheck` peak resident memory (one task each, `sacct MaxRSS`): pop_002 5.3 GB, pop_024 4.1 GB, pop_051 8.1 GB,
pop_033, pop_036, pop_095 above 8.3 GB (killed at an 8 GB limit). These problems cannot be run with the 2 GB per
CPU of the account's limit; the first three need 3 to 5 CPUs worth of memory.
