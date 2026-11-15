/-
Termination of the facet walk of `pivot_facet` (the `8n + 50` loop and the recursion) in exact
arithmetic, and the structure of Case-2 steps.

The walk at one facet point `θ_f` is a sequence of states (tableau rows `L`, coefficients `T`,
row and column labels, entering column). Each step pivots on the strict lexicographic minimum
ratio row of the entering column; the complement of the leaving variable enters next. Pointwise in
`θ_f` the Case-1 and Case-2 branches of `pivot_facet` are the same rule (Case 2 only splits the
facet into the pieces where a given row is the minimum), so one branch of the recursion at depth
`d` follows this walk through `d` Case-2 steps.

* `chain_returns_to_one`, `chain_no_repeat`, `chain_length_bound`: abstract facts about chains of
  a step relation that is injective (every state has at most one predecessor).
* `lex_pivot_reverse_on`: after a lexicographic pivot, the previous leaving row is the strict
  lexicographic minimum for the reverse pivot: the predecessor is determined.
* `walkStep_injective_full`, `walkStep_injective_inv`: the walk step is injective.
* `walk_no_repeat` (**3a**): if the start basis is lexico-feasible at the facet point (every row,
  including α's after the first pivot, stays lexico-positive), no state ever repeats, so the walk
  stops after at most as many pivots as there are label configurations.
* `walk_repeat_returns`: in general (only the non-α rows lexico-positive), the only possible
  repetition is a return to the state right after the first pivot.
* `lcp_multipliers_dependent` (**3b**): two multiplier vectors at the same θ have the same slacks and
  primal point and differ by a vector in `ker Aᵀ` supported on the active rows. A Case-2 step whose
  α stays constant moves along such a segment, so Case 2 needs linearly dependent active rows.
-/
import RpoptProofs.Lex

open Matrix

set_option linter.unusedSectionVars false

namespace Rpopt

section Chain

variable {S : Type*} (step : S → S → Prop)

/-- If the step relation is injective on states with `P`, every repetition in a chain (after the
start) leads back to the state after the first step. -/
theorem chain_returns_to_one (P : S → Prop) (seq : ℕ → S) (m : ℕ)
    (hstep : ∀ k, k < m → step (seq k) (seq (k + 1)))
    (hP : ∀ k, 1 ≤ k → k ≤ m → P (seq k))
    (hinj : ∀ a b c, P a → P b → step a c → step b c → a = b) :
    ∀ i j, 1 ≤ i → i < j → j ≤ m → seq i = seq j → ∃ k, 1 < k ∧ k ≤ m ∧ seq k = seq 1 := by
  intro i
  induction i with
  | zero => intro j h; omega
  | succ i ih =>
    intro j h1 hij hjm heq
    rcases Nat.eq_zero_or_pos i with rfl | hi
    · exact ⟨j, by omega, hjm, heq.symm⟩
    · obtain ⟨j', rfl⟩ : ∃ j', j = j' + 1 := ⟨j - 1, by omega⟩
      have a := hstep i (by omega)
      have b := hstep j' (by omega)
      rw [← heq] at b
      have := hinj _ _ _ (hP i hi (by omega)) (hP j' (by omega) (by omega)) a b
      exact ih j' hi (by omega) (by omega) this

/-- If the step relation is injective on all states of the chain, and no later state equals the
start, the chain never repeats a state. -/
theorem chain_no_repeat (P : S → Prop) (seq : ℕ → S) (m : ℕ)
    (hstep : ∀ k, k < m → step (seq k) (seq (k + 1)))
    (hP : ∀ k, k ≤ m → P (seq k))
    (hinj : ∀ a b c, P a → P b → step a c → step b c → a = b)
    (h0 : ∀ k, 1 ≤ k → k ≤ m → seq k ≠ seq 0) :
    ∀ i j, i < j → j ≤ m → seq i ≠ seq j := by
  intro i j hij hjm heq
  rcases Nat.eq_zero_or_pos i with rfl | hi
  · exact h0 j (by omega) hjm heq.symm
  · obtain ⟨k, hk1, hkm, hk⟩ := chain_returns_to_one step P seq m hstep
      (fun k _ hk => hP k hk) hinj i j hi hij hjm heq
    obtain ⟨k', rfl⟩ : ∃ k', k = k' + 1 := ⟨k - 1, by omega⟩
    have a := hstep 0 (by omega)
    have b := hstep k' (by omega)
    rw [hk] at b
    have := hinj _ _ _ (hP 0 (by omega)) (hP k' (by omega)) a b
    exact h0 k' (by omega) (by omega) this.symm

/-- A non-repeating chain whose states are determined by a finite key (for the walk: the basis
labels, since a tableau is determined by its basis, `tableau_unique`) has at most as many states
as there are keys. -/
theorem chain_length_bound {K : Type*} [Fintype K] (key : S → K) (seq : ℕ → S) (m : ℕ)
    (hno : ∀ i j, i < j → j ≤ m → seq i ≠ seq j)
    (hkey : ∀ i j, i ≤ m → j ≤ m → key (seq i) = key (seq j) → seq i = seq j) :
    m + 1 ≤ Fintype.card K := by
  let f : Fin (m + 1) → K := fun k => key (seq k)
  have hf : Function.Injective f := by
    intro a b hab
    have ha := a.isLt
    have hb := b.isLt
    have h := hkey a b (by omega) (by omega) hab
    by_contra hne
    rcases lt_or_gt_of_ne (Fin.val_ne_of_ne hne) with h1 | h1
    · exact hno a b h1 (by omega) h
    · exact hno b a h1 (by omega) h.symm
  simpa using Fintype.card_le_of_injective f hf

end Chain

section Reverse

variable {ι κ : Type*} [DecidableEq ι] [DecidableEq κ] {N : ℕ}

/-- The ratio vector `L_i / a_i` of row `i` for entering column `c` (`a_i = -T i c`). -/
noncomputable def ratio (T : Matrix ι κ ℝ) (L : ι → Fin N → ℝ) (c : κ) (i : ι) : Fin N → ℝ :=
  (-T i c)⁻¹ • L i

/-- The lexicographic ratio test on a tracked set of rows `S`: the rows of `S` stay
lexico-positive (the untracked rows, here α's row, are not constrained). -/
theorem lex_ratio_pivot_on {T : Matrix ι κ ℝ} {c : κ} {l : ι} (hl : 0 < -T l c)
    {L : ι → Fin N → ℝ} {S : Set ι} (hlS : l ∈ S) (hL : ∀ i ∈ S, LexPos (L i))
    (hmin : ∀ i ∈ S, i ≠ l → 0 < -T i c → LexPos (ratio T L c i - ratio T L c l)) :
    ∀ i ∈ S, LexPos (pivVec T L l c i) := by
  intro i hi
  by_cases hil : i = l
  · subst hil
    simp only [pivVec, if_true]
    exact (hL i hi).smul (by rw [← inv_neg]; exact inv_pos.mpr hl)
  · simp only [pivVec, if_neg hil]
    by_cases ha : 0 < -T i c
    · have e : L i - (T i c / T l c) • L l =
          (-T i c) • ((-T i c)⁻¹ • L i - (-T l c)⁻¹ • L l) := by
        rw [smul_sub, smul_smul, smul_smul, mul_inv_cancel₀ ha.ne', one_smul]
        congr 2
        field_simp
      rw [e]
      exact (hmin i hi hil ha).smul ha
    · have hc : 0 ≤ -(T i c / T l c) := by
        have : T i c / T l c = (-T i c) / (-T l c) := by rw [neg_div_neg_eq]
        rw [this, ← neg_div]
        exact div_nonneg (by linarith) hl.le
      rw [sub_eq_add_neg, ← neg_smul]
      exact (hL i hi).add (LexNonneg.smul (Or.inr (hL l hlS)) hc)

/-- **Reversibility.** After the pivot on `(l, c)`, the column `c` holds the variable that left.
For the reverse pivot (that variable entering), row `l` is decreasing and it is the strict
lexicographic minimum ratio among the tracked rows: the predecessor is determined. -/
theorem lex_pivot_reverse_on {T : Matrix ι κ ℝ} {c : κ} {l : ι} (hl : 0 < -T l c)
    {L : ι → Fin N → ℝ} {S : Set ι} (hL : ∀ i ∈ S, LexPos (L i)) :
    0 < -pivT T l c l c ∧
      ∀ i ∈ S, i ≠ l → 0 < -pivT T l c i c →
        LexPos (ratio (pivT T l c) (pivVec T L l c) c i -
          ratio (pivT T l c) (pivVec T L l c) c l) := by
  have hTl : T l c < 0 := by linarith
  have hpl : pivT T l c l c = (T l c)⁻¹ := by simp [pivT]
  refine ⟨by rw [hpl, neg_pos]; exact inv_lt_zero.mpr hTl, fun i hi hil ha' => ?_⟩
  have hq : pivT T l c i c = T i c / T l c := by simp [pivT, hil]
  have hLi : pivVec T L l c i = L i - (T i c / T l c) • L l := by simp [pivVec, hil]
  have hLl : pivVec T L l c l = (-(T l c)⁻¹) • L l := by simp [pivVec]
  rw [hq] at ha'
  simp only [ratio]
  rw [hq, hpl, hLi, hLl]
  have hq0 : T i c / T l c ≠ 0 := fun h => by rw [h] at ha'; simp at ha'
  have hs0 : -(T l c)⁻¹ ≠ 0 := by
    rw [neg_ne_zero]; exact inv_ne_zero hTl.ne
  have e : (-(T i c / T l c))⁻¹ • (L i - (T i c / T l c) • L l) -
      (-(T l c)⁻¹)⁻¹ • ((-(T l c)⁻¹) • L l) = (-(T i c / T l c))⁻¹ • L i := by
    rw [smul_smul (-(T l c)⁻¹)⁻¹, inv_mul_cancel₀ hs0, one_smul, smul_sub, smul_smul]
    have hTi : T i c ≠ 0 := (div_ne_zero_iff.mp hq0).1
    have hTl0 : T l c ≠ 0 := hTl.ne
    have : (-(T i c / T l c))⁻¹ * (T i c / T l c) = -1 := by field_simp
    rw [this, neg_one_smul, sub_neg_eq_add, add_sub_cancel_right]
  rw [e]
  exact (hL i hi).smul (inv_pos.mpr ha')

end Reverse

section Walk

variable {ι κ V : Type*} [DecidableEq ι] [DecidableEq κ] {N : ℕ}

/-- A state of the facet walk (Rust: the work tableau `Tab` with `row_var`, `col_var`, and the
entering column `e` of `pivot_facet`). `L` holds the lexicographic rows `[value | β]`. -/
@[ext] structure WalkState (ι κ V : Type*) (N : ℕ) where
  L : ι → Fin N → ℝ
  T : Matrix ι κ ℝ
  rv : ι → V
  cv : κ → V
  e : κ

/-- One step of the walk: the strict lexicographic minimum ratio row `l` of the entering column
leaves (it is not α), the pivot is applied, and the complement of the leaving variable is the next
entering column. -/
def WalkStep (α : V) (comp : V → V) (s s' : WalkState ι κ V N) : Prop :=
  ∃ l, s.rv l ≠ α ∧ 0 < -s.T l s.e ∧
    (∀ i, i ≠ l → 0 < -s.T i s.e → LexPos (ratio s.T s.L s.e i - ratio s.T s.L s.e l)) ∧
    s'.L = pivVec s.T s.L l s.e ∧ s'.T = pivT s.T l s.e ∧
    s'.rv = Function.update s.rv l (s.cv s.e) ∧ s'.cv = Function.update s.cv s.e (s.rv l) ∧
    s'.cv s'.e = comp (s.rv l)

/-- Every row lexico-positive. -/
def AllLexPos (s : WalkState ι κ V N) : Prop := ∀ i, LexPos (s.L i)

/-- Every row other than α's lexico-positive. -/
def NonAlphaLexPos (α : V) (s : WalkState ι κ V N) : Prop := ∀ i, s.rv i ≠ α → LexPos (s.L i)

theorem walkStep_allLexPos {α : V} {comp : V → V} {s s' : WalkState ι κ V N}
    (h : WalkStep α comp s s') (hs : AllLexPos s) : AllLexPos s' := by
  obtain ⟨l, -, ha, hm, hL, -⟩ := h
  intro i
  rw [hL]
  exact lex_ratio_pivot_on (S := Set.univ) ha (Set.mem_univ l) (fun i _ => hs i)
    (fun i _ hil hai => hm i hil hai) i (Set.mem_univ i)

theorem walkStep_nonAlphaLexPos {α : V} {comp : V → V} {s s' : WalkState ι κ V N}
    (h : WalkStep α comp s s') (hs : NonAlphaLexPos α s) : NonAlphaLexPos α s' := by
  obtain ⟨l, hlα, ha, hm, hL, -, hrv, -⟩ := h
  have key := lex_ratio_pivot_on (S := {i | s.rv i ≠ α}) ha hlα (fun i hi => hs i hi)
    (fun i _ hil hai => hm i hil hai)
  intro i hi
  rw [hL]
  by_cases hil : i = l
  · subst hil; exact key i hlα
  · have : s'.rv i = s.rv i := by rw [hrv, Function.update_of_ne hil]
    exact key i (show s.rv i ≠ α by rw [← this]; exact hi)

/-- Entering columns and leaving labels of two predecessors of the same state agree. -/
theorem walkStep_entering_eq {comp : V → V} (hcomp : Function.Involutive comp)
    {s₁ s₂ s' : WalkState ι κ V N} {l₁ l₂ : ι}
    (hcv₁ : s'.cv = Function.update s₁.cv s₁.e (s₁.rv l₁)) (he₁ : s'.cv s'.e = comp (s₁.rv l₁))
    (hcv₂ : s'.cv = Function.update s₂.cv s₂.e (s₂.rv l₂)) (he₂ : s'.cv s'.e = comp (s₂.rv l₂))
    (hinj : Function.Injective s'.cv) : s₁.e = s₂.e ∧ s₁.rv l₁ = s₂.rv l₂ := by
  have c₁ : s'.cv s₁.e = s₁.rv l₁ := by rw [hcv₁, Function.update_self]
  have c₂ : s'.cv s₂.e = s₂.rv l₂ := by rw [hcv₂, Function.update_self]
  have k₁ : comp (s'.cv s'.e) = s₁.rv l₁ := by rw [he₁, hcomp]
  have k₂ : comp (s'.cv s'.e) = s₂.rv l₂ := by rw [he₂, hcomp]
  have hl : s₁.rv l₁ = s₂.rv l₂ := k₁.symm.trans k₂
  exact ⟨hinj (c₁.trans (hl.trans c₂.symm)), hl⟩

/-- Once the entering column and the leaving row agree, the predecessor is determined. -/
theorem walkStep_pred_eq {s₁ s₂ s' : WalkState ι κ V N} {l : ι}
    (hee : s₁.e = s₂.e) (ha₁ : 0 < -s₁.T l s₁.e) (ha₂ : 0 < -s₂.T l s₂.e)
    (hL₁ : s'.L = pivVec s₁.T s₁.L l s₁.e) (hT₁ : s'.T = pivT s₁.T l s₁.e)
    (hrv₁ : s'.rv = Function.update s₁.rv l (s₁.cv s₁.e))
    (hcv₁ : s'.cv = Function.update s₁.cv s₁.e (s₁.rv l))
    (hL₂ : s'.L = pivVec s₂.T s₂.L l s₂.e) (hT₂ : s'.T = pivT s₂.T l s₂.e)
    (hrv₂ : s'.rv = Function.update s₂.rv l (s₂.cv s₂.e))
    (hcv₂ : s'.cv = Function.update s₂.cv s₂.e (s₂.rv l)) : s₁ = s₂ := by
  have p₁ : s₁.T l s₁.e ≠ 0 := fun h => by rw [h] at ha₁; simp at ha₁
  have p₂ : s₂.T l s₂.e ≠ 0 := fun h => by rw [h] at ha₂; simp at ha₂
  have hTT : s₁.T = s₂.T := by
    have a := pivT_pivT p₁
    have b := pivT_pivT p₂
    rw [← hT₁] at a
    rw [← hT₂, ← hee] at b
    rw [← a, ← b]
  have hLL : s₁.L = s₂.L := by
    have a := pivVec_pivVec p₁ s₁.L
    have b := pivVec_pivVec p₂ s₂.L
    rw [← hT₁, ← hL₁] at a
    rw [← hT₂, ← hL₂, ← hee] at b
    rw [← a, ← b]
  have hrv : s₁.rv = s₂.rv := by
    funext i
    by_cases hi : i = l
    · subst hi
      have a : s'.cv s₁.e = s₁.rv i := by rw [hcv₁, Function.update_self]
      have b : s'.cv s₂.e = s₂.rv i := by rw [hcv₂, Function.update_self]
      rw [← a, ← b, hee]
    · have a := congrFun hrv₁ i
      have b := congrFun hrv₂ i
      rw [Function.update_of_ne hi] at a b
      rw [← a, ← b]
  have hcv : s₁.cv = s₂.cv := by
    funext k
    by_cases hk : k = s₁.e
    · have a : s'.rv l = s₁.cv s₁.e := by rw [hrv₁, Function.update_self]
      have b : s'.rv l = s₂.cv s₂.e := by rw [hrv₂, Function.update_self]
      rw [hk, ← a, b, hee]
    · have a := congrFun hcv₁ k
      have b := congrFun hcv₂ k
      rw [Function.update_of_ne hk] at a
      rw [Function.update_of_ne (hee ▸ hk)] at b
      rw [← a, ← b]
  exact WalkState.ext hLL hTT hrv hcv hee

/-- **The walk step is injective** when every row is lexico-positive. -/
theorem walkStep_injective_full {α : V} {comp : V → V} (hcomp : Function.Involutive comp)
    {s₁ s₂ s' : WalkState ι κ V N} (h₁ : WalkStep α comp s₁ s') (h₂ : WalkStep α comp s₂ s')
    (f₁ : AllLexPos s₁) (f₂ : AllLexPos s₂) (hinj : Function.Injective s'.cv) : s₁ = s₂ := by
  obtain ⟨l₁, -, ha₁, -, hL₁, hT₁, hrv₁, hcv₁, he₁⟩ := h₁
  obtain ⟨l₂, -, ha₂, -, hL₂, hT₂, hrv₂, hcv₂, he₂⟩ := h₂
  obtain ⟨hee, -⟩ := walkStep_entering_eq hcomp hcv₁ he₁ hcv₂ he₂ hinj
  have r₁ := lex_pivot_reverse_on (S := Set.univ) ha₁ (fun i _ => f₁ i)
  have r₂ := lex_pivot_reverse_on (S := Set.univ) ha₂ (fun i _ => f₂ i)
  rw [← hT₁, ← hL₁] at r₁
  rw [← hT₂, ← hL₂, ← hee] at r₂
  have hl : l₁ = l₂ := by
    by_contra hne
    have A := r₁.2 l₂ (Set.mem_univ _) (Ne.symm hne) r₂.1
    have B := r₂.2 l₁ (Set.mem_univ _) hne r₁.1
    have e : ratio s'.T s'.L s₁.e l₁ - ratio s'.T s'.L s₁.e l₂ =
        -(ratio s'.T s'.L s₁.e l₂ - ratio s'.T s'.L s₁.e l₁) := by abel
    rw [e] at B
    exact not_lexPos_and_neg A B
  subst hl
  exact walkStep_pred_eq hee ha₁ ha₂ hL₁ hT₁ hrv₁ hcv₁ hL₂ hT₂ hrv₂ hcv₂

/-- **The walk step is injective** on non-start states when the rows other than α's are
lexico-positive. -/
theorem walkStep_injective_inv {α : V} {comp : V → V} (hcomp : Function.Involutive comp)
    {s₁ s₂ s' : WalkState ι κ V N} (h₁ : WalkStep α comp s₁ s') (h₂ : WalkStep α comp s₂ s')
    (f₁ : NonAlphaLexPos α s₁) (f₂ : NonAlphaLexPos α s₂) (n₁ : s₁.cv s₁.e ≠ α)
    (n₂ : s₂.cv s₂.e ≠ α) (hinj : Function.Injective s'.cv) : s₁ = s₂ := by
  obtain ⟨l₁, -, ha₁, -, hL₁, hT₁, hrv₁, hcv₁, he₁⟩ := h₁
  obtain ⟨l₂, -, ha₂, -, hL₂, hT₂, hrv₂, hcv₂, he₂⟩ := h₂
  obtain ⟨hee, -⟩ := walkStep_entering_eq hcomp hcv₁ he₁ hcv₂ he₂ hinj
  have r₁ := lex_pivot_reverse_on (S := {i | s₁.rv i ≠ α}) ha₁ (fun i hi => f₁ i hi)
  have r₂ := lex_pivot_reverse_on (S := {i | s₂.rv i ≠ α}) ha₂ (fun i hi => f₂ i hi)
  rw [← hT₁, ← hL₁] at r₁
  rw [← hT₂, ← hL₂, ← hee] at r₂
  have hl : l₁ = l₂ := by
    by_contra hne
    have m₂ : s₁.rv l₂ ≠ α := by
      have e1 : s'.rv l₂ = s₂.cv s₂.e := by rw [hrv₂, Function.update_self]
      have e2 : s'.rv l₂ = s₁.rv l₂ := by rw [hrv₁, Function.update_of_ne (Ne.symm hne)]
      rw [← e2, e1]; exact n₂
    have m₁ : s₂.rv l₁ ≠ α := by
      have e1 : s'.rv l₁ = s₁.cv s₁.e := by rw [hrv₁, Function.update_self]
      have e2 : s'.rv l₁ = s₂.rv l₁ := by rw [hrv₂, Function.update_of_ne hne]
      rw [← e2, e1]; exact n₁
    have A := r₁.2 l₂ m₂ (Ne.symm hne) r₂.1
    have B := r₂.2 l₁ m₁ hne r₁.1
    have e : ratio s'.T s'.L s₁.e l₁ - ratio s'.T s'.L s₁.e l₂ =
        -(ratio s'.T s'.L s₁.e l₂ - ratio s'.T s'.L s₁.e l₁) := by abel
    rw [e] at B
    exact not_lexPos_and_neg A B
  subst hl
  exact walkStep_pred_eq hee ha₁ ha₂ hL₁ hT₁ hrv₁ hcv₁ hL₂ hT₂ hrv₂ hcv₂

/-- **3a: no state repeats** when the start basis is lexico-feasible at the facet point (every
row of the start tableau lexico-positive). The start enters α; afterwards α is never the entering
variable (the complement map never returns α). -/
theorem walk_no_repeat {α : V} {comp : V → V} (hcomp : Function.Involutive comp)
    (hcα : ∀ x, comp x ≠ α) (seq : ℕ → WalkState ι κ V N) (m : ℕ)
    (hstep : ∀ k, k < m → WalkStep α comp (seq k) (seq (k + 1)))
    (hcv : ∀ k, k ≤ m → Function.Injective (seq k).cv)
    (hstart : (seq 0).cv (seq 0).e = α) (hfull : AllLexPos (seq 0)) :
    ∀ i j, i < j → j ≤ m → seq i ≠ seq j := by
  have hall : ∀ k, k ≤ m → AllLexPos (seq k) := by
    intro k
    induction k with
    | zero => intro _; exact hfull
    | succ k ih => intro hk; exact walkStep_allLexPos (hstep k (by omega)) (ih (by omega))
  let step' : WalkState ι κ V N → WalkState ι κ V N → Prop :=
    fun a c => WalkStep α comp a c ∧ Function.Injective c.cv
  refine chain_no_repeat step' AllLexPos seq m
    (fun k hk => ⟨hstep k hk, hcv (k + 1) (by omega)⟩) hall
    (fun a b c ha hb hac hbc => walkStep_injective_full hcomp hac.1 hbc.1 ha hb hac.2) ?_
  intro k hk1 hkm heq
  obtain ⟨k', rfl⟩ : ∃ k', k = k' + 1 := ⟨k - 1, by omega⟩
  obtain ⟨l, -, -, -, -, -, -, -, he⟩ := hstep k' (by omega)
  rw [heq, hstart] at he
  exact hcα _ he.symm

/-- **Repetitions return to the first state.** With only the non-α rows lexico-positive from the
state after the first pivot on, any repeated state is that state: a cycle can be detected by
comparing with it alone. -/
theorem walk_repeat_returns {α : V} {comp : V → V} (hcomp : Function.Involutive comp)
    (hcα : ∀ x, comp x ≠ α) (seq : ℕ → WalkState ι κ V N) (m : ℕ)
    (hstep : ∀ k, k < m → WalkStep α comp (seq k) (seq (k + 1)))
    (hcv : ∀ k, k ≤ m → Function.Injective (seq k).cv) (hinv1 : NonAlphaLexPos α (seq 1)) :
    ∀ i j, 1 ≤ i → i < j → j ≤ m → seq i = seq j → ∃ k, 1 < k ∧ k ≤ m ∧ seq k = seq 1 := by
  have hinv : ∀ k, 1 ≤ k → k ≤ m → NonAlphaLexPos α (seq k) := by
    intro k hk1
    induction k with
    | zero => omega
    | succ k ih =>
      intro hk
      rcases Nat.eq_zero_or_pos k with rfl | hk0
      · exact hinv1
      · exact walkStep_nonAlphaLexPos (hstep k (by omega)) (ih hk0 (by omega))
  have hne : ∀ k, 1 ≤ k → k ≤ m → (seq k).cv (seq k).e ≠ α := by
    intro k hk1 hkm
    obtain ⟨k', rfl⟩ : ∃ k', k = k' + 1 := ⟨k - 1, by omega⟩
    obtain ⟨l, -, -, -, -, -, -, -, he⟩ := hstep k' (by omega)
    rw [he]; exact hcα _
  let step' : WalkState ι κ V N → WalkState ι κ V N → Prop :=
    fun a c => WalkStep α comp a c ∧ Function.Injective c.cv
  exact chain_returns_to_one step' (fun s => NonAlphaLexPos α s ∧ s.cv s.e ≠ α) seq m
    (fun k hk => ⟨hstep k hk, hcv (k + 1) (by omega)⟩)
    (fun k hk1 hkm => ⟨hinv k hk1 hkm, hne k hk1 hkm⟩)
    (fun a b c ha hb hac hbc =>
      walkStep_injective_inv hcomp hac.1 hbc.1 ha.1 hb.1 ha.2 hb.2 hac.2)

end Walk

end Rpopt
