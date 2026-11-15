# RpoptProofs: formal verification of `mplcp` and `mplp_lex`

Lean 4 and Mathlib proofs of the mathematics behind two RPOPT solvers:

- `mplcp` (`src/algorithms/mplcp.rs`): parametric LCP pivoting for strictly convex mp-QPs,
  Jones and Morari (2006);
- `mplp_lex` (`src/algorithms/mplp_lex.rs`): lexicographic perturbation for mp-LPs, Jones, Kerrigan
  and Maciejowski (2007).

The proofs are at the model level, in exact real arithmetic. They justify the steps the Rust code
takes, not its floating point execution. Every result listed as proved has no `sorry`, and
`Audit.lean` prints the axioms of all 80 main theorems: each uses only `propext`,
`Classical.choice` and `Quot.sound` (two use none). Modelling assumptions are explicit hypotheses,
never axioms.

Rust line numbers refer to main at commit `663a59e` (mplcp.rs is unchanged since `2b0c6c7`).

## Build

Toolchain `leanprover/lean4:v4.35.0` (via elan), Mathlib rev `56a005855ea17e0b0958f73ec801ae140b0a1679`.

```sh
cd ppopt_rs/verification/lean
# reuse an already built Mathlib (APFS copy on write: no rebuild, no extra disk). Report/ is not
# tracked by git, so from a worktree use the path in the main checkout:
cp -c -R "/Users/markchen/Documents/Research/JSP/QC-centric/Flexible QC/Report/Fluid Relaxation draft/lean/ScrapFrontier/.lake/packages" .lake/
# or download the prebuilt cache instead:  lake exe cache get
lake build                  # about 3 minutes on top of a built Mathlib
lake env lean Audit.lean    # one line per main theorem: the axioms it depends on
```

`.lake/` is ignored by git. Checking the proofs is local; Mathlib is only on the laptop.

## Layout

| File | Content |
|:-----|:--------|
| `Foundations/QP.lean` | strictly convex QP, KKT conditions, sufficiency and uniqueness |
| `Foundations/LCP.lean` | the pLCP of the KKT system (`M`, `q`, `Q`, `x(λ)`), uniqueness of `w` |
| `Foundations/Chebyshev.lean` | points of a Chebyshev ball are feasible (start samplers) |
| `Tableau.lean` | complementary bases, semantic tableaux, basic solutions, region law |
| `Pivot.lean` | dictionary pivot, principal pivot, `tableau_for` |
| `Lex.lean` | lexicographic order, symbolic `ε`, perturbation block from `T`, uniqueness |
| `Facet.lean` | Case 1 of Algorithm 2 (the fast path of `adjacent`) |
| `Algorithm2.lean` | the Case-2 lexicographic test of `pivot_facet` |
| `FacetWalk.lean` | Case 2 at a facet point: walk invariant, terminal soundness, cover, dependent rows |
| `Termination.lean` | no repeated state on the facet walk; what the two caps can be replaced with |
| `Coverage.lean` | breadth-first exploration as a reachability closure, skipping rules |
| `Optimizations/EntryFacetSkip.lean` | safety of the entry-facet skip (merged) |
| `Optimizations/KnownRegionSkip.lean` | safety of the known-region skip of Case-2 facets (step 2) |
| `MplpLex/Certificate.lean` | LP weak duality and the basis certificate |
| `MplpLex/Pivot.lean` | ratio tests, rank-1 basis update, dual and primal simplex steps |
| `MplpLex/LexUnique.lean` | uniqueness of the lexico-optimal basis |
| `Audit.lean` | `#print axioms` for every main theorem |

## Status

| ID | Lean theorem | Statement | Status |
|:---|:-------------|:----------|:-------|
| A1 | `kkt_unique_minimizer` | a KKT point of a strictly convex QP is its unique global minimizer | proved |
| A1 | `lcp_unique_optimizer` | every pLCP solution `(w, λ)` gives the unique optimizer `x(λ) = -H⁻¹(Fθ + c + Aᵀλ)` | proved |
| A1 | `kkt_lcp` | a KKT pair of the QP is a pLCP solution, with `x = x(λ)` | proved |
| A1 | `optimizer_iff_lcp` | optimizer ⇔ `x(λ)` for a pLCP solution | proved under the hypothesis `KKTNecessity` |
| A1 | `KKTNecessity` | an optimizer has a KKT multiplier (Farkas) | **open**, stated as a named hypothesis |
| A1 | `lcp_w_unique`, `lcpM_posSemidef` | `M = AH⁻¹Aᵀ ⪰ 0`; all LCP solutions share `w` | proved |
| A2 | `basic_solves` | with `ρ ≥ 0` the basic solution of a tableau solves the LCP | proved |
| A2 | `region_law_optimal` | on `CR_B = {θ : ρ(θ) ≥ 0}` the region's law is the unique optimizer | proved |
| A2 | `law_eq_kkt` | the law read from the tableau equals any solution of the KKT system of the active set | proved |
| A2 | `tableau_unique` | a tableau is determined by its basis (pivoted = rebuilt) | proved |
| A3 | `exchange_iff`, `pivot_isTableau` | a pivot with nonzero element has the same solution set, pair exchanged | proved |
| A3 | `pivot_agree_on_facet` | where the pivot row is zero both tableaux give the same values | proved |
| A3 | `tableauFor_isTableau` | principal pivots on distinct pairs with nonzero elements give the tableau of that basis | proved |
| A3 | `pivotSeq_eq_of_isTableau` | a chain of pivots of any length gives the same tableau as any other construction of that basis (rebuilt, other chain, pivots applied later) | proved |
| A4 | `Case1.facet_eq` | the shared facet is the same set from both sides: `CR_B ∩ H = CR_B' ∩ H` | proved |
| A4 | `Case1.cr_le`, `Case1.cr'_ge` | the two regions lie on opposite sides of the facet hyperplane | proved |
| A4 | `Case1.across`, `Case1.own_side` | near every relative-interior facet point, `B` covers its side and the toggled `B'` the other | proved |
| A4 | `Case1.lex_cover` | the same, lexicographically: near the facet `B` or `B'` is lexico-feasible at every point | proved |
| A5 | `lex_unique`, `lexRegion_disjoint` | two different lexico-feasible complementary bases are never valid at the same `θ` | proved |
| A5 | `LexPos.eventually_pos` | a lexico-positive row is positive for all small `ε > 0` (symbolic `ε`) | proved |
| A5 | `isLexTableau_of_isTableau`, `lexRows_pivot` | the perturbation block is implied by `T` (`rfull`) and pivots like the other columns | proved |
| A5 | `lex_ratio_pivot`, `lex_ratio_pivot_on` | the lexicographic ratio test keeps every (tracked) row lexico-positive | proved |
| Alg. 2 | `case2_accept_unique`, `gamma_scale` | the Case-2 test accepts at most one candidate when no `Γ` vanishes; it is scale-free | proved |
| Alg. 2 | `first_step_nonAlpha` | the first pivot (α enters) makes every row except α's lexico-positive, even from a basis that is lexico-infeasible on the facet | proved |
| Alg. 2 | `walk_sound`, `terminal_rows`, `ray_lexFeasible` | **soundness**: a walk from a genuine facet that ends at the terminal push returns a basis that is lexico-feasible at `θ_f + t g` for all small `t > 0` | proved |
| Alg. 2 | `case2_lexmin_accepted` | **cover**: at every facet point the lexicographic minimum candidate passes all its inequalities (16), so the Case-2 sub-facets cover the facet | proved |
| Alg. 2 | `neighbour_unique_on_ray` | the basis lexico-feasible just across a facet point is unique | proved |
| Alg. 2 | `lcp_multipliers_dependent`, `case2_needs_dependent_rows` | two multiplier vectors at one `θ` differ in `ker Aᵀ` on the active rows: a Case-2 step that keeps α fixed needs dependent active rows | proved |
| Alg. 2 | completeness | every lexico-feasible neighbour across `F` is returned | **conditional** on `WalkEndsWithPush` and the work-tableau link (below), and on no cap firing |
| Caps | `walk_no_repeat` | **3a**: from a start basis that is lexico-feasible at the facet point, no walk state repeats | proved |
| Caps | `walk_repeat_returns` | in general, the only possible repetition is a return to the state after the first pivot | proved |
| Caps | `chain_length_bound` | a non-repeating walk has at most as many pivots as there are basis label configurations | proved |
| Caps | no repetition from a lexico-infeasible start | whether the walk can cycle back to its first state | **open** |
| A6 | `reach_least`, `reach_skip` | exploration = least closed set; skipping explored candidates changes nothing | proved |
| A6 | facet-graph connectivity | the facet-adjacency graph of a facet-to-facet partition of a convex set is connected | **open**, hypothesis of `coverage_of_connected` |
| Skip 1 | `entry_facet_unique_neighbour`, `reach_skip_parent` | across the entry facet of a Case-1 child, the only lexico-feasible basis on the parent's side is the parent | proved (needs a lexico-feasible parent) |
| Skip 2 | `known_region_unique_neighbour`, `known_region_skip` | if a known lexico-feasible region `R` on the other side contains `F`, every lexico-feasible basis across `F` is `R`'s; skipping `F` keeps the explored set | proved |
| Skip 2 | `vertices_subset_polyhedron` | the vertices of `F` in `R` imply `F ⊆ R` | proved |
| Facets | `relint_of_strict` | a point on the facet plane that clears every other row strictly is a relative-interior point of the facet | proved |
| Start | `chebyshev_ball_feasible` | every point within the radius of a Chebyshev centre satisfies every inequality row | proved |
| B1 | `lp_certificate`, `basis_certificate`, `lp_weak_duality` | primal and dual feasibility (with complementarity) imply optimality | proved |
| B2 | `rank1_inverse`, `rank1_tableau` | `Lex::pivot` keeps `β = A_B⁻¹` and `T = Aβ` | proved |
| B2 | `dual_pivot` | phase 1: the ratio test keeps the multipliers dual feasible and drives the leaving one to 0 | proved |
| B2 | `primal_direction`, `ratio_test`, `lex_ratio_test` | phase 2: the edge direction and the (lexicographic) ratio test keep primal feasibility | proved |
| B3 | `lexOptimal_unique`, `nondeg_basis_unique` | the lexico-optimal basis is unique for each `θ` (no overlaps) | proved |
| B3 | gap-freeness | every feasible `θ` has a lexico-optimal basis (termination of the lexicographic simplex) | **open** |

Named open pieces of Case 2:

- `WalkEndsWithPush`: at every relative-interior facet point the walk ends at the terminal push
  (not with `P` empty or with α leaving). This is a Lemke-type existence statement for the
  positive semidefinite pLCP, not proved here.
- The work-tableau link: the work tableau with α nonbasic is, for `α = t`, the tableau of the
  returned complementary basis at `θ_f + t g`. It holds by the construction in `adjacent`
  (mplcp.rs 653-666) and `exchange_iff`, but the labelled dictionary (rows not aligned with pairs
  after non-principal pivots) is not formalized, so `walk_sound` states the conclusion on the rows.

## Answers on the two caps (task 3)

**3b, recursion depth (`depth > 6`, mplcp.rs 462).** Bounded in exact arithmetic when the walk
starts from a basis that is lexico-feasible at the facet point, but not by a structural constant
such as `|Z|` or the facet dimension:

- Pointwise in `θ_f`, one branch of the recursion at depth `d` is a walk with `d` Case-2 steps
  (Case 1 and Case 2 are the same lexicographic rule pointwise).
- `walk_no_repeat` shows the walk visits no state twice. `chain_length_bound` then bounds the
  depth by the number of basis label configurations the walk can reach. That number is finite but
  exponential: at most about `n 2^n` almost-complementary bases with their entering column.
  A sharper count, argued from the proved lemmas but not composed into one Lean theorem: when α
  stays on the facet, every state describes the same slacks and primal point
  (`lcp_multipliers_dependent`), so only the pairs active at the facet point change, about `a 2^a`
  states with `a` the number of active constraints there.
- `case2_needs_dependent_rows`: a Case-2 step that keeps α fixed moves only multipliers, along a
  dependency of the active rows. So deep recursion is a symptom of dependent active rows, not a
  defect, and no small bound in terms of `|Z|` follows.
- Measured (laptop debug build, uncommitted): masses3_N3 reaches depth 13 with the cap at 200 and
  no repeated state; the region count is unchanged (955).
- Replacement: no depth limit; an explicit work stack instead of recursion. For each branch,
  remember the state after the first pivot. By `walk_repeat_returns` any repetition is a return to
  that state, so comparing with it detects every cycle exactly (a visited set of `(row_var, e)`
  states does the same at higher cost). On a detected cycle, or on a budget derived from the bound,
  report it and fall back to a robust search for that facet (for example, solve the QP just across
  the facet and take its active set). Never return silently: the guard currently drops neighbours.

**3a, the inner loop (`8n + 50`, mplcp.rs 390).** Each iteration is a step of the same walk, so
`walk_no_repeat` applies: from a lexico-feasible start the loop cannot revisit a state, and it
ends within the number of reachable states. The census never reached it. The cap can become the
same cycle check (compare with the first state), with the bound as a safety net that reports
rather than returns.

## Correspondence with the Rust code

`mplcp.rs` at `663a59e` unless noted; `build` is `mplcp_build.rs`.

| Lean theorem | Rust function or step | What it justifies |
|:-------------|:----------------------|:------------------|
| `lcpM`, `lcpq`, `lcpQ`, `slack_primalOf` | `Plcp::new` (50-117) | the pLCP data `M, q, R` from the KKT cache |
| `lcp_unique_optimizer`, `region_law_optimal` | `tableau_law` (926), `complete_primal_law` | `x(θ) = -H⁻¹(c + Hθ + Aᵀλ(θ))` is the optimizer on the region |
| `law_eq_kkt` | `TABLEAU_LAW` switch (900), `tableau_law` vs `optimal_control_law_dual` | the tableau law equals the KKT-solve law |
| `tableau_unique`, `pivotSeq_eq_of_isTableau` | `REFACTOR` (745), `TabSrc::Lazy` and `resolve` (750-778), `candidate_law` (963), `pivot_solve` (1054-1065) | a tableau reached by a chain of pivots, applied when the child is popped, equals the one rebuilt from scratch, for any period |
| `isTableau_empty`, `tableauFor_isTableau` | `tableau_for` (297-314; check `piv.abs() < 1e-11` at 308) | the region tableau rebuilt from an active set |
| `exchange_iff`, `pivVec`, `pivT` | `Tab::defer_pivot` (202), `Tab::flush` (263), `Tab::pivoted` (226), `Tab::tv`/`rv` (157-193) | the pivot formulas |
| `pivot_isTableau`, `toggle` | `Nbr::Toggle` (644, used at 884), `z_toggle` (536) | the neighbour's tableau is one principal pivot away |
| `Case1` (fields), `Case1.facet_eq` | `adjacent` (601): `zrows` (608-625), `pv` (632), `tve` (638), `blocked` (640) | the exact conditions of the fast path |
| `Case1.across`, `Case1.lex_cover` | fast path return (644) | the toggled basis is the region across the facet |
| `entry_facet_unique_neighbour` | entry-facet skip in `expand_candidates` (824-846), `Item::entry` (1064) | the skipped search could only return the parent |
| `LexPos`, `LexNonneg` | `lex_nonneg` (341-348), `lex_cmp` (327-338) | lexicographic sign, without tolerance |
| `LexPos.eventually_pos` | header (7-9): `ε = (ε0, ε0², …)` | symbolic `ε` is sound |
| `lexRows`, `isLexTableau_of_isTableau` | `rfull` (409-422), `rowmax` (423); `lex_feasible` (1427) | the β block is `-T[:, c]` for nonbasic `w_j`, a unit vector for basic `w_j` |
| `lex_ratio_pivot_on`, `zero_row_beats_positive_row`, `lexmin_zero_row` | Case 1 (438-458): lexmin over `P ∩ Z` on β | the Case-1 pivot keeps the non-α rows lexico-positive |
| `WalkState`, `WalkStep` | `Tab` with `row_var`, `col_var` and the entering column `e`; `e = col_of(comp(leaving))` (457, 519) | the walk of `pivot_facet` |
| `first_step_rows`, `first_step_nonAlpha` | first iteration, `e` = the α column of the work tableau (653-666) | the first pivot establishes the walk invariant |
| `pivVec_num_zero` | Case 1 (zero leaving row) and Case 2 (α's coefficient within `PTOL`) | α stays on the facet |
| `terminal_rows`, `ray_lexFeasible`, `walk_sound` | terminal push `z_after` (431-437) | the returned basis is lexico-feasible just across the facet |
| `case2_lexmin_accepted`, `case2_accept_unique`, `gamma_scale` | Case 2 (459-529): Γ (477), `gq_zero` (478), `lex_nonneg` (482), halfspaces (484-490), Chebyshev (496-506) | the sub-facets cover the facet and at most one candidate wins at each point |
| `neighbour_unique_on_ray` | `out.push(z_after(...))` (434, 513) | one neighbour per facet point |
| `case2_needs_dependent_rows` | slow path entered with `tve ≈ 0` (638); Case 2 (459) | Case 2 moves multipliers among dependent active rows |
| `walk_no_repeat`, `walk_repeat_returns`, `chain_length_bound` | `for _ in 0..(8 * n + 50)` (390), `if depth > 6` (462) | what the caps can be replaced with |
| `known_region_unique_neighbour`, `known_region_skip`, `vertices_subset_polyhedron` | step 2 of branch `mplcp-case2-fix` (not yet merged) | skipping a Case-2 facet contained in a known region |
| `relint_of_strict` | mplp_lex.rs facet point: foot of the region's Chebyshev centre (532-556), else the facet's Chebyshev LP (557-578) | the point used for the lexicographic neighbour is in the facet's relative interior |
| `chebyshev_ball_feasible` | program.rs `ball_points` (1225), `feasible_space_chebychev_ball` (1075) with the Clarabel fallback (1240); used by `gen_optimal_active_set` and `degenerate_start` | sampled start parameters are feasible (inequality rows) |
| `reach_least` | `pivot_solve` (1028), `pivot_solve_par` (1082), `seen` (1031, 1042, 1122) | the explored set does not depend on the visiting order |
| `reach_skip` | `seen` check before `build_cand` (1042); mplp_lex `crossed` (mplp_lex.rs 502, 509, 525-527, 592) | skipping known neighbours is safe |
| `lp_certificate`, `basis_certificate` | mplp_lex.rs header (13-16), `Lex::solve` (264-377), `Ok(())` at 358 | a basis with no lexico-negative slack or multiplier row is optimal |
| `rank1_inverse`, `rank1_tableau` | mplp_lex.rs `Lex::pivot` (159-181) | the rank-1 update of `binv` and `t` |
| `dual_pivot`, `ratio_test` | mplp_lex.rs phase 1 (310-342), ratio `l / y` with `y > 1e-9` (324-338) | phase 1 keeps dual feasibility |
| `primal_direction`, `lex_ratio_test` | mplp_lex.rs phase 2 (343-374), `aj = -T_jp` (365), `slack_cmp` (226-258) | phase 2 keeps primal lexico-feasibility |
| `lexOptimal_unique` | mplp_lex.rs header (17-19), `slack_sign` (196-223), `lam_row` (380-395) | one lexico-optimal basis per `θ` |

## Modelling assumptions

All of these appear as explicit hypotheses of the theorems that need them.

- **Exact arithmetic.** `ZTOL`, `PTOL`, `1e-11`, `1e-12` and the point-location tolerance `1e-9` are 0.
  Tests such as `gam[i] < -PTOL` become `γ_i < 0`.
- **Strict convexity.** `H` is positive definite (`H.PosDef`, or `StrictlyConvex`).
- **Nonzero pivots.** `tableau_for` and the toggles use nonzero pivot elements (`NonzeroPivots`,
  `T v v ≠ 0`); the code checks `|piv| ≥ 1e-11`.
- **Principal-pivot tableaux** for regions (rows and columns indexed by the pairs). The facet walk
  uses labelled tableaux (`WalkState`).
- **Pointwise lexicographic rule in Case 2.** At a facet point the leaving row is the strict
  lexicographic minimum ratio over all decreasing rows (`WalkStep`). The code realises this with
  halfspaces in `θ_f` plus a β test when the `θ_f` part vanishes, which agrees with it off a
  lower-dimensional set.
- **Inequality rows only in the pLCP.** The Schur-complement elimination of equality rows in
  `Plcp::new` (74-115) is not modelled.
- **Relative-interior facet points.** The facet results hold near points where every row outside
  the vanishing set is strictly positive.
- **Lexico-feasible regions in the skips.** The entry-facet skip needs a lexico-feasible parent, the
  known-region skip a lexico-feasible known region `R`.
- **KKT necessity**, **facet-graph connectivity**, **`WalkEndsWithPush`** and the **work-tableau link**
  are hypotheses, see Status.

## Not covered

- Floating point, rounding, and every tolerance listed above.
- The sub-facet polytopes and the Chebyshev radius `> 1e-9` (pieces without interior are dropped);
  the recursion itself beyond its pointwise walk.
- The region builder (`build_region`, `build` 14-137, and `gen_cr_from_prepared` in `mpqp_utils.rs`): vertex
  clipping (now up to 5 parameters), the LP redundancy removal above that, `remove_duplicate_rows`, the
  Chebyshev center and the facet hint. The model treats a region as the set `CR`; its row list only matters
  through the facets that get searched.
- Tableau refactoring and drift (`REFACTOR`, `mplp_lex` `REFRESH`), beyond the exact statement
  `tableau_unique`.
- Start-region selection (`initial_region` 792, `degenerate_start`, `lex_feasible_start` 1579,
  `drop_lex_infeasible` 1491, `mplp_lex` initial basis). Only the feasibility of the sampled points is
  covered (`chebyshev_ball_feasible`). Past bugs of this kind, such as the zero-region
  start from dependent active rows fixed in `668403e`, live at this level.
- The enumeration method (`Method::Enumeration`) and the parallel scheduling of `pivot_solve_par`
  (only its result set, via `reach_least`).
- In `mplp_lex`: the `γ` infinitesimal along the facet normal and the concrete formulas of the lexicographic
  rows (abstract coefficient vectors in `lexOptimal_unique`). The relative-interior point is covered by
  `relint_of_strict` in exact arithmetic; the `1e-6` margin is outside.

## Findings for the implementation

1. **Case-2 test reads `Γ` through an absolute tolerance** (`pivot_facet`, 477-482; fix in progress on
   `mplcp-case2-fix`). `Γ = a_l row_j - a_j row_l` scales with `a_l a_j`. On `pop_001` the twin rows 4 and
   23 have `a ≈ 1.2e-10`, so `Γ`'s β part, about `±1.2e-10`, reads as zero in both directions and both
   twins are accepted. In the model this violates the hypothesis `Γ ≠ 0` of `case2_accept_unique`.
   `gamma_scale` shows the scale-free vector gives the same exact answer, and the test must also be
   relative to the vector's largest entry.
2. **The entry-facet skip needs a lexico-feasible parent** (`hP`), now that it is merged. A
   lexico-infeasible twin parent (possible until finding 1 is fixed, and removed only at the end by
   `drop_lex_infeasible`) has its lexico-feasible twin across the entry facet, which the skip no longer
   searches for. The same holds for the known region of step 2 (`hlex`).
3. **`in_z` uses an absolute tolerance, `zrows` a relative one** (`pivot_facet` 428 against `adjacent`
   619-620), and this now has measured consequences. In a laptop debug build at `f017013`:
   - first-iteration Case 2 (α entering as a Case-2 candidate, so a finite α step off the facet) happens
     2859 times on pop_005, 672 on masses3_N3 and 4161 on masses3_N4;
   - the vanishing row's rounding error is mostly between `1e-9` and `1e-7`;
   - with `in_z` at `1e-6`, the first-iteration Case 2 count falls to 7 on pop_005 and 18 on masses3_N3;
   - the `depth > 6` returns fall from 11279 to 5758 on pop_005, with identical region counts on all three
     problems.

   In the model the first pivot is Case 1 at every genuine facet, and α then stays at 0
   (`pivVec_num_zero`). Later α steps off the facet (about 31k of 67k later Case-2 steps on pop_005)
   follow from this misclassification and from Case-2 steps with α's coefficient at or above `PTOL`.
4. **The slow path is mostly a zero pivot element, not a degenerate facet.** In almost every slow call
   (pop_005 55761 of 55803) there is exactly one vanishing row `v`, but `tve ≈ 0`: the principal pivot
   element `T[v,v]` vanishes, so the toggled basis would have dependent active rows (`Case1` needs
   `T v v > 0`). Case 2 then walks among the dependent rows (`case2_needs_dependent_rows`).
5. **Cycles on pop_005.** With the depth cap raised to 200, the same debug build found 7271 repeated
   `(row_var, e)` states along single branches of pop_005 (none on masses3_N3, max depth 13). From a
   lexico-feasible start this is impossible in exact arithmetic (`walk_no_repeat`), so these cycles come
   from the tolerance tests (findings 1 and 3) or from the open lexico-infeasible-start case. In either
   case `walk_repeat_returns` says the first repeated state is the state after the first pivot.
6. **The `depth > 6` guard drops neighbours silently** (462). See "Answers on the two caps" for the
   replacement.
7. **Lazy child pivots and `REFACTOR = 32` (2b0c6c7) match the model.** `TabSrc::resolve` applies the same
   principal pivot to the unchanged shared parent (the in-place pivot only for the last holder, 767-772), so
   it equals the eager pivot, and `pivotSeq_eq_of_isTableau` makes the period irrelevant in exact arithmetic.
   The pivot element of a `Toggle` is nonzero by the fast-path test (`tve > PTOL`, 638-639), as `Case1`
   assumes. Floating-point drift, measured in `bench/results/mplcp_tableau_handling.md`, is outside the model.
8. **Region construction and start fallback (663a59e) stay outside the model, with one caveat.** Vertex
   clipping up to 5 parameters, the LP redundancy path above, the reuse of prepared rows
   (`gen_cr_from_prepared`) and pairwise `remove_duplicate_rows` all describe the same set in exact
   arithmetic, so the region theorems are unaffected. The facet theorems do assume that every facet of a
   region is in its row list: a row dropped at the tolerance edge in a sliver is a facet whose neighbour
   search does not run (coverage, outside the model). The doc comment of `ball_points` (program.rs 1222-1224)
   says every sampled parameter is feasible; `chebyshev_ball_feasible` proves this for inequality rows only.
   With equality rows the Chebyshev LP fixes them at the centre, and moving θ alone breaks them unless
   `F_E u = 0`. Such samples are rejected by the QP solve, so this is a wording issue, not a wrong result.

The measurements in findings 3 to 5 come from an uncommitted debug build run on the laptop (runs of 4
to 30 s). They are diagnostics, not timings.

## Extending the proofs

New optimizations of `mplcp` get a file in `Optimizations/` that reduces to the lemmas above:

- **Known-neighbour skipping** (skip a facet whose neighbour is already known): `reach_skip` with the
  hypothesis that the skipped neighbour is explored. Across a Case-1 facet the neighbour is given by
  `entry_facet_unique_neighbour`, across a Case-2 facet contained in a known region by
  `known_region_unique_neighbour`.
- **Pruning in the degenerate branching** (Case 2): show that a pruned branch leads only to bases that
  are not lexico-feasible near the facet, or to explored ones (`neighbour_unique_on_ray`,
  `case2_accept_unique`, `reach_skip`).
- **Cycle check instead of the caps**: `walk_repeat_returns` justifies comparing with the state after
  the first pivot; `walk_no_repeat` says the check never fires from a lexico-feasible start.
- **Closing Case-2 completeness**: formalize the labelled dictionary semantics (work-tableau link), and
  prove `WalkEndsWithPush` (Lemke termination with a solution for positive semidefinite `M`).
- **Region rows read from the tableau**: `CR` is already defined from the tableau rows, and
  `law_eq_kkt` covers the slack rows as well as the multipliers.
- **Skipped Chebyshev LP**: the facet theorems only use a relative-interior facet point (`hrel`) and a
  point of the region on its side (`gam_eq_zero`). By `Case1.facet_eq`, the parent's facet point lies
  on the child's facet too.
- **Equality rows**: prove that the Schur-complement elimination of `Plcp::new` preserves the solution
  set and lift A1 to free multipliers.
- **KKT necessity**: closedness of finitely generated cones (conic Carathéodory) plus
  `ProperCone.hyperplane_separation_point` gives Farkas' lemma, which discharges `KKTNecessity`.
