size = theta/x/m: parameters, variables, constraints (rows of A x <= b + F theta that the solvers see after loading; `results/problem_sizes.md` lists every problem with the raw count and the regions).

| problem | size (theta/x/m) | PPOPT fastest gap-free | PPOPT geometric (gaps) | RPOPT fastest (1 thread) | speedup |
|---|---|---|---|---|---:|
| diinf_N2 | 2/6/24 | geometric 144 ms | 144 ms (0.0% gaps) | mplp_lex 0.17 ms | 836x |
| diinf_N3 | 2/9/32 | none within 120 s | 200 ms (49.8% gaps) | mplp_lex 0.40 ms | > 301,508x |
| diinf_N4 | 2/12/42 | none within 120 s | 267 ms (50.4% gaps) | mplp_lex 0.89 ms | > 134,635x |
| diinf_N5 | 2/15/52 | none within 120 s | 399 ms (50.4% gaps) | mplp_lex 1.83 ms | > 65,574x |
| dione_N2 | 2/8/24 | combinatorial 3,014 ms | 142 ms (0.3% gaps) | mplp_lex 0.17 ms | 18,028x |
| dione_N3 | 2/12/32 | none within 120 s | 128 ms (52.7% gaps) | mplp_lex 0.39 ms | > 305,732x |
| dione_N4 | 2/16/42 | none within 120 s | 112 ms (76.8% gaps) | mplp_lex 0.89 ms | > 134,725x |
| sys3x2inf_N2 | 3/8/40 | none within 120 s | 2,546 ms (0.4% gaps) | mplp_lex 3.50 ms | > 34,305x |
| sys3x2inf_N3 | 3/12/60 | none within 120 s | crashed | mplp_lex 18.69 ms | > 6,421x |
| masses2inf_N2 | 4/6/40 | geometric 8,121 ms | 8,121 ms (0.0% gaps) | mplp_lex 7.12 ms | 1,140x |
| masses2inf_N3 | 4/9/60 | geometric 83,868 ms | 83,868 ms (0.0% gaps) | mplp_lex 49.25 ms | 1,703x |
| sys3x2one_N2 | 3/14/40 | geometric 4,517 ms | 4,517 ms (0.0% gaps) | mplp_lex 5.06 ms | 892x |
| masses2one_N2 | 4/12/40 | geometric 44,642 ms | 44,642 ms (0.0% gaps) | mplp_lex 23.89 ms | 1,869x |
