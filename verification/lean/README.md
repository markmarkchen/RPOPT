# RpoptProofs: formal verification of `mplcp` and `mplp_lex`

Lean 4 and Mathlib proofs of the mathematics behind two RPOPT solvers:

- `mplcp` (`src/algorithms/mplcp.rs`): parametric LCP pivoting for strictly convex mp-QPs,
  Jones and Morari (2006);
- `mplp_lex` (`src/algorithms/mplp_lex.rs`): lexicographic perturbation for mp-LPs, Jones, Kerrigan
  and Maciejowski (2007).

The proofs are at the model level, in exact real arithmetic. They justify the steps the Rust code
takes, not its floating point execution. Every result listed as proved has no `sorry`, and
`Audit.lean` prints the axioms of all 126 main theorems: each uses at most `propext`,
`Classical.choice` and `Quot.sound` (two use none, one only `propext` and `Quot.sound`). Modelling
assumptions are explicit hypotheses,
never axioms.

Rust line numbers refer to main at commit `59b2afb`.

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
| `Foundations/Farkas.lean` | checked infeasibility certificates on a box (the DAQP emptiness filter) |
| `Foundations/Chebyshev.lean` | points of a Chebyshev ball are feasible (start samplers) |
| `Tableau.lean` | complementary bases, semantic tableaux, basic solutions, region law |
| `Pivot.lean` | dictionary pivot, principal pivot, `tableau_for` |
| `Lex.lean` | lexicographic order, symbolic `ε`, perturbation block from `T`, uniqueness |
| `Facet.lean` | Case 1 of Algorithm 2 (the fast path of `adjacent`) |
| `Algorithm2.lean` | the Case-2 lexicographic test of `pivot_facet` |
| `FacetWalk.lean` | Case 2 at a facet point: walk invariant, terminal soundness, cover, dependent rows |
| `DependentFacet.lean` | the dependent-row facet: the neighbours across it are `T \ {j}`, `j ∈ P⁻` (direct enumeration) |
| `Termination.lean` | no repeated state on the facet walk; what the two caps can be replaced with |
| `Coverage.lean` | breadth-first exploration as a reachability closure, skipping rules |
| `PivotMeasure.lean` | the off-`A` block of a tableau is positive semidefinite: sign, size and zero pattern of the pivot of a vanishing slack row |
| `Optimizations/EntryFacetSkip.lean` | safety of the entry-facet skip (merged) |
| `Optimizations/KnownRegionSkip.lean` | safety of the known-region skip of Case-2 facets (step 2) |
| `MplpLex/Certificate.lean` | LP weak duality and the basis certificate |
| `MplpLex/Pivot.lean` | ratio tests, rank-1 basis update, dual and primal simplex steps |
| `MplpLex/LexUnique.lean` | uniqueness of the lexico-optimal basis |
| `MplpLex/Crossing.lean` | the direct facet crossing: no admissible ratio-test row means nothing feasible beyond the facet |
| `MplpLex/Enumeration.lean` | the enumeration path for H = 0: conditions (D) and (Z), full-dimensional regions, no overlaps |
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
| Alg. 2 | `case2_accept_unique`, `gamma_scale`, `gamma_scale_nonneg` | the Case-2 test accepts at most one candidate when no `Γ` vanishes; it is scale-free (also in the lexico-nonnegative form the code tests) | proved |
| Alg. 2 | `first_step_nonAlpha` | the first pivot (α enters) makes every row except α's lexico-positive, even from a basis that is lexico-infeasible on the facet | proved |
| Alg. 2 | `walk_sound`, `terminal_rows`, `ray_lexFeasible` | **soundness**: a walk from a genuine facet that ends at the terminal push returns a basis that is lexico-feasible at `θ_f + t g` for all small `t > 0` | proved |
| Alg. 2 | `case2_lexmin_accepted` | **cover**: at every facet point the lexicographic minimum candidate passes all its inequalities (16), so the Case-2 sub-facets cover the facet | proved |
| Alg. 2 | `neighbour_unique_on_ray` | the basis lexico-feasible just across a facet point is unique | proved |
| Alg. 2 | `lcp_multipliers_dependent`, `case2_needs_dependent_rows` | two multiplier vectors at one `θ` differ in `ker Aᵀ` on the active rows: a Case-2 step that keeps α fixed needs dependent active rows | proved |
| Rebuild | `rebuilt_eq_pivoted`, `pivotSeq_perm` | the tableau rebuilt from a child's active set equals the parent's pivoted one, in any pivot order (tableau budget, relative-toggle hand-over, the retry of `tableau_for`) | proved |
| DAQP filter | `farkas_box`, `farkas_box_signed`, `facet_coord_le`, `box_sq_le`, `daqp_box_certificate` | `p ≥ 0` and `g + ‖r‖₁ R < 0` exclude every point with `‖x‖∞ ≤ R`; the signed version; facet coordinates are no larger than the parameters; with `R` the Euclidean radius of the parameter box, no parameter of the box passes | proved |
| Pivot | `pivot_nonneg`, `pivot_cauchy_schwarz`, `tableau_diag_le`, `column_le_pivot`, `slack_column_zero_of_pivot_zero` | for a vanishing slack row `c`: `T c c ≥ 0`; `T k c ² ≤ T k k · T c c ≤ M k k · T c c`; a zero pivot forces a zero slack column | proved |
| Dep. facet | `DepFacet.of_dependency` | `a_c = Σ_{j∈A} μ_j a_j` (row and column relations of `M`) with independent active rows gives row `c` of the tableau `w_c = ρ_c + Σ μ_j w_j` and column `c` equal to `-μ` on `A`, 0 elsewhere; so `T c c = 0` | proved |
| Dep. facet | `identity`, `slack_side`, `needs_pos` | `w_c = -D + Σ μ_j w_j` for every solution; a basis with slacks zero on `T \ {j}` has `μ_j w_j = D`, so across `F` only `j ∈ P⁻` can work | proved |
| Dep. facet | `no_solution_of_empty`, `qp_infeasible_of_empty` | with `P⁻` empty no parameter with `D > 0` is feasible | proved |
| Dep. facet | `on_facet` | **(c)** on `F`, `T \ {j}` is valid exactly where `λ_j/μ_j ≤ λ_k/μ_k` for all `k ∈ P⁻` | proved |
| Dep. facet | `across`, `unique_across`, `returned_mem_S` | **(a), (b)** at a relative-interior point where `j` is the strict argmin, `T \ {j}` is valid across and is the only valid complementary basis just across; every basis the walk can return lies in `S` | proved (ties excluded) |
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
| Skip 2 | `cr_interiors_disjoint` | two different bases lexico-feasible somewhere have critical regions with disjoint interiors | proved |
| Facets | `relint_of_strict` | a point on the facet plane that clears every other row strictly is a relative-interior point of the facet | proved |
| Start | `chebyshev_ball_feasible` | every point within the radius of a Chebyshev centre satisfies every inequality row | proved |
| B1 | `lp_certificate`, `basis_certificate`, `lp_weak_duality` | primal and dual feasibility (with complementarity) imply optimality | proved |
| B2 | `rank1_inverse`, `rank1_tableau` | `Lex::pivot` keeps `β = A_B⁻¹` and `T = Aβ` | proved |
| B2 | `dual_pivot` | phase 1: the ratio test keeps the multipliers dual feasible and drives the leaving one to 0 | proved |
| B2 | `primal_direction`, `ratio_test`, `lex_ratio_test` | phase 2: the edge direction and the (lexicographic) ratio test keep primal feasibility | proved |
| B3 | `lexOptimal_unique`, `nondeg_basis_unique` | the lexico-optimal basis is unique for each `θ` (no overlaps) | proved |
| B2 | `infeasible_beyond`, `row_j_combination` | no inequality position with `T_jp > 0` in the dual ratio test: no feasible point anywhere beyond the facet (Farkas certificate) | proved |
| B2 | `reach_congr` | the explored set depends only on the neighbour relation, not on which procedure computes it | proved |
| B3 | gap-freeness | every feasible `θ` has a lexico-optimal basis (termination of the lexicographic simplex) | **open** |
| Enum | `EnumBasis.LexOpt` | step 1: lexico-optimal at θ means (D) and lexico-positive nonbasic slack rows; `lexOpt_unique` ties it to `lexOptimal_unique` | definition, proved link |
| Enum | `primalLex_iff_of_interior`, `slack_zero_of_interior` | step 2: in the interior of the closed region, primal lexico-feasibility is exactly (Z) | proved |
| Enum | `fullDim_iff`, `interior_region_eq` | step 3: `R_B` is full-dimensional iff (D), (Z) and the closed region is full-dimensional; then both have the same interior | proved |
| Enum | `enum_interiors_disjoint` | two different bases passing (D) and (Z) have closed regions with disjoint interiors | proved |
| Enum | step 4 (walk reaches every full-dimensional `R_B`) | Jones et al. Thm 22 | **open**; needed only to say the walk and the enumeration agree |

Named open pieces of Case 2:

- `WalkEndsWithPush`: at every relative-interior facet point the walk ends at the terminal push
  (not with `P` empty or with α leaving). This is a Lemke-type existence statement for the
  positive semidefinite pLCP, not proved here.
- The work-tableau link: the work tableau with α nonbasic is, for `α = t`, the tableau of the
  returned complementary basis at `θ_f + t g`. It holds by the construction in `adjacent`
  (mplcp.rs 1547-1563) and `exchange_iff`, but the labelled dictionary (rows not aligned with pairs
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
  the facet and take its active set). Never return silently. Implemented in b5c8f6d (explicit stack,
  cycle check, budget `2n` with the `Across` fallback); see finding 10 for what that guarantees.

**3a, the inner loop (`8n + 50`, removed in b5c8f6d).** Each iteration is a step of the same walk, so
`walk_no_repeat` applies: from a lexico-feasible start the loop cannot revisit a state, and it
ends within the number of reachable states. The census never reached it. The cap can become the
same cycle check (compare with the first state), with the bound as a safety net that reports
rather than returns.

## Correspondence with the Rust code

`mplcp.rs` at `59b2afb` unless noted; `build` is `mplcp_build.rs`.

| Lean theorem | Rust function or step | What it justifies |
|:-------------|:----------------------|:------------------|
| `lcpM`, `lcpq`, `lcpQ`, `slack_primalOf` | `Plcp::new` (52-123) | the pLCP data `M, q, R` from the KKT cache |
| `lcp_unique_optimizer`, `region_law_optimal` | `tableau_law` (2521), `complete_primal_law` | `x(θ) = -H⁻¹(c + Hθ + Aᵀλ(θ))` is the optimizer on the region |
| `law_eq_kkt` | `TABLEAU_LAW` switch (2495), `tableau_law` vs `optimal_control_law_dual` | the tableau law equals the KKT-solve law |
| `tableau_unique`, `pivotSeq_eq_of_isTableau` | `REFACTOR` (1898), `TabSrc::Lazy` and `resolve` (2079-2113), `candidate_law` (2558), `pivot_solve` (2755-2786) | a tableau reached by a chain of pivots, applied when the child is popped, equals the one rebuilt from scratch, for any period |
| `rebuilt_eq_pivoted`, `pivotSeq_perm`, `tableau_unique` | tableau budget `TAB_BUDGET` (2036, 512 MiB, `set_tab_budget` 2072), shared by live parents and owned queued tableaux (`ready_bytes` 2719-2725, `tab_len_bytes` 2051-2053): over budget a toggle child is queued as `TabSrc::None` (2760, 2769) and an owned tableau that does not fit is dropped (2770-2777); both are rebuilt by `tableau_of` (2167-2171) in `resolve` (2104-2110), a failed rebuild counted as `ResolveNone` (2107, census builds only) | the rebuilt tableau equals the pivoted or dropped one, so the explored set is unchanged (finding 18) |
| `rebuilt_eq_pivoted`, `reach_congr` | relative-toggle hand-over: a toggle accepted through `nonzero_rel` returns `Nbr::Z(z_toggle(tab, n, v), None)` (1540), the basis of `Nbr::Toggle(v, kk)` without its pivot or entry hyperplane (2460-2481); `candidate_law` rebuilds the tableau with `tableau_for` (2592-2606), kept as `TabSrc::Ready` with depth 0 while it fits the budget, else rebuilt at pop (2770-2777) | the child's tableau and law equal those of the toggle pivot, so the neighbour and the explored set are unchanged (finding 16) |
| `isTableau_empty`, `tableauFor_isTableau`, `pivotSeq_perm` | `tableau_for` (312-358): pivots in the order of `z`, each checked against `pivot_floor` `1e-11 min(M_ii, 1)` (304-306, check at 327); after a failure one retry with the largest remaining relative pivot first (336-355, check at 347, `TabRescued` 356) | the region tableau rebuilt from an active set, the same in either order |
| `exchange_iff`, `pivVec`, `pivT` | `Tab::defer_pivot` (208), `Tab::flush` (269), `Tab::pivoted` (232), `Tab::tv`/`rv` (163-199) | the pivot formulas |
| `pivot_isTableau`, `toggle` | `Nbr::Toggle` (1542, used at 2479), `z_toggle` (1082) | the neighbour's tableau is one principal pivot away |
| `Case1` (fields), `Case1.facet_eq` | `adjacent` (1454): `zrows` (1467-1486), `pv` (1493), `tve` (1499), `single` and the measure (1504-1518), absolute (`tve > PTOL` and not `demoted`, 1523) or scale-free (`nonzero_rel`, 1522) acceptance (1530), `blocked` (1531) | the exact conditions of the fast path |
| `Case1.across`, `Case1.lex_cover` | fast path return: `Nbr::Toggle` (1542), or the basis only, `Nbr::Z`, for `nonzero_rel` (1540) | the toggled basis is the region across the facet |
| `pivot_nonneg`, `column_le_pivot`, `slack_column_zero_of_pivot_zero` | `pivot_measure` (1418-1430) and its O(1) pivot part `pivot_part` (1434-1438), `MEASURE_ZERO` 1e-9 and `MEASURE_NONZERO` 1e-6 (1446-1447), evaluated in `adjacent` for a single vanishing slack row (1508-1523, the column loop skipped when the pivot part decides) and in `rank1_case2` (1202, pivot part first) | the pivot of a vanishing slack row is `≥ 0`; the slack-column part of the measure squared is at most the pivot part; a zero pivot forces a zero slack column; the short-circuits are exact because the measure is a maximum (finding 18) |
| `entry_facet_unique_neighbour` | entry-facet skip in `expand_candidates` (2374-2397), `Item::entry` (2784) | the skipped search could only return the parent |
| `LexPos`, `LexNonneg` | `lex_nonneg` (385-392), `lex_nonneg_rel` (397-400), `lex_cmp` (371-382) | lexicographic sign, without tolerance |
| `LexPos.eventually_pos` | header (7-9): `ε = (ε0, ε0², …)` | symbolic `ε` is sound |
| `lexRows`, `isLexTableau_of_isTableau` | `rfull` (474-487), `rowmax` (488); `lex_feasible` (3169) | the β block is `-T[:, c]` for nonbasic `w_j`, a unit vector for basic `w_j` |
| `lex_ratio_pivot_on`, `zero_row_beats_positive_row`, `lexmin_zero_row` | `in_z` (490, relative through `Facet::sc`, 1472-1479), Case 1 (513-550): lexmin over `P ∩ Z` on β | the Case-1 pivot keeps the non-α rows lexico-positive |
| `WalkState`, `WalkStep` | `walk_one` (419): `Tab` with `row_var`, `col_var`, entering column `e`; `e = col_of(comp(leaving))` (536, 725); `WalkKey` (767-775) | the walk of `pivot_facet` |
| `first_step_rows`, `first_step_nonAlpha` | first call of `walk_one`, `e` = the α column of the work tableau (1585-1598) | the first pivot establishes the walk invariant |
| `pivVec_num_zero` | Case 1 (zero leaving row) and Case 2 (α's coefficient within `PTOL`) | α stays on the facet |
| `terminal_rows`, `ray_lexFeasible`, `walk_sound` | terminal push `push_leaf(z_after(...))` (496-509), tested before the `P` empty exit (510) | the returned basis is lexico-feasible just across the facet |
| `case2_lexmin_accepted`, `case2_accept_unique`, `gamma_scale`, `gamma_scale_nonneg` | Case 2 (551-752): scale-free Γ from the rows of `P` divided by `a_j` once per iteration (556-565) and their pairwise differences (580-600), `gq_zero` relative (586-587), `lex_nonneg_rel` (592), cut rows staged (567-568, 597-600) and added only to a surviving branch (608-612), DAQP filter then Chebyshev (659-707) | the sub-facets cover the facet and at most one candidate wins at each point |
| `farkas_box`, `facet_coord_le`, `box_sq_le`, `daqp_box_certificate` | `daqp_certainly_empty` (1938-2018): multipliers clamped to `p⁺ = max(p, 0)`, `gap` and `res` recomputed from `p⁺` (1986-1988), check `Σp⁺ > 0` and `gap + res R < -1e-10 Σp⁺` (1989), no certificate unless `R ≥ 0` (1940); `R` = `Plcp::coord_bound` (47-48, 118-121), the Euclidean radius of `theta_bbox` (program.rs 242-286), -1 when unbounded; used for the Case-2 sub-facets (659) and the rank-1 pieces (1372); pdaqp.rs `Daqp::certificate` (142) | a sub-facet declared empty without the LP is empty (finding 17) |
| `neighbour_unique_on_ray` | `push_leaf` (504, 1060) | one neighbour per facet point |
| `case2_needs_dependent_rows` | slow path entered with `tve ≈ 0` (1499) or `demoted` (1523); Case 2 (551) | Case 2 moves multipliers among dependent active rows |
| `walk_repeat_returns` | exact cycle check against the state after the first pivot (538-549 in Case 1, 727-735 for Case-2 children) | any repetition returns there, so the check catches every cycle in exact arithmetic |
| `walk_no_repeat`, `chain_length_bound` | explicit stack `pivot_facet` (795-834), `Frame` (779-791), budget `WALK_BUDGET_PER_ROW`, `WALK_BUDGET_MIN` (2027-2028, used at 1636) | the walk is finite; the budget is a cost limit, not a correctness bound |
| `reach_skip` | leaf pruning: a leaf whose active set is in `Seen` is skipped (615-624) | the caller would drop it anyway: same explored set |
| `DepFacet` (fields) | `rank1_case2` (1188-1410) tests: one vanishing row `c` with `γ_c < 0` (1192), a slack row (1197), zero pivot and zero slack column as `pivot_measure ≤ MEASURE_ZERO` (1202), row `c` zero on the `z` columns (1208-1214) and equal to `μ_i = -T[i,kk]` on each active `w_i` column, relative to `√(M_cc / M_jj)` (1250) | the tableau structure of the dependent-row facet, checked directly (instead of `of_dependency`) |
| `hinj_of_dependency` | scale-free multiplier rule: `ν_j`, the size of `μ_j` in units of `√(M_cc / M_jj)` (1218-1227), is zero at most `MU_ZERO` and nonzero above `MU_NONZERO` (1444-1445); a call with a multiplier between goes to the walk (1253-1255) | `T \ {j}` independent for every `j` with `μ_j ≠ 0` (hypothesis `hinj` of `unique_across`); every member kept has `μ_j > 0` (finding 14) |
| `needs_pos`, `no_solution_of_empty`, `qp_infeasible_of_empty` | `P⁻` = the `j` with `μ_j > 0` and `ν_j > MU_NONZERO` (1257-1258); `P⁻` empty returns no neighbour (1262) | only `j ∈ P⁻` can work across; with `P⁻` empty nothing is feasible across |
| `known_region_skip`, `reach_skip` | members already in `Seen` dropped first (1275) | same explored set as returning them |
| `on_facet`, `pieces_cover`, `argmin_only_on_facet` | pieces `F ∩ {μ_k λ_j - μ_j λ_k ≤ 0}` (1326-1357), `facet_polytope` (1157-1174); tie of two ratios as affine functions returns `None` to the walk (1301-1308, 1344) | each member borders `F` on its piece; the pieces cover `F`; at a strict-argmin point only that member is valid |
| `across`, `unique_across`, `returned_mem_S` | piece nonempty by a witness with margin `WITNESS_MARGIN` (1359-1370), then the DAQP filter (1372) or a Chebyshev radius `> 1e-9` (1384-1404); call site in `adjacent` (1555-1563), `RANK1` (1150) | the member of a full-dimensional piece is the unique neighbour just across it |
| `known_region_unique_neighbour`, `known_region_skip`, `vertices_subset_polyhedron` | facet certificate `certify` (2302-2371), asked in `adjacent` after the fast path (1546-1551); `CERT` mode (2194, default 1 at 3 to 5 parameters); audit counter per certified facet (2423-2425), `certificate_audit` (2209), test `certificate_changes_no_region` (`tests/mplcp_certificate.rs`) | skipping a Case-2 facet contained in a known region; the test compares the regions with the certificate on and off (finding 18) |
| `cr_interiors_disjoint` | stray overlapping regions (pop_005, pop_007, pop_011) | which hypothesis an overlap breaks |
| `basic_solves`, `slack_primalOf`, `lcp_unique_optimizer` | tightness guard: `TIGHT_TOL` (2615), `qp_agrees` (2632), `tight_residual` (2648), used in `build_cand` (2680-2690) | an exact tableau makes its active rows tight under its law (residual 0); the QP optimizer at a point is unique |
| `relint_of_strict` | mplp_lex.rs facet point: foot of the region's Chebyshev centre (628-648), else the facet's Chebyshev LP (649-678) | the point used for the lexicographic neighbour is in the facet's relative interior |
| `chebyshev_ball_feasible` | program.rs `ball_points` (1227), `feasible_space_chebychev_ball` (1075) with the Clarabel fallback (1242); used by `gen_optimal_active_set` and `degenerate_start` | sampled start parameters are feasible (inequality rows) |
| `reach_least` | `pivot_solve` (2697), `pivot_solve_par` (2813), `seen` (2700, 2735, 2779, 2822, 2860, 2885) | the explored set does not depend on the visiting order |
| `reach_skip` | `seen_claim` before `build_cand` (2128, 2735); mplp_lex `crossed` (mplp_lex.rs 517, 529, 560-562, 695) | skipping known neighbours is safe |
| `lp_certificate`, `basis_certificate` | mplp_lex.rs header (13-16), `Lex::solve` (273-386), `Ok(())` at 367 | a basis with no lexico-negative slack or multiplier row is optimal |
| `rank1_inverse`, `rank1_tableau` | mplp_lex.rs `Lex::pivot` (168-190) | the rank-1 update of `binv` and `t` |
| `dual_pivot`, `ratio_test` | mplp_lex.rs phase 1 (319-351), ratio `l / y` with `y > 1e-9` (333-347) | phase 1 keeps dual feasibility |
| `primal_direction`, `lex_ratio_test` | mplp_lex.rs phase 2 (352-383), `aj = -T_jp` (374), `slack_cmp` (235-267) | phase 2 keeps primal lexico-feasibility |
| `lexOptimal_unique` | mplp_lex.rs header (17-19), `slack_sign` (205-232), `lam_row` (389-404) | one lexico-optimal basis per `θ` |
| `infeasible_beyond`, `row_j_combination` | mplp_lex.rs `direct_cross` (1071-1116): ratio test over inequality positions with `T_jp > 1e-9` (1077-1091); none means nothing beyond (1092-1099), and `solve` skips the facet (570, 578-582); `direct_row` (1050-1061) | the skipped facet has no neighbour; the LP path's phase 1 uses the same `y > 1e-9` rule (336) and ends in `Fail::Infeasible` |
| `reach_congr`, `lexOptimal_unique` | the success-rate gate `s_avg`, `q_avg` (524, 542, 616-620), no longer fed by these skips (31a12ae) | the gate only chooses between the direct crossing and the LP path, which return the same neighbour |
| `EnumBasis.dual`, `LexOpt` | mplp_lex.rs `dual_lex_feasible_bases` (787-900), test `lex_sign(&lam_out) <= 0` (869); `enumeration_applies` (774-783) requires H = 0 | (D) is a property of the basis alone |
| `EnumBasis.ZCond`, `primalLex_iff_of_interior` | mplp_lex.rs `candidate_ok` (907-989): zero test (943-958), ε-row sign over `B ∪ {j}` (965-985) | (Z) on the identically zero slacks |
| `fullDim_iff`, `enum_interiors_disjoint` | mplp_lex.rs `enumerate_regions` (993-999), dispatch in `solve` (454-455), `use_enumeration` cost model (759-771) | the enumerated regions are the full-dimensional lexicographic regions, without overlaps |

## Modelling assumptions

All of these appear as explicit hypotheses of the theorems that need them.

- **Exact arithmetic.** `ZTOL`, `PTOL`, `1e-11`, `1e-12` and the point-location tolerance `1e-9` are 0.
  Tests such as `gam[i] < -PTOL` become `γ_i < 0`.
- **Strict convexity.** `H` is positive definite (`H.PosDef`, or `StrictlyConvex`).
- **Nonzero pivots.** `tableau_for` and the toggles use nonzero pivot elements (`NonzeroPivots`,
  `T v v ≠ 0`); the code checks `|piv| ≥ 1e-11 min(M_ii, 1)` in `tableau_for`, with one retry in another
  order (`pivotSeq_perm`), and `|piv| > 1e-9 min(M_vv, 1)` in `candidate_law`.
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
- The region builder (`build_region`, `build` 18-146, and `gen_cr_from_prepared` in `mpqp_utils.rs`; its Chebyshev LP now
  retries cold, then with the rows reversed, then with a relaxed right-hand side, `chebyshev_ball_from_or_cold`, 76c5070,
  numerics only): vertex
  clipping (now up to 5 parameters), the LP redundancy removal above that, `remove_duplicate_rows`, the
  Chebyshev center and the facet hint. The model treats a region as the set `CR`; its row list only matters
  through the facets that get searched.
- Tableau refactoring and drift (`REFACTOR`, `mplp_lex` `REFRESH`), beyond the exact statement
  `tableau_unique`.
- Start-region selection (`initial_region` 2152, `degenerate_start`, `lex_feasible_start` 3311,
  `mplp_lex` initial basis; `drop_lex_infeasible` was removed in 52f3103). Only the feasibility of the sampled points is
  covered (`chebyshev_ball_feasible`). Past bugs of this kind, such as the zero-region
  start from dependent active rows fixed in `668403e`, live at this level.
- The enumeration method (`Method::Enumeration`) and the parallel scheduling of `pivot_solve_par`
  (only its result set, via `reach_least`).
- In `mplp_lex`: the `γ` infinitesimal along the facet normal and the concrete formulas of the lexicographic
  rows (abstract coefficient vectors in `lexOptimal_unique`). The relative-interior point is covered by
  `relint_of_strict` in exact arithmetic; the `1e-6` margin is outside.

## Findings for the implementation

1. **Case-2 test read `Γ` through an absolute tolerance** (fixed in 52f3103, 490-496). `Γ = a_l row_j - a_j row_l` scales with `a_l a_j`. On `pop_001` the twin rows 4 and
   23 have `a ≈ 1.2e-10`, so `Γ`'s β part, about `±1.2e-10`, reads as zero in both directions and both
   twins are accepted. In the model this violates the hypothesis `Γ ≠ 0` of `case2_accept_unique`.
   `gamma_scale` shows the scale-free vector gives the same exact answer, and the test must also be
   relative to the vector's largest entry.
2. **The entry-facet skip needs a lexico-feasible parent** (`hP`), now that it is merged. A
   lexico-infeasible twin parent (possible until finding 1 is fixed, and removed only at the end by
   `drop_lex_infeasible`) has its lexico-feasible twin across the entry facet, which the skip no longer
   searches for. The same holds for the known region of step 2 (`hlex`).
3. **`in_z` used an absolute tolerance, `zrows` a relative one** (fixed in 52f3103 through `Facet::sc`, see
   finding 9 for what remains). In a laptop debug build at `f017013`:
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
6. **The `depth > 6` guard dropped neighbours silently** (replaced in b5c8f6d, see finding 10).
7. **Lazy child pivots and `REFACTOR = 32` (2b0c6c7) match the model.** `TabSrc::resolve` applies the same
   principal pivot to the unchanged shared parent (the in-place pivot only for the last holder), so
   it equals the eager pivot, and `pivotSeq_eq_of_isTableau` makes the period irrelevant in exact arithmetic.
   The pivot element of a `Toggle` is nonzero by the fast-path test (`tve > PTOL`, 658-659), as `Case1`
   assumes. Floating-point drift, measured in `bench/results/mplcp_tableau_handling.md`, is outside the model.
8. **Region construction and start fallback (663a59e) stay outside the model, with one caveat.** Vertex
   clipping up to 5 parameters, the LP redundancy path above, the reuse of prepared rows
   (`gen_cr_from_prepared`) and pairwise `remove_duplicate_rows` all describe the same set in exact
   arithmetic, so the region theorems are unaffected. The facet theorems do assume that every facet of a
   region is in its row list: a row dropped at the tolerance edge in a sliver is a facet whose neighbour
   search does not run (coverage, outside the model). `chebyshev_ball_feasible` proves that sampled start
   points satisfy the inequality rows only; with equality rows, moving θ alone can break them. The
   `ball_points` comment now says so (52f3103).
9. **Case-2 fix and facet certificate (52f3103).**
   - *Phase 1 matches the model.* The test on `row_j/a_j - row_l/a_l` (490) has the exact answer of the test
     on `Γ` (`gamma_scale`, `gamma_scale_nonneg`), and the halfspace branch normalises the same vector, so
     the sub-facets are unchanged. Two tolerance-level departures remain:
     - `lex_nonneg_rel` (353-356) ignores every entry below `ZTOL` times the largest entry. A genuine leading
       entry that small, which needs a dynamic range above `1e9`, is skipped, and a later entry decides
       the sign.
     - `in_z` takes its scale from `Facet::sc`, which `adjacent` fills only for variables basic in the
       region tableau (631-636). Rows that hold another variable after a pivot (the entered complements,
       and α at index `2n`) keep scale 1, so the absolute test remains for them.
   - *The certificate matches `known_region_skip`, with two hypotheses taken on trust.* `certify` (972-1040)
     checks the matching opposite row (the hyperplane and side of `R`), a full-dimensional `F` (`nt - 1`
     independent vertex differences), and `F ⊆ R` on `F`'s vertices (`vertices_subset_polyhedron`). It
     does not check:
     - `hreg` (`R`'s row set lies inside the critical region of `R`'s basis) and `hlex` (`R`'s basis is
       lexico-feasible); both are assumed from construction;
     - that the clip vertices are all of `F`'s vertices.
   - *Stray overlaps.* An overlap of two lexico-feasible regions contradicts `cr_interiors_disjoint` (their
     interiors are disjoint in exact arithmetic). So for at least one of the two regions, `hreg` fails (its
     built row set is larger than its basis's region), `hlex` fails (its lexico-feasibility is misjudged),
     or its tableau is not exact. Used as the known region `R`, such a region lets `F ⊆ R` pass on its rows
     while `F` is not inside its true region, and the skip then drops the real neighbour. The same
     failure, or an incomplete vertex list at 6 parameters, is the candidate cause of the region lost on
     masses3_N4.
10. **Explicit stack, cycle check, budget and `Across` fallback (b5c8f6d).**
    - *Items 1 and 3 match the model.* The cycle check compares every state after a pivot with the state
      after the walk's first pivot. Case-2 children inherit that state, or take their own if the walk's
      first pivot is theirs. `walk_repeat_returns` shows that, in exact arithmetic, every repetition returns
      there, so the check is complete; from a lexico-feasible start it never fires (`walk_no_repeat`),
      matching the runs. States are compared by labels (`row_var`, `col_var`, `e`); a labelled tableau is
      determined by its labels (the work-tableau link). A numerical cycle that does not pass through that
      state is outside the theorem's hypotheses, and the budget is what stops it. Leaf pruning omits only
      leaves whose active set is already in `Seen`, which the caller drops anyway, so the explored set is the
      same (`reach_skip`). The sub-facet hint only seeds the region builder (outside the model).
    - *What the budget and `Across` guarantee.* No silent loss: a stopped walk is counted, the bases it
      returned before stopping are kept and sound (`walk_sound`), and the facet gets one more candidate: the
      QP active set at `θ* = c_F + δ g`, with `c_F` the Chebyshev centre of the whole facet and
      `δ = 1e-6 (1 + max|c_F|)` (871-879, 1280-1296).
    - *What they do not guarantee.*
      - One point is probed. On a facet with several neighbours, the remaining stack frames (the sub-facets
        not yet walked) are discarded, and only the neighbour across `c_F` is recovered.
      - The probe takes a finite step `δ`. A neighbour thinner than `δ` along `g` at `c_F` is jumped over.
      - The QP active set at `θ*` is an optimal active set, not necessarily the lexico-feasible basis there.
        At points with dependent active rows it can be a lexico-infeasible twin or a rank-deficient set
        (rejected by the builder). With `drop_lex_infeasible` gone, a twin would be kept.
      - So completeness of the facet is lost when the walk stops on a facet with several neighbours. The
        exploration as a whole stays complete if every neighbour missed there is reached through another
        facet whose search completes (`reach_skip`). It loses a region exactly when every facet adjacency of
        that region to the explored set is a stopped walk whose probe lands elsewhere.
    - *Measured.* On pop_005, all 99 budget stops are on facets where the probe lies outside the feasible
      set (all 99 fallback QPs fail), and the region count stays at 10,297. The stops are at facets with no
      neighbour, where nothing is lost. A smaller budget (32 or 64) loses 3 regions on pop_005 (10,294),
      which shows the second condition can occur.
    - *Comment wording* (847-852): "at most 2^n pivots, Lean walk_no_repeat". The proved bound is the number
      of reachable label states (`chain_length_bound`, about `n 2^n`). `walk_no_repeat` needs a lexico-feasible
      start; finiteness in general comes from `walk_repeat_returns` plus the cycle check.
11. **mplp_lex enumeration for H = 0 (d3ad15c).** Steps 1 to 3 of the argument are proved
    (`MplpLex/Enumeration.lean`), with the slack and multiplier rows taken as the coefficient vectors of
    the perturbed LP (the same modelling link as `LexOptimalBasis`). What the merged path depends on:
    - *Its own exactness:*
      - each enumerated region is a full-dimensional region of the lexicographic partition, and the
        regions do not overlap (`fullDim_iff`, `enum_interiors_disjoint`); this needs no walk;
      - it misses nothing that exists, since it lists every basis. Coverage of the feasible set
        therefore depends only on B3 gap-freeness (every feasible θ has a lexico-optimal basis, open),
        not on Theorem 22.
    - *Step 4* (the walk reaches every full-dimensional `R_B`, Jones et al. Thm 22, connectivity of the
      graph of lexico-optimal bases) is needed only for "the walk's regions equal the enumerated ones",
      that is, to justify switching between the two paths by cost. It is open here, like A6 for mplcp.
      The enumeration is the path that does not need it.
    - *Departures, all at tolerance level:*
      - (D) is decided by `lex_sign` with the absolute `TOL = 1e-9` on the multiplier rows (766);
      - (Z) treats a slack as identically zero below a relative `1e-12`, and reads the ε-row sign with
        the absolute `TOL` (840-882);
      - full dimensionality is decided by the region builder (`gen_cr_from_active_set_lambda_first`).
      The bounding-box pre-filter in `candidate_ok` (856-858) only drops bases whose closed region misses
      the box of Θ; it is sound when that box contains Θ.
    - *Why (Z) matters:* without it, a basis whose zero slack has a lexico-negative ε-row passes, and its
      closed region coincides with that of its lexico-feasible twin. The hypothesis `hZ` of
      `interior_region_eq` fails, which explains the overlapping extra regions on the two degenerate
      problems.
12. **Correctness milestone (c50f7c0).**
    - *pop_081 fix matches the model.* `terminal_rows` asks only that no row vanishing on the facet
      decreases and that α increases. `P` empty is the sub-case where nothing decreases at all, so the push
      is sound there too. The old order (453-458 before) returned "no neighbour" in that sub-case, where
      the model's walk pushes. The code itself ended walks the way `WalkEndsWithPush` rules out. The
      remaining `P` empty exit (α nonbasic, or not increasing) is a ray, the case that hypothesis excludes.
    - *What the tightness guard and the QP decision guarantee.*
      - An exact tableau has its active rows tight under its law: `basic_solves` gives `w_B = 0` and
        `slack_primalOf` gives `b + Sθ - A x(λ) = w`, so the residual is 0 in exact arithmetic.
      - A residual therefore proves the tableau is not a tableau of the basis. This is the inexact-tableau
        case of `cr_interiors_disjoint`: here a pivot on `1.58e-10` noise in a basis that is singular in
        exact arithmetic, which the model excludes through `NonzeroPivots` and `Case1.piv_pos`.
      - A flagged region is kept only if, at one point, the QP active set equals the region's, or the law's
        `x` equals the QP optimizer (unique by `lcp_unique_optimizer`). That certifies the law at one point.
      - It is consistent with `hreg`: an exact region is never rejected (residual 0, or the QP agrees), and
        the regions whose wrong primal law broke `hreg` (the strays) are removed.
    - *What it cannot catch.*
      - A wrong law with the right primal point. With dependent compatible rows the multipliers are free
        in `ker Aᵀ` (`lcp_multipliers_dependent`): the active rows stay tight and the residual is small, but
        the region's multiplier rows, and hence `hreg`, can still be wrong.
      - Rows lost by the builder (law right, set too big).
      - Inexactness below the `1e-3` relative residual.
      - A flagged law that is right at the test point and wrong elsewhere (one-point test).
      - Lexico-feasibility (`hlex`), which is not tested.
    - *Certificate status (mode 1 on at 3 to 5 parameters).* Checked by `certify`: the opposite row, `F`
      full-dimensional, `F ⊆ R` on `F`'s vertices. Still on trust:
      - `hreg` for `R` (now guarded by the tightness check against wrong primal laws, but blind to the
        multiplier-only case above);
      - `hlex` for `R` (0 lexico-infeasible regions measured on 135 problems, not checked per region);
      - exactness of `R`'s tableau;
      - completeness of `F`'s clip vertices.
      The 48 of 48 identical runs and 0 overlaps and 0 gaps on 135 problems support these but do not prove
      them. The masses3_N4 loss at 6 parameters is outside mode 1's range.
13. **The dependent-row facet, before the code (round 7, `DependentFacet.lean`).** Setting: region `R`
    with active set `A` (independent rows), facet `F` where exactly one slack row `c` vanishes and the pivot
    element on `c`'s complement is zero because `a_c = Σ_{j∈A} μ_j a_j`; `T = A ∪ {c}`,
    `P⁻ = {j ∈ A : μ_j > 0}`, `D = -ρ_c` (zero on `F`'s hyperplane, positive across).
    - `of_dependency` derives the tableau structure the lemma uses from the dependency. Row `c` of `R`'s
      tableau is `w_c = ρ_c + Σ μ_j w_j`, with no `z` column; column `c` is `-μ` on `A` and 0 elsewhere.
    - (a) and (b) are proved at every relative-interior point `θ₀` of `F` where the argmin of `λ_j/μ_j`
      over `P⁻` is unique (`unique_across`, `returned_mem_S`). The hypotheses are a symmetric positive
      semidefinite `M` and independent rows of `T \ {j}` (`hinj`). With `P⁻` empty, nothing across `F` is
      feasible (`no_solution_of_empty`, `qp_infeasible_of_empty`).
    - (c): `on_facet`. `T \ {j}` borders `F` exactly on `F ∩ {λ_j/μ_j ≤ λ_k/μ_k for all k ∈ P⁻}`.
    - The explicit candidate (`candidate_solves`): from `R`'s tableau, set `w_j = D/μ_j` and
      `z_c = (λ_j + T_jj w_j)/μ_j`, with the other nonbasic variables at 0. Its slack side is exactly the
      identity `μ_j w_j = D` (`slack_side`).
    - Ties are excluded. Where several `j` attain the minimum (a lower-dimensional part of `F` when the
      ratios are distinct affine functions on `F`), the statement is not made; the lexicographic rule would
      decide there.
    - Skipping explored members of `S` keeps the explored set by `reach_skip`. The code correspondence
      will be added when the enumeration lands.
14. **Rank-1 Case-2 enumeration (f234e64) against `DependentFacet.lean`.** `rank1_case2` checks the
    `DepFacet` hypotheses directly on the tableau and returns the members of `S` whose pieces are
    full-dimensional. Which test discharges which hypothesis:
    - *Rank-1 class:* exactly one row vanishes on the facet (`zrows.len() == 1`), it is a genuine facet row
      (`pv = zrows`, `γ_c < 0`, so `D > 0` across), and it holds a slack (`c ∉ A`, `c_out`). Since `zrows` is a
      single row, every other tableau row is positive at interior points of `F`, which is `hrelA` and `hrelN`.
    - *Zero pivot:* `|T[c,kk]| ≤ PTOL max(|γ_c|, 1)` is `T c c = 0`, part of `row_c`.
    - *The rest of the structure:* `row_c` and `col_c` are checked entry by entry: column `kk` zero on the
      slack rows, row `c` zero on the `z` columns, and equal to `μ_i = -T[i,kk]` on each `w_i` column. These
      are the conclusions of `of_dependency`, checked instead of derived.
    - *Sign of `μ_j`:* `P⁻ = {-T[i,kk] > PTOL}`, the set `needs_pos` allows. An empty `P⁻` returns no
      neighbour (`no_solution_of_empty`, `qp_infeasible_of_empty`), and no `Across` probe is needed there.
    - *`hinj`:* by `hinj_of_dependency`, `T \ {j}` is independent whenever `μ_j ≠ 0`. The code's guard
      (multipliers between `PTOL` and `1e-7` of the largest go to the walk) keeps every used `μ_j` safely
      nonzero.
    - *Ties:* two ratios that coincide as affine functions on `F` return `None`, and the walk with its
      lexicographic rule decides, as the model requires (ties are excluded there). Ties on a
      lower-dimensional set only touch the closed pieces' boundaries. Every interior point of a piece is a
      strict argmin, where `unique_across` and `argmin_only_on_facet` apply; `pieces_cover` shows the pieces
      cover `F`.
    - *Seen first:* members already in `Seen` are dropped before any piece LP. The walk would return the same
      member, and `seen_claim` would drop it, so the explored set is unchanged (`reach_skip`, `known_region_skip`).
    - *Departures, at tolerance level:*
      - the structure tests use an absolute `1e-8` on tableau entries (848, 854-860), and `1e-8` relative on
        the row-`c` comparison (866);
      - a true `μ_j` in `(0, PTOL]` counts as zero, so its member is missed; such a `j` is the argmin only
        near points where `λ_j` vanishes, at `F`'s boundary;
      - a piece thinner than the witness margin `1e-8` or the Chebyshev radius `1e-9` is dropped.
      - `nt = 1` (`d1 == 0`) is left to the walk, although the model covers it.
    - *Scale-free multiplier rule (eefcba1, lines at 59b2afb).* Each active multiplier is now judged in the
      scale of the dependency, `ν_j = |μ_j| √(M_jj / M_cc)` (1218-1227). `ν_j ≤ MU_ZERO = 1e-9` is zero.
      `ν_j > MU_NONZERO = 1e-6` with `μ_j > 0` puts `j` in `P⁻` (1257-1258). A call with any multiplier in
      between goes to the walk (1253-1255). The symmetry test uses the same scale,
      `|T[c, w_j] - μ_j| ≤ 1e-8 (√(M_cc / M_jj) + |μ_j|)` (1250).
      - *`hinj` holds for every member kept.* A member `T \ {j}` is formed only for `j ∈ P⁻`, that is for
        `μ_j > 0`, so `μ_j ≠ 0` and `hinj_of_dependency` gives `hinj`. In the model the tolerances are 0:
        the band is empty, "zero" means `μ_j = 0`, and the rule is exactly the `P⁻ = {j : μ_j > 0}` of
        `needs_pos`.
      - *The band only moves calls.* A multiplier in the band, or a failed symmetry test, makes
        `rank1_case2` return `None` for the whole call, and the walk runs (`R1Fallback`). No call is partly
        enumerated. In the model the walk returns the same neighbours (`walk_sound`,
        `neighbour_unique_on_ray`), with its completeness conditional on `WalkEndsWithPush`, as before.
      - *`ν` is scale-free.* Scaling `a_j` by `s` scales `μ_j` by `1/s` and `√M_jj` by `s`; scaling `a_c`
        scales `μ_j` and `√M_cc` alike. So the rule no longer depends on the units of the constraints, as
        the old `1e-7 μ_max` guard did.
      - *Departure, restated.* A true `μ_j > 0` with `ν_j ≤ 1e-9` counts as zero, and its member is missed.
        When `P⁻` has other members, `j` is the argmin only where `λ_j / μ_j` is smallest, near `λ_j = 0` at
        `F`'s boundary, so a sliver is lost; the earlier item said only this. When `j` is the only positive
        multiplier, `P⁻` comes out empty and the code reports no neighbour across `F` (1262), so the whole
        region across is lost unless another facet reaches it. The measured split supports the thresholds:
        on masses4_N3 the true multipliers lie at `ν` from 1e-1 to 1e-4 and the rounding of exact zeros at
        1e-8 to 1e-13, with nothing between 1e-5 and 1e-8. The trial with `MU_ZERO = 1e-7` changed regions,
        so the multipliers near 1e-8 are not all rounding; at 1e-9 they go to the walk.
      - *Still absolute:* the test that row `c` is zero on the nonbasic `z` columns (1208-1214, `1e-8`). By
        `tableau_off_symm` it repeats, in exact arithmetic, the column part of the pivot measure, which is
        scale-free. With `M` entries up to 5e2 it can only send calls to the walk.
      - *Comment wording* (1183-1184): "the undecided ones (none seen)" is out of date; the census counts
        394 to 1,832 multipliers in the band per problem.
15. **mplp_lex: nothing beyond the facet (76c5070).** `direct_cross` reports "nothing beyond" when no
    inequality position has `T_jp > 1e-9`, where `T = A β` and `T_j = A_j β` is the row of the entering
    constraint `j`, as in `Lex::pivot`.
    - *Proved* (`infeasible_beyond`): with `T_jp ≤ 0` at every inequality position, no `x` is feasible at
      any parameter where `B`'s basic point violates row `j`. The certificate is the identity
      `A_j = Σ_p T_jp A_{b p}`. `T` does not depend on θ, so this holds on the whole open half-space beyond
      the facet hyperplane: the facet lies on the boundary of the feasible parameter set and has no
      neighbour. Skipping it without an LP is exact.
    - *Sign convention:* it matches the code. The candidates are the positions with `T_jp > 1e-9`
      (1076-1078), equality positions `p < neq` are excluded, and in the model they are the rows of `E`
      with free sign. The code's "none" is therefore `T_jp ≤ 1e-9`, not `≤ 0`.
    - *The LP path agrees:* phase 1 of `Lex::solve` uses the same `y > 1e-9` rule (336) and ends in
      `Fail::Infeasible`.
    - *The success-rate gate (31a12ae):* `s_avg` and `q_avg` only decide whether a region tries the direct
      crossing or the LP path. Both return the same neighbour:
      - an accepted direct crossing is lexico-optimal at `θ0 + γ n`, because the check is sign-only and the
        synthetic slack values have the right signs, so it equals the LP path's basis (`lexOptimal_unique`);
      - a failed attempt falls back to the LP path;
      - both report nothing beyond a boundary facet.
      So the neighbour relation, and with it the explored set, does not depend on the gate (`reach_congr`).
      Leaving the skips out of the averages changes speed only.
    - *Departures:*
      - a true `T_jp` in `(0, 1e-9]` counts as `≤ 0`. In exact arithmetic the program is then feasible beyond
        the facet, though only far from the basic point (the slack of row `b p` must grow like
        `|s_j| / T_jp`). Both paths make the same call, so they still agree with each other;
      - the comment at 579 ("counts as a success of the attempt") was out of date after 31a12ae; fixed in
        7457656.
16. **Scale-free pivot test (7457656), hand-over (fa86aa9) and demotion (04d4ca9).** Lines at `e2d4655`
    unless noted. Proved in `PivotMeasure.lean`. With `M` symmetric positive semidefinite, the block of the
    tableau on the slack rows and the `z` columns outside `A` (the Schur complement of `M_AA`) is symmetric
    positive semidefinite (`tableau_off_psd`, `tableau_off_symm`). For a vanishing slack row `c` this gives
    three facts:
    - `pivot_nonneg`: `T c c ≥ 0`;
    - `column_le_pivot`: `T k c ² ≤ M k k · T c c`, that is, the slack-column part of the measure squared is
      at most the pivot part;
    - `slack_column_zero_of_pivot_zero`: a zero pivot forces a zero slack column.
    - *Case 1 (fast path).* The acceptance through `nonzero_rel` (1090) meets `Case1` as follows:
      - single leaving candidate: `pv.len() == 1` gives `gam_v` and `gam_others`; with `zrows.len() == 1`,
        `not_blocked` is vacuous;
      - nonzero pivot: the measure `> 1e-6` makes `T v v ≠ 0` a robust decision. Through `column_le_pivot`,
        even a measure carried by a slack-column entry certifies a pivot of at least `1e-12` relative;
      - sign: `tve > 0` gives `T v v > 0` (with `γ_v < 0`), which is `piv_pos`, and it is α rising (`tve` is
        α's coefficient after the first pivot). `pivot_nonneg` shows that this sign holds automatically in
        exact arithmetic for a slack row, so the test only guards rounding.
    - *Rank-1 entry.* `measure ≤ 1e-9` (850) stands for `T c c = 0` and a zero slack column, the `row_c` (at
      `c`) and `col_c` (off `A`) hypotheses of `DepFacet`. By `slack_column_zero_of_pivot_zero`, in exact
      arithmetic the pivot alone forces the column, so the combined test is exactly the zero-pivot
      condition. The separate structure checks of round 8 (856-870) still hold row `c` to `μ`.
    - *Departure (a): a true pivot of measure in `(0, 1e-9]` is treated as zero*, since 04d4ca9 on both
      branches. The model predicts that the true neighbour across `F` is the Case-1 toggle, whose region is a
      sliver across `F`. Its new multiplier is `z_c = D / T c c` and member `j ∈ P⁻` drops as `λ_j - μ_j z_c`,
      so the sliver's width along `g` at a facet point is `tve · min_j λ_j / μ_j`. Beyond it lies `T \ {j*}`.
      The rank-1 path returns `T \ {j*}` and skips the sliver.
      - On the relative branch (`tve ≤ PTOL`) the width is at most `1e-10 · λ_j*/μ_j*`.
      - On the demoted branch (`tve > PTOL`) the measure bounds only `T c c ≤ 1e-9 M c c`, so the sliver can
        be wider than the builder's Chebyshev radius `1e-9` when `λ_j*/μ_j*` is large. `FastDemoted` is 0 on
        masses8_N2, masses4_N3, masses5_N3 and masses4_N4, so this has not happened there.
    - *Departure (b): a true dependence whose rounding gives a measure above the branch's threshold is treated
      as Case 1.* The threshold is `1e-6` when `tve ≤ PTOL` and `1e-9` when `tve > PTOL`. The model predicts
      that the toggle `A ∪ {c}` is singular (no exact tableau, `NonzeroPivots` fails) and that the neighbours
      are the members `T \ {j}` (`unique_across`). The code would pivot on noise. The tightness guard
      (`TIGHT_TOL`) then flags the garbage law and the QP decides, so the true members are not produced
      through this facet unless they are reached elsewhere. The measured gap (dependences below `4e-10`,
      others at least `1.7e-5`) makes this unlikely.
    - *The demotion (04d4ca9) closes the mismatch found at 7457656.* There the measure gated only
      `tve ≤ PTOL`, so a dependence whose rounding made `tve > PTOL` (large-scale rows) was a Case-1 toggle
      whatever its measure. Now, with a single vanishing slack row (`single`, 1085), the measure is computed
      on both branches (1086), and `measure ≤ MEASURE_ZERO` never gives Case 1: `demoted` (1091) excludes it
      when `tve > PTOL`, and `nonzero_rel` (1090) needs `> MEASURE_NONZERO` when `tve ≤ PTOL`. This is the
      rule specified here.
      - A demoted call passes every class test of `rank1_case2` (840-852: one vanishing row, `pv = zrows`, a
        slack row, the same measure at the same row and column). After the certificate (1114-1119) it
        reaches the rank-1 enumeration (1122-1126), or the walk when a structure test fails or `nt = 1`. Both
        give the model's answer for `T c c = 0` (`unique_across`, `walk_sound`).
      - Scope: calls with several vanishing rows, or with a multiplier row leaving (`var ≥ n`), keep the
        absolute test. A multiplier row needs no measure: its pivot is a diagonal entry of the inverse of the
        positive definite `M_AA`, nonzero in exact arithmetic (argued, not formalized).
      - The gap `(1e-9, 1e-6]` is Case 1 on the absolute branch and goes to the walk on the relative one. In
        exact arithmetic a measure above 0 means `T c c > 0` (`slack_column_zero_of_pivot_zero`,
        `pivot_nonneg`), so for a true pivot both paths give the toggle (`neighbour_unique_on_ray`).
        `PivotGap` counts the gap on both branches.
    - *The hand-over (fa86aa9) is exact by `rebuilt_eq_pivoted`.* A toggle accepted through `nonzero_rel`
      returns `Nbr::Z(z_toggle(tab, n, v), None)` (1108). That is the basis of `Nbr::Toggle(v, kk)`
      (`z_toggle`, 732, also called for the toggle at 1923) with the same hint, but without a pivot or an
      entry hyperplane (1932-1943). `candidate_law` rebuilds the child's tableau from its active set with
      `tableau_for` (2050-2053), and `pivot_solve` queues that tableau as `TabSrc::Ready` with depth 0 (2218).
      - In the theorem, `B` is the parent's basis, `v` the pair of row `v`, `T v v ≠ 0` follows from
        `tve > 0`, and `l` is the child's active set in `tableau_for` order. The rebuilt tableau, and so the
        law read from it, equals the parent's tableau pivoted at `v`. The neighbour relation is unchanged,
        and by `reach_congr` so is the explored set.
      - Without an entry hyperplane the child also searches its entry facet. It finds the parent again, which
        `seen_claim` drops (`reach_skip`): the entry-facet skip is lost, never misapplied.
      - `NonzeroPivots` stays a hypothesis. In exact arithmetic it holds in every order: `M` restricted to
        the child's basis is positive definite, since `M_AA` is and `T v v > 0` is the Schur complement
        (argued, not formalized). The code checks it with the absolute `|piv| < 1e-11` (308).
      - *Caveat, floating point (fixed in 55e352a, merged at ddf0aa8).* These children are exactly those
        with a small absolute toggle pivot, `T v v ≤ 1e-10 |γ_v|`. If `v` is pivoted last in the rebuild,
        that pivot is `T v v` itself, and in another order the small factor of
        `det M_(A∪{v}) = det M_AA · T v v` moves to another pivot. So a rebuild pivot could fall below the
        absolute `1e-11` while the measure was above `1e-6`. `resolve` then returned `None`, and the region's
        neighbour search was skipped without a counter (2176; `pivot_solve_par` 2272). The census of main
        found this on 13 to 26 percent of the regions of the masses problems of 8 to 16 parameters, and on 6
        of the 3529 regions of pop_001.
      - *The fix keeps exactness (lines at ddf0aa8).* `tableau_for` (312-358) refuses a pivot only below
        `pivot_floor = 1e-11 min(M_ii, 1)` (304-306). A basis that fails in the given order is tried once
        more, taking the largest remaining pivot relative to `M_ii` first (336-355). In the model the floor
        is 0, and any order of the same pairs with nonzero pivots gives the same tableau (`pivotSeq_perm`,
        from `tableauFor_isTableau` and `tableau_unique`). So the retry can change which bases are
        accepted, never the tableau of an accepted one. `candidate_law`'s one-pivot step uses the same
        scale, `1e-9 min(M_vv, 1)` for a slack row (2519-2525). A step refused there costs only the tableau
        law (the builder solves the KKT system instead), and the child still gets its pivoted tableau.
      - *What remains of this caveat.* A refused rebuild still makes `pivot_solve` skip the region's search
        (`continue` at 2681; `?` at 2784 in `pivot_solve_par`). `ResolveNone` (2067) counts it only in
        `lp_census` builds, where `census_bump!` is compiled in, so a release build still skips silently.
        The skip can no longer come from the scale of a row: after the fix `ResolveNone` and `TabRefuseRel`
        (refused pivots above `1e-6 M_ii`) were 0 on all 114 census runs. It needs a basis whose relative
        pivot falls below `1e-11` in both orders, a basis singular to working precision. In exact
        arithmetic no built region has one: its basis has a tableau, so `M_BB` is positive definite and
        every order has positive pivots (argued, not formalized). The caveat is closed in the model, and
        its cause is removed from the code. An always-on counter, like `AUDIT_EXTRA`, that the tests check
        to be 0 would make the skip impossible to miss.
      - *Comment wording* (309-311). The claim "accepts every basis that has a pivot order above the
        floors" is more than the greedy order gives. Take unit-diagonal `M` on three pairs with `M_12 = 0`
        and `M_13 = M_23 = cos t / √2`. Every relative pivot starts at 1, so the retry also begins with
        pair 1, then takes pair 2 (relative pivot 1 against `1 - cos²t / 2`), and ends with `sin²t` on
        pair 3, as the order `1, 2, 3` does. The order `1, 3, 2` ends with `sin²t / (1 - cos²t / 2)`, about
        `2 sin²t`. With the floor in between, both attempts refuse a basis that another order accepts. The
        margin is a factor of about 2, and `TabRescued` was 0 on every census run, so this matters only
        for the comment.
17. **Tableau budget and DAQP emptiness filter (a8d003f; filter fixed in 0215a64, closed at ddf0aa8).**
    - *Tableau budget* (since a40c3ea 512 MiB, shared with owned tableaux: finding 18). Over 256 MiB of
      live parent tableaux, a child is queued as `TabSrc::None` (2196-2207 at a8d003f) and `resolve`
      rebuilds it with `tableau_of` (1565 at a8d003f).
      `rebuilt_eq_pivoted`, via `tableauFor_isTableau`, `pivot_isTableau` and `tableau_unique`, shows that the
      rebuilt tableau equals the pivoted one. So the candidates, and by `reach_congr` the explored set, are
      unchanged in exact arithmetic. The rebuild's pivots are nonzero because the child's basis has
      independent rows (its toggle pivot is nonzero). The 1 to 4 sliver regions lost on 3 masses problems
      with budget 1 were a floating-point departure. After the rebuild fix of finding 16, budget 1 and the
      default give the same counts on masses3_N4 and masses3_N5, and differ by one region on masses4_N4
      (16313 against 16312).
    - *DAQP filter, the exact statement* (`farkas_box`): if `p ≥ 0`, `r = Aᵀp`, `g = bᵀp` and `g + ‖r‖₁ R < 0`,
      no `x` with `‖x‖∞ ≤ R` satisfies `A x ≤ b`.
    - *Conventions in the code, checked against the theorem* (`daqp_certainly_empty`, 1903-1983 at ddf0aa8):
      - the rows are the stored facet-space rows `e θ_f ≤ f`, scaled to unit Euclidean norm, with the
        right-hand side scaled alike; a row of norm `≤ 1e-12` returns "not certified";
      - there is no shift: the certificate is checked against the rows passed to DAQP, so DAQP's internal
        form does not matter.
    - *Negative multipliers: closed in 0215a64.* At a8d003f the code charged `neg = Σ p⁻_i` times `R`, while
      by `farkas_box_signed` the sound charge is `Σ p⁻_i (|b_i| + α R)`. Now it clamps `p` to
      `p⁺ = max(p, 0)` and recomputes `gap = Σ p⁺_i b_i` and `res = ‖Σ p⁺_i a_i‖₁` from `p⁺` (1951-1953). The
      test `Σ p⁺ > 0` and `gap + res R < -1e-10 Σ p⁺` (1954) implies `gap + res R < 0`. That is `hcert` of
      `farkas_box`, with `p⁺ ≥ 0` as `hp`. The margin only asks for more, so nothing relies on it.
    - *The radius: closed in 0215a64.* `R` is `Plcp::coord_bound` (47-48, 118-121), `√(Σ_k max(lo_k², hi_k²))`
      over `theta_bbox`, or -1 when the box is unbounded, unknown or not finite. `daqp_certainly_empty`
      accepts no certificate unless `R ≥ 0` (1905; the test `!(bound >= 0.0)` also rejects NaN).
      `daqp_box_certificate` composes the chain in Lean:
      - a parameter `θ` of the box has `θ · θ ≤ R²` (`box_sq_le`);
      - its facet coordinates satisfy `‖θ_f‖∞ ≤ ‖θ_f‖₂ ≤ ‖θ‖₂ ≤ R` (`facet_coord_le`);
      - `farkas_box` then excludes them.

      What it assumes is that every point of a true sub-facet is a parameter of the box. `theta_bbox`
      (program.rs 242-286) is the box of `{A_t θ ≤ b_t}`, or of the feasible parameters when that set is
      unbounded. Every critical region lies in both sets: its rows include `A_t θ ≤ b_t`, and its law is
      feasible on it. So every Case-2 sub-facet and every rank-1 piece lies in the box too. Rows that
      `facet_polytope` drops (norm below `1e-12`) only enlarge the tested set, which keeps the conclusion.
      When both boxes are unbounded, no certificate is accepted and every test goes to the LP.
    - *Status:* the filter now meets `farkas_box` exactly, and `R` is justified, so this finding is
      closed. The bounding-box LPs and the DAQP solve are floating point, outside the model.
18. **Changes that are exact in the model (a0ea326, 81c9572, a40c3ea, eefcba1; lines at 107aa72 unless noted).**
    - *Case-2 tests on precomputed rows* (`walk_one`, 556-612). The rows of `P` divided by `a_j` are now
      computed once per iteration (`rs`, and `rmax`, their largest entry over the first `1 + d1`
      positions), and each pair takes their differences. These are the same floating-point divisions and
      subtractions as before. So `gsc`, `gq_zero`, the vector passed to `lex_nonneg_rel` and the cut rows
      are bit for bit the old ones; the maximum over the two rows equals the old running maximum. The cut
      rows of a pair are staged, and added to the sub-facet in the old order only when the branch passes
      every lexicographic test. Before, a failed branch's rows were discarded with it. So the comparisons,
      the sub-facets and the walk are unchanged, and `case2_lexmin_accepted`, `case2_accept_unique` and
      `gamma_scale` apply as before.
    - *Pivot-measure short-circuit* (`pivot_part` 1403-1407, `adjacent` 1473-1483, `rank1_case2` 1201).
      The measure is the maximum of the pivot part and the column part, so a pivot part above a threshold
      puts the measure above it. `adjacent` skips the column loop only when the pivot part already fixes
      the decision: above `MEASURE_ZERO` when `tve > PTOL` (no demotion), and above `MEASURE_NONZERO`
      when `tve ≤ PTOL` (`nonzero_rel`). Then `m` can be below the full measure, but it is already above
      the threshold that `demoted` or `nonzero_rel` tests, so both are unchanged. `lp_census` builds
      always compute the full measure, so `PivotGap` stays exact. `rank1_case2` returns `None` when
      `pivot_part > MEASURE_ZERO`, exactly when the full measure would. A NaN pivot part decides nothing
      and takes the full loop, as before.
    - *Shared tableau budget* (503468f and a40c3ea, lines at 107aa72). One budget, `TAB_BUDGET` (2001,
      default 512 MiB), now covers the live parent tableaux and the tableaux owned by queued items
      (`ready_bytes`, 2679-2685; `tab_len_bytes`, 2016-2018). Over it, a toggle child is queued as
      `TabSrc::None` (2723, 2732). An owned tableau (a Case-2 or hand-over child) that does not fit is dropped
      to `TabSrc::None` (2733-2740). Both are rebuilt at pop by `tableau_of` (2132-2136).
      - *Exact in the model.* A rebuilt toggle child equals its pivoted tableau (`rebuilt_eq_pivoted`), in
        either order of `tableau_for` (`pivotSeq_perm`). A dropped owned tableau is rebuilt from the same
        active set, and `tableau_unique` makes it the same tableau. So the candidates, and by `reach_congr`
        the explored set, are unchanged. The budget only decides where a tableau comes from.
      - *Exact in floating point for owned tableaux.* The region stores the candidate's active set as given
        (`active_set.to_vec()`, mplcp_build.rs 132), and `candidate_law` and `tableau_of` map it to the
        same pair list. `tableau_for` is deterministic, so the rebuild at pop gives the dropped tableau bit
        for bit. This matches the bisection: dropping only the owned tableaux leaves pop_001 at 3,529
        regions.
      - *Floating-point departure for toggle children (twin rows).* A pivoted tableau keeps the exact
        zeros that cancel in the pivot formula. The rebuild leaves noise of about `1e-20` relative there,
        and on pop_001's twin rows the tie between the twins flips on the sign of that noise. Under a 1 MB
        stress budget (every child rebuilt), pop_001 grows from 3,529 to 4,710 regions, with overlaps. The
        model has no such entries: both tableaux are the same exact tableau. At the 512 MiB default the 58
        gated problems give the same hashes as with no budget, and pop_002 and pop_033, which do exceed the
        budget, give the same region sets as the unbudgeted run. mplcp-build's snap of entries below `1e-17`
        times the largest on this path is the next item.
      - *A point to check with the snap.* The bisection reports 3,529 regions for three variants that
        explore with the same rebuilt tableau as the failing one: `REFACTOR = 1`, `TabSrc::Ready(tableau_of(...))`
        at queue time, and the rebuilt tableau substituted inside `resolve`. All three rebuild from the
        same active set with the deterministic `tableau_for`. If those runs are right, the rebuilt values
        alone do not explain the 4,710, and something else on the `TabSrc::None` path matters. A bitwise
        comparison of the queue-time and pop-time tableaux of one child would settle it, as would two
        repeated stress runs with equal hashes.
    - *Audit counter of the facet certificate* (eefcba1, lines at 59b2afb). `certificate_audit().0`
      (`AUDIT_CERTIFIED`) now counts certified facets in audit mode (`expand_candidates`, 2423-2425), not the
      candidates found across them. The explored set does not depend on it. In the audit modes (2 and 4)
      `adjacent` still searches a certified facet, and in the skipping modes the counter is not touched.
      - `certificate_changes_no_region` (`tests/mplcp_certificate.rs`) checks, on three problems, that the
        regions with the certificate on equal those with it off (`known_region_skip` with `reach_skip`, in
        floating point), that the parallel method returns the serial order, and that audit mode changes no
        region.
      - Its last assertion, `certified_total > 0`, only guards against a vacuous pass. Counting candidates
        made it fail once the rank-1 enumeration returned no candidate for a certified facet whose members
        are all known (they are dropped first, `reach_skip`), although the certificate had fired. Counting
        certified facets measures what the check means: the certificate's condition held somewhere.
      - `AUDIT_EXTRA` still counts candidates across certified facets that built a new region. In exact
        arithmetic `known_region_unique_neighbour` makes every basis found just across such a facet `R`'s,
        so it stays 0. A new region there touches the facet only in a lower dimension, as its comment says,
        which the model's full-dimensional pieces exclude: a numerical effect.
      - The doc comment of `certificate_audit` (2206-2208) still says "candidates returned across certified
        facets".

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
