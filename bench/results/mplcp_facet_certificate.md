# mplcp: skipping the Case-2 neighbour search across facets with a known neighbour (phase 2)

Phase 1 (`mplcp_case2_tolerance.md`) against phase 1 plus the facet certificate. Negishi, AMD EPYC 7763, every
correctness check and every timing run a 1-CPU task; builds 8 CPUs. Serial path only.

## What is skipped, and why it is sound

`adjacent` finds the neighbour across facet F (row k of the current region) by the fast path (Case 1, a single
principal pivot) or, when that fails, by `pivot_facet` (Case 2), which is 94 to 100% of the time spent in
`adjacent` at 6 or more parameters. 82 to 99% of what it returns is a region already known.

**Certificate.** Skip the Case-2 search of F when a single known region R has a row on the same hyperplane with the
opposite normal and R contains F. Containment is checked on the vertices of F (the current region's vertices that
are tight on row k, spanning nt − 1 dimensions, so F is their convex hull) against every row of R; F ⊆ R then
follows by convexity. R is full dimensional (it was built with a positive Chebyshev radius), F lies in its facet
on that hyperplane, so R fills the far side of every relative interior point of F. Lexico-feasible regions do not
overlap (Lean `lex_unique`), so no other region reaches across F, and `adjacent` could only return R, which is
already in `seen`. The Lean model has this as `known_region_unique_neighbour` and `known_region_skip`, with the
hypothesis that R's basis is lexico-feasible; after phase 1 every returned region is (0 lexico-infeasible regions
on 135 problems). The certificate does not rest on the Case-2 theorem.

Implementation (`mplcp.rs`, `mplcp_build.rs`):

- The fast clip builder returns the vertices of the region polytope (flattened) with the region, only when asked
  (`want_verts`), so problems outside the certificate's range pay nothing. Without vertices (more than 6
  parameters, more than 128 rows, more than 600 vertices, or the shared-builder fallback) there is no certificate.
- `Known`: a hash map from the unit-normalised, 1e-5-rounded hyperplane (a, b) to the indices of the regions that
  have a row there. It only generates candidates; the opposite row is then confirmed to 1e-7 and containment to
  1e-9 (relative), so a rounding-edge miss loses a skip and never soundness.
- The check is asked inside `adjacent`, after the fast path has failed, so it is paid only by Case-2 facets
  (asking it for every facet cost more than it saved).
- Only regions that exist when the current breadth-first wave starts count as known (`CertCtx::limit`). The serial
  method and the wave-parallel method then take identical skip decisions and return the regions in the same order
  (tested at 4 threads).
- Switch: `mplcp::set_certificate(mode)`: 0 off, 1 on (default, 3 to 5 parameters), 2 audit (certify but still
  search), 3 on and 4 audit at every count the clip covers (2 to 6). Environment `RPOPT_CERT` in the examples.

**The shared index for `mplcp-build` (what changed).** `seen` in `pivot_solve` and `pivot_solve_par` is now
`Seen = FxHashMap<Vec<usize>, Slot>` with `Slot::Built(region index) | Slot::Rejected`, keyed by the exact active set
returned by `Plcp::active_set`. `seen_claim` marks a new key `Rejected`; when the region is pushed the key of its
`active_set` becomes `Built(index)`. The hyperplane map `Known` carries the same region indices, but it is rounded
and must stay a candidate generator. `build_region` now returns `(region, centre, Option<vertices>)` and takes
`want_verts`; `adjacent` takes `(cert, audit)` and returns `(Vec<Nbr>, Option<usize>)`.

## Results by parameter count

Fraction of Case-2 facets certified (audit run, `lp_census`; "Case-2 facets" are the `adjacent` calls that reached
the slow path):

| parameters | problem | Case-2 facets/region | certified | Case-2 share of `adjacent` calls, off then on | neighbour search share of solve, off then on |
|---:|---|---:|---:|---|---|
| 3 | sys3x2_N7 | 0.1 | 11% | 3% then 3% | 25% then 26% |
| 3 | pop_006 | 1.4 | 41% | 27% then 16% | 62% then 51% |
| 4 | masses2_N8 | 2.2 | 34% | 29% then 19% | 57% then 48% |
| 5 | pop_001 | 3.0 | 31% | 33% then 23% | 1% then 1% |
| 6 (mode 4, not default) | masses3_N3 | 8.1 | 35% | | |
| 6 (mode 4) | masses3_N4 | 7.7 | 33% | | |
| 6 (mode 4) | pop_017 | 7.1 | 36% | | |
| 6 (mode 4) | rand_t6_x5_m20_s0 | 6.8 | 67% | | |
| 7 to 10 | all problems of the high-dimension set | none | 0% | | |

At 7 or more parameters the clip builder does not run (it stops at 6; a coming change of `mplcp-build` stops it at
5), so there are no vertices and **nothing is certified or skipped**: the certificate does not touch the
high-parameter gap. It would need a containment test without vertices (for example one LP per row of R over F),
whose cost against a Case-2 search of 10 to 50 us was not evaluated.

The check costs 0.3 to 1.4 us per Case-2 facet at 3 to 4 parameters, 3.9 us at 5 and 4.1 to 7.0 us at 6 (18 us on
rand_t6_x5_m20_s0, many vertices per facet: 32% of its solve time), against a Case-2 search of 10 to 51 us (9.6 us
on rand_t6, 10.8 on sys3x2_N7, 14 to 28 on pop_006, pop_017, masses2_N8, pop_001, masses3_N3, 50 on masses3_N4).
That is why the default range stops at 5.

## Region signatures (`examples/region_hash.rs`, 1e-9 rounding), 26 problems

12 bench problems and 14 large ones, p1 against p2:

- **Mode 1 (default): identical to phase 1 on 25 of 26.** pop_001 has the same 3529 active sets
  (`examples/cert_diff.rs` finds no active set in only one run) and 3 fewer facet rows (36238 against 36241), the
  borderline-redundant-row effect already seen in phase 1. Mode 0 (off) equals phase 1 on the 12 bench problems.
- **Mode 3 (certificate also at 6 parameters): not sound here.** masses3_N4 loses one region (3642 against 3643): the
  active set `[12,31,33]`, lexico-feasible, Chebyshev radius 5.9e-6, whose centre lies in no region of the
  certified run and whose QP active set is `[12,31,33]`, so it is real. masses3_N3 and masses3_N5 have equal counts
  and different facet rows only. The likely cause is that one known R passes the 1e-9 containment test and is not the
  true region across F (the stray, overlapping regions of pop_005/007/011 described in phase 1 are of this kind),
  but this was not isolated. The default range excludes 6 parameters; mode 3 is only for experiments.

## Tests

`cargo test --profile gate`, one 1-CPU task per test: see the totals in the report. New: `tests/mplcp_certificate.rs`
(its own binary, it flips a process-wide switch): on sys3x2_N5, masses2_N6 and masses3_N3 the regions are the same
set with the certificate on as off, the wave-parallel order equals the serial order, and the audit certifies a
positive number of facets.

## Serial timing, phase 1 against phase 1 plus certificate (1 CPU, release, `NOGEO`)

Per task: p1, p2 and a copy of p2 once each (or one pair for the three largest problems), ratios inside the task,
median over 4 to 5 tasks per problem. Ratio above 1 = the certificate build is faster.

| parameters | problems | p1 / p2 per problem |
|---:|---|---|
| 2 | 13 di and dinv problems (0.07 to 0.6 ms) | 0.95 to 1.03, median 1.000 (noise at this size) |
| 3 | sys3x2_N3 to N7 | 1.002, 1.012, 0.988, 0.977, 0.944 (A/A 0.95 to 1.00 on N7) |
| 3 | pop_006 | 1.149 |
| 4 | masses2_N4, N5, N6, N7, N8 | 1.119, 1.099, 1.120, 1.100, 1.055 |
| 5 | pop_001 | 1.003 |
| 6 | masses3_N3, masses3_N4, pop_017, rand_t6_x5_m20_s0 and s1 (certificate off) | 0.997, 0.998, 0.999, 1.001, 1.004 |

All 30 problems: median 1.001 (0.944 to 1.149); the 16 problems of 4 ms or more: 1.004. A/A (p2 against a copy of p2):
median 1.000, 0.951 to 1.016. So the certificate is a real gain of 5 to 15% where Case-2 searches are a large share
of the time (masses2 family, pop_006) and neutral elsewhere. It is not a lever at 6 or more parameters.
(The first version, which checked every facet before calling `adjacent`, measured 0.74 on rand_t6_x5_m20_s0 and
0.96 on rand_t6_x5_m20_s1; asking only for Case-2 facets and limiting the range to 3 to 5 parameters removed that.)

## Not done

- The Case-2 hint (return the sub-facet Chebyshev centre with each basis and use it as the candidate's LP hint): no
  edits in `pivot_facet` beyond phase 1. Open.
- A certificate for more than 6 parameters, and a cause for the mode-3 loss on masses3_N4.
- The stray overlapping regions of phase 1 (pop_005, pop_007, pop_011) and the zero-region failure of pop_081.
