/-
The flat `Seen` table (mplcp `seen_set.rs`): 128-bit Zobrist keys (the XOR of one random word per
index) and open addressing that compares the stored set on every hit.

* `zobrist_toggle`: the key of `A xor {i}` is the key of `A` XOR the word of `i` (the update
  `idx_settled` uses), in any group where every element is its own inverse.
* `verified_lookup_iff`: when every entry is stored under its own set's key, a lookup that accepts only
  entries whose stored set equals the query finds exactly the stored sets, whatever the key function
  (collisions included). The key only chooses where to look; exactness rests on the comparison.
-/
import RpoptProofs.Pivot

namespace Rpopt

/-- **The incremental key.** With `g + g = 0` in `G` (XOR on bit strings), the key of the toggled
set is the key of the set plus the word of the toggled index. -/
theorem zobrist_toggle {α G : Type*} [Fintype α] [DecidableEq α] [AddCommGroup G]
    (h2 : ∀ g : G, g + g = 0) (z : α → G) (S : Finset α) (i : α) :
    ∑ x ∈ toggle S i, z x = (∑ x ∈ S, z x) + z i := by
  unfold toggle
  by_cases hi : i ∈ S
  · rw [if_pos hi]
    have e := Finset.sum_erase_add S z hi
    rw [← e, add_assoc, h2, add_zero]
  · rw [if_neg hi, Finset.sum_insert hi, add_comm]

/-- **A verified lookup is exact for any key.** If every stored entry carries the key of its own
set, then "some entry has the query's key and stores the query" holds exactly when "some entry stores
the query". -/
theorem verified_lookup_iff {S K : Type*} (key : S → K) (table : Set (K × S))
    (hstored : ∀ e ∈ table, e.1 = key e.2) (q : S) :
    (∃ e ∈ table, e.1 = key q ∧ e.2 = q) ↔ ∃ e ∈ table, e.2 = q := by
  constructor
  · rintro ⟨e, he, -, hq⟩
    exact ⟨e, he, hq⟩
  · rintro ⟨e, he, hq⟩
    exact ⟨e, he, by rw [hstored e he, hq], hq⟩

end Rpopt
