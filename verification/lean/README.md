# RpoptProofs: formal verification of `mplcp` and `mplp_lex`

Lean 4 and Mathlib proofs of the mathematics behind two RPOPT solvers:

- `mplcp` (`src/algorithms/mplcp.rs`): parametric LCP pivoting for strictly convex mp-QPs,
  Jones and Morari (2006);
- `mplp_lex` (`src/algorithms/mplp_lex.rs`): lexicographic perturbation for mp-LPs, Jones, Kerrigan
  and Maciejowski (2007).

The proofs are at the model level, in exact real arithmetic. They justify the steps the Rust code
takes, not its floating point execution. Every result listed as proved has no `sorry`, and
`Audit.lean` prints the axioms of all 155 main theorems: each uses at most `propext`,
`Classical.choice` and `Quot.sound` (three use none, two only `propext` and `Quot.sound`). Modelling
assumptions are explicit hypotheses,
never axioms.

Rust line numbers refer to main at commit `6cd2a18`.

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
| `Foundations/Twins.lean` | duplicated constraints: same feasible set, tight together, multipliers split among twins |
| `Foundations/Chebyshev.lean` | points of a Chebyshev ball are feasible (start samplers); a dual certificate bounds the radius |
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
| `Optimizations/BoxChain.lean` | the box LPs of the region builder: the acceptance certificate, and the shared start vertex on a Case-1 facet |
| `Optimizations/SeenClaims.lean` | claimed candidates: the built regions are the explored set when only non-regions are rejected |
| `Optimizations/IndexFirst.lean` | index-first lookup: a hyperplane-unique facet with a built toggle is Case 1 with a single vanishing row and a nonzero pivot |
| `Optimizations/SeenTable.lean` | the flat `Seen` table: incremental Zobrist key, and exactness of a verified lookup for any key |
| `Optimizations/WeaklyActive.lean` | a weakly active constraint: both labels have the same rows, region and law |
| `Optimizations/ReverseCrossing.lean` | reverse crossing of a Case-1 facet: the pivot keeps the zero rows, the second toggle returns the first region, and the skip keeps the explored set (planned in `mplcp-order`) |
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
| Skip 2 | `known_region_unique_neighbour`, `known_region_skip` | if a known lexico-feasible region `R` on the other side contains `F`, every lexico-feasible basis across `F` is `R`'s; skipping `F` keeps the explored set (an optional mode since 9fec1d5) | proved |
| Skip 2 | `vertices_subset_polyhedron` | the vertices of `F` in `R` imply `F ⊆ R` | proved |
| Skip 2 | `cr_interiors_disjoint` | two different bases lexico-feasible somewhere have critical regions with disjoint interiors | proved |
| Skip 3 | `pivVec_vanish_iff`, `pivVec_prop_iff` | a pivot on a row that vanishes on the facet keeps the set of rows that vanish there (as a set, and in the form `ρ_k = γ_k h` that `zrows` tests) | proved |
| Skip 3 | `toggle_toggle`, `Case1.reverse`, `Case1.reverse_neighbour`, `Case1.reverse_neighbour_single` | seen from `R = toggle S v`, the facet is a Case-1 facet on the same pair (normal `-h`) when the other vanishing rows are inert, always for a single vanishing row; its toggle is `S` with `S`'s tableau | proved |
| Box chain | `box_lp_optimal` | a feasible point with `c` a nonnegative combination of its tight rows maximizes `c`: each accepted box bound is exact | proved |
| Box chain | `box_gap_certified`, `abs_sub_le_extent` | with `c = Aᵀ y + r` and any `y`, `r`, `x`, every point of the polytope in the outer box has `c · x' ≤ c · x + g` (the gap of `certify`): the widened box contains the region | proved |
| Box chain | `box_lp_gap` | with `y ≥ 0` and `c = Aᵀ y + r`, the accepted value falls short of the maximum by at most `y · (b - A x) + r · (x' - x)` | proved |
| Dedupe | `Twins.feasible_iff`, `Twins.tight_iff` | rows that are positive multiples of a kept row change neither the feasible set nor which rows are tight | proved |
| Dedupe | `Twins.vecMul_split`, `Twins.kkt_reduce`, `Twins.kkt_split` | full-program multipliers are exactly the nonnegative splits of reduced ones among the twins | proved |
| Box chain | `Case1.shared_point` | a point of the parent's region on a Case-1 facet lies in the child's region, with the same tight rows | proved |
| Index first | `affine_vanish_prop` | an affine function vanishing on a hyperplane is a multiple of its defining function: the rows vanishing on a facet are the rows proportional to it | proved |
| Index first | `pivot_ne_zero_of_toggle`, `toggle_pivot_pos` | if a basis and its toggle both have tableaux, the pivot is nonzero, and positive for a positive semidefinite `M` | proved |
| Index first | `Case1.of_single_row`, `index_first_unique_neighbour` | a facet with a single vanishing row and a built toggle is Case 1, and near it the only lexico-feasible bases are the two regions | proved |
| Seen table | `zobrist_toggle`, `verified_lookup_iff` | the key of `A xor {i}` is the key of `A` XOR the word of `i`; a lookup that compares the stored set is exact for any key | proved |
| Weakly active | `weakly_active_same_rows`, `weakly_active_same_region` | with `λ_i ≡ 0` the labels `A` and `A \ {i}` have the same rows, region and law | proved |
| Skip 3 | `reach_skip_reverse` | skipping the search of `(R, F)` when an explored region crossed `F` into `R` keeps the explored set, given that this region is `R`'s only neighbour across `F` | proved |
| Facets | `relint_of_strict` | a point on the facet plane that clears every other row strictly is a relative-interior point of the facet | proved |
| Start | `chebyshev_ball_feasible` | every point within the radius of a Chebyshev centre satisfies every inequality row | proved |
| Build | `chebyshev_radius_le` | `y ≥ 0`, `Σ y_k a_k = 0`, `Σ y_k ‖a_k‖ = 1` bound the Chebyshev radius by `Σ y_k b_k` (a certified emptiness test) | proved |
| Build | `reach_of_claims` | with candidates claimed before they are built, the built regions are the explored set when the builder rejects only non-regions | proved |
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

`mplcp.rs` at `6cd2a18` unless noted; `build` is `mplcp_build.rs`.

| Lean theorem | Rust function or step | What it justifies |
|:-------------|:----------------------|:------------------|
| `lcpM`, `lcpq`, `lcpQ`, `slack_primalOf` | `Plcp::new` (99-180) | the pLCP data `M, q, R` from the KKT cache |
| `lcp_unique_optimizer`, `region_law_optimal` | `tableau_law` (3553), `complete_primal_law` | `x(θ) = -H⁻¹(c + Hθ + Aᵀλ(θ))` is the optimizer on the region |
| `law_eq_kkt` | `TABLEAU_LAW` switch (3527), `tableau_law` vs `optimal_control_law_dual` | the tableau law equals the KKT-solve law |
| `tableau_unique`, `pivotSeq_eq_of_isTableau` | `REFACTOR` (2549), `TabSrc::Lazy` and `resolve` (2746-2784), `candidate_law` (3590), `pivot_solve` (3884-3915) | a tableau reached by a chain of pivots, applied when the child is popped, equals the one rebuilt from scratch, for any period |
| `rebuilt_eq_pivoted`, `pivotSeq_perm`, `tableau_unique` | tableau budget `TAB_BUDGET` (2702, 512 MiB, `set_tab_budget` 2739), shared by live parents and owned queued tableaux (`ready_bytes` 3807-3813, `tab_len_bytes` 2717-2719): over budget a toggle child is queued as `TabSrc::None` (3889, 3898) and an owned tableau that does not fit is dropped (3899-3906); both are rebuilt by `rebuild_dropped` (2886-2900: `tableau_for` on `z_of`, then the twin snap) from `resolve` (2781); a failed rebuild counts `ResolveNone` (census builds) and the region takes `across_fallback` | the rebuilt tableau equals the pivoted or dropped one, so the explored set is unchanged (finding 18) |
| `rebuilt_eq_pivoted`, `reach_congr` | relative-toggle hand-over: a toggle accepted through `nonzero_rel` returns `Nbr::Z(z_toggle(tab, n, v), None)` (1952), the basis of `Nbr::Toggle(v, kk)` without its pivot or entry hyperplane (3461-3483); `candidate_law` rebuilds the tableau with `tableau_for` (3624-3638), kept as `TabSrc::Ready` with depth 0 while it fits the budget, else rebuilt at pop (3899-3906) | the child's tableau and law equal those of the toggle pivot, so the neighbour and the explored set are unchanged (finding 16) |
| `isTableau_empty`, `tableauFor_isTableau`, `pivotSeq_perm` | `tableau_for` (372-418): pivots in the order of `z`, each checked against `pivot_floor` `1e-11 min(M_ii, 1)` (361-363, check at 387); after a failure one retry with the largest remaining relative pivot first (396-415, check at 407, `TabRescued` 416) | the region tableau rebuilt from an active set, the same in either order |
| `pivotSeq_perm` | twin snap in `rebuild_dropped` (2886-2900): `snap_noise` (2968-3011) factors the basis again in reverse order and zeroes the entries of twin rows and columns (`twin_rows`, 62-96) that both orders put below `SNAP_REL` 1e-18 of the row maximum and on which they are `unrelated` (2946-2948) | in exact arithmetic both orders give the same tableau, so the snap changes nothing; a numerical safeguard (finding 18) |
| `lex_unique` | probe-across fallback for a region whose tableau cannot be factored: `across_fallback` (2922-2942), the lexicographic Lemke basis at the facet point nearest the centre moved `1e-6` across, kept if its tableau exists and is lexico-feasible; from `pivot_solve` (3853-3854) and `pivot_solve_par` (3989-3990), always counted (`RESOLVE_FALLBACKS` 3055, `resolve_fallbacks` 3058); test `no_region_is_skipped_without_trace` | a kept basis is the only lexico-feasible basis at the probe point, the region containing it; not every neighbour across a degenerate facet (finding 16) |
| `exchange_iff`, `pivVec`, `pivT` | `Tab::defer_pivot` (265), `Tab::flush` (326), `Tab::pivoted` (289), `Tab::tv`/`rv` (220-256) | the pivot formulas |
| `pivot_isTableau`, `toggle` | `Nbr::Toggle` (1954, used at 3481), `z_toggle` (1142) | the neighbour's tableau is one principal pivot away |
| `Case1` (fields), `Case1.facet_eq` | `adjacent` (1849): `zrows` (1866-1889), `pv` (1896-1897), `tve` (1903), `single` and the measure (1908-1922), absolute (`tve > PTOL` and not `demoted`, 1927) or scale-free (`nonzero_rel`, 1926) acceptance (1934), `blocked` (1935) | the exact conditions of the fast path |
| `Case1.across`, `Case1.lex_cover` | fast path return: `Nbr::Toggle` (1954), or the basis only, `Nbr::Z`, for `nonzero_rel` (1952) | the toggled basis is the region across the facet |
| `pivot_nonneg`, `column_le_pivot`, `slack_column_zero_of_pivot_zero` | `pivot_measure` (1657-1669) and its O(1) pivot part `pivot_part` (1673-1677), `MEASURE_ZERO` 1e-9 and `MEASURE_NONZERO` 1e-6 (1807-1808), evaluated in `adjacent` for a single vanishing slack row (1912-1927, the column loop skipped when the pivot part decides) and in `rank1_case2` (1338, pivot part first) | the pivot of a vanishing slack row is `≥ 0`; the slack-column part of the measure squared is at most the pivot part; a zero pivot forces a zero slack column; the short-circuits are exact because the measure is a maximum (finding 18) |
| `entry_facet_unique_neighbour` | entry-facet skip in `expand_candidates` (3331-3362), `Item::entry` (3913) | the skipped search could only return the parent |
| `LexPos`, `LexNonneg` | `lex_nonneg` (445-452), `lex_nonneg_rel` (457-460), `lex_cmp` (431-442) | lexicographic sign, without tolerance |
| `LexPos.eventually_pos` | header (7-9): `ε = (ε0, ε0², …)` | symbolic `ε` is sound |
| `lexRows`, `isLexTableau_of_isTableau` | `rfull` (534-547), `rowmax` (548); `lex_feasible` (4322) | the β block is `-T[:, c]` for nonbasic `w_j`, a unit vector for basic `w_j` |
| `lex_ratio_pivot_on`, `zero_row_beats_positive_row`, `lexmin_zero_row` | `in_z` (550, relative through `Facet::sc`, 1873-1880), Case 1 (573-610): lexmin over `P ∩ Z` on β | the Case-1 pivot keeps the non-α rows lexico-positive |
| `WalkState`, `WalkStep` | `walk_one` (479): `Tab` with `row_var`, `col_var`, entering column `e`; `e = col_of(comp(leaving))` (596, 785); `WalkKey` (827-835) | the walk of `pivot_facet` |
| `first_step_rows`, `first_step_nonAlpha` | first call of `walk_one`, `e` = the α column of the work tableau (2016-2029) | the first pivot establishes the walk invariant |
| `pivVec_num_zero` | Case 1 (zero leaving row) and Case 2 (α's coefficient within `PTOL`) | α stays on the facet |
| `terminal_rows`, `ray_lexFeasible`, `walk_sound` | terminal push `push_leaf(z_after(...))` (556-569), tested before the `P` empty exit (570) | the returned basis is lexico-feasible just across the facet |
| `case2_lexmin_accepted`, `case2_accept_unique`, `gamma_scale`, `gamma_scale_nonneg` | Case 2 (611-812): scale-free Γ from the rows of `P` divided by `a_j` once per iteration (616-625) and their pairwise differences (640-660), `gq_zero` relative (646-647), `lex_nonneg_rel` (652), cut rows staged (627-628, 657-660) and added only to a surviving branch (668-672), DAQP filter then Chebyshev (719-767) | the sub-facets cover the facet and at most one candidate wins at each point |
| `farkas_box`, `facet_coord_le`, `box_sq_le`, `daqp_box_certificate` | `daqp_certainly_empty` (2596-2683): multipliers clamped to `p⁺ = max(p, 0)`, `gap` and `res` recomputed from `p⁺` (2651-2653), check `Σp⁺ > 0` and `gap + res R < -1e-10 Σp⁺` (2654), no certificate unless `R ≥ 0` (2598); `R` = `Plcp::coord_bound` (47-48, 165-168), the Euclidean radius of `theta_bbox` (program.rs 276-318), -1 when unbounded; used for the Case-2 sub-facets (719) and the rank-1 pieces (1611); pdaqp.rs `Daqp::certificate` (142) | a sub-facet declared empty without the LP is empty (finding 17) |
| `box_gap_certified`, `box_lp_optimal`, `box_lp_gap` | boxlp.rs `box_chain_in` (436-489): primal simplex from the previous optimum (`optimise`, 288-409), an optimum accepted when the slacks at the vertex are at least `-1e-8 (1 + abs b)` and the certified gap `g` (`certify`, 240-283) is at most `1e-6 (1 + abs bound)`, after one rebuild on a failure (`rebuild_state`, 211-236); the returned box widened by `g` (484-485); a fallback objective goes to the engine, uncertified (474); called by mplcp_build.rs `lp_path` (283, 321) at 6 or more parameters with the propagated outer box | the widened box contains the region for any multipliers, residual and conditioning, given a valid outer box (finding 19) |
| `Case1.shared_point`, `facet_eq` | boxlp.rs `shared_start` (414-429): a parent box point on the entry hyperplane that satisfies every child row; `init_state` (162-207) needs `n` independent tight rows; only for a Case-1 child (`entry`, mplcp_build.rs 314-317) | on the shared facet the parent's points are the child's, with the same tight rows; the code checks it anyway |
| (exact screen, no theorem) | mplcp_build.rs `lp_path` screen (301-353): reject by a box (301), keep when tight at a box point or the projected centre (342-350), else the LP (352) | in exact arithmetic the kept rows are those that touch the polytope, for any valid outer box; grazing rows at degenerate vertices are a tolerance decision (finding 19) |
| `chebyshev_radius_le`, `reach_of_claims` | cold retry in mplcp_build.rs `build_region` (67-81): a warm Chebyshev LP from the hint with radius at most `1e-8` or a failure is decided again by a cold `chebyshev_ball`; `seen_claim` before `build_cand` (mplcp.rs 3864) | a drop needs two LP runs to agree (no certificate); a claimed set is built or rejected as a non-region, up to the `1e-8` threshold (finding 21) |
| `affine_vanish_prop`, `toggle_pivot_pos`, `Case1.of_single_row`, `index_first_unique_neighbour`, `reach_skip` | index-first skip: `idx_settled` (3298-3322) with the builder's `facet_single` flags (utils.rs `unique_hyperplane_flags`, 102-127), whole-region skip `idx_region_settled` (3325-3329, used at 3821) | the skipped facet is Case 1 with `Z = {v}` and its only neighbour is the known toggle; a Θ row has nothing across (finding 22) |
| `zobrist_toggle`, `verified_lookup_iff` | seen_set.rs (`key_of` 89-91, `find` 104-129); keys of `A xor {i}` in `idx_settled` (3308, 3316) with set-verifying closures | the key only chooses where to look; the stored-set comparison makes the lookup exact (finding 22) |
| `weakly_active_same_rows`, `weakly_active_same_region` | algorithms/mod.rs `drop_weakly_active_duplicates` (162-202): drop `A \ {i}` when `A` has `λ_i ≡ 0` and the same law | both labels have the same rows, region and law, so dropping one loses nothing (finding 23) |
| `Twins.tight_iff`, `Twins.vecMul_split`, `Twins.kkt_split` | program.rs `remove_duplicate_rows` (458-486): equal rows after scaling merged, first copy kept, an equality replaces an inequality it duplicates | same feasible set and optimizer; the multiplier of a merged copy folds onto the kept row (finding 23) |
| `Twins.feasible_iff`, `Twins.kkt_reduce`, `Twins.kkt_split`, `lex_unique` | twin dedupe: `solve_with_dedupe` (2399-2423), `duplicate_rows` (2427-2477, positive multiples of `[A, F, b]` to 1e-11), `dedupe_program` (2481-2499, lowest index kept), `unreduce_regions` (2503-2531, kept indices reported, multiplier total on the kept row) | same optimizer and regions as sets; the output is the reduced program's partition, which can differ from the full program's only where a twin group is part of a primal degeneracy (finding 20) |
| `neighbour_unique_on_ray` | `push_leaf` (564, 1120) | one neighbour per facet point |
| `case2_needs_dependent_rows` | slow path entered with `tve ≈ 0` (1903) or `demoted` (1927); Case 2 (611) | Case 2 moves multipliers among dependent active rows |
| `walk_repeat_returns` | exact cycle check against the state after the first pivot (598-609 in Case 1, 787-795 for Case-2 children) | any repetition returns there, so the check catches every cycle in exact arithmetic |
| `walk_no_repeat`, `chain_length_bound` | explicit stack `pivot_facet` (855-894), `Frame` (839-851), budget `WALK_BUDGET_PER_ROW`, `WALK_BUDGET_MIN` (2692-2693, used at 2067) | the walk is finite; the budget is a cost limit, not a correctness bound |
| `reach_skip` | leaf pruning: a leaf whose active set is in `Seen` is skipped (675-684) | the caller would drop it anyway: same explored set |
| `DepFacet` (fields) | `rank1_case2` (1324-1649) tests: one vanishing row `c` with `γ_c < 0` (1328), a slack row (1333), zero pivot and zero slack column as `pivot_measure ≤ MEASURE_ZERO` (1338), row `c` zero on the `z` columns (1344-1350) and equal to `μ_i = -T[i,kk]` on each active `w_i` column, relative to `√(M_cc / M_jj)` (1402) | the tableau structure of the dependent-row facet, checked directly (instead of `of_dependency`) |
| `hinj_of_dependency` | scale-free multiplier rule: `ν_j`, the size of `μ_j` in units of `√(M_cc / M_jj)` (1375-1379), is zero at most `MU_ZERO` and nonzero above `MU_NONZERO` (1805-1806); a call with a multiplier between goes to the walk (1407-1425; recomputed first only with `set_refine_multipliers(true)`) | `T \ {j}` independent for every `j` with `μ_j ≠ 0` (hypothesis `hinj` of `unique_across`); every member kept has `μ_j > 0` (finding 14) |
| `needs_pos`, `no_solution_of_empty`, `qp_infeasible_of_empty` | `P⁻` = the `j` with `μ_j > 0` and `ν_j > MU_NONZERO` (1426-1427); `P⁻` empty returns no neighbour (1454), after the empty-P⁻ safeguard (1429-1450) sends a doubtful zero to the walk | only `j ∈ P⁻` can work across; with `P⁻` empty nothing is feasible across |
| `known_region_skip`, `reach_skip` | members already in `Seen` dropped first (1489) | same explored set as returning them |
| `on_facet`, `pieces_cover`, `argmin_only_on_facet` | pieces `F ∩ {μ_k λ_j - μ_j λ_k ≤ 0}` (1565-1596), `facet_polytope` (1291-1308); tie of two ratios as affine functions returns `None` to the walk (1519-1528, 1583) | each member borders `F` on its piece; the pieces cover `F`; at a strict-argmin point only that member is valid |
| `across`, `unique_across`, `returned_mem_S` | piece nonempty by a witness with margin `WITNESS_MARGIN` (1598-1609), then the DAQP filter (1611) or a Chebyshev radius `> 1e-9` (1623-1643); call site in `adjacent` (1969-1982), `RANK1` (1284) | the member of a full-dimensional piece is the unique neighbour just across it |
| `known_region_unique_neighbour`, `known_region_skip`, `vertices_subset_polyhedron` | facet certificate `certify` (3158-3227), asked in `adjacent` after the fast path (1958-1963); `CERT` mode (3039, default 0 since 9fec1d5: optional modes 1 to 4); audit counter per certified facet (3400-3402), `certificate_audit` (3065), test `certificate_changes_no_region` (`tests/mplcp_certificate.rs`) | skipping a Case-2 facet contained in a known region; the test compares the regions with the certificate on and off (finding 18) |
| `cr_interiors_disjoint` | stray overlapping regions (pop_005, pop_007, pop_011) | which hypothesis an overlap breaks |
| `basic_solves`, `slack_primalOf`, `lcp_unique_optimizer` | tightness guard: `TIGHT_TOL` (3647), `qp_agrees` (3664), `tight_residual` (3680), used in `build_cand` (3760-3770) | an exact tableau makes its active rows tight under its law (residual 0); the QP optimizer at a point is unique |
| `relint_of_strict` | mplp_lex.rs facet point: foot of the region's Chebyshev centre (628-648), else the facet's Chebyshev LP (649-678) | the point used for the lexicographic neighbour is in the facet's relative interior |
| `chebyshev_ball_feasible` | program.rs `ball_points` (1301), `feasible_space_chebychev_ball` (1149) with the Clarabel fallback (`chebyshev_ball_interior_point`, 1316); used by `gen_optimal_active_set` and `degenerate_start` | sampled start parameters are feasible (inequality rows) |
| `reach_least` | `pivot_solve` (3777), `pivot_solve_par` (3954), `seen` (3789, 3864, 3908, 3966, 4012, 4037) | the explored set does not depend on the visiting order |
| `reach_skip` | `seen_claim` before `build_cand` (2835, 3864); mplp_lex `crossed` (mplp_lex.rs 517, 529, 560-562, 695) | skipping known neighbours is safe |
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
- **LP oracle.** Where an argument uses an LP answer that no certificate checks (the screen's per-row LP,
  the Chebyshev LPs, the `theta_bbox` LPs, the box chain's engine fallbacks), it assumes the LP engine returns
  a true optimum or a true infeasibility verdict (finding 24).

## Not covered

- Floating point, rounding, and every tolerance listed above.
- The sub-facet polytopes and the Chebyshev radius `> 1e-9` (pieces without interior are dropped);
  the recursion itself beyond its pointwise walk.
- The region builder (`build_region`, `build` 18-146, and `gen_cr_from_prepared` in `mpqp_utils.rs`; its Chebyshev LP now
  retries cold, then with the rows reversed, then with a relaxed right-hand side, `chebyshev_ball_from_or_cold`, 76c5070,
  numerics only): vertex
  clipping (now up to 5 parameters), the LP redundancy removal above that, `remove_duplicate_rows`, the
  Chebyshev center and the facet hint. The model treats a region as the set `CR`; its row list only matters
  through the facets that get searched. Only the box LPs of the redundancy screen at 6 or more parameters are
  covered: their acceptance certificate (`box_lp_optimal`) and the shared start (`Case1.shared_point`), see
  finding 19.
- Tableau refactoring and drift (`REFACTOR`, `mplp_lex` `REFRESH`), beyond the exact statement
  `tableau_unique`.
- Start-region selection (`initial_region` 2863, `degenerate_start`, `lex_feasible_start` 4464,
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
        multiplier, `P⁻` comes out empty and the code reported no neighbour across `F` (1262), so the whole
        region across was lost unless another facet reached it; since 62f7442 the empty-P⁻ safeguard sends
        that case to the walk (finding 21). The measured split supports the thresholds:
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
      - *What remained of this caveat: closed at 7961206 (lines at 7961206).* A refused rebuild still made
        `pivot_solve` skip the region's search, counted only in `lp_census` builds. Now a region whose
        tableau cannot be factored takes `across_fallback` (2251-2271), in `pivot_solve` (2918-2919) and in
        `pivot_solve_par` (3033-3034). Each use is counted by the always-on `RESOLVE_FALLBACKS` (2379,
        `resolve_fallbacks()` 2382), and the test `no_region_is_skipped_without_trace` asserts it is 0 on
        three bench problems and pop_001. So no region is skipped silently in either path. A refusal needs a
        basis whose relative pivot falls below `1e-11` in both orders tried. In exact arithmetic no built
        region has one, since its `M_BB` is positive definite (argued, not formalized).
      - *What the probe guarantees in the model.* For each facet row `k` of the region, the probe takes the
        point `on_k` of the facet's hyperplane nearest the region's centre, moves it by
        `δ = 1e-6 (1 + max|on_k|)` along the outward normal, and runs the lexicographic Lemke method there.
        It keeps the basis only if its tableau exists and is lexico-feasible.
        - A basis it keeps is lexico-feasible at the probe point, so by `lex_unique` it is the only
          lexico-feasible complementary basis there: the region of the partition that contains that point.
          It is a genuine region, and the search can only gain from it.
        - It is the neighbour across facet `k` when `on_k` lies in the relative interior of the facet and that
          neighbour reaches at least `δ` beyond the facet at `on_k`.
        - It need not be every neighbour. The projection of the centre onto the hyperplane can fall outside
          the facet, and then another region is found. Across a degenerate facet with several neighbours
          (Case-2 sub-facets or rank-1 pieces), only the one whose piece contains `on_k` is found. A
          neighbour thinner than `δ` there is jumped over. That Lemke terminates on a feasible point is the
          classical result for a positive semidefinite LCP, not formalized here.
        - So, as with the `Across` probe of finding 10, the exploration stays complete only if every
          neighbour missed here is reached through another facet whose search completes (`reach_skip`). The
          always-on counter makes such a case visible.
      - *Comment wording* (309-311). The claim "accepts every basis that has a pivot order above the
        floors" is more than the greedy order gives. Take unit-diagonal `M` on three pairs with `M_12 = 0`
        and `M_13 = M_23 = cos t / √2`. Every relative pivot starts at 1, so the retry also begins with
        pair 1, then takes pair 2 (relative pivot 1 against `1 - cos²t / 2`), and ends with `sin²t` on
        pair 3, as the order `1, 2, 3` does. The order `1, 3, 2` ends with `sin²t / (1 - cos²t / 2)`, about
        `2 sin²t`. With the floor in between, both attempts refuse a basis that another order accepts. The
        margin is a factor of about 2, and `TabRescued` was 0 on every census run, so this matters only
        for the comment.
        The comment was corrected in 169cff5 (350-357 at 7961206). It now says that a basis is accepted
        when the given or the greedy order stays above the floors, and that a refusal means "not found
        with these two orders", not "singular". That is exactly what the code does. One pointer is loose:
        the fallback is at the two call sites of `TabSrc::resolve` (`across_fallback`), not in it.
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
18. **Changes that are exact in the model (a0ea326, 81c9572, a40c3ea, eefcba1, 169cff5; lines at 107aa72 unless
    noted).**
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
      - *Owned tableaux.* The region stores the candidate's active set as given (`active_set.to_vec()`,
        mplcp_build.rs 132), and `candidate_law` and `z_of` map it to the same pair list. `tableau_for` is
        deterministic, so before 7961206 the rebuild at pop gave the dropped tableau bit for bit. Since
        7961206 the rebuild at pop also passes through the twin snap (below), which the kept owned tableau
        does not. On a problem with twin rows the two can then differ in noise-level entries; otherwise they
        are still identical.
      - *Floating-point departure on twin rows, and its cause.* A pivot chain keeps the structural zeros of
        the problem exactly: twin or duplicate constraints give entries that cancel exactly in the pivot
        formula, and 0 stays 0. A fresh factorization leaves rounding noise of `1e-20` to `1e-23` of the row
        maximum in the same places. On pop_001 the tie between the twin rows 4 and 23 is decided by the sign
        of such an entry, so the noise flips the walk into the other twin basis. Under a 1 MB stress budget
        (every queued child rebuilt), pop_001 grew from 3,529 to 4,710 regions, with overlaps. Whether noise
        flips a tie is chaotic: rebuilding every lazy child also gives noisy tableaux but happened to give
        3,529, and two 1 MB runs give bit-identical results, so the failure is deterministic. The model has
        no such entries, since both tableaux are the same exact tableau.
      - *The point I raised is resolved.* The earlier bisection variant that rebuilt at queue time was an
        artefact of debug code (`ready_bytes` wrapped, so almost nothing was dropped). Queue-time and pop-time
        rebuilds are the same function of the same input and are bit-identical, as argued above.
      - *The twin snap (169cff5, lines at 7961206): a numerical safeguard, no exactness claim.* Only the
        `TabSrc::None` rebuilds take it (`rebuild_dropped`, 2215-2229). `snap_noise` (2297-2340) factors the
        same basis again with the pivots in reverse order and aligns the two tableaux by variable. It zeroes
        an entry only when all of the following hold:
        - it lies in a row or column of a twin constraint (`twin_rows`, 58-92: rows of `[M | q | R]` that are
          a positive multiple of another row);
        - both values are at most `SNAP_REL = 1e-18` of the row maximum;
        - the two orders give `unrelated` values (2275-2277: opposite signs, or a factor of 10 apart).

        In the model the snap does nothing: by `pivotSeq_perm` both orders give the same tableau, and
        `unrelated(a, a)` is false for `a ≠ 0`. In floating point it removes entries on which two
        factorizations agree on no digit, and only in the rows and columns of twin constraints, where the
        structural zeros lie. It does not prove that an entry is zero.
        - What it fixes: pop_001 at the stress budget is back to 3,529 (57 of 58 problems equal to budget 0;
          masses4_N4 keeps its one-region sliver difference, where the snap does not act).
        - What it leaves: the tableaux rebuilt by `candidate_law` (`REFACTOR`, Case-2 and hand-over children)
          carry the same noise and are not snapped.
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
19. **Chained box LPs of the region builder (efbd6e9; lines at 685c144 in `boxlp.rs`, and in `mplcp_build.rs`
    for `lp_path`).** At 6 or more parameters `lp_path` (mplcp_build.rs 213) screens the region's rows with
    the 2n bounding-box LPs, which `box_chain` (boxlp.rs 316-355) computes as one primal simplex chain.
    - *Optimality is certified in the model.* An optimum is accepted (boxlp.rs 184-220) when three tests
      pass:
      - no basis multiplier is below `-1e-10`;
      - `c - A_Tᵀ y`, recomputed from the rows, is at most `1e-8` in every coordinate;
      - every row is feasible within `1e-8 (1 + |f|)` at the vertex `x = W b_T`.

      With the tolerances at 0 this is `box_lp_optimal` (the certificate of `basis_certificate`):
      `y ≥ 0`, `c = A_Tᵀ y`, `A_T x = b_T` and `A x ≤ b` make `x` a maximizer, so each accepted bound is
      exact. Three floating-point remarks:
      - Only every 4th LP and the last one recompute all slacks at the vertex (196-203). The others check
        the slacks carried through the rank-one updates. The module comment ("every optimum is verified
        (all rows feasible within 1e-8 at the recomputed vertex ...)") claims more than that.
      - The tightness of the basis rows at `x` is not checked. It follows from `A_T W = I`, which the
        updates keep up to rounding and the refresh every 50 pivots restores (154-165).
      - The tolerances bound the optimality gap by about `1e-8 ‖x' - x‖₁`, plus `1e-10` times the slacks of
        the basis rows. An understated bound is the unsafe direction, since the screen could then reject
        a row that the true box keeps. The screen's margin `1e-7 (1 + |f|)` covers this unless the box is
        large.
    - *The shared start is checked, and the model says why it works.* `shared_start` (boxlp.rs 297-312)
      takes a parent box point on the entry hyperplane that satisfies every row of the child, and
      `init_state` (104-149) needs `n` independent tight rows there. Both are checked, so the start does
      not rest on an argument. The entry facet is set only for a toggle, a Case-1 child. There,
      `Case1.shared_point` shows that a point of the parent's region on the facet lies in the child's
      region and has the same tight rows, because both tableaux agree on the facet
      (`pivot_agree_on_facet`, the pointwise form of the zero-row lemma). A vertex of the parent's region
      on the facet is a vertex of the shared facet (`facet_eq`), and so of the child's region. A parent
      box point need not lie in the parent's region itself: the box LPs run over the rows left after the
      outer-box screen, a polytope that can be larger than the region. So the explicit check is needed.
    - *Any valid outer box gives the same kept rows, in exact arithmetic.* A row is kept
      (mplcp_build.rs 256-279) exactly when its hyperplane meets the polytope:
      - it is rejected when its maximum over a box containing the polytope is below its right-hand side
        (231, used with the outer box and again with the chain's box), which implies it never touches;
      - it is kept when it is tight at one of the 2n box points, or at the projection of the centre when
        that point satisfies every other row (266-276), each of which implies it touches;
      - otherwise the LP with the row as an equality decides (278).

      Each test is sufficient for its side and the LP decides the rest. So the kept rows are the rows
      that touch the polytope, whatever valid outer box is used and whichever optimal vertices the LPs
      return. The chain's box is the exact box of the screened polytope, which is a valid outer box of the
      region.
    - *Departure: grazing rows at degenerate vertices.* "Touches" includes rows that meet the polytope
      only in a lower-dimensional face. At a degenerate vertex such a contact is decided by tolerances:
      the vertex test accepts within `1e-9 (1 + |f|)`, and the LP applies its own tolerance to a feasible
      set that is a single point. The chain and the engine stop at different optimal vertices of the same
      LP, so they catch different grazing rows. The chain keeps up to 0.06% more rows (masses5_N2: 9 of
      15,888) and never fewer on the 64 problems, with identical active sets, region counts and coverage
      everywhere. The extra rows describe the same region. Each costs one more facet search across a
      contact that the facet theorems do not cover (it has no relative interior); whatever that search
      returns is built and checked like any other candidate.
    - *Closed at b98b621 (df4176c; boxlp.rs lines at b98b621).* Every accepted optimum now runs four
      checks:
      - all `m` slacks are recomputed at `x = W b_T` (204-210);
      - the `n` basis rows are tight within `1e-8 (1 + |b|)` (211-216);
      - no multiplier is below `-1e-10`;
      - the residual of `c = Σ y_i a_{T_i}` is within `1e-8` (217-225).

      These are the hypotheses of `box_lp_optimal` up to tolerances, so the first two remarks above are
      resolved, and the module comment (11-24) now describes what the code checks. `box_lp_gap` proves the
      bound the comment states: with `y ≥ 0` and `c = Aᵀ y + r`, a feasible `x'` exceeds the accepted value by
      at most `y · (b - A x) + r · (x' - x)`. For `r = 0` this is
      `Σ_i y_i (b_{T_i} - a_{T_i} · x) ≤ 1e-8 Σ_i y_i (1 + |b_{T_i}|)`.
      - The comment leaves out the residual term `r · (x' - x)`, at most `1e-8 ‖x' - x‖₁`. The negative
        multipliers allowed down to `-1e-10` fold into the same term. For a moderate box it is of the size
        of the term the comment states.
      - Its caveat on scale is right: large multipliers would need a larger screening margin.
      - The grazing-row departure above stays as recorded. It is a tolerance decision, not an open question.
    - *Certified gap (b57d52d; boxlp.rs lines at 6cd2a18).* Each optimum the chain solves now carries a
      gap `g` (`certify`, 240-283):

      `g = Σ_{y_i ≥ 0} y_i |s_{T_i}| + Σ_{y_i < 0} |y_i| Σ_q |a_{T_i q}| D_q + Σ_q |r_q| D_q`,

      where `D_q = max(|x_q - lo_q|, |hi_q - x_q|)` over the caller's outer box. The optimum is accepted when
      the slacks at the vertex are at least `-1e-8 (1 + |b|)` and `g ≤ 1e-6 (1 + |bound|)`. A failed check
      rebuilds the state once (`rebuild_state`, 211-236). The returned box is widened by `g` (484-485), and
      without a finite outer box the chain is not used.
      - `box_gap_certified` proves the bound: with `c = Aᵀ y + r` for any multipliers `y` and residual `r`,
        and any point `x`, every `x'` of the polytope inside the outer box has `c · x' ≤ c · x + g`.
      - Neither feasibility of `x`, nor the signs of `y`, nor the size of `r`, nor the conditioning of the
        basis enters. With `c = ±e_k` this gives `x'_k ≤ hi_k + g` and `x'_k ≥ lo_k - g`, so the widened box
        contains the region whenever the outer box does. The acceptance cap only limits how much screening
        power a large `g` costs.
      - Two inputs are not certified by the chain:
        - The outer box (mplcp_build.rs 292-296) is `theta_bbox`, computed by engine LPs, intersected with
          an interval propagation of the rows. The propagation is exact; the `theta_bbox` part rests on the
          LP engine.
        - An objective that falls back to the engine (boxlp.rs 474) keeps the engine's optimum with `g = 0`.
          A wrong engine optimum understates that bound, which is the unsafe direction for the screen.
          `init_state` already forms the state at the engine's optimum, so running `certify` there too
          would close this whenever the state can be formed.
20. **Duplicated constraints removed inside mplcp (49c13cb; lines at b98b621).** `solve_with_dedupe` (mplcp.rs
    1932-1952) proceeds in three steps:
    - `duplicate_rows` (1956-2006) groups the inequality rows of `[A | F | b]` that are positive multiples of
      each other, to a relative `1e-11`;
    - `dedupe_program` (2010-2028) keeps the lowest-index row of each group, and the reduced program is
      solved;
    - `unreduce_regions` (2032-2060) maps the regions back.

    Proved in `Foundations/Twins.lean`, at every parameter `θ` (the right-hand side `b + F θ` scales with the
    row):
    - *Same feasible set and optimizer* (`Twins.feasible_iff`). The full rows hold exactly when the kept rows
      hold, so both programs have the same feasible parameters and, by strict convexity, the same optimizer
      `x*(θ)`. A twin is tight exactly when its kept row is (`Twins.tight_iff`).
    - *Multipliers split among twins* (`Twins.kkt_reduce`, `Twins.kkt_split`).
      - A KKT multiplier of the full program gives one of the reduced program by putting the weighted total
        of each group on its kept row.
      - Every nonnegative split of a reduced multiplier among the twins of its group is a KKT multiplier of
        the full program: the same combination of rows (`Twins.vecMul_split`), and complementarity.

      So at every `θ`, an active set `A` is optimal for the reduced program exactly when `A ∪ S` is optimal
      for the full one, for any set `S` of twins of rows in `A`. As a set of parameters, a critical region of
      the reduced program is the region of each such `A ∪ S`. The code reports `A`, with the total on the
      kept row.
    - *No region lost, no overlap.* The reported regions are the reduced program's partition. They do not
      overlap, by `lex_unique` for the reduced pLCP, and they cover the same feasible set (`feasible_iff`).
      The relabelling is the identity on kept rows, so distinct regions keep distinct labels. Each region's
      law is the full program's optimizer, and its active set is an optimal active set of the full program.
      The region's rows are the reduced program's. They describe the same set, because a twin's primal row
      is a positive multiple of its kept row's, and an active twin's slack row vanishes identically.
    - *Lexicographic tie-breaking: the dedupe can change the partition, but only through primal degeneracy.*
      The perturbation relaxes row `i` (counted from 0) by `ε^(i+1)`.
      - In the full program, a twin group with members `k < ... < j` acts as one constraint relaxed by
        `min_i ε^(i+1) / s_i`, which for small `ε` is the term of its highest index `j`. So the full
        program's lexico-feasible bases contain the group's highest-index member, and a basis with the
        lowest-index member is its lexico-infeasible twin. On pop_001 the twins are rows 4 and 23: the full
        solve returns `[3, 5, 11, 14, 18, 20, 23]`, and `[3, 4, 5, 11, 14, 18, 20]` is the lexico-infeasible
        one, as `mplcp_case2_scale.rs` asserts.
      - The reduced program places the group at the position of its lowest index `k`. So the two
        perturbation orders differ only in where each group sits relative to the rows with indices between
        `k` and `j`.
      - Where no lexicographic decision involves a group together with such a row, both orders give the same
        partition, up to the label of the group (`k` against `j`). This covers the pure twin case of pop_001.
      - Where a group takes part in a primal degeneracy with such a row, the two orders can split the same set
        of optimal parameters differently. Example: rows 0: `x₁ + x₂ ≤ θ₁ + θ₂`, 1: `x₁ ≤ θ₁`, 2: `x₂ ≤ θ₂`,
        3: twice row 0, and an objective whose unconstrained minimizer lies beyond the corner `(θ₁, θ₂)` in
        both coordinates. The relaxations are `ε`, `ε²`, `ε³` and `ε⁴/2`.
        - In the full program the group {0, 3} is relaxed by `ε⁴/2`, which is less than `ε² + ε³`. So it cuts
          the perturbed corner, and the corner set splits into the bases {1, 3} and {2, 3} along the
          hyperplane where the two pulls are equal.
        - In the reduced program row 0 is relaxed by `ε`, which is more than `ε² + ε³`. So it stays slack,
          and the corner set is one region {1, 2}.
        - Both partitions are valid (same optimizer, no overlap, full cover). They differ in how many regions
          describe the corner.
      - The gate (sets identical on 144 of 145 problems) shows this is rare on the benchmark. The exception,
        pop_076, is a hole of the full solve that the reduced solve fills, not a partition difference.
    - *A diagnostic will misreport.* `count_lex_infeasible(program, regions)` evaluates each reported basis in
      the pLCP of `program`, and `lexcheck` calls it with the full program. A reported basis that holds a kept
      row with a tight higher-index twin is exactly the full program's lexico-infeasible twin: the twin's slack
      row vanishes identically, and its β row starts with `-s` at the kept row's position. So after the
      dedupe, `lexcheck` should count such regions as lexico-infeasible on twin problems. On pop_001 that is
      at least the region `[3, 4, 5, 11, 14, 18, 20]` that the new test asserts. These regions are
      lexico-feasible in the reduced program that was solved. Evaluate against the reduced program, or map
      each kept row to its group's highest index, before reading that count. Fixed in 825e95c:
      `count_lex_infeasible` now tests the reduced program when the dedupe is on, and
      `dedupe_lex_count_is_zero_on_twin_problem` covers pop_001.
21. **Cold retry for thin regions, and the empty-P⁻ safeguard (62f7442; lines at 62f7442 in `mplcp_build.rs`
    and `mplcp.rs`).**
    - *What the warm start proves in the model.* The Chebyshev radius is the optimal value of an LP, so it
      does not depend on where the LP starts. What proves "radius at most `δ`" is a dual vector:
      `chebyshev_radius_le` shows that `y ≥ 0`, `Σ y_k a_k = 0` and `Σ y_k ‖a_k‖ = 1` give `r ≤ Σ y_k b_k`
      for every centre and radius that satisfy the rows. An exact simplex started at the hint (a boundary
      point, so a degenerate vertex with `r = 0`) can make degenerate pivots there, but it stops only at
      a basis whose reduced costs certify optimality. So stopping at radius 0 on a region of radius
      `2.2e-6` (masses5_N3 `[2, 15, 49]`), or failing, is the engine's numerical decision, not a property
      of the problem. Before the fix, such an uncertified answer was taken as a proof of emptiness. That
      is the defect.
    - *The fix* (mplcp_build.rs 66-80). When the warm LP gives radius `≤ 1e-8` or fails, a cold
      `chebyshev_ball` decides with the same threshold. A drop now needs two independent LP runs to agree;
      it still has no certificate. A certified drop would check a dual vector as in
      `chebyshev_radius_le`, with `Σ y_k b_k ≤ 1e-8`, and an infeasible answer would check a Farkas vector
      as in `farkas_box`. Suggested, not required: the results show no lost regions after the fix.
    - *Claims (Finding 16's frame).* A candidate's active set is claimed in `Seen` (`seen_claim`) before
      its region is built, and a claimed set is never tried again. `reach_of_claims` states the invariant
      this needs: if every neighbour of a built region is built or rejected, and the builder rejects only
      sets that are not neighbours of built regions (not regions), then the built regions are exactly the
      explored set. A genuine region that the builder rejects breaks the hypothesis, and no other facet can
      recover it, which is how the 355 thin regions were lost.
      - After the fix, a claimed set is built, or it is rejected by one of the following:
        - the law computation, which fails on dependent rows;
        - a zero row with a negative right-hand side;
        - the Chebyshev decision above;
        - the redundancy step;
        - the tightness guard in `build_cand`.
      - In exact arithmetic each of these rejects only sets that are not full-dimensional regions, except
        the `1e-8` radius threshold, which drops slivers by design. So the invariant holds up to that
        threshold and to the numerics of the remaining LPs.
      - "Claimed implies built or proven empty" is therefore too strong: "decided empty by the cold LP" is
        what the code guarantees.
    - *The empty-P⁻ safeguard* (mplcp.rs 1354-1375). When no multiplier is clearly positive but a positive
      tableau multiplier with `ν` in `(1e-12, 1e-9]` was classified as zero, `refined_multipliers`
      recomputes the dependency from the original rows. If every such multiplier comes out at most `1e-12`,
      the call keeps "no neighbour"; otherwise it goes to the walk.
      - In the model, a truly empty `P⁻` means nothing is feasible across `F` (`no_solution_of_empty`), so
        "no neighbour" is exact there and no walk is needed.
      - The safeguard covers the departure of finding 14: a tiny true `μ_j > 0`, classified as zero, that
        is the only positive multiplier. Then `T \ {j}` is the unique neighbour across `F` (`unique_across`
        with `P⁻ = {j}`). The walk does not classify multipliers, and it returns that neighbour (`walk_sound`,
        complete up to `WalkEndsWithPush`). So the walk is the right fallback, and a skip would have lost
        the region.
      - The walk is not needed for lack of a "swap" neighbour: with `P⁻` truly empty there is no neighbour
        at all.
      - What remains: a positive tableau multiplier at or below `ν = 1e-12` is ignored (`ZERO_GUARD_MIN`;
        such a member could not pass `tableau_for`'s relative floor anyway), and the recomputation's
        `1e-12` cut. With equality rows, `refined_multipliers` returns `None`, and the call goes to the
        walk, which is conservative.
    - *Comment wording* (mplcp.rs 1244-1245). The doc comment of `rank1_case2` says that a gap-band
      multiplier "is recomputed from the original data first". That happens only with
      `set_refine_multipliers(true)`; the default (`REFINE_ON` false) sends gap-band calls to the walk as
      before. The empty-P⁻ safeguard does recompute by default.
22. **Index-first lookup of Case-1 neighbours, and the flat `Seen` table (d832575 and seen-precheck, merged at
    1d25ddd; lines at 1cced96).** `expand_candidates` skips facet `k` of a popped region `R(A)` without a
    tableau when `idx_settled` (mplcp.rs 3281-3305) finds that both of these hold:
    - (H1) the builder flagged the row's hyperplane as shared with no other row of the unreduced system,
      in either orientation and within `1e-6`, and found no numerically zero row
      (`utils::unique_hyperplane_flags`, utils.rs 102-127; `facet_single`);
    - (H2) the key of `A xor {i}` hits a built region.

    A Θ row with H1 is skipped too. `pivot_solve` does not resolve the tableau of a region whose facets are
    all settled (`idx_region_settled`, 3308-3312, used at 3793).
    - *Proved: the facet is a Case-1 facet with `Z = {v}`, and its only neighbour is `A xor {v}`.*
      - `affine_vanish_prop`: an affine function that vanishes on the hyperplane of row `i` is a multiple of
        row `i`'s function. So the tableau rows that vanish identically on the facet are exactly those
        proportional to its row. H1 excludes every other row and every identically zero row, so `Z = {v}`,
        where `v` is the pair of row `i`.
      - The facet row is nonnegative on the full-dimensional `R` and not identically zero, so `γ_v < 0`.
      - `pivot_ne_zero_of_toggle`: if `toggle A v` also has a tableau, which holds by H2 since it is a
        built region, then `T v v ≠ 0`. Otherwise the basic variable of pair `v` would not depend on the
        variable the toggle makes basic, and could not be free in the toggled tableau.
      - `toggle_pivot_pos`: with `M` positive semidefinite the pivot is positive. For a slack row this is
        `pivot_nonneg`; for a multiplier row, the same holds for the toggled basis, whose pivot is the
        inverse.
      - `Case1.of_single_row` assembles the Case-1 hypotheses. `index_first_unique_neighbour` (through
        `entry_facet_unique_neighbour`) then says that near every relative-interior point of the facet,
        every lexico-feasible basis is `A` or `A xor {v}`. So the skipped search could only have returned the
        known region (for a lexico-feasible `R`, as every built region is), and `reach_skip` keeps the
        explored set. `Case1.reverse_neighbour_single` gives the
        same statement from the other side. The β-row hypothesis of the entry-facet theorem holds
        automatically here: row `v`'s β part has the nonzero entry `-T v v` for a multiplier row, or its unit
        entry for a slack row.
    - *The Θ rows.* Across a facet on a Θ row the parameter leaves Θ, and every region lies in Θ, so
      nothing is across it whether or not H1 holds. H1 is extra caution there.
    - *A region with every facet settled.* Its expansion would return no new candidate, so not resolving
      its tableau loses nothing (`reach_skip`). The bypassed `across_fallback` runs only when a tableau
      cannot be factored. It is a numerical safeguard and not part of the neighbour relation. The claim that
      its probes "can only land in known regions" is more than the model gives, since a probe can land in a
      region that touches the facet only at its boundary. It is also more than is needed: the neighbours
      across every facet are known.
    - *Departures.*
      - H1 is tested on unit-normalized rows to `1e-6`. Rows proportional in exact arithmetic whose rounding
        differs by more than `1e-6` would pass as unique, which is the unsafe direction but far above the
        rounding seen. Rows within `1e-6` that are not proportional only block a skip, which is safe.
      - The audit's 7,265 checks in which the old search ended another way are mostly the zero-row test
        failing to see the facet's own row vanish (pop_002 2,751). In exact arithmetic that row always
        vanishes on its own hyperplane, so these are numerical misses of the old path, which the index-first
        path does not depend on.
    - *The `Seen` table* (seen_set.rs: `key_of` 89-91, `find` 104-129).
      - `zobrist_toggle`: the key of `A xor {i}` is the key of `A` XOR the word of `i`, which is the update
        `idx_settled` uses.
      - `verified_lookup_iff`: when every entry is stored under its own set's key, a lookup that accepts
        only entries whose stored set equals the query finds exactly the stored sets, whatever the key
        function. Collisions only cost probes.
      - The two `verify` closures of `idx_settled` test that the stored set has `|A| - 1` elements, all in
        `A` and different from `j`, or `|A| + 1` elements, all in `A ∪ {g}`. For duplicate-free stored sets
        that is set equality.
      - Completeness of the probe sequence (open addressing with no deletions, stopping at an empty slot)
        is the standard invariant and is not formalized.
23. **Preprocessing merge, weakly active labels, clip budget (1cced96).**
    - *Duplicate-row merge* (program.rs `remove_duplicate_rows`, 458-486). After `scale_constraints`, rows of
      `[A | F | b]` that are equal within `1e-12` are merged: positive multiples of each other before scaling.
      The first copy is kept, except that an equality replaces an inequality it duplicates.
      - Inequality copies are the case of `Foundations/Twins.lean` (finding 20), with any idempotent choice
        of representative.
      - The equality case is covered by the same lemmas. A copy of an equality is tight wherever the
        equality holds (`Twins.tight_iff`), so it is implied. Folding its multiplier onto the equality,
        `μ = λ_eq + s λ_ineq`, gives the same combination of rows (`Twins.vecMul_split`, which needs no sign
        condition), and an equality's multiplier carries no sign or complementarity condition.
      - Negative multiples, an inequality `-a x ≤ -b` beside an equality `a x = b`, are not merged. The
        inequality stays as a weakly redundant row, as before the merge.
    - *Weakly active single label* (algorithms/mod.rs `drop_weakly_active_duplicates`, 162-202). Proved in
      `Optimizations/WeaklyActive.lean`.
      - `weakly_active_same_rows`: if `i ∈ A`, row `i` of `A`'s tableau is 0 (`λ_i = 0`), and `A \ {i}` has a
        tableau, the two tableaux have the same rows: the multipliers on `A \ {i}`, 0 at `i`, and the slacks
        off `A`. This uses `law_eq_kkt` on `A \ {i}` with `A`'s basic solution.
      - `weakly_active_same_region`: if `λ_i` vanishes at every parameter, the two labels have the same
        critical region and the same law.
      - An affine `λ_i` that vanishes on a full-dimensional region vanishes identically. So the code's test
        (the row `c_i`, `d_i` of the multiplier law zero within `1e-9`) is the right condition, and its
        `same_law` check is implied.
      - The tableau of `A \ {i}` exists in exact arithmetic, since a principal submatrix of the positive
        definite `M_AA` is positive definite. Dropping either label loses nothing.
      - `mplcp` never returns both: they would be two lexico-feasible bases on the same set, which
        `lex_unique` excludes. That is why the drop never fires for `mplcp`.
    - *Clip work budget* (mplcp_build.rs 406, `CLIP_BUDGET` 438). A clip that exceeds `2^18` work units hands the
      region to the LP path, which in exact arithmetic keeps the same rows (the rows that touch the
      polytope, finding 19). The budget changes only which path decides, not the result.
24. **Per-solve state, the facet certificate off by default, and the LP oracle (6cd2a18).**
    - *Per-solve state* (b57d52d). The box chain's fallback guard and the tableau budget's live-parent count
      (mplcp.rs 3889) now belong to one solve. Overlapping solves no longer share them. That fixes an
      overflow and changes nothing within one solve, so findings 18 and 19 are unchanged.
    - *Facet certificate off by default* (9fec1d5, `CERT` default 0, mplcp.rs 3039). The known-region skip of
      Case-2 facets (`known_region_unique_neighbour`, `known_region_skip`, findings 9 and 12) and its test
      `certificate_changes_no_region` (which sets the modes itself) now describe optional modes 1 to 4. The
      theorems are unchanged. The default search no longer relies on their trust assumptions (`hreg`,
      `hlex`, and the completeness of the clip vertices).
    - *The LP oracle.* Several arguments above use an LP answer that no certificate checks:
      - the per-row LP of the redundancy screen (finding 19);
      - the Chebyshev LPs of the builder (finding 21);
      - the bounding-box LPs of `theta_bbox` (findings 17 and 19);
      - the engine LPs of the box chain's fallbacks;
      - the start and fallback QPs.

      These arguments assume that the LP engine returns a true optimum or a true infeasibility verdict.
      That held only up to the `lp_core` defect of results finding 15, in flight: a pivot of `1.3e-9`
      accepted against an absolute `1e-9`, and an exit marked Optimal with a dual-infeasible basis. The
      certified pieces (`box_gap_certified`, `farkas_box` with the DAQP certificate, `chebyshev_radius_le`
      where a dual vector is checked) do not depend on the engine's optimality. `mplp_lex`'s own simplex is
      covered by its certificate theorems (`basis_certificate`) only for bases that pass its sign tests.

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
