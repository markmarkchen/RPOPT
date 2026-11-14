//! Multiway tree (Mönnigmann & Kastsian, 2011, "Fast explicit MPC with multiway trees", IFAC
//! World Congress).
//!
//! A multiway node of order m hides a complete binary subtree of depth d = log2(m + 1) of the
//! binary search tree: its m hyperplanes are evaluated together (one small dense mat-vec, which
//! the compiler vectorises), then the sign bits select one of up to m + 1 children. The tree
//! height drops by a factor d. The multiway tree is derived from an existing binary tree, as in
//! the paper.

use super::bst::{BinarySearchTree, BstNode};
use super::{PointLocator, LOCATE_TOL};
use crate::linalg::dot;

#[derive(Clone, Copy, Debug)]
enum Slot {
    /// index of a plane inside the node
    Plane(u32),
    /// next multiway node
    Child(u32),
    /// leaf: index into `leaves`
    Leaf(u32),
}

struct MwNode {
    /// k planes, (dim + 1) floats each
    planes: Vec<f64>,
    /// heap-ordered slots of the hidden binary subtree, length 2^(d+1) - 1
    slots: Vec<Slot>,
}

pub struct MultiwayTree {
    dim: usize,
    depth_per_node: usize,
    nodes: Vec<MwNode>,
    leaves: Vec<Vec<usize>>,
    rows: Vec<f64>,
    offsets: Vec<usize>,
    pub height: usize,
}

impl MultiwayTree {
    /// `levels`: number of binary levels collapsed into each multiway node (order m = 2^levels - 1).
    pub fn from_bst(bst: &BinarySearchTree, levels: usize, rows: Vec<f64>, offsets: Vec<usize>) -> Self {
        let d = levels.max(1);
        let n = bst.dim;
        let mut mw = MultiwayTree { dim: n, depth_per_node: d, nodes: vec![], leaves: vec![], rows, offsets, height: 0 };
        // map: BST node -> multiway node, built breadth first
        let mut queue: std::collections::VecDeque<(usize, usize, usize)> = std::collections::VecDeque::new(); // (bst root of node, mw id, height)
        mw.nodes.push(MwNode { planes: vec![], slots: vec![] });
        queue.push_back((0, 0, 1));
        while let Some((root, id, h)) = queue.pop_front() {
            mw.height = mw.height.max(h);
            let nslots = (1usize << (d + 1)) - 1;
            let mut slots = vec![Slot::Leaf(u32::MAX); nslots];
            let mut planes = vec![];
            // heap walk: slot s holds BST node at relative depth level(s)
            let mut stack = vec![(0usize, root, 0usize)];
            while let Some((s, bn, lvl)) = stack.pop() {
                match &bst.nodes[bn] {
                    BstNode::Leaf { candidates } => {
                        mw.leaves.push(candidates.clone());
                        slots[s] = Slot::Leaf((mw.leaves.len() - 1) as u32);
                    }
                    BstNode::Split { plane, below, above } => {
                        if lvl == d {
                            // boundary of this multiway node: start a child node
                            let cid = mw.nodes.len();
                            mw.nodes.push(MwNode { planes: vec![], slots: vec![] });
                            queue.push_back((bn, cid, h + 1));
                            slots[s] = Slot::Child(cid as u32);
                        } else {
                            let pid = planes.len() / (n + 1);
                            planes.extend_from_slice(plane);
                            slots[s] = Slot::Plane(pid as u32);
                            stack.push((2 * s + 1, *below, lvl + 1));
                            stack.push((2 * s + 2, *above, lvl + 1));
                        }
                    }
                }
            }
            mw.nodes[id] = MwNode { planes, slots };
        }
        mw
    }

    /// Largest row violation of region j, infinity once a row exceeds `LOCATE_TOL`.
    #[inline]
    fn region_violation(&self, j: usize, theta: &[f64]) -> f64 {
        let w = self.dim + 1;
        let n = self.dim;
        let mut worst = f64::NEG_INFINITY;
        for k in self.offsets[j]..self.offsets[j + 1] {
            let r = &self.rows[k * w..(k + 1) * w];
            let v = dot(&r[..n], theta) - r[n];
            if v > LOCATE_TOL {
                return f64::INFINITY;
            }
            worst = worst.max(v);
        }
        worst
    }

    pub fn order(&self) -> usize {
        (1 << self.depth_per_node) - 1
    }
}

impl PointLocator for MultiwayTree {
    fn name(&self) -> &'static str {
        "multiway tree"
    }

    fn locate(&self, theta: &[f64]) -> Option<usize> {
        let n = self.dim;
        let w = n + 1;
        let mut node = 0usize;
        let mut above = [false; 64];
        loop {
            let nd = &self.nodes[node];
            // evaluate all planes of the node at once
            let k = nd.planes.len() / w;
            for p in 0..k {
                let pl = &nd.planes[p * w..(p + 1) * w];
                above[p] = dot(&pl[..n], theta) > pl[n];
            }
            let mut s = 0usize;
            loop {
                match nd.slots[s] {
                    Slot::Plane(p) => {
                        s = 2 * s + 1 + above[p as usize] as usize;
                    }
                    Slot::Child(c) => {
                        node = c as usize;
                        break;
                    }
                    Slot::Leaf(l) => {
                        return super::pick_strict_first_rescan(self.leaves[l as usize].iter().cloned(), self.offsets.len() - 1, |j| self.region_violation(j, theta));
                    }
                }
            }
        }
    }

    fn storage_floats(&self) -> usize {
        let t: usize = self.nodes.iter().map(|nd| nd.planes.len() + nd.slots.len()).sum();
        t + self.leaves.iter().map(|l| l.len()).sum::<usize>() + self.rows.len()
    }
}
