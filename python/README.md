# rpopt (Python)

Python bindings for RPOPT, the Rust port of [PPOPT](https://github.com/TAMUparametric/PPOPT), with PPOPT's
names and argument order. The bindings are a separate crate (`rpopt-py`, PyO3 + maturin) that depends on the
`rpopt` crate by path; the main crate is unchanged.

## Install

```bash
python3 -m venv .venv && source .venv/bin/activate
pip install maturin numpy pytest
cd ppopt_rs/python
maturin develop --release        # builds the extension and installs `rpopt` into the venv
pytest tests                     # 72 tests
```

`maturin build --release` produces a wheel (abi3, Python >= 3.9). Rust 1.85 or newer is needed (the main crate
uses edition 2024).

## Usage

```python
import numpy as np
from rpopt.mpqp_program import MPQP_Program
from rpopt.mp_solvers.solve_mpqp import solve_mpqp, mpqp_algorithm
from rpopt.point_location import BinarySearchTree, HashTable

# min 1/2 x'Qx + theta'H'x + c'x   s.t.  A x <= b + F theta,  A_t theta <= b_t   (PPOPT's convention)
prog = MPQP_Program(A, b, c, H, Q, A_t, b_t, F, equality_indices=[0, 1, 2])

sol = solve_mpqp(prog, mpqp_algorithm.geometric)      # GIL released while solving
for cr in sol.critical_regions:                         # A, b, C, d, E, f, active_set, omega_set, ...
    print(cr.active_set, cr.E.shape)

theta = np.array([[0.5], [0.5]])
x = sol.evaluate(theta)                                 # (n_x, 1) for a column theta, None if infeasible
cr = sol.get_region(theta)
assert sol.verify_solution()

# online evaluation with a point location structure
loc = BinarySearchTree(sol)
i = loc.locate(np.array([0.5, 0.5]))                    # region index, -1 if theta is outside every region
idx = loc.locate_many(np.random.uniform(-4, 4, (100000, 2)))   # int64 array, GIL released, rayon parallel
x = sol.critical_regions[i].evaluate(theta)

ht = HashTable.with_storage_limit(sol, 1 << 20)         # hash table capped at 1 MB
```

Existing PPOPT scripts can run unchanged: call `rpopt.install_as_ppopt()` first and `import ppopt...` resolves to
RPOPT (for the modules RPOPT provides). PPOPT's documentation example (`doc/mpc.rst`) is exercised that way in
`tests/test_ppopt_compat.py`; on it RPOPT and PPOPT return the same 7 regions, the same active sets and the same
`x*` at 100 sample points.

### Classes and functions

| name | notes |
|---|---|
| `MPLP_Program(A, b, c, H, A_t, b_t, F, equality_indices=None)` | processed on construction like PPOPT (equalities moved to the top, scaling, redundancy removal) |
| `MPQP_Program(A, b, c, H, Q, A_t, b_t, F, equality_indices=None)` | subclass of `MPLP_Program`; `num_x()`, `num_t()`, `num_constraints()`, `solve_theta(theta)`, `feasible_space_chebychev_ball()`, `evaluate_objective(x, theta)`, `process_constraints()` (no-op), attributes `A, b, c, H, Q, A_t, b_t, F, c_c, c_t, Q_t, equality_indices` |
| `solve_mpqp(program, algorithm=mpqp_algorithm.combinatorial, seed=0, graph_samples=100)` | `algorithm` is an `mpqp_algorithm` member or its name as a string |
| `Solution` | `critical_regions`, `program`, `evaluate`, `evaluate_objective`, `get_region`, `get_region_index`, `verify_solution`, `verify_theta`, `evaluate_many`, `len()` |
| `CriticalRegion` | `A, b, C, d, E, f, active_set, omega_set, lambda_set, regular_set, y_fixation, y_indices, x_indices`, `evaluate`, `lagrange_multipliers`, `is_inside`, `is_full_dimension`, `get_constraints` |
| `SequentialSearch`, `BinarySearchTree`, `MultiwayTree(solution, levels=3)`, `LinearMachine`, `DescriptorFunction(solution, seed=7)`, `HashTable(solution, target_cells=None)`, `HashTable.with_storage_limit(solution, max_bytes)`, `Yann` | each: `locate(theta) -> int`, `locate_many(thetas, parallel=True) -> ndarray`, `storage_floats()`, `info` |

Algorithms: `combinatorial`, `combinatorial_parallel`, `graph`, `graph_exp`, `graph_parallel`, `combinatorial_graph`,
`geometric`, `geometric_parallel` (PPOPT's), and `combinatorial_graph_parallel`, `ahmadi`, `mplcp`, `mplcp_enum`,
`mplp_lex` (RPOPT's).

## Differences from PPOPT

- Shapes follow PPOPT: `b, c, b_t` are columns, results `b, d, f` of a critical region are columns. 1-D arrays are
  accepted for vectors. `evaluate(theta)` returns a column for a 2-D (column) theta and a 1-D array for a 1-D theta.
- `solve_theta` returns a `SolverOutput` with 1-D `sol`, `slack`, `dual` and `active_set`. The solver is RPOPT's
  own LP/QP code, not Gurobi/quadprog, so there is no `solver=` argument.
- `process_constraints()` is a no-op kept for script compatibility: the constructor already runs the full
  processing (as PPOPT's does with its default `post_process=True`).
- `Solution.critical_regions` is a fixed list built once from the Rust result; adding regions with `add_region` or
  editing the matrices of a region is not supported (the regions are read-only).
- Region order differs from PPOPT's for most algorithms; compare solutions as sets of active sets.
- `locate` is an RPOPT addition (PPOPT has only the sequential `PointLocation` and a numba version); `-1` means
  "no region".
- Warnings about malformed input are issued with `warnings.warn` as in PPOPT.

## Missing relative to PPOPT

- Mixed-integer programs (`MPMILP_Program`, `MPMIQP_Program`, `solve_mpmiqp`), `mpqp_algorithm.combinatorial_parallel_exp`
  and `graph_parallel_exp` (raise `NotImplementedError`).
- `ppopt.plot` (`parametric_plot`), `ppopt.problem_generator`, `ppopt.upop`, `ppopt.geometry`, `ppopt.utils`,
  `ppopt.solver_interface`, `Solver` selection, `MPLP_Program.optimal_control_law`, `check_optimality`,
  `sample_theta_space`, `gen_optimal_active_set`, `display_warnings`/`display_latex`.
- Programs are not editable after construction (attributes are read-only copies).
- Parametric objective terms `c_c`, `c_t`, `Q_t` are readable but cannot be passed to the constructor.
