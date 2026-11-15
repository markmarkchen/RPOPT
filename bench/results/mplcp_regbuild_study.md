# mplcp region build: measurements for a dedicated builder (phase 1)

Branch `worktree-agent-a8095c6d163dbc0a4` on main ddf0aa8, with the counters of `census_diag` (`RPOPT_DIAG=1`, feature `lp_census`, gate profile). One census solve per problem on Negishi (SLURM jobs 45909313 build, 45909314 run, 1 CPU each). Counts only, no decision changed. Times are census times (two clock reads per LP) with the diagnostic time removed. The Clarkson columns are a simulation inside the build (its LPs run under purpose `other`; its verdicts are compared with the real ones).

Row classes in the present LP path: propagated parameter box (free), exact box (2n LPs), box vertex, projection test, per-row LP. 'facets' = rows kept before duplicate removal.

### Build study 1: time split of the mplcp region build (diagnostic time removed)

| problem | regions | solve ms | build share | law | rows (KKT + normalise) | Chebyshev stage | gen_cr_from_prepared | of which box LPs | per-row LPs | tab rows (diag) us/region | KKT rows us/region |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rand_t8_x5_m20_s0 | 4111 | 753.5 | 73.7% | 0.7% | 1.3% | 4.5% | 66.6% | 50.1% | 9.1% | 1.90 | 2.38 |
| rand_t10_x5_m20_s0 | 6124 | 1835.9 | 74.2% | 1.0% | 1.0% | 3.8% | 68.4% | 57.7% | 4.8% | 0.43 | 3.01 |
| pop_017 | 4142 | 600.8 | 63.2% | 1.3% | 1.8% | 4.2% | 55.7% | 31.1% | 14.1% | 2.22 | 2.57 |
| rand_t12_x5_m20_s0 | 6143 | 4012.9 | 54.1% | 0.3% | 0.5% | 2.6% | 50.5% | 45.0% | 1.9% | 2.46 | 3.49 |
| masses4_N3 | 2947 | 3111.4 | 36.9% | 0.7% | 0.5% | 2.9% | 33.3% | 18.9% | 11.9% | 1.86 | 4.76 |

### Build study 2: what decides each row (per region), and the entry facet

| problem | rows | facets | propagated box | exact box | box vertex | projection | LP kept | LP dropped | children with entry | entry located | entry by box vertex | by projection | by LP (saved LP) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rand_t8_x5_m20_s0 | 38.0 | 21.8 | 2.0 | 10.9 | 20.0 | 0.1 | 1.8 | 3.3 | 79.6% | 100.0% | 93.7% | 0.2% | 6.1% |
| rand_t10_x5_m20_s0 | 43.0 | 29.9 | 3.0 | 9.2 | 27.8 | 0.0 | 2.0 | 0.9 | 14.6% | 100.0% | 95.2% | 0.0% | 4.8% |
| pop_017 | 36.0 | 14.3 | 3.5 | 8.1 | 13.4 | 0.1 | 0.8 | 10.1 | 71.6% | 100.0% | 96.6% | 0.7% | 2.7% |
| rand_t12_x5_m20_s0 | 48.0 | 39.1 | 4.0 | 4.7 | 37.1 | 0.1 | 1.9 | 0.2 | 78.9% | 100.0% | 97.2% | 0.2% | 2.7% |
| masses4_N3 | 67.4 | 25.3 | 3.3 | 16.8 | 23.2 | 0.1 | 2.0 | 21.9 | 34.4% | 100.0% | 95.0% | 0.1% | 4.7% |

### Build study 3: parent verdicts as hints

| problem | children with parent | rows with counterpart | agree | parent kept, child kept | parent kept, child dropped | parent dropped, child kept | parent dropped, child dropped | rows at LP stage with counterpart (per region) | agree | LP-dropped where parent dropped | LP-kept where parent kept |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rand_t8_x5_m20_s0 | 99.5% | 36.8 | 85.3% | 52.2% | 11.0% | 3.7% | 33.1% | 5.05 | 65.2% | 52.9% | 84.5% |
| rand_t10_x5_m20_s0 | 99.6% | 41.1 | 86.0% | 62.5% | 8.4% | 5.6% | 23.5% | 2.78 | 70.1% | 23.9% | 85.2% |
| pop_017 | 99.5% | 34.7 | 83.2% | 34.1% | 13.3% | 3.6% | 49.1% | 10.78 | 72.1% | 71.2% | 73.7% |
| rand_t12_x5_m20_s0 | 99.7% | 46.7 | 92.7% | 78.7% | 5.2% | 2.1% | 13.9% | 2.01 | 91.6% | 22.8% | 95.8% |
| masses4_N3 | 98.7% | 65.1 | 75.9% | 32.5% | 21.1% | 3.0% | 43.4% | 23.72 | 66.6% | 64.2% | 75.6% |

### Build study 4: the bounding box (child against parent's exact box, and rows an outer box decides)

| problem | box bounds equal to parent's | parent box points feasible in child | of which optimal for the child | vertex-kept rows hit by a feasible parent point | propagated box (6 rounds) rejects /region | 30 rounds rejects /region | exact box rejects /region |
|---|---:|---:|---:|---:|---:|---:|---:|
| rand_t8_x5_m20_s0 | 33.4% | 28.8% | 13.8% | 56.7% | 2.0 | 2.0 | 10.9 |
| rand_t10_x5_m20_s0 | 46.2% | 24.1% | 12.6% | 49.6% | 3.0 | 3.0 | 9.2 |
| pop_017 | 14.8% | 24.1% | 8.9% | 53.0% | 3.5 | 3.5 | 8.1 |
| rand_t12_x5_m20_s0 | 77.0% | 19.1% | 15.1% | 51.3% | 4.0 | 4.0 | 4.7 |
| masses4_N3 | 30.7% | 14.3% | 6.9% | 35.9% | 3.3 | 3.3 | 16.8 |

### Build study 5: tableau rows against KKT rows, hints, and a Clarkson-style redundancy removal (per region)

| problem | regions compared | max row difference above 1e-10 | 1e-8 | 1e-6 | 1e-4 | rows missing | hint margin > 0 | > 1e-9 | > 1e-6 | > 1e-4 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rand_t8_x5_m20_s0 | 3272 | 0.0% | 0.0% | 0.0% | 0.0% | 0.0% | 29.7% | 0.0% | 0.0% | 0.0% |
| rand_t10_x5_m20_s0 | 895 | 0.0% | 0.0% | 0.0% | 0.0% | 0.0% | 49.0% | 0.0% | 0.0% | 0.0% |
| pop_017 | 2965 | 3.0% | 0.0% | 0.0% | 0.0% | 0.0% | 26.1% | 0.0% | 0.0% | 0.0% |
| rand_t12_x5_m20_s0 | 4844 | 2.4% | 0.0% | 0.0% | 0.0% | 0.0% | 28.2% | 0.1% | 0.1% | 0.0% |
| masses4_N3 | 1013 | 5.1% | 1.4% | 1.0% | 1.0% | 0.0% | 31.6% | 0.2% | 0.0% | 0.0% |

### Build study 6: Clarkson-style removal, per region, against the present LPs (A seed = parameter-set rows, B = A + entry + parent's kept rows)

| problem | present: box LPs | per-row LPs | Chebyshev | present LP rows (live) | A: LPs | avg LP rows | rays | decided by ray alone | LP time us | ray time us | final S | verdict mismatches | B: LPs | avg LP rows | rays | LP time us | final S | mismatches | present LP time us (box+row+Cheb) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rand_t8_x5_m20_s0 | 16.0 | 5.1 | 1 | 36.0 | 37.7 | 27.1 | 48.6 | 12.4 | 171.0 | 29.4 | 29.6 | 0 | 29.1 | 33.8 | 38.8 | 154.2 | 33.8 | 0 | 116.1 |
| rand_t10_x5_m20_s0 | 20.0 | 3.0 | 1 | 40.0 | 42.1 | 32.7 | 55.1 | 15.3 | 298.7 | 41.9 | 36.0 | 0 | 32.4 | 39.1 | 44.4 | 258.9 | 39.3 | 0 | 198.2 |
| pop_017 | 12.0 | 10.8 | 1 | 32.5 | 35.9 | 16.4 | 44.9 | 9.3 | 82.3 | 24.4 | 17.9 | 530 | 29.7 | 24.1 | 37.7 | 90.2 | 24.0 | 997 | 71.0 |
| rand_t12_x5_m20_s0 | 24.0 | 2.1 | 1 | 44.0 | 46.1 | 37.5 | 61.4 | 17.8 | 464.0 | 56.2 | 41.6 | 1 | 34.3 | 43.9 | 48.5 | 383.3 | 43.9 | 0 | 322.3 |
| masses4_N3 | 16.0 | 23.9 | 1 | 64.0 | 70.4 | 30.3 | 84.2 | 11.4 | 399.2 | 70.8 | 33.4 | 301 | 62.9 | 46.3 | 73.3 | 564.0 | 46.6 | 376 | 388.4 |

### Build study 7: Clarkson-style removal with a ray phase first (rays along every row normal and +-e_k, then LPs for the rest), per region

| problem | LPs | avg LP rows | rays | LP time us | ray time us | final S | mismatches |
|---|---:|---:|---:|---:|---:|---:|---:|
| rand_t8_x5_m20_s0 | 31.2 | 28.6 | 86.5 | 144.8 | 38.1 | 29.6 | 0 |
| rand_t10_x5_m20_s0 | 34.9 | 34.4 | 99.2 | 261.2 | 58.1 | 36.0 | 0 |
| pop_017 | 29.8 | 17.7 | 79.6 | 69.5 | 32.5 | 17.9 | 530 |
| rand_t12_x5_m20_s0 | 38.6 | 39.3 | 111.8 | 414.2 | 83.5 | 41.6 | 0 |
| masses4_N3 | 64.1 | 31.2 | 154.4 | 369.3 | 103.4 | 33.4 | 301 |

Open item (not investigated): in study 5, 26% to 49% of the KKT rows found no counterpart among the rows read from the parent tableau by one pivot, while the rows that did match agree exactly. Suspicion: children whose active set is more than one toggle from the parent (candidates of the Case-2 walk carry a piv of the last pivot only), so the one-pivot rows describe a different basis. Not checked.
