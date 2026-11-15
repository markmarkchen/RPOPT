/-
Lexicographic enumeration for mpLPs with H = 0 (mplp_lex.rs `enumerate_regions`,
`dual_lex_feasible_bases`, `candidate_ok`; argument in bench/results/mplp_lex_enumeration.md).

A basis `B` has nonbasic slack rows that are affine in θ, `s_j(θ) = s0_j + sθ_j · θ`, with a
θ-independent ε-row `σ_j = e_j - T_j E_B`. For H = 0 the multipliers do not depend on θ, so the dual
lexicographic condition (D) is a property of the basis alone (`dual : Prop`).

* Step 1 (`LexOpt`): `B` is lexicographically optimal at θ iff (D) holds and every nonbasic slack row
  `[s_j(θ) | σ_j]` is lexico-positive. This is the definition used by `LexOptimalBasis`; `lexOpt_unique`
  connects the two.
* Step 2 (`primalLex_iff_of_interior`): in the interior of the closed region, every slack is positive
  or identically zero, so `B` is primal lexico-feasible there iff (Z) holds: every identically zero slack
  has a lexico-positive ε-row.
* Step 3 (`fullDim_iff`, `interior_region_eq`): the lexicographic region `R_B` is full-dimensional iff
  (D), (Z) and the closed region is full-dimensional, and then both have the same interior.
* No overlaps (`enum_interiors_disjoint`): two different bases passing (D) and (Z) have closed regions
  with disjoint interiors (from `lexOptimal_unique`). Without (Z) this fails: a basis with a
  lexico-negative zero slack has the same closed region as its lexico-feasible twin.
-/
import RpoptProofs.MplpLex.LexUnique
import RpoptProofs.FacetWalk

open Matrix Filter Topology

set_option linter.unusedSectionVars false

namespace Rpopt

/-- The data of a basis for the enumeration (H = 0). -/
structure EnumBasis (π : Type*) (M : ℕ) where
  /-- basic rows (the active set) -/
  B : Finset (Fin M)
  /-- slack constant `b_j - T_j b_B` -/
  s0 : Fin M → ℝ
  /-- slack gradient `F_j - T_j F_B` -/
  sθ : Fin M → π → ℝ
  /-- ε-row `e_j - T_j E_B` (identity block, θ-independent) -/
  σ : Fin M → Fin M → ℝ
  /-- (D): every basic inequality multiplier row is lexico-positive; θ-free when H = 0 -/
  dual : Prop

namespace EnumBasis

variable {π : Type*} [Fintype π] {M : ℕ} (E : EnumBasis π M)

/-- Nonbasic slack `s_j(θ)`. -/
def slack (j : Fin M) (θ : π → ℝ) : ℝ := E.s0 j + E.sθ j ⬝ᵥ θ

/-- Lexicographic slack row `[s_j(θ) | σ_j]` (mplp_lex.rs header, line 13). -/
def slackRow (j : Fin M) (θ : π → ℝ) : Fin (M + 1) → ℝ := Fin.cons (E.slack j θ) (E.σ j)

/-- Primal lexicographic feasibility at θ. -/
def PrimalLex (θ : π → ℝ) : Prop := ∀ j, j ∉ E.B → LexPos (E.slackRow j θ)

/-- **Step 1.** Lexicographic optimality at θ: (D) and lexico-positive nonbasic slack rows. -/
def LexOpt (θ : π → ℝ) : Prop := E.dual ∧ E.PrimalLex θ

/-- The lexicographic region `R_B`. -/
def region : Set (π → ℝ) := {θ | E.LexOpt θ}

/-- The closed region (`gen_cr` of the basis, before intersecting with Θ). -/
def closedRegion : Set (π → ℝ) := {θ | ∀ j, j ∉ E.B → 0 ≤ E.slack j θ}

/-- (Z): every identically zero nonbasic slack has a lexico-positive ε-row (`candidate_ok`). -/
def ZCond : Prop := ∀ j, j ∉ E.B → (∀ θ, E.slack j θ = 0) → LexPos (E.σ j)

theorem region_subset_closed : E.region ⊆ E.closedRegion := by
  intro θ hθ j hj
  have := (hθ.2 j hj).zero_nonneg
  simpa [slackRow] using this

theorem primalLex_zcond {θ : π → ℝ} (h : E.PrimalLex θ) : E.ZCond := by
  intro j hj hz
  have := h j hj
  rw [slackRow, hz θ, lexPos_cons_zero] at this
  exact this

/-- A slack that vanishes at an interior point of the closed region vanishes identically. -/
theorem slack_zero_of_interior {θ : π → ℝ} (hθ : θ ∈ interior E.closedRegion) {j : Fin M}
    (hj : j ∉ E.B) (h0 : E.slack j θ = 0) : ∀ θ', E.slack j θ' = 0 := by
  have hn : E.closedRegion ∈ 𝓝 θ := mem_interior_iff_mem_nhds.mp hθ
  have hgrad : ∀ u : π → ℝ, E.sθ j ⬝ᵥ u = 0 := by
    intro u
    have hc : ∀ v : π → ℝ, Tendsto (fun t : ℝ => θ + t • v) (𝓝 0) (𝓝 θ) := by
      intro v
      have : Continuous fun t : ℝ => θ + t • v :=
        continuous_const.add (continuous_id.smul continuous_const)
      simpa using this.tendsto 0
    have e1 : ∀ᶠ t in 𝓝[>] (0 : ℝ), θ + t • u ∈ E.closedRegion ∧
        θ + t • (-u) ∈ E.closedRegion ∧ 0 < t :=
      (((hc u).eventually hn).filter_mono nhdsWithin_le_nhds).and
        ((((hc (-u)).eventually hn).filter_mono nhdsWithin_le_nhds).and self_mem_nhdsWithin)
    obtain ⟨t, h1, h2, ht⟩ := e1.exists
    have a1 := h1 j hj
    have a2 := h2 j hj
    simp only [slack, dotProduct_add, dotProduct_smul, dotProduct_neg, smul_eq_mul] at a1 a2 h0
    have p1 : 0 ≤ t * (E.sθ j ⬝ᵥ u) := by linarith
    have p2 : t * (E.sθ j ⬝ᵥ u) ≤ 0 := by nlinarith
    rcases mul_eq_zero.mp (le_antisymm p2 p1) with h | h
    · exact absurd h ht.ne'
    · exact h
  intro θ'
  have := hgrad (θ' - θ)
  simp only [slack, dotProduct_sub] at this h0 ⊢
  linarith

/-- **Step 2.** In the interior of the closed region, primal lexicographic feasibility is
exactly (Z). -/
theorem primalLex_iff_of_interior {θ : π → ℝ} (hθ : θ ∈ interior E.closedRegion) :
    E.PrimalLex θ ↔ E.ZCond := by
  refine ⟨E.primalLex_zcond, fun hz j hj => ?_⟩
  have hnn : 0 ≤ E.slack j θ := interior_subset hθ j hj
  rcases hnn.lt_or_eq with hpos | hzero
  · exact lexPos_cons_pos hpos _
  · have hid := E.slack_zero_of_interior hθ hj hzero.symm
    rw [slackRow, hid θ, lexPos_cons_zero]
    exact hz j hj hid

theorem lexOpt_iff_of_interior {θ : π → ℝ} (hθ : θ ∈ interior E.closedRegion) :
    E.LexOpt θ ↔ E.dual ∧ E.ZCond := by
  rw [LexOpt, E.primalLex_iff_of_interior hθ]

/-- With (D) and (Z), the lexicographic region and the closed region have the same interior. -/
theorem interior_region_eq (hD : E.dual) (hZ : E.ZCond) :
    interior E.region = interior E.closedRegion := by
  apply Set.Subset.antisymm (interior_mono E.region_subset_closed)
  refine interior_maximal (fun θ hθ => ?_) isOpen_interior
  exact (E.lexOpt_iff_of_interior hθ).mpr ⟨hD, hZ⟩

/-- **Step 3.** `R_B` is full-dimensional iff (D), (Z) and the closed region is
full-dimensional. -/
theorem fullDim_iff :
    (interior E.region).Nonempty ↔ E.dual ∧ E.ZCond ∧ (interior E.closedRegion).Nonempty := by
  constructor
  · rintro ⟨θ, hθ⟩
    have hR : θ ∈ E.region := interior_subset hθ
    exact ⟨hR.1, E.primalLex_zcond hR.2, ⟨θ, interior_mono E.region_subset_closed hθ⟩⟩
  · rintro ⟨hD, hZ, hne⟩
    rwa [E.interior_region_eq hD hZ]

end EnumBasis

section Overlap

variable {π : Type*} [Fintype π] {M K : ℕ}

/-- Step 1 against the B3 model: if the rows of `E` are the coefficient vectors of the perturbed
LP at θ (`hlink`, the modelling link of `LexOptimalBasis`), two bases lexicographically optimal at
the same θ coincide. -/
theorem EnumBasis.lexOpt_unique {A : Matrix (Fin M) (Fin K) ℝ} {q : (π → ℝ) → Fin M → ℝ}
    {d : Fin K → ℝ} {Eq : Set (Fin M)} {E₁ E₂ : EnumBasis π M}
    (hlink₁ : ∀ θ, E₁.LexOpt θ → LexOptimalBasis A (q θ) d Eq E₁.B)
    (hlink₂ : ∀ θ, E₂.LexOpt θ → LexOptimalBasis A (q θ) d Eq E₂.B)
    (hcard : E₁.B.card = E₂.B.card) {θ : π → ℝ} (h₁ : E₁.LexOpt θ) (h₂ : E₂.LexOpt θ) :
    E₁.B = E₂.B :=
  lexOptimal_unique (hlink₁ θ h₁) (hlink₂ θ h₂) hcard

/-- **No overlaps of enumerated regions.** Two different bases that pass (D) and (Z) have closed
regions with disjoint interiors. -/
theorem enum_interiors_disjoint {A : Matrix (Fin M) (Fin K) ℝ} {q : (π → ℝ) → Fin M → ℝ}
    {d : Fin K → ℝ} {Eq : Set (Fin M)} {E₁ E₂ : EnumBasis π M}
    (hlink₁ : ∀ θ, E₁.LexOpt θ → LexOptimalBasis A (q θ) d Eq E₁.B)
    (hlink₂ : ∀ θ, E₂.LexOpt θ → LexOptimalBasis A (q θ) d Eq E₂.B)
    (hcard : E₁.B.card = E₂.B.card) (hne : E₁.B ≠ E₂.B)
    (hD₁ : E₁.dual) (hZ₁ : E₁.ZCond) (hD₂ : E₂.dual) (hZ₂ : E₂.ZCond) :
    Disjoint (interior E₁.closedRegion) (interior E₂.closedRegion) := by
  rw [← E₁.interior_region_eq hD₁ hZ₁, ← E₂.interior_region_eq hD₂ hZ₂]
  refine Set.disjoint_left.mpr fun θ h₁ h₂ => hne ?_
  have m₁ : θ ∈ E₁.region := interior_subset h₁
  have m₂ : θ ∈ E₂.region := interior_subset h₂
  exact EnumBasis.lexOpt_unique hlink₁ hlink₂ hcard m₁ m₂

end Overlap

end Rpopt
