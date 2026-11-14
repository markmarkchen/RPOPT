# LP census: where the LP solves go in RPOPT

Measurement only (no algorithm changed). Tables below are the raw output of
`cargo run --release --features lp_census --example lp_census -- --detail` from `ppopt_rs/`
(baseline 778cdb9 plus the census hooks, single thread, release profile).

Method notes
- Each pair ran in a child process; if the first solve took under 5 s it was solved three times and the run
  with the median solve time is reported. Counts are exact. Times are noisy (machine at load 50 to 100):
  a single descheduled LP call shows up in a purpose's time, for example chebyshev in graph on masses2_N6
  (35 ms for 901 calls, against 2 ms in the sibling runs). The census adds two clock reads per call.
- di_N10 has 11 regions and solves in under 1 ms, so its time shares are not meaningful. Use masses2_N6 and
  sys3x2_N5 (about 900 regions each) and diinf_N4/N6 (mpLP).
- combinatorial on diinf_N4 did not finish in 60 s (skipped); combinatorial on di_N10 took 49 s.
- Purposes: redundancy = one LP per tested row while building a region (the per-row tests in
  `gen_cr_from_kkt`, `gen_cr_from_law_hinted`, `lp_flags`); feasibility = candidate active set (witness LPs,
  lambda/omega rows, KKT region nonempty, `check_feasibility`); neighbour = facet centers (one Chebyshev LP per
  facet), the facet-step solve at a test point (QP or LP), facet completion, mpLP lex facets, mplcp adjacency;
  chebyshev = Chebyshev ball, start point and empty-region test of a region build, `is_full_dimension`;
  bbox = region bounding box (2n LPs) and theta box; init = start-up and sampling. QPs are counted in the same purposes.
- "Region construction" = wall time inside `gen_cr_from_kkt`, `gen_cr_from_law_hinted` and the 1-D builder.
  The control-law computation done by the caller before the call is not included, so this slightly
  understates construction.
- "build dup" = candidate active sets discarded because already seen (facet step hitting an indexed set,
  `attempted` hits, `excluded` hits). No region build is wasted on a duplicate, and every build is kept
  ("build empty" is 0) except in combinatorial, where the empties are the empty-region tests.

## Verdict: can shared region-construction improvements reach 1.5x?

Share of solve time in region construction, and Amdahl bound 1/(1 - f*cut) if the LPs inside construction are
cut by 50%, 80% and 100% (f = construction LP time / total solve time). Values are masses2_N6 / sys3x2_N5 (QP)
or diinf_N4 / diinf_N6 (mpLP). The last column is construction removed entirely.

| algorithm | construction share | of which LP | x at -50% | x at -80% | x at -100% | x if construction free | 1.5x from shared construction work? |
|---|---|---|---|---|---|---|---|
| geometric (mpQP) | 51% / 12% | 39% / 9% | 1.24 / 1.05 | 1.45 / 1.08 | 1.63 / 1.10 | 2.03 / 1.13 | NO (sys3x2 cannot; masses2 needs every construction LP gone) |
| geometric (mpLP) | 11% / 8% | 6% / 5% | 1.03 / 1.02 | 1.05 / 1.04 | 1.07 / 1.05 | 1.12 / 1.08 | NO |
| graph | 22% / 14% | 21% / 13% | 1.12 / 1.07 | 1.20 / 1.12 | 1.26 / 1.15 | 1.28 / 1.16 | NO |
| graph_exp | 60% / 6% | 58% / 6% | 1.41 / 1.03 | 1.87 / 1.05 | 2.38 / 1.06 | 2.48 / 1.07 | NO (masses2 alone looks yes, but its timing is noisy, see note; sys3x2 is 6%) |
| combinatorial_graph | 22% / 5% | 22% / 5% | 1.12 / 1.02 | 1.21 / 1.04 | 1.28 / 1.05 | 1.29 / 1.05 | NO |
| combinatorial | 20% / 1% | 8% / 0% | 1.04 / 1.00 | 1.07 / 1.00 | 1.08 / 1.00 | 1.24 / 1.01 | NO |
| mplcp | 56% / 62% | 1% / 8% | 1.01 / 1.04 | 1.01 / 1.06 | 1.01 / 1.08 | 2.28 / 2.62 | NO via LPs (LP is 1 to 8% of construction); the 44 to 55% non-LP part of construction is where its time goes |

Verdict: shared LP-only region-construction work cannot give every algorithm 1.5x. Only graph_exp on masses2_N6
clears it, and that figure rests on a bbox/redundancy time that varies 3.5x between identical-count runs
(graph_exp bbox 71.7 ms vs graph bbox 20.1 ms for the same 7208 LPs). mplcp is the exception the rule already names.

## Findings per algorithm (time shares are of total solve time)

Upper bound = 1/(1 - f) if one lever removed all of its LPs (and QPs of that purpose).

- geometric, mpQP: LP+QP is 77% (masses2_N6) and 44% (sys3x2_N5) of solve time. Neighbour search is the largest
  piece: 38% and 35% (x1.62, x1.53), of which the 7346 (5510) facet-step QPs are 36% (17%) and the facet-center
  Chebyshev LPs the rest. Redundancy 19% / 2%, bbox 19% / 5%, chebyshev 1% / 2%. 69% of facet steps end on an
  already indexed active set (5088 of 7346 steps on masses2_N6 for 901 regions). Only the neighbour lever can
  reach 1.5x, and only if it removes essentially all facet steps and facet-center LPs.
- geometric, mpLP (diinf_N4/N6): LP is 52% / 47% of solve time; neighbour is 45% / 42% (x1.80 / x1.72); everything
  else is under 3% each. 74% of the LPs are neighbour LPs (1690 for 39 regions). The neighbour lever is the only one.
- graph: LP is only 29% (masses2) and 23% (sys3x2) of solve time, so at least 70% is non-LP work. By purpose:
  feasibility 9% / 10%, chebyshev 11% / 0%, bbox 6% / 12%, redundancy 3% / 1%. The best single lever bounds out at
  x1.12 to x1.14, so graph cannot reach 1.5x from LP reduction alone. The 70%+ is not LP: it is rank tests,
  control-law (Schur) computation, region row assembly, candidate generation and hashing (not measured by this census).
- graph_exp: LP is 65% on masses2_N6 but 21% on sys3x2_N5. masses2: redundancy 20%, bbox 25%, chebyshev 13%
  (noisy). sys3x2: feasibility 15% (x1.17), everything else under 5%. Cannot reach 1.5x on sys3x2 from LPs.
- combinatorial_graph: LP is 42% / 17%. Feasibility is 20% / 12% (x1.25 / x1.14) although it is 86% / 96% of the LP
  count; redundancy 11% / 3%, bbox 10% / 2%. It generates 343,882 (1,237,377) duplicate neighbour entries for
  901 (973) regions, which is non-LP work. Cannot reach 1.5x from LP reduction.
- combinatorial: LP is 42% / 40% (di_N10: 19%) of solve time; feasibility is 34% / 40% (x1.51 / x1.65) and is
  86% to 100% of the LPs. 57% to 80% of those LPs come back infeasible (167,782 of 296,437 on masses2_N6; 5.36M
  of 6.74M on di_N10), after the pruning list already removed 84% of the candidates (1.06M of 1.26M generated on
  masses2_N6; 26M of 33M on di_N10). about 60% of the time is enumeration, rank tests and region work. Removing
  every feasibility LP gives x1.51 to x1.65 at best; at an 80% cut it is x1.37 / x1.47 (masses2 / sys3x2). So it
  cannot reach 1.5x from LP reduction alone unless Farkas pruning removes almost all of them.
- mplcp: LP is 2% (masses2_N6), 9% (sys3x2_N5), 15% (di_N10) of solve time; construction is 56% / 62% of time, of
  which the non-LP part is 55% / 54%. LP reduction gives at most x1.10. What dominates is non-LP work inside
  region construction (row normalisation, `select_rows`, box propagation and vertex clipping: 63 rows per
  build with 75% to 100% decided without an LP) plus the pLCP pivoting itself.
- mplp_lex (mpLP): LP is 2% on diinf_N4 and 0% on diinf_N6 (701 ms for 58 regions, 1657 LPs). LP reduction is
  irrelevant; the lexicographic pivoting dominates.

## Which of the five LP-reduction levers removes the most LPs?

Counts for masses2_N6 (901 regions) unless stated; time bounds are Amdahl on total solve time.

1. Vertex-based region construction (replaces redundancy + bbox + Chebyshev LPs of a region build): removes
   23,434 of 31,611 LPs in geometric (74%), 22,530 of 48,903 in graph (46%), 22,534 of 157,087 in
   combinatorial_graph (14%), about 46,000 of 342,759 in combinatorial (13%, mostly the 22,888 empty-region
   tests). Time bound at most x1.63 (geometric masses2), x1.26 (graph), x1.28 (combinatorial_graph), x1.08
   (combinatorial), x1.10 (geometric sys3x2).
2. Cheap redundancy pre-screens: existing cheap tests already decide 75% (masses2) to 83% (sys3x2) of the 63 / 50
   rows entering per region without an LP, and 66% to 71% for mpLP. 14,424 rows still need an LP, 11,196 (78%)
   prove redundant; kept rows are 8.9 per region on masses2_N6, 6.2 on sys3x2_N5, 10 on mpLP. Ceiling: 14,424
   LPs (46% of geometric LPs) but only 19% of its time (x1.23) and 3% to 4% for the other algorithms.
3. Neighbour search by inspection: only geometric uses it. It is 26% of geometric LPs plus all 7,346 facet-step
   QPs, and 35% to 45% of its time (x1.53 to x1.80). This is the one lever that can reach 1.5x for geometric.
4. Farkas-certificate pruning: feasibility LPs are 54% to 86% of graph-type LPs and 86% to 100% of combinatorial
   LPs, and most come back infeasible (graph on sys3x2_N5: 57,246 of 73,040). Largest LP count by far, but time
   bound is x1.09 to x1.17 (graph, graph_exp), x1.14 to x1.25 (combinatorial_graph), x1.51 to x1.65
   (combinatorial).
5. Avoiding duplicate region builds: nothing to save. Duplicates are caught before the build (facet step sees an
   indexed set, `attempted`/`excluded` hits), every region build in the graph-type algorithms is kept (901 builds
   for 901 regions), and the only wasted builds are combinatorial's 22,888 empty-region tests (about 25,000
   Chebyshev-purpose LPs, 3% of time). Duplicate handling costs matter only as non-LP work in combinatorial_graph.

Ranking by LPs removed: Farkas pruning (combinatorial and graph types) > vertex-based construction (geometric,
graph) > redundancy pre-screens > neighbour inspection > duplicate builds. Ranking by time bound: neighbour
inspection (geometric, x1.5 to x1.8) > Farkas pruning on combinatorial (x1.5 to x1.65) > vertex-based construction
(geometric masses2 only, x1.6) > the rest (x1.05 to x1.3).

## Algorithms that cannot reach 1.5x from LP reduction alone

graph (LP 23% to 29% of time), graph_exp on sys3x2 (21%), combinatorial_graph (17% to 42%), mplcp (2% to 15%) and
mplp_lex (0% to 2%). Combinatorial is borderline only if Farkas pruning removes nearly all feasibility LPs.
What dominates instead: for graph and graph_exp, rank tests, control-law and row assembly, candidate set
generation and hashing; for combinatorial_graph, set enumeration (1.2M neighbour entries on sys3x2_N5) and the
same row assembly; for combinatorial, enumeration of tens of millions of candidates plus rank tests; for mplcp and
mplp_lex, the pivoting and the non-LP half of region construction.

# Raw tables
### Overview

| problem | algorithm | regions | solve ms | LPs | LPs/region | QPs | LP time / solve | build kept | build dup | build empty |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| di_N10 | geometric | 11 | 0.2 | 216 | 19.6 | 35 | 69% | 11 | 10 | 0 |
| di_N10 | graph | 11 | 0.4 | 389 | 35.4 | 100 | 48% | 11 | 1 | 0 |
| di_N10 | graph_exp | 11 | 0.3 | 389 | 35.4 | 100 | 51% | 11 | 1 | 0 |
| di_N10 | combinatorial_graph | 11 | 0.8 | 921 | 83.7 | 1 | 28% | 11 | 669 | 0 |
| di_N10 | mplcp | 11 | 0.2 | 21 | 1.9 | 1 | 15% | 11 | 0 | 0 |
| di_N10 | combinatorial | 11 | 49326.2 | 6739489 | 612680.8 | 0 | 19% | 11 | 0 | 202 |
| masses2_N6 | geometric | 901 | 349.9 | 31611 | 35.1 | 7347 | 77% | 901 | 5088 | 0 |
| masses2_N6 | graph | 901 | 318.3 | 48903 | 54.3 | 100 | 29% | 901 | 2202 | 0 |
| masses2_N6 | graph_exp | 901 | 286.0 | 52961 | 58.8 | 100 | 65% | 901 | 2580 | 0 |
| masses2_N6 | combinatorial_graph | 901 | 855.2 | 157087 | 174.3 | 1 | 42% | 901 | 343882 | 0 |
| masses2_N6 | mplcp | 901 | 223.3 | 4469 | 5.0 | 1 | 2% | 901 | 0 | 0 |
| masses2_N6 | combinatorial | 901 | 2074.3 | 342759 | 380.4 | 0 | 42% | 901 | 0 | 22888 |
| sys3x2_N5 | geometric | 973 | 158.5 | 22179 | 22.8 | 5511 | 44% | 973 | 4252 | 0 |
| sys3x2_N5 | graph | 973 | 611.1 | 88226 | 90.7 | 100 | 23% | 973 | 1657 | 0 |
| sys3x2_N5 | graph_exp | 973 | 647.7 | 111492 | 114.6 | 100 | 21% | 973 | 1702 | 0 |
| sys3x2_N5 | combinatorial_graph | 973 | 2137.6 | 358390 | 368.3 | 1 | 17% | 973 | 1237377 | 0 |
| sys3x2_N5 | mplcp | 973 | 16.8 | 1515 | 1.6 | 1 | 9% | 973 | 0 | 0 |
| sys3x2_N5 | combinatorial | 973 | 47091.1 | 7543221 | 7752.5 | 0 | 40% | 973 | 0 | 21180 |
| diinf_N4 | geometric | 39 | 3.3 | 2284 | 58.6 | 0 | 52% | 39 | 76 | 0 |
| diinf_N4 | mplp_lex | 39 | 22.7 | 982 | 25.2 | 0 | 2% | 39 | 0 | 0 |
| diinf_N6 | geometric | 58 | 6.1 | 3856 | 66.5 | 0 | 47% | 58 | 135 | 0 |
| diinf_N6 | mplp_lex | 58 | 701.1 | 1657 | 28.6 | 0 | 0% | 58 | 0 | 0 |
| diinf_N4 | combinatorial | > 60 s, skipped | | | | | | | | |

### Share of LP count (and of LP time) by purpose

| problem | algorithm | redundancy | feasibility | neighbour | chebyshev | bbox | init | other |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| di_N10 | geometric | 45% (31%) | - | 24% (22%) | 10% (14%) | 20% (25%) | 0% (9%) | - |
| di_N10 | graph | 25% (30%) | 61% (24%) | - | 3% (13%) | 11% (26%) | 0% (7%) | - |
| di_N10 | graph_exp | 25% (31%) | 61% (25%) | - | 3% (11%) | 11% (25%) | 0% (8%) | - |
| di_N10 | combinatorial_graph | 11% (20%) | 83% (51%) | - | 1% (7%) | 5% (16%) | 0% (5%) | - |
| di_N10 | mplcp | 14% (5%) | - | - | 57% (48%) | 19% (12%) | 5% (34%) | 5% (2%) |
| di_N10 | combinatorial | 0% (0%) | 100% (100%) | - | 0% (0%) | 0% (0%) | - | - |
| masses2_N6 | geometric | 46% (46%) | - | 26% (4%) | 6% (2%) | 23% (47%) | 0% (0%) | - |
| masses2_N6 | graph | 29% (11%) | 54% (29%) | - | 2% (38%) | 15% (22%) | 0% (0%) | - |
| masses2_N6 | graph_exp | 27% (30%) | 57% (11%) | - | 2% (20%) | 14% (38%) | 0% (0%) | - |
| masses2_N6 | combinatorial_graph | 9% (27%) | 86% (48%) | - | 1% (1%) | 5% (24%) | 0% (0%) | - |
| masses2_N6 | mplcp | 1% (0%) | - | 79% (50%) | 20% (49%) | 0% (0%) | 0% (0%) | 0% (0%) |
| masses2_N6 | combinatorial | 4% (9%) | 86% (81%) | - | 7% (8%) | 2% (1%) | - | - |
| sys3x2_N5 | geometric | 38% (9%) | - | 27% (65%) | 9% (9%) | 26% (18%) | 0% (0%) | - |
| sys3x2_N5 | graph | 9% (3%) | 83% (44%) | - | 1% (1%) | 7% (52%) | 0% (0%) | - |
| sys3x2_N5 | graph_exp | 8% (21%) | 86% (72%) | - | 1% (1%) | 5% (6%) | 0% (0%) | - |
| sys3x2_N5 | combinatorial_graph | 2% (16%) | 96% (73%) | - | 0% (0%) | 2% (11%) | 0% (0%) | - |
| sys3x2_N5 | mplcp | 2% (1%) | - | 32% (13%) | 64% (85%) | 1% (1%) | 0% (0%) | 0% (0%) |
| sys3x2_N5 | combinatorial | 0% (0%) | 100% (99%) | - | 0% (1%) | 0% (0%) | - | - |
| diinf_N4 | geometric | 16% (5%) | - | 74% (86%) | 3% (2%) | 7% (4%) | 0% (1%) | - |
| diinf_N4 | mplp_lex | 37% (24%) | 4% (1%) | 36% (39%) | 8% (14%) | 16% (20%) | 0% (4%) | - |
| diinf_N6 | geometric | 17% (5%) | - | 74% (89%) | 3% (2%) | 6% (3%) | 0% (1%) | - |
| diinf_N6 | mplp_lex | 41% (27%) | 4% (1%) | 35% (39%) | 7% (12%) | 14% (19%) | 0% (2%) | - |

### Share of total solve time by purpose (LP+QP time) and Amdahl bound 1/(1-f)

| problem | algorithm | LP+QP / solve | redundancy | feasibility | neighbour | chebyshev | bbox | init | other |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| di_N10 | geometric | 69% | 13% (x1.14) | - | 37% (x1.59) | 6% (x1.06) | 10% (x1.11) | 4% (x1.04) | - |
| di_N10 | graph | 48% | 7% (x1.07) | 5% (x1.06) | - | 3% (x1.03) | 6% (x1.06) | 27% (x1.36) | - |
| di_N10 | graph_exp | 51% | 8% (x1.08) | 6% (x1.07) | - | 3% (x1.03) | 6% (x1.07) | 27% (x1.38) | - |
| di_N10 | combinatorial_graph | 28% | 6% (x1.06) | 14% (x1.17) | - | 2% (x1.02) | 5% (x1.05) | 2% (x1.02) | - |
| di_N10 | mplcp | 15% | 1% (x1.01) | - | - | 7% (x1.07) | 2% (x1.02) | 6% (x1.06) | 0% (x1.00) |
| di_N10 | combinatorial | 19% | 0% (x1.00) | 19% (x1.24) | - | 0% (x1.00) | 0% (x1.00) | - | - |
| masses2_N6 | geometric | 77% | 19% (x1.23) | - | 38% (x1.62) | 1% (x1.01) | 19% (x1.24) | 0% (x1.00) | - |
| masses2_N6 | graph | 29% | 3% (x1.03) | 9% (x1.09) | - | 11% (x1.12) | 6% (x1.07) | 0% (x1.00) | - |
| masses2_N6 | graph_exp | 65% | 20% (x1.24) | 7% (x1.08) | - | 13% (x1.15) | 25% (x1.33) | 0% (x1.00) | - |
| masses2_N6 | combinatorial_graph | 42% | 11% (x1.13) | 20% (x1.25) | - | 0% (x1.00) | 10% (x1.11) | 0% (x1.00) | - |
| masses2_N6 | mplcp | 2% | 0% (x1.00) | - | 1% (x1.01) | 1% (x1.01) | 0% (x1.00) | 0% (x1.00) | 0% (x1.00) |
| masses2_N6 | combinatorial | 42% | 4% (x1.04) | 34% (x1.51) | - | 3% (x1.04) | 1% (x1.01) | - | - |
| sys3x2_N5 | geometric | 44% | 2% (x1.02) | - | 35% (x1.53) | 2% (x1.02) | 5% (x1.05) | 0% (x1.00) | - |
| sys3x2_N5 | graph | 23% | 1% (x1.01) | 10% (x1.12) | - | 0% (x1.00) | 12% (x1.14) | 0% (x1.00) | - |
| sys3x2_N5 | graph_exp | 21% | 4% (x1.05) | 15% (x1.17) | - | 0% (x1.00) | 1% (x1.01) | 0% (x1.00) | - |
| sys3x2_N5 | combinatorial_graph | 17% | 3% (x1.03) | 12% (x1.14) | - | 0% (x1.00) | 2% (x1.02) | 0% (x1.00) | - |
| sys3x2_N5 | mplcp | 9% | 0% (x1.00) | - | 1% (x1.01) | 7% (x1.08) | 0% (x1.00) | 0% (x1.00) | 0% (x1.00) |
| sys3x2_N5 | combinatorial | 40% | 0% (x1.00) | 40% (x1.65) | - | 0% (x1.00) | 0% (x1.00) | - | - |
| diinf_N4 | geometric | 52% | 3% (x1.03) | - | 45% (x1.80) | 1% (x1.01) | 2% (x1.02) | 1% (x1.01) | - |
| diinf_N4 | mplp_lex | 2% | 0% (x1.00) | 0% (x1.00) | 1% (x1.01) | 0% (x1.00) | 0% (x1.00) | 0% (x1.00) | - |
| diinf_N6 | geometric | 47% | 2% (x1.02) | - | 42% (x1.72) | 1% (x1.01) | 1% (x1.01) | 0% (x1.00) | - |
| diinf_N6 | mplp_lex | 0% | 0% (x1.00) | 0% (x1.00) | 0% (x1.00) | 0% (x1.00) | 0% (x1.00) | 0% (x1.00) | - |

### Region construction (inside gen_cr_*): share of solve time and Amdahl bound if its LPs are cut

| problem | algorithm | construction / solve | of which LP | non-LP part | x if LP -50% | x if LP -80% | x if LP -100% | x if construction free |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| di_N10 | geometric | 44% | 29% | 15% | 1.17 | 1.30 | 1.40 | 1.78 |
| di_N10 | graph | 24% | 16% | 9% | 1.09 | 1.14 | 1.19 | 1.32 |
| di_N10 | graph_exp | 26% | 17% | 9% | 1.09 | 1.16 | 1.21 | 1.36 |
| di_N10 | combinatorial_graph | 19% | 12% | 7% | 1.06 | 1.11 | 1.14 | 1.24 |
| di_N10 | mplcp | 53% | 9% | 44% | 1.05 | 1.08 | 1.10 | 2.12 |
| di_N10 | combinatorial | 0% | 0% | 0% | 1.00 | 1.00 | 1.00 | 1.00 |
| masses2_N6 | geometric | 51% | 39% | 12% | 1.24 | 1.45 | 1.63 | 2.03 |
| masses2_N6 | graph | 22% | 21% | 1% | 1.12 | 1.20 | 1.26 | 1.28 |
| masses2_N6 | graph_exp | 60% | 58% | 2% | 1.41 | 1.87 | 2.38 | 2.48 |
| masses2_N6 | combinatorial_graph | 22% | 22% | 1% | 1.12 | 1.21 | 1.28 | 1.29 |
| masses2_N6 | mplcp | 56% | 1% | 55% | 1.01 | 1.01 | 1.01 | 2.28 |
| masses2_N6 | combinatorial | 20% | 8% | 12% | 1.04 | 1.07 | 1.08 | 1.24 |
| sys3x2_N5 | geometric | 12% | 9% | 2% | 1.05 | 1.08 | 1.10 | 1.13 |
| sys3x2_N5 | graph | 14% | 13% | 1% | 1.07 | 1.12 | 1.15 | 1.16 |
| sys3x2_N5 | graph_exp | 6% | 6% | 1% | 1.03 | 1.05 | 1.06 | 1.07 |
| sys3x2_N5 | combinatorial_graph | 5% | 5% | 0% | 1.02 | 1.04 | 1.05 | 1.05 |
| sys3x2_N5 | mplcp | 62% | 8% | 54% | 1.04 | 1.06 | 1.08 | 2.62 |
| sys3x2_N5 | combinatorial | 1% | 0% | 1% | 1.00 | 1.00 | 1.00 | 1.01 |
| diinf_N4 | geometric | 11% | 6% | 5% | 1.03 | 1.05 | 1.07 | 1.12 |
| diinf_N4 | mplp_lex | 2% | 1% | 1% | 1.01 | 1.01 | 1.01 | 1.02 |
| diinf_N6 | geometric | 8% | 5% | 3% | 1.02 | 1.04 | 1.05 | 1.08 |
| diinf_N6 | mplp_lex | 0% | 0% | 0% | 1.00 | 1.00 | 1.00 | 1.00 |

### QP solves by purpose

| problem | algorithm | QPs | by purpose |
|---|---|---:|---|
| di_N10 | geometric | 35 | neighbour 34 (14 infeasible, 0.0 ms); init 1 (0 infeasible, 0.0 ms) |
| di_N10 | graph | 100 | init 100 (0 infeasible, 0.1 ms) |
| di_N10 | graph_exp | 100 | init 100 (0 infeasible, 0.1 ms) |
| di_N10 | combinatorial_graph | 1 | init 1 (0 infeasible, 0.0 ms) |
| di_N10 | mplcp | 1 | init 1 (0 infeasible, 0.0 ms) |
| masses2_N6 | geometric | 7347 | neighbour 7346 (1340 infeasible, 127.3 ms); init 1 (0 infeasible, 0.0 ms) |
| masses2_N6 | graph | 100 | init 100 (5 infeasible, 0.3 ms) |
| masses2_N6 | graph_exp | 100 | init 100 (5 infeasible, 0.3 ms) |
| masses2_N6 | combinatorial_graph | 1 | init 1 (0 infeasible, 0.0 ms) |
| masses2_N6 | mplcp | 1 | init 1 (0 infeasible, 0.0 ms) |
| sys3x2_N5 | geometric | 5511 | neighbour 5510 (224 infeasible, 27.3 ms); init 1 (0 infeasible, 0.0 ms) |
| sys3x2_N5 | graph | 100 | init 100 (4 infeasible, 0.4 ms) |
| sys3x2_N5 | graph_exp | 100 | init 100 (4 infeasible, 0.3 ms) |
| sys3x2_N5 | combinatorial_graph | 1 | init 1 (0 infeasible, 0.0 ms) |
| sys3x2_N5 | mplcp | 1 | init 1 (0 infeasible, 0.0 ms) |

### Redundancy removal (rows entering, kept, decided without an LP, tested by LP)

| problem | algorithm | region builds | rows in | rows in / build | kept / build | no LP | LP | LP rows infeasible (redundant) |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| di_N10 | geometric | 11 | 476 | 43.3 | 4.7 | 379 (80%) | 97 (20%) | 78 |
| di_N10 | graph | 11 | 476 | 43.3 | 4.7 | 379 (80%) | 97 (20%) | 78 |
| di_N10 | graph_exp | 11 | 476 | 43.3 | 4.7 | 379 (80%) | 97 (20%) | 78 |
| di_N10 | combinatorial_graph | 11 | 476 | 43.3 | 4.7 | 379 (80%) | 97 (20%) | 78 |
| di_N10 | mplcp | 11 | 476 | 43.3 | 4.7 | 473 (99%) | 3 (1%) | 2 |
| di_N10 | combinatorial | 11 | 476 | 43.3 | 4.7 | 379 (80%) | 97 (20%) | 78 |
| masses2_N6 | geometric | 901 | 56940 | 63.2 | 8.9 | 42516 (75%) | 14424 (25%) | 11196 |
| masses2_N6 | graph | 901 | 56940 | 63.2 | 8.9 | 42519 (75%) | 14421 (25%) | 11196 |
| masses2_N6 | graph_exp | 901 | 56940 | 63.2 | 8.9 | 42519 (75%) | 14421 (25%) | 11196 |
| masses2_N6 | combinatorial_graph | 901 | 56940 | 63.2 | 8.9 | 42515 (75%) | 14425 (25%) | 11196 |
| masses2_N6 | mplcp | 901 | 56940 | 63.2 | 8.9 | 56901 (100%) | 39 (0%) | 15 |
| masses2_N6 | combinatorial | 901 | 56940 | 63.2 | 8.9 | 42516 (75%) | 14424 (25%) | 11196 |
| sys3x2_N5 | geometric | 973 | 48286 | 49.6 | 6.2 | 39910 (83%) | 8376 (17%) | 6314 |
| sys3x2_N5 | graph | 973 | 48286 | 49.6 | 6.2 | 39912 (83%) | 8374 (17%) | 6314 |
| sys3x2_N5 | graph_exp | 973 | 48286 | 49.6 | 6.2 | 39912 (83%) | 8374 (17%) | 6314 |
| sys3x2_N5 | combinatorial_graph | 973 | 48286 | 49.6 | 6.2 | 39912 (83%) | 8374 (17%) | 6314 |
| sys3x2_N5 | mplcp | 973 | 48286 | 49.6 | 6.2 | 48249 (100%) | 37 (0%) | 16 |
| sys3x2_N5 | combinatorial | 973 | 48286 | 49.6 | 6.2 | 39910 (83%) | 8376 (17%) | 6314 |
| diinf_N4 | geometric | 39 | 1052 | 27.0 | 10.3 | 694 (66%) | 358 (34%) | 131 |
| diinf_N4 | mplp_lex | 39 | 1084 | 27.8 | 10.1 | 725 (67%) | 359 (33%) | 140 |
| diinf_N6 | geometric | 58 | 2232 | 38.5 | 10.8 | 1561 (70%) | 671 (30%) | 346 |
| diinf_N6 | mplp_lex | 58 | 2304 | 39.7 | 10.8 | 1631 (71%) | 673 (29%) | 348 |

### Combinatorial candidates

| problem | algorithm | generated | pruned by list | rank deficient | accepted by witness | tested by LP | LP infeasible |
|---|---|---:|---:|---:|---:|---:|---:|
| di_N10 | combinatorial | 33411266 | 26436105 | 36237 | 5558981 | 1379943 | 4670 |
| masses2_N6 | combinatorial | 1257241 | 1055095 | 10570 | 85022 | 106554 | 1688 |
| sys3x2_N5 | combinatorial | 32646711 | 27346466 | 49855 | 2984247 | 2266143 | 1978 |

### Detail: LP calls / infeasible / ms by purpose

| problem | algorithm | redundancy | feasibility | neighbour | chebyshev | bbox | init | other |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| di_N10 | geometric | 97 / 78 / 0.0 | - | 52 / 0 / 0.0 | 22 / 0 / 0.0 | 44 / 0 / 0.0 | 1 / 0 / 0.0 | - |
| di_N10 | graph | 97 / 78 / 0.0 | 236 / 200 / 0.0 | - | 11 / 0 / 0.0 | 44 / 0 / 0.0 | 1 / 0 / 0.0 | - |
| di_N10 | graph_exp | 97 / 78 / 0.0 | 236 / 200 / 0.0 | - | 11 / 0 / 0.0 | 44 / 0 / 0.0 | 1 / 0 / 0.0 | - |
| di_N10 | combinatorial_graph | 97 / 78 / 0.0 | 768 / 682 / 0.1 | - | 11 / 0 / 0.0 | 44 / 0 / 0.0 | 1 / 0 / 0.0 | - |
| di_N10 | mplcp | 3 / 2 / 0.0 | - | - | 12 / 0 / 0.0 | 4 / 0 / 0.0 | 1 / 0 / 0.0 | 1 / 0 / 0.0 |
| di_N10 | combinatorial | 97 / 78 / 0.0 | 6739124 / 5363638 / 9496.3 | - | 224 / 202 / 0.1 | 44 / 0 / 0.0 | - | - |
| masses2_N6 | geometric | 14424 / 11196 / 65.6 | - | 8176 / 44 / 6.3 | 1802 / 0 / 2.6 | 7208 / 0 / 66.7 | 1 / 0 / 0.0 | - |
| masses2_N6 | graph | 14421 / 11196 / 10.5 | 26372 / 14402 / 27.1 | - | 901 / 0 / 35.3 | 7208 / 0 / 20.1 | 1 / 0 / 0.0 | - |
| masses2_N6 | graph_exp | 14421 / 11196 / 56.3 | 30430 / 17298 / 20.9 | - | 901 / 0 / 38.0 | 7208 / 0 / 71.7 | 1 / 0 / 0.0 | - |
| masses2_N6 | combinatorial_graph | 14425 / 11196 / 97.3 | 134552 / 101446 / 172.7 | - | 901 / 0 / 2.1 | 7208 / 0 / 87.5 | 1 / 0 / 0.0 | - |
| masses2_N6 | mplcp | 39 / 15 / 0.0 | - | 3518 / 1591 / 2.3 | 902 / 0 / 2.3 | 8 / 0 / 0.0 | 1 / 0 / 0.0 | 1 / 0 / 0.0 |
| masses2_N6 | combinatorial | 14424 / 11196 / 77.5 | 296437 / 167782 / 700.3 | - | 24690 / 22888 / 71.1 | 7208 / 0 / 12.3 | - | - |
| sys3x2_N5 | geometric | 8376 / 6314 / 3.7 | - | 6018 / 0 / 27.9 | 1946 / 0 / 3.6 | 5838 / 0 / 7.5 | 1 / 0 / 0.0 | - |
| sys3x2_N5 | graph | 8374 / 6314 / 4.2 | 73040 / 57246 / 63.2 | - | 973 / 0 / 1.5 | 5838 / 0 / 73.6 | 1 / 0 / 0.0 | - |
| sys3x2_N5 | graph_exp | 8374 / 6314 / 28.2 | 96306 / 78316 / 95.7 | - | 973 / 0 / 1.6 | 5838 / 0 / 7.7 | 1 / 0 / 0.0 | - |
| sys3x2_N5 | combinatorial_graph | 8374 / 6314 / 56.8 | 343204 / 319862 / 267.1 | - | 973 / 0 / 1.4 | 5838 / 0 / 38.5 | 1 / 0 / 0.0 | - |
| sys3x2_N5 | mplcp | 37 / 16 / 0.0 | - | 490 / 310 / 0.2 | 974 / 0 / 1.3 | 12 / 0 / 0.0 | 1 / 0 / 0.0 | 1 / 0 / 0.0 |
| sys3x2_N5 | combinatorial | 8376 / 6314 / 4.2 | 7505883 / 5219565 / 18611.8 | - | 23124 / 21178 / 99.8 | 5838 / 0 / 6.6 | - | - |
| diinf_N4 | geometric | 358 / 131 / 0.1 | - | 1690 / 217 / 1.5 | 78 / 0 / 0.0 | 156 / 0 / 0.1 | 2 / 0 / 0.0 | - |
| diinf_N4 | mplp_lex | 359 / 140 / 0.1 | 39 / 0 / 0.0 | 349 / 0 / 0.2 | 78 / 0 / 0.1 | 156 / 0 / 0.1 | 1 / 0 / 0.0 | - |
| diinf_N6 | geometric | 671 / 346 / 0.1 | - | 2835 / 446 / 2.6 | 116 / 0 / 0.1 | 232 / 0 / 0.1 | 2 / 0 / 0.0 | - |
| diinf_N6 | mplp_lex | 673 / 348 / 0.2 | 58 / 0 / 0.0 | 577 / 0 / 0.3 | 116 / 0 / 0.1 | 232 / 0 / 0.1 | 1 / 0 / 0.0 | - |
