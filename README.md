# RPOPT

RPOPT (crate `ppopt_rs`) is a Rust rewrite of [PPOPT](https://github.com/TAMUparametric/PPOPT) (Kenefake and Pistikopoulos, TAMU), the
Python package for multiparametric programming, with seven point location methods for evaluating the
explicit solution online.

## What is ported

| PPOPT (Python) | ppopt_rs (Rust) | notes |
|---|---|---|
| `MPLP_Program`, `MPQP_Program` | `program::MpProgram` | same constraint processing: equalities to the top, zero rows moved to `A_t`, row scaling, implicit equalities, rank-reduced equalities, redundant rows removed by LP |
| `MPMILP_Program`, `MPMIQP_Program`, `MITree` | `algorithms::mpmiqp::MpmiProgram` | MILP-based processing, substituted sub-problems, depth-first feasible binary enumeration |
| `CriticalRegion`, `Solution` | `critical_region`, `solution` | `evaluate`, `get_region` (overlapping and not), `verify_solution`, `verify_theta` |
| `utils.mpqp_utils.gen_cr_from_active_set` (+1D) | `mpqp_utils` | same row classification (λ, inactive, Ω) and the same facet test (row at equality must be feasible) |
| `mp_solvers`: combinatorial, graph, graph_exp, geometric, combinatorial_graph, parallel variants | `algorithms` | parallel variants use rayon instead of pathos |
| `solve_mpmiqp` (enumeration) | `algorithms::mpmiqp::solve_mpmiqp_enumeration` | sub-problems solved in parallel |
| `utils.constraint_utilities`, `chebyshev_ball`, `general_utils` | `utils` | |
| `problem_generator` | `problem_generator` | same construction, Rust RNG |
| `upop.point_location.PointLocation` (numba) | `point_location::SequentialSearch` | plus six other methods, see below |
| `plot.parametric_plot` | `plot::parametric_plot_svg` | SVG instead of matplotlib, 2D only |
| solver interfaces (Gurobi, GLPK, quadprog, DAQP) | `solver` | native: dense LP (simplex on the dual), Goldfarb-Idnani QP (the quadprog algorithm), branch and bound for MILP/MIQP, Clarabel for large sparse LPs |

Not ported: the symbolic modeler (`mpmodel.MPModeler`), the C++/JS/Python code generators in `upop`, and the
1D overlap reduction for mpMILP (`region_overlap_utils`). Problems are given as matrices, or as JSON in the
layout of `bench/mpc_problems.py`.

### Deliberate differences

- PPOPT's combinatorial algorithm calls `check_optimality` (one LP over x, θ, λ, slacks) before building each
  region. With LICQ, x and λ are affine in θ, so that LP is feasible exactly when the unreduced critical region
  is nonempty, which region construction checks anyway. The Rust version skips the call (same regions).
- `Solution.is_overlapping`: PPOPT sets it for every mpQP (its `isinstance(MPLP_Program)` check also matches
  `MPQP_Program`). Here only mpLPs and non-convex mpQPs are flagged.
- PPOPT's `get_independent_rows` never keeps the last equality row; the Rust version keeps it when independent.
- Combinatorial graph: PPOPT takes the next candidate with `set.pop()` (Python hash order); the Rust version
  takes candidates first in, first out (breadth-first from the initial region). The explored set, and so the
  solution and the PPOPT-equivalent counts, do not depend on the order, but breadth-first builds the large
  regions near the start early (masses2_N6: half the feasible parameter set covered after 34 ms instead of
  132 ms with a stack).
- Duals are reported in the standard sign convention (Qx + c + A'λ = 0, λ ≥ 0); PPOPT negates the inequality
  multipliers coming from quadprog.

### Exact shortcuts (same answers, less work)

The algorithms visit the same active sets in the same order as PPOPT and keep the same rows, but answer
several of PPOPT's LP questions without that LP:

- **Region rows.** PPOPT keeps a row iff the region with that row at equality is feasible (one LP per row).
  Here a row is dropped without an LP when its maximum over the region's bounding box is below its
  right-hand side, and kept without an LP when the projection of the Chebyshev center onto its hyperplane is
  feasible; only the remaining rows get an LP. This roughly halves the LPs of region construction.
- **One KKT solve per candidate.** The optimality test, the feasibility LP and region construction share the
  control law, the region rows and the feasible point (PPOPT recomputes them).
- **Warm starts.** LPs are solved from a known point by an exact change of origin (x = x0 + z): region row
  tests from the Chebyshev center, combinatorial feasibility tests from the parent's feasible point.
- **Rank tests.** Subsets of a full-rank active set are full rank (singular values interlace), and a superset
  of a full-rank set is tested against an orthonormal basis of the parent; borderline cases fall back to the
  full test. In the combinatorial and combinatorial graph algorithms, the KKT law is computed first and a
  Cholesky factorization of the Schur complement A Q^{-1} A' with all pivots above 1e-10 of the largest
  diagonal entry certifies full rank, so the separate rank test runs only when it does not.
- **Combinatorial feasibility witnesses.** A feasible (x, θ) point of an active set S with S's constraints
  tight is also a feasible point of the child S ∪ {k} if it makes constraint k tight (relative tolerance
  1e-11, 100x stricter than the LP's own acceptance test). Each set carries up to 8 such points, inherited
  from its parent and taken from the LP solutions of its siblings (simplex vertices, tight on their whole
  basis); the feasibility LP runs only when no point applies. This skips 42% to 80% of those LPs on the
  larger benchmark problems, with the same feasible/infeasible decision for every candidate.
- **Region test on the λ rows first.** In the combinatorial algorithm almost every feasible active set has an
  empty critical region, and in 87% to 99.9% of them (measured on di_N8, sys3x2_N3, masses2_N6) the λ >= 0
  rows plus the parameter box are already infeasible. Those rows are tested first, as the same rows the full
  system contains (so an infeasible subsystem is a certificate for the full one); the inactive-constraint
  rows and the primal law are built only for sets that pass. The combinatorial graph and graph algorithms
  test their candidates the same way (identical exploration: the same PPOPT-equivalent LP and rank counts).
- **Pruning list by largest element.** A child S ∪ {i} (i above every element of S) can only contain a pruned
  set whose largest element is i, so only those sets are tested, as 64-bit masks.
- **Facet stepping.** PPOPT scans every found region before each QP; here the scan runs only when the step
  would otherwise build a new region (the only case where it changes the outcome).
- **QPs.** The Hessian factorization and the equality phase of the Goldfarb-Idnani solver depend only on Q
  and the equality rows, so they are computed once per program.

`stats.rs` still counts every LP, QP and rank test PPOPT would make; the benchmark uses those counts.

## Point location methods

All implement `point_location::PointLocator` (`locate(θ) -> Option<region>`), all are exact: the candidate
region is always verified, so a near-tie at a facet cannot return a wrong region.

| method | module | reference |
|---|---|---|
| sequential search | `sequential` | PPOPT `PointLocation` (all rows first, then scan) and an early-exit variant |
| binary search tree | `bst` | Tøndel, Johansen, Bemporad, *Automatica* 39(5):945-950, 2003 |
| linear machine | `linear_machine` | Airan, Bhushan, Bhartiya, *IEEE TAC* 62(3):1403-1410, 2017; correctness check after Sharma, Bhushan, *IFAC-PapersOnLine* 53(2):11368-11373, 2020 |
| multiway tree | `multiway` | Mönnigmann, Kastsian, *IFAC World Congress*, 2011 |
| descriptor function | `descriptor` | Baotić, Borrelli, Bemporad, Morari, *SIAM J. Control Optim.* 47(5):2470-2489, 2008 |
| hash table | `hash_table` | Bayat, Johansen, Jalali, *Automatica* 47(3):571-577, 2011 |
| YANN | `yann` | Braniff, Tian, *Computers & Chemical Engineering*, 2026 (arXiv:2505.07054) |

Implementation notes:

- **BST**: hyperplanes are the distinct facet hyperplanes; regions are classified against each from their
  vertices; split regions are re-checked with an LP on the node polytope (Tøndel's refinement); the split
  minimises max(|I-|, |I+|).
- **Linear machine**: one LP with g_i - g_j = α_ij (b - a'θ), α_ij >= 1 per adjacent pair, solved with Clarabel
  (sparse). The report checks g_i >= g_j at every vertex of R_i for every j, which is necessary and sufficient
  for correctness, in floating point. If the LP is infeasible because of numerically thin regions, those
  regions are left out and reached through the neighbour fallback.
- **Multiway tree**: built from the BST as in the paper; each node hides d binary levels (order 2^d - 1), whose
  hyperplanes are evaluated together before the sign bits pick the child.
- **Descriptor function**: f_i(θ) = w'x_i*(θ) with random w; facets between neighbours are tested by comparing
  f values, domain facets (and the rare discontinuous ones) by their row.
- **Hash table**: uniform grid over the bounding box; each box keeps the regions that meet it (LP test), sorted
  by size. The grid size sets the time/storage trade-off.
- **YANN**: the five layers of the paper (step, ReLU indicator, first-active selector, big-M ReLU pairs,
  linear output), in f64; `locate` runs layers 1-3, `Yann::evaluate` all five.

## Usage

```rust
use ppopt_rs::{solve_mpqp, MpqpAlgorithm, MpProgram};
use ppopt_rs::point_location::{PartitionGeometry, LinearMachine, PointLocator};
use std::sync::Arc;

let prog = Arc::new(MpProgram::from_json_file("bench/problems/di_doc_N3.json".as_ref())?);
let sol = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
let geom = PartitionGeometry::new(&sol);
let lm = LinearMachine::new(&geom).unwrap();
let u = lm.locate(&[1.5, -0.5]).map(|i| sol.critical_regions[i].evaluate(&[1.5, -0.5]));
```

`cargo run --release --example explicit_mpc` solves the PPOPT documentation example and writes an SVG plot.

## Benchmark

Explicit MPC benchmark against PPOPT itself (GitHub `main`, Gurobi 12, quadprog), details in
[bench/README.md](bench/README.md): same algorithm, single thread, every completed PPOPT run is at least
261x slower (32 of 32 runs, median 728x); point location 241x to 1,345x faster (hash table vs PPOPT's numba
search). Two combinatorial PPOPT runs (di_N10, sys3x2_N5) are still being measured.
