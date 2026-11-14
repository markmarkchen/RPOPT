//! Point location for explicit solutions: given θ, find the critical region that contains it.
//!
//! Every method implements [`PointLocator`] and answers the same question as PPOPT's
//! `PointLocation.locate`: the index of a region containing θ, or `None` if θ lies outside the
//! feasible parameter set. All methods are exact up to the boundary tolerance (points on a shared
//! facet may be assigned to either neighbour).

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
