/-
Checked infeasibility certificates for `{x : A x ≤ b}` on a box (mplcp.rs `daqp_certainly_empty`,
`Plcp::coord_bound`; pdaqp.rs `Daqp::certificate`).

* `farkas_box`: `p ≥ 0`, `r = Aᵀ p`, `g = bᵀ p` and `g + ‖r‖₁ R < 0` imply that no `x` with
  `‖x‖∞ ≤ R` satisfies `A x ≤ b`. Since 0215a64 the code clamps `p` to `max(p, 0)` and recomputes
  `r` and `g`, so this is its exact test.
* `farkas_box_signed`: the same for a multiplier vector with a small negative part `p⁻`, when every
  row has `‖a_i‖₁ ≤ α`: the condition is `g + ‖r‖₁ R + Σ p⁻_i (|b_i| + α R) < 0`. Before 0215a64 the
  code charged only `‖p⁻‖₁ R`.
* `facet_coord_le`: with an orthonormal facet basis, `‖θ_f‖₂ ≤ ‖θ‖₂`, so `‖θ_f‖∞ ≤ R` holds on every
  sub-facet polytope as soon as the parameters satisfy `‖θ‖₂ ≤ R`.
* `box_sq_le`, `daqp_box_certificate`: with `R = √(Σ_k max(lo_k², hi_k²))`, the Euclidean radius of
  the parameter box (`coord_bound`), the certificate excludes every parameter of the box.
-/
import Mathlib

open Matrix

set_option linter.unusedSectionVars false

namespace Rpopt

variable {ι κ : Type*} [Fintype ι] [Fintype κ]

/-- `|r · x| ≤ ‖r‖₁ R` when `‖x‖∞ ≤ R`. -/
theorem dot_le_l1_mul_bound (r x : κ → ℝ) {R : ℝ} (hx : ∀ k, |x k| ≤ R) :
    -(∑ k, |r k|) * R ≤ r ⬝ᵥ x := by
  have : ∀ k, -(|r k| * R) ≤ r k * x k := by
    intro k
    have h1 : |r k * x k| ≤ |r k| * R := by
      rw [abs_mul]; exact mul_le_mul_of_nonneg_left (hx k) (abs_nonneg _)
    linarith [neg_abs_le (r k * x k)]
  simp only [dotProduct, neg_mul, Finset.sum_mul]
  rw [← Finset.sum_neg_distrib]
  exact Finset.sum_le_sum fun k _ => this k

/-- **Farkas certificate on a box.** -/
theorem farkas_box (A : Matrix ι κ ℝ) (b : ι → ℝ) {p : ι → ℝ} (hp : ∀ i, 0 ≤ p i) {R : ℝ}
    (hcert : p ⬝ᵥ b + (∑ k, |(p ᵥ* A) k|) * R < 0) :
    ¬ ∃ x : κ → ℝ, (∀ k, |x k| ≤ R) ∧ ∀ i, (A *ᵥ x) i ≤ b i := by
  rintro ⟨x, hx, hAx⟩
  have h1 : p ⬝ᵥ (A *ᵥ x) ≤ p ⬝ᵥ b := by
    simp only [dotProduct]
    exact Finset.sum_le_sum fun i _ => mul_le_mul_of_nonneg_left (hAx i) (hp i)
  have h2 := dot_le_l1_mul_bound (p ᵥ* A) x hx
  rw [← dotProduct_mulVec] at h2
  linarith

/-- **Farkas certificate with a signed multiplier vector.** If every row has `Σ_k |A i k| ≤ α`, a
multiplier vector `p` with negative part `p⁻` still proves emptiness on the box when
`p · b + ‖Aᵀ p‖₁ R + Σ p⁻_i (|b_i| + α R) < 0`. -/
theorem farkas_box_signed (A : Matrix ι κ ℝ) (b : ι → ℝ) (p : ι → ℝ) {R α : ℝ} (hR : 0 ≤ R)
    (hrow : ∀ i, ∑ k, |A i k| ≤ α)
    (hcert : p ⬝ᵥ b + (∑ k, |(p ᵥ* A) k|) * R + ∑ i, max (-p i) 0 * (|b i| + α * R) < 0) :
    ¬ ∃ x : κ → ℝ, (∀ k, |x k| ≤ R) ∧ ∀ i, (A *ᵥ x) i ≤ b i := by
  rintro ⟨x, hx, hAx⟩
  -- each term: p_i (A x)_i ≤ p_i b_i + p⁻_i (|b_i| + α R)
  have hterm : ∀ i, p i * (A *ᵥ x) i ≤ p i * b i + max (-p i) 0 * (|b i| + α * R) := by
    intro i
    have hAxi : |(A *ᵥ x) i| ≤ α * R := by
      have : |(A *ᵥ x) i| ≤ ∑ k, |A i k| * R := by
        simp only [mulVec, dotProduct]
        refine (Finset.abs_sum_le_sum_abs _ _).trans (Finset.sum_le_sum fun k _ => ?_)
        rw [abs_mul]; exact mul_le_mul_of_nonneg_left (hx k) (abs_nonneg _)
      rw [← Finset.sum_mul] at this
      exact this.trans (mul_le_mul_of_nonneg_right (hrow i) hR)
    rcases le_total 0 (p i) with hpi | hpi
    · have : max (-p i) 0 = 0 := max_eq_right (by linarith)
      rw [this, zero_mul, add_zero]
      exact mul_le_mul_of_nonneg_left (hAx i) hpi
    · have : max (-p i) 0 = -p i := max_eq_left (by linarith)
      rw [this]
      have hb : b i ≤ |b i| := le_abs_self _
      have ha : -(α * R) ≤ (A *ᵥ x) i := (abs_le.mp hAxi).1
      have hnn : 0 ≤ (A *ᵥ x) i - b i + |b i| + α * R := by linarith
      have key := mul_nonpos_of_nonpos_of_nonneg hpi hnn
      ring_nf at key ⊢
      linarith
  have h1 : p ⬝ᵥ (A *ᵥ x) ≤ p ⬝ᵥ b + ∑ i, max (-p i) 0 * (|b i| + α * R) := by
    simp only [dotProduct]
    rw [← Finset.sum_add_distrib]
    exact Finset.sum_le_sum fun i _ => hterm i
  have h2 := dot_le_l1_mul_bound (p ᵥ* A) x hx
  rw [← dotProduct_mulVec] at h2
  linarith

/-- **Facet coordinates are no larger than the parameters.** If `θ = N θ_f + c g` with `N`'s
columns orthonormal and orthogonal to `g` (`null_basis`), then `‖θ_f‖₂² ≤ ‖θ‖₂²`. -/
theorem facet_coord_le {π δ : Type*} [Fintype π] [Fintype δ] [DecidableEq δ] (N : Matrix π δ ℝ)
    (g : π → ℝ)
    (hN : Nᵀ * N = 1) (hNg : Nᵀ *ᵥ g = 0) (θf : δ → ℝ) (c : ℝ) :
    θf ⬝ᵥ θf ≤ (N *ᵥ θf + c • g) ⬝ᵥ (N *ᵥ θf + c • g) := by
  classical
  have h1 : (N *ᵥ θf) ⬝ᵥ (N *ᵥ θf) = θf ⬝ᵥ θf := by
    rw [dotProduct_mulVec, ← mulVec_transpose, mulVec_mulVec, hN, one_mulVec]
  have h2 : (N *ᵥ θf) ⬝ᵥ g = 0 := by
    rw [dotProduct_comm, dotProduct_mulVec, ← mulVec_transpose, hNg, zero_dotProduct]
  have hg : 0 ≤ g ⬝ᵥ g := by
    simp only [dotProduct]; exact Finset.sum_nonneg fun i _ => mul_self_nonneg _
  simp only [dotProduct_add, add_dotProduct, dotProduct_smul, smul_dotProduct, smul_eq_mul, h1]
  rw [dotProduct_comm g (N *ᵥ θf), h2]
  nlinarith [mul_nonneg (mul_self_nonneg c) hg]

/-- A point of the box `lo ≤ θ ≤ hi` has `θ · θ ≤ Σ_k max(lo_k², hi_k²)`, the square of
`Plcp::coord_bound`. -/
theorem box_sq_le {π : Type*} [Fintype π] (lo hi θ : π → ℝ) (h : ∀ k, lo k ≤ θ k ∧ θ k ≤ hi k) :
    θ ⬝ᵥ θ ≤ ∑ k, max (lo k ^ 2) (hi k ^ 2) := by
  simp only [dotProduct]
  refine Finset.sum_le_sum fun k _ => ?_
  obtain ⟨h1, h2⟩ := h k
  rcases le_total 0 (θ k) with h0 | h0
  · exact le_max_of_le_right (by nlinarith)
  · exact le_max_of_le_left (by nlinarith)

/-- **The DAQP filter with the radius of the parameter box** (`daqp_certainly_empty` with
`bound = coord_bound`). Let `θ = N θ_f + c g` be the facet coordinates (`null_basis`). If `p ≥ 0` and
`p · b + ‖Aᵀ p‖₁ R < 0` with `R = √(Σ_k max(lo_k², hi_k²))`, no parameter in the box `lo ≤ θ ≤ hi`
has facet coordinates with `A θ_f ≤ b`. -/
theorem daqp_box_certificate {π δ : Type*} [Fintype π] [Fintype δ] [DecidableEq δ]
    (N : Matrix π δ ℝ) (g : π → ℝ) (hN : Nᵀ * N = 1) (hNg : Nᵀ *ᵥ g = 0) (c : ℝ)
    (lo hi : π → ℝ) (A : Matrix ι δ ℝ) (b : ι → ℝ) {p : ι → ℝ} (hp : ∀ i, 0 ≤ p i)
    (hcert : p ⬝ᵥ b + (∑ k, |(p ᵥ* A) k|) * Real.sqrt (∑ k, max (lo k ^ 2) (hi k ^ 2)) < 0) :
    ¬ ∃ θf : δ → ℝ, (∀ k, lo k ≤ (N *ᵥ θf + c • g) k ∧ (N *ᵥ θf + c • g) k ≤ hi k) ∧
      ∀ i, (A *ᵥ θf) i ≤ b i := by
  rintro ⟨θf, hbox, hA⟩
  refine farkas_box A b hp hcert ⟨θf, fun k => ?_, hA⟩
  have hs : θf ⬝ᵥ θf ≤ ∑ k, max (lo k ^ 2) (hi k ^ 2) :=
    (facet_coord_le N g hN hNg θf c).trans (box_sq_le lo hi _ hbox)
  have hk : θf k ^ 2 ≤ θf ⬝ᵥ θf := by
    simp only [dotProduct]
    have := Finset.single_le_sum (f := fun j => θf j * θf j)
      (fun j _ => mul_self_nonneg (θf j)) (Finset.mem_univ k)
    simpa [sq] using this
  exact Real.abs_le_sqrt (hk.trans hs)

end Rpopt
