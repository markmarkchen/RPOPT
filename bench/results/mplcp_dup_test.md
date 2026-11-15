# mplcp near-duplicate test for thin candidates (#93): failed its gate, left off

Branch `overlap-test` (on refine-on 336cfa4 and main 0ebb377). Code: `near_duplicate` in `src/algorithms/mplcp.rs`, switch `set_dup_test` / `MPLCP_DUP=1` (default off).

## The test

A candidate whose Chebyshev radius is below 1e-6 is rejected when its centre lies inside a built region whose active set is one swap, one insertion or one removal away (looked up in `Seen`, |S| m + |S| + m lookups, only for thin candidates), by at least a fraction of the candidate's radius.

## Results, same binary with the test off and on

Version 1 (margin 1e-9 inside the neighbour): 262 sets removed on 17 of 158 problems (none added). `pair_check` of the removed sets (MPLCP_DUP=1 solve): 125 genuine (QP at 16 half-radius points returns exactly the set), 79 spurious (QP returns another set), 55 not buildable from the set alone, 3 mixed. The many genuine ones had centres 0.99 to 1.00 radii inside a kept region of a different set (masses4_N3 [13,23,40] inside [3,23,40]), that is, the same thin polytope under two active sets for which the QP returns either one; others were only 0.03 radii deep (genuine neighbours). So a depth floor is needed.

Version 2 (centre at least 0.5 radius inside): 172 sets removed on 16 of 158 problems, none added. Audit (`set_audit`, old = c345cdc, ten masses problems, this tree with the test off / on):

| | test off | test on |
|---|---:|---:|
| c1 overlap, new only ("gained") | 61 | 52 |
| c1 overlap, lost | 1 | 184 |
| a_lost (genuine regions in old and oracle, absent from new) | 55 | 290 |
| b_gained | 457 | 456 |

## Verdict

The test removes 9 of the 61 new-only overlaps and, with them, 235 regions that c345cdc and the walk-only oracle also have (their pair partners are retained, so each removal is a dropped member of an overlapping pair, but both members are produced by the reference solves). It does not keep "every region that is identical today". The depth of the 62 overlaps (centre depth over own radius) is spread evenly from 0.03 to 1.0, so no depth floor separates the new-only overlaps from the pairs the reference solves share. Not merged; the switch stays off.
