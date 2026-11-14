# Benchmark: RPOPT vs PPOPT on explicit MPC

## Problems (`mpc_problems.py`)

| name | system | horizon | formulation | x | θ | constraints |
|---|---|---|---|---|---|---|
| `di_doc_N3` | double integrator, PPOPT doc/mpc.rst example | 3 | states kept, dynamics as equalities | 9 | 2 | 24 |
| `di_N4` ... `di_N10` | double integrator, Q = I, R = 1, LQR terminal cost, \|x\| <= 4, \|u\| <= 1 | 4 to 10 | condensed | N | 2 | 6N |
| `masses2_N4`, `masses2_N6` | two masses and springs, Ts = 0.5, one force input | 4, 6 | condensed | N | 4 | 10N |
| `sys3x2_N3`, `sys3x2_N5` | 3 states, 2 inputs, open-loop unstable | 3, 5 | condensed | 2N | 3 | 10N |

θ = x0 in every case. Both implementations read the same JSON matrices.

## How the comparison is made

Both sides are measured on the same machine and the same JSON problems:

- **PPOPT**: the current GitHub source (`TAMUparametric/PPOPT`, branch `main`) in a virtual environment with
  its default backends here (Gurobi 12 for LPs through gurobipy, quadprog for QPs, numpy 1.26).
  `bench_ppopt.py` runs every problem and algorithm in a fresh process, times `solve_mpqp` only (the
  program is built outside the timer), and records the region count. Each run has a time limit (300 s for
  the full sweep; the two combinatorial runs that hit it were rerun with a 4 h limit). PPOPT's numba point
  location is timed on the combinatorial solution, or on the geometric one where combinatorial did not finish.
- **Rust**: `ppopt_bench` reports the median of repeated solves (at least 7 runs and 0.3 s, a single run
  above 2 s), one thread for the serial algorithms and all 16 cores (rayon) for the parallel ones.
- `compare.py` divides the two. Serial PPOPT algorithms are compared with the same Rust algorithm on one
  thread, PPOPT's `combinatorial_parallel` (pathos, one process per core) with Rust's
  `combinatorial_parallel`. Where PPOPT has two measurements the faster one is used.

The Rust port also counts the operations PPOPT performs for the same algorithm (every `solve_lp`,
`solve_qp` and `matrix_rank` call at PPOPT's call sites, `src/stats.rs`); those counts are reported as
"PPOPT-equivalent" and were the basis of an earlier lower-bound estimate (`lower_bound.py`,
`results/speedup_lower_bound.md`). That estimate priced each LP at a standalone gurobipy call, which turned
out to cost more than a call inside PPOPT (for example 32.4 s estimated vs 24.3 s measured for
masses2_N6/geometric), so it is not a lower bound and is superseded by the measured comparison.

```sh
python mpc_problems.py problems                                  # problems (numpy, scipy)
cargo build --release
../target/release/ppopt_bench problems results/rust.json --queries 100000
PPOPT_SRC=/path/to/PPOPT/src python bench_ppopt.py problems results/ppopt.json 300
python compare.py results/ppopt.json results/rust.json results/speedup.md
python thread_scaling.py ../target/release/ppopt_bench results/rust.json results/thread_scaling.md
```

`bench_ppopt.py problems out.json <timeout> <problems> <algorithms>` reruns a subset (comma-separated).

**Time to coverage.** For exact mp-QP solvers the quality of a (partial) solution is its completeness:
which share of the feasible parameter set its critical regions cover (the final partitions of both
implementations are complete, overlap-free and identical). `coverage_ppopt.py` logs, for every serial PPOPT
run, the time each region was built (PPOPT is not modified: `gen_cr_from_active_set` is wrapped at run time
and every event is flushed, so a run stopped at its time limit keeps its log). `examples/coverage.rs` builds
the same log for Rust, draws 100 000 uniform samples of Θ, keeps those inside the final partition, and reports
the time at which 50%, 90%, 99% and 100% of them lie in regions built so far, the time at which every final
region has been built, and the total solve time. `coverage_table.py` runs it on each PPOPT log and writes
`results/coverage.md`.

```sh
PPOPT_SRC=/path/to/PPOPT/src python coverage_ppopt.py problems results/coverage_ppopt 600 "" "" 3
cargo build --release --example coverage
python coverage_table.py results/coverage_ppopt results/coverage.json results/coverage.md 100000
```

`ppopt_bench` also checks that all 9 algorithms find the same active sets, samples 2000 states against
direct QP solves (gaps, overlaps, wrong laws), and times every point location method on 100 000 random
states.

## Results

Apple M4 Max (12 performance + 4 efficiency cores), rustc 1.98, `-C target-cpu=native`, fat LTO, mimalloc.
Full tables: `results/speedup.md` (measured, `compare.py`), `results/coverage.md` (time to coverage),
`results/thread_scaling.md`, `results/point_location.md`.

Correctness on every problem: all 9 algorithms find the same active sets, 2000 random states agree with
direct QP solves (no gaps, overlaps or wrong laws), every point location method agrees with sequential
search on 100 000 states, every thread count reproduces the serial region set, and PPOPT's final active
sets equal the Rust ones.

### Speedup over PPOPT (measured)

Same algorithm, single thread, clean sweep (PPOPT runs one at a time, 300 s limit): 32 of 32 completed
runs at 200x or more (min 261x, masses2_N4/geometric; median 728x; max 2,224x). Combinatorial runs that hit
the 300 s limit: di_N8 and masses2_N6 finished on a busy machine (other PPOPT runs and Rust builds in
parallel, so PPOPT's time is inflated) at about 1,400x and 1,000x; di_N10 and sys3x2_N5 are being rerun
with a 4 h limit (results to be merged into `results/ppopt.json`).

Parallel against parallel (PPOPT `combinatorial_parallel`, pathos processes, vs Rust, 16 threads): 671x to
2,837x where PPOPT finished; di_N10 and sys3x2_N5 are lower bounds (>197x, >199x).

Time to coverage (`results/coverage.md`, both sides measured on the busy machine, so tentative): time to
cover 99% or 100% of the feasible parameter set at least 275x faster on every run that reached it; one
run below 200x at 90% coverage (masses2_N4 combinatorial graph, 179x, a visit-order effect).

### Multi-threading (16 threads vs 1)

| problem | algorithm | serial (1 thread, serial code) | 1 thr | 2 thr | 4 thr | 8 thr | 12 thr | 16 thr | speedup 16 vs 1 | speedup 16 vs serial | regions ok |
|---|---|---|---|---|---|---|---|---|---|---|---|
| di_N8 | combinatorial_parallel | 957.6 ms | 966.9 | 504.7 | 263.6 | 146.1 | 107.7 | 102.0 | 9.5x | 9.4x | yes |
| di_N8 | graph_parallel | 0.3 ms | 0.4 | 0.3 | 0.3 | 0.5 | 0.8 | 0.7 | 0.6x | 0.5x | yes |
| di_N8 | combinatorial_graph_parallel | 0.6 ms | 0.6 | 0.5 | 0.3 | 0.5 | 0.8 | 0.7 | 1.0x | 0.9x | yes |
| di_N8 | geometric_parallel | 0.1 ms | 0.2 | 0.2 | 0.2 | 0.3 | 0.5 | 0.4 | 0.5x | 0.3x | yes |
| masses2_N4 | combinatorial_parallel | 29.6 ms | 29.4 | 15.4 | 8.1 | 5.3 | 4.6 | 4.8 | 6.2x | 6.2x | yes |
| masses2_N4 | graph_parallel | 12.8 ms | 13.5 | 7.4 | 4.5 | 3.4 | 3.5 | 3.5 | 3.9x | 3.7x | yes |
| masses2_N4 | combinatorial_graph_parallel | 22.6 ms | 23.0 | 12.4 | 7.2 | 4.9 | 5.0 | 4.9 | 4.7x | 4.6x | yes |
| masses2_N4 | geometric_parallel | 15.6 ms | 15.5 | 8.3 | 4.8 | 3.6 | 3.9 | 3.5 | 4.5x | 4.5x | yes |
| masses2_N6 | combinatorial_parallel | 936.3 ms | 935.8 | 483.9 | 248.9 | 132.3 | 97.1 | 89.9 | 10.4x | 10.4x | yes |
| masses2_N6 | graph_parallel | 75.2 ms | 94.3 | 50.4 | 29.0 | 19.1 | 16.9 | 16.9 | 5.6x | 4.4x | yes |
| masses2_N6 | combinatorial_graph_parallel | 249.8 ms | 253.0 | 137.0 | 76.3 | 47.5 | 39.3 | 38.2 | 6.6x | 6.5x | yes |
| masses2_N6 | geometric_parallel | 62.8 ms | 65.3 | 34.4 | 18.8 | 11.7 | 10.4 | 10.0 | 6.5x | 6.3x | yes |
| sys3x2_N3 | combinatorial_parallel | 52.3 ms | 51.6 | 27.2 | 14.5 | 10.5 | 9.2 | 9.4 | 5.5x | 5.5x | yes |
| sys3x2_N3 | graph_parallel | 4.9 ms | 5.8 | 3.4 | 2.2 | 2.3 | 2.9 | 2.7 | 2.1x | 1.8x | yes |
| sys3x2_N3 | combinatorial_graph_parallel | 11.1 ms | 11.4 | 6.2 | 3.7 | 3.0 | 3.7 | 3.5 | 3.2x | 3.1x | yes |
| sys3x2_N3 | geometric_parallel | 4.7 ms | 4.8 | 2.9 | 1.8 | 2.0 | 2.8 | 2.2 | 2.2x | 2.1x | yes |
| sys3x2_N5 | combinatorial_parallel | 23580.0 ms | 23721.8 | 12491.1 | 6357.6 | 3386.4 | 2485.0 | 2309.9 | 10.3x | 10.2x | yes |
| sys3x2_N5 | graph_parallel | 143.4 ms | 351.6 | 191.1 | 105.5 | 68.7 | 57.4 | 55.7 | 6.3x | 2.6x | yes |
| sys3x2_N5 | combinatorial_graph_parallel | 629.1 ms | 646.4 | 357.9 | 209.5 | 137.8 | 113.8 | 110.2 | 5.9x | 5.7x | yes |
| sys3x2_N5 | geometric_parallel | 37.6 ms | 39.9 | 22.0 | 11.9 | 9.2 | 9.8 | 8.3 | 4.8x | 4.5x | yes |

Times in ms. "regions ok": same region count as the serial algorithm and no sampled gaps, overlaps or wrong laws at every thread count.

### Point location (ns per query unless noted)

Rust over 100 000 random parameters, PPOPT's numba `PointLocation` over 20 000; every Rust method agrees with sequential search (0 mismatches). Bold: fastest; hash table: largest grid.

| problem | regions | PPOPT numba (us) | sequential (PPOPT algorithm) | sequential, early exit | BST | multiway (order 7) | linear machine | descriptor | hash table | YANN | speedup, same algorithm | speedup, fastest |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| di_doc_N3 | 7 | 5.3 | 54 | 29 | 34 | 29 | 28 | 36 | **22** | 52 | 100x | 241x |
| di_N4 | 9 | 5.8 | 67 | 32 | 35 | 32 | 30 | 39 | **23** | 65 | 86x | 257x |
| di_N6 | 11 | 6.1 | 79 | 35 | 42 | 37 | 32 | 42 | **23** | 79 | 77x | 263x |
| di_N8 | 11 | 5.6 | 76 | 36 | 43 | 37 | 32 | 42 | **22** | 77 | 74x | 257x |
| di_N10 | 11 | 5.7 | 75 | 35 | 42 | 37 | 32 | 42 | **23** | 79 | 76x | 253x |
| masses2_N4 | 307 | 23.2 | 4,893 | 1,297 | 189 | 136 | none | 1,076 | **67** | 4,925 | 5x | 348x |
| masses2_N6 | 901 | 63.9 | 14,113 | 5,351 | 255 | 178 | none | 4,296 | **83** | 13,989 | 5x | 770x |
| sys3x2_N3 | 211 | 17.9 | 1,988 | 808 | 126 | 99 | none | 679 | **39** | 1,985 | 9x | 457x |
| sys3x2_N5 | 973 | 68.5 | 9,286 | 4,488 | 163 | 119 | none | 3,559 | **51** | 9,212 | 7x | 1,345x |

No exact linear machine exists for a partition that is not facet-to-facet (an argmax of affine functions is
always face-to-face), which is the case for the four 3-D/4-D problems.

## Optimization log (single thread, `masses2_N6` unless noted, us per PPOPT-equivalent call)

Measured with `examples/profile_solve.rs` (`--profile profiling`) and macOS `sample`. Every change was
checked against the full benchmark (same active sets for all 9 algorithms, 2000 direct-QP checks).

| change | effect |
|---|---|
| start (dense dual simplex, refactor-verified) | geometric 3.28, graph 3.70, combinatorial 6.87 per LP, combinatorial graph 5.52 |
| allocation-free LP core (thread-local scratch), feasibility/point-only entry points, no env lookup | geometric 2.26 |
| facet stepping scans found regions only before building a region | geometric 2.41 -> 2.32 (QP count corrected) |
| phase 1 stops when the artificials reach zero; no artificial drive-out (blocked in phase 2) | geometric 1.87, graph 2.42 |
| `region_rows` writes in place (no per-row vector) | graph 2.14, combinatorial graph 3.61 |
| region row tests warm-started at the Chebyshev center | phase-2 pivots 305k -> 263k (small) |
| bounding-box / projection prefilters for region row tests | geometric 1.29, LPs 65.9k -> 30.6k |
| Chebyshev radius cached on the region (graph `is_full_dimension`) | graph 1.72 |
| combinatorial feasibility warm-started from the parent's point | pivots 3.3M -> 2.46M, time -4% |
| feasibility LP before the Chebyshev LP in region construction | combinatorial 2.65 -> 2.10 |
| law, rows and LP shared between optimality test and construction | small |
| Fx hashing, stack/thread-local rank buffers | graph 1.62, combinatorial graph 3.20 |
| sparse Farkas certificate check (n + 1 nonzeros) | graph 1.58, combinatorial graph 3.00 |
| primal law x(θ) computed lazily | combinatorial graph 2.92 |
| symmetric M streamed by rows, precomputed -(AQ^{-1}H + F) | combinatorial graph 2.65 |
| u128 active-set keys in combinatorial graph | 2.30 (sys3x2_N5: 2.61) |
| rank hints: subsets of full-rank sets, incremental span test for supersets | 2.18 |
| u128 pruning-list checks, const-generic n_θ kernel | sys3x2_N5 graph 1.85 |
| contiguous complete-pivoting rank test | sys3x2_N5 combinatorial graph 2.24 |
| cached Hessian factor + QP workspace + transposed J + cached equality phase | di_doc_N3 graph 450 -> 230 us |
| Cholesky of the Schur complement certifies full rank (skip rank test) | sys3x2_N5 combinatorial graph 2.12 |
| per-thread sharded counters, pivot counter removed | 16-thread speedup 4.8x -> 8.8x (combinatorial) |
| mimalloc global allocator in the binaries | sys3x2_N5 combinatorial graph 1.71, di_doc_N3 graph 143 us |

Combinatorial algorithm (wall time per solve, single thread; measured while two PPOPT runs shared the
machine, so absolute values are a few percent noisy):

| change | di_N8 | di_N10 | sys3x2_N5 |
|---|---|---|---|
| start of this round | 936 ms | 28.8 s | 21.7 s |
| witnesses from the parent's point only | 842 ms (39% of feasibility LPs skipped) | | |
| candidates grouped by parent, sibling LP vertices shared as witnesses | 645 ms (77% skipped) | 21.6 s (80% skipped) | 18.2 s (56% skipped) |
| pruning list indexed by largest element, 64-bit "rest" masks | 602 ms | 16.9 s | |
| region test on the λ and parameter-box rows first | 386 ms | 10.7 s | 11.3 s |
| KKT law first, its Cholesky certifies rank (rank test skipped) | 353 ms | | 10.9 s |
| shifted right hand side shared by the LPs of a group | 343 ms | | |

The λ-rows-first test in the other enumerations (same PPOPT-equivalent counts before and after):
combinatorial graph sys3x2_N5 599 -> 347 ms, masses2_N6 221 -> 154 ms, masses2_N4 22.3 -> 20.2 ms; graph
sys3x2_N5 131 -> 105 ms, masses2 problems unchanged (within noise).

Rejected:

| idea | result |
|---|---|
| row-subset feasibility LP (2n rows closest to binding at the start point, verify the rest) | 24% to 49% of the subset solutions violated an omitted row and needed the full LP: 10% to 20% slower |
| more witnesses per set (32 instead of 8) | no change; 4 slightly worse |
| equality elimination for the children of a set (reduced inequality-only LP) | slower: building the reduced system costs more than the LPs it serves |
| revised-simplex kernel for n <= 16 with equality rows | about 5% slower than the dense tableau |
| dropping region rows that cannot bind inside the parameter box before the nonemptiness LP | slower |
| rank hints for subsets of rank-deficient sets from a tracked Gram-Schmidt dependency | slower (828 -> 1020 ms on sys3x2_N5): most of those subsets were already explored |
| LU fallback before the rank test for failed Cholesky factorizations | wasted work on rank-deficient sets; LU now only after the rank test passes |
