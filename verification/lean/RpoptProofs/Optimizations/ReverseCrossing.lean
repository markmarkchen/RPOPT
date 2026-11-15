/-
Reverse crossing of a Case-1 facet (planned in `mplcp-order`): when region `S` has crossed the
facet `F` into `R = toggle S v` by a Case-1 pivot, the search of `F` from `R` can be skipped.

1. `pivVec_vanish_iff`, `pivVec_prop_iff`: a pivot on a row that vanishes on `F` keeps the set of
   rows that vanish on `F`, both as a set and in the proportional form `ρ_k = γ_k h` that `zrows`
   tests.
2. `toggle_toggle`, `Case1.reverse`, `Case1.reverse_neighbour`: seen from `R` (outward normal `-h`),
   `F` is again a Case-1 facet with the same leaving pair `v`, provided every other vanishing row is
   inert (`γ_i = 0` and `T i v = 0`, vacuous for a single vanishing row). Its toggle is `S`, with
   `S`'s tableau.
3. `reach_skip_reverse`: skipping the search of `(R, F)` when an explored region crossed `F` into
   `R` keeps the explored set, given that this region is the only neighbour of `R` across `F`.

When another vanishing row `i` has `T i v > 0` (allowed by `not_blocked` on `S`'s side), `R` sees two
decreasing rows and its search of `F` is Case 2, not Case 1. Its neighbour is then still `S` near
every relative-interior point, by `entry_facet_unique_neighbour` (local lexicographic uniqueness),
which is the general form of the hypothesis of `reach_skip_reverse`.
-/
import RpoptProofs.Optimizations.EntryFacetSkip

open Matrix

namespace Rpopt

section Rows

variable {Θ ι κ : Type*} [DecidableEq ι] [DecidableEq κ]

/-- **The pivot keeps the zero-row set (set form).** If the pivot row `r` vanishes on the
hyperplane `{h = 0}`, a row of the pivoted tableau vanishes there exactly when the same row of the
original does. -/
theorem pivVec_vanish_iff (T : Matrix ι κ ℝ) (ρ : Θ → ι → ℝ) (h : Θ → ℝ) {r : ι} (c : κ)
    (hr : ∀ θ, h θ = 0 → ρ θ r = 0) (k : ι) :
    (∀ θ, h θ = 0 → pivVec T (ρ θ) r c k = 0) ↔ (∀ θ, h θ = 0 → ρ θ k = 0) := by
  have e : ∀ θ, h θ = 0 → pivVec T (ρ θ) r c = ρ θ := fun θ hθ => pivVec_of_zero (hr θ hθ)
  constructor
  · intro H θ hθ
    have := H θ hθ
    rwa [e θ hθ] at this
  · intro H θ hθ
    rw [e θ hθ]
    exact H θ hθ

/-- **The pivot keeps the zero-row set (the form `zrows` tests).** If the pivot row is
`ρ_r = γ_r h`, a row of the pivoted tableau is a multiple of `h` exactly when the same row of the
original is. -/
theorem pivVec_prop_iff (T : Matrix ι κ ℝ) (ρ : Θ → ι → ℝ) (h : Θ → ℝ) {r : ι} (c : κ) {γr : ℝ}
    (hr : ∀ θ, ρ θ r = γr * h θ) (k : ι) :
    (∃ a : ℝ, ∀ θ, pivVec T (ρ θ) r c k = a * h θ) ↔ (∃ a : ℝ, ∀ θ, ρ θ k = a * h θ) := by
  by_cases hk : k = r
  · rw [hk]
    refine ⟨fun _ => ⟨γr, hr⟩, fun _ => ⟨-(T r c)⁻¹ * γr, fun θ => ?_⟩⟩
    simp only [pivVec, ↓reduceIte, smul_eq_mul, hr θ]
    ring
  · constructor
    · rintro ⟨a, ha⟩
      refine ⟨a + T k c / T r c * γr, fun θ => ?_⟩
      have := ha θ
      simp only [pivVec, if_neg hk, smul_eq_mul, hr θ] at this
      linear_combination this
    · rintro ⟨a, ha⟩
      refine ⟨a - T k c / T r c * γr, fun θ => ?_⟩
      simp only [pivVec, if_neg hk, smul_eq_mul, hr θ, ha θ]
      ring

end Rows

/-- Toggling the same pair twice gives back the basis. -/
theorem toggle_toggle {ι : Type*} [Fintype ι] [DecidableEq ι] (B : Finset ι) (v : ι) :
    toggle (toggle B v) v = B := by
  unfold toggle
  by_cases hv : v ∈ B
  · simp [hv, Finset.insert_erase hv]
  · simp [hv, Finset.erase_insert hv]

namespace Case1

variable {Θ : Type*} {n : ℕ} {M : Matrix (Fin n) (Fin n) ℝ} {r : Θ → Fin n → ℝ}
  {B : Finset (Fin n)} {ρ : Θ → Fin n → ℝ} {T : Matrix (Fin n) (Fin n) ℝ} {h : Θ → ℝ}
  {Z : Finset (Fin n)} {γ : Fin n → ℝ} {v : Fin n}

/-- Coefficients of the vanishing rows of the toggled tableau against `-h` (inert rows: 0). -/
noncomputable def γrev (_ : Case1 M r B ρ T h Z γ v) : Fin n → ℝ :=
  fun i => if i = v then γ v / T v v else 0

/-- **Reverse crossing (Case 1).** Seen from the toggled region `R = toggle B v`, which lies on
`h ≥ 0` (outward normal `-h`), the facet is a Case-1 facet with the same vanishing rows `Z` and the
same leaving pair `v`, when the other vanishing rows are inert (`γ_i = 0`, `T i v = 0`). -/
theorem reverse (H : Case1 M r B ρ T h Z γ v) (hZ : ∀ i ∈ Z, i ≠ v → γ i = 0 ∧ T i v = 0) :
    Case1 M r (toggle B v) H.ρ' (pivT T v v) (fun θ => -h θ) Z H.γrev v where
  tab := H.isTableau'
  v_mem := H.v_mem
  zero_rows := by
    intro i hi θ
    simp only [H.ρ'_apply, γrev]
    by_cases hiv : i = v
    · rw [if_pos hiv, if_pos hiv, H.zero_rows v H.v_mem θ]
      ring
    · obtain ⟨h1, h2⟩ := hZ i hi hiv
      rw [if_neg hiv, if_neg hiv, H.zero_rows i hi θ, H.zero_rows v H.v_mem θ, h1, h2]
      ring
  gam_v := by
    simp only [γrev, if_pos rfl]
    exact div_neg_of_neg_of_pos H.gam_v H.piv_pos
  gam_others := by
    intro i _ hiv
    simp [γrev, hiv]
  piv_pos := by
    rw [pivT_pivot]
    exact inv_pos.mpr H.piv_pos
  not_blocked := by
    intro i hi hiv
    obtain ⟨_, h2⟩ := hZ i hi hiv
    simp [pivT, γrev, hiv, h2]

/-- The toggled tableau of the reverse crossing is the original one. -/
theorem reverse_ρ' (H : Case1 M r B ρ T h Z γ v) (hZ : ∀ i ∈ Z, i ≠ v → γ i = 0 ∧ T i v = 0) :
    (H.reverse hZ).ρ' = ρ := by
  funext θ
  exact pivVec_pivVec H.piv_pos.ne' (ρ θ)

/-- **Reverse crossing: the Case-1 neighbour of `R` across the facet is `S`.** If `S` (basis `B`)
crosses the facet into `R = toggle B v` by a Case-1 pivot, then from `R` the facet is a Case-1
facet on the same pair, and its toggle is `B` with `B`'s tableau (rows and coefficients). -/
theorem reverse_neighbour (H : Case1 M r B ρ T h Z γ v)
    (hZ : ∀ i ∈ Z, i ≠ v → γ i = 0 ∧ T i v = 0) :
    ∃ H' : Case1 M r (toggle B v) H.ρ' (pivT T v v) (fun θ => -h θ) Z H.γrev v,
      toggle (toggle B v) v = B ∧ H'.ρ' = ρ ∧ pivT (pivT T v v) v v = T :=
  ⟨H.reverse hZ, toggle_toggle B v, H.reverse_ρ' hZ, pivT_pivT H.piv_pos.ne'⟩

/-- The single-row case of `reverse_neighbour` (`zrows.len() == 1`): no condition. -/
theorem reverse_neighbour_single (H : Case1 M r B ρ T h Z γ v) (hZ1 : Z = {v}) :
    ∃ H' : Case1 M r (toggle B v) H.ρ' (pivT T v v) (fun θ => -h θ) Z H.γrev v,
      toggle (toggle B v) v = B ∧ H'.ρ' = ρ ∧ pivT (pivT T v v) v v = T :=
  H.reverse_neighbour fun i hi hiv => absurd (Finset.mem_singleton.mp (hZ1 ▸ hi)) hiv

end Case1

/-- **Skipping reverse crossings keeps the explored set.** Neighbours are found facet by facet:
`nbr x F y` means that searching facet `F` of region `x` returns `y`. The evaluated search omits
`(x, F)` (`skip x F`) only when some explored region `z` has crossed `F` into `x` (`cross z F x`),
and every neighbour of `x` across `F` is then that `z` (`Case1.reverse_neighbour`, or
`entry_facet_unique_neighbour` when the search from `x` is Case 2). -/
theorem reach_skip_reverse {α φ : Type*} (nbr cross : α → φ → α → Prop) (skip : α → φ → Prop)
    {s : α}
    (hskip : ∀ x F, skip x F → x ∈ Reach (fun x y => ∃ F, ¬ skip x F ∧ nbr x F y) s →
      ∃ z ∈ Reach (fun x y => ∃ F, ¬ skip x F ∧ nbr x F y) s, cross z F x)
    (hrev : ∀ x z y F, cross z F x → nbr x F y → y = z) :
    Reach (fun x y => ∃ F, ¬ skip x F ∧ nbr x F y) s = Reach (fun x y => ∃ F, nbr x F y) s := by
  refine reach_skip (fun x y ⟨F, _, hF⟩ => ⟨F, hF⟩) fun x hx y ⟨F, hF⟩ hn => ?_
  have hs : skip x F := by
    by_contra hc
    exact hn ⟨F, hc, hF⟩
  obtain ⟨z, hz, hzx⟩ := hskip x F hs hx
  rw [hrev x z y F hzx hF]
  exact hz

end Rpopt
