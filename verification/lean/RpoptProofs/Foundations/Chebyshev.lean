/-
Chebyshev balls (program.rs `feasible_space_chebychev_ball`, `ball_points`; the start samplers of
`gen_optimal_active_set` and `degenerate_start` at many parameters).

The Chebyshev LP returns a centre `c` and radius `r` with `a_k · c + r ‖a_k‖ ≤ b_k` for every
inequality row. Every point within `r` of `c` then satisfies every inequality row. The samplers
move only θ and keep `x` at the centre's value, so the joint distance is the θ distance.

Not covered: equality rows. The Chebyshev LP imposes them on the centre only, and moving θ alone
breaks `A_E x = b_E + F_E θ` unless `F_E u = 0`; such samples are rejected later by the QP solve.
-/
import Mathlib

open scoped RealInnerProductSpace

namespace Rpopt

variable {ι E : Type*} [NormedAddCommGroup E] [InnerProductSpace ℝ E]

/-- **Points of the Chebyshev ball are feasible** (inequality rows). -/
theorem chebyshev_ball_feasible {a : ι → E} {b : ι → ℝ} {c : E} {r : ℝ}
    (hc : ∀ k, ⟪a k, c⟫ + r * ‖a k‖ ≤ b k) {z : E} (hz : ‖z - c‖ ≤ r) : ∀ k, ⟪a k, z⟫ ≤ b k := by
  intro k
  have h1 : ⟪a k, z - c⟫ ≤ ‖a k‖ * ‖z - c‖ := real_inner_le_norm _ _
  have h2 : ‖a k‖ * ‖z - c‖ ≤ ‖a k‖ * r := mul_le_mul_of_nonneg_left hz (norm_nonneg _)
  have h3 := hc k
  rw [inner_sub_right] at h1
  nlinarith

/-- **A certificate that the Chebyshev radius is small** (the emptiness test of `build_region`).
If `y ≥ 0`, `Σ y_k a_k = 0` and `Σ y_k ‖a_k‖ = 1`, every centre and radius that satisfy the Chebyshev
rows have `r ≤ Σ y_k b_k`. So a dual vector with `Σ y_k b_k ≤ 1e-8` proves radius at most `1e-8`
whatever the LP's start; an LP answer without such a vector proves nothing. -/
theorem chebyshev_radius_le [Fintype ι] {a : ι → E} {b y : ι → ℝ} (hy : ∀ k, 0 ≤ y k)
    (hsum : ∑ k, y k • a k = 0) (hw : ∑ k, y k * ‖a k‖ = 1) {c : E} {r : ℝ}
    (hc : ∀ k, ⟪a k, c⟫ + r * ‖a k‖ ≤ b k) : r ≤ ∑ k, y k * b k := by
  have h1 : ∑ k, y k * (⟪a k, c⟫ + r * ‖a k‖) ≤ ∑ k, y k * b k :=
    Finset.sum_le_sum fun k _ => mul_le_mul_of_nonneg_left (hc k) (hy k)
  have h2 : ∑ k, y k * ⟪a k, c⟫ = 0 := by
    have : ⟪∑ k, y k • a k, c⟫ = ∑ k, y k * ⟪a k, c⟫ := by
      rw [sum_inner]
      simp [real_inner_smul_left]
    rw [← this, hsum, inner_zero_left]
  have h3 : ∑ k, y k * (⟪a k, c⟫ + r * ‖a k‖) = ∑ k, y k * ⟪a k, c⟫ + r * ∑ k, y k * ‖a k‖ := by
    rw [Finset.mul_sum, ← Finset.sum_add_distrib]
    exact Finset.sum_congr rfl fun k _ => by ring
  rw [h3, h2, hw] at h1
  linarith

end Rpopt
