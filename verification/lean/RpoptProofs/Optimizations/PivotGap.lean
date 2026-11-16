/-
Pivots in the undecided band decided from the data (mplcp.rs `refined_dependency`, `classify_pivot`,
the forced rank-1 call of `rank1_case2`): `rho = r' Q⁻¹ r / M_cc` with the residual
`r = a_c - Σ_j μ_j a_j` formed from the constraint rows.

* `residual_zero_of_rho`: with `Q⁻¹` positive definite, `r' Q⁻¹ r = 0` forces `r = 0`, that is, the
  exact dependency `a_c = Σ_j μ_j a_j`.
* `dep_row_of`, `dep_col_of`: the dependency of the rows of `A` gives the row and column relations of
  `M = A Q⁻¹ Aᵀ` that `DepFacet.of_dependency` takes.
* `depFacet_of_rho_zero`: so a call whose pivot is decided zero by `rho` has every `DepFacet` hypothesis,
  with the multipliers of the recomputed dependency. The tableau's own row `c` and its symmetry are
  consequences, which is why skipping their checks for that call loses nothing in exact arithmetic.
-/
import RpoptProofs.DependentFacet

open Matrix

set_option linter.unusedSectionVars false

namespace Rpopt

variable {ι κ : Type*} [Fintype ι] [Fintype κ]

/-- **`rho = 0` is an exact dependency.** With `Q⁻¹` positive definite, `r · (Q⁻¹ r) = 0` gives
`r = 0`. -/
theorem residual_zero_of_rho {Hinv : Matrix κ κ ℝ} (hH : Hinv.PosDef) {v : κ → ℝ}
    (h : v ⬝ᵥ (Hinv *ᵥ v) = 0) : v = 0 := by
  by_contra hv
  have := hH.dotProduct_mulVec_pos hv
  rw [star_trivial, h] at this
  exact lt_irrefl _ this

/-- Rows: `a_c = Σ μ_j a_j` gives `(A B)_{c k} = Σ μ_j (A B)_{j k}`. -/
theorem dep_row_of (Ax : Matrix ι κ ℝ) (B : Matrix κ ι ℝ) {S : Finset ι} {c : ι} {μ : ι → ℝ}
    (hdep : ∀ p, Ax c p = ∑ j ∈ S, μ j * Ax j p) (k : ι) :
    (Ax * B) c k = ∑ j ∈ S, μ j * (Ax * B) j k := by
  simp only [Matrix.mul_apply, hdep, Finset.sum_mul, Finset.mul_sum]
  rw [Finset.sum_comm]
  exact Finset.sum_congr rfl fun j _ => Finset.sum_congr rfl fun p _ => by ring

/-- Columns: `a_c = Σ μ_j a_j` gives `(C Aᵀ)_{k c} = Σ μ_j (C Aᵀ)_{k j}`. -/
theorem dep_col_of (C Ax : Matrix ι κ ℝ) {S : Finset ι} {c : ι} {μ : ι → ℝ}
    (hdep : ∀ p, Ax c p = ∑ j ∈ S, μ j * Ax j p) (k : ι) :
    (C * Axᵀ) k c = ∑ j ∈ S, μ j * (C * Axᵀ) k j := by
  simp only [Matrix.mul_apply, Matrix.transpose_apply, hdep, Finset.mul_sum]
  rw [Finset.sum_comm]
  exact Finset.sum_congr rfl fun j _ => Finset.sum_congr rfl fun p _ => by ring

/-- **A pivot decided zero by `rho` is a rank-1 facet.** If the residual of `a_c` against the active
rows `A` with the recomputed multipliers `μ` has `r · (Q⁻¹ r) = 0`, the tableau of `A` (for
`M = A Q⁻¹ Aᵀ`, independent active rows) has the full `DepFacet` structure with these `μ`. -/
theorem depFacet_of_rho_zero {Θ : Type*} [Nonempty Θ] [DecidableEq ι] {Ax : Matrix ι κ ℝ}
    {Hinv : Matrix κ κ ℝ} (hH : Hinv.PosDef) {r : Θ → ι → ℝ} {A : Finset ι} {ρ : Θ → ι → ℝ}
    {T : Matrix ι ι ℝ} {c : ι} {μ : ι → ℝ}
    (htab : ∀ θ, IsTableau (Ax * Hinv * Axᵀ) (r θ) A (ρ θ) T) (hc : c ∉ A)
    (hrho : (Ax c - ∑ j ∈ A, μ j • Ax j) ⬝ᵥ (Hinv *ᵥ (Ax c - ∑ j ∈ A, μ j • Ax j)) = 0)
    (hinjA : ∀ u : ι → ℝ, (∀ i, i ∉ A → u i = 0) →
      (∀ k ∈ A, ((Ax * Hinv * Axᵀ) *ᵥ u) k = 0) → u = 0) :
    DepFacet (Ax * Hinv * Axᵀ) r A ρ T c μ := by
  have hres := residual_zero_of_rho hH hrho
  have hdep : ∀ p, Ax c p = ∑ j ∈ A, μ j * Ax j p := by
    intro p
    have := congrFun hres p
    rw [Pi.sub_apply, Finset.sum_apply, Pi.zero_apply, sub_eq_zero] at this
    rw [this]
    exact Finset.sum_congr rfl fun j _ => by rw [Pi.smul_apply, smul_eq_mul]
  have hrowM : ∀ k, (Ax * Hinv * Axᵀ) c k = ∑ j ∈ A, μ j * (Ax * Hinv * Axᵀ) j k := by
    intro k
    rw [Matrix.mul_assoc]
    exact dep_row_of Ax (Hinv * Axᵀ) hdep k
  have hcolM : ∀ k, (Ax * Hinv * Axᵀ) k c = ∑ j ∈ A, μ j * (Ax * Hinv * Axᵀ) k j :=
    fun k => dep_col_of (Ax * Hinv) Ax hdep k
  exact DepFacet.of_dependency htab hc hrowM hcolM hinjA

end Rpopt
