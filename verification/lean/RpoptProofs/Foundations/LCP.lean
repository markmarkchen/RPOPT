/-
From the KKT system of the mp-QP to the parametric LCP (Jones and Morari 2006, tutorial eq. (plcp)).

  min ½ xᵀHx + (Fθ + c)ᵀx  s.t.  A x ≤ b + Sθ,   H ≻ 0
  w = q + Qθ + M λ,   M = A H⁻¹ Aᵀ,  q = b + A H⁻¹ c,  Q = S + A H⁻¹ F,
  x = -H⁻¹ (Fθ + c + Aᵀ λ).

Rust: `Plcp::new` (mplcp.rs) reads M, q, R = Q from the KKT cache; the tableau starts from
rows [q | R] with T = M (`tableau_for`).
-/
import RpoptProofs.Foundations.QP

open Matrix

namespace Rpopt

variable {n m p : Type*} [Fintype n] [Fintype m] [Fintype p] [DecidableEq n]

/-- A solution of the LCP  `w = r + M z, w ≥ 0, z ≥ 0, w_i z_i = 0`. -/
structure LCPSol (M : Matrix m m ℝ) (r : m → ℝ) (w z : m → ℝ) : Prop where
  eq : w = r + M *ᵥ z
  w_nonneg : ∀ i, 0 ≤ w i
  z_nonneg : ∀ i, 0 ≤ z i
  comp : ∀ i, w i * z i = 0

/-- `M = A H⁻¹ Aᵀ`. -/
noncomputable def lcpM (H : Matrix n n ℝ) (A : Matrix m n ℝ) : Matrix m m ℝ := A * H⁻¹ * Aᵀ

/-- `q = b + A H⁻¹ c`. -/
noncomputable def lcpq (H : Matrix n n ℝ) (A : Matrix m n ℝ) (b : m → ℝ) (c : n → ℝ) : m → ℝ :=
  b + A *ᵥ (H⁻¹ *ᵥ c)

/-- `Q = S + A H⁻¹ F` (called `R` in mplcp.rs). -/
noncomputable def lcpQ (H : Matrix n n ℝ) (A : Matrix m n ℝ) (S : Matrix m p ℝ)
    (F : Matrix n p ℝ) : Matrix m p ℝ := S + A * H⁻¹ * F

/-- The primal point of a multiplier vector: `x = -H⁻¹ (Fθ + c + Aᵀ λ)`. -/
noncomputable def primalOf (H : Matrix n n ℝ) (A : Matrix m n ℝ) (F : Matrix n p ℝ)
    (c : n → ℝ) (θ : p → ℝ) (lam : m → ℝ) : n → ℝ :=
  -(H⁻¹ *ᵥ (F *ᵥ θ + c + Aᵀ *ᵥ lam))

/-- The pLCP right-hand side at `θ`: `q + Qθ`. -/
noncomputable def lcpRhs (H : Matrix n n ℝ) (A : Matrix m n ℝ) (b : m → ℝ) (c : n → ℝ)
    (S : Matrix m p ℝ) (F : Matrix n p ℝ) (θ : p → ℝ) : m → ℝ :=
  lcpq H A b c + lcpQ H A S F *ᵥ θ

/-- The slack of the primal point of `λ` is the pLCP's `w`:
`b + Sθ - A x(λ) = q + Qθ + M λ`. -/
theorem slack_primalOf (H : Matrix n n ℝ) (A : Matrix m n ℝ) (b : m → ℝ) (c : n → ℝ)
    (S : Matrix m p ℝ) (F : Matrix n p ℝ) (θ : p → ℝ) (lam : m → ℝ) :
    b + S *ᵥ θ - A *ᵥ primalOf H A F c θ lam =
      lcpRhs H A b c S F θ + lcpM H A *ᵥ lam := by
  simp only [primalOf, lcpRhs, lcpq, lcpQ, lcpM, mulVec_neg, mulVec_add, add_mulVec,
    mulVec_mulVec, Matrix.mul_assoc]
  abel

/-- Stationarity holds by construction of `x(λ)` when `H` is invertible. -/
theorem stationarity_primalOf {H : Matrix n n ℝ} (hH : IsUnit H.det) (A : Matrix m n ℝ)
    (F : Matrix n p ℝ) (c : n → ℝ) (θ : p → ℝ) (lam : m → ℝ) :
    H *ᵥ primalOf H A F c θ lam + (F *ᵥ θ + c) + Aᵀ *ᵥ lam = 0 := by
  simp only [primalOf, mulVec_neg, mulVec_mulVec, mul_nonsing_inv H hH, one_mulVec]
  abel

/-- **A1 (pLCP to QP).** If `(w, λ)` solves the pLCP at `θ`, then `x(λ)` with `λ` satisfies the
KKT conditions of the QP at `θ`, so `x(λ)` is the unique optimizer. -/
theorem lcp_kkt {H : Matrix n n ℝ} (hH : IsUnit H.det) (A : Matrix m n ℝ) (b : m → ℝ)
    (c : n → ℝ) (S : Matrix m p ℝ) (F : Matrix n p ℝ) (θ : p → ℝ) {w lam : m → ℝ}
    (hs : LCPSol (lcpM H A) (lcpRhs H A b c S F θ) w lam) :
    QPKKT H (F *ᵥ θ + c) A (b + S *ᵥ θ) (primalOf H A F c θ lam) lam := by
  have hw : b + S *ᵥ θ - A *ᵥ primalOf H A F c θ lam = w := by
    rw [slack_primalOf, hs.eq]
  refine ⟨stationarity_primalOf hH A F c θ lam, fun i => ?_, hs.z_nonneg, fun i => ?_⟩
  · have := congrFun hw i
    simp only [Pi.sub_apply] at this
    have := hs.w_nonneg i
    linarith
  · have := congrFun hw i
    simp only [Pi.sub_apply] at this
    rw [this, mul_comm]
    exact hs.comp i

/-- **A1, main form.** Any pLCP solution gives the unique optimizer of the strictly convex QP. -/
theorem lcp_unique_optimizer {H : Matrix n n ℝ} (hH : H.PosDef) (A : Matrix m n ℝ) (b : m → ℝ)
    (c : n → ℝ) (S : Matrix m p ℝ) (F : Matrix n p ℝ) (θ : p → ℝ) {w lam : m → ℝ}
    (hs : LCPSol (lcpM H A) (lcpRhs H A b c S F θ) w lam) :
    IsQPMinimizer H (F *ᵥ θ + c) A (b + S *ᵥ θ) (primalOf H A F c θ lam) ∧
      ∀ y, IsQPMinimizer H (F *ᵥ θ + c) A (b + S *ᵥ θ) y → y = primalOf H A F c θ lam :=
  kkt_unique_minimizer (StrictlyConvex.of_posDef hH)
    (lcp_kkt ((isUnit_iff_isUnit_det H).mp hH.isUnit) A b c S F θ hs)

/-- **A1 (QP to pLCP), the algebraic half of necessity.** A KKT pair of the QP is a pLCP solution
with `w = b + Sθ - A x`, and `x = x(λ)`. -/
theorem kkt_lcp {H : Matrix n n ℝ} (hH : IsUnit H.det) (A : Matrix m n ℝ) (b : m → ℝ)
    (c : n → ℝ) (S : Matrix m p ℝ) (F : Matrix n p ℝ) (θ : p → ℝ) {x : n → ℝ} {lam : m → ℝ}
    (h : QPKKT H (F *ᵥ θ + c) A (b + S *ᵥ θ) x lam) :
    x = primalOf H A F c θ lam ∧
      LCPSol (lcpM H A) (lcpRhs H A b c S F θ) (b + S *ᵥ θ - A *ᵥ x) lam := by
  have hx : x = primalOf H A F c θ lam := by
    have hst := h.stationarity
    have : H *ᵥ x = -(F *ᵥ θ + c + Aᵀ *ᵥ lam) := by linear_combination hst
    have := congrArg (fun v => H⁻¹ *ᵥ v) this
    simp only [mulVec_mulVec, nonsing_inv_mul H hH, one_mulVec] at this
    rw [this, primalOf, mulVec_neg]
  refine ⟨hx, ⟨?_, fun i => ?_, h.dual_feasible, fun i => ?_⟩⟩
  · rw [hx, slack_primalOf]
  · simp only [Pi.sub_apply]; linarith [h.primal_feasible i]
  · rw [mul_comm]; exact h.complementary i

/-- **A1 (iff), conditional on KKT necessity.** Under the explicit hypothesis `KKTNecessity`,
`x` is the optimizer exactly when it is `x(λ)` for a pLCP solution `λ`. -/
theorem optimizer_iff_lcp {H : Matrix n n ℝ} (hH : H.PosDef) (A : Matrix m n ℝ) (b : m → ℝ)
    (c : n → ℝ) (S : Matrix m p ℝ) (F : Matrix n p ℝ) (θ : p → ℝ)
    (hnec : KKTNecessity H (F *ᵥ θ + c) A (b + S *ᵥ θ)) (x : n → ℝ) :
    IsQPMinimizer H (F *ᵥ θ + c) A (b + S *ᵥ θ) x ↔
      ∃ w lam, LCPSol (lcpM H A) (lcpRhs H A b c S F θ) w lam ∧ x = primalOf H A F c θ lam := by
  have hdet : IsUnit H.det := (isUnit_iff_isUnit_det H).mp hH.isUnit
  constructor
  · intro hx
    obtain ⟨lam, hk⟩ := hnec x hx
    obtain ⟨h1, h2⟩ := kkt_lcp hdet A b c S F θ hk
    exact ⟨_, lam, h2, h1⟩
  · rintro ⟨w, lam, hs, rfl⟩
    exact (lcp_unique_optimizer hH A b c S F θ hs).1

/-- `M = A H⁻¹ Aᵀ` is positive semidefinite when `H ≻ 0`. -/
theorem lcpM_posSemidef {H : Matrix n n ℝ} (hH : H.PosDef) (A : Matrix m n ℝ) :
    (lcpM H A).PosSemidef := by
  have := hH.inv.posSemidef.mul_mul_conjTranspose_same A
  rwa [conjTranspose_eq_transpose_of_trivial] at this

/-- **Uniqueness of `w`.** For a symmetric positive semidefinite `M`, all solutions of an LCP have
the same `w` (the multipliers `z` need not be unique: dependent active rows). -/
theorem lcp_w_unique {M : Matrix m m ℝ} (hM : M.PosSemidef) {r w₁ z₁ w₂ z₂ : m → ℝ}
    (h₁ : LCPSol M r w₁ z₁) (h₂ : LCPSol M r w₂ z₂) : w₁ = w₂ := by
  set u := z₁ - z₂
  have hdiff : w₁ - w₂ = M *ᵥ u := by
    rw [h₁.eq, h₂.eq, mulVec_sub]; abel
  have hle : u ⬝ᵥ (w₁ - w₂) ≤ 0 := by
    have e : u ⬝ᵥ (w₁ - w₂) = ∑ i, (z₁ i * w₁ i + z₂ i * w₂ i - z₁ i * w₂ i - z₂ i * w₁ i) := by
      simp only [u, dotProduct, Pi.sub_apply]
      exact Finset.sum_congr rfl fun i _ => by ring
    rw [e]
    refine Finset.sum_nonpos fun i _ => ?_
    have a := h₁.comp i
    have b := h₂.comp i
    have c1 := mul_nonneg (h₁.z_nonneg i) (h₂.w_nonneg i)
    have c2 := mul_nonneg (h₂.z_nonneg i) (h₁.w_nonneg i)
    nlinarith
  have hge : 0 ≤ u ⬝ᵥ (M *ᵥ u) := by simpa using hM.dotProduct_mulVec_nonneg u
  have hz : u ⬝ᵥ (M *ᵥ u) = 0 := by rw [← hdiff] at hge ⊢; linarith
  have hMu : M *ᵥ u = 0 := by
    have := (hM.dotProduct_mulVec_zero_iff (x := u)).mp (by simpa using hz)
    exact this
  rw [hMu] at hdiff
  exact sub_eq_zero.mp hdiff

end Rpopt
