/-
Claimed candidates (mplcp.rs `seen_claim`, `Slot::Rejected`): a candidate active set is claimed in
`Seen` before its region is built, and a claimed set is never tried again. If the build fails, the set
stays claimed as rejected.

`reach_of_claims`: the built regions are exactly the explored set as long as the builder rejects only
sets that are not neighbours of a built region (that is, not regions). A genuine region that the
builder rejects (a thin region whose Chebyshev LP stopped at radius 0, before the cold retry) breaks
the hypothesis, and nothing else can recover it.
-/
import RpoptProofs.Coverage

namespace Rpopt

/-- **Claims lose nothing when only non-regions are rejected.** `E` is the set of built regions, `J`
the set of claimed and rejected candidates. If the start is built, every neighbour of a built region
is built or rejected, every built region is reachable, and no rejected candidate is a neighbour of a
built region, then the built regions are exactly the reachable set. -/
theorem reach_of_claims {α : Type*} {adj : α → α → Prop} {s : α} {E J : Set α} (hs : s ∈ E)
    (hcl : ∀ x ∈ E, ∀ y, adj x y → y ∈ E ∨ y ∈ J) (hJ : ∀ y ∈ J, ∀ x ∈ E, ¬ adj x y)
    (hE : E ⊆ Reach adj s) : E = Reach adj s :=
  Set.Subset.antisymm hE <| reach_least hs fun x hx y hxy =>
    (hcl x hx y hxy).resolve_right fun hy => hJ y hy x hx hxy

end Rpopt
