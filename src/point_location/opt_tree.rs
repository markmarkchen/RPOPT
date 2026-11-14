//! Optimized decision trees for point location.
//!
//! Fuchs, Jones & Morari (2010), "Optimized decision trees for point location in polytopic data
//! sets", ACC (DOI 10.1109/ACC.2010.5530979): every node holds a hyperplane `alpha'x <= beta` that is
//! not restricted to region facets. The node's regions are split into P_l (not entirely above) and
//! P_r (not entirely below); a region cut by the hyperplane is kept on both sides (Algorithm 2,
//! eq. (5), (17)). The optimal cutting problem (eq. (10)) minimizes
//! `J = |#L - #R| + omega * #cut`, with L (R) the regions entirely on the left (right).
//!
//! The paper solves (OCM) as a MILP per node, which is intractable beyond a few hundred regions
//! (their own 487-region example is solved only to early termination). RPOPT has no MIP solver, so
//! the same objective is minimized by search: for a fixed direction `alpha` the best offset `beta`
//! is found exactly by sweeping the sorted projections of the region vertices (the one-dimensional
//! problem behind the LM-OCM / extreme-partition arguments), and the directions come from
//!   * the facet normals of the node's regions (the facet tree of Tondel et al. is the special case
//!     `Candidates::Facets`, and `omega = 1` equals its max(|I-|,|I+|) criterion),
//!   * the coordinate axes (`Candidates::Axes`: a k-d tree with cost-optimal cut positions),
//!   * the principal directions of the region centers,
//!   * random directions, followed by randomized local search around the best one.
//!
//! The choice of search directions follows the observation of Fuchs, Axehill & Morari (2010 CDC,
//! DOI 10.1109/cdc.2010.5718203, Sec. V-B) that good directions align with frequent facet normals.
//! Facet normals are sampled with their natural multiplicity, which weights frequent orientations
//! like their clustering step does, without the mean-shift iteration.
//!
//! Exactness: leaves hold the candidate regions and `locate` verifies the answer with the same f64
//! halfspace test as the other methods, so a wrong tree can only cost speed, never correctness.
//! With `weights` (per-region query probabilities) the cost uses probability mass instead of
//! region counts, which minimizes expected depth for that query distribution.

use super::geometry::PartitionGeometry;
use super::{PointLocator, LOCATE_TOL};
use crate::linalg::{dot, Mat};
use crate::utils::chebyshev_ball;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Candidates {
    /// facet hyperplanes only (the BST of Tondel et al.)
    Facets,
    /// coordinate axes with optimal cut positions (k-d tree)
    Axes,
    /// facets, axes, principal directions, random directions and local search
    Optimized,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Criterion {
    /// J = |L - R| + omega * cut (Fuchs et al., eq. (10))
    Paper,
    /// expected-depth proxy: probability mass of each child times ln of its region count, a cut
    /// region counting (with factor omega) on both sides and half its mass on each
    ExpDepth,
}

#[derive(Clone, Debug)]
pub struct OptTreeOptions {
    pub criterion: Criterion,
    /// stop splitting at this many candidate regions
    pub leaf_size: usize,
    /// weight of cut regions in J (1 equals the facet-tree criterion)
    pub omega: f64,
    pub candidates: Candidates,
    /// random directions per node (Optimized)
    pub num_random: usize,
    /// facet normals sampled per node when the node has more than `exhaustive` regions
    pub num_facets: usize,
    /// nodes with at most this many regions try every facet
    pub exhaustive: usize,
    /// local search rounds around the best direction (Optimized)
    pub refine_rounds: usize,
    /// LP check of cut regions against the node polytope (Tondel refinement)
    pub exact_refinement: bool,
    pub max_depth: usize,
    /// stop splitting when the tree (nodes, planes, leaf lists) exceeds this many bytes
    pub max_bytes: usize,
    /// stop splitting after this many seconds of construction
    pub max_seconds: f64,
    pub seed: u64,
    /// per-region query probability (expected-depth criterion); None means all regions equal
    pub weights: Option<Vec<f64>>,
}

impl Default for OptTreeOptions {
    fn default() -> Self {
        OptTreeOptions {
            criterion: Criterion::Paper,
            leaf_size: 1,
            omega: 1.0,
            candidates: Candidates::Optimized,
            num_random: 48,
            num_facets: 48,
            exhaustive: 12,
            refine_rounds: 3,
            exact_refinement: true,
            max_depth: 96,
            max_bytes: usize::MAX,
            max_seconds: f64::INFINITY,
            seed: 7,
            weights: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Node {
    /// internal: plane index; leaf: offset into `leaves`
    a: u32,
    /// internal: index of the above child (the below child is the next node); 0 marks a leaf
    above: u32,
}

pub struct OptimizedTree {
    pub dim: usize,
    nodes: Vec<Node>,
    planes: Vec<f64>, // (dim + 1) floats per internal node
    leaves: Vec<u32>, // [count, ids...]
    rows: Vec<f64>,   // [a | b] per region row
    offsets: Vec<u32>,
    pub depth: usize,
    pub num_leaves: usize,
    /// the storage or time budget stopped the construction before all leaves were small
    pub truncated: bool,
    pub candidates: Candidates,
    name: &'static str,
}

// ---------------------------------------------------------------------------------------------
// construction
// ---------------------------------------------------------------------------------------------

const TOL: f64 = 1e-9;

struct Builder<'a> {
    geom: &'a PartitionGeometry,
    opts: &'a OptTreeOptions,
    w: Vec<f64>,
    rng: StdRng,
    nodes: Vec<Node>,
    planes: Vec<f64>,
    leaves: Vec<u32>,
    depth: usize,
    num_leaves: usize,
    above_fix: Vec<(usize, usize)>,
}

fn axis_dirs(n: usize) -> Vec<(Vec<f64>, Option<f64>)> {
    (0..n)
        .map(|k| {
            let mut a = vec![0.0; n];
            a[k] = 1.0;
            (a, None)
        })
        .collect()
}

fn normalize(v: &mut [f64]) -> bool {
    let n = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if n < 1e-12 {
        return false;
    }
    for x in v.iter_mut() {
        *x /= n;
    }
    true
}

fn gaussian(rng: &mut StdRng) -> f64 {
    let u: f64 = rng.gen_range(1e-12..1.0);
    let v: f64 = rng.gen_range(0.0..1.0);
    (-2.0 * u.ln()).sqrt() * (2.0 * std::f64::consts::PI * v).cos()
}

impl<'a> Builder<'a> {
    /// (lo, hi) of alpha'x over each region of `set` (from the region vertices).
    fn intervals(&self, set: &[usize], alpha: &[f64]) -> Vec<(f64, f64)> {
        set.iter()
            .map(|&i| {
                let vs = &self.geom.vertices[i];
                if vs.is_empty() {
                    return (f64::NEG_INFINITY, f64::INFINITY);
                }
                let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                for v in vs {
                    let s = dot(alpha, v);
                    lo = lo.min(s);
                    hi = hi.max(s);
                }
                (lo, hi)
            })
            .collect()
    }

    #[inline]
    fn cost(&self, wb: f64, wa: f64, tot: f64, nb: usize, na: usize, m: usize) -> f64 {
        let c = (tot - wb - wa).max(0.0);
        match self.opts.criterion {
            Criterion::Paper => (wb - wa).abs() + self.opts.omega * c,
            Criterion::ExpDepth => {
                let cc = (m - nb - na) as f64 * self.opts.omega;
                let (sb, sa) = (nb as f64 + cc, na as f64 + cc);
                (wb + 0.5 * c) * sb.max(1.0).ln() + (wa + 0.5 * c) * sa.max(1.0).ln()
            }
        }
    }

    /// Best offset for the direction: exact minimum of J over all breakpoints. Returns (J, beta).
    fn best_offset(&self, set: &[usize], alpha: &[f64], extra: Option<f64>) -> Option<(f64, f64)> {
        let iv = self.intervals(set, alpha);
        let m = set.len();
        let mut hs: Vec<(f64, f64)> = iv.iter().zip(set).map(|(x, &i)| (x.1, self.w[i])).collect();
        let mut ls: Vec<(f64, f64)> = iv.iter().zip(set).map(|(x, &i)| (x.0, self.w[i])).collect();
        hs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        ls.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut hw = vec![0.0; m + 1];
        let mut lw = vec![0.0; m + 1];
        for k in 0..m {
            hw[k + 1] = hw[k] + hs[k].1;
            lw[k + 1] = lw[k] + ls[k].1;
        }
        let tot = hw[m];
        let mut cand: Vec<f64> = vec![];
        let mut pts: Vec<f64> = hs.iter().chain(ls.iter()).map(|x| x.0).filter(|v| v.is_finite()).collect();
        pts.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for k in 1..pts.len() {
            let (u, v) = (pts[k - 1], pts[k]);
            if v - u > 1e-7 * (1.0 + u.abs()) {
                cand.push(0.5 * (u + v));
            }
        }
        if let Some(b) = extra {
            cand.push(b);
        }
        let mut best: Option<(f64, f64)> = None;
        for &beta in &cand {
            let nb = hs.partition_point(|x| x.0 <= beta + TOL);
            let nl = ls.partition_point(|x| x.0 < beta - TOL);
            let na = m - nl;
            if nb == 0 || na == 0 {
                continue;
            }
            let wb = hw[nb];
            let wa = tot - lw[nl];
            let cost = self.cost(wb, wa, tot, nb, na, m);
            if best.map(|b| cost < b.0 - 1e-12).unwrap_or(true) {
                best = Some((cost, beta));
            }
        }
        best
    }

    fn eval_many(&self, set: &[usize], dirs: &[(Vec<f64>, Option<f64>)]) -> Option<(f64, Vec<f64>)> {
        let work = set.len() * dirs.len();
        let f = |(a, e): &(Vec<f64>, Option<f64>)| self.best_offset(set, a, *e).map(|(c, b)| (c, a.clone(), b));
        let res: Vec<Option<(f64, Vec<f64>, f64)>> = if work > 4000 { dirs.par_iter().map(f).collect() } else { dirs.iter().map(f).collect() };
        let mut best: Option<(f64, Vec<f64>, f64)> = None;
        for r in res.into_iter().flatten() {
            if best.as_ref().map(|b| r.0 < b.0 - 1e-12).unwrap_or(true) {
                best = Some(r);
            }
        }
        best.map(|(c, mut a, b)| {
            a.push(b);
            (c, a)
        })
    }

    /// Facet hyperplanes of the node's regions (all of them, or a sample with natural multiplicity).
    fn facet_dirs(&mut self, set: &[usize], limit: usize) -> Vec<(Vec<f64>, Option<f64>)> {
        let n = self.geom.dim;
        let mut ids: Vec<usize> = set.iter().flat_map(|&i| self.geom.facets[i].iter().map(|f| f.plane)).collect();
        ids.sort_unstable();
        ids.dedup();
        if ids.len() > limit {
            let mut pick: Vec<usize> = vec![];
            let mut tries = 0;
            while pick.len() < limit && tries < 20 * limit {
                tries += 1;
                let i = set[self.rng.gen_range(0..set.len())];
                let fs = &self.geom.facets[i];
                if fs.is_empty() {
                    continue;
                }
                let p = fs[self.rng.gen_range(0..fs.len())].plane;
                if !pick.contains(&p) {
                    pick.push(p);
                }
            }
            ids = pick;
        }
        ids.into_iter().map(|p| (self.geom.planes[p][..n].to_vec(), Some(self.geom.planes[p][n]))).collect()
    }

    /// Chooses the plane [alpha | beta] for a node holding `set`.
    fn choose(&mut self, set: &[usize]) -> Option<Vec<f64>> {
        let n = self.geom.dim;
        let mut dirs: Vec<(Vec<f64>, Option<f64>)> = vec![];
        let exh = set.len() <= self.opts.exhaustive;
        match self.opts.candidates {
            Candidates::Facets => {
                let d2 = self.facet_dirs(set, if exh { usize::MAX } else { self.opts.num_facets.max(256) });
                return self.eval_facets_only(set, &d2);
            }
            Candidates::Axes => dirs.extend(axis_dirs(n)),
            Candidates::Optimized => {
                dirs.extend(axis_dirs(n));
                let lim = if exh { usize::MAX } else { self.opts.num_facets };
                let fd = self.facet_dirs(set, lim);
                dirs.extend(fd);
                // principal directions of the region centers
                if set.len() >= 3 {
                    let mut mean = vec![0.0; n];
                    let mut tw = 0.0;
                    for &i in set {
                        for k in 0..n {
                            mean[k] += self.w[i] * self.geom.centers[i][k];
                        }
                        tw += self.w[i];
                    }
                    for v in mean.iter_mut() {
                        *v /= tw;
                    }
                    let mut cov = Mat::zeros(n, n);
                    for &i in set {
                        let d: Vec<f64> = (0..n).map(|k| self.geom.centers[i][k] - mean[k]).collect();
                        for a in 0..n {
                            for b in 0..n {
                                cov[(a, b)] += self.w[i] * d[a] * d[b];
                            }
                        }
                    }
                    let e = nalgebra::SymmetricEigen::new(cov.to_nalgebra());
                    let mut order: Vec<usize> = (0..n).collect();
                    order.sort_by(|&a, &b| e.eigenvalues[b].partial_cmp(&e.eigenvalues[a]).unwrap());
                    for &c in order.iter().take(3) {
                        let mut v: Vec<f64> = (0..n).map(|k| e.eigenvectors[(k, c)]).collect();
                        if normalize(&mut v) {
                            dirs.push((v, None));
                        }
                    }
                }
                for _ in 0..self.opts.num_random {
                    let mut v: Vec<f64> = (0..n).map(|_| gaussian(&mut self.rng)).collect();
                    if normalize(&mut v) {
                        dirs.push((v, None));
                    }
                }
            }
        }
        let mut best = self.eval_many(set, &dirs);
        if self.opts.candidates == Candidates::Optimized {
            if let Some((mut cost, mut pl)) = best.clone() {
                for round in 0..self.opts.refine_rounds {
                    let sigma = 0.3 / 3f64.powi(round as i32);
                    let base: Vec<f64> = pl[..n].to_vec();
                    let tries: Vec<(Vec<f64>, Option<f64>)> = (0..16)
                        .filter_map(|_| {
                            let mut v: Vec<f64> = (0..n).map(|k| base[k] + sigma * gaussian(&mut self.rng)).collect();
                            normalize(&mut v).then_some((v, None))
                        })
                        .collect();
                    if let Some((c, p)) = self.eval_many(set, &tries) {
                        if c < cost - 1e-12 {
                            cost = c;
                            pl = p;
                        }
                    }
                }
                best = Some((cost, pl));
            }
        }
        if best.is_none() && self.opts.candidates != Candidates::Facets {
            // axes found nothing valid: fall back to every facet
            let d2 = self.facet_dirs(set, usize::MAX);
            best = self.eval_many(set, &d2);
        }
        best.map(|b| b.1)
    }

    /// Facet tree: candidates are the facet hyperplanes at their own offsets.
    fn eval_facets_only(&self, set: &[usize], dirs: &[(Vec<f64>, Option<f64>)]) -> Option<Vec<f64>> {
        let n = self.geom.dim;
        let m = set.len();
        let totw: f64 = set.iter().map(|&i| self.w[i]).sum();
        let score = |(a, b): &(Vec<f64>, Option<f64>)| -> Option<(f64, Vec<f64>)> {
            let beta = b.unwrap();
            let iv = self.intervals(set, a);
            let (mut nb, mut na, mut wb, mut wa) = (0usize, 0usize, 0.0, 0.0);
            for (k, &i) in set.iter().enumerate() {
                if iv[k].1 <= beta + TOL {
                    nb += 1;
                    wb += self.w[i];
                } else if iv[k].0 >= beta - TOL {
                    na += 1;
                    wa += self.w[i];
                }
            }
            let _ = m;
            if nb == 0 || na == 0 {
                return None;
            }
            let cost = self.cost(wb, wa, totw, nb, na, m);
            let mut pl = a.clone();
            pl.push(beta);
            Some((cost, pl))
        };
        let _ = n;
        let res: Vec<Option<(f64, Vec<f64>)>> = if set.len() * dirs.len() > 4000 { dirs.par_iter().map(score).collect() } else { dirs.iter().map(score).collect() };
        let mut best: Option<(f64, Vec<f64>)> = None;
        for r in res.into_iter().flatten() {
            if best.as_ref().map(|b| r.0 < b.0 - 1e-12).unwrap_or(true) {
                best = Some(r);
            }
        }
        best.map(|b| b.1)
    }

    fn make_leaf(&mut self, idx: usize, set: &[usize]) {
        self.nodes[idx] = Node { a: self.leaves.len() as u32, above: 0 };
        self.leaves.push(set.len() as u32);
        self.leaves.extend(set.iter().map(|&i| i as u32));
        self.num_leaves += 1;
    }

    /// Splits `set` at plane `pl` (region classification from the vertices, LP check for cut regions).
    fn partition(&self, set: &[usize], path: &[Vec<f64>], pl: &[f64]) -> (Vec<usize>, Vec<usize>) {
        let n = self.geom.dim;
        let iv = self.intervals(set, &pl[..n]);
        let beta = pl[n];
        // classify from the vertices; cut regions get the LP check (in parallel)
        let cls: Vec<(bool, bool)> = {
            let f = |k: usize| -> (bool, bool) {
                let (lo, hi) = iv[k];
                if hi <= beta + TOL {
                    (true, false)
                } else if lo >= beta - TOL {
                    (false, true)
                } else if self.opts.exact_refinement {
                    split_check(self.geom, set[k], path, pl)
                } else {
                    (true, true)
                }
            };
            let cut = iv.iter().filter(|&&(lo, hi)| !(hi <= beta + TOL || lo >= beta - TOL)).count();
            if cut >= 4 {
                (0..set.len()).into_par_iter().map(f).collect()
            } else {
                (0..set.len()).map(f).collect()
            }
        };
        let (mut below, mut above) = (vec![], vec![]);
        for (k, &i) in set.iter().enumerate() {
            if cls[k].0 {
                below.push(i);
            }
            if cls[k].1 {
                above.push(i);
            }
        }
        (below, above)
    }

    /// Best-first construction: the leaf with the most candidate regions is split first, until
    /// every leaf is small enough or the storage / time budget is used up (a truncated tree keeps
    /// the remaining big leaves, which `locate` scans: still exact). Returns the final layout in
    /// preorder (below child directly after its parent).
    fn build_all(&mut self) -> bool {
        let n = self.geom.dim;
        let start = std::time::Instant::now();
        struct T {
            plane: Option<Vec<f64>>,
            kids: (usize, usize),
            set: Vec<usize>,
            path: Option<Rc<PNode>>,
            depth: usize,
        }
        let mut t = vec![T { plane: None, kids: (0, 0), set: (0..self.geom.num_regions).collect(), path: None, depth: 0 }];
        let mut heap: BinaryHeap<(usize, Reverse<usize>)> = BinaryHeap::new();
        heap.push((t[0].set.len(), Reverse(0)));
        let mut bytes = 8 + 4 * (t[0].set.len() + 1);
        let mut truncated = false;
        while let Some((sz, Reverse(idx))) = heap.pop() {
            if sz <= self.opts.leaf_size || t[idx].depth >= self.opts.max_depth {
                continue;
            }
            if bytes > self.opts.max_bytes || start.elapsed().as_secs_f64() > self.opts.max_seconds {
                truncated = true;
                break;
            }
            let set = std::mem::take(&mut t[idx].set);
            let Some(pl) = self.choose(&set) else {
                t[idx].set = set;
                continue;
            };
            let mut path_vec = vec![];
            let mut cur = t[idx].path.clone();
            while let Some(pn) = cur {
                path_vec.push(pn.plane.clone());
                cur = pn.parent.clone();
            }
            let (below, above) = self.partition(&set, &path_vec, &pl);
            if below.len() == set.len() || above.len() == set.len() || below.is_empty() || above.is_empty() {
                t[idx].set = set;
                continue;
            }
            let depth = t[idx].depth + 1;
            let mut neg: Vec<f64> = pl.iter().map(|v| -v).collect();
            neg[n] = -pl[n];
            let pb = Some(Rc::new(PNode { plane: pl.clone(), parent: t[idx].path.clone() }));
            let pa = Some(Rc::new(PNode { plane: neg, parent: t[idx].path.clone() }));
            bytes = bytes - 4 * (set.len() + 1) + 4 * (below.len() + above.len() + 2) + 8 * 2 + 8 * (n + 1);
            let (bi, ai) = (t.len(), t.len() + 1);
            heap.push((below.len(), Reverse(bi)));
            heap.push((above.len(), Reverse(ai)));
            t.push(T { plane: None, kids: (0, 0), set: below, path: pb, depth });
            t.push(T { plane: None, kids: (0, 0), set: above, path: pa, depth });
            t[idx].plane = Some(pl);
            t[idx].kids = (bi, ai);
            // the parent's path is no longer needed
            t[idx].path = None;
        }
        // serialize in preorder
        let mut stack = vec![(0usize, 0usize, false)];
        let mut pending_above: Vec<(usize, usize)> = vec![];
        let _ = &mut pending_above;
        // iterative preorder: (tree index, depth, _)
        while let Some((ti, d, _)) = stack.pop() {
            let idx = self.nodes.len();
            self.nodes.push(Node { a: 0, above: 0 });
            self.depth = self.depth.max(d);
            match t[ti].plane.take() {
                Some(pl) => {
                    let pidx = self.planes.len() / (n + 1);
                    self.planes.extend_from_slice(&pl);
                    self.nodes[idx].a = pidx as u32;
                    // the above child is placed after the whole below subtree: patch when visited
                    stack.push((t[ti].kids.1, d + 1, true));
                    self.above_fix.push((idx, t[ti].kids.1));
                    stack.push((t[ti].kids.0, d + 1, false));
                }
                None => {
                    let set = std::mem::take(&mut t[ti].set);
                    self.make_leaf(idx, &set);
                }
            }
            // a node reached as an above child fixes its parent's pointer
            if let Some(pos) = self.above_fix.iter().rposition(|&(_, k)| k == ti) {
                let (par, _) = self.above_fix.swap_remove(pos);
                self.nodes[par].above = idx as u32;
            }
        }
        truncated
    }
}

struct PNode {
    plane: Vec<f64>,
    parent: Option<Rc<PNode>>,
}

/// Does region i meet the closed below / above side of `pl` inside the path polytope with volume?
fn split_check(geom: &PartitionGeometry, i: usize, path: &[Vec<f64>], pl: &[f64]) -> (bool, bool) {
    let n = geom.dim;
    let (a, b) = geom.region_mat(i);
    let mut rows = a;
    let mut rhs = b;
    for h in path {
        rows.push_row(&h[..n]);
        rhs.push(h[n]);
    }
    let check = |sign: f64| -> bool {
        let mut r2 = rows.clone();
        let mut b2 = rhs.clone();
        let row: Vec<f64> = pl[..n].iter().map(|v| sign * v).collect();
        r2.push_row(&row);
        b2.push(sign * pl[n]);
        matches!(chebyshev_ball(&r2, &b2, &[]), Some(s) if s.x[n] > 1e-11)
    };
    (check(1.0), check(-1.0))
}

impl OptimizedTree {
    pub fn new(geom: &PartitionGeometry) -> Self {
        Self::with_options(geom, &OptTreeOptions::default())
    }

    pub fn with_options(geom: &PartitionGeometry, opts: &OptTreeOptions) -> Self {
        let n = geom.dim;
        let r = geom.num_regions;
        let w: Vec<f64> = match &opts.weights {
            Some(w) => {
                let tot: f64 = w.iter().sum::<f64>().max(1e-300);
                w.iter().map(|x| x / tot * r as f64).collect()
            }
            None => vec![1.0; r],
        };
        let mut b = Builder {
            geom,
            opts,
            w,
            rng: StdRng::seed_from_u64(opts.seed),
            nodes: vec![],
            planes: vec![],
            leaves: vec![],
            depth: 0,
            num_leaves: 0,
            above_fix: vec![],
        };
        let truncated = b.build_all();
        let mut rows = vec![];
        for i in 0..r {
            let (e, f) = geom.region_rows(i);
            for k in 0..f.len() {
                rows.extend_from_slice(&e[k * n..(k + 1) * n]);
                rows.push(f[k]);
            }
        }
        let name = match opts.candidates {
            Candidates::Facets => "facet tree",
            Candidates::Axes => "k-d tree",
            Candidates::Optimized => {
                if opts.weights.is_some() {
                    "optimized tree (weighted)"
                } else {
                    "optimized tree"
                }
            }
        };
        OptimizedTree {
            dim: n,
            nodes: b.nodes,
            planes: b.planes,
            leaves: b.leaves,
            rows,
            offsets: geom.offsets.iter().map(|&o| o as u32).collect(),
            depth: b.depth,
            num_leaves: b.num_leaves,
            truncated,
            candidates: opts.candidates,
            name,
        }
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Storage in bytes: nodes, planes, leaf lists and the shared region row table.
    pub fn storage_bytes(&self) -> usize {
        8 * self.nodes.len() + 8 * self.planes.len() + 4 * self.leaves.len() + 8 * self.rows.len() + 4 * self.offsets.len()
    }

    /// Mean number of candidate regions per leaf entry (duplication factor is `num_leaves / R`).
    pub fn leaf_candidates(&self) -> usize {
        self.leaves.len() - self.num_leaves
    }

    /// Leaf offset reached by theta and the number of decisions taken.
    #[inline]
    fn descend(&self, theta: &[f64]) -> (usize, usize) {
        let n = self.dim;
        let w = n + 1;
        let mut i = 0usize;
        let mut d = 0usize;
        loop {
            let nd = self.nodes[i];
            if nd.above == 0 {
                return (nd.a as usize, d);
            }
            let p = &self.planes[nd.a as usize * w..nd.a as usize * w + w];
            i = if dot(&p[..n], theta) <= p[n] { i + 1 } else { nd.above as usize };
            d += 1;
        }
    }

    #[inline]
    fn region_contains(&self, j: usize, theta: &[f64]) -> bool {
        let n = self.dim;
        let w = n + 1;
        for k in self.offsets[j] as usize..self.offsets[j + 1] as usize {
            let r = &self.rows[k * w..(k + 1) * w];
            if dot(&r[..n], theta) > r[n] + LOCATE_TOL {
                return false;
            }
        }
        true
    }

    /// (region, decisions taken, candidate regions tested)
    pub fn locate_counted(&self, theta: &[f64]) -> (Option<usize>, usize, usize) {
        let (off, d) = self.descend(theta);
        let cnt = self.leaves[off] as usize;
        for (t, &j) in self.leaves[off + 1..off + 1 + cnt].iter().enumerate() {
            if self.region_contains(j as usize, theta) {
                return (Some(j as usize), d, t + 1);
            }
        }
        (None, d, cnt)
    }
}

impl PointLocator for OptimizedTree {
    fn name(&self) -> &'static str {
        self.name
    }

    #[inline]
    fn locate(&self, theta: &[f64]) -> Option<usize> {
        let (off, _) = self.descend(theta);
        let cnt = self.leaves[off] as usize;
        self.leaves[off + 1..off + 1 + cnt].iter().map(|&j| j as usize).find(|&j| self.region_contains(j, theta))
    }

    fn storage_floats(&self) -> usize {
        self.storage_bytes() / 8
    }
}

/// Vertices of {x : A x <= b} by enumerating n-subsets of rows, with a feasibility tolerance
/// relative to the row scale. `geometry::enumerate_vertices` uses an absolute 1e-9 test that drops
/// vertices of ill-conditioned regions (up to about 1% of the regions at theta dimension 6), and
/// a tree that classifies regions from an incomplete vertex set misses regions.
pub fn robust_vertices(a: &Mat, b: &[f64], tol: f64) -> Vec<Vec<f64>> {
    let (n, m) = (a.cols, a.rows);
    let mut out: Vec<Vec<f64>> = vec![];
    if m < n || n == 0 {
        return out;
    }
    let mut idx: Vec<usize> = (0..n).collect();
    let scale = b.iter().fold(1.0f64, |s, v| s.max(v.abs()));
    loop {
        let sub = a.select_rows(&idx);
        if let Some(lu) = crate::linalg::Lu::new(&sub) {
            let rhs: Vec<f64> = idx.iter().map(|&i| b[i]).collect();
            let x = lu.solve(&rhs);
            if x.iter().all(|v| v.is_finite()) && (0..m).all(|i| dot(a.row(i), &x) - b[i] <= tol * scale) && !out.iter().any(|p| p.iter().zip(&x).all(|(u, v)| (u - v).abs() <= 1e-9 * (1.0 + u.abs()))) {
                out.push(x);
            }
        }
        let mut k = n;
        loop {
            if k == 0 {
                return out;
            }
            k -= 1;
            if idx[k] < m - n + k {
                idx[k] += 1;
                for t in k + 1..n {
                    idx[t] = idx[t - 1] + 1;
                }
                break;
            }
        }
    }
}
