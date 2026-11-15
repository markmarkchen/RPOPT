/-
B1. Basis optimality certificate for the LP of mplp_lex.rs:

  min dᵀx  s.t.  A x ≤ q,  rows in E with equality,
  d = c + Hθ,  q = b + Fθ  (`cost_at`, `rhs_at`).

A basis is a set `B` of active rows. With multipliers `λ` (zero off `B`, `λ_j ≥ 0` for inequality
rows), stationarity `d + Aᵀλ = 0` and primal feasibility imply optimality (weak duality).
-/
import Mathlib

open Matrix

set_option linter.unusedSectionVars false

namespace Rpopt

variable {m k : Type*} [Fintype m] [Fintype k]

/-- Primal feasibility of the LP. -/
def LPFeasible (A : Matrix m k ℝ) (q : m → ℝ) (E : Set m) (x : k → ℝ) : Prop :=
  ∀ j, (A *ᵥ x) j ≤ q j ∧ (j ∈ E → (A *ᵥ x) j = q j)

/-- Optimality of `x`. -/
def IsLPOptimal (d : k → ℝ) (A : Matrix m k ℝ) (q : m → ℝ) (E : Set m) (x : k → ℝ) : Prop :=
  LPFeasible A q E x ∧ ∀ y, LPFeasible A q E y → d ⬝ᵥ x ≤ d ⬝ᵥ y

/-- Dual feasibility: stationarity and sign of the inequality multipliers. -/
structure LPDualFeasible (d : k → ℝ) (A : Matrix m k ℝ) (E : Set m) (lam : m → ℝ) : Prop where
  stat : d + Aᵀ *ᵥ lam = 0
  sign : ∀ j, j ∉ E → 0 ≤ lam j

theorem dot_d_eq {d : k → ℝ} {A : Matrix m k ℝ} {E : Set m} {lam : m → ℝ}
    (hd : LPDualFeasible d A E lam) (y : k → ℝ) : d ⬝ᵥ y = -(lam ⬝ᵥ (A *ᵥ y)) := by
  have : d = -(Aᵀ *ᵥ lam) := by linear_combination hd.stat
  rw [this, neg_dotProduct, mulVec_transpose, ← dotProduct_mulVec]

/-- **Weak duality.** -/
theorem lp_weak_duality {d : k → ℝ} {A : Matrix m k ℝ} {q : m → ℝ} {E : Set m} {lam : m → ℝ}
    (hd : LPDualFeasible d A E lam) {y : k → ℝ} (hy : LPFeasible A q E y) :
    -(lam ⬝ᵥ q) ≤ d ⬝ᵥ y := by
  rw [dot_d_eq hd]
  have : lam ⬝ᵥ (A *ᵥ y) ≤ lam ⬝ᵥ q := by
    simp only [dotProduct]
    refine Finset.sum_le_sum fun j _ => ?_
    by_cases hj : j ∈ E
    · rw [(hy j).2 hj]
    · exact mul_le_mul_of_nonneg_left (hy j).1 (hd.sign j hj)
  linarith

/-- **B1 (certificate).** Primal feasibility, dual feasibility and complementary slackness imply
optimality, with value `-λᵀq`. -/
theorem lp_certificate {d : k → ℝ} {A : Matrix m k ℝ} {q : m → ℝ} {E : Set m} {x : k → ℝ}
    {lam : m → ℝ} (hx : LPFeasible A q E x) (hd : LPDualFeasible d A E lam)
    (hc : ∀ j, lam j * (q j - (A *ᵥ x) j) = 0) :
    IsLPOptimal d A q E x ∧ d ⬝ᵥ x = -(lam ⬝ᵥ q) := by
  have hval : d ⬝ᵥ x = -(lam ⬝ᵥ q) := by
    rw [dot_d_eq hd]
    have : lam ⬝ᵥ (A *ᵥ x) = lam ⬝ᵥ q := by
      simp only [dotProduct]
      refine Finset.sum_congr rfl fun j _ => ?_
      linear_combination -(hc j)
    rw [this]
  exact ⟨⟨hx, fun y hy => hval ▸ lp_weak_duality hd hy⟩, hval⟩

/-- **B1 (basis form).** A basis `B` whose basic solution is primal feasible (the nonbasic slack
rows are nonnegative: `R_B` primal part) and whose multipliers, zero off `B`, are dual feasible
(the multiplier rows are nonnegative: dual part) certifies an optimal vertex. -/
theorem basis_certificate {d : k → ℝ} {A : Matrix m k ℝ} {q : m → ℝ} {E : Set m}
    {B : Set m} {x : k → ℝ} {lam : m → ℝ} (hact : ∀ j ∈ B, (A *ᵥ x) j = q j)
    (hx : LPFeasible A q E x) (hoff : ∀ j, j ∉ B → lam j = 0)
    (hd : LPDualFeasible d A E lam) : IsLPOptimal d A q E x := by
  refine (lp_certificate hx hd fun j => ?_).1
  by_cases hj : j ∈ B
  · rw [hact j hj, sub_self, mul_zero]
  · rw [hoff j hj, zero_mul]

/-- Complementary slackness of an optimal pair: if `x` is optimal and `λ` is dual feasible with
dual value equal to the optimal value, then `λ_j (q_j - A_j x) = 0` for every row. -/
theorem complementary_of_optimal {d : k → ℝ} {A : Matrix m k ℝ} {q : m → ℝ} {E : Set m}
    {x : k → ℝ} {lam : m → ℝ} (hx : LPFeasible A q E x) (hd : LPDualFeasible d A E lam)
    (hval : d ⬝ᵥ x = -(lam ⬝ᵥ q)) : ∀ j, lam j * (q j - (A *ᵥ x) j) = 0 := by
  have hsum : ∑ j, lam j * (q j - (A *ᵥ x) j) = 0 := by
    have h1 := dot_d_eq hd x
    simp only [Finset.mul_sum, mul_sub, Finset.sum_sub_distrib]
    simp only [dotProduct] at h1 hval
    linarith
  have hnn : ∀ j, 0 ≤ lam j * (q j - (A *ᵥ x) j) := by
    intro j
    by_cases hj : j ∈ E
    · rw [(hx j).2 hj, sub_self, mul_zero]
    · exact mul_nonneg (hd.sign j hj) (sub_nonneg.mpr (hx j).1)
  intro j
  exact (Finset.sum_eq_zero_iff_of_nonneg fun j _ => hnn j).mp hsum j (Finset.mem_univ j)

end Rpopt
