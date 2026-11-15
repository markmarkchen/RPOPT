/-
The dependent-row facet (the dominant Case-2 situation; to replace the walk by a direct
enumeration).

Setting. Region `R` with active set `A` (its pLCP basis: `z` basic on `A`), tableau `(ρ, T)`.
On the facet `F` exactly one slack row `c ∉ A` vanishes, and the pivot element on `c`'s
complement is zero because the constraint row `a_c` is a combination `Σ_{j ∈ A} μ_j a_j` of the
active rows. In the tableau this reads (see the Schur complement of `M = A H⁻¹ Aᵀ`):

  row `c` (holds `w_c`):   `w_c = ρ_c + Σ_{j ∈ A} μ_j w_j`   (no `z` column: `T c k = 0` for `k ∉ A`),
  column `c` (holds `z_c`):  `T i c = -μ_i` for `i ∈ A`,  `T k c = 0` for `k ∉ A`.

`D(θ) = -ρ_c(θ)` is the facet function: `D ≤ 0` on `R`, `D = 0` on the facet hyperplane, `D > 0`
across it. `T = A ∪ {c}` has rank deficiency 1, and the candidates across `F` are
`B_j = T \ {j}`, `j ∈ P⁻ = {j ∈ A : μ_j > 0}`.

Proved here, in exact arithmetic:
* `identity`: every solution of `w = r + M z` has `w_c = -D + Σ_{j ∈ A} μ_j w_j`.
* `no_solution_of_empty`, `qp_infeasible_of_empty`: with `P⁻` empty, no parameter with `D > 0` is
  feasible (no `z` with `w ≥ 0`, and, at the QP level, no feasible `x`).
* `slack_side`: a solution whose slacks vanish on `T \ {j}` has `μ_j w_j = D`; so across `F` only
  `j ∈ P⁻` can work.
* `candidate_solves`: the explicit basic solution of `B_j`, built from `R`'s tableau.
* `on_facet` (**c**): on `F`, `B_j` is valid exactly where `λ_j / μ_j ≤ λ_k / μ_k` for all `k ∈ P⁻`
  (`λ = ρ` on `A`, `R`'s multipliers).
* `across` and `unique_across` (**a**, **b**): at a relative-interior point of `F` where `j` is the
  strict argmin, `B_j` is valid on the far side, its basic variables are positive across, and every
  complementary basis valid at a point just across `F` is `B_j`. So the Case-2 walk can only return
  members of `S = {B_j : j ∈ P⁻}` (`returned_mem_S`), and skipping explored members keeps the
  explored set (`reach_skip`).
Ties (several `j` attaining the minimum at a facet point) are left out: they form a
lower-dimensional part of `F` when the ratios are distinct affine functions on `F`.
-/
import RpoptProofs.FacetWalk
import RpoptProofs.Coverage

open Matrix Filter Topology

set_option linter.unusedSectionVars false

namespace Rpopt

section QPLevel

variable {ι n : Type*} [Fintype ι] [Fintype n]

/-- **No feasible point across `F` when `P⁻` is empty** (QP level). If `a_c = Σ_{j ∈ A} μ_j a_j`
with every `μ_j ≤ 0`, and `D = Σ μ_j q_j - q_c > 0`, the constraints `A x ≤ q` have no solution. -/
theorem qp_infeasible_of_empty {Aq : Matrix ι n ℝ} {q : ι → ℝ} {A : Finset ι} {c : ι}
    {μ : ι → ℝ} (hrow : Aq c = ∑ j ∈ A, μ j • Aq j) (hμ : ∀ j ∈ A, μ j ≤ 0)
    (hD : 0 < ∑ j ∈ A, μ j * q j - q c) (x : n → ℝ) : ¬ QPFeasible Aq q x := by
  intro hx
  have hc : (Aq *ᵥ x) c = ∑ j ∈ A, μ j * (Aq *ᵥ x) j := by
    show Aq c ⬝ᵥ x = ∑ j ∈ A, μ j * (Aq j ⬝ᵥ x)
    rw [hrow, sum_dotProduct]
    exact Finset.sum_congr rfl fun j _ => by rw [smul_dotProduct, smul_eq_mul]
  have hneg : ∑ j ∈ A, μ j * (q j - (Aq *ᵥ x) j) ≤ 0 :=
    Finset.sum_nonpos fun j hj => mul_nonpos_of_nonpos_of_nonneg (hμ j hj) (sub_nonneg.mpr (hx j))
  have hsplit : ∑ j ∈ A, μ j * (q j - (Aq *ᵥ x) j) =
      ∑ j ∈ A, μ j * q j - ∑ j ∈ A, μ j * (Aq *ᵥ x) j := by
    rw [← Finset.sum_sub_distrib]; exact Finset.sum_congr rfl fun j _ => by ring
  have := hx c
  linarith

end QPLevel

variable {ι : Type*} [Fintype ι] [DecidableEq ι] {Θ : Type*}

/-- The dependent-row facet: the tableau structure of row `c` and column `c`. -/
structure DepFacet (M : Matrix ι ι ℝ) (r : Θ → ι → ℝ) (A : Finset ι) (ρ : Θ → ι → ℝ)
    (T : Matrix ι ι ℝ) (c : ι) (μ : ι → ℝ) : Prop where
  tab : ∀ θ, IsTableau M (r θ) A (ρ θ) T
  c_out : c ∉ A
  row_c : ∀ k, T c k = if k ∈ A then μ k else 0
  col_c : ∀ i, T i c = if i ∈ A then -μ i else 0

namespace DepFacet

variable {M : Matrix ι ι ℝ} {r : Θ → ι → ℝ} {A : Finset ι} {ρ : Θ → ι → ℝ}
  {T : Matrix ι ι ℝ} {c : ι} {μ : ι → ℝ}

/-- **The tableau structure follows from the dependency.** If row and column `c` of `M` are the
`μ`-combinations of the rows and columns of `A` (`a_c = Σ_{j ∈ A} μ_j a_j` with `M = A H⁻¹ Aᵀ`), and
the columns of `A` are independent in `M` (independent active rows), then any tableau of `A` has
the row-`c` and column-`c` structure of `DepFacet`. In particular `T c c = 0`: the pivot element on
`c`'s complement vanishes (`tve ≈ 0` in `adjacent`). -/
theorem of_dependency [Nonempty Θ] (htab : ∀ θ, IsTableau M (r θ) A (ρ θ) T) (hc : c ∉ A)
    (hrowM : ∀ k, M c k = ∑ j ∈ A, μ j * M j k) (hcolM : ∀ k, M k c = ∑ j ∈ A, μ j * M k j)
    (hinjA : ∀ u : ι → ℝ, (∀ i, i ∉ A → u i = 0) → (∀ k ∈ A, (M *ᵥ u) k = 0) → u = 0) :
    DepFacet M r A ρ T c μ := by
  obtain ⟨θ₀⟩ := (inferInstance : Nonempty Θ)
  -- every nonbasic assignment gives a solution
  have sol : ∀ y : ι → ℝ, wOf A (ρ θ₀ + T *ᵥ y) y = r θ₀ + M *ᵥ zOf A (ρ θ₀ + T *ᵥ y) y := by
    intro y; rw [htab θ₀]; simp
  -- row c: (T y)_c = Σ_{j ∈ A} μ_j y_j
  have rowlin : ∀ y : ι → ℝ, (T *ᵥ y) c = ∑ j ∈ A, μ j * y j := by
    have key : ∀ y : ι → ℝ, ρ θ₀ c + (T *ᵥ y) c =
        r θ₀ c - ∑ j ∈ A, μ j * r θ₀ j + ∑ j ∈ A, μ j * y j := by
      intro y
      have e := sol y
      have ec := congrFun e c
      have hwc : wOf A (ρ θ₀ + T *ᵥ y) y c = ρ θ₀ c + (T *ᵥ y) c := by simp [wOf, hc]
      have hwj : ∀ j ∈ A, wOf A (ρ θ₀ + T *ᵥ y) y j = y j := fun j hj => by simp [wOf, hj]
      set z := zOf A (ρ θ₀ + T *ᵥ y) y
      have hMc : (M *ᵥ z) c = ∑ j ∈ A, μ j * (M *ᵥ z) j := by
        simp only [mulVec, dotProduct, hrowM, Finset.sum_mul, Finset.mul_sum]
        rw [Finset.sum_comm]
        exact Finset.sum_congr rfl fun j _ => Finset.sum_congr rfl fun k _ => by ring
      have hMj : ∀ j ∈ A, (M *ᵥ z) j = y j - r θ₀ j := by
        intro j hj
        have := congrFun e j
        rw [hwj j hj, Pi.add_apply] at this
        linarith
      rw [hwc, Pi.add_apply, hMc, Finset.sum_congr rfl (fun j hj => by rw [hMj j hj])] at ec
      rw [ec]
      simp only [mul_sub, Finset.sum_sub_distrib]
      ring
    intro y
    have k0 := key 0
    simp only [mulVec_zero, Pi.zero_apply, mul_zero, Finset.sum_const_zero, add_zero] at k0
    linarith [key y]
  -- column c: the variation of z_c by 1 moves z_A by T_{·c} and w_N by T_{·c}
  have colfacts : (∀ i ∈ A, T i c = -μ i) ∧ (∀ i, i ∉ A → T i c = 0) := by
    set y1 : ι → ℝ := Pi.single c 1
    set x1 := ρ θ₀ + T *ᵥ y1
    have e1 := sol y1
    have e0 := sol 0
    simp only [mulVec_zero, add_zero] at e0
    set v : ι → ℝ := fun i => if i ∈ A then μ i + T i c else 0
    have hTcol : ∀ i, (T *ᵥ y1) i = T i c := by
      intro i; simp [y1, mulVec, dotProduct, Pi.single_apply]
    -- M v equals the change of w
    have hMv : M *ᵥ v = wOf A x1 y1 - wOf A (ρ θ₀) 0 := by
      have hdiff : zOf A x1 y1 - zOf A (ρ θ₀) 0 = fun i => if i ∈ A then T i c else y1 i := by
        funext i; by_cases hi : i ∈ A <;> simp [zOf, hi, x1, hTcol]
      have hsplit : v = (zOf A x1 y1 - zOf A (ρ θ₀) 0) - y1 +
          (fun i => if i ∈ A then μ i else 0) := by
        rw [hdiff]; funext i
        by_cases hi : i ∈ A
        · have hic : i ≠ c := fun e => hc (e ▸ hi)
          have : y1 i = 0 := by simp [y1, Pi.single_apply, hic]
          simp [v, hi, this]; ring
        · simp [v, hi]
      have hMy1 : M *ᵥ y1 = M *ᵥ (fun i => if i ∈ A then μ i else 0) := by
        funext k
        have l : (M *ᵥ y1) k = M k c := by simp [y1, mulVec, dotProduct, Pi.single_apply]
        rw [l, hcolM k]
        simp only [mulVec, dotProduct]
        rw [show (∑ i, M k i * (if i ∈ A then μ i else 0)) = ∑ i, (if i ∈ A then μ i * M k i else 0)
          from Finset.sum_congr rfl fun i _ => by split_ifs <;> ring]
        rw [Finset.sum_ite_mem, Finset.univ_inter]
      rw [hsplit, mulVec_add, mulVec_sub, mulVec_sub, ← hMy1, e1, e0]
      ext k; simp only [Pi.add_apply, Pi.sub_apply]; ring
    have hvA : ∀ k ∈ A, (M *ᵥ v) k = 0 := by
      intro k hk
      rw [hMv]
      have hkc : k ≠ c := fun e => hc (e ▸ hk)
      have : y1 k = 0 := by simp [y1, Pi.single_apply, hkc]
      simp [wOf, hk, this]
    have hv0 := hinjA v (fun i hi => by simp [v, hi]) hvA
    refine ⟨fun i hi => ?_, fun i hi => ?_⟩
    · have := congrFun hv0 i
      simp only [v, if_pos hi, Pi.zero_apply] at this
      linarith
    · have hw : wOf A x1 y1 - wOf A (ρ θ₀) 0 = 0 := by rw [← hMv, hv0, mulVec_zero]
      have := congrFun hw i
      simp only [wOf, if_neg hi, Pi.sub_apply, Pi.zero_apply, x1, Pi.add_apply, hTcol] at this
      linarith
  refine ⟨htab, hc, fun k => ?_, fun i => ?_⟩
  · have := rowlin (Pi.single k 1)
    have lhs : (T *ᵥ Pi.single k 1) c = T c k := by simp [mulVec, dotProduct, Pi.single_apply]
    rw [lhs] at this
    rw [this]
    by_cases hk : k ∈ A
    · rw [if_pos hk, Finset.sum_eq_single k (fun j _ hjk => by simp [Pi.single_apply, hjk])
        (fun h => absurd hk h)]
      simp
    · rw [if_neg hk]
      exact Finset.sum_eq_zero fun j hj => by
        have : j ≠ k := fun e => hk (e ▸ hj)
        simp [Pi.single_apply, this]
  · by_cases hi : i ∈ A
    · rw [if_pos hi, colfacts.1 i hi]
    · rw [if_neg hi, colfacts.2 i hi]

/-- The facet function `D = -ρ_c`. -/
def D (ρ : Θ → ι → ℝ) (c : ι) (θ : Θ) : ℝ := -ρ θ c

/-- **The identity** `w_c = -D + Σ_{j ∈ A} μ_j w_j` for every solution of `w = r + M z`. -/
theorem identity (H : DepFacet M r A ρ T c μ) {θ : Θ} {w z : ι → ℝ} (h : w = r θ + M *ᵥ z) :
    w c = ρ θ c + ∑ j ∈ A, μ j * w j := by
  have e := congrFun ((H.tab θ w z).mp h) c
  have hx : xB A w z c = w c := by simp [xB, H.c_out]
  rw [hx] at e
  rw [e, Pi.add_apply]
  congr 1
  simp only [mulVec, dotProduct, H.row_c]
  have hterm : ∀ k, (if k ∈ A then μ k else 0) * yN A w z k = if k ∈ A then μ k * w k else 0 := by
    intro k; by_cases hk : k ∈ A <;> simp [hk, yN]
  simp only [hterm]
  rw [Finset.sum_ite_mem, Finset.univ_inter]

/-- **`P⁻` empty: nothing across.** With every `μ_j ≤ 0` and `D > 0`, no `z` makes all slacks
nonnegative (so there is no pLCP solution and no optimizer at that parameter). -/
theorem no_solution_of_empty (H : DepFacet M r A ρ T c μ) (hμ : ∀ j ∈ A, μ j ≤ 0) {θ : Θ}
    (hD : 0 < D ρ c θ) : ¬ ∃ z : ι → ℝ, ∀ i, 0 ≤ (r θ + M *ᵥ z) i := by
  rintro ⟨z, hz⟩
  have e := H.identity (w := r θ + M *ᵥ z) (z := z) rfl
  have hneg : ∑ j ∈ A, μ j * (r θ + M *ᵥ z) j ≤ 0 :=
    Finset.sum_nonpos fun j hj => mul_nonpos_of_nonpos_of_nonneg (hμ j hj) (hz j)
  have := hz c
  simp only [D] at hD
  linarith

/-- **Slack side.** A solution whose slacks vanish on `T \ {j}` (that is on `A \ {j}` and at `c`)
has `μ_j w_j = D`. Across `F` (`D > 0`) a nonnegative `w_j` then needs `μ_j > 0`. -/
theorem slack_side (H : DepFacet M r A ρ T c μ) {θ : Θ} {w z : ι → ℝ} (h : w = r θ + M *ᵥ z)
    {j : ι} (hj : j ∈ A) (hzero : ∀ i ∈ A, i ≠ j → w i = 0) (hc : w c = 0) :
    μ j * w j = D ρ c θ := by
  have e := H.identity h
  have hs : ∑ i ∈ A, μ i * w i = μ j * w j :=
    Finset.sum_eq_single j (fun i hi hij => by rw [hzero i hi hij, mul_zero])
      (fun h' => absurd hj h')
  rw [hs, hc] at e
  simp only [D]
  linarith

/-- Across `F` the candidate `T \ {j}` needs `j ∈ P⁻`. -/
theorem needs_pos (H : DepFacet M r A ρ T c μ) {θ : Θ} {w z : ι → ℝ} (h : w = r θ + M *ᵥ z)
    {j : ι} (hj : j ∈ A) (hzero : ∀ i ∈ A, i ≠ j → w i = 0) (hc : w c = 0) (hwj : 0 ≤ w j)
    (hD : 0 < D ρ c θ) : 0 < μ j := by
  have := H.slack_side h hj hzero hc
  by_contra hle
  push_neg at hle
  nlinarith [mul_nonpos_of_nonpos_of_nonneg hle hwj]

/-! ### The candidate `B_j = T \ {j}` built from `R`'s tableau -/

/-- `w_j = D / μ_j`. -/
noncomputable def s (ρ : Θ → ι → ℝ) (c : ι) (μ : ι → ℝ) (j : ι) (θ : Θ) : ℝ := D ρ c θ / μ j

/-- `z_c = (ρ_j + T_jj s) / μ_j`. -/
noncomputable def t (ρ : Θ → ι → ℝ) (T : Matrix ι ι ℝ) (c : ι) (μ : ι → ℝ) (j : ι) (θ : Θ) : ℝ :=
  (ρ θ j + T j j * s ρ c μ j θ) / μ j

/-- Nonbasic values of `R`'s tableau for the candidate: `w_j = s`, `z_c = t`, others 0. -/
noncomputable def y (ρ : Θ → ι → ℝ) (T : Matrix ι ι ℝ) (c : ι) (μ : ι → ℝ) (j : ι) (θ : Θ) :
    ι → ℝ := Pi.single j (s ρ c μ j θ) + Pi.single c (t ρ T c μ j θ)

/-- Basic values of `R`'s tableau for the candidate. -/
noncomputable def x (ρ : Θ → ι → ℝ) (T : Matrix ι ι ℝ) (c : ι) (μ : ι → ℝ) (j : ι) (θ : Θ) :
    ι → ℝ := ρ θ + T *ᵥ y ρ T c μ j θ

theorem x_apply (ρ : Θ → ι → ℝ) (T : Matrix ι ι ℝ) (c : ι) (μ : ι → ℝ) (j : ι) (θ : Θ) (i : ι) :
    x ρ T c μ j θ i = ρ θ i + T i j * s ρ c μ j θ + T i c * t ρ T c μ j θ := by
  simp only [x, y, mulVec_add, Pi.add_apply]
  simp [mulVec, dotProduct, Pi.single_apply]
  ring

/-- The candidate's slacks and multipliers. -/
noncomputable def wj (A : Finset ι) (ρ : Θ → ι → ℝ) (T : Matrix ι ι ℝ) (c : ι) (μ : ι → ℝ)
    (j : ι) (θ : Θ) : ι → ℝ := wOf A (x ρ T c μ j θ) (y ρ T c μ j θ)

noncomputable def zj (A : Finset ι) (ρ : Θ → ι → ℝ) (T : Matrix ι ι ℝ) (c : ι) (μ : ι → ℝ)
    (j : ι) (θ : Θ) : ι → ℝ := zOf A (x ρ T c μ j θ) (y ρ T c μ j θ)

/-- The candidate basis `T \ {j} = (A \ {j}) ∪ {c}`. -/
def Bj (A : Finset ι) (c j : ι) : Finset ι := insert c (A.erase j)

/-- Where the candidate's basic solution is valid. -/
def CRj (ρ : Θ → ι → ℝ) (T : Matrix ι ι ℝ) (c : ι) (μ : ι → ℝ) (j : ι) : Set Θ :=
  {θ | 0 ≤ s ρ c μ j θ ∧ 0 ≤ t ρ T c μ j θ ∧ ∀ i, i ≠ j → i ≠ c → 0 ≤ x ρ T c μ j θ i}

section Candidate

variable (H : DepFacet M r A ρ T c μ) {j : ι} (hj : j ∈ A) (hμj : μ j ≠ 0)
include H hj hμj

theorem hjc : j ≠ c := fun e => H.c_out (e ▸ hj)

theorem x_c (θ : Θ) : x ρ T c μ j θ c = 0 := by
  rw [x_apply, H.row_c j, if_pos hj, H.row_c c, if_neg H.c_out, zero_mul, add_zero]
  simp only [s, D]
  field_simp
  ring

theorem x_j (θ : Θ) : x ρ T c μ j θ j = 0 := by
  rw [x_apply, H.col_c j, if_pos hj]
  simp only [t]
  field_simp
  ring

theorem y_apply (θ : Θ) (i : ι) :
    y ρ T c μ j θ i = if i = j then s ρ c μ j θ else if i = c then t ρ T c μ j θ else 0 := by
  have hjc := H.hjc hj hμj
  by_cases h1 : i = j
  · subst h1; simp [y, Pi.single_apply, hjc]
  · by_cases h2 : i = c
    · subst h2; simp [y, Pi.single_apply, h1]
    · simp [y, Pi.single_apply, h1, h2]

/-- **The candidate solves the pLCP on its validity region**, with support `T \ {j}`. -/
theorem candidate_solves {θ : Θ} (hθ : θ ∈ CRj ρ T c μ j) :
    LCPSol M (r θ) (wj A ρ T c μ j θ) (zj A ρ T c μ j θ) := by
  obtain ⟨hs, ht, hx⟩ := hθ
  have hjc := H.hjc hj hμj
  refine ⟨?_, fun i => ?_, fun i => ?_, fun i => ?_⟩
  · rw [H.tab θ]; simp [wj, zj, x]
  · simp only [wj, wOf]
    split_ifs with hi
    · rw [H.y_apply hj hμj]
      by_cases h1 : i = j
      · simp [h1, hs]
      · have : i ≠ c := fun e => H.c_out (e ▸ hi)
        simp [h1, this]
    · by_cases h2 : i = c
      · rw [h2, H.x_c hj hμj]
      · exact hx i (fun e => hi (e ▸ hj)) h2
  · simp only [zj, zOf]
    split_ifs with hi
    · by_cases h1 : i = j
      · rw [h1, H.x_j hj hμj]
      · exact hx i h1 (fun e => H.c_out (e ▸ hi))
    · rw [H.y_apply hj hμj]
      have h1 : i ≠ j := fun e => hi (e ▸ hj)
      by_cases h2 : i = c
      · simp [h1, h2, ht, Ne.symm hjc]
      · simp [h1, h2]
  · simp only [wj, zj, wOf, zOf]
    split_ifs with hi
    · rw [H.y_apply hj hμj]
      by_cases h1 : i = j
      · rw [h1, H.x_j hj hμj, mul_zero]
      · have : i ≠ c := fun e => H.c_out (e ▸ hi)
        simp [h1, this]
    · rw [H.y_apply hj hμj]
      have h1 : i ≠ j := fun e => hi (e ▸ hj)
      by_cases h2 : i = c
      · rw [h2, H.x_c hj hμj, zero_mul]
      · simp [h1, h2]

/-- Support of the candidate: multipliers vanish off `T \ {j}`, slacks vanish on it. -/
theorem candidate_support (θ : Θ) (i : ι) :
    (i ∉ Bj A c j → zj A ρ T c μ j θ i = 0) ∧ (i ∈ Bj A c j → wj A ρ T c μ j θ i = 0) := by
  have hjc := H.hjc hj hμj
  constructor
  · intro hi
    simp only [Bj, Finset.mem_insert, Finset.mem_erase, not_or, not_and_or, not_not] at hi
    obtain ⟨hic, hij⟩ := hi
    simp only [zj, zOf]
    split_ifs with hA
    · rcases hij with h | h
      · rw [h, H.x_j hj hμj]
      · exact absurd hA h
    · rw [H.y_apply hj hμj]
      have : i ≠ j := fun e => hA (e ▸ hj)
      simp [this, hic]
  · intro hi
    simp only [Bj, Finset.mem_insert, Finset.mem_erase] at hi
    simp only [wj, wOf]
    rcases hi with h | ⟨hij, hA⟩
    · rw [h, if_neg H.c_out, H.x_c hj hμj]
    · rw [if_pos hA, H.y_apply hj hμj]
      have : i ≠ c := fun e => H.c_out (e ▸ hA)
      simp [hij, this]

/-- Strict positivity of the candidate's basic variables. -/
theorem candidate_pos {θ : Θ} (hs : 0 < s ρ c μ j θ) (ht : 0 < t ρ T c μ j θ)
    (hx : ∀ i, i ≠ j → i ≠ c → 0 < x ρ T c μ j θ i) (i : ι) :
    (i ∈ Bj A c j → 0 < zj A ρ T c μ j θ i) ∧ (i ∉ Bj A c j → 0 < wj A ρ T c μ j θ i) := by
  have hjc := H.hjc hj hμj
  constructor
  · intro hi
    simp only [Bj, Finset.mem_insert, Finset.mem_erase] at hi
    simp only [zj, zOf]
    rcases hi with h | ⟨hij, hA⟩
    · rw [h, if_neg H.c_out, H.y_apply hj hμj]; simp [Ne.symm hjc, ht]
    · rw [if_pos hA]; exact hx i hij (fun e => H.c_out (e ▸ hA))
  · intro hi
    simp only [Bj, Finset.mem_insert, Finset.mem_erase, not_or, not_and_or, not_not] at hi
    obtain ⟨hic, hij⟩ := hi
    simp only [wj, wOf]
    split_ifs with hA
    · rcases hij with h | h
      · rw [h, H.y_apply hj hμj]; simp [hs]
      · exact absurd hA h
    · exact hx i (fun e => hA (e ▸ hj)) hic

end Candidate

/-! ### (c) The piece of `F` that `B_j` borders -/

/-- **(c)** On the facet (`D = 0`, inside `R`'s region), the candidate `T \ {j}` is valid exactly
where `j` attains `min_{k ∈ P⁻} λ_k / μ_k` (`λ = ρ` on `A`). -/
theorem on_facet (H : DepFacet M r A ρ T c μ) {j : ι} (hj : j ∈ A) (hμj : 0 < μ j) {θ : Θ}
    (hD : D ρ c θ = 0) (hR : ∀ i, 0 ≤ ρ θ i) :
    θ ∈ CRj ρ T c μ j ↔ ∀ i ∈ A, 0 < μ i → ρ θ j / μ j ≤ ρ θ i / μ i := by
  have hs : s ρ c μ j θ = 0 := by simp [s, hD]
  have ht : t ρ T c μ j θ = ρ θ j / μ j := by simp [t, hs]
  have hxA : ∀ i ∈ A, x ρ T c μ j θ i = ρ θ i - μ i * (ρ θ j / μ j) := by
    intro i hi; rw [x_apply, hs, ht, H.col_c i, if_pos hi]; ring
  have hxN : ∀ i, i ∉ A → x ρ T c μ j θ i = ρ θ i := by
    intro i hi; rw [x_apply, hs, ht, H.col_c i, if_neg hi]; ring
  constructor
  · rintro ⟨-, -, hx⟩ i hi hμi
    by_cases hij : i = j
    · rw [hij]
    · have := hx i hij (fun e => H.c_out (e ▸ hi))
      rw [hxA i hi] at this
      rw [div_le_div_iff₀ hμj hμi]
      have h2 : μ i * (ρ θ j / μ j) ≤ ρ θ i := by linarith
      rw [mul_div_assoc', div_le_iff₀ hμj] at h2
      linarith
  · intro hmin
    have htn : 0 ≤ ρ θ j / μ j := div_nonneg (hR j) hμj.le
    refine ⟨hs.ge, ht ▸ htn, fun i hij hic => ?_⟩
    by_cases hi : i ∈ A
    · rw [hxA i hi]
      rcases lt_or_ge 0 (μ i) with hμi | hμi
      · have := hmin i hi hμi
        rw [div_le_div_iff₀ hμj hμi] at this
        have : μ i * (ρ θ j / μ j) ≤ ρ θ i := by
          rw [mul_div_assoc', div_le_iff₀ hμj]; linarith
        linarith
      · have : μ i * (ρ θ j / μ j) ≤ 0 := mul_nonpos_of_nonpos_of_nonneg hμi htn
        linarith [hR i]
    · rw [hxN i hi]; exact hR i

/-! ### (a), (b) Just across a relative-interior point -/

variable [TopologicalSpace Θ]

/-- **(a)** At a relative-interior point `θ₀` of the facet where `j ∈ P⁻` is the strict argmin,
the candidate's basic variables other than `w_j` stay positive nearby, so `T \ {j}` is valid on
the far side (`D ≥ 0`) and strictly positive across (`D > 0`). -/
theorem across (H : DepFacet M r A ρ T c μ) (hcont : ∀ i, Continuous fun θ => ρ θ i) {j : ι}
    (hj : j ∈ A) (hμj : 0 < μ j) {θ₀ : Θ} (hD₀ : D ρ c θ₀ = 0)
    (hrelA : ∀ i ∈ A, 0 < ρ θ₀ i) (hrelN : ∀ i, i ∉ A → i ≠ c → 0 < ρ θ₀ i)
    (hmin : ∀ i ∈ A, i ≠ j → 0 < μ i → ρ θ₀ j / μ j < ρ θ₀ i / μ i) :
    ∀ᶠ θ in 𝓝 θ₀, 0 < t ρ T c μ j θ ∧ ∀ i, i ≠ j → i ≠ c → 0 < x ρ T c μ j θ i := by
  have cs : Continuous fun θ => s ρ c μ j θ := by
    simp only [s, D]; exact (hcont c).neg.div_const _
  have ct : Continuous fun θ => t ρ T c μ j θ := by
    simp only [t]; exact ((hcont j).add (continuous_const.mul cs)).div_const _
  have cx : ∀ i, Continuous fun θ => x ρ T c μ j θ i := by
    intro i
    have : (fun θ => x ρ T c μ j θ i) =
        fun θ => ρ θ i + T i j * s ρ c μ j θ + T i c * t ρ T c μ j θ := by
      funext θ; exact x_apply ρ T c μ j θ i
    rw [this]
    exact ((hcont i).add (continuous_const.mul cs)).add (continuous_const.mul ct)
  have hs0 : s ρ c μ j θ₀ = 0 := by simp [s, hD₀]
  have ht0 : t ρ T c μ j θ₀ = ρ θ₀ j / μ j := by simp [t, hs0]
  have htpos : 0 < t ρ T c μ j θ₀ := by rw [ht0]; exact div_pos (hrelA j hj) hμj
  have hxpos : ∀ i, i ≠ j → i ≠ c → 0 < x ρ T c μ j θ₀ i := by
    intro i hij hic
    rw [x_apply, hs0, ht0, H.col_c i]
    split_ifs with hi
    · rcases lt_or_ge 0 (μ i) with hμi | hμi
      · have := hmin i hi hij hμi
        rw [div_lt_div_iff₀ hμj hμi] at this
        have : μ i * (ρ θ₀ j / μ j) < ρ θ₀ i := by
          rw [mul_div_assoc', div_lt_iff₀ hμj]; linarith
        linarith
      · have : μ i * (ρ θ₀ j / μ j) ≤ 0 :=
          mul_nonpos_of_nonpos_of_nonneg hμi (div_nonneg (hrelA j hj).le hμj.le)
        linarith [hrelA i hi]
    · simp only [mul_zero, zero_mul, add_zero]; exact hrelN i hi hic
  have e1 := (ct.continuousAt (x := θ₀)).eventually (lt_mem_nhds htpos)
  have e2 : ∀ᶠ θ in 𝓝 θ₀, ∀ i, i ≠ j → i ≠ c → 0 < x ρ T c μ j θ i := by
    refine eventually_all.mpr fun i => ?_
    by_cases h : i ≠ j ∧ i ≠ c
    · filter_upwards [((cx i).continuousAt (x := θ₀)).eventually (lt_mem_nhds (hxpos i h.1 h.2))]
        with θ hθ _ _ using hθ
    · exact Eventually.of_forall fun θ h1 h2 => absurd ⟨h1, h2⟩ h
  filter_upwards [e1, e2] with θ h1 h2 using ⟨h1, h2⟩

/-- **(a), (b) Uniqueness just across.** With `M` symmetric positive semidefinite and the
columns of `T \ {j}` independent (`hinj`), every complementary basis that is valid at a point
just across `F` near `θ₀` is `T \ {j}`. -/
theorem unique_across (H : DepFacet M r A ρ T c μ) (hM : M.PosSemidef)
    (hcont : ∀ i, Continuous fun θ => ρ θ i) {j : ι} (hj : j ∈ A) (hμj : 0 < μ j) {θ₀ : Θ}
    (hD₀ : D ρ c θ₀ = 0) (hrelA : ∀ i ∈ A, 0 < ρ θ₀ i)
    (hrelN : ∀ i, i ∉ A → i ≠ c → 0 < ρ θ₀ i)
    (hmin : ∀ i ∈ A, i ≠ j → 0 < μ i → ρ θ₀ j / μ j < ρ θ₀ i / μ i)
    (hinj : ∀ u : ι → ℝ, (∀ i, i ∉ Bj A c j → u i = 0) → M *ᵥ u = 0 → u = 0) :
    ∀ᶠ θ in 𝓝 θ₀, 0 < D ρ c θ → ∀ (C : Finset ι) (ρC : ι → ℝ) (TC : Matrix ι ι ℝ),
      IsTableau M (r θ) C ρC TC → (∀ i, 0 ≤ ρC i) → C = Bj A c j := by
  have hμ0 : μ j ≠ 0 := hμj.ne'
  filter_upwards [H.across hcont hj hμj hD₀ hrelA hrelN hmin] with θ ⟨ht, hx⟩ hD C ρC TC hC hρC
  have hs : 0 < s ρ c μ j θ := div_pos hD hμj
  have sol := H.candidate_solves hj hμ0 (θ := θ) ⟨hs.le, ht.le, fun i h1 h2 => (hx i h1 h2).le⟩
  have pos := H.candidate_pos hj hμ0 hs ht hx
  have supp := H.candidate_support hj hμ0 θ
  have solC := basic_solves hC hρC
  have hw := lcp_w_unique hM sol solC
  set w := wj A ρ T c μ j θ
  set z := zj A ρ T c μ j θ
  -- outside T \ {j}: w > 0, so the slack is basic in C and C's multiplier vanishes
  have out : ∀ i, i ∉ Bj A c j → i ∉ C ∧ basicZ C ρC i = 0 := by
    intro i hi
    have hwpos : 0 < basicW C ρC i := by rw [← hw]; exact (pos i).2 hi
    refine ⟨fun hiC => ?_, ?_⟩
    · have : basicW C ρC i = 0 := by simp [basicW, wOf, hiC]
      linarith
    · have := solC.comp i
      rcases mul_eq_zero.mp this with h | h
      · linarith
      · exact h
  -- the multipliers agree
  have hz : z = basicZ C ρC := by
    have hu := hinj (z - basicZ C ρC) (fun i hi => by
      simp only [Pi.sub_apply, (supp i).1 hi, (out i hi).2, sub_self]) (by
      rw [mulVec_sub]
      have e1 := sol.eq
      have e2 := solC.eq
      have : M *ᵥ z = M *ᵥ basicZ C ρC := by
        have h1 : M *ᵥ z = w - r θ := by rw [e1]; abel
        have h2 : M *ᵥ basicZ C ρC = basicW C ρC - r θ := by rw [e2]; abel
        rw [h1, h2, hw]
      rw [this, sub_self])
    exact sub_eq_zero.mp hu
  ext i
  constructor
  · intro hiC
    by_contra hi
    exact (out i hi).1 hiC
  · intro hi
    by_contra hiC
    have h1 : basicZ C ρC i = 0 := by simp [basicZ, zOf, hiC]
    have h2 : 0 < z i := (pos i).1 hi
    rw [hz, h1] at h2
    exact lt_irrefl 0 h2

/-- **(b)** Hence every basis valid just across `F` at such a point lies in
`S = {T \ {j} : j ∈ P⁻}`. -/
theorem returned_mem_S (H : DepFacet M r A ρ T c μ) (hM : M.PosSemidef)
    (hcont : ∀ i, Continuous fun θ => ρ θ i) {j : ι} (hj : j ∈ A) (hμj : 0 < μ j) {θ₀ : Θ}
    (hD₀ : D ρ c θ₀ = 0) (hrelA : ∀ i ∈ A, 0 < ρ θ₀ i)
    (hrelN : ∀ i, i ∉ A → i ≠ c → 0 < ρ θ₀ i)
    (hmin : ∀ i ∈ A, i ≠ j → 0 < μ i → ρ θ₀ j / μ j < ρ θ₀ i / μ i)
    (hinj : ∀ u : ι → ℝ, (∀ i, i ∉ Bj A c j → u i = 0) → M *ᵥ u = 0 → u = 0) :
    ∀ᶠ θ in 𝓝 θ₀, 0 < D ρ c θ → ∀ (C : Finset ι) (ρC : ι → ℝ) (TC : Matrix ι ι ℝ),
      IsTableau M (r θ) C ρC TC → (∀ i, 0 ≤ ρC i) →
        C ∈ (A.filter fun k => 0 < μ k).image (Bj A c) := by
  filter_upwards [H.unique_across hM hcont hj hμj hD₀ hrelA hrelN hmin hinj] with θ h hD C ρC TC
    hC hρC
  rw [h hD C ρC TC hC hρC]
  exact Finset.mem_image.mpr ⟨j, Finset.mem_filter.mpr ⟨hj, hμj⟩, rfl⟩

end DepFacet

end Rpopt
