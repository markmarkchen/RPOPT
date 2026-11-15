/-
The pivot of a dictionary `x = ρ + T y` (mplcp.rs `Tab::defer_pivot`, `Tab::flush`,
`Tab::pivoted`; also the deferred pivots of `pivot_facet`).

Pivot on row `r`, column `c`, pivot element `p = T r c ≠ 0`:
  ρ'_r = -ρ_r / p                ρ'_i = ρ_i - (T_ic / p) ρ_r      (i ≠ r)
  T'_rc = 1 / p                  T'_rk = -T_rk / p                 (k ≠ c)
  T'_ic = T_ic / p               T'_ik = T_ik - T_ic T_rk / p      (i ≠ r, k ≠ c)
Rust: `buf[n + k]` is the new row r of T, `rrow` the new row r of ρ, and every other row gets
`+ f * row r` with `f = T_ic` (`f == 0` rows are skipped, which changes nothing).

The right-hand side rows are allowed to take values in any real vector space `V`, so one
definition covers the numeric column, the θ coefficients, and the lexicographic block β.
-/
import RpoptProofs.Tableau

open Matrix

set_option linter.unusedSectionVars false

namespace Rpopt

section Dictionary

variable {ι κ : Type*} [DecidableEq ι] [DecidableEq κ]

/-- Pivoted right-hand side rows (any real vector space of row values). -/
noncomputable def pivVec {V : Type*} [AddCommGroup V] [Module ℝ V] (T : Matrix ι κ ℝ)
    (L : ι → V) (r : ι) (c : κ) : ι → V :=
  fun i => if i = r then (-(T r c)⁻¹) • L r else L i - (T i c / T r c) • L r

/-- Pivoted coefficient matrix. -/
noncomputable def pivT (T : Matrix ι κ ℝ) (r : ι) (c : κ) : Matrix ι κ ℝ :=
  fun i k =>
    if i = r then (if k = c then (T r c)⁻¹ else -(T r k / T r c))
    else (if k = c then T i c / T r c else T i k - T i c * T r k / T r c)

variable {V : Type*} [AddCommGroup V] [Module ℝ V]

@[simp] theorem pivT_pivot (T : Matrix ι κ ℝ) (r : ι) (c : κ) :
    pivT T r c r c = (T r c)⁻¹ := by simp [pivT]

/-- Pivoting twice on the same position gives back the tableau (values). -/
theorem pivVec_pivVec {T : Matrix ι κ ℝ} {r : ι} {c : κ} (hp : T r c ≠ 0) (L : ι → V) :
    pivVec (pivT T r c) (pivVec T L r c) r c = L := by
  funext i
  by_cases hi : i = r
  · subst hi
    simp only [pivVec, pivT, if_true, inv_inv, smul_smul]
    rw [show -T i c * -(T i c)⁻¹ = 1 by field_simp, one_smul]
  · simp only [pivVec, pivT, if_neg hi, if_true, smul_sub, smul_smul, sub_eq_iff_eq_add]
    have : T i c / T r c / (T r c)⁻¹ * -(T r c)⁻¹ = -(T i c / T r c) := by field_simp
    rw [this, neg_smul]
    abel

/-- Pivoting twice on the same position gives back the tableau (coefficients). -/
theorem pivT_pivT {T : Matrix ι κ ℝ} {r : ι} {c : κ} (hp : T r c ≠ 0) :
    pivT (pivT T r c) r c = T := by
  funext i k
  by_cases hi : i = r <;> by_cases hk : k = c
  · subst hi hk; simp [pivT]
  · subst hi; simp [pivT, hk]; field_simp
  · subst hk; simp [pivT, hi]; field_simp
  · simp [pivT, hi, hk]; field_simp; ring

/-- On the hyperplane where the pivot row vanishes, the pivot does not change the values. -/
theorem pivVec_of_zero {T : Matrix ι κ ℝ} {r : ι} {c : κ} {L : ι → V} (h0 : L r = 0) :
    pivVec T L r c = L := by
  funext i; by_cases hi : i = r
  · subst hi; simp [pivVec, h0]
  · simp [pivVec, hi, h0]

/-- The pivot is linear in the right-hand side: pivoting commutes with any linear map of the row
values (e.g. evaluation at θ, or evaluation of the perturbation at ε). -/
theorem pivVec_map {W : Type*} [AddCommGroup W] [Module ℝ W] (φ : V →ₗ[ℝ] W)
    (T : Matrix ι κ ℝ) (L : ι → V) (r : ι) (c : κ) :
    (fun i => φ (pivVec T L r c i)) = pivVec T (fun i => φ (L i)) r c := by
  funext i; by_cases hi : i = r <;> simp [pivVec, hi]

theorem pivVec_add (T : Matrix ι κ ℝ) (L L' : ι → V) (r : ι) (c : κ) :
    pivVec T (L + L') r c = pivVec T L r c + pivVec T L' r c := by
  funext i; by_cases hi : i = r <;> simp [pivVec, hi, smul_add]; abel

theorem pivVec_smul (T : Matrix ι κ ℝ) (a : ℝ) (L : ι → V) (r : ι) (c : κ) :
    pivVec T (a • L) r c = a • pivVec T L r c := by
  funext i; by_cases hi : i = r <;> simp [pivVec, hi, smul_sub, smul_comm a]

variable [Fintype κ]

/-- **A3 (one direction).** The pivoted dictionary holds after exchanging `x_r` and `y_c`. -/
theorem exchange_mp {T : Matrix ι κ ℝ} {r : ι} {c : κ} (hp : T r c ≠ 0) {ρ x : ι → ℝ}
    {y : κ → ℝ} (h : x = ρ + T *ᵥ y) :
    Function.update x r (y c) = pivVec T ρ r c + pivT T r c *ᵥ Function.update y c (x r) := by
  have hsplit : ∀ (U : Matrix ι κ ℝ) (z : κ → ℝ) (i : ι),
      (U *ᵥ z) i = U i c * z c + ∑ k ∈ Finset.univ.erase c, U i k * z k := by
    intro U z i
    simp only [mulVec, dotProduct]
    exact (Finset.add_sum_erase Finset.univ (fun k => U i k * z k) (Finset.mem_univ c)).symm
  set S : ι → ℝ := fun i => ∑ k ∈ Finset.univ.erase c, T i k * y k with hS
  have hx : ∀ i, x i = ρ i + T i c * y c + S i := by
    intro i; rw [h, Pi.add_apply, hsplit]; ring
  have hsum : ∀ i, ∑ k ∈ Finset.univ.erase c, pivT T r c i k * Function.update y c (x r) k
      = ∑ k ∈ Finset.univ.erase c, pivT T r c i k * y k := by
    intro i; refine Finset.sum_congr rfl fun k hk => ?_
    rw [Function.update_of_ne (Finset.ne_of_mem_erase hk)]
  funext i
  rw [Pi.add_apply, hsplit, hsum, Function.update_self]
  by_cases hi : i = r
  · subst hi
    have e : ∑ k ∈ Finset.univ.erase c, pivT T i c i k * y k = -(S i) / T i c := by
      rw [hS, neg_div, Finset.sum_div, ← Finset.sum_neg_distrib]
      refine Finset.sum_congr rfl fun k hk => ?_
      simp only [pivT, if_true, if_neg (Finset.ne_of_mem_erase hk)]
      ring
    rw [e, Function.update_self]
    simp only [pivVec, pivT, if_true, smul_eq_mul]
    rw [hx i]
    field_simp
    ring
  · have e : ∑ k ∈ Finset.univ.erase c, pivT T r c i k * y k
        = S i - T i c * S r / T r c := by
      simp only [hS]
      rw [Finset.mul_sum, Finset.sum_div, ← Finset.sum_sub_distrib]
      refine Finset.sum_congr rfl fun k hk => ?_
      simp only [pivT, if_neg hi, if_neg (Finset.ne_of_mem_erase hk)]
      ring
    rw [e, Function.update_of_ne hi]
    simp only [pivVec, pivT, if_neg hi, if_true, smul_eq_mul]
    rw [hx i, hx r]
    field_simp
    ring

/-- **A3 (exchange).** With a nonzero pivot element the pivoted dictionary has exactly the same
solutions, with `x_r` and `y_c` exchanged. -/
theorem exchange_iff {T : Matrix ι κ ℝ} {r : ι} {c : κ} (hp : T r c ≠ 0) (ρ x : ι → ℝ)
    (y : κ → ℝ) :
    x = ρ + T *ᵥ y ↔
      Function.update x r (y c) = pivVec T ρ r c + pivT T r c *ᵥ Function.update y c (x r) := by
  refine ⟨exchange_mp hp, fun h => ?_⟩
  have hp' : pivT T r c r c ≠ 0 := by simpa using hp
  have := exchange_mp hp' h
  simp only [Function.update_self, Function.update_idem, Function.update_eq_self,
    pivVec_pivVec hp, pivT_pivT hp] at this
  exact this

end Dictionary

section Principal

variable {ι : Type*} [Fintype ι] [DecidableEq ι]

/-- The complementary basis with pair `v` exchanged (`z_toggle` in mplcp.rs). -/
def toggle (B : Finset ι) (v : ι) : Finset ι := if v ∈ B then B.erase v else insert v B

theorem mem_toggle_ne {B : Finset ι} {v i : ι} (h : i ≠ v) : i ∈ toggle B v ↔ i ∈ B := by
  unfold toggle; split_ifs <;> simp [h]

theorem mem_toggle_self {B : Finset ι} {v : ι} : v ∈ toggle B v ↔ v ∉ B := by
  unfold toggle; split_ifs with h <;> simp [h]

theorem xB_toggle (B : Finset ι) (v : ι) (w z : ι → ℝ) :
    xB (toggle B v) w z = Function.update (xB B w z) v (yN B w z v) := by
  funext i
  by_cases hi : i = v
  · subst hi
    by_cases hv : i ∈ B <;> simp [xB, yN, mem_toggle_self, hv]
  · simp [xB, Function.update_of_ne hi, mem_toggle_ne hi]

theorem yN_toggle (B : Finset ι) (v : ι) (w z : ι → ℝ) :
    yN (toggle B v) w z = Function.update (yN B w z) v (xB B w z v) := by
  funext i
  by_cases hi : i = v
  · subst hi
    by_cases hv : i ∈ B <;> simp [xB, yN, mem_toggle_self, hv]
  · simp [yN, Function.update_of_ne hi, mem_toggle_ne hi]

/-- **A3 (principal pivot).** With a nonzero pivot element `T v v`, the principal pivot is a
tableau of the basis with pair `v` exchanged: same solution set, pair exchanged. -/
theorem pivot_isTableau {M : Matrix ι ι ℝ} {r : ι → ℝ} {B : Finset ι} {ρ : ι → ℝ}
    {T : Matrix ι ι ℝ} (h : IsTableau M r B ρ T) {v : ι} (hp : T v v ≠ 0) :
    IsTableau M r (toggle B v) (pivVec T ρ v v) (pivT T v v) := by
  intro w z
  rw [h w z, xB_toggle, yN_toggle]
  exact exchange_iff hp ρ (xB B w z) (yN B w z)

/-- **A3 (facet agreement).** Where the pivot row vanishes (on the facet hyperplane), the two
tableaux give the same values, hence the same basic solution. -/
theorem pivot_agree_on_facet {T : Matrix ι ι ℝ} {ρ : ι → ℝ} {v : ι} (h0 : ρ v = 0) :
    pivVec T ρ v v = ρ := pivVec_of_zero h0

/-! `tableau_for` (mplcp.rs): principal pivots `(i, i)` for the listed pairs, from `(r, M)`. -/

/-- The tableau after a list of principal pivots. -/
noncomputable def pivotSeq : (ι → ℝ) → Matrix ι ι ℝ → List ι → (ι → ℝ) × Matrix ι ι ℝ
  | ρ, T, [] => (ρ, T)
  | ρ, T, i :: l => pivotSeq (pivVec T ρ i i) (pivT T i i) l

/-- Every pivot element met along the list is nonzero (Rust: `piv.abs() < 1e-11 → None`). -/
def NonzeroPivots : Matrix ι ι ℝ → List ι → Prop
  | _, [] => True
  | T, i :: l => T i i ≠ 0 ∧ NonzeroPivots (pivT T i i) l

/-- The basis after a list of toggles. -/
def toggleSeq : Finset ι → List ι → Finset ι
  | B, [] => B
  | B, i :: l => toggleSeq (toggle B i) l

theorem pivotSeq_isTableau {M : Matrix ι ι ℝ} {r : ι → ℝ} :
    ∀ (l : List ι) {B : Finset ι} {ρ : ι → ℝ} {T : Matrix ι ι ℝ},
      IsTableau M r B ρ T → NonzeroPivots T l →
        IsTableau M r (toggleSeq B l) (pivotSeq ρ T l).1 (pivotSeq ρ T l).2
  | [], _, _, _, h, _ => h
  | i :: l, _, _, _, h, hp => pivotSeq_isTableau l (pivot_isTableau h hp.1) hp.2

theorem toggleSeq_eq_union :
    ∀ (l : List ι) {B : Finset ι}, l.Nodup → (∀ i ∈ l, i ∉ B) → toggleSeq B l = B ∪ l.toFinset
  | [], B, _, _ => by simp [toggleSeq]
  | i :: l, B, hnd, hout => by
    have hiB : i ∉ B := hout i (by simp)
    have hl := List.nodup_cons.mp hnd
    have ht : toggle B i = insert i B := by simp [toggle, hiB]
    simp only [toggleSeq, ht]
    rw [toggleSeq_eq_union l hl.2]
    · ext x; simp only [Finset.mem_union, Finset.mem_insert, List.mem_toFinset, List.mem_cons]
      tauto
    · intro x hx hxB
      rcases Finset.mem_insert.mp hxB with rfl | hx'
      · exact hl.1 hx
      · exact hout x (by simp [hx]) hx'

/-- **`tableau_for` is correct.** Principal pivots on distinct pairs `l` with nonzero pivot
elements, from the all-slack tableau `(r, M)`, give a tableau of the basis `l`. -/
theorem tableauFor_isTableau (M : Matrix ι ι ℝ) (r : ι → ℝ) {l : List ι} (hnd : l.Nodup)
    (hp : NonzeroPivots M l) :
    IsTableau M r l.toFinset (pivotSeq r M l).1 (pivotSeq r M l).2 := by
  have := pivotSeq_isTableau l (isTableau_empty M r) hp
  rwa [toggleSeq_eq_union l hnd (by simp), Finset.empty_union] at this

/-- **Pivoted and rebuilt tableaux agree, for chains of any length** (`REFACTOR`, lazy child
pivots in `TabSrc::resolve`). A chain of principal pivots with nonzero elements from a tableau of
`B` gives exactly the tableau that any other construction gives for the resulting basis: rebuilt
from scratch (`tableau_for`), reached by another chain, or by the same pivots applied later. -/
theorem pivotSeq_eq_of_isTableau {M : Matrix ι ι ℝ} {r : ι → ℝ} {B : Finset ι} {ρ : ι → ℝ}
    {T : Matrix ι ι ℝ} (h : IsTableau M r B ρ T) (l : List ι) (hp : NonzeroPivots T l)
    {ρ' : ι → ℝ} {T' : Matrix ι ι ℝ} (h' : IsTableau M r (toggleSeq B l) ρ' T') :
    pivotSeq ρ T l = (ρ', T') := by
  obtain ⟨h1, h2⟩ := tableau_unique (pivotSeq_isTableau l h hp) h'
  exact Prod.ext h1 h2

end Principal

end Rpopt
