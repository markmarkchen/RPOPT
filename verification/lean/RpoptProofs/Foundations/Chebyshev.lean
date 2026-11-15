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

end Rpopt
