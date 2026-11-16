/-
Index-first lookup of Case-1 neighbours (mplcp.rs `idx_settled`, `idx_region_settled`,
`utils::unique_hyperplane_flags`): the search of facet `i` of region `R(A)` is skipped when (H1) no other
row of the unreduced system lies on the facet's hyperplane and no row vanishes identically, and (H2)
`A xor {i}` is a built region.

* `affine_vanish_prop`: an affine function that vanishes on the hyperplane `{⟪g, θ⟫ + b = 0}` is a
  multiple of `⟪g, θ⟫ + b`. So the rows of the tableau that vanish identically on the facet are exactly
  the rows proportional to it, and (H1) says that the facet's own row `v` is the only one (`Z = {v}`).
* `pivot_ne_zero_of_toggle`, `toggle_pivot_pos`: if the toggled basis has a tableau too (H2: it is a
  region), the pivot element `T v v` is nonzero, and positive for a positive semidefinite `M`.
* `Case1.of_single_row`, `index_first_unique_neighbour`: so the facet is a Case-1 facet with `Z = {v}`, and
  near every relative-interior point the only lexico-feasible bases are `A` and `A xor {v}`.
-/
import RpoptProofs.Optimizations.ReverseCrossing
import RpoptProofs.PivotMeasure

open Matrix Filter Topology
open scoped RealInnerProductSpace

set_option linter.unusedSectionVars false

namespace Rpopt

section Affine

variable {E : Type*} [NormedAddCommGroup E] [InnerProductSpace ℝ E]

/-- **An affine function vanishing on a hyperplane is a multiple of it.** If `⟪g', θ⟫ + c = 0`
wherever `⟪g, θ⟫ + b = 0`, with `g ≠ 0`, then `⟪g', θ⟫ + c = γ (⟪g, θ⟫ + b)` for one `γ` and all `θ`. -/
theorem affine_vanish_prop {g g' : E} {b c : ℝ} (hg : g ≠ 0)
    (h : ∀ θ : E, ⟪g, θ⟫ + b = 0 → ⟪g', θ⟫ + c = 0) :
    ∃ γ : ℝ, ∀ θ : E, ⟪g', θ⟫ + c = γ * (⟪g, θ⟫ + b) := by
  have hgg : ⟪g, g⟫ ≠ 0 := inner_self_ne_zero.mpr hg
  obtain ⟨γ, hγ⟩ : ∃ γ : ℝ, γ = ⟪g, g'⟫ / ⟪g, g⟫ := ⟨_, rfl⟩
  obtain ⟨w, hw⟩ : ∃ w : E, w = g' - γ • g := ⟨_, rfl⟩
  have hgw : ⟪g, w⟫ = 0 := by
    rw [hw, inner_sub_right, real_inner_smul_right, hγ, div_mul_cancel₀ _ hgg, sub_self]
  obtain ⟨θ₀, hθ₀⟩ : ∃ θ₀ : E, θ₀ = (-b / ⟪g, g⟫) • g := ⟨_, rfl⟩
  have h0 : ⟪g, θ₀⟫ + b = 0 := by
    rw [hθ₀, real_inner_smul_right, div_mul_cancel₀ _ hgg]
    ring
  have h1 : ⟪g, θ₀ + w⟫ + b = 0 := by rw [inner_add_right, hgw, add_zero, h0]
  have e0 := h θ₀ h0
  have e1 := h (θ₀ + w) h1
  rw [inner_add_right] at e1
  have hgw' : ⟪g', w⟫ = 0 := by linarith
  have hww : ⟪w, w⟫ = 0 := by
    have hsplit : g' = w + γ • g := by rw [hw, sub_add_cancel]
    have : ⟪g', w⟫ = ⟪w, w⟫ + γ * ⟪g, w⟫ := by
      rw [hsplit, inner_add_left, real_inner_smul_left]
    rw [hgw, mul_zero, add_zero] at this
    rw [← this, hgw']
  have hw0 : w = 0 := inner_self_eq_zero.mp hww
  have hg' : g' = γ • g := by
    rw [hw0] at hw
    exact sub_eq_zero.mp hw.symm
  have hgθ₀ : ⟪g, θ₀⟫ = -b := by linarith
  have hc : c = γ * b := by
    rw [hg', real_inner_smul_left, hgθ₀] at e0
    linarith
  refine ⟨γ, fun θ => ?_⟩
  rw [hg', real_inner_smul_left, hc]
  ring

end Affine

section Pivot

variable {ι : Type*} [Fintype ι] [DecidableEq ι] {M : Matrix ι ι ℝ} {r : ι → ℝ} {B : Finset ι}
  {ρ ρ' : ι → ℝ} {T T' : Matrix ι ι ℝ} {v : ι}

/-- **A toggled basis with a tableau has a nonzero pivot.** If both `B` and `toggle B v` have
tableaux, then `T v v ≠ 0`: with a zero pivot the basic variable of pair `v` would not depend on the
variable that the toggle makes basic, so it could not be free in the toggled tableau. -/
theorem pivot_ne_zero_of_toggle (h : IsTableau M r B ρ T)
    (h' : IsTableau M r (toggle B v) ρ' T') : T v v ≠ 0 := by
  intro h0
  obtain ⟨y', hy'⟩ : ∃ y' : ι → ℝ, y' = Function.update (0 : ι → ℝ) v (ρ v + 1) := ⟨_, rfl⟩
  obtain ⟨x', hx'⟩ : ∃ x' : ι → ℝ, x' = ρ' + T' *ᵥ y' := ⟨_, rfl⟩
  have hsol : wOf (toggle B v) x' y' = r + M *ᵥ zOf (toggle B v) x' y' :=
    (h' _ _).mpr (by rw [xB_wOf_zOf, yN_wOf_zOf, hx'])
  have hB := (h _ _).mp hsol
  have hyB' := yN_wOf_zOf (toggle B v) x' y'
  rw [yN_toggle] at hyB'
  have hxv : xB B (wOf (toggle B v) x' y') (zOf (toggle B v) x' y') v = ρ v + 1 := by
    have hy'v : y' v = ρ v + 1 := by rw [hy', Function.update_self]
    have := congrFun hyB' v
    rw [Function.update_self, hy'v] at this
    exact this
  have hyk : ∀ k, k ≠ v → yN B (wOf (toggle B v) x' y') (zOf (toggle B v) x' y') k = 0 := by
    intro k hk
    have hy'k : y' k = 0 := by rw [hy', Function.update_of_ne hk]; rfl
    have := congrFun hyB' k
    rw [Function.update_of_ne hk, hy'k] at this
    exact this
  have hsum : (T *ᵥ yN B (wOf (toggle B v) x' y') (zOf (toggle B v) x' y')) v = 0 := by
    simp only [mulVec, dotProduct]
    refine Finset.sum_eq_zero fun k _ => ?_
    by_cases hk : k = v
    · rw [hk, h0, zero_mul]
    · rw [hyk k hk, mul_zero]
  have := congrFun hB v
  rw [Pi.add_apply, hsum, add_zero, hxv] at this
  linarith

/-- **The pivot is positive** when `M` is positive semidefinite and both `B` and `toggle B v` have
tableaux (`pivot_nonneg` for a slack row; for a multiplier row, the same applied to the toggled basis,
whose pivot is the inverse). -/
theorem toggle_pivot_pos (hM : M.PosSemidef) (h : IsTableau M r B ρ T)
    (h' : IsTableau M r (toggle B v) ρ' T') : 0 < T v v := by
  have hne := pivot_ne_zero_of_toggle h h'
  by_cases hv : v ∈ B
  · have hv' : v ∉ toggle B v := fun hm => (mem_toggle_self.mp hm) hv
    have hT' : T' = pivT T v v := ((tableau_unique h' (pivot_isTableau h hne)).2)
    have h2 : IsTableau M r (toggle (toggle B v) v) ρ T := by rw [toggle_toggle]; exact h
    have hne' : T' v v ≠ 0 := pivot_ne_zero_of_toggle h' h2
    have hnn := pivot_nonneg h' hM hv'
    rw [hT', pivT_pivot] at hne' hnn
    exact inv_pos.mp (lt_of_le_of_ne hnn (Ne.symm hne'))
  · exact lt_of_le_of_ne (pivot_nonneg h hM hv) (Ne.symm hne)

end Pivot

namespace Case1

variable {Θ : Type*} {n : ℕ} {M : Matrix (Fin n) (Fin n) ℝ} {r : Θ → Fin n → ℝ}
  {B : Finset (Fin n)} {ρ : Θ → Fin n → ℝ} {T : Matrix (Fin n) (Fin n) ℝ} {h : Θ → ℝ}
  {v : Fin n} {γv : ℝ}

/-- **The index-first facet is a Case-1 facet.** If the facet's own row `v` is the only row that
vanishes identically on it (`Z = {v}`, hypothesis H1 with `affine_vanish_prop`), decreases across it, and
the toggled basis has a tableau (H2), the Case-1 hypotheses hold. -/
theorem of_single_row (hM : M.PosSemidef) (htab : ∀ θ, IsTableau M (r θ) B (ρ θ) T)
    (hv : ∀ θ, ρ θ v = γv * h θ) (hγ : γv < 0) {θ₁ : Θ} {ρ₁ : Fin n → ℝ}
    {T₁ : Matrix (Fin n) (Fin n) ℝ} (h' : IsTableau M (r θ₁) (toggle B v) ρ₁ T₁) :
    Case1 M r B ρ T h {v} (fun _ => γv) v where
  tab := htab
  v_mem := Finset.mem_singleton_self v
  zero_rows := by
    intro i hi θ
    rw [Finset.mem_singleton.mp hi]
    exact hv θ
  gam_v := hγ
  gam_others := fun _ hi hiv => absurd (Finset.mem_singleton.mp hi) hiv
  piv_pos := toggle_pivot_pos hM (htab θ₁) h'
  not_blocked := fun _ hi hiv => absurd (Finset.mem_singleton.mp hi) hiv

end Case1

variable {Θ : Type*} [TopologicalSpace Θ] {n : ℕ}

/-- **What the index-first skip relies on.** Under H1 (`Z = {v}`) and H2 (the toggled basis has a
tableau), near every relative-interior point of the facet, every lexico-feasible complementary basis is
`B` (on its side) or `toggle B v` (on the other). So the search of the facet could only return the
known region `toggle B v`. -/
theorem index_first_unique_neighbour {M : Matrix (Fin n) (Fin n) ℝ} (hM : M.PosSemidef)
    {r : Θ → Fin n → ℝ} {B : Finset (Fin n)} {ρ : Θ → Fin n → ℝ} {T : Matrix (Fin n) (Fin n) ℝ}
    {h : Θ → ℝ} {v : Fin n} {γv : ℝ} (htab : ∀ θ, IsTableau M (r θ) B (ρ θ) T)
    (hv : ∀ θ, ρ θ v = γv * h θ) (hγ : γv < 0) {θ₁ : Θ} {ρ₁ : Fin n → ℝ}
    {T₁ : Matrix (Fin n) (Fin n) ℝ} (h' : IsTableau M (r θ₁) (toggle B v) ρ₁ T₁)
    (hcont : ∀ i, Continuous fun θ => ρ θ i) {θ₀ : Θ} (h0 : h θ₀ = 0)
    (hrel : ∀ i, i ∉ ({v} : Finset (Fin n)) → 0 < ρ θ₀ i) {θ₂ : Θ}
    (hP : LexFeasible ((Case1.of_single_row hM htab hv hγ h').L θ₂))
    (hβv : Fin.tail ((Case1.of_single_row hM htab hv hγ h').L θ₀ v) ≠ 0) :
    ∀ᶠ θ in 𝓝 θ₀, ∀ (C : Finset (Fin n)) (LC : Fin n → Fin (n + 1) → ℝ)
      (TC : Matrix (Fin n) (Fin n) ℝ), IsLexTableau M (r θ) C LC TC → LexFeasible LC →
        (C = B ∨ C = toggle B v) ∧ (h θ < 0 → C = B) ∧ (0 < h θ → C = toggle B v) :=
  entry_facet_unique_neighbour hM (Case1.of_single_row hM htab hv hγ h') hcont h0 hrel
    (fun _ hi hiv => absurd (Finset.mem_singleton.mp hi) hiv) hP hβv

end Rpopt
