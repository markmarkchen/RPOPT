/-
Case-1 adjacency: the fast path of `adjacent` (mplcp.rs, "Fast path of Algorithm 2").

The region `CR ρ = {θ : ρ(θ) ≥ 0}` of the basis `B` has a facet on the hyperplane `h(θ) = 0`
(`h(θ) = g·θ + b`, `g` the unit outward normal of the region row, so the region is on `h ≤ 0`).
`Z` is the set of tableau rows that vanish identically on the hyperplane (`zrows`):
`ρ_i(θ) = γ_i h(θ)` with `γ_i = ρθ_i · g` (`gam`). The fast path fires when exactly one of them,
`v`, has `γ_v < 0` (`pv.len() == 1`), the pivot element is positive (`tve > PTOL`), and no other
row of `Z` is blocked. The neighbour is then the principal pivot on `v` (`Nbr::Toggle`).

All statements are in exact arithmetic (PTOL = ZTOL = 0).
-/
import RpoptProofs.Lex

open Matrix Filter Topology

set_option linter.unusedSectionVars false

namespace Rpopt

variable {Θ : Type*} {n : ℕ}

/-- The hypotheses of the Case-1 fast path in `adjacent` (exact arithmetic). -/
structure Case1 (M : Matrix (Fin n) (Fin n) ℝ) (r : Θ → Fin n → ℝ) (B : Finset (Fin n))
    (ρ : Θ → Fin n → ℝ) (T : Matrix (Fin n) (Fin n) ℝ) (h : Θ → ℝ) (Z : Finset (Fin n))
    (γ : Fin n → ℝ) (v : Fin n) : Prop where
  /-- `(ρ θ, T)` is a tableau of `B` at every `θ`. -/
  tab : ∀ θ, IsTableau M (r θ) B (ρ θ) T
  /-- `v` vanishes on the facet. -/
  v_mem : v ∈ Z
  /-- Rows of `Z` vanish identically on the hyperplane (`zrows`): `ρ_i = γ_i h`. -/
  zero_rows : ∀ i ∈ Z, ∀ θ, ρ θ i = γ i * h θ
  /-- `v` decreases across the facet (`gam[v] < -PTOL`). -/
  gam_v : γ v < 0
  /-- `v` is the only such row (`pv.len() == 1`). -/
  gam_others : ∀ i ∈ Z, i ≠ v → 0 ≤ γ i
  /-- Pivot sign condition (`tve = -T[v,kk]/gam[v] > PTOL`, i.e. `T v v > 0`). -/
  piv_pos : 0 < T v v
  /-- No other row of `Z` blocks (`!blocked`). -/
  not_blocked : ∀ i ∈ Z, i ≠ v → 0 ≤ T i v + γ i * (-(T v v) / γ v)

namespace Case1

variable {M : Matrix (Fin n) (Fin n) ℝ} {r : Θ → Fin n → ℝ} {B : Finset (Fin n)}
  {ρ : Θ → Fin n → ℝ} {T : Matrix (Fin n) (Fin n) ℝ} {h : Θ → ℝ} {Z : Finset (Fin n)}
  {γ : Fin n → ℝ} {v : Fin n}

/-- Rows of the toggled tableau. -/
noncomputable def ρ' (_ : Case1 M r B ρ T h Z γ v) : Θ → Fin n → ℝ :=
  fun θ => pivVec T (ρ θ) v v

theorem ρ'_apply (H : Case1 M r B ρ T h Z γ v) (θ : Θ) (i : Fin n) :
    H.ρ' θ i = if i = v then -(T v v)⁻¹ * ρ θ v else ρ θ i - T i v / T v v * ρ θ v := by
  simp [ρ', pivVec, smul_eq_mul]

/-- **A4 (the neighbour's tableau).** The principal pivot on `v` is a tableau of the toggled basis
at every `θ`. -/
theorem isTableau' (H : Case1 M r B ρ T h Z γ v) (θ : Θ) :
    IsTableau M (r θ) (toggle B v) (H.ρ' θ) (pivT T v v) :=
  pivot_isTableau (H.tab θ) H.piv_pos.ne'

theorem ρ'_v (H : Case1 M r B ρ T h Z γ v) (θ : Θ) :
    H.ρ' θ v = (-(T v v)⁻¹ * γ v) * h θ := by
  rw [H.ρ'_apply, if_pos rfl, H.zero_rows v H.v_mem]; ring

theorem coef_v_pos (H : Case1 M r B ρ T h Z γ v) : 0 < -(T v v)⁻¹ * γ v := by
  have := inv_pos.mpr H.piv_pos
  nlinarith [H.gam_v]

/-- **A4 (sides).** The region of `B` lies on `h ≤ 0` ... -/
theorem cr_le (H : Case1 M r B ρ T h Z γ v) {θ : Θ} (hθ : θ ∈ CR ρ) : h θ ≤ 0 := by
  have := hθ v
  rw [H.zero_rows v H.v_mem] at this
  by_contra hc
  push_neg at hc
  nlinarith [H.gam_v]

/-- ... and the region of the toggled basis on `h ≥ 0`. -/
theorem cr'_ge (H : Case1 M r B ρ T h Z γ v) {θ : Θ} (hθ : θ ∈ CR H.ρ') : 0 ≤ h θ := by
  have := hθ v
  rw [H.ρ'_v] at this
  by_contra hc
  push_neg at hc
  nlinarith [H.coef_v_pos]

/-- **A4 (the shared facet is the same set from both sides).** On the hyperplane the two
tableaux have the same values, so `CR_B ∩ H = CR_B' ∩ H`. -/
theorem facet_eq (H : Case1 M r B ρ T h Z γ v) :
    CR ρ ∩ {θ | h θ = 0} = CR H.ρ' ∩ {θ | h θ = 0} := by
  ext θ
  simp only [Set.mem_inter_iff, Set.mem_setOf_eq]
  constructor <;> rintro ⟨h1, h2⟩ <;> refine ⟨?_, h2⟩
  all_goals
    have hv : ρ θ v = 0 := by rw [H.zero_rows v H.v_mem, h2, mul_zero]
    have e : H.ρ' θ = ρ θ := pivVec_of_zero hv
  · rw [CR, Set.mem_setOf_eq, e]; exact h1
  · rw [CR, Set.mem_setOf_eq, ← e]; exact h1

/-- If the region has a point strictly inside its side, the rows of `Z` cannot increase across
the facet; with `gam_others` they are identically zero (degenerate rows). -/
theorem gam_eq_zero (H : Case1 M r B ρ T h Z γ v) {θ₁ : Θ} (h₁ : θ₁ ∈ CR ρ) (hneg : h θ₁ < 0)
    {i : Fin n} (hi : i ∈ Z) (hiv : i ≠ v) : γ i = 0 := by
  have := h₁ i
  rw [H.zero_rows i hi] at this
  have h0 := H.gam_others i hi hiv
  nlinarith

/-- Coefficient of `h` in a toggled row of `Z \ {v}`; nonnegative by `not_blocked`. -/
theorem coef_nonneg (H : Case1 M r B ρ T h Z γ v) {i : Fin n} (hi : i ∈ Z) (hiv : i ≠ v) :
    0 ≤ γ i - T i v / T v v * γ v := by
  have hnb := H.not_blocked i hi hiv
  have hp := H.piv_pos
  have hg := H.gam_v
  have hg0 : γ v ≠ 0 := hg.ne
  have e : γ i - T i v / T v v * γ v = (-γ v / T v v) * (T i v + γ i * (-(T v v) / γ v)) := by
    field_simp
    ring
  rw [e]
  exact mul_nonneg (div_nonneg (by linarith) hp.le) hnb

theorem ρ'_zero_row (H : Case1 M r B ρ T h Z γ v) {i : Fin n} (hi : i ∈ Z) (hiv : i ≠ v)
    (θ : Θ) : H.ρ' θ i = (γ i - T i v / T v v * γ v) * h θ := by
  rw [H.ρ'_apply, if_neg hiv, H.zero_rows i hi, H.zero_rows v H.v_mem]; ring

variable [TopologicalSpace Θ]

/-- Rows outside `Z` stay positive near a relative-interior point of the facet, in both tableaux. -/
theorem eventually_pos_off_Z (H : Case1 M r B ρ T h Z γ v)
    (hcont : ∀ i, Continuous fun θ => ρ θ i) {θ₀ : Θ} (h0 : h θ₀ = 0)
    (hrel : ∀ i, i ∉ Z → 0 < ρ θ₀ i) :
    ∀ᶠ θ in 𝓝 θ₀, ∀ i, i ∉ Z → 0 < ρ θ i ∧ 0 < H.ρ' θ i := by
  refine eventually_all.mpr fun i => ?_
  by_cases hi : i ∈ Z
  · exact Eventually.of_forall fun _ hi' => absurd hi hi'
  · have hiv : i ≠ v := fun e => hi (e ▸ H.v_mem)
    have hv0 : ρ θ₀ v = 0 := by rw [H.zero_rows v H.v_mem, h0, mul_zero]
    have c1 : ContinuousAt (fun θ => ρ θ i) θ₀ := (hcont i).continuousAt
    have c2 : ContinuousAt (fun θ => H.ρ' θ i) θ₀ := by
      have : (fun θ => H.ρ' θ i) = fun θ => ρ θ i - T i v / T v v * ρ θ v := by
        funext θ; rw [H.ρ'_apply, if_neg hiv]
      rw [this]
      exact ((hcont i).sub (continuous_const.mul (hcont v))).continuousAt
    have e2 : H.ρ' θ₀ i = ρ θ₀ i := by rw [H.ρ'_apply, if_neg hiv, hv0]; ring
    have p1 := c1.eventually (lt_mem_nhds (hrel i hi))
    have p2 := c2.eventually (lt_mem_nhds (show 0 < H.ρ' θ₀ i by rw [e2]; exact hrel i hi))
    filter_upwards [p1, p2] with θ a b using fun _ => ⟨a, b⟩

/-- **A4 (the region across the facet is the toggled basis's region).** Near every
relative-interior point `θ₀` of the facet, every point on the far side `h ≥ 0` lies in the
toggled basis's critical region; so that region is full-dimensional and adjacent across the
facet. -/
theorem across (H : Case1 M r B ρ T h Z γ v) (hcont : ∀ i, Continuous fun θ => ρ θ i)
    {θ₀ : Θ} (h0 : h θ₀ = 0) (hrel : ∀ i, i ∉ Z → 0 < ρ θ₀ i) :
    ∀ᶠ θ in 𝓝 θ₀, 0 ≤ h θ → θ ∈ CR H.ρ' := by
  filter_upwards [H.eventually_pos_off_Z hcont h0 hrel] with θ hθ hh i
  by_cases hi : i ∈ Z
  · by_cases hiv : i = v
    · subst hiv; rw [H.ρ'_v]; exact mul_nonneg H.coef_v_pos.le hh
    · rw [H.ρ'_zero_row hi hiv]; exact mul_nonneg (H.coef_nonneg hi hiv) hh
  · exact (hθ i hi).2.le

/-- **A4 (own side).** If the rows of `Z` do not increase across the facet (true when the region
has a point with `h < 0`, see `gam_eq_zero`), the region of `B` contains the near side. -/
theorem own_side (H : Case1 M r B ρ T h Z γ v) (hcont : ∀ i, Continuous fun θ => ρ θ i)
    {θ₀ : Θ} (h0 : h θ₀ = 0) (hrel : ∀ i, i ∉ Z → 0 < ρ θ₀ i) (hγ : ∀ i ∈ Z, γ i ≤ 0) :
    ∀ᶠ θ in 𝓝 θ₀, h θ ≤ 0 → θ ∈ CR ρ := by
  filter_upwards [H.eventually_pos_off_Z hcont h0 hrel] with θ hθ hh i
  by_cases hi : i ∈ Z
  · rw [H.zero_rows i hi]; exact mul_nonneg_of_nonpos_of_nonpos (hγ i hi) hh
  · exact (hθ i hi).1.le

/-! ### Lexicographic version -/

/-- Lexicographic rows of `B` and of the toggled basis. -/
noncomputable def L (_ : Case1 M r B ρ T h Z γ v) : Θ → Fin n → Fin (n + 1) → ℝ :=
  fun θ => lexRows B (ρ θ) T

noncomputable def L' (H : Case1 M r B ρ T h Z γ v) : Θ → Fin n → Fin (n + 1) → ℝ :=
  fun θ => lexRows (toggle B v) (H.ρ' θ) (pivT T v v)

theorem L'_eq (H : Case1 M r B ρ T h Z γ v) (θ : Θ) : H.L' θ = pivVec T (H.L θ) v v :=
  lexRows_pivot H.piv_pos.ne'

theorem L_cons (H : Case1 M r B ρ T h Z γ v) (θ : Θ) (i : Fin n) :
    H.L θ i = Fin.cons (ρ θ i) (Fin.tail (H.L θ i)) := by
  simp [L, lexRows]

theorem L'_cons (H : Case1 M r B ρ T h Z γ v) (θ : Θ) (i : Fin n) :
    H.L' θ i = Fin.cons (H.ρ' θ i) (Fin.tail (H.L' θ i)) := by
  simp [L', lexRows]

/-- The β part of a row does not depend on `θ`. -/
theorem tail_L (H : Case1 M r B ρ T h Z γ v) (θ θ' : Θ) (i : Fin n) :
    Fin.tail (H.L θ i) = Fin.tail (H.L θ' i) := by
  simp [L, lexRows]

theorem lexPos_of_num_pos {u : Fin (n + 1) → ℝ} (hu : 0 < u 0) : LexPos u :=
  (lexPos_succ_iff u).mpr (Or.inl hu)

/-- **A4, lexicographic local cover.** Near every relative-interior point of the shared facet,
`B` is lexico-feasible on the near side, the toggled basis on the far side, and one of the two on
the facet itself. Hypotheses: `B` is lexico-feasible somewhere on its side, the identically zero
rows of `Z` are degenerate (`γ_i = 0`, see `gam_eq_zero`), and the β row of `v` is nonzero (true
for any basis, whose β block is the basis inverse). -/
theorem lex_cover (H : Case1 M r B ρ T h Z γ v) (hcont : ∀ i, Continuous fun θ => ρ θ i)
    {θ₀ : Θ} (h0 : h θ₀ = 0) (hrel : ∀ i, i ∉ Z → 0 < ρ θ₀ i)
    (hγ0 : ∀ i ∈ Z, i ≠ v → γ i = 0) {θ₁ : Θ} (hP : LexFeasible (H.L θ₁))
    (hβv : Fin.tail (H.L θ₀ v) ≠ 0) :
    ∀ᶠ θ in 𝓝 θ₀, (h θ < 0 → LexFeasible (H.L θ)) ∧ (0 < h θ → LexFeasible (H.L' θ)) ∧
      (LexFeasible (H.L θ) ∨ LexFeasible (H.L' θ)) := by
  -- β rows of the degenerate rows of Z are lexico-positive (B is lexico-feasible at θ₁)
  have hdeg : ∀ θ, ∀ i ∈ Z, i ≠ v → LexPos (H.L θ i) := by
    intro θ i hi hiv
    have h1 := hP i
    have hz : ∀ θ', ρ θ' i = 0 := fun θ' => by rw [H.zero_rows i hi, hγ0 i hi hiv, zero_mul]
    rw [H.L_cons θ₁ i, hz θ₁, lexPos_cons_zero] at h1
    rw [H.L_cons θ i, hz θ, lexPos_cons_zero, H.tail_L θ θ₁]
    exact h1
  -- the toggled degenerate rows: T i v ≥ 0
  have hTiv : ∀ i ∈ Z, i ≠ v → 0 ≤ T i v := by
    intro i hi hiv
    have := H.not_blocked i hi hiv
    rwa [hγ0 i hi hiv, zero_mul, add_zero] at this
  have hp := H.piv_pos
  filter_upwards [H.eventually_pos_off_Z hcont h0 hrel] with θ hθ
  -- B on the near side
  have nearB : h θ < 0 → LexFeasible (H.L θ) := by
    intro hh i
    by_cases hi : i ∈ Z
    · by_cases hiv : i = v
      · subst hiv
        refine lexPos_of_num_pos ?_
        simp only [L, lexRows, Fin.cons_zero]
        rw [H.zero_rows i hi]; nlinarith [H.gam_v]
      · exact hdeg θ i hi hiv
    · refine lexPos_of_num_pos ?_
      simp only [L, lexRows, Fin.cons_zero]; exact (hθ i hi).1
  -- the toggled basis where the row v of B is lexico-negative, or on the far side
  have farB' : (0 < h θ ∨ LexPos (-(H.L θ v))) → LexFeasible (H.L' θ) := by
    intro hcase i
    by_cases hi : i ∈ Z
    · by_cases hiv : i = v
      · subst hiv
        rcases hcase with hh | hneg
        · refine lexPos_of_num_pos ?_
          simp only [L', lexRows, Fin.cons_zero]
          rw [H.ρ'_v]; exact mul_pos H.coef_v_pos hh
        · rw [H.L'_eq]
          simp only [pivVec, if_true]
          rw [show (-(T i i)⁻¹) • H.L θ i = (T i i)⁻¹ • (-(H.L θ i)) by rw [smul_neg, neg_smul]]
          exact hneg.smul (inv_pos.mpr hp)
      · rw [H.L'_eq]
        simp only [pivVec, if_neg hiv]
        rw [sub_eq_add_neg, ← neg_smul]
        rcases (hTiv i hi hiv).lt_or_eq with hT | hT
        · rcases hcase with hh | hneg
          · -- numeric part positive
            have : H.L' θ i = pivVec T (H.L θ) v v i := by rw [H.L'_eq]
            rw [pivVec, if_neg hiv, sub_eq_add_neg, ← neg_smul] at this
            rw [← this]
            refine lexPos_of_num_pos ?_
            simp only [L', lexRows, Fin.cons_zero]
            rw [H.ρ'_zero_row hi hiv, hγ0 i hi hiv]
            have : 0 < (0 - T i v / T v v * γ v) := by
              have := div_pos hT hp
              nlinarith [H.gam_v]
            exact mul_pos this hh
          · rw [show -(T i v / T v v) • H.L θ v = (T i v / T v v) • (-(H.L θ v)) by
              rw [smul_neg, neg_smul]]
            exact (hdeg θ i hi hiv).add (Or.inr (hneg.smul (div_pos hT hp)))
        · rw [← hT, zero_div, neg_zero, zero_smul, add_zero]
          exact hdeg θ i hi hiv
    · refine lexPos_of_num_pos ?_
      simp only [L', lexRows, Fin.cons_zero]; exact (hθ i hi).2
  refine ⟨nearB, fun hh => farB' (Or.inl hh), ?_⟩
  rcases lt_trichotomy (h θ) 0 with hh | hh | hh
  · exact Or.inl (nearB hh)
  · -- on the facet: the sign of the β row of v decides
    have hv0 : ρ θ v = 0 := by rw [H.zero_rows v H.v_mem, hh, mul_zero]
    have hcons : H.L θ v = Fin.cons 0 (Fin.tail (H.L θ v)) := by
      conv_lhs => rw [H.L_cons θ v]
      rw [hv0]
    have hne : Fin.tail (H.L θ v) ≠ 0 := by rwa [H.tail_L θ θ₀]
    rcases lexPos_or_neg hne with hpos | hneg
    · left
      intro i
      by_cases hi : i ∈ Z
      · by_cases hiv : i = v
        · subst hiv; rw [hcons, lexPos_cons_zero]; exact hpos
        · exact hdeg θ i hi hiv
      · refine lexPos_of_num_pos ?_
        simp only [L, lexRows, Fin.cons_zero]; exact (hθ i hi).1
    · right
      refine farB' (Or.inr ?_)
      rw [hcons]
      have : -(Fin.cons 0 (Fin.tail (H.L θ v)) : Fin (n + 1) → ℝ) =
          Fin.cons 0 (-(Fin.tail (H.L θ v))) := by
        funext j; refine Fin.cases ?_ (fun j => ?_) j <;> simp
      rw [this, lexPos_cons_zero]
      exact hneg
  · exact Or.inr (farB' (Or.inl hh))

end Case1

end Rpopt
