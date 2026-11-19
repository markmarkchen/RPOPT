size = theta/x/m: parameters, variables, constraints (rows of A x <= b + F theta that the solvers see after loading; `results/problem_sizes.md` lists every problem with the raw count and the regions).

| problem | size (theta/x/m) | regions | sequential (PPOPT algorithm) | sequential, early exit | BST | multiway (order 7) | linear machine | descriptor | hash table (16R boxes) | YANN |
|---|---|---|---|---|---|---|---|---|---|---|
| di_N10 | 2/10/42 | 11 | 74 | 33 | 41 | 36 | 29 | 41 | 22 | 76 |
| di_N4 | 2/4/18 | 9 | 65 | 30 | 35 | 31 | 28 | 38 | 22 | 65 |
| di_N6 | 2/6/26 | 11 | 74 | 33 | 41 | 36 | 30 | 41 | 22 | 74 |
| di_N8 | 2/8/34 | 11 | 74 | 33 | 40 | 36 | 29 | 41 | 21 | 76 |
| di_doc_N3 | 2/9/20 | 7 | 51 | 28 | 33 | 28 | 25 | 35 | 21 | 48 |
| masses2_N4 | 4/4/40 | 307 | 4773 | 1252 | 180 | 135 | none exists (118 non-f2f facets) | 1038 | 65 | 4670 |
| masses2_N6 | 4/6/60 | 901 | 13795 | 5167 | 233 | 165 | none exists (286 non-f2f facets) | 4070 | 73 | 13664 |
| sys3x2_N3 | 3/6/30 | 211 | 1956 | 784 | 120 | 96 | none exists (6 non-f2f facets) | 667 | 37 | 1931 |
| sys3x2_N5 | 3/10/50 | 973 | 8999 | 4338 | 147 | 113 | none exists (22 non-f2f facets) | 3476 | 46 | 9007 |
