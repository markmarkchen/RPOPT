/-
Weakly active constraints (algorithms/mod.rs `drop_weakly_active_duplicates`): when the multiplier of an
active constraint `i` vanishes identically on the region of `A`, the labels `A` and `A \ {i}` describe the
same region with the same law, and the solver keeps one of them.

* `weakly_active_same_rows`: if `A` and `A.erase i` both have tableaux and row `i` of `A`'s tableau is 0,
  the two tableaux have the same rows. At every parameter where this holds the two regions agree; if
  `λ_i` vanishes identically, they are the same set with the same law (`weakly_active_same_region`).
-/
import RpoptProofs.Tableau

open Matrix

set_option linter.unusedSectionVars false

namespace Rpopt

variable {ι : Type*} [Fintype ι] [DecidableEq ι] {M : Matrix ι ι ℝ} {r : ι → ℝ} {A : Finset ι}
  {ρ ρ' : ι → ℝ} {T T' : Matrix ι ι ℝ} {i : ι}

/-- **Same rows.** If `i ∈ A`, the tableau of `A` has `ρ i = 0` (the multiplier of `i` is 0) and
`A.erase i` has a tableau too, both tableaux have the same rows: the multipliers on `A.erase i`,
0 at `i` (a multiplier for `A`, a slack for `A.erase i`), and the slacks off `A`. -/
theorem weakly_active_same_rows (h : IsTableau M r A ρ T) (h' : IsTableau M r (A.erase i) ρ' T')
    (hi : i ∈ A) (h0 : ρ i = 0) : ρ' = ρ := by
  have hsol := basic_eq h
  obtain ⟨hz, hw⟩ := law_eq_kkt h' (z := basicZ A ρ)
    (fun j hj => by
      by_cases hjA : j ∈ A
      · have hji : j = i := by
          by_contra hne
          exact hj (Finset.mem_erase.mpr ⟨hne, hjA⟩)
        simp [basicZ, zOf, hji, h0]
      · simp [basicZ, zOf, hjA])
    (fun j hj => by
      rw [← hsol]
      have hjA : j ∈ A := Finset.mem_of_mem_erase hj
      simp [basicW, wOf, hjA])
  funext j
  by_cases hj : j ∈ A.erase i
  · have := hz j hj
    have hjA : j ∈ A := Finset.mem_of_mem_erase hj
    simp [basicZ, zOf, hjA] at this
    exact this.symm
  · have := hw j hj
    rw [← hsol] at this
    by_cases hjA : j ∈ A
    · have hji : j = i := by
        by_contra hne
        exact hj (Finset.mem_erase.mpr ⟨hne, hjA⟩)
      subst hji
      simp [basicW, wOf, hjA] at this
      rw [← this, h0]
    · simp [basicW, wOf, hjA] at this
      exact this.symm

/-- **Same region, same law.** If the multiplier of `i` vanishes at every parameter (an identically
zero row), the two labels have the same rows at every parameter, hence the same critical region
`{θ : ρ(θ) ≥ 0}` and the same law. -/
theorem weakly_active_same_region {Θ : Type*} {r : Θ → ι → ℝ} {ρ ρ' : Θ → ι → ℝ}
    (h : ∀ θ, IsTableau M (r θ) A (ρ θ) T) (h' : ∀ θ, IsTableau M (r θ) (A.erase i) (ρ' θ) T')
    (hi : i ∈ A) (h0 : ∀ θ, ρ θ i = 0) : ρ' = ρ ∧ CR ρ' = CR ρ := by
  have e : ρ' = ρ := funext fun θ => weakly_active_same_rows (h θ) (h' θ) hi (h0 θ)
  exact ⟨e, by rw [e]⟩

end Rpopt
