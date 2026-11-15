/-
The Case-2 lexicographic test of `pivot_facet` (Algorithm 2 of Jones and Morari 2006;
mplcp.rs, the loop `for &l in &pset` with the lexico-inequalities (16)).

For a candidate leaving row `l` and every other decreasing row `j` (`a_j = -T[j,e] > 0`),
`pivot_facet` forms `Γ_j = a_l row_j - a_j row_l` and, when its θ_f part is zero (`gq_zero`),
accepts `l` only if `[Γ_0 | Γ_β]` is lexico-nonnegative.

In exact arithmetic:
* `gamma_scale`: the test does not depend on the scale of `a`: `Γ = (a_l a_j) · (row_j/a_j - row_l/a_l)`.
* `case2_accept_unique`: two different candidates are both accepted only if one of their `Γ`
  vectors is exactly zero. For rows of a basis tableau this cannot happen (the β rows are rows of
  the basis inverse), so the accepted candidate is unique.

The implementation reads `Γ` through the absolute tolerance `ZTOL = 1e-9` (`gq_zero`, and
`lex_nonneg` at mplcp.rs:341-348). `Γ` scales with `a_l a_j`; on pop_001 the twin rows 4 and 23
have `a ≈ 1.2e-10`, so `Γ_β ≈ ±1.2e-10 < ZTOL` is read as the zero vector, the hypothesis
`Γ ≠ 0` of `case2_accept_unique` is violated numerically, and both twins are accepted: one of
them is the lexico-infeasible basis. The fix is a scale-free and relative test (`gamma_scale`
shows the scale-free vector gives the same exact answer).
-/
import RpoptProofs.Lex

namespace Rpopt

variable {N : ℕ}

theorem lexPos_smul_iff {u : Fin N → ℝ} {a : ℝ} (ha : 0 < a) : LexPos (a • u) ↔ LexPos u := by
  refine ⟨fun h => ?_, fun h => h.smul ha⟩
  have := h.smul (inv_pos.mpr ha)
  rwa [smul_smul, inv_mul_cancel₀ ha.ne', one_smul] at this

/-- The Case-2 test is scale-free in exact arithmetic. -/
theorem gamma_scale {rowl rowj : Fin N → ℝ} {al aj : ℝ} (hl : 0 < al) (hj : 0 < aj) :
    LexPos (al • rowj - aj • rowl) ↔ LexPos (aj⁻¹ • rowj - al⁻¹ • rowl) := by
  have e : al • rowj - aj • rowl = (al * aj) • (aj⁻¹ • rowj - al⁻¹ • rowl) := by
    rw [smul_sub, smul_smul, smul_smul]
    congr 2 <;> field_simp
  rw [e, lexPos_smul_iff (mul_pos hl hj)]

/-- The scale-free form, for the lexico-nonnegative test the code applies (`lex_nonneg_rel` on
`row_j/a_j - row_l/a_l`, 52f3103): same exact answer as on `Γ`. -/
theorem gamma_scale_nonneg {rowl rowj : Fin N → ℝ} {al aj : ℝ} (hl : 0 < al) (hj : 0 < aj) :
    LexNonneg (al • rowj - aj • rowl) ↔ LexNonneg (aj⁻¹ • rowj - al⁻¹ • rowl) := by
  have e : al • rowj - aj • rowl = (al * aj) • (aj⁻¹ • rowj - al⁻¹ • rowl) := by
    rw [smul_sub, smul_smul, smul_smul]
    congr 2 <;> field_simp
  have hc : 0 < al * aj := mul_pos hl hj
  rw [e]
  constructor
  · rintro (h | h)
    · left; exact (smul_eq_zero.mp h).resolve_left hc.ne'
    · right; exact (lexPos_smul_iff hc).mp h
  · rintro (h | h)
    · left; rw [h, smul_zero]
    · right; exact (lexPos_smul_iff hc).mpr h

/-- Both directions of the test pass exactly when `Γ = 0`. -/
theorem lexNonneg_both_iff {u : Fin N → ℝ} : LexNonneg u ∧ LexNonneg (-u) ↔ u = 0 := by
  constructor
  · rintro ⟨h1 | h1, h2 | h2⟩
    · exact h1
    · exact h1
    · exact absurd (neg_eq_zero.mp h2) h1.ne_zero
    · exact absurd h2 (not_lexPos_and_neg h1)
  · rintro rfl; exact ⟨Or.inl rfl, Or.inl (by simp)⟩

/-- `Γ` of candidate `l` against row `j`. -/
def gammaVec {ι : Type*} (row : ι → Fin N → ℝ) (a : ι → ℝ) (l j : ι) : Fin N → ℝ :=
  a l • row j - a j • row l

/-- A candidate passes the θ-independent part of the Case-2 test. -/
def Case2Accepts {ι : Type*} (P : Set ι) (row : ι → Fin N → ℝ) (a : ι → ℝ) (l : ι) : Prop :=
  ∀ j ∈ P, j ≠ l → LexNonneg (gammaVec row a l j)

/-- **Case 2 accepts at most one candidate** when no `Γ` vanishes. -/
theorem case2_accept_unique {ι : Type*} {P : Set ι} {row : ι → Fin N → ℝ} {a : ι → ℝ}
    (hne : ∀ l ∈ P, ∀ j ∈ P, l ≠ j → gammaVec row a l j ≠ 0) {l₁ l₂ : ι} (h₁ : l₁ ∈ P)
    (h₂ : l₂ ∈ P) (a₁ : Case2Accepts P row a l₁) (a₂ : Case2Accepts P row a l₂) : l₁ = l₂ := by
  by_contra hne12
  have g1 := a₁ l₂ h₂ (Ne.symm hne12)
  have g2 := a₂ l₁ h₁ hne12
  have e : gammaVec row a l₂ l₁ = -gammaVec row a l₁ l₂ := by
    simp only [gammaVec]; abel
  rw [e] at g2
  exact hne l₁ h₁ l₂ h₂ hne12 (lexNonneg_both_iff.mp ⟨g1, g2⟩)

end Rpopt
