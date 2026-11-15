# mplcp exploration order: does it drive the Case-2 cost?

Data: Negishi, 1-CPU tasks, scratch-instrumented builds (no trace code in the repo), from the
`worktree-agent-a2cd0480ac0fec818` g/ directory. Builds: base = main c345cdc, cand = 17f22f4 (first scale-free commit),
cand3 = 04d4ca9 (fix). Case numbers: 0 none, 1 fast, 2 rank-1, 3 walk, 4 certificate skip. "All-known" = every returned
neighbour was already in Seen (calls that returned nothing are counted as not all-known).

## Canonical order A/B (candidate list of each region sorted by active set; median of 3 tasks, ms)
| problem | base | base+C | cand | cand+C | cand3 | cand3+C |
|---|---:|---:|---:|---:|---:|---:|
| masses8_N2 | 1598 | 1813 | 2351 | 2675 | 1191 | 1354 |
| masses4_N3 | 3806 | 3911 | 2358 | 2416 | 1978 | 2099 |
| masses5_N3 | 30544 | 32656 | 13218 | 14012 | 11653 | 12798 |
| masses4_N4 | 69.7 s | 71.7 s | 39.1 s | n/a | 39.3 s | 39.3 s |

Canonical order makes every build 0% to 14% slower and leaves the cand/base ratio unchanged on masses8_N2
(1.47 without, 1.48 with). Order is not the cause of the masses8_N2 swing.

## Trace, one solve each (walk = case 3; ms)
| problem | build | walk calls | walk ms | all-known walk calls | all-known walk ms | share of walk ms | share of solve |
|---|---|---:|---:|---:|---:|---:|---:|
| masses8_N2 | base | 530 | 653.5 | 288 | 98.6 | 15% | 5.4% |
| masses8_N2 | cand | 748 | 1505.6 | 530 | 83.8 | 6% | 3.1% |
| masses8_N2 | cand3 | 308 | 208.2 | 226 | 32.8 | 16% | 2.3% |
| masses4_N3 | base | 6069 | 2325 | 4346 | 502 | 22% | 12.8% |
| masses4_N3 | cand3 | 3517 | 595 | 2514 | 74.7 | 13% | 3.6% |
| masses5_N3 | base | 21173 | 25424 | 15409 | 2802 | 11% | 8.6% |
| masses5_N3 | cand3 | 14478 | 5915 | 10134 | 568.7 | 10% | 4.5% |
| masses4_N4 | base | 49903 | 55458 | 37528 | 8055 | 15% | 11.6% |
| masses4_N4 | cand3 | 32534 | 26303 | 23527 | 1023.5 | 4% | 2.6% |

Fast all-known calls cost under 0.5% of the solve. Rank-1 calls never return an all-known set in this trace.

First divergence of the discovery order (masses8_N2, 395 regions, identical region sets): base vs cand and base vs cand3
at region 11; cand vs cand3 at region 201. masses4_N3: base vs cand at region 12, cand vs cand3 at region 282.
