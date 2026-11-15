<div align="center">

# RPOPT

**Fast, exact multiparametric programming in Rust**

A rewrite of [PPOPT](https://github.com/TAMUparametric/PPOPT) for explicit model predictive control:
mpLP, mpQP, mpMILP and mpMIQP solvers, plus fast exact point location.

![license](https://img.shields.io/badge/license-MIT-blue)
![rust](https://img.shields.io/badge/rust-2024%20edition-orange)
![python](https://img.shields.io/badge/python-PPOPT--compatible%20bindings-3776AB)
![tests](https://img.shields.io/badge/tests-133%20Rust%20%2B%2072%20Python-brightgreen)

</div>

## Speedup over PPOPT

**Against each of PPOPT's four solvers** (same algorithm, one thread, runs where PPOPT finished with a gap-free
answer; median, then range):

| PPOPT solver | mpQP | mpLP |
|---|---:|---:|
| `combinatorial` | **1,003x** (705x to 1,656x) | **922x** (780x to 1,475x) |
| `graph` | **1,090x** (600x to 2,608x) | **1,195x** (1 run) |
| `geometric` | **997x** (554x to 2,252x) | **231x** (185x to 968x) |
| `combinatorial_graph` | **1,034x** (408x to 2,751x) | **234x** (213x to 261x) |

**Other comparisons:**

| comparison | cases | median | range |
|---|---:|---:|---:|
| mpQP, PPOPT's fastest solver against RPOPT's fastest | 9 problems | **1,739x** | 753x to 4,062x |
| mpQP, parallel against parallel | 7 runs | **2,063x** | 1,387x to 5,889x |
| mpLP, RPOPT's fastest against PPOPT's fastest gap-free solver | 6 problems | **1,422x** | 836x to 18,028x |
| mpLP, no gap-free PPOPT answer within 120 s | 7 problems | **> 134,000x** | > 6,400x to > 305,000x |
| point location, per query (PPOPT numba, RPOPT hash table) | 9 problems | **676x** | 593x to 3,603x |

<sub>Same Apple M4 Max; PPOPT `main` with Gurobi 12. "> " is a lower bound (PPOPT hit its time limit). Details and
caveats: [Performance](#performance).</sub>

## Highlights

- **Exact and verified.** Each benchmark run checks that all algorithms agree. It also samples 2000 parameters
  against direct QP solves, looking for gaps, overlaps and wrong control laws.
- **All of PPOPT's algorithms, plus twelve from the literature:**
  - Arnström-Axehill with DAQP, as published;
  - Ahmadi-Moshkenani;
  - mp-LCP pivoting, and mp-LCP enumeration;
  - lexicographic mpLP;
  - active-set dynamic programming;
  - randomized MPR;
  - approximate mpQP;
  - mpMILP comparison (Dua-Pistikopoulos), and mpMILP branch and bound;
  - exact mpMIQP comparison (Oberdieck-Pistikopoulos);
  - combinatorial with constraint-pair pruning (Kenefake-Pistikopoulos).
- **Robust on degenerate problems.** RPOPT closes holes that facet stepping (as implemented in PPOPT) leaves at
  degenerate points. On degenerate mpLPs, every RPOPT exact solver returns a gap-free partition; PPOPT's geometric
  and graph solvers leave 10% to 77% of the feasible parameter set uncovered on most of them.
- **Exact mixed-integer comparison.** mp-MIQP solutions with quadric region boundaries (Oberdieck and
  Pistikopoulos 2015), in addition to PPOPT's overlapping enumeration.
- **Fast online evaluation.** Point location takes 9 to 32 ns per query with a hash table built to a storage
  budget. Ten other exact locators are included. The table, trees, walking and sequential methods return the same
  region as `Solution::get_region` on every tested query, including points on region boundaries.
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
| `combinatorial_kp`, `combinatorial_kp_parallel` | Kenefake, Pistikopoulos 2023 | paper | `combinatorial` plus pruning by constraints and constraint pairs that can never be active together (pair LPs with complementarity branching in place of the paper's MILP); same regions; switches on above 2M candidates per level: 1.5x faster on sys3x2_N5, 1.2x slower on di_N10 |
| `graph`, `graph_exp`, `graph_parallel` | Oberdieck et al. 2017, connected graph | PPOPT | `graph_exp` has no pruning list; on mpLPs, one-pivot swaps plus a facet-completion pass make the partition gap-free |
| `geometric`, `geometric_parallel` | facet stepping (Spjøtvold et al. 2006) | PPOPT | handles every program class; fixed for degenerate problems (see below); skips facets whose neighbour is already known |
| `combinatorial_graph`, `combinatorial_graph_parallel` | Arnström, Axehill 2024, as simplified in PPOPT | PPOPT | breadth-first visit order |
| `parametric_daqp` | Arnström, Axehill 2024, as published | paper | least-distance transformation and DAQP subproblems; 1.5x to 2.9x faster than the authors' `pdaqp` |
| `ahmadi` | Ahmadi-Moshkenani, Johansen, Olaru 2018 | paper | handles LICQ and strict-complementarity violations (PPOPT ships an empty stub) |
| `mplcp`, `mplcp_parallel` | Jones, Morari 2006, mp-LCP pivoting | paper | the fastest serial mpQP solver on the benchmark; lexicographic perturbation: no gaps or overlaps at degenerate vertices (the Case-2 sign test is scale-free since the pop_001 twin-row fix, so no end filter is needed: 0 lexico-infeasible regions on 135 problems; serial 1.04x faster, 1.45x to 1.58x on the two largest degenerate cases, `bench/results/mplcp_case2_tolerance.md`); mpLPs are routed to `mplp_lex`; the neighbour search on the facet a region was entered through by a single pivot is skipped (it can only return the parent; about 2% faster serial, same regions; measured on Negishi, median ratio 1.02 to 1.03 against an A/A noise band of 0.99 to 1.01 on the larger problems, see `bench/results/negishi_entry_skip_ab.md`; no resolvable gain at 16 threads); a facet certificate (a facet whose Case-2 neighbour search would only return a known region is skipped when a known region has the opposite row on the same hyperplane and contains the facet, checked on the facet's clip vertices, 3 to 5 parameters only; 11% to 41% of the Case-2 facets skipped, 1.05x to 1.15x faster serial on `masses2_N4` to `N8` and `pop_006`, `bench/results/mplcp_facet_certificate.md`) is implemented but **off by default** (`mplcp::set_certificate(1)`, or `RPOPT_CERT=1` in the examples): at 6 parameters it lost a real region on `masses3_N4`, and the stray overlapping regions of `pop_005`, `pop_007` and `pop_011` could pass as the known neighbour, so it stays off until that is understood; each region's law is read from the pivot tableau instead of a fresh KKT solve (same regions; coefficients agree with the KKT solve to about 6e-9 relative, so not bit-identical; `mplcp::set_tableau_law(false)` restores the KKT solve); child tableaux are pivoted lazily when popped (a shared parent, no time change, 4% to 29% less peak memory on 3 to 8 parameter problems) and rebuilt from scratch every 32 levels instead of 8 (the pivot drift does not grow with depth; 1% to 6% faster serial on sys3x2_N3 to N7 and masses2_N8, same active sets and order, facet rows within 2e-12 except 1.6e-8 on pop_005; measured on Negishi against an A/A band of 0.98 to 1.00, see `bench/results/mplcp_tableau_handling.md`); the region build no longer repeats the KKT rows and the Chebyshev LP when it falls back to the LP path, and uses vertex clipping only up to 5 parameters (6 parameters: 1.6x to 21x faster, 7 to 10 parameters: 1.01x to 1.04x, 3 to 5 parameters: 1.01x to 1.06x, see `bench/results/mplcp_build_speed.md`) |
| `mplcp_enum` | Herceg, Jones, Kvasnica, Morari 2015 | paper | basis enumeration for general M (mpQP and mpLP); exponential |
| `mplp_lex` | Jones, Kerrigan, Maciejowski 2007 | paper | lexicographic mpLP: unique, gap- and overlap-free partition for degenerate mpLPs; the fastest mpLP solver on the benchmark and 3x to 30x faster than `graph` at 5 to 12 parameters; `combinatorial` is 1.5x faster on the two inf-norm MPC problems with 48 and 52 regions at 10 and 12 parameters (mostly regions of radius 2.6e-8), and `combinatorial_graph` 1.09x on one random 10-parameter mpLP (`bench/results/mplp_lex_highdim.md`); the facet interior point comes from the region center's foot on the facet plane, with the facet LP as fallback (4 to 18% faster, same regions) |
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

- **Strictly convex mpQP with up to about 4 parameters:** `mplcp`. It is the fastest serial method on 8 of the 9
  benchmark problems (1.2x to 1.8x faster than `geometric`) and the reference for degenerate mpQPs. `mplcp_parallel`
  scales to 4.7x on 8 threads on the larger problems and falls back to serial on small ones.
- **Strictly convex mpQP with 5 or more parameters:** `graph`, which is 1.3x to 1.9x faster than `mplcp` there
  on the large suite. `combinatorial` is worth trying when the constraints are few.
- **mpLP:** `mplp_lex`. It is the fastest on every mpLP in the benchmark (2 to 4 parameters) and on 13 of 16
  problems at 5 to 12 parameters, where it beats `graph` by 3x to 30x; it gives a unique gap-free partition even
  when the problem is primal and dual degenerate. On mpLPs with few regions (under about 2,000) and 4 to 6 variables
  `combinatorial` can be up to 1.7x faster.
- **General fallback:** `geometric`, which handles every program class and is robust on degenerate problems.
- **`combinatorial`:** only for small problems, or when an exhaustive active-set enumeration is the point.
- **`solve_mpc_dp`:** for long MPC horizons with few regions.

## Point location

All locators implement `point_location::PointLocator`: `locate(theta) -> Option<region index>`. Every method is
exact: the candidate region is always verified with the same halfspace test. All methods except the descriptor
function and YANN share one tie rule: a region that contains θ strictly wins over one that contains it only within
the tolerance, so they return the same region as `Solution::get_region` even on facets. A region that holds θ only within the tolerance would extrapolate
its law past its own boundary. Region classification for the trees and the hash table is conservative: interiors
are certified, and thin regions are kept as candidates rather than dropped.

There is one point location tolerance. `Solution::point_location_tolerance` defaults to
`solution::DEFAULT_POINT_LOCATION_TOL` (1e-9, also `point_location::LOCATE_TOL`), and every locator takes the value of
the solution it is built from (`SequentialSearch::new`, `PartitionGeometry::new`, hence the trees, tables and walks), so
`Solution::get_region` and the locators agree. Another value goes in the solution field before building locators
(`sol.point_location_tolerance = 1e-5` gives PPOPT's), or `PartitionGeometry::with_tol` / `SequentialSearch::with_tol`.
PPOPT uses 1e-5; 1e-9 is used here because floating-point seams between neighbouring regions are below 1e-10 on the
benchmark problems, while a 1e-5 tolerance returns a region for points up to 1e-5 outside the partition, whose extended
law violates the constraints by up to about 2e4 times the distance (0.16 at distance 9e-6 on masses2_N10).

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
| weighted nearest neighbour (power diagram) | `wnn` | Jones, Grieder, Raković 2006; research locator, see below |

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

**Weighted nearest neighbour** (`examples/pl_wnn_bench.rs`, `bench/results/pl_wnn.md`): a partition that is a power
diagram can be searched as a nearest-neighbour problem in one dimension more. Every mpLP partition qualifies (its
sites and weights come from the value function, no LP), but most mpQP partitions do not: only the 8 small double
integrator problems of the 12 benchmarks, and none of 15 random and mass-chain mpQPs, because those partitions are not
facet-to-facet. Where it applies, the exact k-d search still scores 25% to 40% of the regions per query and is 5x to
250x slower than the hash table, so it is kept as a tested research locator with the hash table as exact fallback.

## Saving and loading

`Solution::save(path, Format::Binary | Format::Json)` and `Solution::load(path)` (the format is detected).

- **Self-contained:** the processed program is stored with the regions, so a loaded solution can evaluate, verify
  and rebuild locators without the original problem.
- **Binary format:** little-endian, with a versioned header and a checksum. Floats are stored bit-exact, and the
  same solution always gives the same bytes.
- **Errors, not panics:** wrong headers, truncation, bit flips and dimension mismatches all return errors.
- **Hash tables:** `HashTable::save/load` stores the built index, so a controller starts in milliseconds instead
  of rebuilding it. Format version 2 also stores the point location tolerance; version 1 files must be rebuilt.

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

**Examples:**

| problem | PPOPT | RPOPT | speedup |
|---|---:|---:|---:|
| mpQP sys3x2_N5, 973 regions, `geometric` in both | 18.6 s | 17.6 ms | 1,061x |
| mpQP sys3x2_N5, fastest solver (PPOPT `geometric`, RPOPT `mplcp`) | 18.6 s | 9.7 ms | 1,920x |
| mpQP di_N10, `combinatorial` in both | > 4 h, unfinished | 6.1 s | > 2,344x |
| mpLP masses2one_N2, 828 regions (PPOPT `geometric`, RPOPT `mplp_lex`) | 44.6 s | 24 ms | 1,869x |
| point location, sys3x2_N5, per query | 68.5 µs | 19 ns | 3,603x |

**Against PPOPT, mpQP, same algorithm, one thread** (PPOPT runs one at a time, 300 s limit, plus two 4-hour runs):

| | completed runs | min | median | max |
|---|---|---|---|---|
| speedup | 32 of 32 at 200x or more | 408x (masses2_N6, combinatorial_graph) | 1,034x | 2,751x |

RPOPT times are the faster median of two full runs.

- **Combinatorial on di_N10 and sys3x2_N5:** PPOPT ran 4 hours without finishing, against 6.1 s and 6.5 s in
  RPOPT: more than 2,344x and 2,203x.
- **Combinatorial on di_N8 and masses2_N6:** PPOPT exceeded 300 s, against 0.23 s and 0.29 s: more than 1,324x
  and 1,048x.
- **Parallel against parallel** (PPOPT's process pool, RPOPT's threads): 1,387x to 5,889x, median 2,063x, on the 7
  runs PPOPT finished.
- **Fastest against fastest:** PPOPT's fastest algorithm against RPOPT's fastest is 753x to 4,062x per problem.
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
- **LP census at 3 to 10 parameters** (`bench/results/lp_census_highdim.md`, `--features lp_census`): why `mplcp` and
  `geometric` lose ground to `graph` as the parameter count grows (neighbour search per facet; vertex clipping is
  compiled only for 2 to 5 parameters, and lifting that cap was measured 1.1x to 15x slower, so it is not kept).
- **Region build by parameter count** (`bench/results/mplcp_build_speed.md`, Negishi, serial, paired per task, A/A
  band 0.99 to 1.01 on the larger problems). Speedup over the previous main:

  | parameters | `mplcp` | `graph` | `geometric` |
  |---|---|---|---|
  | 2 to 4 | 1.01x to 1.06x (median 1.02x) | 1.00x (4 parameters) | 1.01x (4 parameters) |
  | 5 | 0.99x to 1.04x (median 1.01x) | not timed | not timed |
  | 6 | 1.66x (rand_t6), 3.5x (masses3_N3), 12x (masses3_N4), 21x (pop_017) | 1.8x to 32x | 1.1x to 4.3x |
  | 7 to 10 | 1.01x to 1.04x | 1.01x (8 parameters) | 0.98x (8 parameters) |

  The 6-parameter gain comes from stopping vertex clipping at 5 parameters in the shared builder and in
  `mplcp`'s builder: on 6 parameters the clip needed hundreds of vertices per region (up to 40 ms) and, when it
  gave up, ran twice before the LP path (about 0.1 ms) took over. Rows differ from the previous clip results on a few
  regions of the tolerance-edge kind, see the results file.

**Against the authors' implementation of Arnström-Axehill 2024** (`pdaqp` 0.7.1, ParametricDAQP.jl):
`parametric_daqp` is 1.5x to 2.9x faster on the larger problems. `geometric` is about 14x faster than `pdaqp` on
sys3x2_N5 (46 ms against 645 ms). On that problem `pdaqp` returns 967 regions against 973. The 6 it drops are
thin but real: direct QP solves confirm them, and its default `lowdim_tol` discards them. With
`lowdim_tol = 0` it returns all 973.

**Fastest serial times in RPOPT:**

| problem | regions | time |
|---|---|---|
| double integrator problems | 7 to 11 | 27 to 65 µs (`mplcp`) |
| sys3x2_N3 | 211 | 1.4 ms (`mplcp`) |
| masses2_N4 | 307 | 6.5 ms (`graph_exp`) |
| sys3x2_N5 | 973 | 9.7 ms (`mplcp`) |
| masses2_N6 | 901 | 27.5 ms (`mplcp`) |

**Larger problems** (`bench/problems_large`, 131 MPC, POP and random problems with up to 428,841 regions and 10
parameters; `bench/results/large.md`): `mplcp` is the fastest exact solver overall, at a geometric mean of 0.92x the
time of `graph` (119 problems solved).
- The advantage holds at few parameters: 0.17x to 0.57x of `graph` at 1 to 3 parameters and 0.91x at 4.
- From 5 parameters on, `graph` is faster: `mplcp` takes 1.29x at 5 and 1.4x to 1.9x at 6 to 10.
- Five `mplcp` and five `geometric` runs returned no regions, because of a start-region bug since fixed. An earlier
  summary counted them as solves and reported 0.76x.
- Those runs come from a shared, busy machine; a rerun on Negishi is pending.

## Correctness

- **`ppopt_bench`** (`src/bin/ppopt_bench.rs`) checks every problem it runs:
  - every algorithm returns the same set of active sets;
  - 2000 random parameters agree with direct QP solves, with no gaps, no overlaps and no wrong control laws;
  - every point location method agrees with sequential search on 100,000 queries.
- **`examples/mplp_bench.rs`** checks every mpLP solution at 2,000 random parameters against direct LP solves (no
  gaps, feasible laws, optimal objective). Every exact RPOPT solver has 0 gaps and 0 errors on all
  `bench/problems_lp` problems it finishes.
- **`cargo test --profile gate`** (137 tests, 3 more ignored) covers the list below.
  - The `gate` profile is release optimisation without fat LTO, so test builds are fast. A cold build of all
    test binaries takes 42 s instead of 246 s, and a rebuild after a one-file change 3.2 s instead of 242 s
    (Negishi, 16 CPUs).
  - Results match the release profile: region hashes were identical for `mplcp`, `graph`, `geometric` and
    `combinatorial_graph` on 58 problem runs.
  - The integration tests compile as one binary (`tests/all/`). `mplcp_rows` and `mplcp_tableau_law` stay
    separate, because they flip a process-wide switch.
  - Timing builds keep `--release`.

  The tests cover:
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
- **Formal verification:** [`verification/lean/`](verification/lean/README.md) has Lean 4 and Mathlib proofs (no
  `sorry`, standard axioms only) of the exact-arithmetic mathematics behind `mplcp` and `mplp_lex`. They show
  that a pLCP solution gives the unique mp-QP optimizer, that each region's tableau law is optimal on its region
  and equals the KKT-solve law, that pivots and the Case-1 facet step are correct, that lexico-feasible bases
  never overlap (both solvers), and that the entry-facet skip is safe when the parent region is lexico-feasible.
  For the Case-2 degenerate-facet step they show that every basis it returns from a genuine facet is
  lexico-feasible just across it, that its sub-facets cover the facet, and that its pivot walk never repeats a
  state from a lexico-feasible start (so the `depth > 6` and `8n + 50` caps can become a cycle check that reports
  instead of dropping neighbours). The known-region skip of Case-2 facets is safe when the known region is
  lexico-feasible. Open: KKT necessity, Case-2 completeness (a Lemke-type termination statement), and the
  facet-graph connectivity behind coverage. Tolerances and floating point are not modelled. The model traced the
  pop_001 overlap to absolute tolerances in the Case-2 sign test of `pivot_facet`; that is fixed (see "Bugs found
  and fixed" below and `bench/results/mplcp_case2_tolerance.md`).

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
- **Point location tolerance.** 1e-9 instead of PPOPT's 1e-5, shared by `Solution` and every locator (see
  [Point location](#point-location)); set `point_location_tolerance = 1e-5` for PPOPT's behaviour.

Bugs found and fixed while porting:

- **Facet stepping on degenerate problems** (PPOPT's version and the original port) can leave holes: 16% of the
  parameter set on a 3D degenerate vertex, and no regions at all on another case. There were three causes:
  - zero-width facets are skipped;
  - a degenerate step point picks an arbitrary basis;
  - a facet with several neighbours is stepped across only once.
  RPOPT picks a canonical basis at degenerate points and completes suspect facets (Spjøtvold et al. 2006). Results
  on non-degenerate problems are unchanged.
- **Zero regions from a dependent start.** `mplcp` and `geometric` returned no regions on five POP problems
  (`pop_001`, `pop_019`, `pop_031`, `pop_052`, `pop_071`) because the sampled start active set had linearly
  dependent rows (duplicate bounds binding together), so it had no multiplier law. The start now requires full row
  rank and falls back to a canonical basis (`degenerate_start`, now also used by `mplcp`).
- **Overlapping regions from `mplcp` on degenerate programs.** After that fix `mplcp` returned the same parameter
  set twice on `pop_001`, `pop_019` and `pop_031` (index sets that differ in which of several dependent rows is
  active, for example rows 4 and 23 of `pop_001`), because a basis with an identically vanishing multiplier or slack
  is lexico-infeasible and the fallback start (and, in `pop_001`, a facet pivot) can land on one. `mplcp` now
  replaces a lexico-infeasible start by the lexicographic Lemke basis at the start region's Chebyshev center
  (`lex_feasible_start`). A lexico-infeasible region found by a facet pivot was first dropped by an end filter
  (`drop_lex_infeasible`); the filter is gone since the Case-2 fix below.
- **No start region above 15 parameters.** On `masses8_N2` (16 parameters, `bench/problems_highdim`) every solver
  that walks from a start region (`mplcp`, `graph`, `geometric`, `combinatorial_graph`, `parametric_daqp`) returned no
  regions; only the enumeration solvers found the 394. There were two causes:
  - the feasible-space Chebyshev LP stopped at a point violating one row by 1.1e-8, which the simplex's 1e-9 check
    rejected;
  - the start samplers drew from the cube around the centre, which at 16 parameters lies almost entirely outside
    the feasible set.

  That one LP now falls back to Clarabel, and the samplers fall back to the centre and to points in the Chebyshev
  ball. Both run only where the old code found nothing, so problems that already worked are unchanged.
- **Case-2 sign test of `mplcp` (`pivot_facet`).** The lexico-inequality Γ = a_l row_j − a_j row_l scales with
  a_l a_j, which is 1e-20 for rows with a near the pivot tolerance 1e-10, so the absolute zero tolerance read both
  twins (rows 4 and 23 of `pop_001`) as "zero, so nonnegative" and accepted both. The test now uses the scale-free
  row_j/a_j − row_l/a_l with tolerances relative to its largest entry, and `in_z` uses the same relative tolerance
  as the zero-row test of `adjacent` (it could differ, which sent Case 2 a finite step off the facet). With the end
  filter removed `pop_001` gives 3529 regions (main 3535: six were twins of existing regions), 0 lexico-infeasible
  regions on 135 problems, and the serial solve is 1.04x faster in the median (1.45x on `masses3_N4`, 1.58x on
  `pop_001`). The `depth > 6` guard is unchanged and still fires on `pop_005`, `masses4_N3`, `masses3_N4`,
  `masses5_N2` and `masses3_N3`. Open and not caused by this change: a stray overlapping region on `pop_005`,
  `pop_007` and `pop_011` (also on main), and 2 regions on `pop_081` (also on main). Details:
  `bench/results/mplcp_case2_tolerance.md`.
- **Dense LP on larger problems.** It reported false infeasibility (from 48 variables up) because the anti-cycling
  rule switched on too early. This is fixed: decisions now match Clarabel up to 192 variables.
- **Parameter bounding box.** When constraint processing drops parameter bounds as redundant, the box now falls
  back to the feasible parameter set.
- **Graph solvers on degenerate mpLPs.** The pruning list blocked the one-pivot step to a neighbouring basis, and
  the search never left lower-dimensional optimal bases. PPOPT's graph solver leaves 10% to 24% of the parameter
  set uncovered on several of the mpLP benchmarks. RPOPT generates the pivot swaps directly and finishes with a
  facet-completion pass, so the partition is gap-free.
- **Large, badly scaled problems.** Three RPOPT defects found by diffing the algorithms' partitions on the large suite:
  `mplcp` tested facet rows for zero with an absolute tolerance and missed regions when the rows were large (2 real
  regions on pop_006); `combinatorial_graph` kept noise slivers and two rank-deficient regions reached through its
  incremental update; and redundancy removal could drop both rows of a near-duplicate pair. All three are fixed. The
  remaining differences between algorithms on these problems are thin regions (Chebyshev radius below 1e-6).
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
tests/                  integration tests: all/ (one module per method or feature, one binary) and two
                        separate binaries that flip a process-wide switch
bench/                  problem generators (problems: mpQP MPC, problems_lp: mpLP MPC, problems_large,
                        problems_highdim: mpQP at 12 to 16 parameters),
                        PPOPT runner, comparison scripts, results (see bench/README.md)
python/                 rpopt-py, the PPOPT-compatible Python bindings
docs/                   technical-report (algorithms, speed techniques, benchmarks, results) and
                        mplcp-report (why pLCP pivoting is fast, and why recent surveys omit it)
verification/lean/      Lean 4 proofs of the mplcp and mplp_lex mathematics (status table in its README)
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
