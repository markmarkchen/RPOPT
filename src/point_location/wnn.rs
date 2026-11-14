//! Weighted nearest-neighbour point location (Jones, Grieder & Raković, "A logarithmic-time
//! solution to the point location problem for parametric linear programming", Automatica 42(12),
//! 2006, pp. 2215-2218).
//!
//! # The idea
//!
//! If the partition is a power diagram (a regular subdivision), every region i has a site c_i and
//! a weight w_i with
//!
//! ```text
//! R_i = { θ : |θ - c_i|² - w_i <= |θ - c_j|² - w_j  for all j }.
//! ```
//!
//! Expanding the squares, |θ - c_i|² - w_i = |θ|² - g_i(θ) with the affine function
//! g_i(θ) = 2 c_i'θ + (w_i - |c_i|²). Point location is therefore an additively weighted nearest
//! neighbour query, or equivalently argmax_i g_i(θ) (the "descriptor" of Borrelli et al. 2001 for
//! the value function). The lifting map of Arya et al. turns it into an ordinary nearest-neighbour
//! query: with K = max_j w_j and z_i = sqrt(K - w_i) one has |θ - c_i|² - w_i = |(θ, 0) - (c_i, z_i)|² - K,
//! so the nearest lifted point (c_i, z_i) to (θ, 0) in R^(n+1) names the region. A k-d tree over
//! the lifted points answers it in logarithmic expected time.
//!
//! # Which partitions are power diagrams
//!
//! * mpLP (the paper's Theorem 2): the value function V(θ) = min c'y is convex piecewise affine,
//!   V = max_r (F_r'θ + f_r), and the critical region of piece r is exactly where piece r is the
//!   maximum. With s_r = F_r/2 and w_r = -f_r - |s_r|² the cells coincide with the power diagram of
//!   (s_r, w_r), intersected with the feasible set. The proof assumes nondegeneracy: two adjacent
//!   regions carrying the same affine piece (a degenerate LP whose optimiser is not unique) give
//!   the same site and weight and cannot be separated.
//! * mpQP (the paper's Remark 1): "may or may not" be a power diagram, examples are known where it
//!   is not. The value function of an mpQP is quadratic with a different Hessian in every region,
//!   so the theorem does not carry over. Whether a given partition is a power diagram is decided
//!   by an LP on the facet adjacencies (below). The same LP is the one of the linear machine
//!   (Airan et al. 2017): a linear machine with discriminants g_i IS a power diagram, with
//!   c_i = (slope of g_i)/2 and w_i = g_i(0) + |c_i|².
//!
//! Empirically (bench/results/pl_wnn.md): the mpLP partitions of bench/problems_lp are certified
//! power diagrams through the value function (10 of 10; some have duplicate pieces, see below);
//! of the 12 mpQP problems of bench/problems only the 8 double-integrator ones are (their
//! partitions are small, 7 to 11 regions in 2 parameters); the mass-chain and 3x2 system MPCs and
//! every random mpQP of the dimension sweep (2 to 8 parameters) have facets with two or more
//! neighbours, so they are not facet-to-facet and cannot be power diagrams. The adjacency LP is no
//! help for the mpLP partitions: their adjacency graph from `PartitionGeometry` is disconnected
//! (2 to 8 components in the 2-parameter examples), so the local conditions cannot fix the
//! relative gauge of the components and the LP solution fails the certificate. The value-function
//! route does not need adjacencies.
//!
//! # Computing (c_i, w_i)
//!
//! 1. Theory (mpLP with an objective that is affine in θ and linear in x): the pieces V_r come from
//!    evaluating the program objective along the region's law x_r(θ).
//! 2. LP over the facet adjacencies, for every pair (i, j) of regions that share a facet
//!    a'θ <= b (on the side of i): g_i - g_j = α_ij (b - a'θ) with α_ij >= 1. For a facet-to-facet
//!    partition these local conditions make the argmax globally correct (the lifted partition is
//!    locally convex). The LP is sparse and solved with Clarabel; if it is infeasible, thin sliver
//!    regions are left out of the LP (they are found through the neighbour fallback of `locate`).
//!
//! Both candidates are certified before use: g_i must dominate every other g_j at every vertex of
//! R_i (g_i - g_j is affine, so this proves it on all of R_i). The check is done with the k-d
//! tree itself. If neither candidate is certified the partition is reported as not a power
//! diagram ([`WnnStatus`]) and every query goes to the exact fallback.
//!
//! # Exactness
//!
//! The nearest-neighbour search only proposes a candidate. It is verified with the halfspace test
//! of the other locators (`LOCATE_TOL`, rule of `pick_strict_first`): a candidate that contains θ
//! strictly is returned; otherwise its facet neighbours are tried; a θ outside the feasible set by
//! more than `LOCATE_TOL` answers `None`; otherwise the exact fallback (the hash table, or a full
//! strict-first scan) decides. An answer is therefore never a guess; numerical error in the
//! weights, a near-tie at a facet, a sliver region or a non-power-diagram partition only costs a
//! fallback.

use super::geometry::PartitionGeometry;
use super::hash_table::HashTable;
use super::walk::RowGraph;
use super::{pick_strict_first, PointLocator};
use crate::linalg::dot;
use crate::solution::Solution;
use crate::solver::sparse_lp::SparseLp;
use std::sync::Arc;
use std::time::Instant;

const LEAF: usize = 8;

/// Queries farther outside a domain plane than this many `LOCATE_TOL` answer `None` without the fallback.
const OUTSIDE_MARGIN: f64 = 1e3;

/// How the site and weight data were obtained, or why there are none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WnnStatus {
    /// from the value function (mpLP theory), certified
    Theory,
    /// from the facet-adjacency LP, certified
    Lp,
    /// some facet has two or more neighbours among the regions kept in the LP, so the partition
    /// is not facet-to-facet and cannot be a power diagram (the LP is not solved)
    NotFacetToFacet,
    /// the LP is infeasible: no power diagram with these facets (typically a partition that is not
    /// facet-to-facet, or one whose regions are not the cells of a regular subdivision)
    LpInfeasible,
    /// a candidate (LP solution or value function) failed the vertex certificate
    NotCertified,
}

#[derive(Clone, Debug)]
pub struct WnnReport {
    pub status: WnnStatus,
    /// facets with two or more neighbours (a power diagram is facet-to-facet)
    pub non_facet_to_facet: usize,
    /// sliver regions left out of the tree (reached through the neighbour or exact fallback)
    pub excluded: usize,
    /// max over regions i, vertices v of R_i, regions j of g_j(v) - g_i(v)
    pub max_vertex_violation: f64,
    /// regions (in the tree) whose Chebyshev center is classified correctly by the argmax
    pub centers_correct: usize,
    pub num_regions: usize,
    /// regions whose affine function is identical to another region's (degenerate mpLP)
    pub duplicate_sites: usize,
    pub lp_seconds: f64,
    pub tree_seconds: f64,
}

/// What answered a query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WnnOutcome {
    /// the nearest neighbour contained θ
    Tree,
    /// a facet neighbour of the nearest neighbour contained θ
    Neighbor,
    /// θ outside the feasible set (by more than `LOCATE_TOL`), answered `None` without fallback
    Outside,
    /// the exact fallback answered
    Fallback,
}

struct Node {
    left: u32,
    right: u32,
    start: u32,
    end: u32,
}

/// k-d tree over the lifted points (c_i, z_i). Stores the affine form g_i = W_i'θ + W_i0 of every
/// point in tree order, so a leaf scan is one contiguous pass.
struct KdTree {
    n: usize,
    nodes: Vec<Node>,
    /// per node: lo (n+1), hi (n+1) of the lifted bounding box
    boxes: Vec<f64>,
    /// [W | W0] per point, tree order
    pts: Vec<f64>,
    ids: Vec<u32>,
    k_const: f64,
}

impl KdTree {
    /// `g`: R x (n+1) rows [W | W0]; regions with `used[i] == false` are left out.
    fn build(n: usize, g: &[f64], used: &[bool]) -> Option<KdTree> {
        let w1 = n + 1;
        let ids0: Vec<usize> = (0..used.len()).filter(|&i| used[i]).collect();
        if ids0.is_empty() {
            return None;
        }
        // w_i = W0_i + |c_i|², c_i = W_i/2
        let wts: Vec<f64> = ids0.iter().map(|&i| g[i * w1 + n] + (0..n).map(|k| 0.25 * g[i * w1 + k] * g[i * w1 + k]).sum::<f64>()).collect();
        let k_const = wts.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        // lifted coordinates, indexed like ids0
        let mut lift = vec![0.0; ids0.len() * w1];
        for (p, &i) in ids0.iter().enumerate() {
            for k in 0..n {
                lift[p * w1 + k] = 0.5 * g[i * w1 + k];
            }
            lift[p * w1 + n] = (k_const - wts[p]).max(0.0).sqrt();
        }
        let mut order: Vec<usize> = (0..ids0.len()).collect();
        let mut tree = KdTree { n, nodes: vec![], boxes: vec![], pts: vec![], ids: vec![], k_const };
        tree.split(&lift, &mut order, 0, ids0.len());
        for &p in &order {
            let i = ids0[p];
            tree.pts.extend_from_slice(&g[i * w1..(i + 1) * w1]);
            tree.ids.push(i as u32);
        }
        // z lower bound per node must be exact in K - w terms: recompute from the stored forms
        tree.fix_z_bounds(&wts, &order);
        Some(tree)
    }

    fn split(&mut self, lift: &[f64], order: &mut [usize], s: usize, e: usize) -> u32 {
        let w1 = self.n + 1;
        let idx = self.nodes.len();
        self.nodes.push(Node { left: 0, right: 0, start: s as u32, end: e as u32 });
        let mut lo = vec![f64::INFINITY; w1];
        let mut hi = vec![f64::NEG_INFINITY; w1];
        for &p in &order[s..e] {
            for k in 0..w1 {
                lo[k] = lo[k].min(lift[p * w1 + k]);
                hi[k] = hi[k].max(lift[p * w1 + k]);
            }
        }
        let (mut dim, mut ext) = (0, -1.0);
        for k in 0..w1 {
            if hi[k] - lo[k] > ext {
                ext = hi[k] - lo[k];
                dim = k;
            }
        }
        self.boxes.extend_from_slice(&lo);
        self.boxes.extend_from_slice(&hi);
        if e - s > LEAF && ext > 0.0 {
            let mid = (s + e) / 2;
            order[s..e].select_nth_unstable_by(mid - s, |&a, &b| lift[a * w1 + dim].total_cmp(&lift[b * w1 + dim]));
            let l = self.split(lift, order, s, mid);
            let r = self.split(lift, order, mid, e);
            self.nodes[idx].left = l;
            self.nodes[idx].right = r;
        }
        idx as u32
    }

    /// Replace the z-range of every node by exact values computed without the sqrt rounding.
    fn fix_z_bounds(&mut self, wts: &[f64], order: &[usize]) {
        let w1 = self.n + 1;
        // z² = K - w; node z_lo² = K - max w over the node's points
        for ni in 0..self.nodes.len() {
            let (s, e) = (self.nodes[ni].start as usize, self.nodes[ni].end as usize);
            let wmax = order[s..e].iter().map(|&p| wts[p]).fold(f64::NEG_INFINITY, f64::max);
            let zlo2 = (self.k_const - wmax).max(0.0);
            // store z_lo² in the lo slot of the z coordinate (the box test squares it)
            self.boxes[ni * 2 * w1 + self.n] = zlo2;
        }
    }

    /// max_i g_i(θ) and its region id (ties: first found).
    fn argmax(&self, theta: &[f64]) -> (f64, usize) {
        let q = dot(theta, theta);
        let mut best = (f64::NEG_INFINITY, usize::MAX);
        self.search(0, theta, q, &mut best);
        (best.0, best.1)
    }

    /// Upper bound of g over the node's points.
    #[inline]
    fn bound(&self, ni: usize, theta: &[f64], q: f64) -> f64 {
        let n = self.n;
        let w1 = n + 1;
        let b = &self.boxes[ni * 2 * w1..(ni + 1) * 2 * w1];
        let (lo, hi) = (&b[..w1], &b[w1..]);
        let mut d2 = lo[n]; // z_lo²
        for k in 0..n {
            let t = theta[k];
            let d = if t < lo[k] {
                lo[k] - t
            } else if t > hi[k] {
                t - hi[k]
            } else {
                0.0
            };
            d2 += d * d;
        }
        q - d2 + self.k_const
    }

    fn search(&self, ni: usize, theta: &[f64], q: f64, best: &mut (f64, usize)) {
        let node = &self.nodes[ni];
        if node.left == 0 {
            let n = self.n;
            let w1 = n + 1;
            for p in node.start as usize..node.end as usize {
                let w = &self.pts[p * w1..(p + 1) * w1];
                let v = dot(&w[..n], theta) + w[n];
                if v > best.0 {
                    *best = (v, self.ids[p] as usize);
                }
            }
            return;
        }
        let (l, r) = (node.left as usize, node.right as usize);
        let (bl, br) = (self.bound(l, theta, q), self.bound(r, theta, q));
        let slack = |b: f64| 1e-10 * (1.0 + q + b.abs());
        let (first, bf, second, bs) = if bl >= br { (l, bl, r, br) } else { (r, br, l, bl) };
        if bf >= best.0 - slack(best.0) {
            self.search(first, theta, q, best);
        }
        if bs >= best.0 - slack(best.0) {
            self.search(second, theta, q, best);
        }
    }

    /// (nodes visited, points scored) of one nearest-neighbour search.
    fn search_cost(&self, theta: &[f64]) -> (usize, usize) {
        let q = dot(theta, theta);
        let mut best = (f64::NEG_INFINITY, usize::MAX);
        let mut cost = (0, 0);
        self.search_counted(0, theta, q, &mut best, &mut cost);
        cost
    }

    fn search_counted(&self, ni: usize, theta: &[f64], q: f64, best: &mut (f64, usize), cost: &mut (usize, usize)) {
        cost.0 += 1;
        let node = &self.nodes[ni];
        if node.left == 0 {
            let n = self.n;
            let w1 = n + 1;
            for p in node.start as usize..node.end as usize {
                cost.1 += 1;
                let w = &self.pts[p * w1..(p + 1) * w1];
                let v = dot(&w[..n], theta) + w[n];
                if v > best.0 {
                    *best = (v, self.ids[p] as usize);
                }
            }
            return;
        }
        let (l, r) = (node.left as usize, node.right as usize);
        let (bl, br) = (self.bound(l, theta, q), self.bound(r, theta, q));
        let slack = |b: f64| 1e-10 * (1.0 + q + b.abs());
        let (first, bf, second, bs) = if bl >= br { (l, bl, r, br) } else { (r, br, l, bl) };
        if bf >= best.0 - slack(best.0) {
            self.search_counted(first, theta, q, best, cost);
        }
        if bs >= best.0 - slack(best.0) {
            self.search_counted(second, theta, q, best, cost);
        }
    }

    fn storage_floats(&self) -> usize {
        self.pts.len() + self.boxes.len() + (self.nodes.len() * 4 + self.ids.len()).div_ceil(2)
    }
}

pub struct Wnn {
    n: usize,
    num_regions: usize,
    tree: Option<KdTree>,
    graph: RowGraph,
    domain: Vec<Vec<f64>>,
    fallback: Option<Arc<dyn PointLocator>>,
    /// boundary tolerance, taken from the geometry (the solution's point location tolerance)
    tol: f64,
    pub report: WnnReport,
}

impl Wnn {
    /// Power-diagram locator with the 8 MB hash table as exact fallback (full scan when the hash
    /// table does not support the dimension). `sol` enables the mpLP value-function route.
    pub fn new(geom: &PartitionGeometry, sol: Option<&Solution>) -> Self {
        let fb: Option<Arc<dyn PointLocator>> = if HashTable::supports_dim(geom.dim) { Some(Arc::new(HashTable::with_storage_limit(geom, 8 << 20))) } else { None };
        Self::with_fallback(geom, sol, fb)
    }

    /// `fallback = None` uses a strict-first scan of all regions.
    pub fn with_fallback(geom: &PartitionGeometry, sol: Option<&Solution>, fallback: Option<Arc<dyn PointLocator>>) -> Self {
        let n = geom.dim;
        let r = geom.num_regions;
        let nftf = geom.non_facet_to_facet();
        let mut report = WnnReport { status: WnnStatus::LpInfeasible, non_facet_to_facet: nftf, excluded: 0, max_vertex_violation: f64::NAN, centers_correct: 0, num_regions: r, duplicate_sites: 0, lp_seconds: 0.0, tree_seconds: 0.0 };
        let mut tree = None;
        // 1. theory (mpLP)
        if let Some(sol) = sol {
            if let Some(g) = value_function_pieces(sol, r, n) {
                let t0 = Instant::now();
                let used = vec![true; r];
                if let Some(t) = KdTree::build(n, &g, &used) {
                    report.tree_seconds = t0.elapsed().as_secs_f64();
                    let (cert, rep) = certify(&t, &g, &used, geom);
                    if cert {
                        report.status = WnnStatus::Theory;
                        fill(&mut report, rep);
                        report.duplicate_sites = count_duplicates(&g, n);
                        tree = Some(t);
                    } else {
                        report.status = WnnStatus::NotCertified;
                        fill(&mut report, rep);
                    }
                }
            }
        }
        // 2. LP over the facet adjacencies
        if tree.is_none() {
            let extent = geom.extent().max(1e-12);
            let mut last = report.status;
            for rel in [0.0, 1e-6, 1e-3] {
                let excluded: Vec<bool> = (0..r).map(|i| geom.radii[i] < rel * extent).collect();
                if rel > 0.0 && excluded.iter().all(|&e| !e) {
                    continue;
                }
                // a power diagram is facet-to-facet: a facet with two neighbours settles the question
                // without the LP (up to the sliver regions excluded at this level)
                let multi = geom.facets.iter().enumerate().filter(|(i, _)| !excluded[*i]).flat_map(|(_, fs)| fs.iter()).filter(|f| f.neighbors.iter().filter(|&&j| !excluded[j]).count() >= 2).count();
                if multi > 0 {
                    last = WnnStatus::NotFacetToFacet;
                    continue;
                }
                let t0 = Instant::now();
                let sol_g = lp_weights(geom, &excluded);
                report.lp_seconds += t0.elapsed().as_secs_f64();
                let Some(g) = sol_g else {
                    last = WnnStatus::LpInfeasible;
                    continue;
                };
                let used: Vec<bool> = excluded.iter().map(|&e| !e).collect();
                let t0 = Instant::now();
                let Some(t) = KdTree::build(n, &g, &used) else {
                    last = WnnStatus::LpInfeasible;
                    continue;
                };
                report.tree_seconds = t0.elapsed().as_secs_f64();
                let (cert, rep) = certify(&t, &g, &used, geom);
                let ex = excluded.iter().filter(|&&e| e).count();
                if cert {
                    report.status = WnnStatus::Lp;
                    report.excluded = ex;
                    fill(&mut report, rep);
                    tree = Some(t);
                    break;
                }
                last = WnnStatus::NotCertified;
                report.excluded = ex;
                fill(&mut report, rep);
            }
            if tree.is_none() {
                report.status = last;
            }
        }
        Wnn { n, num_regions: r, tree, graph: RowGraph::new(geom), domain: geom.domain_planes.clone(), fallback, tol: geom.tol, report }
    }

    pub fn is_power_diagram(&self) -> bool {
        self.tree.is_some()
    }

    /// Floats of the locator proper (tree, rows and neighbour lists for the verification), without
    /// the fallback structure.
    pub fn storage_floats_excl_fallback(&self) -> usize {
        self.tree.as_ref().map_or(0, |t| t.storage_floats()) + self.graph.storage_floats()
    }

    /// (nodes visited, regions scored) by the nearest-neighbour search for θ; (0, 0) without a tree.
    pub fn search_cost(&self, theta: &[f64]) -> (usize, usize) {
        self.tree.as_ref().map_or((0, 0), |t| t.search_cost(theta))
    }

    fn exact(&self, theta: &[f64]) -> Option<usize> {
        match &self.fallback {
            Some(f) => f.locate(theta),
            None => pick_strict_first(self.tol, 0..self.num_regions, |j| self.graph.violation(j, theta)),
        }
    }

    /// The answer and what produced it.
    pub fn locate_outcome(&self, theta: &[f64]) -> (Option<usize>, WnnOutcome) {
        let Some(tree) = &self.tree else {
            return (self.exact(theta), WnnOutcome::Fallback);
        };
        let (_, i) = tree.argmax(theta);
        if self.graph.violation(i, theta) <= 0.0 {
            return (Some(i), WnnOutcome::Tree);
        }
        let n = self.n;
        // clearly outside the feasible set: beyond a domain plane by far more than the boundary
        // tolerance (the margin keeps the early exit exact when the rows of the solution and the
        // normalized rows of the geometry weigh LOCATE_TOL differently)
        if !self.domain.iter().all(|p| dot(&p[..n], theta) <= p[n] + OUTSIDE_MARGIN * self.tol) {
            return (None, WnnOutcome::Outside);
        }
        let g0 = self.graph.row0(i);
        let rows = self.graph.rows(i).1.len();
        for g in g0..g0 + rows {
            for &j in self.graph.neighbors(g) {
                if self.graph.violation(j as usize, theta) <= 0.0 {
                    return (Some(j as usize), WnnOutcome::Neighbor);
                }
            }
        }
        (self.exact(theta), WnnOutcome::Fallback)
    }
}

impl PointLocator for Wnn {
    fn name(&self) -> &'static str {
        "wnn"
    }

    fn locate(&self, theta: &[f64]) -> Option<usize> {
        self.locate_outcome(theta).0
    }

    fn storage_floats(&self) -> usize {
        self.storage_floats_excl_fallback() + self.fallback.as_ref().map_or(0, |f| f.storage_floats())
    }
}

struct CertRep {
    max_violation: f64,
    centers_correct: usize,
}

fn fill(report: &mut WnnReport, c: CertRep) {
    report.max_vertex_violation = c.max_violation;
    report.centers_correct = c.centers_correct;
}

/// Vertex certificate: for every region in the tree and every vertex v of it, g_i(v) must be the
/// maximum of all g_j(v) up to a rounding-level tolerance.
fn certify(tree: &KdTree, g: &[f64], used: &[bool], geom: &PartitionGeometry) -> (bool, CertRep) {
    let n = geom.dim;
    let w1 = n + 1;
    let mut max_v = f64::NEG_INFINITY;
    let mut ok = true;
    let mut centers = 0;
    let mut counted = 0;
    for i in 0..geom.num_regions {
        if !used[i] {
            continue;
        }
        counted += 1;
        let wi = &g[i * w1..(i + 1) * w1];
        for v in &geom.vertices[i] {
            let gi = dot(&wi[..n], v) + wi[n];
            let (best, _) = tree.argmax(v);
            let mag = 1.0 + dot(&wi[..n].iter().map(|x| x.abs()).collect::<Vec<_>>(), &v.iter().map(|x| x.abs()).collect::<Vec<_>>()) + wi[n].abs();
            let viol = best - gi;
            max_v = max_v.max(viol / mag);
            if viol > 1e-6 * mag {
                ok = false;
            }
        }
        let c = &geom.centers[i];
        let gi = dot(&wi[..n], c) + wi[n];
        let (best, _) = tree.argmax(c);
        if best - gi <= 1e-9 * (1.0 + gi.abs()) {
            centers += 1;
        }
    }
    (ok && counted > 0, CertRep { max_violation: max_v, centers_correct: centers })
}

fn count_duplicates(g: &[f64], n: usize) -> usize {
    let w1 = n + 1;
    let r = g.len() / w1;
    let mut keys: Vec<Vec<i64>> = (0..r).map(|i| (0..w1).map(|k| (g[i * w1 + k] * 1e8).round() as i64).collect()).collect();
    keys.sort();
    keys.windows(2).filter(|w| w[0] == w[1]).count()
}

/// Affine value-function pieces [F_r | f_r] of an mpLP solution (objective affine in θ and x,
/// regions that carry a closed-form law). None for mpQP and other objectives.
fn value_function_pieces(sol: &Solution, r: usize, n: usize) -> Option<Vec<f64>> {
    let p = sol.continuous.as_ref()?;
    if p.is_qp() || p.h.max_abs() != 0.0 || p.q_t.max_abs() != 0.0 || sol.critical_regions.len() != r {
        return None;
    }
    let w1 = n + 1;
    let mut g = vec![0.0; r * w1];
    for (i, cr) in sol.critical_regions.iter().enumerate() {
        let obj = |th: &[f64]| p.evaluate_objective(&cr.evaluate(th), th);
        let zero = vec![0.0; n];
        let f0 = obj(&zero);
        for k in 0..n {
            let mut e = vec![0.0; n];
            e[k] = 1.0;
            g[i * w1 + k] = obj(&e) - f0;
        }
        g[i * w1 + n] = f0;
    }
    if g.iter().any(|v| !v.is_finite()) {
        return None;
    }
    Some(g)
}

/// The adjacency LP: g_i - g_j = α_ij (b - a'θ), α_ij >= 1, minimise Σ α. Rows [W | W0] per
/// region; excluded regions get zero weights (and are not in the tree).
fn lp_weights(geom: &PartitionGeometry, excluded: &[bool]) -> Option<Vec<f64>> {
    let n = geom.dim;
    let w = n + 1;
    let r = geom.num_regions;
    let adj: Vec<(usize, usize, usize)> = geom.adjacency_list().into_iter().filter(|&(i, j, _)| !excluded[i] && !excluded[j]).collect();
    let mut lp = SparseLp::new(r * w + adj.len());
    let mut nbrs = vec![vec![]; r];
    for (e, &(i, j, row)) in adj.iter().enumerate() {
        let (ei, fi) = geom.region_rows(i);
        let a = &ei[row * n..(row + 1) * n];
        let b = fi[row];
        let alpha = r * w + e;
        for k in 0..n {
            lp.add_eq(&[(i * w + k, 1.0), (j * w + k, -1.0), (alpha, a[k])], 0.0);
        }
        lp.add_eq(&[(i * w + n, 1.0), (j * w + n, -1.0), (alpha, -b)], 0.0);
        lp.add_le(&[(alpha, -1.0)], -1.0);
        lp.c[alpha] = 1.0;
        nbrs[i].push(j);
        nbrs[j].push(i);
    }
    // gauge: the first region of every connected component, and every excluded region, has zero weights
    let mut seen = vec![false; r];
    for s in 0..r {
        if excluded[s] {
            for k in 0..w {
                lp.add_eq(&[(s * w + k, 1.0)], 0.0);
            }
            continue;
        }
        if seen[s] {
            continue;
        }
        seen[s] = true;
        for k in 0..w {
            lp.add_eq(&[(s * w + k, 1.0)], 0.0);
        }
        let mut st = vec![s];
        while let Some(u) = st.pop() {
            for &v in &nbrs[u] {
                if !seen[v] {
                    seen[v] = true;
                    st.push(v);
                }
            }
        }
    }
    let x = lp.solve()?;
    Some(x[..r * w].to_vec())
}
