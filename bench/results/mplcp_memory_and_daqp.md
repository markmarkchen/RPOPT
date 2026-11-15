# mplcp: a byte budget for live parent tableaux, and DAQP as an emptiness filter

All timings are serial, on Negishi (AMD EPYC 7763), one 1-CPU SLURM task per solve (or per paired alternation), median of
5 alternations with one A/A pair (`binary#2` against the same binary) per task. Memory runs use `region_hash`
(`VmHWM` is the peak resident size). Trees: `d2` = fb3389e (memory knobs, no budget), `e1` = 19b0c9e (budget),
`m3b`/`m8` = the branch merged with main c345cdc without and with the DAQP filter, `s1b`/`s1` = the same stacked on main
7457656 (scale-free pivot test). Scripts and raw outputs are under `~/rpopt/mplcp-build-speed/` on Negishi (`mem_out/`,
`ab_out/`, `cen2_out/`, `split_out/`).

## 1. Memory on large POP problems

`mplcp` queues the children of a region with a shared pointer to the parent's tableau (pivoted when popped). In
breadth-first order a parent stays alive until its last child is popped, so the live parents grow with the frontier.
At n = 77 rows a tableau is 53,592 bytes.

pop_002 (321,026 regions, 7 parameters, n = 77), one solve per variant:

| variant | solve s | peak live parent tableaux | their MB | peak RSS MB |
|---|---:|---:|---:|---:|
| base (breadth first, all lazy) | 88.7 | 36,437 | 1,862 | 3,151 |
| rebuild always (budget 1 byte) | 93.8 (+5.7%) | 0 | 0 | 1,302 |
| budget 200 MB | 93.3 (+5.2%) | 3,913 | 200 | 1,503 |
| newest region first (LIFO), lazy | cancelled | | | |
| LIFO + rebuild always | 98.3 (+11%) | 0 | 0 | 3,251 |

* Dropping every tableau takes RSS from 3.15 GB to 1.30 GB (-59%) for +5 s on 321,026 regions, about 16 us per region
  (one rebuild from the active set at n = 77 against one pivot of a shared copy). The remaining 1.3 GB is the regions
  themselves (about 4.1 KB per region), the queue and the `Seen` map, not tableaux.
* LIFO order raises RSS (3.25 GB with rebuild) and changes the region count by 2 on masses4_N3, so it is rejected.
* Below the budget nothing changes except the byte accounting. On 58 problems (the 49 hash problems and nine large ones)
  the region hash, facet rows and sorted-active-set hash of `e1` are identical to `d2`
  (`bench/results/` gate: `region_hash` prints `sets`, a hash of the sorted active sets only).
* Paired timing, `e1` against `d2`, 10 problems from 2 to 16 parameters, 3 alternations: geometric mean of medians 0.998,
  A/A 0.991; every problem within 0.975 to 1.017.
* **Default budget 256 MiB** (`mplcp::set_tab_budget`, `MPLCP_TAB_BUDGET` in `region_hash`; 0 = none). Children of a
  parent that would exceed it are queued without a tableau and rebuild it from their active set when popped.
* Rebuild safety check: with budget 1 byte (every child rebuilds) the region count and sorted active sets stay the same on 55 of 58 problems and
  differ by 1 to 4 regions (slivers of Chebyshev radius around 1e-7) on three degenerate masses problems
  (`masses3_N5` 10,437 to 10,436, `masses4_N4` 16,050 to 16,046, `masses5_N3` 5,060 to 5,057). `splitcheck` on
  `masses3_N5` and `masses4_N4` with budget 1: 4,600 and 2,864 feasible samples, 0 overlaps, 0 gaps. This only happens
  beyond the budget (problems of several GB), where the alternative is running out of memory.
* The sets hash is not a valid gate for a rebuild: a rebuilt tableau gives rows that differ in the last digits, so the
  1e-9 rounded rows hash differs on 20 problems, 17 of them with identical region counts and active sets (`region_hash` sets column).

### Peaks with the budget

| problem | regions | solve s | peak RSS | live parents | note |
|---|---:|---:|---:|---:|---|
| pop_002 | 321,026 | 93.3 (budget 200 MB) | 1.50 GB | 3,913 | base 88.7 s, 3.15 GB |
| pop_033 (8 parameters, n = 80) | 860,655 | 296 | 5.73 GB | 6,990 (256 MB) | 12 GB request; base 9.4 GB (coordinator's earlier figure) |
| pop_036 | not finished | | killed at 14 GB after 11 min | | millions of regions |
| pop_095 | not finished | | killed at 14 GB after 19.5 min | | millions of regions |

pop_033 spends 6.7 KB per region (rows 19.2 per region of 9 doubles = 1.4 KB, the law, the multiplier law, active sets,
the `Seen` entry). pop_036 and pop_095 need more than 14 GB; the budget cannot help there because the tableaux are
already capped at 256 MB, the regions are the memory. I did not rerun them with a larger request. The per-region
comparison with `graph` (3.7 KB per region from the earlier 3.2 GB on pop_033) points at the stored region data, not at
the search; I did not measure which field.

## 2. Why parametric_daqp beats mplcp (census, one solve each, gate profile)

| problem | mplcp ms (regions) | neighbour search, of which LP | region build | pdaqp ms (regions) | pdaqp in tests / in `gen_cr_from_kkt` ms |
|---|---|---|---|---|---|
| masses5_N2 | 454 (370) | 58%, 29% | 41% | 134 (303) | 2 / 132 |
| masses3_N3 | 354 (955) | 58%, 17% | 41% | 149 (907) | 17 / 128 |
| masses4_N3 | 4205 (2891) | 71%, 30% | 29% | 938 (2291) | 59 / 857 |
| pop_007 | 11330 (12235) | 75%, 32% | 24% | 3176 (11738) | 460 / 2590 |
| rand_t12_x5_m20_s0 | 3906 (6143) | 45%, 28% | 54% | 2164 (6061) | 53 / 2097 |
| pop_005 | 5954 (10289) | 67%, 21% | 32% | 2851 (10273) | 1096 / 1607 |

(before the scale-free pivot test; with it, masses4_N3 is 2.3 s.) parametric_daqp spends 85% to 97% of its time in the
shared region builder, at the same cost per region as mplcp's own build (rand_t12 346 against 343 us); its own tests cost
0.3 to 1.7 us each. The gap is mplcp's neighbour search. A DAQP solve costs 1 to 3 us; an mplcp neighbour LP 3 to 32 us.
The "Build 8" bbox gate was never shipped (rays/LPTAU variants changed rows), so no gate misjudges masses5_N2: both solvers
run the same 2n box LPs per region, which is a shared cost.

### Extra regions are slivers, not splits (`examples/splitcheck.rs`)

mplcp returns 5% to 25% more regions than parametric_daqp on the degenerate problems. No region of either solver has the
same primal law (x = A theta + b, rounded to 1e-6) as a region sharing a facet with it (`same_law_as_neighbour` 0 on all
five), so these are not lexicographic splits of one critical region that could be merged. The regions one solver has and
the other lacks are slivers:

| problem | mplcp regions | pdaqp regions | mplcp laws pdaqp lacks (radius median, max) | pdaqp laws mplcp lacks |
|---|---:|---:|---|---:|
| masses5_N2 | 370 | 303 | 97 (2.7e-7, 1.1e-5) | 30 |
| masses3_N3 | 955 | 907 | 151 (4.6e-7, 1.1e-4) | 103 |
| masses4_N3 | 2889 | 2292 | 933 (2.2e-7, 4.9e-3) | 336 |
| masses3_N4 | 3642 | 3431 | 808 (4.8e-7, 2.8e-3) | 597 |
| pop_007 | 12235 | 11738 | 1068 (3.8e-4, 2.0) | 513 |

Overlap and gap sampling (20,000 samples, 2,800 to 5,500 feasible): 0 overlaps and 0 gaps for both solvers on all five.
Both solvers drop or keep slivers differently; the counts are not a structural handicap of lexicographic splitting.
Sampling cannot see slivers of radius 1e-7, so "overlap-free" is at the sampling resolution.

## 3. DAQP as an emptiness filter for the sub-facet tests

Site: the Chebyshev LP of the Case-2 branch test (`pivot_facet`) and of the rank-1 sub-facet test, for polytopes
`{x : e x <= f}`. `daqp_certainly_empty` runs the in-crate DAQP kernel on the unit-normalised rows and accepts an
**empty** verdict only with a checked Farkas certificate. The multipliers are clamped to `max(p, 0)` and the residual
`r = sum p_i a_i` and gap `g = sum p_i b_i` are recomputed from the clamped vector; the polytope is empty when
`g + |r|_1 R < -1e-10 sum p_i`. `R` is the Euclidean radius of the parameter box (`theta_bbox`, finite LPs), because the
facet coordinates satisfy `|x|_inf <= |x|_2 <= |theta|_2`; when the box is unbounded or unavailable no certificate is
accepted. (The first version used a fixed R = 1e3 and charged negative multipliers as `neg * R`; Lean round 11,
Finding 17, showed that is not sound in principle: 5 random problems, `rand_t8_x5_m20_s1`, `rand_t8_x6_m18_s0`,
`rand_t8_x6_m24_s0`, `rand_t10_x6_m18_s0`, `rand_t10_x6_m24_s0`, leave a parameter unbounded above. Fixed in 23300a2.)
Hashes after the fix: identical to the build before it on 58 of 58 problems, and identical with DAQP on and off on those
5 unbounded-box problems (where no certificate is accepted). The certified share is unchanged: masses3_N4 84%, masses4_N3
74%, pop_005 88%. Everything else, including every nonempty answer, goes to the LP unchanged.

What did not work first, and why the shipped form is narrower:
* DAQP deciding "nonempty" on rows shifted by 2e-7 (witness = minimum-norm point as hint): regions differed by 1 to 11
  on the degenerate masses problems and pop_001 got 28% slower (the minimum-norm point is a poor warm start for the
  region-build LP).
* DAQP deciding "empty" from `Status::Infeasible` alone: wrong on thin polytopes. On masses4_N3 it declared 10 polytopes
  infeasible that contain a ball of radius up to 1e-6 (nearly parallel rows pass the kernel's singular test), losing one
  region. The certificate check removes this: in the shipped form hashes and sets are identical to the LP-only build on
  all 58 problems, in both trees. (The same defect probably costs parametric_daqp slivers; not checked.)
* The kernel's `certificate()` accessor is new; the kernel is otherwise unchanged.
* The filter pauses for 8,192 tests when fewer than 60% of 512 calls are certified (thin or nonempty polytopes make
  the DAQP solve pure overhead): without the pause rand_t12 was 1.13x slower, masses5_N2 1.03x.

Share of tests answered by DAQP (census, stacked on 7457656): masses3_N3 83%, masses3_N4 84%, masses4_N3 75%, pop_005 88%,
pop_007 77%, pop_017 78%, masses5_N2 55%, rand_t12 25% (paused), rand_t10 2% (paused), rand_t8 12% (paused). Cost per
answered-or-not test 2 to 7 us (rand_t12 22 us, masses5_N2 11 us, where the filter pauses).

### Paired timing

Alone (merged with c345cdc, `m8` against `m3b`, 14 problems, 5 alternations): geometric mean of medians 0.977, A/A 1.001.

Stacked on main 7457656 (`s1` against `s1b`, same problems): geometric mean 0.985, A/A 0.998:

| problem | `s1b` ms | `s1`/`s1b` | A/A |
|---|---:|---:|---:|
| pop_005 | 4511 | 0.894 | 0.999 |
| masses3_N4 | 1578 | 0.942 | 0.999 |
| masses3_N3 | 219 | 0.990 | 0.999 |
| masses4_N3 | 2320 | 0.997 | 0.998 |
| masses5_N2 | 290 | 1.003 | 0.998 |
| rand_t12_x5_m20_s0 | 3662 | 1.003 | 1.001 |
| rand_t10_x5_m20_s0 | 1667 | 0.992 | 0.997 |
| pop_017 | 494 | 1.003 | 1.003 |
| rand_t8_x5_m20_s0 | 640 | 1.004 | 1.004 |
| rand_t6_x5_m20_s0, rand_t4_x5_m20_s0, sys3x2_N5, masses2_N6, di_N6 | | 0.987 to 1.000 | 0.989 to 1.003 |

Alone the filter gave 0.87 on pop_005, 0.91 on masses3_N4, 0.955 on masses4_N3 and ~1.00 elsewhere. Stacked on the scale-free
pivot test, the masses neighbour LPs per region fall (masses4_N3 36.9 to 14.7 at the time of the diagnosis) and the filter
only helps where empty sub-facet tests still dominate: pop_005 -10.6%, masses3_N4 -5.8%, masses4_N3 -0.3%. The
prediction of 15% to 27% on the masses problems was too high for two reasons: the certificate check hands 16% of
the DAQP infeasible answers back to the LP on masses5_N2 (554 of 3,532), and half of the sub-facet tests on masses5_N2 are nonempty (the LP is needed
for those).

## 4. What is left in the neighbour search (stacked on 7457656)

| problem | solve ms (census) | neighbour search | region build | tableau update | rest | Case 2 share of `adjacent` time |
|---|---:|---:|---:|---:|---:|---:|
| masses5_N2 | 326 | 43% | 57% | 0% | 0% | 95% |
| masses3_N3 | 242 | 41% | 58% | 0% | 1% | 95% |
| masses4_N3 | 2495 | 51% | 49% | 0% | 0% | 98% |
| masses3_N4 | 1718 | 55% | 45% | 0% | 0% | 97% |
| pop_005 | 4403 | 62% | 37% | 0% | 1% | 97% |
| pop_007 | 10823 | 74% | 26% | 0% | 0% | 99% |
| rand_t12_x5_m20_s0 | 3917 | 45% | 54% | 0% | 0% | 94% |

Tableau updates are not a cost (0%). The neighbour search is still 41% to 74% of the time and 94% to 99% of it is the
Case-2 path (22 `adjacent` calls per region on masses4_N3, 55% of them Case 2, 34 us per Case-2 call). The existing
timers do not split a Case-2 call into pivots, tableau copies, row tests and `Seen` lookups; that needs timers inside
`pivot_facet`/`walk_one`, which I did not add.

## 5. Per-region memory: where the bytes are, and the 512 MiB budget over parents and owned tableaux

**Measurement caveat for section 1.** `region_hash` sorts and hashes a copy of every facet row after the solve. On the
largest problems that copy raises the peak, so the "peak RSS" of section 1 (pop_033: 5.7 GB) overstated the solve. The
numbers below come from `examples/region_bytes.rs`, which reads `VmHWM` right after `solve_mpqp` (no copy), and from the
unbudgeted `region_hash` runs, where the queue and the parents dominate and the hash copy does not matter.

### What a stored region holds (capacity bytes per region, end of solve)

| field | pop_033 (nt 8, nx 8, n 80) | pop_005 (nt 7, nx 6) |
|---|---:|---:|
| primal law A, b | 512, 64 | 336, 48 |
| dual law C, d | 431, 54 | 269, 38 |
| facet rows E, rhs f | 1231, 154 | 889, 127 |
| active_set, omega, lambda, regular sets | 54, 20, 36, 257 | 38, 41, 27, 169 |
| `CriticalRegion` struct in the `Vec` (400 B each, `Vec` doubling slack included) | 487 | 637 |
| total by capacity | 3,301 | 2,620 |

pop_002 totals 2,649 B per region by capacity.

* `graph` stores the same `CriticalRegion` (identical field bytes on pop_005, pop_017 and masses3_N4 for both algorithms),
  so the data of a region is not what separates the two solvers. All fields are part of the output (`solution_io`,
  `lowrank` and `mplp_lex` read `omega_set`, `lambda_set`, `regular_set`), so nothing can be dropped without changing the
  output type. Slack in the vectors is small (`omega_set` 41 against 29 bytes, `regular_set` 169 against 142), the
  doubling slack of the region `Vec` is 100 to 250 B per region, and `malloc_trim` after the solve returns 8% to 20% of
  the resident size (pop_002 1,067 to 848 MB, pop_033 3,097 to 2,897 MB): free memory held between the long-lived regions.
* The difference is in the **search structures**, and it is large without the budget:

| structure | pop_033 unbudgeted | pop_033, 512 MiB | pop_002 unbudgeted | pop_002, 512 MiB |
|---|---:|---:|---:|---:|
| live parent tableaux | 100,611 = 3,684 MB | 11,804 = 432 MB | 36,392 = 1,860 MB | 10,017 = 512 MB |
| queued items (centre, owned tableaux), peak | 3,213 MB (281,167 items) | 354 MB | not recorded | 403 MB |
| `Seen` map | 80 MB (861,346 entries) | 80 MB | 34 MB | 34 MB |
| peak resident size | 9,135 MB | 3,521 MB | 3,143 MB | 1,377 MB |
| solve time | 299.7 s | 314.0 s (+4.8%) | 87.4 s | 87.4 s |

* The queued items were the missing half of the budget: a Case-2 child arrives with its own tableau (`TabSrc::Ready`),
  which the 256 MiB budget of section 1 did not count. On pop_033 they held 3.2 GB of the 9.1 GB. They now count against
  the same budget (`ready_bytes`); a child whose owned tableau does not fit is queued without it and rebuilds it when
  popped. The default budget is now 512 MiB for parents and owned tableaux together.
* 256 MiB for both costs more time for no memory: pop_033 355 s (+18%) at the same peak as 314 s for 512 MiB; the peak
  is the stored regions, not the search.
* **pop_033: 9.1 GB to 3.5 GB (-61%) at +4.8% time, against `graph`'s 3.2 GB.** pop_002: 3.14 GB to 1.38 GB (-56%) at
  +0.0% time, against `graph`'s 0.93 GB. The sorted active sets are identical to the unbudgeted run on both (pop_002
  `cb70fefe84cd4bd8`, pop_033 `1adc7e3c8d0b4eb3`).
* What remains above `graph` is the 512 MiB of tableaux and a queue of a few hundred MB; lowering the budget trades time
  for it. Cutting the stored bytes needs a change of `CriticalRegion` (one allocation per region instead of eleven would
  save about 240 B of 3.3 KB, 7%), which is not a local change.

### The very large problems

| problem | solver | outcome |
|---|---|---|
| pop_036 | `mplcp` before the budget | killed at 32.7 GB after 31 min |
| pop_036 | `graph` | killed at 26.6 GB after 35 min |
| pop_036 | `mplcp` now, 48 GB request | **solved: 6,655,636 regions, 3,299 s, 123.7 M facet rows**; `region_hash` peak 35.9 GB (includes the hash copy of all rows, about 9 GB); 6,666,880 `Seen` entries (669 MB), peak queue 2.65 M items (1.0 GB) |
| pop_095 | `mplcp` before the budget | killed at 32.7 GB after 41 min |
| pop_095 | `graph` | not finished: 10,500 s timeout at 25.1 GB |
| pop_095 | `mplcp` now, 48 GB request | **solved: 5,012,492 regions, 40 min wall (job), peak resident 18.3 GB (`region_bytes`, no hash copy), 3,447 B of stored fields per region, 3.6 KB resident growth per region** |

### Correctness caveat of the rebuild path

Dropping a tableau and rebuilding it from the active set is exact only in exact arithmetic. With `MPLCP_TAB_BUDGET=1000000`
(every child rebuilt) the sorted active sets differ from the unbudgeted run on `masses4_N4` and `masses5_N3` (1 to 3
sliver regions) and on **`pop_001`: 3,529 regions become 4,710, with 9,009 of 13,497 sampled points inside two regions**
(2,354 regions share their law with another). Still present after merging the `tableau_for` fix (ddf0aa8). Bisection on
pop_001 with `MPLCP_TAB_BUDGET=1000000`: dropping only the owned (Case-2) tableaux gives 3,529 regions; dropping only the
lazily pivoted children gives 4,710. Rebuilding those children with `tableau_of` when they are popped (`TabSrc::None`)
gives 4,710, but rebuilding the same tableau when they are queued (`TabSrc::Ready(tableau_of(...))`) gives 3,529, as does
replacing the pivoted tableau by the rebuilt one inside `resolve` (rows and columns of the two tableaux are in the same order
in all 2,389 pairs compared), and `REFACTOR = 1` (3,529). It is not the entry-facet skip and not the depth counter, and I did not find the cause. The budget only triggers beyond 512 MiB of tableaux, which no problem
below pop_001 size reaches, and on pop_002 and pop_033 (past the budget) the sets equal the unbudgeted run, but a large
twin-row problem could be affected.

### Gates for this change

* Region hashes and sorted-set hashes, branch (u1 = 3ca21bf) against main 9a9664a: identical on 58 of 58 problems.
  Default budget against no budget (`MPLCP_TAB_BUDGET=0`) on the merged tree, 58 problems: identical.
* Paired serial timing, u1 against main, 14 problems at 2 to 16 parameters, 5 alternations, A/A geometric mean 1.0005:
  geometric mean 0.9985 (per problem 0.986 to 1.013): the accounting costs nothing below the budget.
