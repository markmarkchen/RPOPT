# Re-run on a second machine (Apple M3 Pro, 36 GB), November 2026

The README numbers come from an Apple M4 Max. These are the same benchmarks run independently on an
Apple M3 Pro (36 GB), with PPOPT 1.6.12 from PyPI and Gurobi 13. They are raw outputs, kept for comparison.

| file | what |
|---|---|
| `rust.json`, `rust_log.txt` | `ppopt_bench` on `bench/problems` (main at 9eef9da) |
| `ppopt.json`, `ppopt_log.txt` | `bench/bench_ppopt.py` on the same problems |
| `speedup.md` | `bench/compare.py` on the two files above |
| `pop_reference.json` | region counts and times stored in the POP v1.5 `.mat` files (67 mpQP problems) |
| `pop_fixed.json`, `pop_fixed_log.txt` | `ppopt_bench` on the 67 POP problems with `fix/duplicate-constraint-rows` |
| `pop_fixed2.json`, `pop_fixed2_log.txt` | the 14 that still differed, re-run with `fix/weakly-active-duplicates` |
| `online_bench.txt`, `online_export_log.txt` | explicit law vs DAQP / qpOASES (`bench/online-qp` branch) |

Summary:

- **RPOPT vs PPOPT:** region counts are identical on every problem. The single-thread speed-up is
  393–3,543× for geometric, which is lower than the README's M4 Max numbers. Parallel vs parallel the median
  is 613×, against 2,063× in the README; PPOPT's parallel times here were about 3× faster than the
  README's.
- **POP:** 33 of 67 problems match POP's region count exactly. On 20, every RPOPT algorithm agrees
  on a few more regions than POP (+0.2–10%). On pop_042, POP stores 3 regions and RPOPT finds 2; a
  grid check confirms 2. On 13, the RPOPT algorithms differ from each other by 1–3 regions (thin
  regions; pop_045 has small overlaps).
- **Online:**
  - The explicit law takes 7–16 ns per step (81–114 ns at 6–8 parameters).
  - DAQP with a warm start takes 0.17–2.4 µs; qpOASES hotstart takes 0.8–9 µs.
  - Every first input agrees to 1.4e-9 or better, with no failures.
  - The hash table uses up to 52 MB. Building it for masses4_N3 (8 parameters) took about 14 hours.
