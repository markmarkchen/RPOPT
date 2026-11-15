# Explicit MPC against online QP solvers

The explicit law (RPOPT solution + hash-table point location) against two online solvers that solve
the same condensed QP at every step, on the same closed-loop trajectories:

- **DAQP** (dual active set, Arnström, Bemporad, Axehill 2022), warm-started from the previous
  working set, and cold.
- **qpOASES** (online active set strategy, Ferreau, Bock, Diehl 2008), `hotstart` from the previous
  parameter (homotopy), and a cold `init` per step. Options: `setToMPC()`.

Every solver's first input is checked against the explicit law (column `u err`).

Timing: per step, the minimum over repeated calls (explicit: 5 x 64 calls, in Rust; online: 20
warm or 5 cold calls, in C++). A warm-started call needs the previous step's state, so each
repetition re-solves the previous step untimed before timing the current one; the first step of a
trajectory is left out of the warm rows. The explicit and online numbers come from two processes,
both single-threaded, `-O3`, on the same machine.

```sh
make                       # needs DAQP and qpOASES built as static libraries, see Makefile
cargo run --release --example export_online -- data ../problems/*.json   # trajectories + explicit times
./online_bench data/*.bin
```
