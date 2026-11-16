# mplcp box chain: why it falls back, a certified bound, and per-solve state

Base main a1e846a (census: main 1cced96). Branch `worktree-agent-a3946c55839ffe05f`. Negishi, release profile, one solve per process in every
gate task. The code review is audit-build's (findings 10 and 12); the gate is its script set (`m9_*`, `m11_*`).

## 1. Why the chain falls back (census, one solve each, scratch counters, base code)

Chain objectives per solve: masses4_N3 47,232 (2,952 regions x 16), masses5_N3 (5,273 x 20), masses5_N4. "Counted" = what the guard counts today.

| cause | masses4_N3 | masses5_N3 | masses5_N4 |
|---|---:|---:|---:|
| objectives solved by the chain | 33,073 | 25,687 | 55,347 |
| fallback: slack negative at the verified vertex | 4,157 | 5,593 | 11,811 |
| fallback: certificate residual above 1e-8 | 3,702 | 2,479 | 5,163 |
| fallback: basis row not tight | 27 | 56 | 45 |
| fallback: singular refresh, no blocking row, tiny pivot, iteration limit | 0 | 0 | 0 |
| chains whose state could not be formed at an engine optimum (not counted by the guard) | 4,227 | 5,243 | 9,486 |
| chains without a shared start vertex (first LP by the engine) | 2,266 | 1,797 | 3,547 |
| chains started from a shared vertex | 637 | 218 | 674 |

- **Slack failures are not borderline:** 91% to 98% violate the 1e-8 margin by more than 1e4 times. **Residual failures are:** 66% to 85% are within 10
  tolerances. 88% of the failed verifications come after 6 or more pivots, and 85% with max|W| above 1e4 (ill-conditioned bases).
- **The state that could not be formed (94%):** n or more rows are tight at 1e-9 but the 1e-6 independence test refuses them (4,036 of 4,277 on masses4_N3,
  14,745 of 15,756 on masses5_N3). The engine's optimum is not a vertex: it lies inside an optimal face when the objective is parallel to a facet
  normal (the Theta box rows). Choosing the rows by pivoted Gram-Schmidt, or widening the tightness to 1e-7, does not help (8,545 and 27,745 refusals).
- **Rebuilding the state from the basis rows once, when a check fails** (W, R, y, vertex): verified after the rebuild 5,392 (65%) on masses4_N3 and
  12,215 (50%) on masses5_N3 (the updates had drifted); the simplex went on from the rebuilt state (feasible vertex, a negative multiplier) 93 and 366;
  the rebuilt vertex was infeasible, so the simplex had taken a wrong step (stale slacks or R in the ratio test), 2,845 (34%) and 12,003 (49%).
  Fallbacks per solve on masses4_N3 fall from about 7,900 to about 2,900 with the rebuild. Rebuilding at every optimum instead cost the same time.
- **Value of the chain** (solve ms, chain off / default / guard off): masses4_N3 1692 / 1615 / 1614, masses5_N3 9103 / 8783 / 8776, masses5_N4
  396.2 s / 398.8 s / 396.6 s: worth 4.5%, 3.5% and 0%; the guard changes under 1% (it fired at region 2,031 on masses5_N3 and 4,253 on masses5_N4, never on
  masses4_N3). An engine box LP costs about 2.5 us on these problems (all 47,232 of them are about 7% of the solve on masses4_N3), so the 1.19x
  ceiling is not reachable through the chain on the masses family; a crossover to a vertex inside the optimal face would save about 4,000 to 15,000 engine
  LPs per solve (under 1% of the solve) and was not built.

## 2. What changed

- **A certified bound instead of tolerances (finding 12).** At an accepted optimum the vertex is recomputed from the basis rows and the slacks must be
  at least -1e-8 (1 + |f|). With the multipliers y = W' c and r = c - sum y_i a_Ti the bound satisfies c.x' <= c.x + g for every x' of the region, with
  g = sum_{y_i>=0} y_i |s_Ti| + sum_{y_i<0} |y_i| sum_q |a_Ti,q| D_q + sum_q |r_q| D_q, where D_q = max(x_q - lo_q, hi_q - x_q) over the outer box (the propagated
  parameter box of the caller, which contains the region; without a finite outer box the chain is not used). The optimum is accepted when g <= 1e-6 (1 + |bound|);
  the returned box is widened by g, so the row screening is sound for any conditioning and a large g only costs screening power.
- **Rebuild on a failed check** as described above; an infeasible rebuilt vertex goes to the engine. `Stats::init_failures` counts every failed attempt to form a
  state; the guard counts these together with the failed objectives (`set_guard_count_init(false)` restores the old count).
- **Per-solve state (finding 10).** The fallback guard (`ChainGuard`, atomics) and the count of live parent tableaux (`live_parents`) are created in `pivot_solve` and
  `pivot_solve_par`, passed to the builder and to `TabSrc::resolve`/`discard`, and the switch `CHAIN` is read at solve start; the memory statistics and the last-solve
  record stay global and are reporting only. The adaptive counters of the DAQP filter restart with every solve (an epoch bumped at solve start).
- Reporting: `RPOPT_BOX_STATS=1` in `region_hash`, `examples/box_rows_diff.rs`, `box_chain_detail()`.

## 3. Gate

Reference: main a1e846a; new: head 6d68838. Tests: **150 passed, 0 failed, 5 ignored (155 tests, CI gate, tag order_ab11)**.

**Concurrency test** `concurrent_solves_match_sequential` (staggered starts 25 ms apart, two solves per thread, four rounds, one thread with the parallel method), run with
overflow checks and debug assertions on in the gate profile: on main it **fails 3 times of 3** (`attempt to add with overflow` at the budget test of `pivot_solve`,
`mplcp.rs:3861`, the wrapped live-parent counter); on the new tree it passes 3 of 3. (A first version that started all threads at once passed on both and proved nothing.)

**Region sets** (sorted active sets, region count; rows rounded to 1e-6 reported separately), 20 problems: the sorted active sets are identical to main on all 20.
Rows: identical on 16; different on three, all grazing rows:

| problem | row hash | rows dropped by the chain (against the shared builder) | rows only in the chain | cut into the chain's region by the dropped row |
|---|---|---:|---:|---|
| masses3_N4 | differs | 1 | 5 | 9.4e-11 inside (the region is the same) |
| masses4_N3 | differs (the guard switched the chain off at region 1,119, so main and the new run mix chain and shared rows) | 0 | 0 | |
| pop_002 | one row more (4,219,332 against 4,219,331) | not run | | |

No dropped row cuts a chain region by more than 1e-9 (`box_rows_diff`, 0 on all 19 problems run with it).

**Chain statistics, guard and gap distribution** (new tree; fallbacks include the chains that could not start; "off" = the guard switched the chain off; gaps in the objectives the chain solved):

| problem | regions via the chain | fallbacks | init failures | rebuild resolved | guard off at region | regions without an outer box | g median | g above 1e-9 | g above 1e-7 |
|---|---:|---:|---:|---:|---:|---:|---|---:|---:|
| masses3_N3 | 954 | 1,125 | 336 | 274 | no | 0 | 1e-11 | 28.0% | 10.6% |
| masses3_N4 | 3,642 | 5,184 | 1,377 | 1,399 | no | 0 | 1e-10 | 33.1% | 13.3% |
| masses4_N3 | 1,119 | 3,581 | 1,239 | 741 | 1,119 | 0 | 1e-12 | 28.3% | 12.8% |
| masses5_N3 | 370 | 1,496 | 704 | 271 | 370 | 0 | 1e-16 | 17.7% | 7.8% |
| masses8_N2 | 398 | 2,300 | 949 | 314 | no | 0 | 1e-16 | 6.2% | 2.7% |
| pop_005 | 10,293 | 2,255 | 1,974 | 182 | no | 0 | 1e-12 | 5.3% | 1.2% |
| pop_017 | 4,141 | 1,188 | 557 | 445 | no | 0 | 1e-12 | 11.3% | 3.6% |
| pop_002 | 321,025 | 64,169 | 14,729 | 39,606 | no | 0 | 1e-11 | 19.7% | 4.7% |
| rand_t8 | 4,110 | 203 | 64 | 387 | no | 0 | 1e-11 | 4.0% | 1.3% |
| rand_t12 | 6,142 | 2,757 | 1,357 | 1,101 | no | 0 | 1e-16 | 5.5% | 2.4% |
| rand_t14 | 2,469 | 278 | 54 | 112 | no | 0 | 1e-16 | 0.7% | 0.3% |
| rand_t16 | 1,875 | 2,486 | 2,460 | 11 | no | 0 | 1e-16 | 0.1% | 0.04% |

(masses2_N4 to N8 have fewer than 6 parameters: the chain is not used. The share of gaps above 1e-7, the caller's rejection margin, is up to 13% on masses3_N4,
masses4_N3 and masses3_N3: the widened box rejects fewer rows there.)

**Paired timing** (main, new, main again as the A/A; one task per problem and alternation, 5 alternations, shuffled order, 3 solves per binary, median):

| problem | main ms | new / main | alternations | A/A | guard |
|---|---:|---:|---|---:|---|
| masses4_N3 | 1527 | 1.020 | 1.020 1.020 1.027 1.024 1.018 | 0.998 | off at region 1,119 |
| masses5_N3 | 7925 | 0.991 | 0.988 0.991 0.988 0.991 0.994 | 0.999 | off at region 370 |
| rand_t12_x5_m20_s0 | 1865 | 0.979 | 0.975 0.983 0.983 0.975 0.979 | 1.001 | not fired |
| rand_t14_x5_m15_s0 | 686 | 1.001 | 0.999 1.028 1.001 0.999 1.004 | 1.000 | not fired |
| rand_t16_x5_m15_s0 | 671 | 0.998 | 0.995 1.000 0.978 1.001 0.998 | 1.003 | not fired |

The guard counting the chains that could not start, against not counting them (`set_guard_count_init(false)`, the old count), same two masses problems, 5 alternations:
masses4_N3 1.023 (counted: 1.028), masses5_N3 1.019 (counted: 0.993) of main's time. Counting the restarts makes the guard fire earlier (masses4_N3 at region 1,119, where
before it never fired), but not counting them is no faster: the cost is the widened box (13% of the gaps on masses4_N3 are above the caller's reject margin), not the
guard. Rule kept at 20%, not tuned in this gate.

## 4. Outcome

The chain is sound now (the bound is certified from the data for any conditioning) and one solve can no longer steer another, and it is no faster on masses: 2% slower on
masses4_N3, equal or 1% to 2% faster on the rest of the timing set (rand_t12 2.1% faster). The 5% to 6% that the chain was worth on masses4_N3 and masses5_N3 is given back
by the guard firing earlier together with the widened box.

Not done, noted: outcome (iii) of the rebuild (34% to 49% of the failed checks) points at the 50-pivot refresh period being too long for ill-conditioned bases; a refresh
every 8 pivots or when max|W| passes 1e4 is the obvious test (at most a few thousand engine LPs per solve at 2.5 us, so well under 1%). A crossover from a non-vertex
engine optimum (94% of the state failures) would remove 4,000 to 15,000 engine LPs per solve, also under 1%.
