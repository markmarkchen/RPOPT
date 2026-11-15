/-
The direct facet crossing of mplp_lex (`direct_cross`, mplp_lex.rs; commits aaacfda and 31a12ae).

The facet of the basis `B` (positions `p ↦ b p`, `β = A_B⁻¹`, `T = A β`) where the inactive row `j`
becomes active is crossed by one dual ratio test on row `T_j = A_j β`: the leaving position `p` is
chosen among `T_jp > 0` (inequality positions). `direct_cross` reports "nothing beyond" when no
inequality position has `T_jp > 1e-9`.

* `infeasible_beyond`: if `T_jp ≤ 0` at every inequality position, then at every parameter where
  `B`'s basic point violates row `j` (beyond the facet), the LP has no feasible point. It is a
  Farkas certificate: `A_j = Σ_p T_jp A_{b p}` (`row_eq_combination`), and each term bounds `A_j x`
  from below by `A_j x_B > q_j`. Since `T` does not depend on θ, the whole open half-space beyond
  the facet hyperplane is infeasible: the facet lies on the boundary of the feasible parameter set,
  and it has no neighbour.
* `reach_congr`: the explored set depends only on the neighbour relation. The success-rate gate
  only chooses, per facet, between two procedures that return the same neighbours, so it cannot
  change the explored set.
-/
import RpoptProofs.MplpLex.Pivot
import RpoptProofs.MplpLex.Certificate
import RpoptProofs.Coverage

open Matrix

set_option linter.unusedSectionVars false

namespace Rpopt

section Crossing

variable {m K : Type*} [Fintype m] [Fintype K] [DecidableEq K]

/-- `(A y)_j = Σ_p T_jp (A y)_{b p}` for every `y`. -/
theorem row_j_combination (A : Matrix m K ℝ) (b : K → m) {β : Matrix K K ℝ}
    (hβ : basisMat A b * β = 1) (j : m) (y : K → ℝ) :
    (A *ᵥ y) j = ∑ p, (A j ᵥ* β) p * (A *ᵥ y) (b p) := by
  have hrow := row_eq_combination A b hβ j
  have e1 : (A *ᵥ y) j = A j ⬝ᵥ y := rfl
  have e2 : A j ⬝ᵥ y = (A j ᵥ* β) ⬝ᵥ (basisMat A b *ᵥ y) := by
    conv_lhs => rw [hrow]
    exact (dotProduct_mulVec _ _ _).symm
  rw [e1, e2]
  rfl

/-- **No admissible row in the dual ratio test: nothing feasible beyond the facet.** If every
inequality position has `T_jp ≤ 0`, and `B`'s basic point `x_B` (tight on the basic rows) violates
row `j`, then no `x` satisfies the constraints. -/
theorem infeasible_beyond (A : Matrix m K ℝ) (b : K → m) {β : Matrix K K ℝ}
    (hβ : basisMat A b * β = 1) (E : Set m) (q : m → ℝ) (j : m)
    (hT : ∀ p, b p ∉ E → (A j ᵥ* β) p ≤ 0) {xB : K → ℝ}
    (hxB : ∀ p, (A *ᵥ xB) (b p) = q (b p)) (hbeyond : q j < (A *ᵥ xB) j) :
    ∀ x, ¬ LPFeasible A q E x := by
  intro x hx
  have h1 := row_j_combination A b hβ j x
  have h2 := row_j_combination A b hβ j xB
  have hle : ∑ p, (A j ᵥ* β) p * q (b p) ≤ ∑ p, (A j ᵥ* β) p * (A *ᵥ x) (b p) :=
    Finset.sum_le_sum fun p _ => by
      by_cases hp : b p ∈ E
      · rw [(hx (b p)).2 hp]
      · exact mul_le_mul_of_nonpos_left (hx (b p)).1 (hT p hp)
  have h3 : (A *ᵥ xB) j = ∑ p, (A j ᵥ* β) p * q (b p) := by
    rw [h2]; exact Finset.sum_congr rfl fun p _ => by rw [hxB p]
  have := (hx j).1
  linarith

end Crossing

/-- **The explored set depends only on the neighbour relation.** Choosing, facet by facet, between
two procedures that return the same neighbours (the direct crossing and the LP path) cannot change
it. -/
theorem reach_congr {α : Type*} {adj₁ adj₂ : α → α → Prop} (h : ∀ x y, adj₁ x y ↔ adj₂ x y)
    (s : α) : Reach adj₁ s = Reach adj₂ s := by
  ext x
  constructor
  · intro hx
    induction hx with
    | refl => exact Relation.ReflTransGen.refl
    | tail _ hab ih => exact Relation.ReflTransGen.tail ih ((h _ _).mp hab)
  · intro hx
    induction hx with
    | refl => exact Relation.ReflTransGen.refl
    | tail _ hab ih => exact Relation.ReflTransGen.tail ih ((h _ _).mpr hab)

end Rpopt
