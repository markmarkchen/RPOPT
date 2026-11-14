<div align="center">

# RPOPT

**Fast, exact multiparametric programming in Rust**

A rewrite of [PPOPT](https://github.com/TAMUparametric/PPOPT) for explicit model predictive control:
mpLP, mpQP, mpMILP and mpMIQP solvers, plus fast exact point location.

![license](https://img.shields.io/badge/license-MIT-blue)
![rust](https://img.shields.io/badge/rust-2024%20edition-orange)
![python](https://img.shields.io/badge/python-PPOPT--compatible%20bindings-3776AB)
![tests](https://img.shields.io/badge/tests-116%20Rust%20%2B%2072%20Python-brightgreen)

</div>

## Speedup over PPOPT

| task (explicit MPC benchmarks) | PPOPT | RPOPT | speedup |
|---|---:|---:|---:|
| **mpQP, every completed run**, same algorithm, 1 thread (32 runs) | | | **389x to 2,703x**<br>median 1,020x |
| mpQP, PPOPT's fastest algorithm vs RPOPT's fastest, per problem | | | **755x to 4,167x** |
| combinatorial, di_N10 and sys3x2_N5 | > 4 h (unfinished) | 6.0 s, 6.5 s | **> 2,200x** |
| geometric, sys3x2_N5 (973 regions) | 18.6 s | 17.5 ms | **1,062x** |
| **mpLP** (degenerate MPC), RPOPT's fastest vs PPOPT's fastest gap-free algorithm (6 problems) | 0.14 to 84 s | 0.17 to 49 ms | **836x to 18,028x** |
| mpLP where PPOPT gives no gap-free answer within 120 s (7 problems) | > 120 s | 0.4 to 19 ms | **> 6,400x** |
| point location, per query | 5 to 68 µs | 9 to 32 ns | **590x to 3,600x** |

<sub>Quiet-machine runs on one Apple M4 Max. PPOPT is the GitHub `main` source with Gurobi 12 and quadprog. mpQP: the
12 MPC problems in `bench/problems`. mpLP: infinity-norm and 1-norm MPC (`bench/problems_lp`), checked at 1,000
random parameters per problem. PPOPT's geometric leaves 50% to 77% of the feasible parameter set uncovered on five of
them, so mpLP rows compare against PPOPT's fastest algorithm with no sampled gap. Point location uses the
storage-limited hash table. RPOPT also runs 1.5x to 2.9x faster than `pdaqp`, the authors' own implementation of
Arnström and Axehill (2024). Details: [Performance](#performance) and [bench/README.md](bench/README.md).</sub>

## Highlights

- **Exact and verified.** Each benchmark run checks that all algorithms agree. It also samples 2000 parameters
  against direct QP solves, looking for gaps, overlaps and wrong control laws.
- **All of PPOPT's algorithms, plus eleven from the literature:**
  - Arnström-Axehill with DAQP, as published;
  - Ahmadi-Moshkenani;
  - mp-LCP pivoting, and mp-LCP enumeration;
  - lexicographic mpLP;
  - active-set dynamic programming;
  - randomized MPR;
  - approximate mpQP;
  - mpMILP comparison (Dua-Pistikopoulos), and mpMILP branch and bound;
  - exact mpMIQP comparison (Oberdieck-Pistikopoulos).
- **Robust on degenerate problems.** RPOPT closes holes that facet stepping (as implemented in PPOPT) leaves at
  degenerate points. On degenerate mpLPs, every RPOPT exact solver returns a gap-free partition; PPOPT's geometric
  and graph solvers leave 10% to 77% of the feasible parameter set uncovered on most of them.
- **Exact mixed-integer comparison.** mp-MIQP solutions with quadric region boundaries (Oberdieck and
  Pistikopoulos 2015), in addition to PPOPT's overlapping enumeration.
- **Fast online evaluation.** Point location takes 9 to 32 ns per query with a hash table built to a storage
  budget. Nine other exact locators are included, and all of them agree on every query, including points on
  region boundaries.
- **Drop-in Python.** PPOPT-compatible bindings: `rpopt.install_as_ppopt()` runs existing PPOPT scripts unchanged.
- **Deployable.** Solutions and point location indexes save to a bit-exact binary format and load in about 1 ms.

**Contents:** [Install](#install) · [Quick start](#quick-start) · [Algorithms](#algorithms) ·
[Point location](#point-location) · [Saving and loading](#saving-and-loading) · [Performance](#performance) ·
[Correctness](#correctness) · [How it is fast](#how-it-is-fast) · [Differences from PPOPT](#differences-from-ppopt) ·
[Project layout](#project-layout) · [References](#references) · [License](#license)

## Install

Rust 1.85 or newer (edition 2024):

```toml
[dependencies]
rpopt = { git = "https://github.com/markmarkchen/RPOPT" }
```

The release profile uses fat LTO and one codegen unit. `.cargo/config.toml` sets `target-cpu=native` for local
builds; remove it to build portable binaries. The `mimalloc` feature (on by default) uses mimalloc as the global
allocator in the binaries and examples.

Python bindings (PyO3, maturin, abi3, Python 3.9 or newer):

```bash
cd python
python3 -m venv .venv && source .venv/bin/activate
pip install maturin numpy pytest
maturin develop --release
```

## Quick start

Rust:

```rust
use rpopt::{solve_mpqp, Format, MpProgram, MpqpAlgorithm, Solution};
use rpopt::point_location::{HashTable, PartitionGeometry, PointLocator};
use std::sync::Arc;

// min 1/2 x'Qx + (c + H theta)'x  s.t.  A x <= b + F theta,  A_t theta <= b_t   (PPOPT's convention)
let prog = Arc::new(MpProgram::from_json_file("bench/problems/di_doc_N3.json".as_ref())?);
let sol = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
println!("{} critical regions", sol.critical_regions.len());

// online evaluation: build an exact point locator within a 4 MB budget
let geom = PartitionGeometry::new(&sol);
let table = HashTable::with_storage_limit(&geom, 4 << 20);
let theta = [1.5, -0.5];
let x = table.locate(&theta).map(|i| sol.critical_regions[i].evaluate(&theta));

// save the explicit solution and the index, load them in the controller
sol.save("controller.rpopt", Format::Binary)?;
table.save("controller.index")?;
let sol2 = Solution::load("controller.rpopt")?;
```

Python, with PPOPT's API:

```python
import rpopt
rpopt.install_as_ppopt()          # optional: makes `import ppopt...` resolve to RPOPT

from rpopt.mpqp_program import MPQP_Program
from rpopt.mp_solvers.solve_mpqp import solve_mpqp, mpqp_algorithm
from rpopt.point_location import HashTable

prog = MPQP_Program(A, b, c, H, Q, A_t, b_t, F)
sol = solve_mpqp(prog, mpqp_algorithm.geometric)    # the GIL is released while solving
ht = HashTable.with_storage_limit(sol, 4 << 20)
idx = ht.locate_many(thetas)                        # parallel, GIL-free
sol.save("controller.rpopt")
```

`cargo run --release --example explicit_mpc` solves the double integrator example of PPOPT's documentation and
writes an SVG plot of the partition.

## Algorithms

Each method below is either a `MpqpAlgorithm` variant, run with `solve_mpqp(&prog, alg)`, or a dedicated
function. Every exact method returns the same set of critical regions as PPOPT on non-degenerate problems.

### Exact mpQP / mpLP

| name | method | origin | notes |
|---|---|---|---|
| `combinatorial`, `combinatorial_parallel` | Gupta, Bhartiya, Nataraj 2011 | PPOPT | exhaustive over active sets; exponential in the horizon |
| `graph`, `graph_exp`, `graph_parallel` | Oberdieck et al. 2017, connected graph | PPOPT | `graph_exp` has no pruning list; on mpLPs, one-pivot swaps plus a facet-completion pass make the partition gap-free |
| `geometric`, `geometric_parallel` | facet stepping (Spjøtvold et al. 2006) | PPOPT | usually the fastest serial method; fixed for degenerate problems (see below) |
| `combinatorial_graph`, `combinatorial_graph_parallel` | Arnström, Axehill 2024, as simplified in PPOPT | PPOPT | breadth-first visit order |
| `parametric_daqp` | Arnström, Axehill 2024, as published | paper | least-distance transformation and DAQP subproblems; 1.5x to 2.9x faster than the authors' `pdaqp` |
| `ahmadi` | Ahmadi-Moshkenani, Johansen, Olaru 2018 | paper | handles LICQ and strict-complementarity violations (PPOPT ships an empty stub) |
| `mplcp`, `mplcp_parallel` | Jones, Morari 2006, mp-LCP pivoting | paper | the fastest serial mpQP solver on the benchmark; lexicographic perturbation: no gaps or overlaps at degenerate vertices; mpLPs are routed to `mplp_lex` |
| `mplcp_enum` | Herceg, Jones, Kvasnica, Morari 2015 | paper | basis enumeration for general M (mpQP and mpLP); exponential |
| `mplp_lex` | Jones, Kerrigan, Maciejowski 2007 | paper | lexicographic mpLP: unique, gap- and overlap-free partition for degenerate mpLPs; the fastest mpLP solver on the benchmark |
| `algorithms::dp::solve_mpc_dp` | Mitze, Mönnigmann 2020, 2022; Mitze, Kvasnica, Mönnigmann 2023 | paper | active-set dynamic programming over the MPC horizon; takes the MPC model (`mpc::MpcProblem`) |

### Approximate and supporting methods

| function | method | notes |
|---|---|---|
| `algorithms::mpr::solve_mpqp_mpr` | Falsone, Bianchi, Prandini 2026, randomized MPR | probabilistic coverage guarantee; returns only exact regions |
| `algorithms::approx::solve_mpqp_approx` | Bemporad, Filippi 2003 | value error within eps; fewer regions only at large eps |
| `trimming` | Hou, You 2024 | constraint trimming for sequences of related QPs (closed-loop MPC) |
| `lowrank::compress` | Nielsen, Axehill 2016 | about 2x less memory for the stored laws and facets, about 10x slower evaluation |
| `reduction::merge_regions` | Geyer, Torrisi, Morari 2008, optimal region merging | merges regions with the same law of the target outputs; `ReduceTarget::FirstInputs(k)` merges by the first control move only (7 to 11 regions become 3 on the double integrators); greedy fallback above 20,000 arrangement cells |
| `reduction::clip_reduce` | Kvasnica, Fikar 2012, clipping | removes saturated regions and clips the law; 33% to 60% fewer regions on the larger problems, 1 region on the double integrators |

### Mixed integer

| function | method | notes |
|---|---|---|
| `algorithms::mpmiqp::solve_mpmiqp_enumeration` | PPOPT's enumeration (MITree) | mpMILP and mpMIQP, overlapping partition |
| `algorithms::mpmilp_bb::solve_mpmilp_enumeration_compared` | Dua, Pistikopoulos 2000 comparison | non-overlapping mpMILP partition (dominance LP plus split on the equal-objective hyperplane) |
| `algorithms::mpmilp_bb::solve_mpmilp_bb`, `solve_mpmilp_bb_general` | Oberdieck, Wittmann-Hohlbein, Pistikopoulos 2014 | branch and bound with bound-based pruning; native branching on general integers |
| `algorithms::mpmiqp_exact::solve_mpmiqp_exact` | Oberdieck, Pistikopoulos 2015, exact comparison | non-overlapping mpMIQP partition: overlaps split on the quadric Δz(θ) = 0, which is stored exactly on the piece; emptiness of quadric pieces by spatial branch and bound |

### Which one to use

- **Strictly convex mpQP:** `mplcp`. It is the fastest serial method on 8 of the 9 benchmark problems (1.2x to
  3.7x faster than `geometric`) and the reference for degenerate mpQPs. `mplcp_parallel`
  scales to 4.7x on 8 threads on the larger problems and falls back to serial on small ones.
- **mpLP:** `mplp_lex`. It is the fastest on every mpLP in the benchmark and gives a unique gap-free partition
  even when the problem is primal and dual degenerate.
- **General fallback:** `geometric`, which handles every program class and is robust on degenerate problems.
- **`combinatorial`:** only for small problems, or when an exhaustive active-set enumeration is the point.
- **`solve_mpc_dp`:** for long MPC horizons with few regions.

## Point location

All locators implement `point_location::PointLocator`: `locate(theta) -> Option<region index>`. Every method is
exact: the candidate region is always verified with the same halfspace test. All methods share one tie rule: a
region that contains θ strictly wins over one that contains it only within the tolerance, so they return the same
region as `Solution::get_region` even on facets. A region that holds θ only within the tolerance would extrapolate
its law past its own boundary. Region classification for the trees and the hash table is conservative: interiors
are certified, and thin regions are kept as candidates rather than dropped.

| method | module | reference |
|---|---|---|
| sequential search (PPOPT's, plus an early-exit variant) | `sequential` | PPOPT `PointLocation` |
| hash table | `hash_table` | Bayat, Johansen, Jalali 2011, plus RPOPT's storage-limited build |
| binary search tree | `bst` | Tøndel, Johansen, Bemporad 2003 |
| optimized decision tree, facet tree, k-d tree | `opt_tree` | Fuchs, Jones, Morari 2010 (cut search in place of the MILP) |
| multiway tree | `multiway` | Mönnigmann, Kastsian 2011 |
| linear machine | `linear_machine` | Airan, Bhushan, Bhartiya; exists only for facet-to-facet partitions |
| descriptor function | `descriptor` | Baotić, Borrelli, Bemporad, Morari (CDC 2001; SIAM J. Control Optim. 2008) |
| YANN | `yann` | Braniff, Tian 2026 (arXiv 2505.07054) |
| subdivision walking | `walk` | Wang, Jones, Maciejowski 2007 (sequences of nearby queries) |
| graph traversal | `graph_traversal` | Herceg, Mariéthoz, Morari 2013 (sequences of nearby queries) |

**Recommendation: `HashTable::with_storage_limit(&geom, max_bytes)`.** It builds the fastest exact table that fits
the budget. The choice is made by a deterministic predicted cost, so the same input always gives the same table.

Measured on the benchmark (ns per query, 100,000 uniform queries):

| problem (regions) | hash table, 8 MB | BST | sequential | PPOPT numba |
|---|---|---|---|---|
| double integrator (7 to 11) | 9 ns | 34 to 42 ns | 61 to 91 ns | 5.3 to 6.1 µs |
| sys3x2_N3 (211) | 20 ns | 127 ns | 2.3 µs | 17.9 µs |
| masses2_N4 (307) | 31 ns | 186 ns | 5.1 µs | 23.2 µs |
| sys3x2_N5 (973) | 20 ns | 160 ns | 10.2 µs | 68.5 µs |
| masses2_N6 (901) | 37 ns | 251 ns | 14.3 µs | 63.9 µs |

The walking methods start from the previous answer. On closed-loop trajectories they still run 2x to 5x slower than
the hash table on these problems.

**Trees against grids, 2 to 10 parameters** (`examples/pl_dim_sweep.rs`, random mpQPs and spring chains with 11 to
7,919 regions): at equal storage no tree beat the hash table on volume-uniform or closed-loop queries up to 8
parameters. The hash table is 5x to 20x faster at 2 to 4 parameters and 3x to 10x faster at 5 to 8. Trees grow
1.4x to 2x deeper than log2 of the region count, because regions that straddle a cut are copied into both
branches. Trees win only on region-uniform queries at 4 to 6 parameters, where many tiny regions crowd a few grid
cells. The hash table supports up to 16 parameters.

## Saving and loading

`Solution::save(path, Format::Binary | Format::Json)` and `Solution::load(path)` (the format is detected).

- **Self-contained:** the processed program is stored with the regions, so a loaded solution can evaluate, verify
  and rebuild locators without the original problem.
- **Binary format:** little-endian, with a versioned header and a checksum. Floats are stored bit-exact, and the
  same solution always gives the same bytes.
- **Errors, not panics:** wrong headers, truncation, bit flips and dimension mismatches all return errors.
- **Hash tables:** `HashTable::save/load` stores the built index, so a controller starts in milliseconds instead
  of rebuilding it.

| problem | regions | JSON | binary | binary load | hash table rebuild | hash table load |
|---|---|---|---|---|---|---|
| sys3x2_N3 | 211 | 310 KB | 136 KB | 0.17 ms | 0.3 s | 0.5 ms |
| sys3x2_N5 | 973 | 1.9 MB | 813 KB | 0.72 ms | 2.2 s | 2.0 ms |
| masses2_N6 | 901 | 1.9 MB | 828 KB | 0.68 ms | 4.3 s | 1.8 ms |

These timings come from a heavily loaded machine; `cargo run --release --example io_bench` reproduces them.

## Performance

**mpQP benchmark:** 12 explicit MPC problems (`bench/problems`): double integrators at horizons 3 to 10, two
masses with springs, and an unstable 3-state, 2-input system, with 7 to 973 regions and 2 to 4 parameters.
**mpLP benchmark:** infinity-norm and 1-norm MPC of the same plants (`bench/problems_lp`, generated by
`bench/mplp_problems.py`), heavily primal and dual degenerate, with 20 to 28,336 regions. PPOPT is the current
GitHub source with its default backends (Gurobi 12, quadprog). Both run on the same Apple M4 Max with no other
load.

**Against PPOPT, mpQP, same algorithm, one thread** (PPOPT runs one at a time, 300 s limit, plus two 4-hour runs):

| | completed runs | min | median | max |
|---|---|---|---|---|
| speedup | 32 of 32 at 200x or more | 389x (sys3x2_N5, combinatorial_graph) | 1,020x | 2,703x |

RPOPT times are the faster median of two full runs.

- **Combinatorial on di_N10 and sys3x2_N5:** PPOPT ran 4 hours without finishing, against 6.0 s and 6.5 s in
  RPOPT: more than 2,398x and 2,206x.
- **Combinatorial on di_N8 and masses2_N6:** PPOPT exceeded 300 s, against 0.22 s and 0.29 s: more than 1,342x
  and 1,049x.
- **Parallel against parallel** (PPOPT's process pool, RPOPT's threads): 1,286x to 5,755x, median 2,110x, on the 7
  runs PPOPT finished.
- **Fastest against fastest:** PPOPT's fastest algorithm against RPOPT's fastest is 755x to 4,167x per problem.
  RPOPT's fastest is `mplcp` on 8 of the 9 problems.
- **Time to reach full coverage of the feasible parameter set:** 275x or more on every run.
  [bench/README.md](bench/README.md) has the method and all tables (`bench/results/speedup.md`).

**Against PPOPT, mpLP** (1,000 random parameters per solution checked against direct LP solves). PPOPT's
geometric and graph solvers leave parts of the feasible set uncovered on most of these problems, so RPOPT's
fastest solver is compared with PPOPT's fastest solver that leaves no sampled gap:

| problem | regions | PPOPT, fastest gap-free | PPOPT geometric (uncovered) | RPOPT `mplp_lex` | speedup |
|---|---:|---|---|---:|---:|
| diinf_N2 | 20 | geometric 144 ms | 144 ms (0%) | 0.17 ms | 836x |
| diinf_N3 | 31 | none in 120 s | 200 ms (50%) | 0.40 ms | > 300,000x |
| diinf_N4 | 39 | none in 120 s | 267 ms (50%) | 0.89 ms | > 134,000x |
| diinf_N5 | 47 | none in 120 s | 399 ms (50%) | 1.8 ms | > 65,000x |
| dione_N2 | 20 | combinatorial 3.0 s | 142 ms (0.3%) | 0.17 ms | 18,028x |
| dione_N3 | 30 | none in 120 s | 128 ms (53%) | 0.39 ms | > 300,000x |
| dione_N4 | 43 | none in 120 s | 112 ms (77%) | 0.89 ms | > 134,000x |
| sys3x2inf_N2 | 152 | none in 120 s | 2.5 s (0.4%) | 3.5 ms | > 34,000x |
| sys3x2inf_N3 | 504 | none in 120 s | crashes | 19 ms | > 6,400x |
| masses2inf_N2 | 202 | geometric 8.1 s | 8.1 s (0%) | 7.1 ms | 1,140x |
| masses2inf_N3 | 1,098 | geometric 84 s | 84 s (0%) | 49 ms | 1,703x |
| sys3x2one_N2 | 249 | geometric 4.5 s | 4.5 s (0%) | 5.1 ms | 892x |
| masses2one_N2 | 828 | geometric 45 s | 45 s (0%) | 24 ms | 1,869x |

Same algorithm on mpLP, where PPOPT finished: median 261x over 23 pairs. Graph, combinatorial and
combinatorial_graph are 213x to 17,227x. RPOPT's `geometric` is 185x to 968x on the 5 problems where PPOPT's
geometric covers the feasible set. On the 7 where PPOPT's leaves gaps (0.3% to 77% uncovered, with fewer
regions), it is 16x to 351x. Use `mplp_lex` for mpLPs.

**Against the previous RPOPT release** (commit 778cdb9, frozen binaries, alternating runs, one thread). The
regions are identical, except for mpLP `graph`, whose old partitions had gaps and missing regions:

| | pairs | median | range | below 0.95x |
|---|---:|---:|---|---:|
| mpLP (13 problems above, 5 algorithms) | 54 | **3.48x** | 0.97x to 1,269x | 0 |
| mpQP (12 problems, 5 algorithms) | 60 | **1.10x** | 0.98x to 1.79x | 0 |

- **mpLP:** `mplp_lex` 6.7x, `graph` 14x, `combinatorial_graph` 4.1x. Beyond these 13 problems (busy-machine
  measurements), `mplp_lex` reaches 83x at longer horizons (diinf_N10: 2.7 s to 33 ms), and `geometric` no longer
  stalls at degenerate vertices: diinf_N8 went from 2.1 s to 50 ms, and diinf_N10 from more than 40 s to 0.5 s.
- **mpQP:** `mplcp` 1.27x (1.15x to 1.45x) and `geometric` 1.25x (up to 1.79x). `graph`, `combinatorial_graph` and
  `combinatorial` gain 1.03x to 1.05x: their time is in rank tests and candidate enumeration rather than LPs
  (`bench/results/lp_census.md`).

**Against the authors' implementation of Arnström-Axehill 2024** (`pdaqp` 0.7.1, ParametricDAQP.jl):
`parametric_daqp` is 1.5x to 2.9x faster on the larger problems. `geometric` is about 14x faster than `pdaqp` on
sys3x2_N5 (46 ms against 645 ms). On that problem `pdaqp` returns 967 regions against 973. The 6 it drops are
thin but real: direct QP solves confirm them, and its default `lowdim_tol` discards them. With
`lowdim_tol = 0` it returns all 973.

**Fastest serial times in RPOPT:**

| problem | regions | time |
|---|---|---|
| double integrator problems | 7 to 11 | 27 to 63 µs (`mplcp`) |
| sys3x2_N3 | 211 | 1.4 ms (`mplcp`) |
| masses2_N4 | 307 | 6.5 ms (`graph_exp`) |
| sys3x2_N5 | 973 | 9.6 ms (`mplcp`) |
| masses2_N6 | 901 | 27.5 ms (`mplcp`) |

**Larger problems** (`bench/problems_large`, 131 MPC, POP and random problems with up to 428,841 regions and 10
parameters; `bench/results/large.md`): `mplcp` is the fastest exact solver overall, at a geometric mean of 0.76x the
time of `graph`. Those runs come from a shared, busy machine.

## Correctness

- **`ppopt_bench`** (`src/bin/ppopt_bench.rs`) checks every problem it runs:
  - every algorithm returns the same set of active sets;
  - 2000 random parameters agree with direct QP solves, with no gaps, no overlaps and no wrong control laws;
  - every point location method agrees with sequential search on 100,000 queries.
- **`examples/mplp_bench.rs`** checks every mpLP solution at 2,000 random parameters against direct LP solves (no
  gaps, feasible laws, optimal objective). Every exact RPOPT solver has 0 gaps and 0 errors on all
  `bench/problems_lp` problems it finishes.
- **`cargo test --release`** (116 tests) covers:
  - parity with PPOPT's results and examples;
  - each new algorithm against `geometric` and `combinatorial`, on the benchmark and on random problems;
  - degenerate problems (gaps and overlaps on 20,000 samples), including gap-free graph solvers on degenerate
    mpLPs and the boundary tie rule;
  - mpLP, mpMILP and mpMIQP checks against direct LP, MILP and MIQP solves;
  - point location on uniform queries and closed-loop sequences, and agreement of all 12 locators with
    `Solution::get_region` on 24,000 uniform and near-facet queries at 6 and 8 parameters;
  - binary round trips and corrupted files.
- **Python tests:** `python/tests` (72 tests) compare the bindings with the Rust results, and with PPOPT itself on its
  documentation example.
- **LP solver:** the LP changes were checked against 904,241 captured LPs, with zero changes in feasibility
  decisions.

## How it is fast

<details>
<summary>Exact shortcuts and low-level optimizations (click to expand)</summary>

The algorithms visit the same active sets as PPOPT, but answer most of PPOPT's LP questions without solving an LP:

- **Feasibility witnesses (combinatorial).** A feasible point of a set S that also makes the new constraint
  tight proves the child S ∪ {k} feasible. Each set carries up to 8 such points. This skips 42% to 80% of the
  feasibility LPs.
- **Multiplier rows first.** The region test first checks only λ(θ) ≥ 0 plus the parameter box. That alone rejects
  87% to 99.9% of combinatorial candidates before the full region is built.
- **Rank for free.** A well-conditioned Cholesky factor of the Schur complement A Q⁻¹ A' certifies full rank.
  Supersets of a known set get a bordered (incremental) update instead of a new factorization.
- **Region rows.** Bounding-box and projection tests decide most redundancy questions, which halves those LPs.
  LPs are warm-started from a known point by an exact change of origin.
- **Shared KKT work.** The control law, the rows and the feasible point are computed once per candidate. The QP
  Hessian factorization is cached per program.
- **LP solver.** Allocation-free dense dual simplex. A revised, const-generic path handles up to 16 variables, and a
  fixed-size path handles up to 4. Answers are verified on the original rows, with sparse Farkas certificates.
- **Region construction without redundancy LPs.** The parameter box is tightened by interval propagation and
  clipped exactly against the region's rows, so almost every row is settled as redundant or as a facet without an
  LP (masses2_N6: 21,632 redundancy LPs became 8).
- **Facet skipping in `geometric`.** A facet whose neighbour across it is already known is not stepped over: no
  centre LP and no step QP.
- **Lexicographic pivoting (`mplp_lex`).** The ε identity block of the lexicographic rows is implicit, compared
  only on exact ties, and the basis inverse is updated by rank-1 steps with a periodic refactorization. That
  removed the per-pivot dense rebuild that took 98% of the time (diinf_N10: 16.8 s to 84 ms).
- **mp-LCP tableau (`mplcp`).** Child tableaux are written in one pass, one stored block is reconstructed on demand
  with the same arithmetic, regions are kept by index, and waves go to rayon only when their estimated serial cost
  is at least 20 µs.
- **Degenerate vertices.** When enumerating candidate bases at a degenerate vertex would exceed 1,000 bases, one
  LP over the multiplier polyhedron picks the canonical basis.
- **Data layout.** Active sets are bitmasks, hashing is Fx (with the high bits rotated into the bucket index),
  kernels are specialized for the parameter dimension, counters are padded to separate cache lines, and builds use
  mimalloc, fat LTO and `target-cpu=native`.

`src/stats.rs` still counts every LP, QP and rank test PPOPT would make. The benchmark reports these
"PPOPT-equivalent" counts next to the times.

</details>

## Differences from PPOPT

<details>
<summary>Deliberate differences and bugs fixed while porting (click to expand)</summary>

Deliberate:

- **Skipped `check_optimality` LP.** PPOPT's combinatorial algorithm runs it before building each region; under
  LICQ its answer equals the region's nonemptiness, which region construction already checks.
- **Combinatorial graph visit order.** PPOPT pops candidates from a Python set, in hash order. RPOPT uses breadth
  first, which finds the large regions early; the explored set is the same.
- **Overlap flag.** `Solution.is_overlapping` is set only for mpLPs and non-convex mpQPs. PPOPT sets it for every
  mpQP.
- **Equality rows.** `get_independent_rows` keeps the last equality row when it is independent.
- **Dual signs.** Duals use the standard sign convention (Qx + c + A'λ = 0, λ ≥ 0).

Bugs found and fixed while porting:

- **Facet stepping on degenerate problems** (PPOPT's version and the original port) can leave holes: 16% of the
  parameter set on a 3D degenerate vertex, and no regions at all on another case. There were three causes:
  - zero-width facets are skipped;
  - a degenerate step point picks an arbitrary basis;
  - a facet with several neighbours is stepped across only once.
  RPOPT picks a canonical basis at degenerate points and completes suspect facets (Spjøtvold et al. 2006). Results
  on non-degenerate problems are unchanged.
- **Dense LP on larger problems.** It reported false infeasibility (from 48 variables up) because the anti-cycling
  rule switched on too early. This is fixed: decisions now match Clarabel up to 192 variables.
- **Parameter bounding box.** When constraint processing drops parameter bounds as redundant, the box now falls
  back to the feasible parameter set.
- **Graph solvers on degenerate mpLPs.** The pruning list blocked the one-pivot step to a neighbouring basis, and
  the search never left lower-dimensional optimal bases. PPOPT's graph solver leaves 10% to 24% of the parameter
  set uncovered on several of the mpLP benchmarks. RPOPT generates the pivot swaps directly and finishes with a
  facet-completion pass, so the partition is gap-free.
- **Point location at boundaries.** PPOPT returns the first region within its tolerance, or the lowest objective
  among those regions for overlapping solutions. Near a facet, that can be a neighbour whose law, extended past
  its boundary, is infeasible and looks better than the optimum. RPOPT prefers a region that contains the
  parameter strictly.

Not ported: the symbolic modeler (`mpmodel`), the code generators in `upop`, and PPOPT's 1D overlap reduction for
mpMILP. Problems are given as matrices or as JSON (see `bench/mpc_problems.py`).

</details>

## Project layout

<details>
<summary>Source tree (click to expand)</summary>

```
src/
  program.rs            MpProgram: problem data, PPOPT's constraint processing, KKT caches
  algorithms/           mpQP/mpLP solvers (mod.rs: PPOPT's algorithms), one file per added method,
                        mpmiqp.rs / mpmilp_bb.rs / mpmiqp_exact.rs (mixed integer), incremental.rs
                        (bordered KKT updates)
  mpqp_utils.rs         critical region construction (PPOPT's gen_cr_from_active_set and shortcuts)
  solver/               LP (lp_core dense dual simplex, lp_rev revised, lp_small fixed size), QP
                        (Goldfarb-Idnani), MILP/MIQP branch and bound, sparse LP via Clarabel
  point_location/       all locators, PartitionGeometry (facets, adjacency)
  solution.rs, critical_region.rs, solution_io.rs, io.rs   results and binary/JSON persistence
  mpc.rs, problem_generator.rs, reduction/lowrank/trimming  MPC models, test problems, extras
  bin/ppopt_bench.rs    the benchmark and correctness harness
examples/               profiling, LP replay, coverage, I/O, DP, MPR, approximation and sequence benchmarks
tests/                  integration tests (one file per method or feature)
bench/                  problem generators (problems: mpQP MPC, problems_lp: mpLP MPC, problems_large),
                        PPOPT runner, comparison scripts, results (see bench/README.md)
python/                 rpopt-py, the PPOPT-compatible Python bindings
```

</details>

## References

<details>
<summary>Papers behind the algorithms (click to expand)</summary>

Algorithms (DOIs):
- Gupta, Bhartiya, Nataraj, Automatica 47(9), 2011, 10.1016/j.automatica.2011.06.019
- Spjøtvold et al., Automatica 42(12), 2006, 10.1016/j.automatica.2006.06.026
- Arnström, Axehill, IEEE CDC 2024, 10.1109/cdc56724.2024.10886132
- Arnström, Bemporad, Axehill (DAQP), IEEE TAC 67(8), 2022, 10.1109/TAC.2022.3176430
- Ahmadi-Moshkenani, Johansen, Olaru, IEEE TAC 63(10), 2018, 10.1109/TAC.2018.2791479
- Jones, Morari, IEEE CDC 2006, 10.1109/CDC.2006.377797
- Herceg, Jones, Kvasnica, Morari, Automatica 62, 2015, 10.1016/j.automatica.2015.09.019
- Jones, Kerrigan, Maciejowski, Automatica 43(10), 2007, 10.1016/j.automatica.2007.03.008
- Mitze, Mönnigmann, Automatica 2020, 10.1016/j.automatica.2020.109132
- Falsone, Bianchi, Prandini, Automatica 2026, 10.1016/j.automatica.2026.112873
- Bemporad, Filippi, JOTA 2003, 10.1023/A:1023696221899
- Hou, You, arXiv 2412.11098
- Nielsen, Axehill, IEEE CDC 2016, 10.1109/CDC.2016.7798819
- Dua, Pistikopoulos, Ann. Oper. Res. 99, 2000, 10.1023/A:1019241000636
- Oberdieck, Wittmann-Hohlbein, Pistikopoulos, J. Global Optim. 59, 2014, 10.1007/s10898-014-0143-9

Point location:
- Tøndel, Johansen, Bemporad, Automatica 39(5), 2003
- Bayat, Johansen, Jalali, Automatica 47(3), 2011
- Wang, Jones, Maciejowski, ECC 2007, 10.23919/ECC.2007.7068623
- Herceg, Mariéthoz, Morari, ECC 2013, 10.23919/ECC.2013.6669667
- Braniff, Tian, arXiv 2505.07054

</details>

## License

MIT, see [LICENSE](LICENSE). RPOPT is a port of PPOPT (MIT License, Copyright (c) 2021 Dustin Kenefake,
Efstratios N. Pistikopoulos), whose notice is reproduced in the license file.
