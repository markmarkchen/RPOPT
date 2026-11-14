//! Point location for explicit solutions: given θ, find the critical region that contains it.
//!
//! Every method implements [`PointLocator`] and answers the same question as PPOPT's
//! `PointLocation.locate`: the index of a region containing θ, or `None` if θ lies outside the
//! feasible parameter set. All methods are exact up to the boundary tolerance and share one tie
//! rule ([`pick_strict_first`], the rule of `Solution::get_region`): the first region that
//! contains θ with tolerance 0, otherwise the first within [`LOCATE_TOL`]. A point on a shared
//! facet therefore goes to the same neighbour in every method. Classification inside the trees
//! and tables is conservative: a region is dropped from a cell or a branch only when several
//! independent checks agree that it cannot contain a point there.

pub mod bst;
pub mod descriptor;
pub mod geometry;
pub mod graph_traversal;
pub mod hash_table;
pub mod linear_machine;
pub mod multiway;
pub mod opt_tree;
pub mod sequential;
pub mod walk;
pub mod yann;

pub use graph_traversal::GraphTraversal;
pub use walk::SubdivisionWalk;
pub use bst::BinarySearchTree;
pub use descriptor::DescriptorFunction;
pub use geometry::PartitionGeometry;
pub use hash_table::HashTable;
pub use linear_machine::LinearMachine;
pub use multiway::MultiwayTree;
pub use opt_tree::{Candidates, Criterion, OptTreeOptions, OptimizedTree};
pub use sequential::{FlatSequential, SequentialSearch};
pub use yann::Yann;

pub trait PointLocator: Send + Sync {
    fn name(&self) -> &'static str;
    /// Index of the critical region containing θ, or None.
    fn locate(&self, theta: &[f64]) -> Option<usize>;
    /// Number of stored floating point values (online memory footprint).
    fn storage_floats(&self) -> usize;
}

/// Boundary tolerance shared by all locators for the final membership checks.
pub const LOCATE_TOL: f64 = 1e-9;

/// First candidate that contains theta strictly (tolerance 0); only when there is none, the first
/// one within `LOCATE_TOL`. `violation(j)` is the largest row violation of region j, or infinity
/// as soon as one row exceeds `LOCATE_TOL`. Interior points return at the first strict hit, so the
/// usual cost equals that of a plain first-within-tolerance scan.
#[inline]
pub fn pick_strict_first(cands: impl Iterator<Item = usize>, mut violation: impl FnMut(usize) -> f64) -> Option<usize> {
    let mut fallback = None;
    for j in cands {
        let v = violation(j);
        if v <= 0.0 {
            return Some(j);
        }
        if v <= LOCATE_TOL && fallback.is_none() {
            fallback = Some(j);
        }
    }
    fallback
}

/// [`pick_strict_first`] over the candidate list of a tree leaf, plus a rescan of all `num_regions`
/// regions when the leaf only offers a region that holds θ within `LOCATE_TOL`. A tree separates
/// regions by classifying their vertices against its planes with a tolerance, so a point within
/// that tolerance of a plane can reach a leaf that lacks the region containing it strictly; the
/// rescan restores the shared tie rule. It only runs for points on or within `LOCATE_TOL` of a
/// region boundary that no candidate contains strictly, never for interior points or for points
/// far outside the partition.
#[inline]
pub fn pick_strict_first_rescan(cands: impl Iterator<Item = usize>, num_regions: usize, mut violation: impl FnMut(usize) -> f64) -> Option<usize> {
    let mut fallback = None;
    for j in cands {
        let v = violation(j);
        if v <= 0.0 {
            return Some(j);
        }
        if v <= LOCATE_TOL && fallback.is_none() {
            fallback = Some(j);
        }
    }
    if fallback.is_some() {
        return pick_strict_first(0..num_regions, violation);
    }
    None
}
