/-
B3. Uniqueness of the lexico-optimal basis (Jones, Kerrigan and Maciejowski 2007; mplp_lex.rs
header: "the perturbed program is primal and dual nondegenerate, so for every θ exactly one basis
is optimal").

The perturbed LP at `θ`: `q → q + ε⃗` (`ε⃗_j = ε^(j+1)`), `d → d + δ⃗` (`δ⃗_k = δ^(k+1)`).
A basis is *lexico-optimal* when its nonbasic slack rows `[q_j - A_j x₀ | e_j - A_j β E_B]` and
its inequality multiplier rows `[κ d | κ]` (`κ = -A_B^{-T}`) are lexico-positive. Here the rows
are abstract coefficient vectors of the slacks and multipliers as polynomials in `ε` and `δ`;
`lexOptimal_unique` shows that two lexico-optimal bases with the same number of rows coincide.

(`slack_sign` and `lam_row` in mplp_lex.rs compute these rows; their numeric columns at a facet
point carry one more infinitesimal, the step `γ` along the normal, which is an extra leading
coordinate and changes nothing below.)
-/
import RpoptProofs.MplpLex.Certificate
import RpoptProofs.Lex

open Matrix Filter Topology

set_option linter.unusedSectionVars false

namespace Rpopt

variable {m k : Type*} [Fintype m] [Fintype k] [DecidableEq m]

/-- A strictly nondegenerate optimal basis of one LP: the basic rows are active, the others have
positive slack, the multipliers vanish off `B` and are positive on its inequality rows. -/
structure NondegBasis (A : Matrix m k ℝ) (q : m → ℝ) (d : k → ℝ) (E : Set m) (B : Finset m)
    (x : k → ℝ) (lam : m → ℝ) : Prop where
  active : ∀ j ∈ B, (A *ᵥ x) j = q j
  slack_pos : ∀ j, j ∉ B → (A *ᵥ x) j < q j
  stat : d + Aᵀ *ᵥ lam = 0
  off : ∀ j, j ∉ B → lam j = 0
  mult_pos : ∀ j ∈ B, j ∉ E → 0 < lam j
  eq_in : ∀ j ∈ E, j ∈ B

namespace NondegBasis

variable {A : Matrix m k ℝ} {q : m → ℝ} {d : k → ℝ} {E : Set m} {B : Finset m} {x : k → ℝ}
  {lam : m → ℝ}

theorem feasible (h : NondegBasis A q d E B x lam) : LPFeasible A q E x := by
  intro j
  by_cases hj : j ∈ B
  · exact ⟨(h.active j hj).le, fun _ => h.active j hj⟩
  · exact ⟨(h.slack_pos j hj).le, fun hE => absurd (h.eq_in j hE) hj⟩

theorem dual (h : NondegBasis A q d E B x lam) : LPDualFeasible d A E lam where
  stat := h.stat
  sign j hj := by
    by_cases hB : j ∈ B
    · exact (h.mult_pos j hB hj).le
    · exact (h.off j hB).ge

theorem optimal (h : NondegBasis A q d E B x lam) :
    IsLPOptimal d A q E x ∧ d ⬝ᵥ x = -(lam ⬝ᵥ q) :=
  lp_certificate h.feasible h.dual fun j => by
    by_cases hj : j ∈ B
    · rw [h.active j hj, sub_self, mul_zero]
    · rw [h.off j hj, zero_mul]

end NondegBasis

/-- Two strictly nondegenerate optimal bases of the same LP, with the same number of rows, are
equal: the multipliers of one are complementary to the primal point of the other. -/
theorem nondeg_basis_unique {A : Matrix m k ℝ} {q : m → ℝ} {d : k → ℝ} {E : Set m}
    {B₁ B₂ : Finset m} {x₁ x₂ : k → ℝ} {lam₁ lam₂ : m → ℝ}
    (h₁ : NondegBasis A q d E B₁ x₁ lam₁) (h₂ : NondegBasis A q d E B₂ x₂ lam₂)
    (hcard : B₁.card = B₂.card) : B₁ = B₂ := by
  -- both optimal values agree
  obtain ⟨o₁, -⟩ := h₁.optimal
  obtain ⟨o₂, v₂⟩ := h₂.optimal
  have heq : d ⬝ᵥ x₁ = d ⬝ᵥ x₂ := le_antisymm (o₁.2 x₂ o₂.1) (o₂.2 x₁ o₁.1)
  have hcs := complementary_of_optimal h₁.feasible h₂.dual (heq.trans v₂)
  have hsub : B₂ ⊆ B₁ := by
    intro j hj
    by_cases hE : j ∈ E
    · exact h₁.eq_in j hE
    · by_contra hj1
      have hpos := h₂.mult_pos j hj hE
      have hsl := h₁.slack_pos j hj1
      have := hcs j
      have : q j - (A *ᵥ x₁) j = 0 := by
        rcases mul_eq_zero.mp this with h | h
        · exact absurd h hpos.ne'
        · exact h
      linarith
  exact (Finset.eq_of_subset_of_card_le hsub hcard.le).symm

variable {M K : ℕ}

/-- A lexico-optimal basis of the perturbed LP at a fixed `θ`: polynomial primal point `X ε`
and multipliers `Λ δ`, whose nonbasic slack rows `Sl j` and inequality multiplier rows `Lm j`
are lexico-positive. -/
structure LexOptimalBasis (A : Matrix (Fin M) (Fin K) ℝ) (q : Fin M → ℝ) (d : Fin K → ℝ)
    (E : Set (Fin M)) (B : Finset (Fin M)) : Prop where
  exists_data : ∃ (X : ℝ → Fin K → ℝ) (Λ : ℝ → Fin M → ℝ) (Sl : Fin M → Fin (M + 1) → ℝ)
      (Lm : Fin M → Fin (K + 1) → ℝ),
    (∀ ε, ∀ j ∈ B, (A *ᵥ X ε) j = q j + pw ε j) ∧
    (∀ ε, ∀ j, j ∉ B → q j + pw ε j - (A *ᵥ X ε) j = evalPoly (Sl j) ε) ∧
    (∀ j, j ∉ B → LexPos (Sl j)) ∧
    (∀ δ, (d + pw δ) + Aᵀ *ᵥ Λ δ = 0) ∧
    (∀ δ, ∀ j, j ∉ B → Λ δ j = 0) ∧
    (∀ δ, ∀ j ∈ B, j ∉ E → Λ δ j = evalPoly (Lm j) δ) ∧
    (∀ j ∈ B, j ∉ E → LexPos (Lm j)) ∧
    (∀ j ∈ E, j ∈ B)

/-- **B3 (lexicographic uniqueness, mplp_lex).** Two lexico-optimal bases of the perturbed LP at
the same `θ`, with the same number of rows (`n_x`), are equal. So the regions `R_B` of different
bases are disjoint: no overlaps. -/
theorem lexOptimal_unique {A : Matrix (Fin M) (Fin K) ℝ} {q : Fin M → ℝ} {d : Fin K → ℝ}
    {E : Set (Fin M)} {B₁ B₂ : Finset (Fin M)} (h₁ : LexOptimalBasis A q d E B₁)
    (h₂ : LexOptimalBasis A q d E B₂) (hcard : B₁.card = B₂.card) : B₁ = B₂ := by
  obtain ⟨X₁, Λ₁, S₁, L₁, a₁, s₁, sp₁, st₁, of₁, lm₁, lp₁, e₁⟩ := h₁.exists_data
  obtain ⟨X₂, Λ₂, S₂, L₂, a₂, s₂, sp₂, st₂, of₂, lm₂, lp₂, e₂⟩ := h₂.exists_data
  -- a common small ε making every nonbasic slack positive, and a common small δ for multipliers
  have hε : ∀ᶠ ε in 𝓝[>] (0 : ℝ), (∀ j, j ∉ B₁ → 0 < evalPoly (S₁ j) ε) ∧
      (∀ j, j ∉ B₂ → 0 < evalPoly (S₂ j) ε) := by
    refine (eventually_all.mpr fun j => ?_).and (eventually_all.mpr fun j => ?_)
    · by_cases hj : j ∈ B₁
      · exact Eventually.of_forall fun _ h => absurd hj h
      · filter_upwards [(sp₁ j hj).eventually_pos] with ε h _ using h
    · by_cases hj : j ∈ B₂
      · exact Eventually.of_forall fun _ h => absurd hj h
      · filter_upwards [(sp₂ j hj).eventually_pos] with ε h _ using h
  have hδ : ∀ᶠ δ in 𝓝[>] (0 : ℝ), (∀ j ∈ B₁, j ∉ E → 0 < evalPoly (L₁ j) δ) ∧
      (∀ j ∈ B₂, j ∉ E → 0 < evalPoly (L₂ j) δ) := by
    refine (eventually_all.mpr fun j => ?_).and (eventually_all.mpr fun j => ?_)
    · by_cases hj : j ∈ B₁ ∧ j ∉ E
      · filter_upwards [(lp₁ j hj.1 hj.2).eventually_pos] with δ h _ _ using h
      · exact Eventually.of_forall fun _ h1 h2 => absurd ⟨h1, h2⟩ hj
    · by_cases hj : j ∈ B₂ ∧ j ∉ E
      · filter_upwards [(lp₂ j hj.1 hj.2).eventually_pos] with δ h _ _ using h
      · exact Eventually.of_forall fun _ h1 h2 => absurd ⟨h1, h2⟩ hj
  obtain ⟨ε, hε₁, hε₂⟩ := hε.exists
  obtain ⟨δ, hδ₁, hδ₂⟩ := hδ.exists
  have mk : ∀ (B : Finset (Fin M)) (X : ℝ → Fin K → ℝ) (Λ : ℝ → Fin M → ℝ)
      (S : Fin M → Fin (M + 1) → ℝ) (L : Fin M → Fin (K + 1) → ℝ),
      (∀ j ∈ B, (A *ᵥ X ε) j = q j + pw ε j) →
      (∀ j, j ∉ B → q j + pw ε j - (A *ᵥ X ε) j = evalPoly (S j) ε) →
      (∀ j, j ∉ B → 0 < evalPoly (S j) ε) →
      ((d + pw δ) + Aᵀ *ᵥ Λ δ = 0) → (∀ j, j ∉ B → Λ δ j = 0) →
      (∀ j ∈ B, j ∉ E → Λ δ j = evalPoly (L j) δ) →
      (∀ j ∈ B, j ∉ E → 0 < evalPoly (L j) δ) → (∀ j ∈ E, j ∈ B) →
      NondegBasis A (q + pw ε) (d + pw δ) E B (X ε) (Λ δ) := by
    intro B X Λ S L ha hs hsp hst hof hlm hlp he
    exact { active := fun j hj => by simpa using ha j hj
            slack_pos := fun j hj => by
              have := hs j hj; have := hsp j hj
              simp only [Pi.add_apply]; linarith
            stat := hst
            off := hof
            mult_pos := fun j hj hE => by rw [hlm j hj hE]; exact hlp j hj hE
            eq_in := he }
  exact nondeg_basis_unique
    (mk B₁ X₁ Λ₁ S₁ L₁ (a₁ ε) (s₁ ε) hε₁ (st₁ δ) (of₁ δ) (lm₁ δ) hδ₁ e₁)
    (mk B₂ X₂ Λ₂ S₂ L₂ (a₂ ε) (s₂ ε) hε₂ (st₂ δ) (of₂ δ) (lm₂ δ) hδ₂ e₂) hcard

end Rpopt
