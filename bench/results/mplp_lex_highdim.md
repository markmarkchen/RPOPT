# mplp_lex on mpLPs with 5 to 12 parameters

Question: is `mplp_lex` the fastest exact mpLP solver above 4 parameters, where `mplcp` loses to `graph` on mpQPs?
Answer: it beats `graph` by 3x to 30x at every size, and is fastest on 13 of 16 problems, but it loses to
`combinatorial` and `combinatorial_graph` (1.2x to 1.7x) on three problems with few regions or few facets
per variable (masses4one_N1, masses5inf_N1, masses6inf_N1). The cause is the facet-by-facet neighbour search.

## Set

`bench/problems_lp_highdim/` (16 problems, written by `bench/gen_highdim_lp.py` and `examples/gen_random --lp`):
oscillating-masses inf-norm and 1-norm MPC with 3 to 6 masses (6 to 12 parameters) at horizons N=1 to 3, and random
mpLPs (`generate_mplp`) with 5 to 12 parameters. Sized so that `mplp_lex` finishes in under 40 s.

## Step 1 (Negishi, build job 45874466, array 45874467, one 1-CPU task per problem and solver, 300 s timeout, 1 GB)

Commit a7d8300 (main c3e85cb plus the problem set). `mplp_bench --one` (median of repeated solves under 2 s,
one solve above). Correctness of `mplp_lex`: 2,000 sampled parameters per problem, 0 gaps and 0 errors on all 16.
Solvers run: `mplp_lex`, `graph`, `geometric`, `combinatorial_graph`, `combinatorial`, `mplcp_enum`. `mplcp` and
`mplcp_parallel` are routed to `mplp_lex` on mpLPs, `mplp_lex` itself is the only lexicographic code, and
`graph_exp`, `ahmadi` and `parametric_daqp` were not run. Cells: time (time relative to `graph`), `timeout` is 300 s,
`OOM (1 GB)` is the child killed at 1 GB. The region count of `graph` and `mplp_lex` differ on degenerate
problems because `graph` returns overlapping or duplicated regions there; `mplp_lex` gives the unique partition.

| problem | params | regions (lex/graph) | mplp_lex | graph | geometric | combinatorial_graph | combinatorial | mplcp_enum | gaps/errs lex |
|---|---|---|---|---|---|---|---|---|---|
| randlp_t5_x5_m40_s0 | 5 | 562/562 | 58 ms (0.06x) | 948 ms (1.00x) | 127 ms (0.13x) | 127 ms (0.13x) | 244 ms (0.26x) | 51859 ms (54.68x) | 0/0 |
| randlp_t5_x5_m40_s1 | 5 | 1563/1563 | 163 ms (0.14x) | 1133 ms (1.00x) | 453 ms (0.40x) | 277 ms (0.24x) | 499 ms (0.44x) | 59344 ms (52.36x) | 0/0 |
| masses3inf_N2 | 6 | 740/948 | 235 ms (0.03x) | 6783 ms (1.00x) | 8176 ms (1.21x) | 3554 ms (0.52x) | 657 ms (0.10x) | timeout | 0/0 |
| masses3inf_N3 | 6 | 10684/14268 | 6657 ms (0.04x) | 189855 ms (1.00x) | timeout | OOM (1 GB) | OOM (1 GB) | timeout | 0/0 |
| masses3one_N2 | 6 | 19076/- | 4127 ms | timeout | 128204 ms | OOM (1 GB) | OOM (1 GB) | timeout | 0/0 |
| randlp_t6_x5_m40_s1 | 6 | 1818/1818 | 352 ms (0.11x) | 3241 ms (1.00x) | 695 ms (0.21x) | 446 ms (0.14x) | 1212 ms (0.37x) | 84060 ms (25.94x) | 0/0 |
| masses4inf_N2 | 8 | 1974/2768 | 2398 ms (0.04x) | 67187 ms (1.00x) | 70609 ms (1.05x) | 14585 ms (0.22x) | 31928 ms (0.48x) | timeout | 0/0 |
| masses4one_N1 | 8 | 2048/2048 | 577 ms (0.03x) | 21130 ms (1.00x) | 12714 ms (0.60x) | 3983 ms (0.19x) | 468 ms (0.02x) | timeout | 0/0 |
| randlp_t8_x5_m40_s0 | 8 | 557/557 | 165 ms (0.03x) | 4879 ms (1.00x) | 326 ms (0.07x) | 168 ms (0.03x) | 879 ms (0.18x) | 193382 ms (39.64x) | 0/0 |
| masses5inf_N1 | 10 | 48/48 | 32 ms (0.04x) | 821 ms (1.00x) | 451 ms (0.55x) | 22 ms (0.03x) | 19 ms (0.02x) | 291 ms (0.35x) | 0/0 |
| masses5inf_N2 | 10 | 4190/- | 13965 ms | timeout | timeout | OOM (1 GB) | OOM (1 GB) | timeout | 0/0 |
| masses5one_N1 | 10 | 8190/- | 4157 ms | timeout | timeout | 70415 ms | 4655 ms | timeout | 0/0 |
| randlp_t10_x6_m50_s0 | 10 | 12105/- | 9423 ms | timeout | 216657 ms | 8132 ms | OOM (1 GB) | timeout | 0/0 |
| masses6inf_N1 | 12 | 52/52 | 70 ms (0.03x) | 2083 ms (1.00x) | 1142 ms (0.55x) | 49 ms (0.02x) | 42 ms (0.02x) | 583 ms (0.28x) | 0/0 |
| masses6one_N1 | 12 | 32763/- | 36158 ms | timeout | timeout | OOM (1 GB) | 50443 ms | timeout | 0/0 |
| randlp_t12_x5_m40_s0 | 12 | 29/29 | 9 ms (0.35x) | 26 ms (1.00x) | 17 ms (0.68x) | 81 ms (3.16x) | 2068 ms (80.89x) | timeout | 0/0 |

Nodes differ between tasks, so compare solvers only within a row's neighbourhood (the same row's tasks ran on
different nodes: absolute times are good to roughly 10%, 1% for serial A/A on a quiet node, more under load).

Geometric mean of `mplp_lex` time / `graph` time, over problems where `graph` finished:

| parameters | problems | mplp_lex / graph |
|---:|---:|---:|
| 5 | 2 | 0.09 |
| 6 | 3 | 0.05 |
| 8 | 3 | 0.03 |
| 10 | 1 | 0.04 |
| 12 | 2 | 0.10 |

`graph` timed out on the five largest problems, where `mplp_lex` took 4 to 36 s.

Where `mplp_lex` is not fastest (time against the best other solver):

| problem | parameters | regions | mplp_lex | best other | ratio |
|---|---:|---:|---:|---|---:|
| masses4one_N1 | 8 | 2,048 | 577 ms | combinatorial 468 ms | 1.23 |
| masses5inf_N1 | 10 | 48 | 32 ms | combinatorial 19 ms | 1.68 |
| masses6inf_N1 | 12 | 52 | 70 ms | combinatorial 42 ms | 1.67 |

(`combinatorial_graph` is second on the last two, 22 ms and 49 ms.) These are problems with 4 to 6 variables and
few regions, where enumerating active sets is cheap. `mplp_lex` still wins on the larger ones (masses5one_N1,
8,190 regions: 4.2 s against 4.7 s for `combinatorial`; masses6one_N1, 32,763 regions: 36 s against 50 s), and
`combinatorial` runs out of memory or time on the biggest random and inf-norm problems.

## Step 2: census of the three losses

`--features lp_census`, gate profile, laptop (counts are exact, times only for shares; seconds-long runs).
masses6inf_N1, 52 regions, 34 ms in `mplp_lex`:

| LP purpose | calls | time [ms] | share |
|---|---:|---:|---:|
| neighbour (one facet-centre Chebyshev LP per facet) | 1,332 | 15.5 | 46% |
| bounding box (shared region builder, 2n per region) | 1,248 | 10.3 | 30% |
| Chebyshev (region) | 132 | 0.9 | 3% |
| redundancy | 252 | 0.5 | 1% |
| rest (lex pivots, region assembly) | | about 7 | 20% |

`combinatorial` on the same problem spends the same 10 ms in bounding boxes and about 4 ms in feasibility LPs: its
whole advantage is the absence of the 1,332 facet LPs (25 per region, 52 regions found, about one new region per
25 LPs). masses5inf_N1 (1,109 neighbour LPs, 6.5 of 15 ms) and masses4one_N1 (17,416 LPs, 44 of 243 ms, 18%) look the same. Facet
deduplication already works (each facet is crossed once, from one side); the per-facet LP is the cost.

## Fix 1: facet interior point without an LP (commit 41da3ca)

The facet needs a relative-interior point theta0 only for the lexicographic data. Now the Chebyshev center of the region
is computed once per region (one LP) and its foot on each facet plane is used as theta0 when it satisfies every other
facet row with a margin of 1e-6 (relative); otherwise the facet's own Chebyshev LP runs as before. The hit rate is only
about 25% (neighbour LPs 1,332 to 1,072 on masses6inf_N1 including the 66 region-center LPs), but the gain is 4 to 18%.

- Region hashes (active sets, facet rows at 1e-9) identical to main: 24 problems (the 6 shown above and `problems_lp` N2 to N4) plus 15 more (the larger highdim problems and `problems_lp` N5, N6), all identical.
- Serial A/B on Negishi (array 45876599, build 45875582, 5 alternations per problem, base / candidate / base again in shuffled order, 1 CPU, median per-task ratio):

| problem | candidate / base | range | A/A | A/A range |
|---|---:|---|---:|---|
| masses6inf_N1 | 0.885 | 0.83-1.17 | 1.008 | 1.00-1.29 |
| masses5inf_N1 | 0.907 | 0.88-0.93 | 1.012 | 0.99-1.04 |
| masses4one_N1 | 0.844 | 0.79-0.86 | 0.986 | 0.95-1.02 |
| randlp_t12_x5_m40_s0 | 0.846 | 0.75-0.96 | 0.996 | 0.88-1.00 |
| masses3inf_N2 | 0.963 | 0.94-0.99 | 1.007 | 0.98-1.01 |
| randlp_t5_x5_m40_s0 | 0.951 | 0.89-0.97 | 0.997 | 0.95-1.04 |
| masses5one_N1 | 0.823 | 0.81-0.83 | 1.001 | 1.00-1.01 |
| randlp_t10_x6_m50_s0 | 0.935 | 0.93-0.95 | 0.990 | 0.98-1.00 |

Kept. It moves the three losses to about 1.04 (masses4one_N1), 1.5 and 1.5 against `combinatorial`: not enough to
win them.

## What remains (not done)

The neighbour step still needs a point on each facet. Removing the LP altogether needs the origin of each region
row (which nonbasic slack or multiplier produced it), so that the single lexicographic pivot of Corollary 18 can be
taken directly; the rows lose that tag in the shared region builder (`mpqp_utils.rs`, `gen_cr_from_active_set_lambda_first`),
which another teammate is changing. Proposed change for that builder: return, with each facet row of E, the index
of the basis row or slack it came from. The 2n bounding-box LPs per region (30% here) are the other large share and
are also in the shared builder.
