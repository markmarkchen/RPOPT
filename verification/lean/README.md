# RpoptProofs: formal verification of `mplcp` and `mplp_lex`

Lean 4 and Mathlib proofs of the mathematics behind two RPOPT solvers:

- `mplcp` (`src/algorithms/mplcp.rs`): parametric LCP pivoting for strictly convex mp-QPs,
  Jones and Morari (2006);
- `mplp_lex` (`src/algorithms/mplp_lex.rs`): lexicographic perturbation for mp-LPs, Jones, Kerrigan
  and Maciejowski (2007).

The proofs are at the model level, in exact real arithmetic. They justify the steps the Rust code
takes, not its floating point execution. Every result listed as proved has no `sorry`, and
`Audit.lean` prints the axioms of all 47 main theorems: each uses only `propext`,
`Classical.choice` and `Quot.sound` (two use none). Modelling assumptions are explicit hypotheses,
never axioms.

Rust line numbers refer to commit `7157ee7` (`mplcp: read each region's law from the pivot tableau
by default`).

## Build

Toolchain `leanprover/lean4:v4.35.0` (via elan), Mathlib rev `56a005855ea17e0b0958f73ec801ae140b0a1679`.

```sh
cd ppopt_rs/verification/lean
# reuse an already built Mathlib (APFS copy on write: no rebuild, no extra disk)
cp -c -R "../../../Report/Fluid Relaxation draft/lean/ScrapFrontier/.lake/packages" .lake/
# or download the prebuilt cache instead:  lake exe cache get
lake build                  # a few minutes on top of a built Mathlib
lake env lean Audit.lean    # one line per main theorem: the axioms it depends on
```

`.lake/` is ignored by git.

## Layout

| File | Content |
|:-----|:--------|
| `Foundations/QP.lean` | strictly convex QP, KKT conditions, sufficiency and uniqueness |
| `Foundations/LCP.lean` | the pLCP of the KKT system (`M`, `q`, `Q`, `x(λ)`), uniqueness of `w` |
| `Tableau.lean` | complementary bases, semantic tableaux, basic solutions, region law |
| `Pivot.lean` | dictionary pivot, principal pivot, `tableau_for` |
| `Lex.lean` | lexicographic order, symbolic `ε`, perturbation block from `T`, uniqueness |
| `Facet.lean` | Case 1 of Algorithm 2 (the fast path of `adjacent`) |
| `Algorithm2.lean` | the Case-2 lexicographic test of `pivot_facet` |
| `Coverage.lean` | breadth-first exploration as a reachability closure, skipping rules |
| `Optimizations/EntryFacetSkip.lean` | safety of the entry-facet skip |
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
| A4 | `Case1.facet_eq` | the shared facet is the same set from both sides: `CR_B ∩ H = CR_B' ∩ H` | proved |
| A4 | `Case1.cr_le`, `Case1.cr'_ge` | the two regions lie on opposite sides of the facet hyperplane | proved |
| A4 | `Case1.across`, `Case1.own_side` | near every relative-interior facet point, `B` covers its side and the toggled `B'` the other | proved |
| A4 | `Case1.lex_cover` | the same, lexicographically: near the facet `B` or `B'` is lexico-feasible at every point | proved |
| A5 | `lex_unique`, `lexRegion_disjoint` | two different lexico-feasible complementary bases are never valid at the same `θ` | proved |
| A5 | `LexPos.eventually_pos` | a lexico-positive row is positive for all small `ε > 0` (symbolic `ε`) | proved |
| A5 | `isLexTableau_of_isTableau`, `lexRows_pivot` | the perturbation block is implied by `T` (`rfull`) and pivots like the other columns | proved |
| A5 | `lex_ratio_pivot` | the lexicographic ratio test keeps every row lexico-positive | proved |
| Alg. 2 | `case2_accept_unique`, `gamma_scale` | the Case-2 test accepts at most one candidate when no `Γ` vanishes; it is scale-free | proved |
| Alg. 2 | Case 2 as a whole | the degenerate-facet step returns exactly the lexico-feasible neighbours | **open** (see Findings) |
| A6 | `reach_least`, `reach_skip` | exploration = least closed set; skipping explored candidates changes nothing | proved |
| A6 | facet-graph connectivity | the facet-adjacency graph of a facet-to-facet partition of a convex set is connected | **open**, hypothesis of `coverage_of_connected` |
| Skip | `entry_facet_unique_neighbour` | across the entry facet of a Case-1 child, the only lexico-feasible basis on the parent's side is the parent | proved |
| Skip | `reach_skip_parent` | omitting the explored parent does not change the explored set | proved |
| B1 | `lp_certificate`, `basis_certificate`, `lp_weak_duality` | primal and dual feasibility (with complementarity) imply optimality | proved |
| B2 | `rank1_inverse`, `rank1_tableau` | `Lex::pivot` keeps `β = A_B⁻¹` and `T = Aβ` | proved |
| B2 | `dual_pivot` | phase 1: the ratio test keeps the multipliers dual feasible and drives the leaving one to 0 | proved |
| B2 | `primal_direction`, `ratio_test`, `lex_ratio_test` | phase 2: the edge direction and the (lexicographic) ratio test keep primal feasibility | proved |
| B3 | `lexOptimal_unique`, `nondeg_basis_unique` | the lexico-optimal basis is unique for each `θ` (no overlaps) | proved |
| B3 | gap-freeness | every feasible `θ` has a lexico-optimal basis (termination of the lexicographic simplex) | **open** |

## Correspondence with the Rust code

`mplcp.rs` unless noted; `build` is `mplcp_build.rs`.

| Lean theorem | Rust function or step | What it justifies |
|:-------------|:----------------------|:------------------|
| `lcpM`, `lcpq`, `lcpQ`, `slack_primalOf` | `Plcp::new` (50-117) | the pLCP data `M, q, R` from the KKT cache |
| `lcp_unique_optimizer`, `region_law_optimal` | `tableau_law` (776-808), `complete_primal_law` | `x(θ) = -H⁻¹(c + Hθ + Aᵀλ(θ))` is the optimizer on the region |
| `law_eq_kkt` | `TABLEAU_LAW` switch (750), `tableau_law` vs `optimal_control_law_dual` | the tableau law equals the KKT-solve law (default since `7157ee7`) |
| `tableau_unique` | `REFACTOR` (680), `candidate_law` (813-857), `pivot_solve` (911-924) | a tableau reached by pivots equals the one rebuilt from scratch |
| `isTableau_empty`, `tableauFor_isTableau` | `tableau_for` (297-314; check `piv.abs() < 1e-11` at 308) | the region tableau rebuilt from an active set |
| `exchange_iff`, `pivVec`, `pivT` | `Tab::defer_pivot` (202-221), `Tab::flush` (263-288), `Tab::pivoted` (226-260), `Tab::tv`/`rv` (157-193) | the pivot formulas |
| `pivot_isTableau`, `toggle` | `Nbr::Toggle` (589), `z_toggle` (488-498), `expand_candidates` (735-738) | the neighbour's tableau is one principal pivot away |
| `pivot_agree_on_facet`, `Case1.facet_eq` | `adjacent` (553-632) | both sides describe the same facet |
| `Case1` (structure fields) | `adjacent`: `zrows` (560-572), `pv` (579), `tve` (585), `blocked` (587) | the exact conditions of the fast path |
| `Case1.across`, `Case1.lex_cover` | fast path return (589) | the toggled basis is the region across the facet |
| `Lex.LexPos`, `LexNonneg` | `lex_nonneg` (341-348), `lex_cmp` (327-338) | lexicographic sign, without tolerance |
| `LexPos.eventually_pos` | header (7-9): `ε = (ε0, ε0², …)` | symbolic `ε` is sound |
| `lexRows`, `isLexTableau_of_isTableau` | `rfull` in `pivot_facet` (370-395); `lex_feasible` (commit 88f948f) | the β block is `-T[:, c]` for nonbasic `w_j`, a unit vector for basic `w_j` |
| `lex_ratio_pivot`, `zero_row_beats_positive_row` | `pivot_facet` Case 1 (412-431) | the lexmin over `P ∩ Z` on β keeps lexico-feasibility |
| `lex_unique`, `lexRegion_disjoint` | `solve` doc (646-655): no overlaps | one region per `θ` |
| `case2_accept_unique`, `gamma_scale` | `pivot_facet` Case 2 (436-462), `gq_zero` (447), `lex_nonneg` (451) | the θ-independent part of the lexico-inequalities (16) |
| `reach_least` | `pivot_solve` (878-930), `pivot_solve_par` (944-1017), `seen` (880, 898, 987) | the explored set does not depend on the visiting order |
| `reach_skip` | `seen` check before `build_cand` (898-901); mplp_lex `crossed` (mplp_lex.rs 502, 509, 524-526, 557) | skipping known neighbours is safe |
| `entry_facet_unique_neighbour`, `reach_skip_parent` | entry-facet skip (branch `mplcp-entry-skip`) | the skipped search could only return the parent |
| `lp_certificate`, `basis_certificate` | mplp_lex.rs header (13-16), `solve` (264-377) termination at `Ok(())` (358) | a basis with no lexico-negative slack or multiplier row is optimal |
| `rank1_inverse`, `rank1_tableau` | mplp_lex.rs `Lex::pivot` (159-181) | the rank-1 update of `binv` and `t` |
| `dual_pivot`, `ratio_test` | mplp_lex.rs phase 1 (310-342), ratio `l / y` with `y > 1e-9` (324-338) | phase 1 keeps dual feasibility |
| `primal_direction`, `lex_ratio_test` | mplp_lex.rs phase 2 (343-374), `aj = -T_jp` (365), `slack_cmp` (226-258) | phase 2 keeps primal lexico-feasibility |
| `lexOptimal_unique` | mplp_lex.rs header (17-19): "regions are disjoint"; `slack_sign` (196-223), `lam_row` (380-395) | one lexico-optimal basis per `θ` |

## Modelling assumptions

All of these appear as explicit hypotheses of the theorems that need them.

- **Exact arithmetic.** `ZTOL`, `PTOL`, `1e-11`, `1e-12` and the point-location tolerance `1e-9` are 0.
  Tests such as `gam[i] < -PTOL` become `γ_i < 0`.
- **Strict convexity.** `H` is positive definite (`H.PosDef`, or `StrictlyConvex`).
- **Nonzero pivots.** `tableau_for` and the toggles use nonzero pivot elements (`NonzeroPivots`,
  `T v v ≠ 0`); the code checks `|piv| ≥ 1e-11`.
- **Principal-pivot tableaux.** Rows and columns are indexed by the constraint pairs, as for every
  region tableau in the Rust code. The work tableau of `pivot_facet` (extra `α` column, non-principal
  pivots) is covered only through `exchange_iff`, `lex_ratio_pivot` and `Algorithm2.lean`.
- **Inequality rows only in the pLCP.** The Schur-complement elimination of equality rows in
  `Plcp::new` (74-115) is not modelled.
- **Relative-interior facet points.** The facet results hold near points where every tableau row
  outside `Z` is strictly positive; the remaining facet points form a lower-dimensional set.
- **Lexico-feasible parent.** `entry_facet_unique_neighbour` assumes the parent basis is lexico-feasible
  somewhere on its side (`hP`), that the other rows of `Z` are degenerate (`hγ0`, which follows from
  `gam_eq_zero`), and that the β row of `v` is nonzero (true for any basis).
- **KKT necessity** (`KKTNecessity`) and **facet-graph connectivity** (`coverage_of_connected`) are
  hypotheses, see Status.

## Not covered

- Floating point, rounding, and every tolerance listed above.
- The hard caps of `pivot_facet`: recursion depth `> 6` (433) and `8n + 50` iterations (363).
- Case 2 of Algorithm 2 as a whole (sub-facet polytopes, Chebyshev radius `> 1e-9`, recursion), and the
  `α` variable in `pivot_facet`.
- The region builder (`build_region`, `build` 14-130): vertex clipping, redundancy removal, the Chebyshev
  center and the facet hint.
- Tableau refactoring and drift (`REFACTOR`, `mplp_lex` `REFRESH`), beyond the exact statement
  `tableau_unique`.
- Start-region selection (`initial_region`, `degenerate_start`, `lex_feasible_start` and
  `drop_lex_infeasible` of commit 88f948f, `mplp_lex` initial basis). Past bugs of this kind, such as the
  zero-region start from dependent active rows fixed in `668403e`, live at this level.
- The enumeration method (`Method::Enumeration`) and the parallel scheduling of `pivot_solve_par`
  (only its result set, via `reach_least`).
- In `mplp_lex`: the `γ` infinitesimal along the facet normal, the relative-interior point LP, and the
  concrete formulas of the lexicographic rows (they enter `lexOptimal_unique` as abstract coefficient
  vectors).

## Findings for the implementation

1. **Case-2 test reads `Γ` through an absolute tolerance** (`pivot_facet`, 446-454). On `pop_001`
   (`bench/problems_large`), parent `[3,5,11,14,15,18]`, facet 9, the twin rows 4 and 23 have
   `a ≈ 1.2e-10` (just above `PTOL`), so `Γ = a_l row_j - a_j row_l` has a β part of about `±1.2e-10`,
   below `ZTOL = 1e-9`. `gq_zero` is true and `lex_nonneg` reads the vector as zero in both directions,
   so both twins are accepted and the lexico-infeasible `[3,4,5,11,14,18,20]` is returned next to
   `[3,5,11,14,18,20,23]`. In the model this is the failing hypothesis `Γ ≠ 0` of `case2_accept_unique`.
   `gamma_scale` shows that the scale-free vector `row_j/a_j - row_l/a_l` gives the same exact answer;
   an experiment with that vector and a tolerance relative to its largest entry removes every
   lexico-infeasible region on `pop_001` without `drop_lex_infeasible` (3529 regions; the `dependent_start`,
   `mplcp`, `mplcp_rows`, `mplcp_tableau_law` and `overlap_strict` tests pass). Dividing by `a` alone is not
   enough: a second parent, `[1,5,12,17,18,20]`, then shows noise of `4.7e-6` deciding the sign, so the test
   has to be relative.
2. **The entry-facet skip needs a lexico-feasible parent.** `entry_facet_unique_neighbour` assumes it
   (`hP`). If the parent is a lexico-infeasible twin (possible while finding 1 is open; such parents are
   removed only at the end, by `drop_lex_infeasible`), the lexico-feasible basis across the entry facet is
   the parent's twin, not the parent. The skip then stops the child from finding the twin, and if no other
   region reaches it, the parameter set of the parent is left uncovered once the parent is dropped. The
   skip is safe once finding 1 is fixed at the source, or if it is applied only to parents that pass
   `lex_feasible`.
3. **Latent tolerance mismatch** (not observed to fail): `adjacent` finds the rows of `Z` with a relative
   tolerance (566-571), `pivot_facet` with an absolute one (`in_z`, 401). A row that vanishes on the facet
   but is large elsewhere can leave `P ∩ Z` empty at the first iteration, and Case 2 then takes a finite
   `α` step, away from the facet.

## Extending the proofs

New optimizations of `mplcp` get a file in `Optimizations/` that reduces to the lemmas above:

- **Known-neighbour skipping** (skip a facet whose neighbour is already known): `reach_skip` with the
  hypothesis that the skipped neighbour is explored. For mplcp the facet-to-neighbour map has to be the
  right one; across a Case-1 facet that is `entry_facet_unique_neighbour`, and across a Case-2 facet it
  needs the open Case-2 theorem.
- **Pruning in the degenerate branching** (Case 2): prove that a pruned branch leads only to bases that
  are not lexico-feasible near the facet, or to explored ones; `lex_unique`, `case2_accept_unique` and
  `reach_skip` are the tools. This is the place to formalize Case 2 in full: model the work tableau with
  the `α` column as a dictionary (`exchange_iff` already covers non-principal pivots) and prove that each
  pivot keeps the non-`α` rows lexico-nonnegative (`lex_ratio_pivot`).
- **Region rows read from the tableau** (the region `{θ : ρ(θ) ≥ 0}` without a KKT solve): `CR` is
  already defined from the tableau rows, and `law_eq_kkt` gives the slack rows (`i ∉ B`) as well as the
  multipliers, so the rows equal those of the KKT construction.
- **Skipped Chebyshev LP** (no interior point per region): the facet theorems only use a relative-interior
  point of the facet (`hrel`) and a point of the region on its side (`gam_eq_zero`); a replacement must
  still supply them, for example the parent's facet point, which `Case1.facet_eq` shows lies on the
  child's facet too.
- **Equality rows**: prove that the Schur-complement elimination of `Plcp::new` preserves the solution set
  (block elimination with `M_EE` invertible) and lift A1 to free multipliers.
- **KKT necessity**: closedness of finitely generated cones (conic Carathéodory) plus
  `ProperCone.hyperplane_separation_point` gives Farkas' lemma, which discharges `KKTNecessity`.
