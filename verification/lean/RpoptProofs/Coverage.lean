/-
Breadth-first exploration as a reachability closure (mplcp.rs `pivot_solve`, `pivot_solve_par`;
mplp_lex.rs `solve`).

The `seen` set and the queue compute the least set that contains the start region and is closed
under the neighbour relation the algorithm actually evaluates (`adj'`); the order (breadth first,
waves in parallel) does not matter for the set. This file proves the graph part of coverage:

* `reach_least`: the explored set is the least closed set containing the start;
* `reach_skip`: skipping neighbour candidates that are already explored (the entry-facet skip,
  known-neighbour skipping, `crossed` in mplp_lex) leaves the explored set unchanged;
* `reach_skip_parent`: the special case of the entry-facet skip (the skipped neighbour is the
  region's discovery parent);
* `coverage_of_connected`: if the facet-adjacency graph is connected on a set of regions, the
  search reaches all of them.

The geometric input, that the facet-adjacency graph of a facet-to-facet polyhedral partition of
a convex set is connected (A6), is NOT proved here; it is the explicit hypothesis of
`coverage_of_connected`.
-/
import Mathlib

namespace Rpopt

variable {α : Type*}

/-- Regions reachable from `s` by neighbour steps. -/
def Reach (adj : α → α → Prop) (s : α) : Set α := {x | Relation.ReflTransGen adj s x}

theorem start_mem_reach (adj : α → α → Prop) (s : α) : s ∈ Reach adj s :=
  Relation.ReflTransGen.refl

theorem reach_closed {adj : α → α → Prop} {s x y : α} (hx : x ∈ Reach adj s) (h : adj x y) :
    y ∈ Reach adj s :=
  Relation.ReflTransGen.tail hx h

/-- The explored set is the least set containing the start and closed under the neighbour
relation (any worklist order computes it). -/
theorem reach_least {adj : α → α → Prop} {s : α} {S : Set α} (hs : s ∈ S)
    (hcl : ∀ x ∈ S, ∀ y, adj x y → y ∈ S) : Reach adj s ⊆ S := by
  intro x hx
  induction hx with
  | refl => exact hs
  | tail _ hab ih => exact hcl _ ih _ hab

/-- **Skipping known neighbours.** If the evaluated relation `adj'` is part of `adj`, and every
candidate it omits from an explored region is itself explored, the explored set is the same. -/
theorem reach_skip {adj adj' : α → α → Prop} {s : α} (hsub : ∀ x y, adj' x y → adj x y)
    (hskip : ∀ x ∈ Reach adj' s, ∀ y, adj x y → ¬ adj' x y → y ∈ Reach adj' s) :
    Reach adj' s = Reach adj s := by
  apply Set.Subset.antisymm
  · intro x hx
    induction hx with
    | refl => exact start_mem_reach _ _
    | tail _ hab ih => exact reach_closed ih (hsub _ _ hab)
  · refine reach_least (start_mem_reach _ _) fun x hx y hxy => ?_
    by_cases h : adj' x y
    · exact reach_closed hx h
    · exact hskip x hx y hxy h

/-- **Entry-facet skip, graph form.** If the only omitted neighbours of an explored region are
its discovery parent (an explored region), nothing is lost. -/
theorem reach_skip_parent {adj adj' : α → α → Prop} {s : α} (parent : α → α)
    (hsub : ∀ x y, adj' x y → adj x y)
    (hparent : ∀ x ∈ Reach adj' s, x ≠ s → parent x ∈ Reach adj' s)
    (honly : ∀ x y, adj x y → ¬ adj' x y → x ≠ s ∧ y = parent x) :
    Reach adj' s = Reach adj s :=
  reach_skip hsub fun x hx y hxy hn => by
    obtain ⟨hxs, rfl⟩ := honly x y hxy hn
    exact hparent x hx hxs

/-- **Coverage (conditional).** If every region of `R` is connected to the start through the
facet-adjacency graph (hypothesis, the geometric part of A6), the search reaches all of `R`. -/
theorem coverage_of_connected {adj : α → α → Prop} {s : α} {R : Set α}
    (hconn : ∀ x ∈ R, Relation.ReflTransGen adj s x) : R ⊆ Reach adj s :=
  fun x hx => hconn x hx

end Rpopt
