# mplcp: rank-1 enumeration of Case 2

Branch `mplcp-case2-p3b`, base main f0e1084 (witness tree, Lean round 7). Commits: 547ab84 (census counters), 182c672
(the enumeration), ccbbe18 (hypotheses of the Lean lemma checked), 22f33e6 and the commit after it (repeated-test
counters). Timings: serial, 1 CPU, release profile, Negishi; one array task per (problem, alternation), the binaries
run once each in a shuffled order on the same node, median ratio over 4 to 5 tasks per problem (each pair in its own
task above 5 s). Ratio above 1: the second binary is faster. "before" = b522425 (the merged witness tree, which the
code is identical to apart from this change and `mplp_lex`).

## The counters that sized it

Census of the 49-problem set before building (23 bench, 7 generated at 3 to 5 parameters, 12 census at 6 to 10,
7 `problems_highdim`; job array 45888858). A Case-2 call is "the target class" when exactly one row c vanishes on the
facet, it is a slack row, its pivot T[c,kk] is zero and every slack entry of column kk is zero (the hypotheses of the
Lean lemma `DependentFacet`).

### G. Case-2 calls by class, per problem (calls, with the share of Case-2 calls)
| params | problem | solve ms | Case-2 calls | 1 slack row, zero pivot, slack columns zero (target) | 1 slack row, zero pivot, slack column not zero | 1 slack row, other | 1 multiplier row | 2 rows | 3+ rows | Case-2 ms by class (target / not zero / other / mult / 2 / 3+) | Case-2 time as share of solve |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|
| 2 | di_N10 | 0 | 14 | 14 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 / 0 / 0 / 0 / 0 / 0 | 0% |
| 2 | di_N12 | 0 | 14 | 14 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 / 0 / 0 / 0 / 0 / 0 | 0% |
| 2 | di_N15 | 0 | 14 | 14 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 / 0 / 0 / 0 / 0 / 0 | 0% |
| 2 | di_N20 | 0 | 14 | 14 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 / 0 / 0 / 0 / 0 / 0 | 0% |
| 2 | di_N25 | 0 | 14 | 14 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 / 0 / 0 / 0 / 0 / 0 | 0% |
| 2 | di_N30 | 1 | 14 | 14 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 / 0 / 0 / 0 / 0 / 0 | 0% |
| 2 | di_N4 | 0 | 12 | 12 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 / 0 / 0 / 0 / 0 / 0 | 0% |
| 2 | di_N6 | 0 | 14 | 14 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 / 0 / 0 / 0 / 0 / 0 | 0% |
| 2 | di_N8 | 0 | 14 | 14 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 / 0 / 0 / 0 / 0 / 0 | 0% |
| 2 | di_doc_N3 | 0 | 8 | 8 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 / 0 / 0 / 0 / 0 / 0 | 0% |
| 2 | dinv_N4 | 0 | 12 | 12 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 / 0 / 0 / 0 / 0 / 0 | 0% |
| 2 | dinv_N6 | 0 | 14 | 14 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 / 0 / 0 / 0 / 0 / 0 | 0% |
| 2 | dinv_N8 | 0 | 14 | 14 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 / 0 / 0 / 0 / 0 / 0 | 0% |
| 3 | rand_t3_x5_m20_s0 | 3 | 165 | 165 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 1 / 0 / 0 / 0 / 0 / 0 | 31% |
| 3 | sys3x2_N3 | 4 | 118 | 118 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 1 / 0 / 0 / 0 / 0 / 0 | 23% |
| 3 | sys3x2_N4 | 12 | 209 | 209 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 2 / 0 / 0 / 0 / 0 / 0 | 17% |
| 3 | sys3x2_N5 | 24 | 296 | 296 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 3 / 0 / 0 / 0 / 0 / 0 | 12% |
| 3 | sys3x2_N6 | 39 | 306 | 306 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 3 / 0 / 0 / 0 / 0 / 0 | 8% |
| 3 | sys3x2_N7 | 56 | 290 | 290 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 2 / 0 / 0 / 0 / 0 / 0 | 4% |
| 4 | masses2_N4 | 18 | 853 | 853 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 6 / 0 / 0 / 0 / 0 / 0 | 33% |
| 4 | masses2_N5 | 31 | 1234 | 1234 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 11 / 0 / 0 / 0 / 0 / 0 | 35% |
| 4 | masses2_N6 | 62 | 1996 | 1996 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 22 / 0 / 0 / 0 / 0 / 0 | 35% |
| 4 | masses2_N7 | 113 | 2912 | 2912 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 35 / 0 / 0 / 0 / 0 / 0 | 31% |
| 4 | masses2_N8 | 204 | 4203 | 4203 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 60 / 0 / 0 / 0 / 0 / 0 | 29% |
| 4 | rand_t4_x5_m20_s0 | 44 | 910 | 910 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 11 / 0 / 0 / 0 / 0 / 0 | 25% |
| 4 | rand_t4_x5_m20_s1 | 29 | 1101 | 1101 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 12 / 0 / 0 / 0 / 0 / 0 | 41% |
| 5 | rand_t5_x5_m20_s0 | 202 | 2820 | 2820 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 51 / 0 / 0 / 0 / 0 / 0 | 25% |
| 5 | rand_t5_x5_m20_s1 | 136 | 3841 | 3841 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 55 / 0 / 0 / 0 / 0 / 0 | 41% |
| 5 | rand_t5_x5_m20_s2 | 196 | 4679 | 4679 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 70 / 0 / 0 / 0 / 0 / 0 | 36% |
| 5 | rand_t5_x6_m24_s0 | 728 | 15947 | 15946 (100%) | 1 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 302 / 0 / 0 / 0 / 0 / 0 | 42% |
| 6 | masses3_N3 | 422 | 8465 | 8252 (97%) | 213 (3%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 164 / 88 / 0 / 0 / 0 / 0 | 60% |
| 6 | masses3_N4 | 4245 | 30401 | 29310 (96%) | 1081 (4%) | 0 (0%) | 0 (0%) | 8 (0%) | 2 (0%) | 1833 / 1260 / 0 / 0 / 0 / 0 | 73% |
| 6 | pop_017 | 1102 | 31110 | 30432 (98%) | 3 (0%) | 1 (0%) | 0 (0%) | 674 (2%) | 0 (0%) | 531 / 1 / 0 / 0 / 5 / 0 | 49% |
| 6 | rand_t6_x5_m20_s0 | 277 | 12060 | 12060 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 109 / 0 / 0 / 0 / 0 / 0 | 39% |
| 7 | pop_005 | 5947 | 55766 | 55748 (100%) | 18 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 3932 / 4 / 0 / 0 / 0 / 0 | 66% |
| 7 | pop_021 | 194 | 7845 | 7824 (100%) | 21 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 106 / 0 / 0 / 0 / 0 / 0 | 55% |
| 8 | masses4_N3 | 4884 | 35894 | 33384 (93%) | 2509 (7%) | 0 (0%) | 0 (0%) | 1 (0%) | 0 (0%) | 1415 / 2036 / 0 / 0 / 0 / 0 | 71% |
| 8 | pop_027 | 615 | 24461 | 24461 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 314 / 0 / 0 / 0 / 0 / 0 | 51% |
| 8 | rand_t8_x5_m20_s0 | 1127 | 36354 | 36354 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 479 / 0 / 0 / 0 / 0 / 0 | 43% |
| 10 | masses5_N2 | 621 | 5477 | 5268 (96%) | 208 (4%) | 0 (0%) | 0 (0%) | 1 (0%) | 0 (0%) | 271 / 120 / 0 / 0 / 0 / 0 | 63% |
| 10 | rand_t10_x5_m15_s0 | 650 | 12871 | 12871 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 240 / 0 / 0 / 0 / 0 / 0 | 37% |
| 10 | rand_t10_x5_m20_s0 | 3650 | 65869 | 65869 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 1722 / 0 / 0 / 0 / 0 / 0 | 47% |
| 12 | masses6_N2 | 1143 | 5990 | 5779 (96%) | 210 (4%) | 0 (0%) | 0 (0%) | 1 (0%) | 0 (0%) | 368 / 388 / 0 / 0 / 0 / 0 | 66% |
| 12 | rand_t12_x5_m15_s0 | 550 | 7683 | 7683 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 200 / 0 / 0 / 0 / 0 / 0 | 36% |
| 12 | rand_t12_x5_m20_s0 | 6532 | 69559 | 69551 (100%) | 8 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 3810 / 47 / 0 / 0 / 0 / 0 | 59% |
| 14 | masses7_N2 | 1830 | 6270 | 6051 (97%) | 218 (3%) | 0 (0%) | 0 (0%) | 1 (0%) | 0 (0%) | 702 / 409 / 0 / 0 / 0 / 0 | 61% |
| 14 | rand_t14_x5_m15_s0 | 2643 | 16404 | 16404 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 1012 / 0 / 0 / 0 / 0 / 0 | 38% |
| 16 | masses8_N2 | 2403 | 6337 | 6115 (96%) | 222 (4%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 873 / 429 / 0 / 0 / 0 / 0 | 54% |
| 16 | rand_t16_x5_m15_s0 | 2595 | 12874 | 12874 (100%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 0 (0%) | 868 / 0 / 0 / 0 / 0 / 0 | 33% |
### H. The target class: |P-|, members of S, Seen, empty sub-facets, ties, walk depth
| params | problem | target calls | |P-| = 0 / 1 / 2 / 3 / 4 / 5 / 6 / 7+ | members of S | already in Seen | calls with all of S in Seen | unseen members: empty sub-facet / nonempty | calls with a tie | depth of the current walk 0 / 1 / 2 / 3-5 / 6+ |
|---:|---|---:|---|---:|---:|---:|---|---:|---|
| 2 | di_N10 | 14 | 14 / 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 (-) | 0 (0%) | 0 / 0 | 0 | 14 / 0 / 0 / 0 / 0 |
| 2 | di_N12 | 14 | 14 / 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 (-) | 0 (0%) | 0 / 0 | 0 | 14 / 0 / 0 / 0 / 0 |
| 2 | di_N15 | 14 | 14 / 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 (-) | 0 (0%) | 0 / 0 | 0 | 14 / 0 / 0 / 0 / 0 |
| 2 | di_N20 | 14 | 14 / 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 (-) | 0 (0%) | 0 / 0 | 0 | 14 / 0 / 0 / 0 / 0 |
| 2 | di_N25 | 14 | 14 / 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 (-) | 0 (0%) | 0 / 0 | 0 | 14 / 0 / 0 / 0 / 0 |
| 2 | di_N30 | 14 | 14 / 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 (-) | 0 (0%) | 0 / 0 | 0 | 14 / 0 / 0 / 0 / 0 |
| 2 | di_N4 | 12 | 12 / 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 (-) | 0 (0%) | 0 / 0 | 0 | 12 / 0 / 0 / 0 / 0 |
| 2 | di_N6 | 14 | 14 / 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 (-) | 0 (0%) | 0 / 0 | 0 | 14 / 0 / 0 / 0 / 0 |
| 2 | di_N8 | 14 | 14 / 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 (-) | 0 (0%) | 0 / 0 | 0 | 14 / 0 / 0 / 0 / 0 |
| 2 | di_doc_N3 | 8 | 8 / 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 (-) | 0 (0%) | 0 / 0 | 0 | 8 / 0 / 0 / 0 / 0 |
| 2 | dinv_N4 | 12 | 12 / 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 (-) | 0 (0%) | 0 / 0 | 0 | 12 / 0 / 0 / 0 / 0 |
| 2 | dinv_N6 | 14 | 14 / 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 (-) | 0 (0%) | 0 / 0 | 0 | 14 / 0 / 0 / 0 / 0 |
| 2 | dinv_N8 | 14 | 14 / 0 / 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 (-) | 0 (0%) | 0 / 0 | 0 | 14 / 0 / 0 / 0 / 0 |
| 3 | rand_t3_x5_m20_s0 | 165 | 91 / 55 / 17 / 2 / 0 / 0 / 0 / 0 | 95 | 48 (51%) | 29 (18%) | 12 / 35 | 0 | 91 / 74 / 0 / 0 / 0 |
| 3 | sys3x2_N3 | 118 | 96 / 8 / 5 / 0 / 6 / 3 / 0 / 0 | 57 | 13 (23%) | 0 (0%) | 29 / 15 | 0 | 96 / 22 / 0 / 0 / 0 |
| 3 | sys3x2_N4 | 209 | 156 / 13 / 13 / 1 / 7 / 4 / 11 / 4 | 184 | 38 (21%) | 1 (0%) | 111 / 35 | 0 | 156 / 53 / 0 / 0 / 0 |
| 3 | sys3x2_N5 | 296 | 224 / 22 / 15 / 1 / 2 / 2 / 9 / 21 | 294 | 65 (22%) | 8 (3%) | 192 / 37 | 0 | 224 / 72 / 0 / 0 / 0 |
| 3 | sys3x2_N6 | 306 | 254 / 27 / 4 / 0 / 1 / 2 / 1 / 17 | 200 | 44 (22%) | 8 (3%) | 128 / 28 | 0 | 254 / 52 / 0 / 0 / 0 |
| 3 | sys3x2_N7 | 290 | 274 / 12 / 1 / 0 / 1 / 2 / 0 / 0 | 28 | 10 (36%) | 5 (2%) | 10 / 8 | 0 | 274 / 16 / 0 / 0 / 0 |
| 4 | masses2_N4 | 853 | 582 / 118 / 97 / 33 / 23 / 0 / 0 / 0 | 503 | 259 (51%) | 101 (12%) | 114 / 130 | 0 | 582 / 271 / 0 / 0 / 0 |
| 4 | masses2_N5 | 1234 | 834 / 134 / 128 / 45 / 47 / 46 / 0 / 0 | 943 | 405 (43%) | 90 (7%) | 351 / 187 | 0 | 834 / 400 / 0 / 0 / 0 |
| 4 | masses2_N6 | 1996 | 1336 / 230 / 151 / 92 / 69 / 72 / 46 / 0 | 1720 | 678 (39%) | 138 (7%) | 733 / 309 | 0 | 1336 / 660 / 0 / 0 / 0 |
| 4 | masses2_N7 | 2912 | 2042 / 327 / 195 / 98 / 82 / 95 / 60 / 13 | 2265 | 871 (38%) | 198 (7%) | 971 / 423 | 0 | 2042 / 870 / 0 / 0 / 0 |
| 4 | masses2_N8 | 4203 | 2922 / 458 / 309 / 138 / 130 / 133 / 90 / 23 | 3382 | 1304 (39%) | 319 (8%) | 1440 / 638 | 0 | 2922 / 1281 / 0 / 0 / 0 |
| 4 | rand_t4_x5_m20_s0 | 910 | 0 / 295 / 421 / 99 / 95 / 0 / 0 / 0 | 1814 | 1066 (59%) | 440 (48%) | 368 / 380 | 0 | 0 / 910 / 0 / 0 / 0 |
| 4 | rand_t4_x5_m20_s1 | 1101 | 266 / 108 / 150 / 302 / 154 / 121 / 0 / 0 | 2535 | 1150 (45%) | 175 (16%) | 1059 / 326 | 0 | 266 / 835 / 0 / 0 / 0 |
| 5 | rand_t5_x5_m20_s0 | 2820 | 0 / 1642 / 1028 / 135 / 15 / 0 / 0 / 0 | 4163 | 2752 (66%) | 1587 (56%) | 351 / 1060 | 0 | 0 / 2820 / 0 / 0 / 0 |
| 5 | rand_t5_x5_m20_s1 | 3841 | 400 / 242 / 710 / 917 / 1033 / 539 / 0 / 0 | 11240 | 4884 (43%) | 540 (14%) | 5370 / 986 | 0 | 400 / 3441 / 0 / 0 / 0 |
| 5 | rand_t5_x5_m20_s2 | 4679 | 620 / 202 / 1157 / 1328 / 961 / 411 / 0 / 0 | 12399 | 9466 (76%) | 1998 (43%) | 2441 / 492 | 0 | 620 / 4059 / 0 / 0 / 0 |
| 5 | rand_t5_x6_m24_s0 | 15946 | 1273 / 1555 / 2863 / 5259 / 3997 / 899 / 100 / 0 | 44141 | 26989 (61%) | 4093 (26%) | 14153 / 2999 | 1 | 1280 / 14663 / 1 / 2 / 0 |
| 6 | masses3_N3 | 8252 | 2785 / 3191 / 1564 / 712 / 0 / 0 / 0 / 0 | 8455 | 6583 (78%) | 3933 (48%) | 1317 / 555 | 0 | 2777 / 5450 / 11 / 6 / 8 |
| 6 | masses3_N4 | 29310 | 9057 / 10279 / 5417 / 3099 / 1458 / 0 / 0 / 0 | 36242 | 24245 (67%) | 12185 (42%) | 9382 / 2615 | 0 | 9088 / 20046 / 59 / 68 / 49 |
| 6 | pop_017 | 30432 | 3620 / 9207 / 9206 / 6209 / 2190 / 0 / 0 / 0 | 55006 | 45498 (83%) | 19058 (63%) | 8280 / 1228 | 1 | 3655 / 26774 / 0 / 2 / 1 |
| 6 | rand_t6_x5_m20_s0 | 12060 | 2081 / 8144 / 1623 / 212 / 0 / 0 / 0 / 0 | 12026 | 11717 (97%) | 9670 (80%) | 231 / 78 | 0 | 2081 / 9979 / 0 / 0 / 0 |
| 7 | pop_005 | 55748 | 23275 / 16883 / 7197 / 5039 / 2223 / 953 / 178 / 0 | 61119 | 42018 (69%) | 19867 (36%) | 16413 / 2688 | 0 | 23116 / 32072 / 148 / 217 / 195 |
| 7 | pop_021 | 7824 | 709 / 2743 / 3154 / 1218 / 0 / 0 / 0 / 0 | 12705 | 10053 (79%) | 4639 (59%) | 2116 / 536 | 0 | 709 / 7115 / 0 / 0 / 0 |
| 8 | masses4_N3 | 33384 | 8989 / 13143 / 8146 / 3106 / 0 / 0 / 0 / 0 | 38753 | 32901 (85%) | 19358 (58%) | 3953 / 1899 | 0 | 8954 / 24144 / 225 / 18 / 43 |
| 8 | pop_027 | 24461 | 7944 / 14696 / 1737 / 84 / 0 / 0 / 0 / 0 | 18422 | 17651 (96%) | 15751 (64%) | 310 / 461 | 0 | 7944 / 16517 / 0 / 0 / 0 |
| 8 | rand_t8_x5_m20_s0 | 36354 | 5475 / 21455 / 8046 / 1284 / 94 / 0 / 0 / 0 | 41775 | 40487 (97%) | 29646 (82%) | 450 / 838 | 0 | 5475 / 30879 / 0 / 0 / 0 |
| 10 | masses5_N2 | 5268 | 1746 / 2496 / 1026 / 0 / 0 / 0 / 0 / 0 | 4548 | 4334 (95%) | 3310 (63%) | 54 / 160 | 0 | 1722 / 3512 / 26 / 1 / 7 |
| 10 | rand_t10_x5_m15_s0 | 12871 | 2001 / 5610 / 4664 / 596 / 0 / 0 / 0 / 0 | 16726 | 15526 (93%) | 9947 (77%) | 6 / 1194 | 0 | 2001 / 10870 / 0 / 0 / 0 |
| 10 | rand_t10_x5_m20_s0 | 65869 | 9022 / 33626 / 20974 / 2220 / 27 / 0 / 0 / 0 | 82342 | 76737 (93%) | 51685 (78%) | 377 / 5228 | 0 | 9022 / 56847 / 0 / 0 / 0 |
| 12 | masses6_N2 | 5779 | 1842 / 2806 / 1131 / 0 / 0 / 0 / 0 / 0 | 5068 | 4868 (96%) | 3739 (65%) | 38 / 162 | 0 | 1816 / 3929 / 26 / 0 / 8 |
| 12 | rand_t12_x5_m15_s0 | 7683 | 1181 / 4758 / 1627 / 111 / 6 / 0 / 0 / 0 | 8369 | 8116 (97%) | 6250 (81%) | 9 / 244 | 0 | 1181 / 6502 / 0 / 0 / 0 |
| 12 | rand_t12_x5_m20_s0 | 69551 | 5955 / 35831 / 23646 / 3970 / 149 / 0 / 0 / 0 | 95629 | 93565 (98%) | 61655 (89%) | 801 / 1263 | 0 | 5953 / 63402 / 7 / 40 / 149 |
| 14 | masses7_N2 | 6051 | 1869 / 2992 / 1190 / 0 / 0 / 0 / 0 / 0 | 5372 | 5181 (96%) | 3993 (66%) | 26 / 165 | 0 | 1836 / 4181 / 26 / 0 / 8 |
| 14 | rand_t14_x5_m15_s0 | 16404 | 1074 / 7265 / 5050 / 2534 / 481 / 0 / 0 / 0 | 26891 | 26089 (97%) | 14528 (89%) | 11 / 791 | 0 | 1074 / 15329 / 0 / 0 / 1 |
| 16 | masses8_N2 | 6115 | 1857 / 3059 / 1199 / 0 / 0 / 0 / 0 / 0 | 5457 | 5351 (98%) | 4152 (68%) | 28 / 78 | 0 | 1835 / 4248 / 26 / 0 / 6 |
| 16 | rand_t16_x5_m15_s0 | 12874 | 0 / 8120 / 3624 / 946 / 176 / 8 / 0 / 0 | 18950 | 17907 (94%) | 11974 (93%) | 0 / 1043 | 0 | 0 / 12874 / 0 / 0 / 0 |

## What the enumeration does

`adjacent` calls `rank1_case2` right after the fast path and the certificate. With one vanishing slack row c,
zero pivot T[c,kk], column kk zero on every slack row, row c zero on every nonbasic z column and equal to mu_j on the
w_j column of every active row j (the hypotheses of `DepFacet`, checked), P- = {j active : mu_j = -T[j,kk] > 1e-10}.
* P- empty: no neighbour (the facet is on the boundary of the feasible set; Lean `no_solution_of_empty`,
  `qp_infeasible_of_empty`).
* Otherwise the neighbours are the sets A + {c} - {j}, j in P- (Lean `returned_mem_S`). A set already in `Seen` is dropped
  (`reach_skip`, exact). For the others the piece of the facet where j is the strict argmin of lambda_j / mu_j is tested,
  F with {mu_k lambda_j - mu_j lambda_k <= 0, k in P-}: first by the witness points already known (the region centre
  projected onto the facet, the centres of the earlier members), else one Chebyshev LP, accepted at radius above 1e-9
  as in the walk. The witness or centre is the candidate's hint.
* Independence of the columns of A + {c} - {j} (`hinj`) follows from mu_j != 0 and the independence of A; a multiplier
  between 1e-10 and 1e-7 of the largest one is treated as neither zero nor safely positive and hands the call to the walk.
* Ties (two ratios identical as affine functions on F, or a constant tie) hand the call to the walk, as do several
  vanishing rows, a multiplier row, a nonzero pivot or slack column, a failed row-c check, and nt = 1.
`mplcp::set_rank1(false)` restores the walk.

Counters after (census table 3i, one run per problem; "before" = b522425, "after" = this build):

17
0
| params | problem | LP/region before | LP/region after | neighbour LPs/region after | Case-2 calls | answered by the enumeration | handed to the walk | members dropped as known | emitted on a witness | sub-facet LPs | of which empty | distinct sets tested | repeated tests | of which empty |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 3 | sys3x2_N7 | 1.0 | 1.0 | 0.0 | 290 | 290 | 0 | 10 | 2 | 16 | 10 | 18 | 0 | 0 |
| 4 | masses2_N8 | 1.9 | 1.6 | 0.6 | 4203 | 4202 | 1 | 1303 | 257 | 1819 | 1438 | 1944 | 132 | 128 |
| 5 | rand_t5_x5_m20_s2 | 5.6 | 2.3 | 1.3 | 4676 | 4676 | 0 | 9464 | 42 | 2888 | 2438 | 1852 | 1078 | 1069 |
| 6 | masses3_N3 | 49.0 | 44.2 | 12.7 | 8465 | 7908 | 557 | 6283 | 127 | 1562 | 1178 | 1116 | 573 | 571 |
| 6 | pop_017 | 34.2 | 26.2 | 2.3 | 31110 | 30242 | 868 | 45358 | 196 | 9175 | 8195 | 3875 | 5496 | 5474 |
| 7 | pop_005 | 54.2 | 49.5 | 14.0 | 55766 | 51585 | 4181 | 38140 | 418 | 14575 | 13204 | 10262 | 4731 | 4680 |
| 7 | pop_021 | 35.5 | 24.4 | 3.8 | 7845 | 7824 | 21 | 10053 | 117 | 2535 | 2116 | 987 | 1665 | 1600 |
| 8 | masses4_N3 | 87.8 | 78.5 | 36.9 | 35894 | 29825 | 6069 | 28565 | 168 | 4733 | 3393 | 3035 | 1866 | 1840 |
| 8 | rand_t8_x5_m20_s0 | 29.3 | 22.4 | 0.3 | 36354 | 36354 | 0 | 40487 | 111 | 1177 | 450 | 1078 | 210 | 210 |
| 10 | masses5_N2 | 60.7 | 50.1 | 16.5 | 5477 | 4995 | 482 | 4090 | 19 | 151 | 47 | 143 | 27 | 27 |
| 12 | masses6_N2 | 68.1 | 57.3 | 19.8 | 5990 | 5484 | 506 | 4604 | 16 | 138 | 32 | 134 | 20 | 20 |
| 12 | rand_t12_x5_m15_s0 | 31.9 | 26.2 | 0.1 | 7683 | 7668 | 15 | 8101 | 105 | 139 | 0 | 244 | 0 | 0 |
| 12 | rand_t12_x5_m20_s0 | 44.3 | 32.7 | 5.5 | 69559 | 68374 | 1185 | 91802 | 234 | 1118 | 235 | 1142 | 210 | 210 |
| 14 | masses7_N2 | 69.4 | 56.3 | 15.5 | 6270 | 5725 | 545 | 4891 | 16 | 123 | 14 | 131 | 8 | 8 |
| 14 | rand_t14_x5_m15_s0 | 38.0 | 30.2 | 0.3 | 16404 | 16378 | 26 | 26050 | 187 | 604 | 0 | 791 | 0 | 0 |
| 16 | masses8_N2 | 73.1 | 59.8 | 15.5 | 6337 | 5807 | 530 | 5071 | 1 | 61 | 20 | 52 | 10 | 10 |
| 16 | rand_t16_x5_m15_s0 | 41.7 | 34.8 | 0.4 | 12874 | 12865 | 9 | 17889 | 212 | 831 | 0 | 1043 | 0 | 0 |

Measure for emitting unseen members without the sub-facet test (idea 2): the last two columns count, over a whole run, the
sub-facet tests of an active set that had already been tested before (from the same or another facet) and how many of
those repeats were empty. Of the empty verdicts, the repeats are 67% on pop_017 (5,474 of 8,195), 76% on pop_021 (1,600
of 2,116), 54% on masses4_N3 (1,840 of 3,393), 89% on rand_t12_x5_m20_s0 (210 of 235), 44% on rand_t5_x5_m20_s2, 35% on
pop_005 (4,680 of 13,204), 57% on masses5_N2 (27 of 47), 63% on masses6_N2, 57% on masses7_N2 and 50% on masses8_N2; the
random 12 to 16 parameter problems without masses have no or few empty verdicts (rand_t12_x5_m15, rand_t14, rand_t16: none).
A verdict cannot be reused by the active set alone, because the sub-facet belongs to the facet it is tested from, so a cache
keyed by the set would not be exact; emitting the unseen members without the test is the exact route and is not built.

## Region hashes against b522425 (job arrays 45889590 and 45889591, 159 problems)

Identical on 141 of 147 comparable problems (two pop problems timed out in both builds). The six that differ:
masses3_N5, pop_005, pop_007, pop_047 (same regions and facet rows, only the rounded rows differ: the hint of the
returned candidates is now the witness or sub-facet centre instead of the walk's, and the redundancy step depends on
the centre, as in the witness commit), masses5_N3 (5,061 to 5,060 regions, 194,461 to 194,427 facet rows) and
rand_t8_x6_m24_s0 (32,474 to 32,475 regions, 581,170 to 581,181 rows).

Coverage by `lexcheck` (20,000 samples, gate build): no gap and no overlap on 151 of the 159 problems, including pop_005 (which had 326 sampled overlaps in the phase-3 build and
has none in b522425 either), masses5_N3 and rand_t8_x6_m24_s0 (the two whose region counts changed by one) and
masses3_N5, pop_007, pop_047 (hash-only changes); b522425 gives the same on those six. Not checked: pop_002, pop_024, pop_033,
pop_036, pop_051 (the 1.9 GB limit of a 1-CPU task killed them) and masses4_N5, masses5_N4, pop_095 (over 40 minutes).

## Time

| params | problems | b522425 / rank-1 build (geometric mean, range) |
|---:|---:|---|
| 2 | 13 | 1.208 (1.140 to 1.228) |
| 3 | 6 | 1.087 (1.023 to 1.190) |
| 4 | 7 | 1.269 (1.165 to 1.327) |
| 5 | 4 | 1.219 (1.109 to 1.307) |
| 6 | 4 | 1.342 (1.144 to 1.574) |
| 7 | 2 | 1.465 (1.221 to 1.759) |
| 8 | 3 | 1.481 (1.160 to 1.769) |
| 10 | 3 | 1.452 (1.311 to 1.603) |
| 12 | 3 | 1.403 (1.253 to 1.497) |
| 14 | 2 | 1.420 (1.369 to 1.472) |
| 16 | 2 | 1.363 (1.345 to 1.382) |

Per problem (m1 = b522425, rk2 = this build; the A/A column is not valid in this table):

| params | problem | regions | main ms | final | A/A |
|---:|---|---|---:|---:|---:|
| 2 | di_N10 | 11 / 11 | 0.156 | 1.219 | - |
| 2 | di_N12 | 11 / 11 | 0.183 | 1.213 | - |
| 2 | di_N15 | 11 / 11 | 0.232 | 1.207 | - |
| 2 | di_N20 | 11 / 11 | 0.326 | 1.201 | - |
| 2 | di_N25 | 11 / 11 | 0.419 | 1.223 | - |
| 2 | di_N30 | 11 / 11 | 0.567 | 1.227 | - |
| 2 | di_N4 | 9 / 9 | 0.077 | 1.222 | - |
| 2 | di_N6 | 11 / 11 | 0.113 | 1.211 | - |
| 2 | di_N8 | 11 / 11 | 0.134 | 1.220 | - |
| 2 | di_doc_N3 | 7 / 7 | 0.066 | 1.140 | - |
| 2 | dinv_N4 | 9 / 9 | 0.070 | 1.228 | - |
| 2 | dinv_N6 | 11 / 11 | 0.112 | 1.202 | - |
| 2 | dinv_N8 | 11 / 11 | 0.131 | 1.195 | - |
| 3 | rand_t3_x5_m20_s0 | 123 / 123 | 2.635 | 1.190 | - |
| 3 | sys3x2_N3 | 211 / 211 | 3.674 | 1.090 | - |
| 3 | sys3x2_N4 | 503 / 503 | 10.286 | 1.091 | - |
| 3 | sys3x2_N5 | 973 / 973 | 22.097 | 1.085 | - |
| 3 | sys3x2_N6 | 1503 / 1503 | 38.172 | 1.050 | - |
| 3 | sys3x2_N7 | 2095 / 2095 | 52.531 | 1.023 | - |
| 4 | masses2_N4 | 307 / 307 | 15.625 | 1.297 | - |
| 4 | masses2_N5 | 493 / 493 | 27.123 | 1.304 | - |
| 4 | masses2_N6 | 901 / 901 | 52.033 | 1.315 | - |
| 4 | masses2_N7 | 1641 / 1641 | 96.950 | 1.260 | - |
| 4 | masses2_N8 | 2837 / 2837 | 198.842 | 1.224 | - |
| 4 | rand_t4_x5_m20_s0 | 797 / 797 | 37.658 | 1.165 | - |
| 4 | rand_t4_x5_m20_s1 | 484 / 484 | 22.328 | 1.327 | - |
| 5 | rand_t5_x5_m20_s0 | 1423 / 1423 | 201.750 | 1.109 | - |
| 5 | rand_t5_x5_m20_s1 | 1136 / 1136 | 113.353 | 1.269 | - |
| 5 | rand_t5_x5_m20_s2 | 2301 / 2301 | 191.578 | 1.307 | - |
| 5 | rand_t5_x6_m24_s0 | 5036 / 5036 | 711.770 | 1.198 | - |
| 6 | masses3_N3 | 955 / 955 | 361.553 | 1.230 | - |
| 6 | masses3_N4 | 3643 / 3643 | 2549.854 | 1.144 | 1.000 |
| 6 | pop_017 | 4142 / 4142 | 770.129 | 1.574 | - |
| 6 | rand_t6_x5_m20_s0 | 1774 / 1774 | 250.844 | 1.465 | - |
| 7 | pop_005 | 10289 / 10289 | 5225.860 | 1.221 | 1.006 |
| 7 | pop_021 | 679 / 679 | 173.697 | 1.759 | - |
| 8 | masses4_N3 | 2891 / 2891 | 4207.052 | 1.160 | 1.001 |
| 8 | pop_027 | 1472 / 1472 | 553.722 | 1.769 | - |
| 8 | rand_t8_x5_m20_s0 | 4111 / 4111 | 1001.726 | 1.583 | - |
| 10 | masses5_N2 | 370 / 370 | 485.721 | 1.311 | - |
| 10 | rand_t10_x5_m15_s0 | 1859 / 1859 | 597.682 | 1.455 | - |
| 10 | rand_t10_x5_m20_s0 | 6124 / 6124 | 2642.795 | 1.603 | - |
| 12 | masses6_N2 | 380 / 380 | 910.798 | 1.253 | - |
| 12 | rand_t12_x5_m15_s0 | 1012 / 1012 | 473.212 | 1.472 | - |
| 12 | rand_t12_x5_m20_s0 | 6143 / 6143 | 5598.867 | 1.497 | 1.005 |
| 14 | masses7_N2 | 390 / 390 | 1535.917 | 1.369 | - |
| 14 | rand_t14_x5_m15_s0 | 2470 / 2470 | 1952.926 | 1.472 | - |
| 16 | masses8_N2 | 395 / 395 | 2113.723 | 1.345 | - |
| 16 | rand_t16_x5_m15_s0 | 1876 / 1876 | 1917.024 | 1.382 | - |

A/A, b522425 against a copy of itself, on the four slowest problems: 1.000 to 1.006.

## Tests and gates

* `cargo test --profile gate`, one 1-CPU task per test (array 45889724, 147 tests listed), on the merged tree
  (main f0e1084 plus the rank-1 commits up to ccbbe18; the later commits only change census counters): 144 passed,
  0 failed, 3 ignored.
* Both builds compile: default and `--features lp_census` (`cargo check --tests --examples`).
* Region hashes: 141 of 147 comparable problems identical to b522425, six differ (above). `lexcheck`: see above.
