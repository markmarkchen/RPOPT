/-
The scale-free pivot measure of mplcp (`pivot_measure`, `MEASURE_ZERO`, `MEASURE_NONZERO`).

For a basis `A` (z basic on `A`) of `w = r + M z` with `M` symmetric positive semidefinite, the block
of the tableau on the slack rows and the `z` columns outside `A` (the Schur complement of `M_AA`) is
itself symmetric positive semidefinite. For a vanishing slack row `c` with pivot `T c c`:

* `tableau_off_psd`, `tableau_off_symm`: `uᵀ T u ≥ 0` and `T k c = T c k` off `A`.
* `pivot_nonneg`: `T c c ≥ 0`. With `γ_c < 0`, `tve = -T c c / γ_c ≥ 0`: the sign of a nonzero pivot
  is always the one `Case1.piv_pos` needs (α rises); the code's `tve > 0` only guards rounding.
* `pivot_cauchy_schwarz`, `tableau_diag_le`, `column_le_pivot`: `T k c ² ≤ T k k · T c c ≤ M k k · T c c`.
  In the code's normalisation, (slack-column part of the measure)² ≤ pivot part: a slack-column
  entry above `1e-6` certifies a pivot of at least `1e-12` relative.
* `slack_column_zero_of_pivot_zero`: a zero pivot forces a zero slack column, the `col_c` hypothesis
  of `DepFacet` off `A`. So "measure = 0" (both parts) is exactly the zero-pivot condition.
-/
import RpoptProofs.Tableau

open Matrix

set_option linter.unusedSectionVars false

namespace Rpopt

variable {ι : Type*} [Fintype ι] [DecidableEq ι]

section Offblock

variable {M : Matrix ι ι ℝ} {r : ι → ℝ} {A : Finset ι} {ρ : ι → ℝ} {T : Matrix ι ι ℝ}

/-- Change of the multipliers when the nonbasic `z` outside `A` move by `u` (the slacks of `A` stay
0): `z_A` moves by `(T u)_A`, `z_N` by `u`. -/
def dz (A : Finset ι) (T : Matrix ι ι ℝ) (u : ι → ℝ) : ι → ℝ :=
  fun i => if i ∈ A then (T *ᵥ u) i else u i

/-- The matching change of the slacks: 0 on `A`, `(T u)_N` outside. -/
def dw (A : Finset ι) (T : Matrix ι ι ℝ) (u : ι → ℝ) : ι → ℝ :=
  fun i => if i ∈ A then 0 else (T *ᵥ u) i

theorem tableau_delta (h : IsTableau M r A ρ T) (u : ι → ℝ) (hu : ∀ i ∈ A, u i = 0) :
    dw A T u = M *ᵥ dz A T u := by
  have s1 : wOf A (ρ + T *ᵥ u) u = r + M *ᵥ zOf A (ρ + T *ᵥ u) u := (h _ _).mpr (by simp)
  have s0 : wOf A ρ 0 = r + M *ᵥ zOf A ρ 0 := (h _ _).mpr (by simp)
  have ew : dw A T u = wOf A (ρ + T *ᵥ u) u - wOf A ρ 0 := by
    funext i; by_cases hi : i ∈ A <;> simp [dw, wOf, hi, hu]
  have ez : dz A T u = zOf A (ρ + T *ᵥ u) u - zOf A ρ 0 := by
    funext i; by_cases hi : i ∈ A <;> simp [dz, zOf, hi]
  rw [ew, ez, mulVec_sub, s1, s0]
  abel

theorem tableau_bilin (h : IsTableau M r A ρ T) {u v : ι → ℝ} (hu : ∀ i ∈ A, u i = 0)
    (hv : ∀ i ∈ A, v i = 0) : v ⬝ᵥ (T *ᵥ u) = dz A T v ⬝ᵥ (M *ᵥ dz A T u) := by
  rw [← tableau_delta h u hu]
  simp only [dotProduct]
  refine Finset.sum_congr rfl fun i _ => ?_
  by_cases hi : i ∈ A <;> simp [dz, dw, hi, hv]

theorem symm_of_posSemidef (hM : M.PosSemidef) (x y : ι → ℝ) :
    x ⬝ᵥ (M *ᵥ y) = y ⬝ᵥ (M *ᵥ x) := by
  have hs : Mᵀ = M := by
    have := hM.isHermitian
    rwa [IsHermitian, conjTranspose_eq_transpose_of_trivial] at this
  rw [dotProduct_mulVec, ← mulVec_transpose, hs, dotProduct_comm]

/-- The off-`A` block of the tableau is positive semidefinite. -/
theorem tableau_off_psd (h : IsTableau M r A ρ T) (hM : M.PosSemidef) (u : ι → ℝ)
    (hu : ∀ i ∈ A, u i = 0) : 0 ≤ u ⬝ᵥ (T *ᵥ u) := by
  rw [tableau_bilin h hu hu]
  simpa using hM.dotProduct_mulVec_nonneg (dz A T u)

theorem single_T (T : Matrix ι ι ℝ) (a b : ι) :
    Pi.single a (1 : ℝ) ⬝ᵥ (T *ᵥ Pi.single b 1) = T a b := by
  simp [dotProduct, mulVec, Pi.single_apply]

/-- The off-`A` block of the tableau is symmetric. -/
theorem tableau_off_symm (h : IsTableau M r A ρ T) (hM : M.PosSemidef) {k c : ι} (hk : k ∉ A)
    (hc : c ∉ A) : T k c = T c k := by
  have hek : ∀ i ∈ A, (Pi.single k (1 : ℝ) : ι → ℝ) i = 0 := fun i hi => by
    have hik : i ≠ k := fun e => hk (e ▸ hi)
    simp [Pi.single_apply, hik]
  have hec : ∀ i ∈ A, (Pi.single c (1 : ℝ) : ι → ℝ) i = 0 := fun i hi => by
    have hic : i ≠ c := fun e => hc (e ▸ hi)
    simp [Pi.single_apply, hic]
  rw [← single_T T k c, ← single_T T c k, tableau_bilin h hec hek, tableau_bilin h hek hec,
    symm_of_posSemidef hM]

/-- **The pivot of a vanishing slack row is never negative.** -/
theorem pivot_nonneg (h : IsTableau M r A ρ T) (hM : M.PosSemidef) {c : ι} (hc : c ∉ A) :
    0 ≤ T c c := by
  have hec : ∀ i ∈ A, (Pi.single c (1 : ℝ) : ι → ℝ) i = 0 := fun i hi => by
    have hic : i ≠ c := fun e => hc (e ▸ hi)
    simp [Pi.single_apply, hic]
  have := tableau_off_psd h hM _ hec
  rwa [single_T] at this

/-- The quadratic form along `e_k + t e_c`. -/
theorem quad_line (T : Matrix ι ι ℝ) (k c : ι) (t : ℝ) :
    (Pi.single k (1 : ℝ) + t • Pi.single c 1) ⬝ᵥ (T *ᵥ (Pi.single k 1 + t • Pi.single c 1)) =
      T k k + t * T k c + t * T c k + t * t * T c c := by
  simp only [mulVec_add, mulVec_smul, dotProduct_add, add_dotProduct, dotProduct_smul,
    smul_dotProduct, single_T, smul_eq_mul]
  ring

/-- **Cauchy-Schwarz for the pivot.** `T k c ² ≤ T k k · T c c` for slack rows `k, c ∉ A`. -/
theorem pivot_cauchy_schwarz (h : IsTableau M r A ρ T) (hM : M.PosSemidef) {k c : ι}
    (hk : k ∉ A) (hc : c ∉ A) : T k c ^ 2 ≤ T k k * T c c := by
  have hline : ∀ t : ℝ, 0 ≤ T k k + 2 * t * T k c + t * t * T c c := by
    intro t
    have hu : ∀ i ∈ A, (Pi.single k (1 : ℝ) + t • Pi.single c 1 : ι → ℝ) i = 0 := by
      intro i hi
      have h1 : i ≠ k := fun e => hk (e ▸ hi)
      have h2 : i ≠ c := fun e => hc (e ▸ hi)
      simp [Pi.single_apply, h1, h2]
    have := tableau_off_psd h hM _ hu
    rw [quad_line, ← tableau_off_symm h hM hk hc] at this
    linarith
  have hcc := pivot_nonneg h hM hc
  rcases hcc.lt_or_eq with hpos | hzero
  · have := hline (-T k c / T c c)
    have e : T k k + 2 * (-T k c / T c c) * T k c + (-T k c / T c c) * (-T k c / T c c) * T c c =
        T k k - T k c ^ 2 / T c c := by field_simp; ring
    rw [e] at this
    have : T k c ^ 2 / T c c ≤ T k k := by linarith
    rwa [div_le_iff₀ hpos] at this
  · rw [← hzero, mul_zero]
    by_contra hne
    push_neg at hne
    have hkc : T k c ≠ 0 := fun e => by rw [e] at hne; simp at hne
    have := hline (-(T k k + 1) / (2 * T k c))
    rw [← hzero] at this
    have e : T k k + 2 * (-(T k k + 1) / (2 * T k c)) * T k c +
        (-(T k k + 1) / (2 * T k c)) * (-(T k k + 1) / (2 * T k c)) * 0 = -1 := by
      field_simp; ring
    linarith

/-- **A zero pivot forces a zero slack column** (the `col_c` hypothesis of `DepFacet` off `A`). -/
theorem slack_column_zero_of_pivot_zero (h : IsTableau M r A ρ T) (hM : M.PosSemidef) {c : ι}
    (hc : c ∉ A) (h0 : T c c = 0) : ∀ k, k ∉ A → T k c = 0 := by
  intro k hk
  have := pivot_cauchy_schwarz h hM hk hc
  rw [h0, mul_zero] at this
  exact pow_eq_zero_iff (n := 2) (by norm_num) |>.mp (le_antisymm this (sq_nonneg _))

/-- The slack diagonal of the tableau is at most that of `M` (a Schur complement). -/
theorem tableau_diag_le (h : IsTableau M r A ρ T) (hM : M.PosSemidef) {k : ι} (hk : k ∉ A) :
    T k k ≤ M k k := by
  set e : ι → ℝ := Pi.single k 1
  have he : ∀ i ∈ A, e i = 0 := fun i hi => by
    have hik : i ≠ k := fun h' => hk (h' ▸ hi)
    simp [e, Pi.single_apply, hik]
  set v : ι → ℝ := fun i => if i ∈ A then (T *ᵥ e) i else 0
  have hdec : e = dz A T e - v := by
    funext i; by_cases hi : i ∈ A <;> simp [dz, v, hi, he i]
  have hTkk : T k k = dz A T e ⬝ᵥ (M *ᵥ dz A T e) := by
    rw [← single_T T k k, tableau_bilin h he he]
  have hvdw : v ⬝ᵥ (M *ᵥ dz A T e) = 0 := by
    rw [← tableau_delta h e he]
    simp only [dotProduct]
    exact Finset.sum_eq_zero fun i _ => by by_cases hi : i ∈ A <;> simp [v, dw, hi]
  have hMkk : M k k = e ⬝ᵥ (M *ᵥ e) := by simp [e, dotProduct, mulVec, Pi.single_apply]
  have hvv := hM.dotProduct_mulVec_nonneg v
  simp only [star_trivial] at hvv
  rw [hMkk, hTkk]
  conv_rhs => rw [hdec]
  rw [mulVec_sub, dotProduct_sub, sub_dotProduct, sub_dotProduct]
  have hs := symm_of_posSemidef hM (dz A T e) v
  linarith

/-- **The slack-column part of the measure is bounded by the pivot part.** `T k c ² ≤ M k k · T c c`,
that is `(T k c / √(M_kk M_cc))² ≤ T c c / M_cc`: a slack-column entry above `1e-6` certifies a
pivot of at least `1e-12` relative. -/
theorem column_le_pivot (h : IsTableau M r A ρ T) (hM : M.PosSemidef) {k c : ι} (hk : k ∉ A)
    (hc : c ∉ A) : T k c ^ 2 ≤ M k k * T c c := by
  have h1 := pivot_cauchy_schwarz h hM hk hc
  have h2 := tableau_diag_le h hM hk
  have h3 := pivot_nonneg h hM hc
  nlinarith

end Offblock

end Rpopt
