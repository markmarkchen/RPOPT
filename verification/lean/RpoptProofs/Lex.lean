/-
Lexicographic perturbation (mplcp.rs header, `lex_cmp`, `lex_nonneg`, `rfull` in `pivot_facet`,
and `lex_feasible` of commit 88f948f).

The right-hand side `q + Rθ` is replaced by `q + Rθ + ε⃗` with `ε⃗_i = ε^(i+1)`. The value of a
basic variable is then a polynomial in `ε` whose coefficient vector is the *lexicographic row*
`[ρ_i | β_i]`; `ε` is never given a numeric value, rows are compared lexicographically.

* `LexPos`, `evalPoly`, `LexPos.eventually_pos`: a lexico-positive row is a positive number for
  every small enough `ε > 0` (this is what "symbolic ε" means).
* `lex_ratio_pivot`: the lexicographic ratio test keeps every row lexico-positive.
* `lexRows`, `isLexTableau_of_isTableau`: the perturbation block is determined by `T`
  (`rfull`: column j of β is `-T[:, j]` when `w_j` is nonbasic, the unit vector of its row when
  `w_j` is basic), and `lexRows_pivot`: the pivot transforms it like any other column.
* `lex_unique` (**A5**): two different lexico-feasible complementary bases are never valid at the
  same `θ`, so the lexicographic regions do not overlap.
-/
import RpoptProofs.Pivot

open Matrix Filter Topology

set_option linter.unusedSectionVars false

namespace Rpopt

section LexOrder

variable {N : ℕ}

/-- Lexico-positive: the first nonzero entry is positive. -/
def LexPos (u : Fin N → ℝ) : Prop := ∃ k : Fin N, (∀ j : Fin N, j < k → u j = 0) ∧ 0 < u k

/-- Lexico-nonnegative: zero or lexico-positive (Rust `lex_nonneg`, without tolerance). -/
def LexNonneg (u : Fin N → ℝ) : Prop := u = 0 ∨ LexPos u

/-- The polynomial `Σ_j u_j ε^j` of a lexicographic row. -/
noncomputable def evalPoly (u : Fin N → ℝ) (ε : ℝ) : ℝ := ∑ j, u j * ε ^ (j : ℕ)

theorem LexPos.smul {u : Fin N → ℝ} (h : LexPos u) {a : ℝ} (ha : 0 < a) : LexPos (a • u) := by
  obtain ⟨k, hk, hpos⟩ := h
  exact ⟨k, fun j hj => by simp [hk j hj], by simpa using mul_pos ha hpos⟩

theorem LexNonneg.smul {u : Fin N → ℝ} (h : LexNonneg u) {a : ℝ} (ha : 0 ≤ a) :
    LexNonneg (a • u) := by
  rcases h with rfl | h
  · left; simp
  · rcases ha.lt_or_eq with ha | rfl
    · exact Or.inr (h.smul ha)
    · left; simp

theorem LexPos.add {u v : Fin N → ℝ} (hu : LexPos u) (hv : LexNonneg v) : LexPos (u + v) := by
  rcases hv with rfl | ⟨kv, hkv, hv⟩
  · simpa using hu
  obtain ⟨ku, hku, hu⟩ := hu
  rcases lt_trichotomy ku kv with h | rfl | h
  · refine ⟨ku, fun j hj => ?_, ?_⟩
    · simp [hku j hj, hkv j (hj.trans h)]
    · simp [hkv ku h, hu]
  · exact ⟨ku, fun j hj => by simp [hku j hj, hkv j hj], by simp; linarith⟩
  · refine ⟨kv, fun j hj => ?_, ?_⟩
    · simp [hkv j hj, hku j (hj.trans h)]
    · simp [hku kv h, hv]

theorem LexPos.ne_zero {u : Fin N → ℝ} (h : LexPos u) : u ≠ 0 := by
  rintro rfl; obtain ⟨k, -, hk⟩ := h; simp at hk

/-- A nonzero row is lexico-positive or lexico-negative. -/
theorem lexPos_or_neg {u : Fin N → ℝ} (h : u ≠ 0) : LexPos u ∨ LexPos (-u) := by
  classical
  have hne : (Finset.univ.filter fun j => u j ≠ 0).Nonempty := by
    by_contra hc
    apply h; funext j
    by_contra hj
    exact hc ⟨j, Finset.mem_filter.mpr ⟨Finset.mem_univ _, fun h0 => hj (by simp [h0])⟩⟩
  set k := (Finset.univ.filter fun j => u j ≠ 0).min' hne
  have hk : u k ≠ 0 := by
    exact (Finset.mem_filter.mp (Finset.min'_mem _ hne)).2
  have hbelow : ∀ j, j < k → u j = 0 := by
    intro j hj
    by_contra hj0
    have := Finset.min'_le (Finset.univ.filter fun j => u j ≠ 0) j
      (Finset.mem_filter.mpr ⟨Finset.mem_univ _, hj0⟩)
    exact absurd hj (not_lt.mpr this)
  rcases hk.lt_or_gt with hneg | hpos
  · right; exact ⟨k, fun j hj => by simp [hbelow j hj], by simpa using hneg⟩
  · left; exact ⟨k, hbelow, hpos⟩

/-- A row is not both lexico-positive and lexico-negative. -/
theorem not_lexPos_and_neg {u : Fin N → ℝ} (h : LexPos u) : ¬ LexPos (-u) := by
  rintro ⟨k₂, hk₂, h₂⟩
  obtain ⟨k₁, hk₁, h₁⟩ := h
  rcases lt_trichotomy k₁ k₂ with h | rfl | h
  · have := hk₂ k₁ h; simp at this; linarith
  · simp at h₂; linarith
  · have := hk₁ k₂ h; simp [this] at h₂

theorem lexPos_succ_iff {N : ℕ} (u : Fin (N + 1) → ℝ) :
    LexPos u ↔ 0 < u 0 ∨ (u 0 = 0 ∧ LexPos (Fin.tail u)) := by
  constructor
  · rintro ⟨k, hk, hpos⟩
    refine Fin.cases (motive := fun k => (∀ j, j < k → u j = 0) → 0 < u k →
      0 < u 0 ∨ (u 0 = 0 ∧ LexPos (Fin.tail u))) (fun _ h => Or.inl h)
      (fun k' hk' hpos' => Or.inr ⟨hk' 0 (Fin.succ_pos k'), k', fun j hj => ?_, hpos'⟩) k hk hpos
    exact hk' j.succ (Fin.succ_lt_succ_iff.mpr hj)
  · rintro (h | ⟨h0, k, hk, hpos⟩)
    · exact ⟨0, fun j hj => absurd hj (Fin.not_lt_zero j), h⟩
    · refine ⟨k.succ, fun j hj => ?_, hpos⟩
      refine Fin.cases (motive := fun j => j < k.succ → u j = 0) (fun _ => h0)
        (fun j' hj' => hk j' (Fin.succ_lt_succ_iff.mp hj')) j hj

theorem lexPos_cons_pos {N : ℕ} {a : ℝ} (ha : 0 < a) (t : Fin N → ℝ) :
    LexPos (Fin.cons a t : Fin (N + 1) → ℝ) :=
  (lexPos_succ_iff _).mpr (Or.inl (by simpa using ha))

theorem lexPos_cons_zero {N : ℕ} (t : Fin N → ℝ) :
    LexPos (Fin.cons 0 t : Fin (N + 1) → ℝ) ↔ LexPos t := by
  rw [lexPos_succ_iff]; simp

theorem evalPoly_succ {N : ℕ} (u : Fin (N + 1) → ℝ) (ε : ℝ) :
    evalPoly u ε = u 0 + ε * evalPoly (Fin.tail u) ε := by
  simp only [evalPoly, Fin.sum_univ_succ, Fin.val_zero, pow_zero, mul_one, Fin.val_succ,
    Finset.mul_sum, Fin.tail]
  congr 1
  refine Finset.sum_congr rfl fun j _ => ?_
  ring

theorem evalPoly_cons {N : ℕ} (a : ℝ) (t : Fin N → ℝ) (ε : ℝ) :
    evalPoly (Fin.cons a t : Fin (N + 1) → ℝ) ε = a + ε * evalPoly t ε := by
  rw [evalPoly_succ]; simp

theorem continuous_evalPoly (u : Fin N → ℝ) : Continuous (evalPoly u) := by
  unfold evalPoly
  fun_prop

/-- **Symbolic ε.** A lexico-positive row is a positive number for all small `ε > 0`. -/
theorem LexPos.eventually_pos : ∀ {N : ℕ} {u : Fin N → ℝ}, LexPos u →
    ∀ᶠ ε in 𝓝[>] (0 : ℝ), 0 < evalPoly u ε
  | 0, _, ⟨k, _, _⟩ => k.elim0
  | N + 1, u, h => by
    rcases (lexPos_succ_iff u).mp h with h0 | ⟨h0, ht⟩
    · have hc : ContinuousAt (evalPoly u) 0 := (continuous_evalPoly u).continuousAt
      have : evalPoly u 0 = u 0 := by rw [evalPoly_succ]; simp
      have hev : ∀ᶠ ε in 𝓝 (0 : ℝ), 0 < evalPoly u ε :=
        hc.eventually (lt_mem_nhds (by rw [this]; exact h0))
      exact nhdsWithin_le_nhds hev
    · have ih := LexPos.eventually_pos ht
      have hpos : ∀ᶠ ε in 𝓝[>] (0 : ℝ), 0 < ε := self_mem_nhdsWithin
      filter_upwards [ih, hpos] with ε h1 h2
      rw [evalPoly_succ, h0, zero_add]
      exact mul_pos h2 h1

theorem evalPoly_add (u v : Fin N → ℝ) (ε : ℝ) :
    evalPoly (u + v) ε = evalPoly u ε + evalPoly v ε := by
  simp [evalPoly, add_mul, Finset.sum_add_distrib]

theorem evalPoly_smul (a : ℝ) (u : Fin N → ℝ) (ε : ℝ) :
    evalPoly (a • u) ε = a * evalPoly u ε := by
  simp [evalPoly, Finset.mul_sum, mul_assoc]

/-- Evaluation at `ε` as a linear map (so that `pivVec_map` applies). -/
noncomputable def evalPolyL (ε : ℝ) : (Fin N → ℝ) →ₗ[ℝ] ℝ where
  toFun u := evalPoly u ε
  map_add' u v := evalPoly_add u v ε
  map_smul' a u := by simp [evalPoly_smul]

end LexOrder

section RatioTest

variable {ι κ : Type*} [DecidableEq ι] [DecidableEq κ] {N : ℕ}

/-- **Lexicographic ratio test (mplcp Case 1, mplp_lex phase 2).** Entering column `c`; the rows
that decrease are those with `a_i = -T i c > 0`. If every row is lexico-positive and `l` is the
(unique) lexicographic minimum of `L i / a_i` over the decreasing rows, the pivot on `(l, c)`
keeps every row lexico-positive, and the entering variable takes the lexico-positive value
`L l / a_l`. -/
theorem lex_ratio_pivot {T : Matrix ι κ ℝ} {c : κ} {l : ι} (hl : 0 < -T l c)
    {L : ι → Fin N → ℝ} (hL : ∀ i, LexPos (L i))
    (hmin : ∀ i, i ≠ l → 0 < -T i c → LexPos ((-T i c)⁻¹ • L i - (-T l c)⁻¹ • L l)) :
    ∀ i, LexPos (pivVec T L l c i) := by
  intro i
  by_cases hi : i = l
  · subst hi
    simp only [pivVec, if_true]
    exact (hL i).smul (by rw [← inv_neg]; exact inv_pos.mpr hl)
  · simp only [pivVec, if_neg hi]
    have hTl : T l c ≠ 0 := by intro h; rw [h] at hl; simp at hl
    by_cases ha : 0 < -T i c
    · have e : L i - (T i c / T l c) • L l = (-T i c) • ((-T i c)⁻¹ • L i - (-T l c)⁻¹ • L l) := by
        rw [smul_sub, smul_smul, smul_smul, mul_inv_cancel₀ ha.ne', one_smul]
        congr 2
        field_simp
      rw [e]
      exact (hmin i hi ha).smul ha
    · have hc : 0 ≤ -(T i c / T l c) := by
        have : T i c / T l c = (-T i c) / (-T l c) := by rw [neg_div_neg_eq]
        rw [this, ← neg_div]
        exact div_nonneg (by linarith) hl.le
      rw [sub_eq_add_neg, ← neg_smul]
      exact (hL i).add (LexNonneg.smul (Or.inr (hL l)) hc)

/-- A row that vanishes on the facet (numeric part 0) always beats, in the ratio test, a row whose
numeric part is positive (`pivot_facet` Case 1 restricts the lexmin to `P ∩ Z` and compares only
the β parts). -/
theorem zero_row_beats_positive_row {u v : Fin (N + 1) → ℝ} (hu : u 0 = 0) (hv : 0 < v 0)
    {a b : ℝ} (ha : 0 < a) (hb : 0 < b) : LexPos (b⁻¹ • v - a⁻¹ • u) :=
  (lexPos_succ_iff _).mpr (Or.inl (by simp [hu]; exact mul_pos (inv_pos.mpr hb) hv))

end RatioTest

section Uniqueness

variable {n : ℕ}

/-- The Rust perturbation vector: `ε⃗_i = ε^(i+1)`. -/
def pw (ε : ℝ) : Fin n → ℝ := fun i => ε ^ ((i : ℕ) + 1)

/-- Restriction of a vector to the pairs outside `B` (rows whose slack is basic). -/
def offB (B : Finset (Fin n)) (u : Fin n → ℝ) : Fin n → ℝ := fun i => if i ∈ B then 0 else u i

/-- Restriction of a vector to the pairs in `B` (columns whose slack is nonbasic). -/
def onB (B : Finset (Fin n)) (u : Fin n → ℝ) : Fin n → ℝ := fun i => if i ∈ B then u i else 0

/-- Shifting the right-hand side by `u` changes only `ρ`: the new rows are
`ρ + u|_rows of basic w - T u|_columns of nonbasic w`. -/
theorem tableau_shift {M : Matrix (Fin n) (Fin n) ℝ} {r : Fin n → ℝ} {B : Finset (Fin n)}
    {ρ : Fin n → ℝ} {T : Matrix (Fin n) (Fin n) ℝ} (h : IsTableau M r B ρ T) (u : Fin n → ℝ) :
    IsTableau M (r + u) B (ρ + offB B u - T *ᵥ onB B u) T := by
  intro w z
  have hx : xB B (w - u) z = xB B w z - offB B u := by
    funext i; by_cases hi : i ∈ B <;> simp [xB, offB, hi]
  have hy : yN B (w - u) z = yN B w z - onB B u := by
    funext i; by_cases hi : i ∈ B <;> simp [yN, onB, hi]
  rw [show r + u + M *ᵥ z = (r + M *ᵥ z) + u by abel, ← sub_eq_iff_eq_add, h (w - u) z, hx, hy,
    mulVec_sub]
  constructor <;> intro e <;> linear_combination e

/-- The lexicographic rows `[ρ_i | β_i]` of a tableau: `β_ij = -T_ij` when `w_j` is nonbasic
(`j ∈ B`), the unit vector of its row when `w_j` is basic. This is `rfull` in `pivot_facet` and the
vector tested by `lex_feasible` (88f948f). -/
def lexRows (B : Finset (Fin n)) (ρ : Fin n → ℝ) (T : Matrix (Fin n) (Fin n) ℝ) :
    Fin n → Fin (n + 1) → ℝ :=
  fun i => Fin.cons (ρ i) (fun j => if j ∈ B then -T i j else if i = j then 1 else 0)

/-- The rows `L` describe the perturbed tableau for every `ε`. -/
def IsLexTableau (M : Matrix (Fin n) (Fin n) ℝ) (r : Fin n → ℝ) (B : Finset (Fin n))
    (L : Fin n → Fin (n + 1) → ℝ) (T : Matrix (Fin n) (Fin n) ℝ) : Prop :=
  ∀ ε : ℝ, IsTableau M (r + pw ε) B (fun i => evalPoly (L i) ε) T

/-- **The perturbation block is implied by `T`.** Every tableau, with its `lexRows`, is a tableau
of the perturbed system (no β block needs to be stored). -/
theorem isLexTableau_of_isTableau {M : Matrix (Fin n) (Fin n) ℝ} {r : Fin n → ℝ}
    {B : Finset (Fin n)} {ρ : Fin n → ℝ} {T : Matrix (Fin n) (Fin n) ℝ}
    (h : IsTableau M r B ρ T) : IsLexTableau M r B (lexRows B ρ T) T := by
  intro ε
  have e : (fun i => evalPoly (lexRows B ρ T i) ε) = ρ + offB B (pw ε) - T *ᵥ onB B (pw ε) := by
    funext i
    have hterm : ∀ j : Fin n, ε * ((if j ∈ B then -T i j else if i = j then 1 else 0) *
        ε ^ (j : ℕ)) = -(T i j * onB B (pw ε) j) + (if i = j then offB B (pw ε) j else 0) := by
      intro j
      by_cases hj : j ∈ B <;> by_cases hij : i = j <;> simp [hj, hij, onB, offB, pw] <;> ring
    rw [show lexRows B ρ T i = (Fin.cons (ρ i)
      (fun j => if j ∈ B then -T i j else if i = j then 1 else 0) : Fin (n + 1) → ℝ) from rfl,
      evalPoly_cons]
    simp only [evalPoly, Finset.mul_sum, Pi.add_apply, Pi.sub_apply, mulVec, dotProduct]
    rw [Finset.sum_congr rfl (fun j _ => hterm j)]
    simp only [Finset.sum_add_distrib, Finset.sum_neg_distrib, Finset.sum_ite_eq,
      Finset.mem_univ, if_true]
    ring
  rw [e]
  exact tableau_shift h (pw ε)

/-- The pivot transforms the lexicographic rows like the numeric column (`pivVec`). -/
theorem lexRows_pivot {B : Finset (Fin n)} {ρ : Fin n → ℝ} {T : Matrix (Fin n) (Fin n) ℝ}
    {v : Fin n} (hp : T v v ≠ 0) :
    lexRows (toggle B v) (pivVec T ρ v v) (pivT T v v) = pivVec T (lexRows B ρ T) v v := by
  funext i j
  refine Fin.cases ?_ (fun j => ?_) j
  · by_cases hi : i = v <;> simp [lexRows, pivVec, hi]
  · by_cases hjv : j = v
    · subst hjv
      by_cases hvB : j ∈ B <;> by_cases hi : i = j <;>
        simp [lexRows, pivVec, pivT, hi, hvB, mem_toggle_self] <;> field_simp <;> ring
    · have hm : j ∈ toggle B v ↔ j ∈ B := mem_toggle_ne hjv
      by_cases hjB : j ∈ B <;> by_cases hi : i = v
      · subst hi; simp [lexRows, pivVec, pivT, hjB, hm, hjv]; ring
      · simp [lexRows, pivVec, pivT, hjB, hm, hjv, hi]; ring
      · subst hi; simp [lexRows, pivVec, pivT, hjB, hm, hjv, Ne.symm hjv]
      · simp [lexRows, pivVec, pivT, hjB, hm, hjv, hi, Ne.symm hjv]

/-- A basis is lexico-feasible: every lexicographic row is lexico-positive. -/
def LexFeasible (L : Fin n → Fin (n + 1) → ℝ) : Prop := ∀ i, LexPos (L i)

/-- **A5 (lexicographic uniqueness).** For a symmetric positive semidefinite `M` (the mp-QP case,
`lcpM_posSemidef`), two lexico-feasible complementary bases of the same pLCP right-hand side are
equal. Hence the lexicographic regions of different bases are disjoint: no overlaps. -/
theorem lex_unique {M : Matrix (Fin n) (Fin n) ℝ} (hM : M.PosSemidef) {r : Fin n → ℝ}
    {B₁ B₂ : Finset (Fin n)} {L₁ L₂ : Fin n → Fin (n + 1) → ℝ} {T₁ T₂ : Matrix (Fin n) (Fin n) ℝ}
    (h₁ : IsLexTableau M r B₁ L₁ T₁) (h₂ : IsLexTableau M r B₂ L₂ T₂)
    (f₁ : LexFeasible L₁) (f₂ : LexFeasible L₂) : B₁ = B₂ := by
  have e₁ : ∀ᶠ ε in 𝓝[>] (0 : ℝ), ∀ i, 0 < evalPoly (L₁ i) ε :=
    eventually_all.mpr fun i => (f₁ i).eventually_pos
  have e₂ : ∀ᶠ ε in 𝓝[>] (0 : ℝ), ∀ i, 0 < evalPoly (L₂ i) ε :=
    eventually_all.mpr fun i => (f₂ i).eventually_pos
  obtain ⟨ε, hε₁, hε₂⟩ := (e₁.and e₂).exists
  set ρ₁ : Fin n → ℝ := fun i => evalPoly (L₁ i) ε
  set ρ₂ : Fin n → ℝ := fun i => evalPoly (L₂ i) ε
  have s₁ := basic_solves (h₁ ε) (fun i => (hε₁ i).le)
  have s₂ := basic_solves (h₂ ε) (fun i => (hε₂ i).le)
  have hw := lcp_w_unique hM s₁ s₂
  have side : ∀ (B B' : Finset (Fin n)) (ρ ρ' : Fin n → ℝ), (∀ i, 0 < ρ i) →
      basicW B ρ = basicW B' ρ' → ∀ i, i ∉ B → i ∉ B' := by
    intro B B' ρ ρ' hpos hw i hi hi'
    have := congrFun hw i
    simp only [basicW, wOf, hi, hi', if_false, if_true, Pi.zero_apply] at this
    exact (hpos i).ne' this
  ext i
  constructor
  · intro hi; by_contra hi'; exact side B₂ B₁ ρ₂ ρ₁ hε₂ hw.symm i hi' hi
  · intro hi; by_contra hi'; exact side B₁ B₂ ρ₁ ρ₂ hε₁ hw i hi' hi

/-- Lexicographic region of a basis with parametric rows: `{θ : the basis is lexico-feasible}`. -/
def LexRegion {Θ : Type*} (L : Θ → Fin n → Fin (n + 1) → ℝ) : Set Θ := {θ | LexFeasible (L θ)}

/-- **A5 (no overlaps).** Lexicographic regions of different bases are disjoint. -/
theorem lexRegion_disjoint {Θ : Type*} {M : Matrix (Fin n) (Fin n) ℝ} (hM : M.PosSemidef)
    {r : Θ → Fin n → ℝ} {B₁ B₂ : Finset (Fin n)} {L₁ L₂ : Θ → Fin n → Fin (n + 1) → ℝ}
    {T₁ T₂ : Matrix (Fin n) (Fin n) ℝ} (h₁ : ∀ θ, IsLexTableau M (r θ) B₁ (L₁ θ) T₁)
    (h₂ : ∀ θ, IsLexTableau M (r θ) B₂ (L₂ θ) T₂) (hne : B₁ ≠ B₂) :
    Disjoint (LexRegion L₁) (LexRegion L₂) :=
  Set.disjoint_left.mpr fun θ hθ₁ hθ₂ => hne (lex_unique hM (h₁ θ) (h₂ θ) hθ₁ hθ₂)

end Uniqueness

end Rpopt
