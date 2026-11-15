-- Axiom audit: every main theorem must depend only on propext, Classical.choice, Quot.sound.
-- Run:  lake env lean Audit.lean
import RpoptProofs

-- A1 KKT and LCP
#print axioms Rpopt.kkt_unique_minimizer
#print axioms Rpopt.lcp_kkt
#print axioms Rpopt.lcp_unique_optimizer
#print axioms Rpopt.kkt_lcp
#print axioms Rpopt.optimizer_iff_lcp
#print axioms Rpopt.lcpM_posSemidef
#print axioms Rpopt.lcp_w_unique
-- A2 basis semantics
#print axioms Rpopt.basic_solves
#print axioms Rpopt.law_eq_kkt
#print axioms Rpopt.tableau_unique
#print axioms Rpopt.region_law_optimal
-- A3 principal pivot
#print axioms Rpopt.exchange_iff
#print axioms Rpopt.pivot_isTableau
#print axioms Rpopt.pivot_agree_on_facet
#print axioms Rpopt.tableauFor_isTableau
#print axioms Rpopt.pivotSeq_eq_of_isTableau
-- A4 Case-1 adjacency
#print axioms Rpopt.Case1.isTableau'
#print axioms Rpopt.Case1.cr_le
#print axioms Rpopt.Case1.cr'_ge
#print axioms Rpopt.Case1.facet_eq
#print axioms Rpopt.Case1.across
#print axioms Rpopt.Case1.own_side
#print axioms Rpopt.Case1.lex_cover
-- A5 lexicographic perturbation
#print axioms Rpopt.LexPos.eventually_pos
#print axioms Rpopt.lex_ratio_pivot
#print axioms Rpopt.isLexTableau_of_isTableau
#print axioms Rpopt.lexRows_pivot
#print axioms Rpopt.lex_unique
#print axioms Rpopt.lexRegion_disjoint
-- Algorithm 2, Case 2 test
#print axioms Rpopt.gamma_scale
#print axioms Rpopt.case2_accept_unique
-- A6 coverage (graph part) and the entry-facet skip
#print axioms Rpopt.reach_least
#print axioms Rpopt.reach_skip
#print axioms Rpopt.reach_skip_parent
#print axioms Rpopt.coverage_of_connected
#print axioms Rpopt.entry_facet_unique_neighbour
#print axioms Rpopt.entry_facet_both_sides
-- B mplp_lex
#print axioms Rpopt.lp_weak_duality
#print axioms Rpopt.lp_certificate
#print axioms Rpopt.basis_certificate
#print axioms Rpopt.ratio_test
#print axioms Rpopt.lex_ratio_test
#print axioms Rpopt.rank1_inverse
#print axioms Rpopt.rank1_tableau
#print axioms Rpopt.dual_pivot
#print axioms Rpopt.primal_direction
#print axioms Rpopt.nondeg_basis_unique
#print axioms Rpopt.lexOptimal_unique
-- Round 2: step-2 certificate (known-region skip)
#print axioms Rpopt.vertices_subset_polyhedron
#print axioms Rpopt.active_row_multiple
#print axioms Rpopt.near_side_subset
#print axioms Rpopt.lexFeasible_of_mem_interior
#print axioms Rpopt.known_region_unique_neighbour
#print axioms Rpopt.known_region_skip
-- Round 2: Case 2 of Algorithm 2 at a facet point
#print axioms Rpopt.lex_ratio_pivot_on
#print axioms Rpopt.first_step_rows
#print axioms Rpopt.lexmin_zero_row
#print axioms Rpopt.first_step_nonAlpha
#print axioms Rpopt.pivVec_num_zero
#print axioms Rpopt.terminal_rows
#print axioms Rpopt.ray3_eventually
#print axioms Rpopt.ray_lexFeasible
#print axioms Rpopt.walk_sound
#print axioms Rpopt.exists_lexmin
#print axioms Rpopt.case2_lexmin_accepted
#print axioms Rpopt.neighbour_unique_on_ray
#print axioms Rpopt.lcp_multipliers_dependent
#print axioms Rpopt.case2_needs_dependent_rows
-- Round 2: termination of the facet walk (the two caps)
#print axioms Rpopt.chain_returns_to_one
#print axioms Rpopt.chain_no_repeat
#print axioms Rpopt.chain_length_bound
#print axioms Rpopt.lex_pivot_reverse_on
#print axioms Rpopt.walkStep_allLexPos
#print axioms Rpopt.walkStep_nonAlphaLexPos
#print axioms Rpopt.walkStep_injective_full
#print axioms Rpopt.walkStep_injective_inv
#print axioms Rpopt.walk_no_repeat
#print axioms Rpopt.walk_repeat_returns
-- Round 3b: region construction and start fallback
#print axioms Rpopt.relint_of_strict
#print axioms Rpopt.chebyshev_ball_feasible
