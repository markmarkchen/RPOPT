/-
Known-region skip (step 2 of branch `mplcp-case2-fix`): skip the neighbour search on a Case-2
facet `F` when one known full-dimensional region `R` has a row on the same hyperplane with the
opposite normal and every vertex of `F` satisfies `R`'s rows.

Proved here, in exact arithmetic, for parameters `θ ∈ ℝ^π`:

1. `vertices_subset_polyhedron`: the vertices of `F` lie in `R`, so `F = conv(vertices) ⊆ R`.
2. `active_row_multiple`, `near_side_subset`: `R` (in halfspace form `E θ ≤ f`, the rows of a
   `CriticalRegion`) then contains a half-neighbourhood, on its side, of every
   relative-interior point of `F`.
3. `lexFeasible_of_mem_interior`: a basis that is lexico-feasible somewhere is lexico-feasible at
   every interior point of its critical region.
4. `known_region_unique_neighbour`: near every relative-interior point of `F`, every
   lexico-feasible basis on `R`'s side is `R`'s basis (`lex_unique`).
5. `known_region_skip`: so the search across `F` could only return `R`, which is explored, and
   skipping it leaves the explored set unchanged (`reach_skip`).

Hypothesis to keep in mind: `R`'s basis must be lexico-feasible (`hlex`). A lexico-infeasible
twin region (same set, wrong basis) does not satisfy it.
-/
import RpoptProofs.Optimizations.EntryFacetSkip

open Matrix Filter Topology

set_option linter.unusedSectionVars false

namespace Rpopt

section Geometry

variable {π κ : Type*} [Fintype π] [Fintype κ]

theorem continuous_dot (g : π → ℝ) : Continuous fun θ : π → ℝ => g ⬝ᵥ θ := by
  show Continuous fun θ : π → ℝ => ∑ j, g j * θ j
  exact continuous_finset_sum _ fun j _ => continuous_const.mul (continuous_apply j)

theorem continuous_mulVec_apply {ι : Type*} (E : Matrix ι π ℝ) (k : ι) :
    Continuous fun θ : π → ℝ => (E *ᵥ θ) k :=
  continuous_dot (fun j => E k j)

/-- A vector orthogonal to every vector orthogonal to `g ≠ 0` is a multiple of `g`. -/
theorem eq_smul_of_perp {g a : π → ℝ} (hg : g ≠ 0)
    (h : ∀ u, g ⬝ᵥ u = 0 → a ⬝ᵥ u = 0) : ∃ c : ℝ, a = c • g := by
  have hgg : g ⬝ᵥ g ≠ 0 := fun h0 => hg (dotProduct_self_eq_zero.mp h0)
  refine ⟨(a ⬝ᵥ g) / (g ⬝ᵥ g), ?_⟩
  have hgu : g ⬝ᵥ (a - ((a ⬝ᵥ g) / (g ⬝ᵥ g)) • g) = 0 := by
    rw [dotProduct_sub, dotProduct_smul, smul_eq_mul, dotProduct_comm g a]
    field_simp
    ring
  have hau := h _ hgu
  have huu : (a - ((a ⬝ᵥ g) / (g ⬝ᵥ g)) • g) ⬝ᵥ (a - ((a ⬝ᵥ g) / (g ⬝ᵥ g)) • g) = 0 := by
    rw [sub_dotProduct, smul_dotProduct, hau, hgu]; simp
  exact sub_eq_zero.mp (dotProduct_self_eq_zero.mp huu)

/-- A region in halfspace form `E θ ≤ f` (`CriticalRegion::e`, `f`). -/
def Polyhedron (E : Matrix κ π ℝ) (f : κ → ℝ) : Set (π → ℝ) := {θ | ∀ k, (E *ᵥ θ) k ≤ f k}

theorem convex_polyhedron (E : Matrix κ π ℝ) (f : κ → ℝ) : Convex ℝ (Polyhedron E f) := by
  intro x hx y hy a b ha hb hab
  simp only [Polyhedron, Set.mem_setOf_eq] at hx hy ⊢
  intro k
  simp only [mulVec_add, mulVec_smul, Pi.add_apply, Pi.smul_apply, smul_eq_mul]
  have h1 := mul_le_mul_of_nonneg_left (hx k) ha
  have h2 := mul_le_mul_of_nonneg_left (hy k) hb
  have h3 : a * f k + b * f k = f k := by rw [← add_mul, hab, one_mul]
  linarith

/-- **Convexity step.** If every vertex of `F` satisfies `R`'s rows, the whole convex hull does. -/
theorem vertices_subset_polyhedron {E : Matrix κ π ℝ} {f : κ → ℝ} {V : Set (π → ℝ)}
    (hV : V ⊆ Polyhedron E f) : convexHull ℝ V ⊆ Polyhedron E f :=
  convexHull_min hV (convex_polyhedron E f)

/-- A row of `R` that is active at a relative-interior point of `F ⊆ R` (inside the hyperplane
`g·θ + b₀ = 0`) is a multiple of the hyperplane. -/
theorem active_row_multiple {E : Matrix κ π ℝ} {f : κ → ℝ} {g : π → ℝ} (hg : g ≠ 0) {b₀ : ℝ}
    {F : Set (π → ℝ)} (hF : F ⊆ Polyhedron E f) {θ₀ : π → ℝ}
    (hrel : ∀ u, g ⬝ᵥ u = 0 → ∃ t : ℝ, 0 < t ∧ θ₀ + t • u ∈ F ∧ θ₀ - t • u ∈ F)
    (hθ₀ : g ⬝ᵥ θ₀ + b₀ = 0) {k : κ} (hk : (E *ᵥ θ₀) k = f k) :
    ∃ γ : ℝ, ∀ θ, (E *ᵥ θ) k - f k = γ * (g ⬝ᵥ θ + b₀) := by
  have hrow : ∀ θ, (E *ᵥ θ) k = (fun j => E k j) ⬝ᵥ θ := fun θ => rfl
  have hperp : ∀ u, g ⬝ᵥ u = 0 → (fun j => E k j) ⬝ᵥ u = 0 := by
    intro u hu
    obtain ⟨t, ht, h1, h2⟩ := hrel u hu
    have a1 := hF h1 k
    have a2 := hF h2 k
    rw [hrow, dotProduct_add, dotProduct_smul, smul_eq_mul] at a1
    rw [hrow, dotProduct_sub, dotProduct_smul, smul_eq_mul] at a2
    rw [hrow] at hk
    have p1 : t * ((fun j => E k j) ⬝ᵥ u) ≤ 0 := by linarith
    have p2 : 0 ≤ t * ((fun j => E k j) ⬝ᵥ u) := by linarith
    have := le_antisymm p1 p2
    rcases mul_eq_zero.mp this with h | h
    · exact absurd h ht.ne'
    · exact h
  obtain ⟨c, hc⟩ := eq_smul_of_perp hg hperp
  refine ⟨c, fun θ => ?_⟩
  have e1 : (E *ᵥ θ) k = c * (g ⬝ᵥ θ) := by rw [hrow, hc, smul_dotProduct, smul_eq_mul]
  have e0 : (E *ᵥ θ₀) k = c * (g ⬝ᵥ θ₀) := by rw [hrow, hc, smul_dotProduct, smul_eq_mul]
  rw [e1]
  have : f k = -(c * b₀) := by rw [← hk, e0]; linear_combination c * hθ₀
  rw [this]; ring

/-- **`R` covers its side of `F`.** If `F ⊆ R`, `F` lies in the hyperplane, `θ₀` is a
relative-interior point of `F`, and `R` has a point strictly on its side, then `R` contains every
point near `θ₀` on that side. -/
theorem near_side_subset {E : Matrix κ π ℝ} {f : κ → ℝ} {g : π → ℝ} (hg : g ≠ 0) {b₀ : ℝ}
    {F : Set (π → ℝ)} (hF : F ⊆ Polyhedron E f) {θ₀ : π → ℝ} (hθ₀F : θ₀ ∈ F)
    (hrel : ∀ u, g ⬝ᵥ u = 0 → ∃ t : ℝ, 0 < t ∧ θ₀ + t • u ∈ F ∧ θ₀ - t • u ∈ F)
    (hθ₀ : g ⬝ᵥ θ₀ + b₀ = 0) {θ₂ : π → ℝ} (hθ₂ : θ₂ ∈ Polyhedron E f)
    (hside : g ⬝ᵥ θ₂ + b₀ < 0) :
    ∀ᶠ θ in 𝓝 θ₀, g ⬝ᵥ θ + b₀ ≤ 0 → θ ∈ Polyhedron E f := by
  have hall : ∀ k, ∀ᶠ θ in 𝓝 θ₀, g ⬝ᵥ θ + b₀ ≤ 0 → (E *ᵥ θ) k ≤ f k := by
    intro k
    by_cases hk : (E *ᵥ θ₀) k = f k
    · obtain ⟨γ, hγ⟩ := active_row_multiple hg hF hrel hθ₀ hk
      have hγ0 : 0 ≤ γ := by
        have := hθ₂ k
        have e := hγ θ₂
        by_contra hneg
        push_neg at hneg
        nlinarith
      refine Eventually.of_forall fun θ hθ => ?_
      have := hγ θ
      nlinarith [mul_nonpos_of_nonneg_of_nonpos hγ0 hθ]
    · have hlt : (E *ᵥ θ₀) k < f k := lt_of_le_of_ne (hF hθ₀F k) hk
      have := (continuous_mulVec_apply E k).continuousAt.eventually (gt_mem_nhds hlt)
      filter_upwards [this] with θ h _ using h.le
  filter_upwards [eventually_all.mpr hall] with θ h hθ k using h k hθ

end Geometry

section Lex

variable {π : Type*} [Fintype π] {n : ℕ}

/-- **Interior points are lexico-feasible.** If a basis is lexico-feasible at one point, it is
lexico-feasible at every interior point of its critical region: there, each tableau row is
either positive or identically zero, and the lexicographic sign of an identically zero row does
not depend on `θ`. -/
theorem lexFeasible_of_mem_interior {B : Finset (Fin n)} {ρ₀ : Fin n → ℝ}
    {Rρ : Matrix (Fin n) π ℝ} {T : Matrix (Fin n) (Fin n) ℝ} {θ₁ θ : π → ℝ}
    (hlex : LexFeasible (lexRows B (ρ₀ + Rρ *ᵥ θ₁) T))
    (hθ : θ ∈ interior (CR fun θ => ρ₀ + Rρ *ᵥ θ)) :
    LexFeasible (lexRows B (ρ₀ + Rρ *ᵥ θ) T) := by
  intro i
  have hcr : θ ∈ CR (fun θ => ρ₀ + Rρ *ᵥ θ) := interior_subset hθ
  have hcons : ∀ θ', lexRows B (ρ₀ + Rρ *ᵥ θ') T i =
      Fin.cons ((ρ₀ + Rρ *ᵥ θ') i) (fun j => if j ∈ B then -T i j else if i = j then 1 else 0) :=
    fun θ' => rfl
  rcases (hcr i).lt_or_eq with hpos | hzero
  · rw [hcons]; exact lexPos_cons_pos hpos _
  · -- the row is identically zero: its gradient vanishes
    have hgrad : ∀ u : π → ℝ, (Rρ *ᵥ u) i = 0 := by
      intro u
      have hn : CR (fun θ => ρ₀ + Rρ *ᵥ θ) ∈ 𝓝 θ := mem_interior_iff_mem_nhds.mp hθ
      have hc : ∀ v : π → ℝ, Tendsto (fun t : ℝ => θ + t • v) (𝓝 0) (𝓝 θ) := by
        intro v
        have : Continuous fun t : ℝ => θ + t • v := continuous_const.add (continuous_id.smul continuous_const)
        simpa using this.tendsto 0
      have e1 : ∀ᶠ t in 𝓝[>] (0 : ℝ), θ + t • u ∈ CR (fun θ => ρ₀ + Rρ *ᵥ θ) ∧
          θ + t • (-u) ∈ CR (fun θ => ρ₀ + Rρ *ᵥ θ) ∧ 0 < t :=
        (((hc u).eventually hn).filter_mono nhdsWithin_le_nhds).and
          ((((hc (-u)).eventually hn).filter_mono nhdsWithin_le_nhds).and self_mem_nhdsWithin)
      obtain ⟨t, h1, h2, ht⟩ := e1.exists
      have a1 := h1 i
      have a2 := h2 i
      simp only [mulVec_add, mulVec_smul, mulVec_neg, Pi.add_apply, Pi.smul_apply, Pi.neg_apply,
        smul_eq_mul] at a1 a2
      have h0 : ρ₀ i + (Rρ *ᵥ θ) i = 0 := by simpa using hzero.symm
      have p1 : 0 ≤ t * (Rρ *ᵥ u) i := by linarith
      have p2 : t * (Rρ *ᵥ u) i ≤ 0 := by nlinarith
      have := le_antisymm p2 p1
      rcases mul_eq_zero.mp this with h | h
      · exact absurd h ht.ne'
      · exact h
    have hconst : ∀ θ', (ρ₀ + Rρ *ᵥ θ') i = 0 := by
      intro θ'
      have := hgrad (θ' - θ)
      rw [mulVec_sub, Pi.sub_apply] at this
      have h0 : (ρ₀ + Rρ *ᵥ θ) i = 0 := hzero.symm
      simp only [Pi.add_apply] at h0 ⊢
      linarith
    have h1 := hlex i
    rw [hcons, hconst, lexPos_cons_zero] at h1
    rw [hcons, hconst, lexPos_cons_zero]
    exact h1

/-- **Step-2 certificate.** Let `R` be a known region with lexico-feasible basis `B_R` (tableau
rows `ρ₀ + Rρ θ`), whose halfspace form `E θ ≤ f` lies in its critical region, and let `F` lie in
the hyperplane `g·θ + b₀ = 0`, with `F ⊆ R` and `R` strictly on the side `g·θ + b₀ < 0`. Then near
every relative-interior point `θ₀` of `F`, every lexico-feasible complementary basis at a point on
`R`'s side is `B_R`. -/
theorem known_region_unique_neighbour {M : Matrix (Fin n) (Fin n) ℝ} (hM : M.PosSemidef)
    {r : (π → ℝ) → Fin n → ℝ} {BR : Finset (Fin n)} {ρ₀ : Fin n → ℝ} {Rρ : Matrix (Fin n) π ℝ}
    {TR : Matrix (Fin n) (Fin n) ℝ} (htab : ∀ θ, IsTableau M (r θ) BR (ρ₀ + Rρ *ᵥ θ) TR)
    {θ₁ : π → ℝ} (hlex : LexFeasible (lexRows BR (ρ₀ + Rρ *ᵥ θ₁) TR))
    {κ : Type*} [Fintype κ] {E : Matrix κ π ℝ} {f : κ → ℝ}
    (hreg : Polyhedron E f ⊆ CR fun θ => ρ₀ + Rρ *ᵥ θ)
    {g : π → ℝ} (hg : g ≠ 0) {b₀ : ℝ} {F : Set (π → ℝ)} (hF : F ⊆ Polyhedron E f)
    {θ₀ : π → ℝ} (hθ₀F : θ₀ ∈ F) (hθ₀ : g ⬝ᵥ θ₀ + b₀ = 0)
    (hrel : ∀ u, g ⬝ᵥ u = 0 → ∃ t : ℝ, 0 < t ∧ θ₀ + t • u ∈ F ∧ θ₀ - t • u ∈ F)
    {θ₂ : π → ℝ} (hθ₂ : θ₂ ∈ Polyhedron E f) (hside : g ⬝ᵥ θ₂ + b₀ < 0) :
    ∀ᶠ θ in 𝓝 θ₀, g ⬝ᵥ θ + b₀ < 0 → ∀ (C : Finset (Fin n)) (LC : Fin n → Fin (n + 1) → ℝ)
      (TC : Matrix (Fin n) (Fin n) ℝ), IsLexTableau M (r θ) C LC TC → LexFeasible LC → C = BR := by
  obtain ⟨U, hUsub, hUopen, hθ₀U⟩ :=
    eventually_nhds_iff.mp (near_side_subset hg hF hθ₀F hrel hθ₀ hθ₂ hside)
  filter_upwards [hUopen.mem_nhds hθ₀U] with θ hθU hneg C LC TC hC fC
  have hopen : IsOpen (U ∩ {θ | g ⬝ᵥ θ + b₀ < 0}) :=
    hUopen.inter (isOpen_lt ((continuous_dot g).add continuous_const) continuous_const)
  have hsub : U ∩ {θ | g ⬝ᵥ θ + b₀ < 0} ⊆ CR fun θ => ρ₀ + Rρ *ᵥ θ :=
    fun y hy => hreg (hUsub y hy.1 (le_of_lt hy.2))
  have hint : θ ∈ interior (CR fun θ => ρ₀ + Rρ *ᵥ θ) := interior_maximal hsub hopen ⟨hθU, hneg⟩
  exact lex_unique hM hC (isLexTableau_of_isTableau (htab θ)) fC
    (lexFeasible_of_mem_interior hlex hint)

end Lex

/-- **Skipping certified facets keeps the explored set.** If every neighbour candidate that the
skip omits is a known (explored) region, the explored set is unchanged. With
`known_region_unique_neighbour`, the only candidate across a certified facet is the known region
`R`. -/
theorem known_region_skip {α : Type*} {adj adj' : α → α → Prop} {s : α}
    (hsub : ∀ x y, adj' x y → adj x y)
    (hknown : ∀ x ∈ Reach adj' s, ∀ y, adj x y → ¬ adj' x y → ∃ R ∈ Reach adj' s, y = R) :
    Reach adj' s = Reach adj s :=
  reach_skip hsub fun x hx y hxy hn => by
    obtain ⟨R, hR, rfl⟩ := hknown x hx y hxy hn
    exact hR

end Rpopt
