/-
B2. Simplex pivots of mplp_lex.rs (`Lex::pivot`, phase 1 and phase 2 of `Lex::solve`).

A basis is a map `b : K → m` from basis positions to constraint rows (`Work::b`), with
`β = A_B⁻¹` (`Work::binv`, `A_B β = 1`) and `T = A β` (`Work::t`).

* `rank1_inverse`, `rank1_tableau`: `Lex::pivot` (row `j` enters at position `p`) produces the
  inverse of the new basis matrix and the new `T`.
* `ratio_test`, `lex_ratio_test`: the (lexicographic) minimum ratio keeps every nonnegative
  quantity nonnegative and makes the chosen one exactly zero.
* `dual_pivot`: phase 1 (dual simplex with the artificial multipliers `cp`): the ratio test over
  `y = T_jp > 0` keeps the multipliers dual feasible, the leaving position's multiplier becomes 0
  and the entering row gets `t ≥ 0`.
* `primal_direction`: phase 2 (primal simplex): along `dir = -β e_p` the other basic rows stay
  active, the leaving row gets slack `t`, the nonbasic slacks change by `-t a_j` with
  `a_j = -T_jp`, and the cost changes by `t λ_p`.
-/
import RpoptProofs.Lex

open Matrix

set_option linter.unusedSectionVars false

namespace Rpopt

section Ratio

variable {ι : Type*}

/-- **Ratio test.** With `t = s_l / a_l` minimal over `a_j > 0`, every `s_j - t a_j` stays
nonnegative and the `l` one becomes zero. -/
theorem ratio_test {S : Set ι} {s a : ι → ℝ} (hs : ∀ j ∈ S, 0 ≤ s j) {l : ι} (hl : l ∈ S)
    (hal : 0 < a l) (hmin : ∀ j ∈ S, 0 < a j → s l / a l ≤ s j / a j) :
    0 ≤ s l / a l ∧ (∀ j ∈ S, 0 ≤ s j - s l / a l * a j) ∧ s l - s l / a l * a l = 0 := by
  refine ⟨div_nonneg (hs l hl) hal.le, fun j hj => ?_, by field_simp; ring⟩
  by_cases haj : 0 < a j
  · have := hmin j hj haj
    rw [le_div_iff₀ haj] at this
    linarith
  · have : s l / a l * a j ≤ 0 :=
      mul_nonpos_of_nonneg_of_nonpos (div_nonneg (hs l hl) hal.le) (not_lt.mp haj)
    linarith [hs j hj]

variable {N : ℕ}

/-- **Lexicographic ratio test** (`slack_cmp` in phase 2). With the unique lexicographic minimum
`l` of `s_j / a_j` over `a_j > 0`, the step `s_l / a_l` is lexico-positive and every other row
`s_j - (a_j / a_l) s_l` stays lexico-positive. -/
theorem lex_ratio_test {S : Set ι} {s : ι → Fin N → ℝ} {a : ι → ℝ}
    (hs : ∀ j ∈ S, LexPos (s j)) {l : ι} (hl : l ∈ S) (hal : 0 < a l)
    (hmin : ∀ j ∈ S, j ≠ l → 0 < a j → LexPos ((a j)⁻¹ • s j - (a l)⁻¹ • s l)) :
    LexPos ((a l)⁻¹ • s l) ∧ ∀ j ∈ S, j ≠ l → LexPos (s j - (a j / a l) • s l) := by
  refine ⟨(hs l hl).smul (inv_pos.mpr hal), fun j hj hjl => ?_⟩
  by_cases haj : 0 < a j
  · have e : s j - (a j / a l) • s l = a j • ((a j)⁻¹ • s j - (a l)⁻¹ • s l) := by
      rw [smul_sub, smul_smul, smul_smul, mul_inv_cancel₀ haj.ne', one_smul]
      congr 2
    rw [e]
    exact (hmin j hj hjl haj).smul haj
  · have hc : 0 ≤ -(a j / a l) := by
      rw [← neg_div]; exact div_nonneg (by linarith [not_lt.mp haj]) hal.le
    rw [sub_eq_add_neg, ← neg_smul]
    exact (hs j hj).add (LexNonneg.smul (Or.inr (hs l hl)) hc)

end Ratio

section Basis

variable {m K : Type*} [Fintype K] [DecidableEq K]

/-- The basis matrix: row `p` is the constraint row `b p`. -/
def basisMat (A : Matrix m K ℝ) (b : K → m) : Matrix K K ℝ := fun p k => A (b p) k

/-- The update of `Lex::pivot` (mplp_lex.rs) applied to the columns of `β` (or of `T`):
column `p` is divided by `u_p`, every other column `q` gets `- (u_q / u_p) ×` column `p`. -/
noncomputable def rank1 {R : Type*} (β : Matrix R K ℝ) (u : K → ℝ) (p : K) : Matrix R K ℝ :=
  fun k q => if q = p then β k p / u p else β k q - β k p * u q / u p

theorem vecMul_rank1 {R : Type*} [Fintype R] (a : R → ℝ) (β : Matrix R K ℝ) (u : K → ℝ)
    (p : K) (q : K) :
    (a ᵥ* rank1 β u p) q =
      if q = p then (a ᵥ* β) p / u p else (a ᵥ* β) q - (a ᵥ* β) p * u q / u p := by
  by_cases hq : q = p
  · simp only [vecMul, dotProduct, rank1, hq, if_true, Finset.sum_div]
    exact Finset.sum_congr rfl fun k _ => by ring
  · simp only [vecMul, dotProduct, rank1, hq, if_false, Finset.sum_div, Finset.sum_mul,
      ← Finset.sum_sub_distrib]
    exact Finset.sum_congr rfl fun k _ => by ring

/-- **`Lex::pivot` keeps `β = A_B⁻¹`.** If `A_B β = 1` and row `j` enters at position `p` with
`u = A_j β` (the row `T_j`) and `u_p ≠ 0`, the updated `β` is the inverse of the new basis
matrix. -/
theorem rank1_inverse [Fintype m] (A : Matrix m K ℝ) (b : K → m) {β : Matrix K K ℝ}
    (hβ : basisMat A b * β = 1) (j : m) (p : K) (hu : (A j ᵥ* β) p ≠ 0) :
    basisMat A (Function.update b p j) * rank1 β (A j ᵥ* β) p = 1 := by
  have hrow : ∀ p' : K, (fun k => A (b p') k) ᵥ* β = fun q => if p' = q then 1 else 0 := by
    intro p'; funext q
    have := congrFun (congrFun hβ p') q
    simpa [Matrix.mul_apply, basisMat, vecMul, dotProduct, Matrix.one_apply] using this
  ext p' q
  have hmul : (basisMat A (Function.update b p j) * rank1 β (A j ᵥ* β) p) p' q =
      ((fun k => A (Function.update b p j p') k) ᵥ* rank1 β (A j ᵥ* β) p) q := by
    simp [Matrix.mul_apply, basisMat, vecMul, dotProduct]
  rw [hmul, vecMul_rank1, Matrix.one_apply]
  by_cases hp' : p' = p
  · subst hp'
    simp only [Function.update_self]
    have e : (fun k => A j k) = A j := rfl
    rw [e]
    by_cases hq : q = p'
    · subst hq; simp [hu]
    · simp [hq, Ne.symm hq]; field_simp; ring
  · simp only [Function.update_of_ne hp', hrow]
    by_cases hq : q = p
    · subst hq; simp [hp']
    · simp [hq, hp']

/-- **`Lex::pivot` keeps `T = A β`**: the same update applied to `T` gives `A` times the new `β`. -/
theorem rank1_tableau [Fintype m] (A : Matrix m K ℝ) (β : Matrix K K ℝ) (u : K → ℝ) (p : K) :
    A * rank1 β u p = rank1 (A * β) u p := by
  ext i q
  have h := vecMul_rank1 (A i) β u p q
  simp only [vecMul, dotProduct] at h
  simp only [Matrix.mul_apply, rank1] at h ⊢
  rw [h]

/-- Every constraint row is a combination of the basis rows: `A_j = (A_j β) A_B`. -/
theorem row_eq_combination [Fintype m] (A : Matrix m K ℝ) (b : K → m) {β : Matrix K K ℝ}
    (hβ : basisMat A b * β = 1) (j : m) : A j = (A j ᵥ* β) ᵥ* basisMat A b := by
  have h' : β * basisMat A b = 1 := mul_eq_one_comm.mp hβ
  rw [vecMul_vecMul, h', vecMul_one]

/-- **B2, phase 1 (dual simplex step).** Multipliers by basis position, stationarity
`c + λ A_B = 0` and `λ ≥ 0`. Row `j` enters at position `p` chosen by the ratio test over
`u_q = T_jq > 0`, `t = λ_p / u_p`. The new multipliers (`λ_q - t u_q`, and `t` for the entering
row at position `p`) satisfy stationarity for the new basis and are nonnegative; the leaving
position's old multiplier is driven to zero. -/
theorem dual_pivot [Fintype m] (A : Matrix m K ℝ) (b : K → m) {β : Matrix K K ℝ}
    (hβ : basisMat A b * β = 1) (c : K → ℝ) {lam : K → ℝ}
    (hstat : c + lam ᵥ* basisMat A b = 0) (hlam : ∀ q, 0 ≤ lam q) (j : m) (p : K)
    (hup : 0 < (A j ᵥ* β) p)
    (hmin : ∀ q, 0 < (A j ᵥ* β) q → lam p / (A j ᵥ* β) p ≤ lam q / (A j ᵥ* β) q) :
    let u := A j ᵥ* β
    let t := lam p / u p
    let lam' := Function.update (lam - t • u) p t
    c + lam' ᵥ* basisMat A (Function.update b p j) = 0 ∧ (∀ q, 0 ≤ lam' q) ∧
      lam p - t * u p = 0 := by
  intro u t lam'
  obtain ⟨ht, hnn, hzero⟩ := ratio_test (S := Set.univ) (s := lam) (a := u)
    (fun q _ => hlam q) (Set.mem_univ p) hup (fun q _ hq => hmin q hq)
  refine ⟨?_, fun q => ?_, hzero⟩
  · have hrow := row_eq_combination A b hβ j
    -- split off position p in both products
    have hsplit : ∀ (μ : K → ℝ) (bb : K → m) (k : K), (μ ᵥ* basisMat A bb) k =
        μ p * A (bb p) k + ∑ q ∈ Finset.univ.erase p, μ q * A (bb q) k := by
      intro μ bb k
      simp only [vecMul, dotProduct, basisMat]
      exact (Finset.add_sum_erase Finset.univ (fun q => μ q * A (bb q) k)
        (Finset.mem_univ p)).symm
    funext k
    have e1 := hsplit lam' (Function.update b p j) k
    have e2 := hsplit (lam - t • u) b k
    have hs : ∑ q ∈ Finset.univ.erase p, lam' q * A (Function.update b p j q) k =
        ∑ q ∈ Finset.univ.erase p, (lam - t • u) q * A (b q) k := by
      refine Finset.sum_congr rfl fun q hq => ?_
      have hqp := Finset.ne_of_mem_erase hq
      simp [lam', Function.update_of_ne hqp]
    have hp0 : (lam - t • u) p = 0 := by
      simp only [Pi.sub_apply, Pi.smul_apply, smul_eq_mul]; exact hzero
    have hlin : (lam - t • u) ᵥ* basisMat A b = lam ᵥ* basisMat A b - t • A j := by
      rw [sub_vecMul, smul_vecMul, ← hrow]
    have hk := congrFun hlin k
    have hst := congrFun hstat k
    simp only [Pi.add_apply, Pi.zero_apply, Pi.sub_apply, Pi.smul_apply, smul_eq_mul] at hk hst ⊢
    rw [e1, hs, Function.update_self]
    simp only [lam', Function.update_self]
    rw [e2, hp0, zero_mul, zero_add] at hk
    linarith
  · by_cases hq : q = p
    · subst hq; simp only [lam', Function.update_self]; exact ht
    · simp only [lam', Function.update_of_ne hq, Pi.sub_apply, Pi.smul_apply, smul_eq_mul]
      have := hnn q (Set.mem_univ q)
      linarith [mul_comm (lam p / u p) (u q)]

/-- **B2, phase 2 (primal simplex direction).** With `dir = -β e_p`: the basic rows other than
`p` stay active, the leaving row `b p` gets slack `t`, the slack of any row `j` changes by
`-t a_j` with `a_j = A_j dir = -T_jp` (Rust `aj = -w.t[j * nx + p]`), and with stationarity
`d + λ A_B = 0` the cost changes by `t λ_p` (decreases when the leaving multiplier is negative). -/
theorem primal_direction [Fintype m] (A : Matrix m K ℝ) (b : K → m) {β : Matrix K K ℝ}
    (hβ : basisMat A b * β = 1) (p : K) :
    let dir := -(β *ᵥ Pi.single p 1)
    basisMat A b *ᵥ dir = -Pi.single p 1 ∧
      (∀ j, (A *ᵥ dir) j = -(A * β) j p) ∧
      (∀ (d : K → ℝ) (lam : K → ℝ), d + lam ᵥ* basisMat A b = 0 → d ⬝ᵥ dir = lam p) := by
  intro dir
  have h1 : basisMat A b *ᵥ dir = -Pi.single p 1 := by
    simp only [dir, mulVec_neg, mulVec_mulVec, hβ, one_mulVec]
  refine ⟨h1, fun j => ?_, fun d lam hst => ?_⟩
  · simp only [dir, mulVec_neg, mulVec_mulVec, Pi.neg_apply, neg_inj]
    simp [mulVec, dotProduct, Pi.single_apply]
  · have hd : d = -(lam ᵥ* basisMat A b) := by linear_combination hst
    rw [hd, neg_dotProduct, ← dotProduct_mulVec, h1]
    simp [dotProduct, Pi.single_apply]

end Basis

end Rpopt
