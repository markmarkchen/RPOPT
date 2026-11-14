//! Binary search tree over the distinct facet hyperplanes (Tøndel, Johansen & Bemporad, 2003,
//! "Evaluation of piecewise affine control via binary search tree", Automatica 39(5)).
//!
//! Every node stores one hyperplane a'θ <= b. Each region is pre-classified against every
//! hyperplane from its vertices (entirely below, entirely above, or split). At a node with region
//! index set I, the hyperplane minimising max(|I-|, |I+|) (ties: min |I-| + |I+|) is selected and
//! split regions are re-checked with an LP on the node's polytope (the Tøndel refinement), so a
//! region follows a branch only if it meets that branch with nonzero volume. Leaves keep the few
//! remaining candidates, which are checked directly.

use super::geometry::{has_interior, PartitionGeometry};
use super::{PointLocator, LOCATE_TOL};
use crate::linalg::{dot, Mat};

#[derive(Clone, Debug)]
pub enum BstNode {
    Split { plane: Vec<f64>, below: usize, above: usize },
    Leaf { candidates: Vec<usize> },
}

pub struct BinarySearchTree {
    pub dim: usize,
    pub nodes: Vec<BstNode>,
    geom_rows: Vec<f64>, // [a | b] per row, per region
    offsets: Vec<usize>,
    pub depth: usize,
    /// construction stopped at `max_nodes`
    pub truncated: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Below,
    Above,
    Both,
}

pub struct BstOptions {
    /// stop splitting at this many candidates
    pub leaf_size: usize,
    /// re-check split regions on the node polytope with an LP (Tøndel refinement)
    pub exact_refinement: bool,
    pub max_depth: usize,
    /// per-region query probability: balance probability mass instead of region counts (expected
    /// query depth for that query distribution); None counts regions
    pub weights: Option<Vec<f64>>,
    /// stop splitting once the tree has this many nodes (the open work becomes big leaves, still exact)
    pub max_nodes: usize,
}

impl Default for BstOptions {
    fn default() -> Self {
        BstOptions { leaf_size: 1, exact_refinement: true, max_depth: 64, weights: None, max_nodes: usize::MAX }
    }
}

impl BinarySearchTree {
    pub fn new(geom: &PartitionGeometry) -> Self {
        Self::with_options(geom, &BstOptions::default())
    }

    pub fn with_options(geom: &PartitionGeometry, opts: &BstOptions) -> Self {
        let n = geom.dim;
        let np = geom.planes.len();
        let r = geom.num_regions;
        // classification of each region against each plane from its vertices
        let tol = 1e-9;
        let mut side = vec![Side::Both; r * np];
        for i in 0..r {
            for (p, pl) in geom.planes.iter().enumerate() {
                let mut lo = false;
                let mut hi = false;
                for v in &geom.vertices[i] {
                    let s = dot(&pl[..n], v) - pl[n];
                    if s < -tol {
                        lo = true;
                    } else if s > tol {
                        hi = true;
                    }
                }
                side[i * np + p] = match (lo, hi) {
                    (true, false) => Side::Below,
                    (false, true) => Side::Above,
                    (false, false) => Side::Below, // degenerate (lies in the plane): never happens for full-dim regions
                    (true, true) => Side::Both,
                };
            }
        }
        let wts = opts.weights.as_deref();
        let mut geom_rows = vec![];
        for i in 0..r {
            let (e, f) = geom.region_rows(i);
            for k in 0..f.len() {
                geom_rows.extend_from_slice(&e[k * n..(k + 1) * n]);
                geom_rows.push(f[k]);
            }
        }
        let mut tree = BinarySearchTree { dim: n, nodes: vec![], geom_rows, offsets: geom.offsets.clone(), depth: 0, truncated: false };
        // iterative build: (node index, candidate set, path halfspaces, depth)
        tree.nodes.push(BstNode::Leaf { candidates: vec![] });
        let mut work: Vec<(usize, Vec<usize>, Vec<Vec<f64>>, usize)> = vec![(0, (0..r).collect(), vec![], 0)];
        while let Some((node, set, path, depth)) = work.pop() {
            tree.depth = tree.depth.max(depth);
            if tree.nodes.len() >= opts.max_nodes {
                tree.truncated = true;
            }
            if set.len() <= opts.leaf_size || depth >= opts.max_depth || tree.truncated {
                tree.nodes[node] = BstNode::Leaf { candidates: set };
                continue;
            }
            // choose the most balanced plane
            let mut best: Option<(usize, f64, f64)> = None; // (plane, max, sum)
            for p in 0..np {
                let (mut nb, mut na) = (0usize, 0usize);
                let (mut wb, mut wa) = (0.0f64, 0.0f64);
                for &i in &set {
                    let w = wts.map(|w| w[i]).unwrap_or(1.0);
                    match side[i * np + p] {
                        Side::Below => {
                            nb += 1;
                            wb += w;
                        }
                        Side::Above => {
                            na += 1;
                            wa += w;
                        }
                        Side::Both => {
                            nb += 1;
                            na += 1;
                            wb += w;
                            wa += w;
                        }
                    }
                }
                if nb == set.len() && na == set.len() {
                    continue;
                }
                if nb == 0 || na == 0 {
                    continue;
                }
                let key = (wb.max(wa), wb + wa);
                if best.map(|(_, m, s)| key < (m, s)).unwrap_or(true) {
                    best = Some((p, key.0, key.1));
                }
            }
            let Some((p, _, _)) = best else {
                tree.nodes[node] = BstNode::Leaf { candidates: set };
                continue;
            };
            let pl = geom.planes[p].clone();
            let mut below = vec![];
            let mut above = vec![];
            for &i in &set {
                match side[i * np + p] {
                    Side::Below => below.push(i),
                    Side::Above => above.push(i),
                    Side::Both => {
                        if opts.exact_refinement {
                            let (in_b, in_a) = split_check(geom, i, &path, &pl);
                            if in_b {
                                below.push(i);
                            }
                            if in_a {
                                above.push(i);
                            }
                        } else {
                            below.push(i);
                            above.push(i);
                        }
                    }
                }
            }
            if below.len() == set.len() && above.len() == set.len() {
                tree.nodes[node] = BstNode::Leaf { candidates: set };
                continue;
            }
            let bi = tree.nodes.len();
            tree.nodes.push(BstNode::Leaf { candidates: vec![] });
            let ai = tree.nodes.len();
            tree.nodes.push(BstNode::Leaf { candidates: vec![] });
            tree.nodes[node] = BstNode::Split { plane: pl.clone(), below: bi, above: ai };
            let mut pb = path.clone();
            pb.push(pl.clone());
            let mut pa = path;
            let mut neg: Vec<f64> = pl.iter().map(|v| -v).collect();
            neg[n] = -pl[n];
            pa.push(neg);
            work.push((bi, below, pb, depth + 1));
            work.push((ai, above, pa, depth + 1));
        }
        tree
    }

    #[inline]
    fn region_violation(&self, j: usize, theta: &[f64]) -> f64 {
        let w = self.dim + 1;
        let n = self.dim;
        let mut worst = f64::NEG_INFINITY;
        for k in self.offsets[j]..self.offsets[j + 1] {
            let r = &self.geom_rows[k * w..(k + 1) * w];
            let v = dot(&r[..n], theta) - r[n];
            if v > LOCATE_TOL {
                return f64::INFINITY;
            }
            worst = worst.max(v);
        }
        worst
    }

    #[inline]
    #[allow(dead_code)]
    fn region_contains(&self, j: usize, theta: &[f64]) -> bool {
        let w = self.dim + 1;
        let n = self.dim;
        for k in self.offsets[j]..self.offsets[j + 1] {
            let r = &self.geom_rows[k * w..(k + 1) * w];
            if dot(&r[..n], theta) > r[n] + LOCATE_TOL {
                return false;
            }
        }
        true
    }

    /// Leaf reached by θ (without the final membership check).
    pub fn leaf(&self, theta: &[f64]) -> &[usize] {
        let n = self.dim;
        let mut node = 0;
        loop {
            match &self.nodes[node] {
                BstNode::Split { plane, below, above } => {
                    node = if dot(&plane[..n], theta) <= plane[n] { *below } else { *above };
                }
                BstNode::Leaf { candidates } => return candidates,
            }
        }
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }
}

/// Does region i ∩ path polytope meet the closed below / above side of `pl` with volume?
fn split_check(geom: &PartitionGeometry, i: usize, path: &[Vec<f64>], pl: &[f64]) -> (bool, bool) {
    let n = geom.dim;
    let (a, b) = geom.region_mat(i);
    let mut rows = a;
    let mut rhs = b;
    for h in path {
        rows.push_row(&h[..n]);
        rhs.push(h[n]);
    }
    // known points of the region (a hint that the cut piece has an interior) and the scale of the
    // shifted second LP; a missed region is a wrong answer, so "no" needs more than one LP
    let centroid: Vec<f64> = {
        let vs = &geom.vertices[i];
        (0..n).map(|k| if vs.is_empty() { geom.centers[i][k] } else { vs.iter().map(|v| v[k]).sum::<f64>() / vs.len() as f64 }).collect()
    };
    let hints = [geom.centers[i].as_slice(), centroid.as_slice()];
    let scale = (0..n).map(|k| 0.5 * (geom.bbox_hi[i][k] - geom.bbox_lo[i][k])).fold(0.0f64, f64::max);
    let check = |sign: f64| -> bool {
        let mut r2 = rows.clone();
        let mut b2 = rhs.clone();
        let row: Vec<f64> = pl[..n].iter().map(|v| sign * v).collect();
        r2.push_row(&row);
        b2.push(sign * pl[n]);
        has_interior(&r2, &b2, 1e-9, &hints, &geom.centers[i], scale)
    };
    let _ = Mat::zeros(0, 0);
    (check(1.0), check(-1.0))
}

impl PointLocator for BinarySearchTree {
    fn name(&self) -> &'static str {
        "binary search tree"
    }

    fn locate(&self, theta: &[f64]) -> Option<usize> {
        super::pick_strict_first_rescan(self.leaf(theta).iter().cloned(), self.offsets.len() - 1, |j| self.region_violation(j, theta))
    }

    fn storage_floats(&self) -> usize {
        let tree: usize = self
            .nodes
            .iter()
            .map(|nd| match nd {
                BstNode::Split { plane, .. } => plane.len() + 2,
                BstNode::Leaf { candidates } => candidates.len(),
            })
            .sum();
        tree + self.geom_rows.len()
    }
}
