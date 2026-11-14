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

const TAG_SHIFT: u32 = 30;
const PAYLOAD: u32 = (1 << TAG_SHIFT) - 1;
const TAG_LEAF: u32 = 1;
const TAG_NODE: u32 = 2;
const MARGIN: f64 = 1e-8;
/// leaves whose test cost (3 per candidate + rows) is at most this are not split further
const SPLIT_COST: f64 = 8.0;
const MAX_DEPTH: u32 = 12;

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

    /// Adaptive variant: tries a few base grids and splits mixed boxes greedily within `max_words`
    /// of storage (table entries plus record words); keeps the variant with the lowest expected cost.
    pub fn new_adaptive(geom: &PartitionGeometry, max_words: usize) -> Self {
        let n = geom.dim;
        let r = geom.num_regions.max(1) as f64;
        let mut best: Option<(f64, HashTable)> = None;
        // deterministic sample of query points in the grid box for empirical selection
        let mut state = 0x1234_5678_9abc_def0u64;
        let pts: Vec<Vec<f64>> = (0..4096)
            .map(|_| {
                (0..n)
                    .map(|k| {
                        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                        let t = (state >> 11) as f64 / (1u64 << 53) as f64;
                        geom.global_lo[k] + t * (geom.global_hi[k] - geom.global_lo[k])
                    })
                    .collect()
            })
            .collect();
        for split in [false, true] {
            for mult in [0.5f64, 2.0, 8.0, 32.0, 128.0] {
                let per = ((r * mult).powf(1.0 / n as f64).round() as usize).max(1);
                let cells = per.pow(n as u32);
                if cells > max_words / 2 {
                    continue;
                }
                let (_, ht) = Self::build_scored(geom, &vec![per; n], if split { max_words } else { 0 }, "hash table (adaptive)", max_words);
                if ht.tab.is_empty() || ht.storage_floats() > max_words {
                    continue;
                }
                let mut t = f64::INFINITY;
                for _ in 0..3 {
                    let t0 = std::time::Instant::now();
                    let mut sink = 0usize;
                    for p in &pts {
                        sink = sink.wrapping_add(std::hint::black_box(&ht).locate(p).unwrap_or(usize::MAX));
                    }
                    std::hint::black_box(sink);
                    t = t.min(t0.elapsed().as_secs_f64());
                }
                if best.as_ref().map(|b| t < b.0).unwrap_or(true) {
                    best = Some((t, ht));
                }
            }
        }
        match best {
            Some((_, ht)) => ht,
            None => Self::build(geom, &vec![1; n], 0, "hash table (adaptive)"),
        }
    }

    fn build(geom: &PartitionGeometry, cells_per_dim: &[usize], cap: usize, name: &'static str) -> Self {
        Self::build_scored(geom, cells_per_dim, cap, name, usize::MAX).1
    }

    fn build_scored(geom: &PartitionGeometry, cells_per_dim: &[usize], cap: usize, name: &'static str, hard_cap: usize) -> (f64, Self) {
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
        let mut words = total + 1 + grows.len() + arena.iter().map(|nd| record_words(&nd.cands)).sum::<usize>();
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
        HashTable { dim: n, lo: vec![], inv_h: vec![], cells_per_dim: cells_per_dim.to_vec(), strides: vec![], tab: vec![], sentinel: 0, grows: vec![], stream: vec![], name: "", mixed: 0, leaves: 0, total_cands: 0, total_rows: 0, max_cands: 0, max_depth: 0 }
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

    fn storage_floats(&self) -> usize {
        self.tab.len() + self.stream.len() + self.grows.len() + 3 * self.dim
    }
}
