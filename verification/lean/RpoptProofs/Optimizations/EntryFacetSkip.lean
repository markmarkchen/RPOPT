/-
Entry-facet skip (branch `mplcp-entry-skip`): when region B' was entered from its parent B by a
Case-1 pivot (`Nbr::Toggle`), the neighbour search of B' on the facet it was entered through is
skipped.

Safety argument, proved here in exact arithmetic:

1. `Case1.facet_eq`: the shared facet is the same set seen from B and from B'.
2. `entry_facet_unique_neighbour`: near every relative-interior point of that facet, every
   lexico-feasible complementary basis on B's side of the hyperplane *is* B, every one on the
   far side is B', and no third basis is lexico-feasible anywhere in the neighbourhood. So the
   search of B' across its entry facet could only return B, which is already in `seen`.
3. `Coverage.reach_skip_parent`: omitting a neighbour that is the explored parent does not change
   the explored set.

Future optimizations (known-neighbour skipping, pruning in the degenerate branching) get their own
files here; each must show that what it omits is already explored (`reach_skip`).
-/
import RpoptProofs.Facet
import RpoptProofs.Coverage

open Matrix Filter Topology

namespace Rpopt

variable {Θ : Type*} [TopologicalSpace Θ] {n : ℕ}

/-- **Entry-facet skip is safe (local form).** Let the toggled basis `B' = toggle B v` be entered
from `B` by a Case-1 pivot across `h = 0` (`H : Case1 …`), with `M` symmetric positive
semidefinite (`lcpM_posSemidef`). Then near every relative-interior point `θ₀` of the shared facet,
any complementary basis `C` that is lexico-feasible at `θ` is `B` or `B'`; it is `B` when `θ` is
on `B`'s side and `B'` on the other side. -/
theorem entry_facet_unique_neighbour {M : Matrix (Fin n) (Fin n) ℝ} (hM : M.PosSemidef)
    {r : Θ → Fin n → ℝ} {B : Finset (Fin n)} {ρ : Θ → Fin n → ℝ} {T : Matrix (Fin n) (Fin n) ℝ}
    {h : Θ → ℝ} {Z : Finset (Fin n)} {γ : Fin n → ℝ} {v : Fin n}
    (H : Case1 M r B ρ T h Z γ v) (hcont : ∀ i, Continuous fun θ => ρ θ i)
    {θ₀ : Θ} (h0 : h θ₀ = 0) (hrel : ∀ i, i ∉ Z → 0 < ρ θ₀ i)
    (hγ0 : ∀ i ∈ Z, i ≠ v → γ i = 0) {θ₁ : Θ} (hP : LexFeasible (H.L θ₁))
    (hβv : Fin.tail (H.L θ₀ v) ≠ 0) :
    ∀ᶠ θ in 𝓝 θ₀, ∀ (C : Finset (Fin n)) (LC : Fin n → Fin (n + 1) → ℝ)
      (TC : Matrix (Fin n) (Fin n) ℝ), IsLexTableau M (r θ) C LC TC → LexFeasible LC →
        (C = B ∨ C = toggle B v) ∧ (h θ < 0 → C = B) ∧ (0 < h θ → C = toggle B v) := by
  filter_upwards [H.lex_cover hcont h0 hrel hγ0 hP hβv] with θ hθ C LC TC hC fC
  obtain ⟨hnear, hfar, hor⟩ := hθ
  have tB : IsLexTableau M (r θ) B (H.L θ) T := isLexTableau_of_isTableau (H.tab θ)
  have tB' : IsLexTableau M (r θ) (toggle B v) (H.L' θ) (pivT T v v) :=
    isLexTableau_of_isTableau (H.isTableau' θ)
  refine ⟨?_, fun hh => lex_unique hM hC tB fC (hnear hh),
    fun hh => lex_unique hM hC tB' fC (hfar hh)⟩
  rcases hor with p | p
  · exact Or.inl (lex_unique hM hC tB fC p)
  · exact Or.inr (lex_unique hM hC tB' fC p)

/-- The parent `B` itself is lexico-feasible on its side near the facet, and the entered basis on
the other side (the facet is a genuine boundary between exactly these two regions). -/
theorem entry_facet_both_sides {M : Matrix (Fin n) (Fin n) ℝ}
    {r : Θ → Fin n → ℝ} {B : Finset (Fin n)} {ρ : Θ → Fin n → ℝ} {T : Matrix (Fin n) (Fin n) ℝ}
    {h : Θ → ℝ} {Z : Finset (Fin n)} {γ : Fin n → ℝ} {v : Fin n}
    (H : Case1 M r B ρ T h Z γ v) (hcont : ∀ i, Continuous fun θ => ρ θ i)
    {θ₀ : Θ} (h0 : h θ₀ = 0) (hrel : ∀ i, i ∉ Z → 0 < ρ θ₀ i)
    (hγ0 : ∀ i ∈ Z, i ≠ v → γ i = 0) {θ₁ : Θ} (hP : LexFeasible (H.L θ₁))
    (hβv : Fin.tail (H.L θ₀ v) ≠ 0) :
    ∀ᶠ θ in 𝓝 θ₀, (h θ < 0 → LexFeasible (H.L θ)) ∧ (0 < h θ → LexFeasible (H.L' θ)) := by
  filter_upwards [H.lex_cover hcont h0 hrel hγ0 hP hβv] with θ hθ using ⟨hθ.1, hθ.2.1⟩

end Rpopt
