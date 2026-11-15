/-
Duplicated constraints (mplcp.rs `duplicate_rows`, `dedupe_program`, `unreduce_regions`).

A row `i` is a positive multiple of its representative `rep i`, the lowest index of its group:
`[A_i | b_i] = s_i [A_{rep i} | b_{rep i}]` with `s_i > 0`, and `rep (rep i) = rep i`. Here `b` is
the right-hand side at one parameter (`b + F θ`), so every statement holds at every `θ`.

* `Twins.feasible_iff`: the full and the reduced row sets have the same feasible points, hence the
  same feasible parameters and the same optimizer of any objective.
* `Twins.tight_iff`: a twin is tight exactly when its representative is.
* `Twins.kkt_reduce`: multipliers of the full program give multipliers of the reduced program (the
  weighted total of each group on its kept row), with the same combination of rows and
  complementarity.
* `Twins.kkt_split`: conversely, every nonnegative split of a reduced multiplier among the twins of
  its group is a multiplier of the full program.
-/
import Mathlib

open Matrix

set_option linter.unusedSectionVars false

namespace Rpopt

variable {m k : Type*} [Fintype m] [Fintype k] [DecidableEq m]

/-- Rows `i` that are positive multiples of their representative `rep i`. -/
structure Twins (A : Matrix m k ℝ) (b : m → ℝ) (rep : m → m) (s : m → ℝ) : Prop where
  pos : ∀ i, 0 < s i
  idem : ∀ i, rep (rep i) = rep i
  row : ∀ i, A i = s i • A (rep i)
  rhs : ∀ i, b i = s i * b (rep i)

namespace Twins

variable {A : Matrix m k ℝ} {b : m → ℝ} {rep : m → m} {s : m → ℝ}

theorem mulVec_eq (h : Twins A b rep s) (x : k → ℝ) (i : m) :
    (A *ᵥ x) i = s i * (A *ᵥ x) (rep i) := by
  show A i ⬝ᵥ x = s i * (A (rep i) ⬝ᵥ x)
  rw [h.row i, smul_dotProduct, smul_eq_mul]

theorem slack_eq (h : Twins A b rep s) (x : k → ℝ) (i : m) :
    b i - (A *ᵥ x) i = s i * (b (rep i) - (A *ᵥ x) (rep i)) := by
  rw [h.mulVec_eq x i, h.rhs i]
  ring

/-- **Same feasible set.** The full rows hold exactly when the kept rows (`rep i = i`) hold. -/
theorem feasible_iff (h : Twins A b rep s) (x : k → ℝ) :
    (∀ i, (A *ᵥ x) i ≤ b i) ↔ (∀ i, rep i = i → (A *ᵥ x) i ≤ b i) := by
  refine ⟨fun H i _ => H i, fun H i => ?_⟩
  have h1 := H (rep i) (h.idem i)
  have h2 : 0 ≤ b i - (A *ᵥ x) i := by
    rw [h.slack_eq x i]
    exact mul_nonneg (h.pos i).le (by linarith)
  linarith

/-- A twin is tight exactly when its representative is. -/
theorem tight_iff (h : Twins A b rep s) (x : k → ℝ) (i : m) :
    (A *ᵥ x) i = b i ↔ (A *ᵥ x) (rep i) = b (rep i) := by
  have e := h.slack_eq x i
  have hs := (h.pos i).ne'
  constructor
  · intro hi
    have : s i * (b (rep i) - (A *ᵥ x) (rep i)) = 0 := by rw [← e, hi, sub_self]
    rcases mul_eq_zero.mp this with h0 | h0
    · exact absurd h0 hs
    · linarith
  · intro hi
    have : b i - (A *ᵥ x) i = 0 := by rw [e, hi, sub_self, mul_zero]
    linarith

/-- The weighted total of the multipliers of the group of `j`. -/
noncomputable def total (rep : m → m) (s lam : m → ℝ) (j : m) : ℝ :=
  ∑ i ∈ Finset.univ.filter (fun i => rep i = j), s i * lam i

/-- Splitting multipliers among twins: if the weighted multipliers of each group add up to `μ` of
its representative, both give the same combination of rows. -/
theorem vecMul_split (h : Twins A b rep s) {lam μ : m → ℝ} (hsplit : ∀ j, total rep s lam j = μ j) :
    lam ᵥ* A = μ ᵥ* A := by
  funext q
  simp only [vecMul, dotProduct]
  have e : ∀ i, lam i * A i q = s i * lam i * A (rep i) q := by
    intro i
    rw [h.row i, Pi.smul_apply, smul_eq_mul]
    ring
  simp only [e]
  rw [← Finset.sum_fiberwise Finset.univ rep (fun i => s i * lam i * A (rep i) q)]
  refine Finset.sum_congr rfl fun j _ => ?_
  rw [← hsplit j, total, Finset.sum_mul]
  refine Finset.sum_congr rfl fun i hi => ?_
  rw [(Finset.mem_filter.mp hi).2]

/-- **Full to reduced.** The weighted group totals of a nonnegative, complementary multiplier of the
full program form a multiplier of the reduced program: nonnegative, zero off the kept rows,
complementary, and with the same combination of rows. -/
theorem kkt_reduce (h : Twins A b rep s) {x : k → ℝ} {lam : m → ℝ} (hlam : ∀ i, 0 ≤ lam i)
    (hcs : ∀ i, lam i * (b i - (A *ᵥ x) i) = 0) :
    (∀ j, 0 ≤ total rep s lam j) ∧ (∀ j, rep j ≠ j → total rep s lam j = 0) ∧
      (∀ j, total rep s lam j * (b j - (A *ᵥ x) j) = 0) ∧
      total rep s lam ᵥ* A = lam ᵥ* A := by
  refine ⟨fun j => Finset.sum_nonneg fun i _ => mul_nonneg (h.pos i).le (hlam i), fun j hj => ?_,
    fun j => ?_, (h.vecMul_split fun j => rfl).symm⟩
  · refine Finset.sum_eq_zero fun i hi => ?_
    have hi' := (Finset.mem_filter.mp hi).2
    exact absurd (by rw [← hi', h.idem i]) hj
  · rw [total, Finset.sum_mul]
    refine Finset.sum_eq_zero fun i hi => ?_
    have hi' := (Finset.mem_filter.mp hi).2
    have e := h.slack_eq x i
    rw [hi'] at e
    have := hcs i
    rw [e] at this
    linear_combination this

/-- **Reduced to full.** Every nonnegative split `lam` of a complementary reduced multiplier `μ`
among the twins of each group is complementary in the full program, with the same combination of
rows. -/
theorem kkt_split (h : Twins A b rep s) {x : k → ℝ} {lam μ : m → ℝ} (hlam : ∀ i, 0 ≤ lam i)
    (hsplit : ∀ j, total rep s lam j = μ j) (hcs : ∀ j, μ j * (b j - (A *ᵥ x) j) = 0) :
    lam ᵥ* A = μ ᵥ* A ∧ ∀ i, lam i * (b i - (A *ᵥ x) i) = 0 := by
  refine ⟨h.vecMul_split hsplit, fun i => ?_⟩
  rw [h.slack_eq x i]
  by_cases ht : b (rep i) - (A *ᵥ x) (rep i) = 0
  · rw [ht, mul_zero, mul_zero]
  · have hμ : μ (rep i) = 0 := by
      rcases mul_eq_zero.mp (hcs (rep i)) with h0 | h0
      · exact h0
      · exact absurd h0 ht
    have hzero : s i * lam i = 0 := by
      have hsum := hsplit (rep i)
      rw [hμ, total] at hsum
      have hnn : ∀ i' ∈ Finset.univ.filter (fun i' => rep i' = rep i), 0 ≤ s i' * lam i' :=
        fun i' _ => mul_nonneg (h.pos i').le (hlam i')
      exact (Finset.sum_eq_zero_iff_of_nonneg hnn).mp hsum i (by simp)
    have hl : lam i = 0 := by
      rcases mul_eq_zero.mp hzero with h0 | h0
      · exact absurd h0 (h.pos i).ne'
      · exact h0
    rw [hl, zero_mul]

end Twins

end Rpopt
