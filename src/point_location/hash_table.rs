//! Hash table point location (Bayat, Johansen & Jalali, 2011, "Using hash tables to manage the
//! time-storage complexity in a point location problem: Application to explicit model predictive
//! control", Automatica 47(3)).
//!
//! The bounding box of the partition is split into a uniform grid of boxes. Online, the box index
//! is computed with n floor operations. A box is one of
//!   * empty (no region can contain a point of it): answer None,
//!   * covered (a region contains the whole box, proven offline with a margin): answer that region,
//!   * mixed: a short record of candidate regions, each with only the rows that can cut the box;
//!     the candidates are tested in order with the same f64 halfspace test as sequential search.
//! In the adaptive variant, mixed boxes are recursively split into 2^n children (a 2^n-tree
//! below the base grid) while a storage budget allows, greedily where the expected query cost is
//! highest, so most queries end at an empty or covered leaf after a few table loads.
//!
//! Exactness: a row is dropped from a box only if every point of the (slightly enlarged) box
//! satisfies it with margin 1e-8, a candidate is dropped only if some row is violated by every
//! point of the enlarged box by more than the tolerance. Neither can change the answer.

use super::geometry::PartitionGeometry;
use super::{PointLocator, LOCATE_TOL};
use crate::linalg::Mat;
use crate::utils::chebyshev_ball;
use rayon::prelude::*;
use std::collections::BinaryHeap;

mod persist;
pub use persist::HASH_TABLE_FORMAT_VERSION;

const TAG_SHIFT: u32 = 30;
const PAYLOAD: u32 = (1 << TAG_SHIFT) - 1;
const TAG_LEAF: u32 = 1;
const TAG_NODE: u32 = 2;
const MARGIN: f64 = 1e-8;
/// leaves whose test cost (3 per candidate + rows) is at most this are not split further
const SPLIT_COST: f64 = 8.0;
const MAX_DEPTH: u32 = 12;
/// storage limit used when a caller gives none
pub const DEFAULT_STORAGE_BYTES: usize = 4 << 20;

pub struct HashTable {
    dim: usize,
    lo: Vec<f64>,
    inv_h: Vec<f64>,
    cells_per_dim: Vec<usize>,
    strides: Vec<usize>,
    /// entry: tag 0 = pure (0 none, 1 + j region j), tag 1 = leaf record offset in `stream`,
    /// tag 2 = index of a block of 2^n child entries. The root block is the base grid followed
    /// by one sentinel entry (0) used for queries outside the grid.
    tab: Vec<u32>,
    sentinel: usize,
    /// all region rows packed [a | b] (stride n + 1); region j owns rows offsets[j]..offsets[j+1]
    /// leaf record (u32 words): [ncand, then per candidate: region, nrows, nrows row ids]
    grows: Vec<f64>,
    /// compact records: candidates are bare region ids and all rows of the region are tested
    compact: bool,
    offs: Vec<u32>,
    stream: Vec<u32>,
    name: &'static str,
    mixed: usize,
    leaves: usize,
    total_cands: usize,
    total_rows: usize,
    max_cands: usize,
    max_depth: usize,
}

/// candidate regions with the ids (into the global packed row table) of their rows that can cut the box
type Cands = Vec<(u32, Vec<u32>)>;

struct Node {
    cands: Cands,
    depth: u32,
    lo: Vec<f64>,
    hi: Vec<f64>,
    children: Vec<usize>,
}

fn is_pure(c: &Cands) -> bool {
    c.is_empty() || (c.len() == 1 && c[0].1.is_empty())
}

fn record_words(c: &Cands) -> usize {
    if is_pure(c) {
        0
    } else {
        1 + c.iter().map(|(_, r)| 2 + r.len()).sum::<usize>()
    }
}

fn leaf_cost(c: &Cands) -> f64 {
    if is_pure(c) {
        0.0
    } else {
        3.0 * c.len() as f64 + c.iter().map(|(_, r)| r.len()).sum::<usize>() as f64
    }
}

/// Keep, per candidate, the rows that can reject a point of the box [elo, ehi]; drop candidates
/// that are rejected everywhere; stop after a candidate that covers the box.
fn refine(parent: &Cands, grows: &[f64], n: usize, elo: &[f64], ehi: &[f64]) -> Cands {
    let w = n + 1;
    let mut out: Cands = vec![];
    for (j, rows) in parent {
        let mut keep: Vec<u32> = vec![];
        let mut sep = false;
        for &id in rows {
            let row = &grows[id as usize * w..(id as usize + 1) * w];
            let (mut mx, mut mn) = (0.0, 0.0);
            for k in 0..n {
                let (p, q) = (row[k] * elo[k], row[k] * ehi[k]);
                mx += p.max(q);
                mn += p.min(q);
            }
            if mn > row[n] + LOCATE_TOL + MARGIN {
                sep = true;
                break;
            }
            if mx > row[n] - MARGIN {
                keep.push(id);
            }
        }
        if sep {
            continue;
        }
        let empty = keep.is_empty();
        out.push((*j, keep));
        if empty {
            break;
        }
    }
    out
}

/// Drop candidates whose region does not meet the box in a set of positive volume (Chebyshev
/// radius LP over the candidate's active rows plus the box).
fn lp_filter(cands: Cands, grows: &[f64], n: usize, blo: &[f64], bhi: &[f64]) -> Cands {
    let w = n + 1;
    let mut out: Cands = vec![];
    for (j, rows) in cands {
        if !rows.is_empty() {
            let m = rows.len();
            let mut a = Mat::zeros(m + 2 * n, n);
            let mut b = vec![0.0; m + 2 * n];
            for (r, &id) in rows.iter().enumerate() {
                let row = &grows[id as usize * w..(id as usize + 1) * w];
                for k in 0..n {
                    a[(r, k)] = row[k];
                }
                b[r] = row[n];
            }
            for k in 0..n {
                a[(m + 2 * k, k)] = 1.0;
                b[m + 2 * k] = bhi[k];
                a[(m + 2 * k + 1, k)] = -1.0;
                b[m + 2 * k + 1] = -blo[k];
            }
            if !chebyshev_ball(&a, &b, &[]).map(|s| s.x[n] > 1e-12).unwrap_or(false) {
                continue;
            }
        }
        let empty = rows.is_empty();
        out.push((j, rows));
        if empty {
            break;
        }
    }
    out
}

/// Fixed seeded sample of `count` points in the bounding box of the partition.
fn sample_points(geom: &PartitionGeometry, count: usize) -> Vec<Vec<f64>> {
    let mut state = 0x1234_5678_9abc_def0u64;
    (0..count)
        .map(|_| {
            (0..geom.dim)
                .map(|k| {
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    let t = (state >> 11) as f64 / (1u64 << 53) as f64;
                    geom.global_lo[k] + t * (geom.global_hi[k] - geom.global_lo[k])
                })
                .collect()
        })
        .collect()
}

impl HashTable {
    /// Uniform grid with about `target_cells` boxes (split evenly across dimensions), no splitting.
    pub fn new(geom: &PartitionGeometry, target_cells: usize) -> Self {
        let n = geom.dim;
        let per = ((target_cells.max(1) as f64).powf(1.0 / n as f64).round() as usize).max(1);
        Self::build(geom, &vec![per; n], 0, "hash table")
    }

    pub fn with_grid(geom: &PartitionGeometry, cells_per_dim: &[usize]) -> Self {
        Self::build(geom, cells_per_dim, 0, "hash table")
    }

    /// Fastest exact table whose storage ([`HashTable::storage_bytes`]) fits in `max_bytes`.
    ///
    /// Uniform grids of about 0.5x to 256x the region count are built in increasing size. Each
    /// is scored by a predicted cost computed from the table's own statistics over a fixed seeded
    /// sample of points in the parameter box (no timing, so the same input always gives the same
    /// table): per query a constant, plus a mixed-leaf entry cost, plus the candidates visited and
    /// halfspace rows evaluated, plus a small working-set term. The search stops when the next grid
    /// no longer fits or after two grids in a row that do not improve the prediction. The row table
    /// that every method needs (8 bytes per coefficient) cannot be shrunk: if `max_bytes` is below
    /// the smallest table (one box), that smallest table is returned.
    pub fn with_storage_limit(geom: &PartitionGeometry, max_bytes: usize) -> Self {
        let n = geom.dim;
        let r = geom.num_regions.max(1) as f64;
        let pts = sample_points(geom, 2048);
        let mut best: Option<(f64, HashTable)> = None;
        let mut last_per = 0usize;
        let mut worse = 0;
        let mults = [0.25f64, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0];
        for (mi, &mult) in mults.iter().enumerate() {
            let per = ((r * mult).powf(1.0 / n as f64).round() as usize).max(1);
            if per == last_per && mi > 0 {
                continue;
            }
            last_per = per;
            let cells = per.checked_pow(n as u32).unwrap_or(usize::MAX);
            let base_bytes = cells.saturating_mul(4).saturating_add(geom.f.len() * (n + 1) * 8);
            if base_bytes > max_bytes && best.is_some() {
                break;
            }
            // pruned records are faster; bare candidate lists are smaller. Try both.
            let mut any_fit = false;
            for compact in [false, true] {
                let ht = Self::build_scored(geom, &vec![per; n], 0, "hash table", usize::MAX, compact).1;
                let fits = ht.storage_bytes() <= max_bytes;
                if !fits && best.is_some() {
                    continue;
                }
                any_fit |= fits;
                let cost = ht.predicted_cost(&pts);
                if best.as_ref().map(|b| cost < b.0).unwrap_or(true) {
                    worse = 0;
                    best = Some((cost, ht));
                } else if fits {
                    worse += 1;
                }
                if fits && !compact {
                    // the pruned table fits: a bare-list table of the same grid is only
                    // interesting when smaller, which it never helps in speed
                    break;
                }
            }
            if !any_fit || worse >= 3 {
                break;
            }
        }
        match best {
            Some((_, ht)) => ht,
            None => Self::build(geom, &vec![1; n], 0, "hash table"),
        }
    }

    /// [`HashTable::with_storage_limit`] with [`DEFAULT_STORAGE_BYTES`].
    pub fn with_default_limit(geom: &PartitionGeometry) -> Self {
        Self::with_storage_limit(geom, DEFAULT_STORAGE_BYTES)
    }

    /// Experimental adaptive 2^n-tree below a uniform base grid, split greedily within
    /// `max_bytes`. It was not faster than the best uniform grid for equal storage on the benchmark
    /// problems, so `with_storage_limit` does not use it.
    pub fn with_adaptive_split(geom: &PartitionGeometry, cells_per_dim: &[usize], max_bytes: usize) -> Self {
        Self::build_scored(geom, cells_per_dim, max_bytes / 4, "hash table (adaptive)", max_bytes / 4, false).1
    }

    pub fn with_name(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    /// Storage in bytes: table entries and leaf records (u32) plus the shared row table (f64).
    pub fn storage_bytes(&self) -> usize {
        4 * (self.tab.len() + self.stream.len() + self.offs.len()) + 8 * self.grows.len() + 8 * 3 * self.dim
    }

    /// Predicted per-query cost (arbitrary units, roughly ns) over the given points.
    pub fn predicted_cost(&self, pts: &[Vec<f64>]) -> f64 {
        let mut total = 0.0;
        for p in pts {
            total += self.query_cost(p);
        }
        let bytes = self.storage_bytes() as f64;
        total / pts.len().max(1) as f64 + 0.15 * (bytes / 65536.0).max(1.0).log2()
    }

    /// Cost of one query from the work the lookup does: levels, candidates visited, rows evaluated.
    fn query_cost(&self, theta: &[f64]) -> f64 {
        let n = self.dim;
        let mut cell = 0usize;
        let mut inside = true;
        let mut fr = [0.0f64; 8];
        for k in 0..n {
            let u = (theta[k] - self.lo[k]) * self.inv_h[k];
            let m = self.cells_per_dim[k];
            inside &= (u >= -1e-9) & (u <= m as f64 + 1e-9);
            let c = (u as usize).min(m - 1);
            cell += c * self.strides[k];
            fr[k] = u - c as f64;
        }
        let mut cost = 10.0;
        let mut e = self.tab[if inside { cell } else { self.sentinel }];
        while e >> TAG_SHIFT == TAG_NODE {
            let mut ci = 0usize;
            for k in 0..n {
                let f = fr[k] * 2.0;
                let b = (f >= 1.0) as usize;
                fr[k] = f - b as f64;
                ci |= b << k;
            }
            e = self.tab[(e & PAYLOAD) as usize + ci];
            cost += 2.0;
        }
        if e >> TAG_SHIFT != TAG_LEAF {
            return cost;
        }
        cost += 8.0;
        let w = n + 1;
        let mut p = (e & PAYLOAD) as usize;
        let nc = self.stream[p] as usize;
        p += 1;
        if self.compact {
            for &j in &self.stream[p..p + nc] {
                cost += 3.0;
                let (s, t) = (self.offs[j as usize] as usize, self.offs[j as usize + 1] as usize);
                let mut ok = true;
                for id in s..t {
                    cost += 1.5;
                    let row = &self.grows[id * w..(id + 1) * w];
                    let d: f64 = (0..n).map(|k| row[k] * theta[k]).sum();
                    if d > row[n] + LOCATE_TOL {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    break;
                }
            }
            return cost;
        }
        for _ in 0..nc {
            let nr = self.stream[p + 1] as usize;
            cost += 3.0;
            let mut ok = true;
            for &id in &self.stream[p + 2..p + 2 + nr] {
                cost += 1.5;
                let row = &self.grows[id as usize * w..(id as usize + 1) * w];
                let d: f64 = (0..n).map(|k| row[k] * theta[k]).sum();
                if d > row[n] + LOCATE_TOL {
                    ok = false;
                    break;
                }
            }
            if ok {
                break;
            }
            p += 2 + nr;
        }
        cost
    }

    fn build(geom: &PartitionGeometry, cells_per_dim: &[usize], cap: usize, name: &'static str) -> Self {
        Self::build_scored(geom, cells_per_dim, cap, name, usize::MAX, false).1
    }

    fn build_scored(geom: &PartitionGeometry, cells_per_dim: &[usize], cap: usize, name: &'static str, hard_cap: usize, compact: bool) -> (f64, Self) {
        let n = geom.dim;
        assert!(n >= 1 && n <= 8, "hash table supports 1 to 8 parameters");
        let w = n + 1;
        let lo = geom.global_lo.clone();
        let hi = geom.global_hi.clone();
        let h: Vec<f64> = (0..n).map(|k| ((hi[k] - lo[k]) / cells_per_dim[k] as f64).max(1e-12)).collect();
        let inv_h: Vec<f64> = h.iter().map(|v| 1.0 / v).collect();
        let eps: Vec<f64> = h.iter().map(|v| 1e-8 * v).collect();
        let mut strides = vec![1usize; n];
        for k in 1..n {
            strides[k] = strides[k - 1] * cells_per_dim[k - 1];
        }
        let total: usize = cells_per_dim.iter().product();

        // regions in order of decreasing size, their full row lists, and the base cells their
        // (slightly enlarged) bounding boxes touch
        let mut order: Vec<usize> = (0..geom.num_regions).collect();
        order.sort_by(|&a, &b| geom.radii[b].partial_cmp(&geom.radii[a]).unwrap());
        let mut lists: Vec<Vec<u32>> = vec![vec![]; total];
        for &i in &order {
            let cl: Vec<usize> = (0..n).map(|k| (((geom.bbox_lo[i][k] - eps[k] - lo[k]) * inv_h[k]).floor().max(0.0) as usize).min(cells_per_dim[k] - 1)).collect();
            let ch: Vec<usize> = (0..n).map(|k| (((geom.bbox_hi[i][k] + eps[k] - lo[k]) * inv_h[k]).floor().max(0.0) as usize).min(cells_per_dim[k] - 1)).collect();
            let mut idx = cl.clone();
            'outer: loop {
                let cell: usize = (0..n).map(|k| idx[k] * strides[k]).sum();
                lists[cell].push(i as u32);
                let mut k = 0;
                loop {
                    if k == n {
                        break 'outer;
                    }
                    if idx[k] < ch[k] {
                        idx[k] += 1;
                        break;
                    }
                    idx[k] = cl[k];
                    k += 1;
                }
            }
        }
        let mut grows: Vec<f64> = Vec::with_capacity(geom.f.len() * w);
        for r in 0..geom.f.len() {
            grows.extend_from_slice(&geom.e[r * n..(r + 1) * n]);
            grows.push(geom.f[r]);
        }
        let mut arena: Vec<Node> = (0..total)
            .into_par_iter()
            .map(|c| {
                let mut blo = vec![0.0; n];
                let mut bhi = vec![0.0; n];
                for k in 0..n {
                    let ik = (c / strides[k]) % cells_per_dim[k];
                    blo[k] = lo[k] + ik as f64 * h[k];
                    bhi[k] = lo[k] + (ik + 1) as f64 * h[k];
                }
                let elo: Vec<f64> = (0..n).map(|k| blo[k] - eps[k]).collect();
                let ehi: Vec<f64> = (0..n).map(|k| bhi[k] + eps[k]).collect();
                let parent: Cands = lists[c].iter().map(|&j| (j, (geom.offsets[j as usize] as u32..geom.offsets[j as usize + 1] as u32).collect())).collect();
                let cands = lp_filter(refine(&parent, &grows, n, &elo, &ehi), &grows, n, &blo, &bhi);
                Node { cands, depth: 0, lo: blo, hi: bhi, children: vec![] }
            })
            .collect();
        drop(lists);

        // greedy splitting of the most expensive mixed boxes within the storage cap
        let mut words = total + 1 + 2 * grows.len() + arena.iter().map(|nd| record_words(&nd.cands)).sum::<usize>();
        if words > hard_cap || (cap > 0 && words > cap) {
            return (f64::INFINITY, Self::empty_like(n, cells_per_dim));
        }
        let prio = |nd: &Node| -> u64 { (0.5f64.powi((n as u32 * nd.depth) as i32) * leaf_cost(&nd.cands)).to_bits() };
        let splittable = |nd: &Node| nd.depth < MAX_DEPTH && n <= 4 && leaf_cost(&nd.cands) > SPLIT_COST;
        let mut heap: BinaryHeap<(u64, usize)> = BinaryHeap::new();
        if cap > 0 {
            for (i, nd) in arena.iter().enumerate() {
                if splittable(nd) {
                    heap.push((prio(nd), i));
                }
            }
        }
        let nchild = 1usize << n;
        loop {
            let mut batch: Vec<usize> = vec![];
            while batch.len() < 32 {
                match heap.pop() {
                    Some((_, i)) => batch.push(i),
                    None => break,
                }
            }
            if batch.is_empty() {
                break;
            }
            let results: Vec<Vec<Node>> = batch
                .par_iter()
                .map(|&idx| {
                    let (plo, phi, depth) = (&arena[idx].lo, &arena[idx].hi, arena[idx].depth);
                    (0..nchild)
                        .map(|ci| {
                            let mut clo = vec![0.0; n];
                            let mut chi = vec![0.0; n];
                            for k in 0..n {
                                let mid = 0.5 * (plo[k] + phi[k]);
                                if (ci >> k) & 1 == 1 {
                                    clo[k] = mid;
                                    chi[k] = phi[k];
                                } else {
                                    clo[k] = plo[k];
                                    chi[k] = mid;
                                }
                            }
                            let elo: Vec<f64> = (0..n).map(|k| clo[k] - eps[k]).collect();
                            let ehi: Vec<f64> = (0..n).map(|k| chi[k] + eps[k]).collect();
                            let cands = lp_filter(refine(&arena[idx].cands, &grows, n, &elo, &ehi), &grows, n, &clo, &chi);
                            Node { cands, depth: depth + 1, lo: clo, hi: chi, children: vec![] }
                        })
                        .collect()
                })
                .collect();
            for (&idx, kids) in batch.iter().zip(results) {
                let new_words = words - record_words(&arena[idx].cands) + nchild + kids.iter().map(|k| record_words(&k.cands)).sum::<usize>();
                if new_words > cap {
                    continue;
                }
                words = new_words;
                let first = arena.len();
                for kd in kids {
                    if splittable(&kd) {
                        heap.push((prio(&kd), arena.len()));
                    }
                    arena.push(kd);
                }
                arena[idx].children = (first..first + nchild).collect();
                arena[idx].cands = vec![];
            }
        }

        // expected cost: sum over leaves of volume * (levels + leaf test cost)
        fn score(arena: &[Node], i: usize, n: usize, depth: u32) -> f64 {
            let nd = &arena[i];
            if nd.children.is_empty() {
                0.5f64.powi((n as u32 * depth) as i32) * (2.0 * depth as f64 + 2.0 + leaf_cost(&nd.cands))
            } else {
                nd.children.iter().map(|&c| score(arena, c, n, depth + 1)).sum()
            }
        }
        let sc: f64 = (0..total).map(|c| score(&arena, c, n, 0)).sum::<f64>() / total as f64;

        // serialize
        let mut ht = HashTable {
            dim: n,
            lo,
            inv_h,
            cells_per_dim: cells_per_dim.to_vec(),
            strides,
            tab: vec![0u32; total + 1],
            sentinel: total,
            grows,
            compact,
            offs: geom.offsets.iter().map(|&o| o as u32).collect(),
            stream: vec![],
            name,
            mixed: 0,
            leaves: 0,
            total_cands: 0,
            total_rows: 0,
            max_cands: 0,
            max_depth: 0,
        };
        for c in 0..total {
            let e = ht.emit(&mut arena, c, geom, 0);
            ht.tab[c] = e;
        }
        assert!(ht.stream.len() < PAYLOAD as usize && ht.tab.len() < PAYLOAD as usize);
        (sc, ht)
    }

    fn empty_like(n: usize, cells_per_dim: &[usize]) -> Self {
        HashTable { dim: n, lo: vec![], inv_h: vec![], cells_per_dim: cells_per_dim.to_vec(), strides: vec![], tab: vec![], sentinel: 0, grows: vec![], compact: false, offs: vec![], stream: vec![], name: "", mixed: 0, leaves: 0, total_cands: 0, total_rows: 0, max_cands: 0, max_depth: 0 }
    }

    fn emit(&mut self, arena: &mut Vec<Node>, i: usize, geom: &PartitionGeometry, depth: usize) -> u32 {
        let n = self.dim;
        self.max_depth = self.max_depth.max(depth);
        if !arena[i].children.is_empty() {
            let kids = arena[i].children.clone();
            let base = self.tab.len();
            self.tab.extend(std::iter::repeat(0u32).take(kids.len()));
            for (ci, &kid) in kids.iter().enumerate() {
                let e = self.emit(arena, kid, geom, depth + 1);
                self.tab[base + ci] = e;
            }
            return (TAG_NODE << TAG_SHIFT) | base as u32;
        }
        self.leaves += 1;
        let mut cands = std::mem::take(&mut arena[i].cands);
        if cands.is_empty() {
            return 0;
        }
        if cands.len() == 1 && cands[0].1.is_empty() {
            return cands[0].0 + 1;
        }
        // order candidates by how many sample points of the box they contain (likely region first)
        if cands.len() > 1 && cands.iter().all(|(_, r)| !r.is_empty()) {
            let samples = 24usize;
            let mut state = 0x9E3779B97F4A7C15u64 ^ i as u64;
            let (blo, bhi) = (&arena[i].lo, &arena[i].hi);
            let mut pts: Vec<Vec<f64>> = vec![];
            for _ in 0..samples {
                let mut p = vec![0.0; n];
                for k in 0..n {
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    let t = (state >> 11) as f64 / (1u64 << 53) as f64;
                    p[k] = blo[k] + t * (bhi[k] - blo[k]);
                }
                pts.push(p);
            }
            let hits: Vec<usize> = cands
                .iter()
                .map(|(j, _)| {
                    let (e, f) = geom.region_rows(*j as usize);
                    pts.iter().filter(|p| (0..f.len()).all(|r| (0..n).map(|k| e[r * n + k] * p[k]).sum::<f64>() <= f[r] + LOCATE_TOL)).count()
                })
                .collect();
            let mut idx: Vec<usize> = (0..cands.len()).collect();
            idx.sort_by_key(|&q| std::cmp::Reverse(hits[q]));
            cands = idx.into_iter().map(|q| cands[q].clone()).collect();
        }
        let off = self.stream.len();
        self.mixed += 1;
        self.max_cands = self.max_cands.max(cands.len());
        self.total_cands += cands.len();
        self.stream.push(cands.len() as u32);
        if self.compact {
            for (j, _) in &cands {
                self.total_rows += (self.offs[*j as usize + 1] - self.offs[*j as usize]) as usize;
                self.stream.push(*j);
            }
            return (TAG_LEAF << TAG_SHIFT) | off as u32;
        }
        for (j, rws) in &cands {
            self.total_rows += rws.len();
            self.stream.push(*j);
            self.stream.push(rws.len() as u32);
            self.stream.extend_from_slice(rws);
        }
        (TAG_LEAF << TAG_SHIFT) | off as u32
    }

    #[inline(always)]
    fn locate_g<const N: usize>(&self, theta: &[f64]) -> Option<usize> {
        let theta: &[f64; N] = theta[..N].try_into().unwrap();
        let mut cell = 0usize;
        let mut inside = true;
        let mut fr = [0.0f64; N];
        for k in 0..N {
            let u = (theta[k] - self.lo[k]) * self.inv_h[k];
            let m = self.cells_per_dim[k];
            // NaN fails both comparisons and counts as outside
            inside &= (u >= -1e-9) & (u <= m as f64 + 1e-9);
            let c = (u as usize).min(m - 1);
            cell += c * self.strides[k];
            fr[k] = u - c as f64;
        }
        let mut e = self.tab[if inside { cell } else { self.sentinel }];
        while e >> TAG_SHIFT == TAG_NODE {
            let mut ci = 0usize;
            for k in 0..N {
                let f = fr[k] * 2.0;
                let b = (f >= 1.0) as usize;
                fr[k] = f - b as f64;
                ci |= b << k;
            }
            e = self.tab[(e & PAYLOAD) as usize + ci];
        }
        if e >> TAG_SHIFT != TAG_LEAF {
            return (e as usize).checked_sub(1);
        }
        let mut p = (e & PAYLOAD) as usize;
        let nc = self.stream[p] as usize;
        p += 1;
        if self.compact {
            for &j in &self.stream[p..p + nc] {
                let (s, t) = (self.offs[j as usize] as usize, self.offs[j as usize + 1] as usize);
                let mut ok = true;
                for row in self.grows[s * (N + 1)..t * (N + 1)].chunks_exact(N + 1) {
                    let mut d = 0.0;
                    for k in 0..N {
                        d += row[k] * theta[k];
                    }
                    if d > row[N] + LOCATE_TOL {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    return Some(j as usize);
                }
            }
            return None;
        }
        for _ in 0..nc {
            let j = self.stream[p];
            let nr = self.stream[p + 1] as usize;
            let mut ok = true;
            for &id in &self.stream[p + 2..p + 2 + nr] {
                let row = &self.grows[id as usize * (N + 1)..(id as usize + 1) * (N + 1)];
                let mut d = 0.0;
                for k in 0..N {
                    d += row[k] * theta[k];
                }
                if d > row[N] + LOCATE_TOL {
                    ok = false;
                    break;
                }
            }
            if ok {
                return Some(j as usize);
            }
            p += 2 + nr;
        }
        None
    }

    /// leaves that need row tests
    pub fn mixed_cells(&self) -> usize {
        self.mixed
    }

    pub fn leaves(&self) -> usize {
        self.leaves
    }

    pub fn max_depth(&self) -> usize {
        self.max_depth
    }

    pub fn mean_rows_per_mixed_cell(&self) -> f64 {
        self.total_rows as f64 / self.mixed.max(1) as f64
    }

    /// base grid boxes
    pub fn num_cells(&self) -> usize {
        self.cells_per_dim.iter().product()
    }

    pub fn mean_candidates(&self) -> f64 {
        self.total_cands as f64 / self.mixed.max(1) as f64
    }

    pub fn max_candidates(&self) -> usize {
        self.max_cands
    }
}

impl PointLocator for HashTable {
    fn name(&self) -> &'static str {
        self.name
    }

    #[inline]
    fn locate(&self, theta: &[f64]) -> Option<usize> {
        match self.dim {
            1 => self.locate_g::<1>(theta),
            2 => self.locate_g::<2>(theta),
            3 => self.locate_g::<3>(theta),
            4 => self.locate_g::<4>(theta),
            5 => self.locate_g::<5>(theta),
            6 => self.locate_g::<6>(theta),
            7 => self.locate_g::<7>(theta),
            _ => self.locate_g::<8>(theta),
        }
    }

    /// storage in units of 8 bytes (see [`HashTable::storage_bytes`])
    fn storage_floats(&self) -> usize {
        self.storage_bytes().div_ceil(8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// k x k unit-square cells as a synthetic 2D partition, each cell with 4 rows.
    fn square_partition(k: usize) -> PartitionGeometry {
        let mut e = vec![];
        let mut f = vec![];
        let mut offsets = vec![0];
        let (mut bl, mut bh, mut radii) = (vec![], vec![], vec![]);
        let h = 1.0 / k as f64;
        for i in 0..k {
            for j in 0..k {
                let (x0, x1, y0, y1) = (i as f64 * h, (i + 1) as f64 * h, j as f64 * h, (j + 1) as f64 * h);
                e.extend_from_slice(&[1.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, -1.0]);
                f.extend_from_slice(&[x1, -x0, y1, -y0]);
                offsets.push(f.len());
                bl.push(vec![x0, y0]);
                bh.push(vec![x1, y1]);
                radii.push(h / 2.0);
            }
        }
        PartitionGeometry {
            dim: 2,
            num_regions: k * k,
            e,
            f,
            offsets,
            centers: vec![],
            radii,
            vertices: vec![],
            bbox_lo: bl,
            bbox_hi: bh,
            planes: vec![],
            facets: vec![],
            domain_planes: vec![],
            global_lo: vec![0.0, 0.0],
            global_hi: vec![1.0, 1.0],
        }
    }

    #[test]
    fn storage_limit_respected_and_deterministic() {
        let geom = square_partition(6);
        let mut prev_cost = f64::INFINITY;
        for limit in [6_000usize, 20_000, 100_000, 1_000_000] {
            let a = HashTable::with_storage_limit(&geom, limit);
            let b = HashTable::with_storage_limit(&geom, limit);
            assert!(a.storage_bytes() <= limit, "limit {limit}: {} bytes", a.storage_bytes());
            assert_eq!(a.tab, b.tab);
            assert_eq!(a.stream, b.stream);
            assert_eq!(a.cells_per_dim, b.cells_per_dim);
            // exact against brute force on a seeded sample
            for p in sample_points(&geom, 3000) {
                let truth = (0..geom.num_regions).find(|&i| geom.contains(i, &p, LOCATE_TOL));
                let got = a.locate(&p);
                assert!(got == truth || got.map(|g| geom.contains(g, &p, LOCATE_TOL)).unwrap_or(false));
            }
            // more storage never predicts a slower table
            let c = a.predicted_cost(&sample_points(&geom, 2048));
            assert!(c <= prev_cost + 1e-9);
            prev_cost = c;
        }
    }
}
