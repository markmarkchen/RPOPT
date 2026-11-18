# Real problems for RPOPT benchmarking

Two applications from our group. Every problem is stored as RPOPT JSON, with the same keys as `bench/problems/*.json`:

    min_x  1/2 x'Qx + (c + H theta)'x
    s.t.   A x <= b + F theta,   A_t theta <= b_t

None of them have been solved with RPOPT yet.

## 1. SIS18 synchrotron beam control: mpQP (`sis18_beam_mpqp/`)

This is explicit offset-free MPC for the SIS18 orbit feedback, from Faulwasser, Lens and Kellett. At horizon N, the MPC problem splits into two independent mpQPs: a "mean" channel and a "shape" channel. The parameter set is a box.

| file | x | constraints | theta | known regions |
|---|---|---|---|---|
| `sis18_N30_mean.json`  | 31 | 181 | 3 | **22,774** (our own facet-crossing solver) |
| `sis18_N30_shape.json` | 30 | 120 | 3 | 3,642 (our own solver) |
| `sis18_N13_mean.json`  | 14 |  79 | 3 | 4,347 (our solver), 4,349 (PPOPT `graph_parallel_exp`, 2,173 s on 10 cores) |
| `sis18_N13_shape.json` | 13 |  52 | 4 | not recorded |

`sis18_N30_mean` is the largest. PPOPT did not finish on the N=30 problems.

## 2. Waste-water treatment with a ReLU network model: mp-MIQP (`wastewater_nn_mpmiqp/`)

This is MPC of the BSM1 waste-water plant. The plant model is a ReLU neural network, so every hidden neuron adds one binary variable through a big-M formulation. In the published paper this MIQP was solved online with CPLEX (`cplexmiqp`) at every step. It has never been solved explicitly.

| file | continuous | binary | constraints | theta |
|---|---|---|---|---|
| `ww_nn9_feb16.json` | 18 | 12 | 78 | 9 |
| `ww_nn9_feb15.json` | 24 | 16 | 104 | 9 |

`ww_nn9_feb16` is the problem from the paper: Ramesh, Brahmbhatt, Avraamidou and Ganesh, *J. Process Control* 162 (2026) 103717.
- The network is the same: one hidden layer of 4 ReLU neurons, with inputs [S_NO2, S_O5, S_NH, Qa, KLa5, Qin] and 3 outputs.
- The horizon is the same: Np = Nc = 3, so 4 × 3 = 12 binaries and 2 × 3 inputs + 4 × 3 hidden variables = 18 continuous.
- All 8 big-M bounds match the paper's OBBT values exactly.
- Theta is 9-dimensional: the 3 outputs, the 3 set points, the 2 previous inputs and the inflow Qin.

`ww_nn9_feb15` is a larger draft with 16 binaries, which fits 4 neurons over a horizon of 4.

- There is one extra key, `binary_indices`: the binaries are the last variables (indices 18..29 and 24..39). `A` is the POP `[A E]`.
- The parameter set is the POP `CRA theta <= CRb`, which is a box of plus or minus 1 in scaled units. Each theta holds the 3 states and outputs, the set points, the 2 previous inputs and the inflow disturbance.
- The continuous block of Q is only positive semidefinite: 3 of its 18 eigenvalues are zero (4 of 24 for feb15), because the hidden-layer variables carry no cost. A solver that needs Q strictly positive definite will need a small regularisation.
- The theta-only terms `Qt`, `ct` and `cc` are left out. The original POP `.mat` files are included.
- Suggested solvers: `solve_mpmiqp_exact` (Oberdieck–Pistikopoulos), with `solve_mpmiqp_enumeration` (MITree) as a cross-check.
- Expect it to be hard: 9 parameters, and up to 4,096 or 65,536 binary combinations before the feasibility screen.
