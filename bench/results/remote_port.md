# Port of the collaborator's RPOPT branches (Nov 2026)

Source: `markmarkchen/RPOPT` (a `git subtree split` of `ppopt_rs/`), all branches from remote commit 9eef9da.
Local base: main 1feb98b. Each remote commit is one local commit with the original author and message plus a
"Port:" line.

## What was ported

| Remote branch | Remote commit | Content |
|---|---|---|
| `fix/duplicate-constraint-rows` (also under `fix/weakly-active-duplicates`) | b7224ea | Merge equal rows of `[A | F | b]` (after `scale_constraints`) in `base_constraint_processing`; an equality replaces an inequality it duplicates; `solve_mpc_dp` dedups mapped active sets; `tests/duplicate_rows.rs` |
| `fix/weakly-active-duplicates` | 1f25ef9 | For mpQPs, `solve_mpqp_with` drops the region labelled A \ {i} when the region labelled A has the same law and lambda_i = 0; `tests/weakly_active.rs` |
| `bench/online-qp` | e3d85fc | `bench/online/` (C++ benchmark), `examples/export_online.rs` |
| `fix/bench-ppopt-scripts` | 2ca64c1 | fixes to three PPOPT bench scripts |
| `results/m3pro-2026-11` | 4577110 | `bench/results_m3pro/` raw results |

All five patches applied with `git am -3 --directory=ppopt_rs` with no textual conflicts. The conflicts were
semantic, with our mplcp dedupe:

* The merge runs in `base_constraint_processing`, which `new` calls before `post_process`, so `post_process=false`
  does not skip it. Added `MpProgram::new_opts`, `from_problem_data_opts` and `from_json_file_opts` with a
  `merge_duplicates` switch (default on everywhere).
* `mplcp_pop_001_twin_rows_lex_feasible`, `dedupe_keeps_the_regions_and_the_output_shape` and
  `no_region_is_skipped_without_trace` build their programs with `merge_duplicates=false`, so they still test
  mplcp's own twin handling. Their pinned values are unchanged (rows 4 and 23, 3,529 regions).
* `dependent_start.rs` (the dependent-start and no-overlap regressions) runs each program both without and with
  the merge. All of these pass locally in release.
* `tests/weakly_active.rs` expects `{}` for `Mplcp` instead of `{3}`: mplcp's lexicographic rule rejects the basis
  {3} (identically zero multiplier) and reports the region once under the smaller label, so the drop never fires
  for it. The other algorithms report `{3}`. 4 regions each.
* `examples/region_hash.rs` (gate tool): under `RPOPT_CANON` it maps the sets of the merged program to the
  indexing of the program built without the merge (row matching, then the lowest-index twin).

mplcp's internal dedupe (`set_dedupe`, `duplicate_rows`, `solve_with_dedupe`) is kept as a safeguard. On a
merged program it is a no-op.

## Index question

Solutions index the processed rows of the program (after rows moved to `A_t`, scaling, this merge and the
strong-redundancy pass), not the rows of the input file. Before the port that was already so (rows move to `A_t`
and redundant rows go). The merge shifts the indices of the rows after a dropped duplicate down. Active sets of
the ported tree are therefore comparable with main only after mapping, which `region_hash` does under
`RPOPT_CANON`. Exception: mplcp reports lowest-index twins on main (via its dedupe), and the merge keeps the first
copy too, so the mapped sets coincide.

## Gate results (Negishi, 1-CPU tasks, mplcp, `gate` profile)

* `cargo check --tests --examples`: passes. `cargo check --features lp_census --examples` fails on main 1feb98b
  already (`MU_AUDIT` not found at `src/algorithms/mplcp.rs:1793`, left over from c73b8b8); not touched.
* Region sets, ported tree against main 1feb98b on the 153 problems of the earlier audit list (the 159 sweep names
  minus pop_002, 024, 033, 036, 051, 095; the problem list of "158" would add those six, which we skipped
  as before): **identical on 153 of 153**, the same count and the same sorted active sets after mapping. No
  region set changes, so there is no twin-merge or weakly-active change to confirm with `pair_check`. The merge
  shrinks the processed program on 49 POP problems (for example pop_031 9 to 7 rows, pop_081 5 to 3, pop_011
  26 to 22); mplcp already removed those twins internally.
* The weakly-active drop never fires for mplcp (see above).
* Time: no mplcp solve got slower. Ratios per task on shared nodes, one run each, with 1-CPU noise of a few
  percent: pop_001 16.4 s to 16.5 s, pop_007 3.77 to 3.93, pop_028 33.2 to 27.0 (noise or load, single run), others
  within a few percent.

## Other solvers on the twin POP problems (region counts)

| Problem | POP v1.5 | mplcp | main: graph, comb., comb_graph, p_daqp | main: geometric | ported: all five |
|---|---|---|---|---|---|
| pop_010 | 36 | 36 | 37 | 36 | 36 |
| pop_031 | 4 | 4 | 6 | 4 | 4 |
| pop_052 | 3 | 3 | 4 | 3 | 3 |
| pop_071 | 5 | 5 | 7 | 5 | 5 |
| pop_081 | 4 | 4 | 6 | 4 | 4 |
| pop_093 | 7 | 7 | 8 | 7 | 7 |

Other POP problems whose processed program the merge shrinks (only those with at most 500 mplcp regions; the
array was cut short because the shared 64-CPU cap held it in the queue, so pop_035 is partial and the remaining
problems of the list were not run):

| Problem | POP | mplcp | main (graph, comb., comb_graph) | main p_daqp | main geometric | ported (all five) |
|---|---|---|---|---|---|---|
| pop_011 | 381 | 397 | 408 | 406 | 397 | 397 (p_daqp 396) |
| pop_029 | 27 | 27 | 30 | 30 | 27 | 27 |
| pop_034 | n/a | 9 | 10 | 10 | 9 | 9 |
| pop_015, 022, 030 | 243, 2, 286 | same | same | same | same | same |
| pop_035 (graph, comb_graph, p_daqp only) | 76 | 77 | 77 | 77 | not run | not run |

Open points: pop_011 has 397 regions in mplcp and geometric against POP's 381, which is unchanged by the port.
parametric_daqp gives 396 on pop_011 after the port, one less than the others (not investigated).
Not run: the other 15 problems of the extras list and pop_035 on the ported tree.

## Not ported

Nothing was left out. The full test suite is for the lead to run on the cluster CI.
