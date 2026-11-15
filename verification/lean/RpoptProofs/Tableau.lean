/-
Complementary bases and their tableaux (Jones and Morari 2006; mplcp.rs `Tab`, `tableau_for`).

The pLCP is `w = r + M z` (`r = q + Qθ`). A complementary basis is given by the set `B` of pairs
whose multiplier `z_i` is basic (Rust: the `z` rows of `z_rows`); for `i ∉ B` the slack `w_i` is
basic. Rows and columns of a principal-pivot tableau are both indexed by the pairs, as in the
Rust code (row i and column i always hold the two variables of pair i, because `tableau_for` and
`Nbr::Toggle` only make principal pivots).

A tableau `x_B = ρ + T y_N` of `B` is defined *semantically*: it has exactly the same solution set
as `w = r + M z`. Everything else (basic solution, region law, uniqueness) follows from that.
-/
import RpoptProofs.Foundations.LCP

open Matrix

set_option linter.unusedSectionVars false

namespace Rpopt

variable {ι : Type*} [Fintype ι] [DecidableEq ι]

/-- Basic variables of the complementary basis `B`: `z_i` for `i ∈ B`, `w_i` otherwise. -/
def xB (B : Finset ι) (w z : ι → ℝ) : ι → ℝ := fun i => if i ∈ B then z i else w i

/-- Nonbasic variables of `B`: `w_i` for `i ∈ B`, `z_i` otherwise. -/
def yN (B : Finset ι) (w z : ι → ℝ) : ι → ℝ := fun i => if i ∈ B then w i else z i

/-- The slacks `w` from basic values `x` and nonbasic values `y`. -/
def wOf (B : Finset ι) (x y : ι → ℝ) : ι → ℝ := fun i => if i ∈ B then y i else x i

/-- The multipliers `z` from basic values `x` and nonbasic values `y`. -/
def zOf (B : Finset ι) (x y : ι → ℝ) : ι → ℝ := fun i => if i ∈ B then x i else y i

@[simp] theorem xB_wOf_zOf (B : Finset ι) (x y : ι → ℝ) : xB B (wOf B x y) (zOf B x y) = x := by
  funext i; by_cases h : i ∈ B <;> simp [xB, wOf, zOf, h]

@[simp] theorem yN_wOf_zOf (B : Finset ι) (x y : ι → ℝ) : yN B (wOf B x y) (zOf B x y) = y := by
  funext i; by_cases h : i ∈ B <;> simp [yN, wOf, zOf, h]

/-- `(ρ, T)` is a tableau of the complementary basis `B` for `w = r + M z`: both systems have the
same solutions `(w, z)`. -/
def IsTableau (M : Matrix ι ι ℝ) (r : ι → ℝ) (B : Finset ι) (ρ : ι → ℝ) (T : Matrix ι ι ℝ) :
    Prop :=
  ∀ w z : ι → ℝ, w = r + M *ᵥ z ↔ xB B w z = ρ + T *ᵥ yN B w z

/-- The initial tableau (all slacks basic): rows `[q | R]`, `T = M` (mplcp.rs `tableau_for`). -/
theorem isTableau_empty (M : Matrix ι ι ℝ) (r : ι → ℝ) : IsTableau M r ∅ r M := by
  intro w z
  have h1 : xB ∅ w z = w := by funext i; simp [xB]
  have h2 : yN ∅ w z = z := by funext i; simp [yN]
  rw [h1, h2]

/-- Slack values of the basic solution of `(B, ρ)` (nonbasic variables at zero). -/
def basicW (B : Finset ι) (ρ : ι → ℝ) : ι → ℝ := wOf B ρ 0

/-- Multiplier values of the basic solution of `(B, ρ)`. -/
def basicZ (B : Finset ι) (ρ : ι → ℝ) : ι → ℝ := zOf B ρ 0

/-- The basic solution satisfies the system. -/
theorem basic_eq {M : Matrix ι ι ℝ} {r : ι → ℝ} {B : Finset ι} {ρ : ι → ℝ} {T : Matrix ι ι ℝ}
    (h : IsTableau M r B ρ T) : basicW B ρ = r + M *ᵥ basicZ B ρ := by
  rw [h]; simp [basicW, basicZ]

/-- **A2 (basis semantics).** If `ρ ≥ 0`, the basic solution solves the LCP. -/
theorem basic_solves {M : Matrix ι ι ℝ} {r : ι → ℝ} {B : Finset ι} {ρ : ι → ℝ}
    {T : Matrix ι ι ℝ} (h : IsTableau M r B ρ T) (hρ : ∀ i, 0 ≤ ρ i) :
    LCPSol M r (basicW B ρ) (basicZ B ρ) where
  eq := basic_eq h
  w_nonneg i := by unfold basicW wOf; split_ifs <;> simp [hρ i]
  z_nonneg i := by unfold basicZ zOf; split_ifs <;> simp [hρ i]
  comp i := by unfold basicW basicZ wOf zOf; split_ifs <;> simp

/-- **A2 (the tableau law is the KKT-solve law).** Any `z` with `z_i = 0` off `B` and
`(r + M z)_i = 0` on `B` (the linear system `M_BB z_B = -r_B` that a KKT solve or Cholesky
factorisation solves) equals the tableau rows on `B`; the slacks off `B` equal the other rows. -/
theorem law_eq_kkt {M : Matrix ι ι ℝ} {r : ι → ℝ} {B : Finset ι} {ρ : ι → ℝ}
    {T : Matrix ι ι ℝ} (h : IsTableau M r B ρ T) {z : ι → ℝ} (hz : ∀ i, i ∉ B → z i = 0)
    (hw : ∀ i, i ∈ B → (r + M *ᵥ z) i = 0) :
    (∀ i, i ∈ B → z i = ρ i) ∧ (∀ i, i ∉ B → (r + M *ᵥ z) i = ρ i) := by
  have hy : yN B (r + M *ᵥ z) z = 0 := by
    funext i; by_cases hi : i ∈ B <;> simp [yN, hi, hw, hz]
  have key := (h (r + M *ᵥ z) z).mp rfl
  rw [hy, mulVec_zero, add_zero] at key
  refine ⟨fun i hi => ?_, fun i hi => ?_⟩
  · have := congrFun key i; simpa [xB, hi] using this
  · have := congrFun key i; simpa [xB, hi] using this

/-- **Uniqueness of the tableau.** Two tableaux of the same basis and system are equal. This is
why a tableau obtained by a chain of pivots equals the one rebuilt from scratch (REFACTOR in
mplcp.rs), and why the law read from the parent pivot (`candidate_law`) equals the fresh one. -/
theorem tableau_unique {M : Matrix ι ι ℝ} {r : ι → ℝ} {B : Finset ι} {ρ ρ' : ι → ℝ}
    {T T' : Matrix ι ι ℝ} (h : IsTableau M r B ρ T) (h' : IsTableau M r B ρ' T') :
    ρ = ρ' ∧ T = T' := by
  have row : ∀ y : ι → ℝ, ρ + T *ᵥ y = ρ' + T' *ᵥ y := by
    intro y
    have e := (h (wOf B (ρ + T *ᵥ y) y) (zOf B (ρ + T *ᵥ y) y)).mpr (by simp)
    have := (h' _ _).mp e
    simpa using this
  have hρ : ρ = ρ' := by simpa using row 0
  refine ⟨hρ, ?_⟩
  funext i k
  have := congrFun (row (Pi.single k 1)) i
  simpa [hρ, mulVec_single] using this

/-- Critical region of a parametric tableau `ρ : Θ → (ι → ℝ)`: `{θ : ρ(θ) ≥ 0}`. -/
def CR {Θ : Type*} (ρ : Θ → ι → ℝ) : Set Θ := {θ | ∀ i, 0 ≤ ρ θ i}

/-- **A2 (region law, mp-QP form).** On the critical region of a basis, the region's affine law
`λ(θ) = basicZ B ρ(θ)`, `x(θ) = -H⁻¹(Fθ + c + Aᵀ λ(θ))` gives the unique optimizer of the QP. -/
theorem region_law_optimal {n m p : Type*} [Fintype n] [Fintype m] [Fintype p] [DecidableEq n]
    [DecidableEq m] {H : Matrix n n ℝ} (hH : H.PosDef) (A : Matrix m n ℝ) (b : m → ℝ) (c : n → ℝ)
    (S : Matrix m p ℝ) (F : Matrix n p ℝ) {B : Finset m} {ρ : (p → ℝ) → m → ℝ}
    {T : Matrix m m ℝ} (htab : ∀ θ, IsTableau (lcpM H A) (lcpRhs H A b c S F θ) B (ρ θ) T)
    {θ : p → ℝ} (hθ : θ ∈ CR ρ) :
    IsQPMinimizer H (F *ᵥ θ + c) A (b + S *ᵥ θ) (primalOf H A F c θ (basicZ B (ρ θ))) ∧
      ∀ y, IsQPMinimizer H (F *ᵥ θ + c) A (b + S *ᵥ θ) y →
        y = primalOf H A F c θ (basicZ B (ρ θ)) :=
  lcp_unique_optimizer hH A b c S F θ (basic_solves (htab θ) hθ)

end Rpopt
