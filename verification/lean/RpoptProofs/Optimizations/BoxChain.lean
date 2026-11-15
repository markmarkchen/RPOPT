/-
The chained bounding-box LPs of mplcp's region builder (`boxlp::box_chain`, used by
`mplcp_build::lp_path` at 6 or more parameters).

* `box_lp_optimal`: the acceptance test of an optimum. A feasible point `x`, and `c` a nonnegative
  combination of the rows that are tight at `x`, make `x` a maximizer of `c · x` over
  `{A x ≤ b}` (LP duality, the same certificate as `basis_certificate`). So every accepted box
  bound is the exact bound.
* `Case1.shared_point`: a point of the parent's region on the shared facet lies in the child's
  region and has the same tight rows there (the start vertex of the chain).
-/
import RpoptProofs.Facet

open Matrix

set_option linter.unusedSectionVars false

namespace Rpopt

/-- **The chain's acceptance test certifies optimality.** If `x` satisfies `A x ≤ b`, `y ≥ 0`,
`c = Aᵀ y`, and `y` is zero on every row that is not tight at `x`, then no feasible point has a
larger value of `c`. -/
theorem box_lp_optimal {m k : Type*} [Fintype m] [Fintype k] {A : Matrix m k ℝ} {b : m → ℝ}
    {c x : k → ℝ} {y : m → ℝ} (hy : ∀ j, 0 ≤ y j) (hc : y ᵥ* A = c)
    (hcs : ∀ j, y j ≠ 0 → (A *ᵥ x) j = b j) {x' : k → ℝ} (hx' : ∀ j, (A *ᵥ x') j ≤ b j) :
    c ⬝ᵥ x' ≤ c ⬝ᵥ x := by
  rw [← hc, ← dotProduct_mulVec, ← dotProduct_mulVec]
  have h1 : y ⬝ᵥ (A *ᵥ x') ≤ y ⬝ᵥ b := by
    simp only [dotProduct]
    exact Finset.sum_le_sum fun j _ => mul_le_mul_of_nonneg_left (hx' j) (hy j)
  have h2 : y ⬝ᵥ (A *ᵥ x) = y ⬝ᵥ b := by
    simp only [dotProduct]
    refine Finset.sum_congr rfl fun j _ => ?_
    by_cases hj : y j = 0
    · rw [hj, zero_mul, zero_mul]
    · rw [hcs j hj]
  linarith

/-- **How far an accepted bound can fall short** (df4176c). With `y ≥ 0` and `c = Aᵀ y + r`, the
value at any feasible `x'` exceeds the value at `x` by at most `y · (b - A x) + r · (x' - x)`. For
`r = 0` and `y` on the basis rows this is the bound of the module comment,
`Σ_i y_i (b_{T_i} - a_{T_i} · x)`; the residual tolerance adds the term `r · (x' - x)`. -/
theorem box_lp_gap {m k : Type*} [Fintype m] [Fintype k] {A : Matrix m k ℝ} {b : m → ℝ}
    {c x r : k → ℝ} {y : m → ℝ} (hy : ∀ j, 0 ≤ y j) (hc : y ᵥ* A + r = c) {x' : k → ℝ}
    (hx' : ∀ j, (A *ᵥ x') j ≤ b j) :
    c ⬝ᵥ x' - c ⬝ᵥ x ≤ y ⬝ᵥ (b - A *ᵥ x) + r ⬝ᵥ (x' - x) := by
  rw [← hc, add_dotProduct, add_dotProduct, ← dotProduct_mulVec, ← dotProduct_mulVec,
    dotProduct_sub, dotProduct_sub]
  have h1 : y ⬝ᵥ (A *ᵥ x') ≤ y ⬝ᵥ b := by
    simp only [dotProduct]
    exact Finset.sum_le_sum fun j _ => mul_le_mul_of_nonneg_left (hx' j) (hy j)
  linarith

namespace Case1

variable {Θ : Type*} {n : ℕ} {M : Matrix (Fin n) (Fin n) ℝ} {r : Θ → Fin n → ℝ}
  {B : Finset (Fin n)} {ρ : Θ → Fin n → ℝ} {T : Matrix (Fin n) (Fin n) ℝ} {h : Θ → ℝ}
  {Z : Finset (Fin n)} {γ : Fin n → ℝ} {v : Fin n}

/-- **The shared start vertex.** A point of the parent's region on the facet hyperplane lies in the
toggled child's region, and the same rows are tight there (the two tableaux agree on the facet). -/
theorem shared_point (H : Case1 M r B ρ T h Z γ v) {θ : Θ} (hθ : θ ∈ CR ρ) (h0 : h θ = 0) :
    θ ∈ CR H.ρ' ∧ ∀ i, (H.ρ' θ i = 0 ↔ ρ θ i = 0) := by
  have hv : ρ θ v = 0 := by rw [H.zero_rows v H.v_mem, h0, mul_zero]
  have e : H.ρ' θ = ρ θ := pivVec_of_zero hv
  refine ⟨?_, fun i => by rw [e]⟩
  have : θ ∈ CR ρ ∩ {θ | h θ = 0} := ⟨hθ, h0⟩
  rw [H.facet_eq] at this
  exact this.1

end Case1

end Rpopt
