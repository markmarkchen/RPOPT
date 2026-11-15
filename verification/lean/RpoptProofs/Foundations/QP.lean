/-
Strictly convex QP with linear inequality constraints: the KKT conditions are sufficient, and the
optimizer they certify is the unique one.

  min ½ xᵀ H x + dᵀ x   s.t.  A x ≤ b,      H ≻ 0.

In mplcp the cost and right-hand side are affine in θ (d = Fθ + c, b = b₀ + Sθ); the statements
here are pointwise in θ, so they hold for every θ.
-/
import Mathlib

open Matrix

namespace Rpopt

variable {n m : Type*} [Fintype n] [Fintype m]

/-- Objective `½ xᵀ H x + dᵀ x`. -/
noncomputable def qpObj (H : Matrix n n ℝ) (d x : n → ℝ) : ℝ :=
  (1 / 2 : ℝ) * (x ⬝ᵥ (H *ᵥ x)) + d ⬝ᵥ x

/-- `x` satisfies `A x ≤ b`. -/
def QPFeasible (A : Matrix m n ℝ) (b : m → ℝ) (x : n → ℝ) : Prop :=
  ∀ i, (A *ᵥ x) i ≤ b i

/-- `x` is a global minimizer of the QP. -/
def IsQPMinimizer (H : Matrix n n ℝ) (d : n → ℝ) (A : Matrix m n ℝ) (b : m → ℝ)
    (x : n → ℝ) : Prop :=
  QPFeasible A b x ∧ ∀ y, QPFeasible A b y → qpObj H d x ≤ qpObj H d y

/-- KKT conditions of the QP at `(x, λ)`. -/
structure QPKKT (H : Matrix n n ℝ) (d : n → ℝ) (A : Matrix m n ℝ) (b : m → ℝ)
    (x : n → ℝ) (lam : m → ℝ) : Prop where
  stationarity : H *ᵥ x + d + Aᵀ *ᵥ lam = 0
  primal_feasible : QPFeasible A b x
  dual_feasible : ∀ i, 0 ≤ lam i
  complementary : ∀ i, lam i * (b i - (A *ᵥ x) i) = 0

/-- Strict convexity in elementary form: `H` symmetric and `uᵀ H u > 0` for `u ≠ 0`. -/
structure StrictlyConvex (H : Matrix n n ℝ) : Prop where
  symm : Hᵀ = H
  pos : ∀ u : n → ℝ, u ≠ 0 → 0 < u ⬝ᵥ (H *ᵥ u)

/-- Mathlib's positive definiteness gives the elementary form. -/
theorem StrictlyConvex.of_posDef {H : Matrix n n ℝ} (hH : H.PosDef) : StrictlyConvex H where
  symm := by
    have h := hH.isHermitian
    rw [IsHermitian, conjTranspose_eq_transpose_of_trivial] at h
    exact h
  pos u hu := by simpa using hH.dotProduct_mulVec_pos hu

/-- Expansion of the objective around `x` (needs only symmetry of `H`). -/
theorem qpObj_sub {H : Matrix n n ℝ} (hs : Hᵀ = H) (d x y : n → ℝ) :
    qpObj H d y - qpObj H d x =
      (1 / 2 : ℝ) * ((y - x) ⬝ᵥ (H *ᵥ (y - x))) + (H *ᵥ x + d) ⬝ᵥ (y - x) := by
  set u := y - x with hu
  have hy : y = x + u := by rw [hu]; abel
  have hxu : x ⬝ᵥ (H *ᵥ u) = (H *ᵥ x) ⬝ᵥ u := by
    rw [dotProduct_mulVec, ← mulVec_transpose, hs]
  have hux : u ⬝ᵥ (H *ᵥ x) = (H *ᵥ x) ⬝ᵥ u := dotProduct_comm _ _
  rw [hy]
  simp only [qpObj, mulVec_add, dotProduct_add, add_dotProduct, hxu, hux]
  ring

/-- Under KKT, the first-order term is nonnegative towards every feasible point. -/
theorem kkt_first_order {H : Matrix n n ℝ} {d : n → ℝ} {A : Matrix m n ℝ} {b : m → ℝ}
    {x : n → ℝ} {lam : m → ℝ} (h : QPKKT H d A b x lam) {y : n → ℝ} (hy : QPFeasible A b y) :
    0 ≤ (H *ᵥ x + d) ⬝ᵥ (y - x) := by
  have hg : H *ᵥ x + d = -(Aᵀ *ᵥ lam) := by
    have := h.stationarity
    linear_combination this
  rw [hg, neg_dotProduct, mulVec_transpose, ← dotProduct_mulVec, mulVec_sub]
  have hc : lam ⬝ᵥ (b - A *ᵥ x) = 0 := by
    simp only [dotProduct, Pi.sub_apply]
    exact Finset.sum_eq_zero fun i _ => h.complementary i
  have hnn : 0 ≤ lam ⬝ᵥ (b - A *ᵥ y) := by
    simp only [dotProduct, Pi.sub_apply]
    exact Finset.sum_nonneg fun i _ =>
      mul_nonneg (h.dual_feasible i) (sub_nonneg.mpr (hy i))
  have : lam ⬝ᵥ (A *ᵥ y - A *ᵥ x) = lam ⬝ᵥ (b - A *ᵥ x) - lam ⬝ᵥ (b - A *ᵥ y) := by
    rw [← dotProduct_sub]; congr 1; abel
  rw [this, hc]
  linarith

/-- **A1 (sufficiency, strict).** A KKT point of a strictly convex QP is strictly better than
every other feasible point. -/
theorem kkt_strict {H : Matrix n n ℝ} (hH : StrictlyConvex H) {d : n → ℝ} {A : Matrix m n ℝ}
    {b : m → ℝ} {x : n → ℝ} {lam : m → ℝ} (h : QPKKT H d A b x lam) {y : n → ℝ}
    (hy : QPFeasible A b y) (hne : y ≠ x) : qpObj H d x < qpObj H d y := by
  have h1 := qpObj_sub hH.symm d x y
  have h2 := kkt_first_order h hy
  have h3 := hH.pos (y - x) (sub_ne_zero.mpr hne)
  linarith

/-- **A1 (sufficiency and uniqueness).** A KKT point of a strictly convex QP is its unique global
minimizer. -/
theorem kkt_unique_minimizer {H : Matrix n n ℝ} (hH : StrictlyConvex H) {d : n → ℝ}
    {A : Matrix m n ℝ} {b : m → ℝ} {x : n → ℝ} {lam : m → ℝ} (h : QPKKT H d A b x lam) :
    IsQPMinimizer H d A b x ∧ ∀ y, IsQPMinimizer H d A b y → y = x := by
  refine ⟨⟨h.primal_feasible, fun y hy => ?_⟩, fun y hy => ?_⟩
  · by_cases hyx : y = x
    · rw [hyx]
    · exact (kkt_strict hH h hy hyx).le
  · by_contra hne
    have := kkt_strict hH h hy.1 hne
    have := hy.2 x h.primal_feasible
    linarith

/-- **KKT necessity**, stated as an explicit hypothesis (not proved here). For linear constraints
no constraint qualification is needed; the classical proof is Farkas' lemma for the finitely
generated cone of the active constraint normals, whose closedness is not available in the
Mathlib version used. -/
def KKTNecessity (H : Matrix n n ℝ) (d : n → ℝ) (A : Matrix m n ℝ) (b : m → ℝ) : Prop :=
  ∀ x, IsQPMinimizer H d A b x → ∃ lam, QPKKT H d A b x lam

end Rpopt
