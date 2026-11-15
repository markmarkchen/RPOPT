/-
Case 2 of Algorithm 2 (`pivot_facet`) at a facet point, in exact arithmetic.

`adjacent` builds the work tableau (mplcp.rs 593-611): the region tableau with the extra column of
α, the distance across the facet, whose coefficients are `γ_i = ρθ_i · g`. At a facet point
`θ_f` every row is a lexicographic vector `[value at θ_f | β]`. The walk (`Termination.lean`)
pivots with the strict lexicographic ratio rule; pointwise in `θ_f`, Case 1 (rows of `P ∩ Z`,
value 0) and Case 2 (rows with a positive value) are the same rule.

* `first_step_rows`: the first pivot (α enters, a row of `Z` leaves) makes every row except α's
  lexico-positive, even when the start basis is lexico-infeasible on the facet itself.
* `pivVec_num_zero`: α's value on the facet stays 0 through Case-1 steps (zero leaving row) and
  Case-2 steps (α's coefficient 0).
* `terminal_rows`, `ray3`, `ray_lexFeasible` (**soundness**): at the terminal push
  (`P ∩ Z` empty, α basic and increasing, mplcp.rs 404-411), every row of the returned basis is
  lexico-positive in the order `[value | α coefficient | β]`, hence lexico-positive at
  `θ_f + t g` for all small `t > 0`: the returned basis is lexico-feasible just across the facet.
* `case2_lexmin_accepted` (**cover**): at every facet point the lexicographic minimum candidate
  passes all of its lexico-inequalities (16), so the sub-facets of the Case-2 branches cover the
  facet.
* `neighbour_unique_on_ray`: the basis that is lexico-feasible just across `θ_f` is unique.
* `lcp_multipliers_dependent` (**Case 2 needs dependent rows**): two multiplier vectors at one
  `θ` have the same slacks and primal point and differ in `ker Aᵀ` on the active rows.

Named open hypotheses for full completeness (see README): `WalkEndsWithPush` (the walk at a
facet point ends with the terminal push, a Lemke-type existence statement) and the work-tableau
link (the work tableau with α nonbasic is, for `α = t`, the tableau of the complementary basis at
`θ_f + t g`, by construction in `adjacent` and `exchange_iff`).
-/
import RpoptProofs.Termination
import RpoptProofs.Algorithm2

open Matrix Filter Topology

set_option linter.unusedSectionVars false

namespace Rpopt

section Rows

variable {ι κ : Type*} [DecidableEq ι] [DecidableEq κ] {N : ℕ}

theorem LexPos.zero_nonneg {u : Fin (N + 1) → ℝ} (h : LexPos u) : 0 ≤ u 0 := by
  rcases (lexPos_succ_iff u).mp h with h | ⟨h, -⟩
  · exact h.le
  · exact h.ge

theorem LexNonneg.zero_nonneg {u : Fin (N + 1) → ℝ} (h : LexNonneg u) : 0 ≤ u 0 := by
  rcases h with rfl | h
  · simp
  · exact h.zero_nonneg

theorem lexPos_of_zero_pos {u : Fin (N + 1) → ℝ} (h : 0 < u 0) : LexPos u :=
  (lexPos_succ_iff u).mpr (Or.inl h)

theorem pivVec_zero_apply (T : Matrix ι κ ℝ) (L : ι → Fin (N + 1) → ℝ) (l : ι) (c : κ) {i : ι}
    (hil : i ≠ l) : pivVec T L l c i 0 = L i 0 - T i c / T l c * L l 0 := by
  simp [pivVec, hil]

/-- **First pivot.** At a facet point, α (column `c`) enters and the strict lexicographic minimum
row `l` of `P` leaves; `l` vanishes on the facet. If the values are nonnegative, the rows that
vanish on the facet do not increase across it (`γ_i ≤ 0`), and the identically zero rows are
lexico-positive (the basis is lexico-feasible somewhere), then every row except `l` (now α's) is
lexico-positive after the pivot. Rows of `P ∩ Z` with a lexico-negative β are allowed. -/
theorem first_step_rows {T : Matrix ι κ ℝ} {c : κ} {l : ι} (hl : 0 < -T l c)
    {L : ι → Fin (N + 1) → ℝ} (hl0 : L l 0 = 0)
    (hZ : ∀ i, L i 0 = 0 → 0 ≤ -T i c) (hdeg : ∀ i, L i 0 = 0 → T i c = 0 → LexPos (L i))
    (hnum : ∀ i, 0 ≤ L i 0)
    (hmin : ∀ i, i ≠ l → 0 < -T i c → LexPos (ratio T L c i - ratio T L c l)) :
    ∀ i, i ≠ l → LexPos (pivVec T L l c i) := by
  intro i hil
  by_cases ha : 0 < -T i c
  · simp only [pivVec, if_neg hil]
    have e : L i - (T i c / T l c) • L l =
        (-T i c) • ((-T i c)⁻¹ • L i - (-T l c)⁻¹ • L l) := by
      rw [smul_sub, smul_smul, smul_smul, mul_inv_cancel₀ ha.ne', one_smul]
      congr 2
      field_simp
    rw [e]
    exact (hmin i hil ha).smul ha
  · rcases (hnum i).lt_or_eq with hpos | hzero
    · refine lexPos_of_zero_pos ?_
      rw [pivVec_zero_apply T L l c hil, hl0, mul_zero, sub_zero]
      exact hpos
    · have hT : T i c = 0 := by have := hZ i hzero.symm; linarith
      have e : pivVec T L l c i = L i := by simp [pivVec, hil, hT]
      rw [e]
      exact hdeg i hzero.symm hT

/-- α's value on the facet stays 0 (Case 1: the leaving row is zero on the facet; Case 2: α's
coefficient in the entering column is 0). -/
theorem pivVec_num_zero {T : Matrix ι κ ℝ} {L : ι → Fin (N + 1) → ℝ} {l : ι} {c : κ} {i : ι}
    (hil : i ≠ l) (hi : L i 0 = 0) (hcase : L l 0 = 0 ∨ T i c = 0) : pivVec T L l c i 0 = 0 := by
  rw [pivVec_zero_apply T L l c hil, hi]
  rcases hcase with h | h <;> simp [h]

/-- The three-level lexicographic vector `[value | α coefficient | β]` of a row of the returned
basis (α is an infinitesimal larger than every power of ε). -/
def ray3 (u : Fin (N + 1) → ℝ) (a : ℝ) : Fin (N + 2) → ℝ := Fin.cons (u 0) (Fin.cons a (Fin.tail u))

/-- **Terminal soundness.** α is basic in row `ra` with value 0 on the facet and increasing in the
entering column `e` (`T ra e > 0`); no row that vanishes on the facet decreases in `e` (`P ∩ Z`
empty); the other rows are lexico-positive (the walk invariant). Pivoting α out (`z_after`),
every row of the returned complementary basis is lexico-positive in `[value | α coef | β]`. -/
theorem terminal_rows {T : Matrix ι κ ℝ} {e : κ} {ra : ι} (hα : 0 < T ra e)
    {L : ι → Fin (N + 1) → ℝ} (hα0 : L ra 0 = 0) (hrows : ∀ i, i ≠ ra → LexPos (L i))
    (hZ : ∀ i, i ≠ ra → L i 0 = 0 → 0 ≤ T i e) :
    ∀ i, LexPos (ray3 (pivVec T L ra e i) (pivT T ra e i e)) := by
  intro i
  by_cases hi : i = ra
  · subst hi
    have h0 : pivVec T L i e i 0 = 0 := by simp [pivVec, hα0]
    have hc : pivT T i e i e = (T i e)⁻¹ := by simp [pivT]
    rw [ray3, h0, lexPos_cons_zero, hc]
    exact lexPos_cons_pos (inv_pos.mpr hα) _
  · have h0 : pivVec T L ra e i 0 = L i 0 := by
      rw [pivVec_zero_apply T L ra e hi, hα0, mul_zero, sub_zero]
    have hc : pivT T ra e i e = T i e / T ra e := by simp [pivT, hi]
    rcases ((hrows i hi).zero_nonneg).lt_or_eq with hpos | hzero
    · rw [ray3, h0]; exact lexPos_cons_pos hpos _
    · rw [ray3, h0, ← hzero, lexPos_cons_zero, hc]
      rcases (hZ i hi hzero.symm).lt_or_eq with hT | hT
      · exact lexPos_cons_pos (div_pos hT hα) _
      · rw [← hT, zero_div, lexPos_cons_zero]
        have e1 : pivVec T L ra e i = L i := by simp [pivVec, hi, ← hT]
        rw [e1]
        have := hrows i hi
        rw [show L i = Fin.cons (L i 0) (Fin.tail (L i)) by simp, ← hzero, lexPos_cons_zero]
          at this
        exact this

/-- A three-level lexico-positive row is a lexico-positive row at `θ_f + t g` for all small
`t > 0` (value `u₀ + t a`, β unchanged). -/
theorem ray3_eventually {u : Fin (N + 1) → ℝ} {a : ℝ} (h : LexPos (ray3 u a)) :
    ∀ᶠ t in 𝓝[>] (0 : ℝ), LexPos (Fin.cons (u 0 + t * a) (Fin.tail u) : Fin (N + 1) → ℝ) := by
  rcases (lexPos_succ_iff _).mp h with h0 | ⟨h0, h1⟩
  · have hc : Continuous fun t : ℝ => u 0 + t * a := by fun_prop
    have hpos : 0 < u 0 := by simpa [ray3] using h0
    have hev : ∀ᶠ t in 𝓝 (0 : ℝ), 0 < u 0 + t * a :=
      hc.continuousAt.eventually (lt_mem_nhds (by simpa using hpos))
    filter_upwards [hev.filter_mono nhdsWithin_le_nhds] with t ht using lexPos_cons_pos ht _
  · have hu0 : u 0 = 0 := by simpa [ray3] using h0
    have h1' : LexPos (Fin.cons a (Fin.tail u) : Fin (N + 1) → ℝ) := by simpa [ray3] using h1
    rcases (lexPos_succ_iff _).mp h1' with ha | ⟨ha, ht⟩
    · have ha' : 0 < a := by simpa using ha
      filter_upwards [self_mem_nhdsWithin] with t (htpos : 0 < t)
      exact lexPos_cons_pos (by rw [hu0]; exact add_pos_of_nonneg_of_pos le_rfl (mul_pos htpos ha')) _
    · have ha' : a = 0 := by simpa using ha
      have ht' : LexPos (Fin.tail u) := by simpa using ht
      refine Eventually.of_forall fun t => ?_
      rw [hu0, ha', mul_zero, add_zero, lexPos_cons_zero]
      exact ht'

/-- All rows together: the returned basis is lexico-feasible along the ray. -/
theorem ray_lexFeasible {ι' : Type*} [Finite ι'] {u : ι' → Fin (N + 1) → ℝ} {a : ι' → ℝ}
    (h : ∀ i, LexPos (ray3 (u i) (a i))) :
    ∀ᶠ t in 𝓝[>] (0 : ℝ), ∀ i, LexPos (Fin.cons (u i 0 + t * a i) (Fin.tail (u i)) :
      Fin (N + 1) → ℝ) :=
  eventually_all.mpr fun i => ray3_eventually (h i)

end Rows

section Cover

variable {ι : Type*} {N : ℕ}

theorem LexNonneg.add {u v : Fin N → ℝ} (hu : LexNonneg u) (hv : LexNonneg v) :
    LexNonneg (u + v) := by
  rcases hu with rfl | hu
  · simpa using hv
  · exact Or.inr (hu.add hv)

/-- A finite nonempty family of vectors has a lexicographic minimum. -/
theorem exists_lexmin (s : Finset ι) (hs : s.Nonempty) (R : ι → Fin N → ℝ) :
    ∃ l ∈ s, ∀ j ∈ s, LexNonneg (R j - R l) := by
  classical
  induction s using Finset.induction_on with
  | empty => exact absurd hs (by simp)
  | insert a s ha ih =>
    rcases s.eq_empty_or_nonempty with rfl | hne
    · exact ⟨a, by simp, fun j hj => by simp at hj; subst hj; exact Or.inl (sub_self _)⟩
    · obtain ⟨m, hm, hmin⟩ := ih hne
      by_cases hd : LexNonneg (R m - R a)
      · refine ⟨a, Finset.mem_insert_self a s, fun j hj => ?_⟩
        rcases Finset.mem_insert.mp hj with rfl | hj
        · exact Or.inl (sub_self _)
        · have := (hmin j hj).add hd
          rwa [sub_add_sub_cancel] at this
      · have hneg : LexPos (R a - R m) := by
          have hne0 : R m - R a ≠ 0 := fun h0 => hd (Or.inl h0)
          rcases lexPos_or_neg hne0 with h | h
          · exact absurd (Or.inr h) hd
          · rwa [neg_sub] at h
        refine ⟨m, Finset.mem_insert_of_mem hm, fun j hj => ?_⟩
        rcases Finset.mem_insert.mp hj with rfl | hj
        · exact Or.inr hneg
        · exact hmin j hj

/-- **Case-2 cover.** At a facet point, among the decreasing candidates `s` (`a > 0`), the
lexicographic minimum ratio row `l` passes all its lexico-inequalities (16): every `Γ_j` is
lexico-nonnegative, in particular its numeric part, the inequality that `pivot_facet` adds to the
sub-facet polytope (`g0 + g_q θ_f ≥ 0`, mplcp.rs 455-460), holds. So every facet point lies in the
sub-facet of some accepted candidate. -/
theorem case2_lexmin_accepted (s : Finset ι) (hs : s.Nonempty) (L : ι → Fin (N + 1) → ℝ)
    (a : ι → ℝ) (ha : ∀ j ∈ s, 0 < a j) :
    ∃ l ∈ s, ∀ j ∈ s, LexNonneg (gammaVec L a l j) ∧ 0 ≤ gammaVec L a l j 0 := by
  obtain ⟨l, hl, hmin⟩ := exists_lexmin s hs (fun j => (a j)⁻¹ • L j)
  refine ⟨l, hl, fun j hj => ?_⟩
  have e : gammaVec L a l j = (a l * a j) • ((a j)⁻¹ • L j - (a l)⁻¹ • L l) := by
    simp only [gammaVec, smul_sub, smul_smul]
    have h1 : a l * a j * (a j)⁻¹ = a l := by field_simp [(ha j hj).ne']
    have h2 : a l * a j * (a l)⁻¹ = a j := by field_simp [(ha l hl).ne']
    rw [h1, h2]
  have hg : LexNonneg (gammaVec L a l j) := by
    rw [e]; exact (hmin j hj).smul (mul_pos (ha l hl) (ha j hj)).le
  exact ⟨hg, hg.zero_nonneg⟩

end Cover

section Walk

variable {ι κ V : Type*} [DecidableEq ι] [DecidableEq κ] {N : ℕ}

/-- At a genuine facet (`P ∩ Z` nonempty), the strict lexicographic minimum row vanishes on the
facet: a zero row always beats a row with a positive value. -/
theorem lexmin_zero_row {T : Matrix ι κ ℝ} {c : κ} {l : ι} (hl : 0 < -T l c)
    {L : ι → Fin (N + 1) → ℝ} (hnum : ∀ i, 0 ≤ L i 0)
    (hmin : ∀ i, i ≠ l → 0 < -T i c → LexPos (ratio T L c i - ratio T L c l))
    (hpz : ∃ i, 0 < -T i c ∧ L i 0 = 0) : L l 0 = 0 := by
  obtain ⟨i, hai, hi0⟩ := hpz
  by_contra hne
  have hpos : 0 < L l 0 := lt_of_le_of_ne (hnum l) (Ne.symm hne)
  have hil : i ≠ l := by rintro rfl; exact hne hi0
  have h := (hmin i hil hai).zero_nonneg
  simp only [ratio, Pi.sub_apply, Pi.smul_apply, smul_eq_mul, hi0, mul_zero, zero_sub] at h
  have : 0 < (-T l c)⁻¹ * L l 0 := mul_pos (inv_pos.mpr hl) hpos
  linarith

/-- **The first pivot establishes the walk invariant** (rows other than α's lexico-positive). -/
theorem first_step_nonAlpha {α : V} {comp : V → V} {s₀ s₁ : WalkState ι κ V (N + 1)}
    (h : WalkStep α comp s₀ s₁) (hstart : s₀.cv s₀.e = α) (hnum : ∀ i, 0 ≤ s₀.L i 0)
    (hZ : ∀ i, s₀.L i 0 = 0 → 0 ≤ -s₀.T i s₀.e)
    (hdeg : ∀ i, s₀.L i 0 = 0 → s₀.T i s₀.e = 0 → LexPos (s₀.L i))
    (hpz : ∃ i, 0 < -s₀.T i s₀.e ∧ s₀.L i 0 = 0) : NonAlphaLexPos α s₁ := by
  obtain ⟨l, -, ha, hm, hL, -, hrv, -⟩ := h
  have hl0 := lexmin_zero_row ha hnum hm hpz
  have key := first_step_rows ha hl0 hZ hdeg hnum hm
  intro i hi
  have hil : i ≠ l := by
    rintro rfl
    apply hi
    rw [hrv, Function.update_self, hstart]
  rw [hL]
  exact key i hil

/-- **Soundness of the facet walk.** A walk that starts at a genuine facet (α entering, `P ∩ Z`
nonempty, values nonnegative, zero rows not increasing, identically zero rows lexico-positive)
and ends at the terminal push (α basic in row `ra` with value 0 and `T ra e > 0`, no zero row
decreasing) returns a complementary basis whose every row is lexico-positive in
`[value | α coef | β]`, hence (`ray_lexFeasible`) lexico-feasible at `θ_f + t g` for all small
`t > 0`. -/
theorem walk_sound {α : V} {comp : V → V} (seq : ℕ → WalkState ι κ V (N + 1)) (m : ℕ)
    (hm : 1 ≤ m) (hstep : ∀ k, k < m → WalkStep α comp (seq k) (seq (k + 1)))
    (hstart : (seq 0).cv (seq 0).e = α) (hnum : ∀ i, 0 ≤ (seq 0).L i 0)
    (hZ0 : ∀ i, (seq 0).L i 0 = 0 → 0 ≤ -(seq 0).T i (seq 0).e)
    (hdeg : ∀ i, (seq 0).L i 0 = 0 → (seq 0).T i (seq 0).e = 0 → LexPos ((seq 0).L i))
    (hpz : ∃ i, 0 < -(seq 0).T i (seq 0).e ∧ (seq 0).L i 0 = 0)
    {ra : ι} (hra : ∀ i, i ≠ ra → (seq m).rv i ≠ α) (hα : 0 < (seq m).T ra (seq m).e)
    (hα0 : (seq m).L ra 0 = 0)
    (hZ : ∀ i, i ≠ ra → (seq m).L i 0 = 0 → 0 ≤ (seq m).T i (seq m).e) :
    ∀ i, LexPos (ray3 (pivVec (seq m).T (seq m).L ra (seq m).e i)
      (pivT (seq m).T ra (seq m).e i (seq m).e)) := by
  have h1 : NonAlphaLexPos α (seq 1) :=
    first_step_nonAlpha (hstep 0 (by omega)) hstart hnum hZ0 hdeg hpz
  have hinv : ∀ k, 1 ≤ k → k ≤ m → NonAlphaLexPos α (seq k) := by
    intro k hk1
    induction k with
    | zero => omega
    | succ k ih =>
      intro hk
      rcases Nat.eq_zero_or_pos k with rfl | hk0
      · exact h1
      · exact walkStep_nonAlphaLexPos (hstep k (by omega)) (ih hk0 (by omega))
  exact terminal_rows hα hα0 (fun i hi => hinv m hm le_rfl i (hra i hi)) hZ

end Walk

section Unique

variable {Θ : Type*} [AddCommGroup Θ] [Module ℝ Θ] {n : ℕ}

/-- **The neighbour just across a facet point is unique.** Two bases that are lexico-feasible at
`θ_f + t g` for all small `t > 0` are equal. With terminal soundness, the walk's output at `θ_f`
is that neighbour; Case-2 completeness reduces to the walk ending with the terminal push
(`WalkEndsWithPush`) on the branch that contains `θ_f` (`case2_lexmin_accepted`). -/
theorem neighbour_unique_on_ray {M : Matrix (Fin n) (Fin n) ℝ} (hM : M.PosSemidef)
    {r : Θ → Fin n → ℝ} {θf g : Θ} {C D : Finset (Fin n)}
    {LC LD : ℝ → Fin n → Fin (n + 1) → ℝ} {TC TD : Matrix (Fin n) (Fin n) ℝ}
    (hC : ∀ t, IsLexTableau M (r (θf + t • g)) C (LC t) TC)
    (hD : ∀ t, IsLexTableau M (r (θf + t • g)) D (LD t) TD)
    (fC : ∀ᶠ t in 𝓝[>] (0 : ℝ), LexFeasible (LC t))
    (fD : ∀ᶠ t in 𝓝[>] (0 : ℝ), LexFeasible (LD t)) : C = D := by
  obtain ⟨t, h1, h2⟩ := (fC.and fD).exists
  exact lex_unique hM (hC t) (hD t) h1 h2

end Unique

section Dependent

variable {n m p : Type*} [Fintype n] [Fintype m] [Fintype p] [DecidableEq n]

/-- **Two multiplier vectors at one θ** (two pLCP solutions) have the same slacks and the same
primal point, and their difference lies in `ker Aᵀ` and vanishes on the inactive rows. -/
theorem lcp_multipliers_dependent {H : Matrix n n ℝ} (hH : H.PosDef) (A : Matrix m n ℝ)
    (b : m → ℝ) (c : n → ℝ) (S : Matrix m p ℝ) (F : Matrix n p ℝ) (θ : p → ℝ)
    {w₁ lam₁ w₂ lam₂ : m → ℝ} (h₁ : LCPSol (lcpM H A) (lcpRhs H A b c S F θ) w₁ lam₁)
    (h₂ : LCPSol (lcpM H A) (lcpRhs H A b c S F θ) w₂ lam₂) :
    w₁ = w₂ ∧ primalOf H A F c θ lam₁ = primalOf H A F c θ lam₂ ∧ Aᵀ *ᵥ (lam₁ - lam₂) = 0 ∧
      ∀ i, 0 < w₁ i → lam₁ i - lam₂ i = 0 := by
  have hw := lcp_w_unique (lcpM_posSemidef hH A) h₁ h₂
  have hx : primalOf H A F c θ lam₁ = primalOf H A F c θ lam₂ :=
    ((lcp_unique_optimizer hH A b c S F θ h₂).2 _ (lcp_unique_optimizer hH A b c S F θ h₁).1)
  have hdet : IsUnit H.det := (isUnit_iff_isUnit_det H).mp hH.isUnit
  have hA : Aᵀ *ᵥ (lam₁ - lam₂) = 0 := by
    have h1 : H⁻¹ *ᵥ (F *ᵥ θ + c + Aᵀ *ᵥ lam₁) = H⁻¹ *ᵥ (F *ᵥ θ + c + Aᵀ *ᵥ lam₂) := by
      simpa [primalOf] using hx
    have h2 := congrArg (fun v => H *ᵥ v) h1
    simp only [mulVec_mulVec, mul_nonsing_inv H hdet, one_mulVec] at h2
    rw [mulVec_sub]
    have : Aᵀ *ᵥ lam₁ = Aᵀ *ᵥ lam₂ := add_left_cancel h2
    rw [this, sub_self]
  refine ⟨hw, hx, hA, fun i hi => ?_⟩
  have a := h₁.comp i
  have b' := h₂.comp i
  rw [← hw] at b'
  have z1 : lam₁ i = 0 := by
    rcases mul_eq_zero.mp a with h | h
    · exact absurd h hi.ne'
    · exact h
  have z2 : lam₂ i = 0 := by
    rcases mul_eq_zero.mp b' with h | h
    · exact absurd h hi.ne'
    · exact h
  rw [z1, z2, sub_self]

/-- **Case 2 needs dependent active rows.** If the multipliers at `θ` are not unique, the active
rows (`w_i = 0`) are linearly dependent: a nonzero combination of them vanishes. A Case-2 step
whose α stays constant moves the multipliers along such a segment (slacks and primal point fixed). -/
theorem case2_needs_dependent_rows {H : Matrix n n ℝ} (hH : H.PosDef) (A : Matrix m n ℝ)
    (b : m → ℝ) (c : n → ℝ) (S : Matrix m p ℝ) (F : Matrix n p ℝ) (θ : p → ℝ)
    {w₁ lam₁ w₂ lam₂ : m → ℝ} (h₁ : LCPSol (lcpM H A) (lcpRhs H A b c S F θ) w₁ lam₁)
    (h₂ : LCPSol (lcpM H A) (lcpRhs H A b c S F θ) w₂ lam₂) (hne : lam₁ ≠ lam₂) :
    ∃ d : m → ℝ, d ≠ 0 ∧ Aᵀ *ᵥ d = 0 ∧ ∀ i, 0 < w₁ i → d i = 0 := by
  obtain ⟨-, -, hA, hsupp⟩ := lcp_multipliers_dependent hH A b c S F θ h₁ h₂
  exact ⟨lam₁ - lam₂, sub_ne_zero.mpr hne, hA, hsupp⟩

end Dependent

end Rpopt
