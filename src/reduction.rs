//! Complexity reduction of explicit solutions.
//!
//! 1. **Optimal region merging**, Geyer, Torrisi, Morari, "Optimal complexity reduction of
//!    polyhedral piecewise affine systems", Automatica 44(7), 2008 (DOI 10.1016/j.automatica.2007.11.027).
//!    Regions with the same (affine) law, the same "colour", are merged into the minimum number
//!    of disjoint convex polyhedra:
//!    * Algorithm 3 (Sec. 7.1): the hyperplane arrangement of all facets is collected and every
//!      region is cut into the cells of the arrangement; each cell carries a marking (the side of
//!      every hyperplane).
//!    * Algorithm 1 (Sec. 5.1, disjoint optimal complexity reduction, branch and bound on the
//!      markings): envelope of the white cells (Lemma 12), black cells inside the envelope,
//!      branching on the hyperplanes that separate white cells, with the bounds z and z-bar and the
//!      branching heuristics of Sec. 5.2 (connected components first, then hyperplanes with only
//!      white cells on one side).
//!
//!    The merge key is a target: the full law x(theta) ([`ReduceTarget::FullLaw`]) or only some of
//!    its components, typically the first control move u0 ([`ReduceTarget::FirstInputs`]).
//!
//! 2. **Clipping**, Kvasnica, Fikar, "Clipping-based complexity reduction in explicit MPC", IEEE
//!    TAC 57(7), 2012 (DOI 10.1109/TAC.2011.2179428): Algorithm 1 (suitable augmentation: saturated
//!    regions are removed and covered by extending their unsaturated neighbours), with the filter
//!    phi(k) = max(lo, min(k, hi)) of Theorem 4.12. Several inputs are processed one component at
//!    a time (Sec. IV-B), each with its own partition and clipping filter. The extended regions
//!    may overlap each other inside saturated areas (any of them gives the same clipped value, as in
//!    the paper); they never overlap an unsaturated original region.

use crate::critical_region::CriticalRegion;
use crate::linalg::{dot, norm, Mat};
use crate::point_location::geometry::{enumerate_vertices, PartitionGeometry};
use crate::solution::Solution;
use crate::utils::chebyshev_ball;
use rayon::prelude::*;
use std::cell::Cell as StdCell;
use std::collections::{HashMap, VecDeque};

const SIDE_TOL: f64 = 1e-7;
const RAD_EPS: f64 = 1e-8;

// ------------------------------------------------------------------------------------------------
// Target and options
// ------------------------------------------------------------------------------------------------

/// Which components of the optimizer must keep their law.
#[derive(Clone, Debug, PartialEq)]
pub enum ReduceTarget {
    /// The whole optimizer x(theta).
    FullLaw,
    /// The first k components of x(theta) (the first control moves u0 when x stacks the inputs).
    FirstInputs(usize),
    /// Explicit component indices (e.g. u0 when states precede inputs in x).
    Components(Vec<usize>),
}

impl ReduceTarget {
    /// Component indices for an optimizer of `nx` components.
    pub fn indices(&self, nx: usize) -> Vec<usize> {
        match self {
            ReduceTarget::FullLaw => (0..nx).collect(),
            ReduceTarget::FirstInputs(k) => (0..(*k).min(nx)).collect(),
            ReduceTarget::Components(v) => v.iter().copied().filter(|&i| i < nx).collect(),
        }
    }

    /// Target of a benchmark problem JSON: reads the `"mpc"` key (A, B, N). If the optimizer is the
    /// stacked input sequence (N n_u variables) it is `FirstInputs(n_u)`; if states precede inputs
    /// (N (n_x + n_u) variables) it is the components of u0; without `"mpc"` it is `FullLaw`.
    pub fn from_problem_json(path: &std::path::Path) -> ReduceTarget {
        Self::try_from_problem_json(path).unwrap_or(ReduceTarget::FullLaw)
    }

    fn try_from_problem_json(path: &std::path::Path) -> Option<ReduceTarget> {
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
        let mpc = v.get("mpc")?;
        let nx = mpc.get("A")?.as_array()?.len();
        let nu = mpc.get("B")?.as_array()?.first()?.as_array()?.len();
        let n = mpc.get("N")?.as_u64()? as usize;
        let nvars = v.get("A")?.as_array()?.first()?.as_array()?.len();
        if nvars == n * (nx + nu) && nx > 0 {
            Some(ReduceTarget::Components((nx * n..nx * n + nu).collect()))
        } else {
            Some(ReduceTarget::FirstInputs(nu))
        }
    }
}

/// Saturation bounds (lo, hi) of the input components `indices` read from the `u_poly` of the
/// `"mpc"` entry (rows that bound a single input by +-e_j); `None` for a missing side.
pub fn input_bounds_from_json(path: &std::path::Path, target: &ReduceTarget, nx: usize) -> Vec<(Option<f64>, Option<f64>)> {
    let idx = target.indices(nx);
    let mut out = vec![(None, None); idx.len()];
    let Ok(txt) = std::fs::read_to_string(path) else { return out };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) else { return out };
    let Some(up) = v.get("mpc").and_then(|m| m.get("u_poly")) else { return out };
    let (Some(h), Some(k)) = (up.get("H").and_then(|h| h.as_array()), up.get("k").and_then(|k| k.as_array())) else { return out };
    // input number of component idx[c]: components are the input components in order
    for (c, slot) in out.iter_mut().enumerate() {
        for (row, kk) in h.iter().zip(k) {
            let row: Vec<f64> = row.as_array().map(|r| r.iter().map(|x| x.as_f64().unwrap_or(0.0)).collect()).unwrap_or_default();
            let nz: Vec<usize> = (0..row.len()).filter(|&j| row[j].abs() > 1e-12).collect();
            if nz.len() == 1 && nz[0] == c {
                let bound = kk.as_f64().unwrap_or(f64::NAN) / row[c];
                if row[c] > 0.0 {
                    slot.1 = Some(bound);
                } else {
                    slot.0 = Some(bound);
                }
            }
        }
    }
    out
}

#[derive(Clone, Debug)]
pub struct ReduceOptions {
    /// Two laws are the same if all coefficients agree within `law_tol * (1 + |value|)`.
    pub law_tol: f64,
    /// Branch and bound nodes per colour before the greedy fallback takes over.
    pub bnb_node_budget: usize,
    /// If the arrangement has more cells than this, greedy pairwise merging of regions is used instead.
    pub max_cells: usize,
}

impl Default for ReduceOptions {
    fn default() -> Self {
        ReduceOptions { law_tol: 1e-9, bnb_node_budget: 20_000, max_cells: 20_000 }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MergeReport {
    pub regions_before: usize,
    pub regions_after: usize,
    /// number of distinct laws (colours)
    pub colours: usize,
    pub planes: usize,
    /// cells of the hyperplane arrangement (Algorithm 3)
    pub cells: usize,
    /// branch and bound finished within budget for every colour (the result is optimal for the arrangement)
    pub exact: bool,
    pub seconds: f64,
    /// true if the cell limit was hit and the input was returned unchanged
    pub skipped: bool,
    /// the arrangement exceeded `max_cells`; regions were merged greedily pair by pair (paper Sec. 9.2)
    pub greedy: bool,
}

struct NoObjective(usize);
impl crate::solution::Objective for NoObjective {
    fn evaluate_objective(&self, _x: &[f64], _theta: &[f64]) -> f64 {
        f64::NAN
    }
    fn num_t(&self) -> usize {
        self.0
    }
}

fn make_region(a: Mat, b: Vec<f64>, e: Mat, f: Vec<f64>) -> CriticalRegion {
    let n = e.cols;
    CriticalRegion {
        a,
        b,
        c: Mat::zeros(0, n),
        d: vec![],
        e,
        f,
        active_set: vec![],
        omega_set: vec![],
        lambda_set: vec![],
        regular_set: (vec![], vec![]),
        y_fixation: None,
        y_indices: None,
        x_indices: None,
        cheb_radius: None,
    }
}

fn law_of(r: &CriticalRegion, idx: &[usize]) -> (Vec<f64>, Vec<f64>) {
    let mut a = Vec::with_capacity(idx.len() * r.a.cols);
    let mut b = Vec::with_capacity(idx.len());
    for &i in idx {
        a.extend_from_slice(r.a.row(i));
        b.push(r.b[i]);
    }
    (a, b)
}

fn same_law(l1: &(Vec<f64>, Vec<f64>), l2: &(Vec<f64>, Vec<f64>), tol: f64) -> bool {
    let close = |x: &[f64], y: &[f64]| x.iter().zip(y).all(|(p, q)| (p - q).abs() <= tol * (1.0 + p.abs().max(q.abs())));
    close(&l1.0, &l2.0) && close(&l1.1, &l2.1)
}

// ------------------------------------------------------------------------------------------------
// Small geometry helpers
// ------------------------------------------------------------------------------------------------

/// Polytope {x : A x <= b} with unit-norm rows.
#[derive(Clone, Debug)]
struct Poly {
    n: usize,
    a: Vec<f64>,
    b: Vec<f64>,
}

impl Poly {
    fn new(n: usize) -> Self {
        Poly { n, a: vec![], b: vec![] }
    }
    fn push(&mut self, row: &[f64], rhs: f64) {
        let nr = norm(row);
        if nr < 1e-12 {
            return;
        }
        self.a.extend(row.iter().map(|v| v / nr));
        self.b.push(rhs / nr);
    }
    fn with(&self, row: &[f64], rhs: f64) -> Poly {
        let mut p = self.clone();
        p.push(row, rhs);
        p
    }
    fn mat(&self) -> Mat {
        Mat::from_vec(self.b.len(), self.n, self.a.clone())
    }
    fn radius(&self) -> f64 {
        match chebyshev_ball(&self.mat(), &self.b, &[]) {
            Some(s) => s.x[self.n],
            None => -1.0,
        }
    }
    fn intersect(&self, o: &Poly) -> Poly {
        let mut p = self.clone();
        p.a.extend_from_slice(&o.a);
        p.b.extend_from_slice(&o.b);
        p
    }
    /// Drops redundant rows (one LP per row, rows removed one after the other).
    fn prune(&self) -> Poly {
        let n = self.n;
        let m = self.b.len();
        let mut keep = vec![true; m];
        for k in 0..m {
            let mut a = Mat::zeros(0, n);
            let mut b = vec![];
            for j in 0..m {
                if j != k && keep[j] {
                    a.push_row(&self.a[j * n..(j + 1) * n]);
                    b.push(self.b[j]);
                }
            }
            if b.is_empty() {
                continue;
            }
            let row = &self.a[k * n..(k + 1) * n];
            let c: Vec<f64> = row.iter().map(|v| -v).collect();
            if let Some(x) = crate::solver::lp::lp_point(Some(&c), &a, &b, &[]) {
                if dot(row, &x) <= self.b[k] + 1e-9 {
                    keep[k] = false;
                }
            }
        }
        let mut p = Poly::new(n);
        for k in 0..m {
            if keep[k] {
                p.a.extend_from_slice(&self.a[k * n..(k + 1) * n]);
                p.b.push(self.b[k]);
            }
        }
        p
    }
    /// Bounding box by 2n LPs (infinite sides if unbounded).
    fn bbox(&self) -> (Vec<f64>, Vec<f64>) {
        let n = self.n;
        let a = self.mat();
        let mut lo = vec![f64::NEG_INFINITY; n];
        let mut hi = vec![f64::INFINITY; n];
        for k in 0..n {
            let mut c = vec![0.0; n];
            c[k] = 1.0;
            if let Some(x) = crate::solver::lp::lp_point(Some(&c), &a, &self.b, &[]) {
                lo[k] = x[k];
            }
            c[k] = -1.0;
            if let Some(x) = crate::solver::lp::lp_point(Some(&c), &a, &self.b, &[]) {
                hi[k] = x[k];
            }
        }
        (lo, hi)
    }
}

/// Rows of {A x <= b} that are facets of the polytope with the given vertices (duplicates of one
/// hyperplane keep one row).
fn facet_rows(n: usize, a: &[f64], b: &[f64], verts: &[Vec<f64>]) -> Vec<usize> {
    let mut keep: Vec<usize> = vec![];
    for k in 0..b.len() {
        let row = &a[k * n..(k + 1) * n];
        let tight: Vec<&Vec<f64>> = verts.iter().filter(|v| (dot(row, v) - b[k]).abs() <= 1e-7).collect();
        if tight.is_empty() {
            continue;
        }
        if affine_dim(&tight) + 1 < n {
            continue;
        }
        // duplicate hyperplane?
        let dup = keep.iter().any(|&j| {
            let rj = &a[j * n..(j + 1) * n];
            rj.iter().zip(row).all(|(p, q)| (p - q).abs() <= 1e-9) && (b[j] - b[k]).abs() <= 1e-9
        });
        if !dup {
            keep.push(k);
        }
    }
    keep
}

/// Dimension of the affine hull of the points.
fn affine_dim(pts: &[&Vec<f64>]) -> usize {
    if pts.len() <= 1 {
        return 0;
    }
    let n = pts[0].len();
    let p0 = pts[0];
    let mut basis: Vec<Vec<f64>> = vec![];
    for p in &pts[1..] {
        let mut v: Vec<f64> = p.iter().zip(p0.iter()).map(|(x, y)| x - y).collect();
        for bv in &basis {
            let c = dot(&v, bv);
            for (x, y) in v.iter_mut().zip(bv) {
                *x -= c * y;
            }
        }
        let nv = norm(&v);
        if nv > 1e-9 {
            for x in v.iter_mut() {
                *x /= nv;
            }
            basis.push(v);
            if basis.len() == n {
                break;
            }
        }
    }
    basis.len()
}

/// Rank of the given rows of `planes` is at least `need`.
fn rank_ge(planes: &Planes, ids: &[u32], need: usize) -> bool {
    if need == 0 {
        return true;
    }
    if ids.len() < need {
        return false;
    }
    let n = planes.n;
    let mut basis: Vec<Vec<f64>> = vec![];
    for &id in ids {
        let mut v = planes.a[id as usize * n..(id as usize + 1) * n].to_vec();
        for bv in &basis {
            let c = dot(&v, bv);
            for (x, y) in v.iter_mut().zip(bv) {
                *x -= c * y;
            }
        }
        let nv = norm(&v);
        if nv > 1e-9 {
            for x in v.iter_mut() {
                *x /= nv;
            }
            basis.push(v);
            if basis.len() >= need {
                return true;
            }
        }
    }
    false
}

// ------------------------------------------------------------------------------------------------
// Hyperplane arrangement and cells (Algorithm 3)
// ------------------------------------------------------------------------------------------------

struct Planes {
    n: usize,
    /// unit normals, canonical orientation (first non-zero entry positive)
    a: Vec<f64>,
    b: Vec<f64>,
}

impl Planes {
    fn len(&self) -> usize {
        self.b.len()
    }
    #[inline]
    fn side(&self, p: usize, x: &[f64]) -> f64 {
        dot(&self.a[p * self.n..(p + 1) * self.n], x) - self.b[p]
    }
}

/// Distinct hyperplanes of all rows; returns the planes and, per input row, (plane id, sign).
fn build_planes(n: usize, rows: &[(Vec<f64>, f64)]) -> (Planes, Vec<(usize, f64)>) {
    let total = rows.len();
    let mut canon: Vec<(Vec<f64>, f64)> = Vec::with_capacity(total);
    for (a, f) in rows {
        let mut v = a.clone();
        v.push(*f);
        let lead = v[..n].iter().find(|x| x.abs() > 1e-9).cloned().unwrap_or(1.0);
        let s = if lead < 0.0 { -1.0 } else { 1.0 };
        for x in v.iter_mut() {
            *x *= s;
        }
        canon.push((v, s));
    }
    let mut order: Vec<usize> = (0..total).collect();
    order.sort_by(|&x, &y| canon[x].0[0].partial_cmp(&canon[y].0[0]).unwrap());
    let mut parent: Vec<usize> = (0..total).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for (ii, &i) in order.iter().enumerate() {
        for &j in &order[ii + 1..] {
            if canon[j].0[0] - canon[i].0[0] > 1e-7 {
                break;
            }
            let close = canon[i].0.iter().zip(&canon[j].0).all(|(a, b)| (a - b).abs() <= 1e-7 * (1.0 + a.abs()));
            if close {
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[ri] = rj;
                }
            }
        }
    }
    let mut id_of_root: HashMap<usize, usize> = HashMap::new();
    let mut planes = Planes { n, a: vec![], b: vec![] };
    let mut row_plane = Vec::with_capacity(total);
    for k in 0..total {
        let root = find(&mut parent, k);
        let id = *id_of_root.entry(root).or_insert_with(|| {
            planes.a.extend_from_slice(&canon[root].0[..n]);
            planes.b.push(canon[root].0[n]);
            planes.b.len() - 1
        });
        row_plane.push((id, canon[k].1));
    }
    (planes, row_plane)
}

#[derive(Clone)]
struct Vert {
    x: Vec<f64>,
    /// ids of the planes through the vertex (sorted)
    tight: Vec<u32>,
}

fn intersect_sorted(a: &[u32], b: &[u32]) -> Vec<u32> {
    let (mut i, mut j) = (0, 0);
    let mut out = vec![];
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
        }
    }
    out
}

fn insert_sorted(v: &mut Vec<u32>, p: u32) {
    if let Err(pos) = v.binary_search(&p) {
        v.insert(pos, p);
    }
}

/// Splits a convex vertex set by plane `p` into the part with a'x <= b and the part with a'x >= b.
fn split_verts(vs: &[Vert], p: usize, planes: &Planes) -> (Vec<Vert>, Vec<Vert>) {
    let s: Vec<f64> = vs.iter().map(|v| planes.side(p, &v.x)).collect();
    let mut neg = vec![];
    let mut pos = vec![];
    for (v, &sv) in vs.iter().zip(&s) {
        if sv < -SIDE_TOL {
            neg.push(v.clone());
        } else if sv > SIDE_TOL {
            pos.push(v.clone());
        } else {
            let mut w = v.clone();
            insert_sorted(&mut w.tight, p as u32);
            neg.push(w.clone());
            pos.push(w);
        }
    }
    let n = planes.n;
    for i in 0..vs.len() {
        if s[i] >= -SIDE_TOL {
            continue;
        }
        for j in 0..vs.len() {
            if s[j] <= SIDE_TOL {
                continue;
            }
            let common = intersect_sorted(&vs[i].tight, &vs[j].tight);
            if !rank_ge(planes, &common, n.saturating_sub(1)) {
                continue;
            }
            let t = s[i] / (s[i] - s[j]);
            let x: Vec<f64> = vs[i].x.iter().zip(&vs[j].x).map(|(a, b)| a + t * (b - a)).collect();
            let mut tight = common;
            insert_sorted(&mut tight, p as u32);
            let w = Vert { x, tight };
            if !neg.iter().any(|q| q.x.iter().zip(&w.x).all(|(a, b)| (a - b).abs() <= 1e-9)) {
                neg.push(w.clone());
                pos.push(w);
            }
        }
    }
    (neg, pos)
}

fn cuts(vs: &[Vert], p: usize, planes: &Planes) -> bool {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for v in vs {
        let s = planes.side(p, &v.x);
        lo = lo.min(s);
        hi = hi.max(s);
    }
    lo < -SIDE_TOL && hi > SIDE_TOL
}

fn split_region(verts: Vec<Vert>, cands: Vec<u32>, planes: &Planes, count: &std::sync::atomic::AtomicUsize, cap: usize) -> Option<Vec<Vec<Vert>>> {
    use std::sync::atomic::Ordering;
    let mut out = vec![];
    let mut stack = vec![(verts, cands)];
    while let Some((vs, cs)) = stack.pop() {
        match cs.iter().position(|&p| cuts(&vs, p as usize, planes)) {
            None => {
                if count.fetch_add(1, Ordering::Relaxed) > cap {
                    return None;
                }
                out.push(vs)
            }
            Some(pos) => {
                if count.load(Ordering::Relaxed) > cap {
                    return None;
                }
                let p = cs[pos] as usize;
                let rem: Vec<u32> = cs[pos + 1..].to_vec();
                let (neg, posv) = split_verts(&vs, p, planes);
                stack.push((neg, rem.clone()));
                stack.push((posv, rem));
            }
        }
    }
    Some(out)
}

struct Cells {
    words: usize,
    colour: Vec<u32>,
    verts: Vec<Vec<Vert>>,
    /// marking bits, `words` u64 per cell; bit p = 1 if the cell lies on the a'x >= b side of plane p
    mark: Vec<u64>,
}

impl Cells {
    fn mark(&self, c: u32) -> &[u64] {
        &self.mark[c as usize * self.words..(c as usize + 1) * self.words]
    }
}

// ------------------------------------------------------------------------------------------------
// Algorithm 1: branch and bound on markings
// ------------------------------------------------------------------------------------------------

struct Group {
    cells: Vec<u32>,
}

struct Bnb<'a> {
    cells: &'a Cells,
    budget: usize,
    nodes: StdCell<usize>,
    exact: StdCell<bool>,
}

/// Envelope of a set of cells: (and, or) of the markings. Plane p is a star where or != and.
fn envelope(cells: &Cells, w: &[u32]) -> (Vec<u64>, Vec<u64>) {
    let mut and = vec![!0u64; cells.words];
    let mut or = vec![0u64; cells.words];
    for &c in w {
        for (k, &m) in cells.mark(c).iter().enumerate() {
            and[k] &= m;
            or[k] |= m;
        }
    }
    (and, or)
}

fn inside_envelope(m: &[u64], and: &[u64], or: &[u64]) -> bool {
    m.iter().zip(and).zip(or).all(|((&m, &a), &o)| {
        let fixed = !(o ^ a);
        (m ^ a) & fixed == 0
    })
}

#[inline]
fn bit(m: &[u64], p: usize) -> bool {
    (m[p / 64] >> (p % 64)) & 1 == 1
}

impl<'a> Bnb<'a> {
    fn components(&self, w: &[u32]) -> Vec<Vec<u32>> {
        let n = w.len();
        if n < 2 || n > 4000 {
            return vec![w.to_vec()];
        }
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        for i in 0..n {
            let mi = self.cells.mark(w[i]);
            for j in i + 1..n {
                let mj = self.cells.mark(w[j]);
                let mut diff = 0u32;
                for (a, b) in mi.iter().zip(mj) {
                    diff += (a ^ b).count_ones();
                    if diff > 1 {
                        break;
                    }
                }
                if diff == 1 {
                    let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                    if ri != rj {
                        parent[ri] = rj;
                    }
                }
            }
        }
        let mut groups: HashMap<usize, Vec<u32>> = HashMap::new();
        for i in 0..n {
            let r = find(&mut parent, i);
            groups.entry(r).or_default().push(w[i]);
        }
        let mut v: Vec<Vec<u32>> = groups.into_values().collect();
        v.sort_by_key(|g| g[0]);
        v
    }

    /// Greedy merging (Sec. 9.2 style): grow a cluster with the cell whose addition keeps the
    /// envelope free of black cells; the cluster becomes all unassigned white cells in the envelope.
    fn greedy(&self, w: &[u32], b: &[u32]) -> Vec<Group> {
        let mut unassigned: Vec<u32> = w.to_vec();
        let mut out = vec![];
        while let Some(seed) = unassigned.pop() {
            let mut cluster = vec![seed];
            loop {
                let mut grown = false;
                for k in 0..unassigned.len() {
                    let cand = unassigned[k];
                    let mut test = cluster.clone();
                    test.push(cand);
                    let (and, or) = envelope(self.cells, &test);
                    if b.iter().any(|&bc| inside_envelope(self.cells.mark(bc), &and, &or)) {
                        continue;
                    }
                    let mut newc = vec![];
                    let mut rest = vec![];
                    for &u in &unassigned {
                        if inside_envelope(self.cells.mark(u), &and, &or) {
                            newc.push(u);
                        } else {
                            rest.push(u);
                        }
                    }
                    for &c in &cluster {
                        newc.push(c);
                    }
                    cluster = newc;
                    unassigned = rest;
                    grown = true;
                    break;
                }
                if !grown {
                    break;
                }
            }
            out.push(Group { cells: cluster });
        }
        out
    }

    /// Minimal number of convex groups covering the white cells without covering a black cell
    /// (at most `limit` groups, otherwise None).
    fn mrg(&self, w: Vec<u32>, b: &[u32], limit: usize, check_conn: bool) -> Option<Vec<Group>> {
        if w.is_empty() {
            return Some(vec![]);
        }
        if limit == 0 {
            return None;
        }
        let nodes = self.nodes.get() + 1;
        self.nodes.set(nodes);
        if nodes > self.budget {
            self.exact.set(false);
            let g = self.greedy(&w, b);
            return if g.len() <= limit { Some(g) } else { None };
        }
        let (and, or) = envelope(self.cells, &w);
        let bi: Vec<u32> = b.iter().copied().filter(|&c| inside_envelope(self.cells.mark(c), &and, &or)).collect();
        if bi.is_empty() {
            return Some(vec![Group { cells: w }]);
        }
        if check_conn {
            let comps = self.components(&w);
            if comps.len() > 1 {
                let mut res: Vec<Group> = vec![];
                let nc = comps.len();
                for (ci, comp) in comps.into_iter().enumerate() {
                    let remaining = nc - ci - 1;
                    if res.len() + remaining + 1 > limit {
                        return None;
                    }
                    let lim = limit - res.len() - remaining;
                    let part = self.mrg(comp, &bi, lim, false)?;
                    res.extend(part);
                }
                return Some(res);
            }
        }
        // stars with branching priorities (Sec. 5.2)
        let mut cand: Vec<(u8, i64, usize, Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>)> = vec![];
        for k in 0..and.len() {
            let mut star = or[k] & !and[k];
            while star != 0 {
                let t = star.trailing_zeros() as usize;
                star &= star - 1;
                let p = k * 64 + t;
                let (mut wn, mut wp, mut bn, mut bp) = (vec![], vec![], vec![], vec![]);
                for &c in &w {
                    if bit(self.cells.mark(c), p) {
                        wp.push(c)
                    } else {
                        wn.push(c)
                    }
                }
                for &c in &bi {
                    if bit(self.cells.mark(c), p) {
                        bp.push(c)
                    } else {
                        bn.push(c)
                    }
                }
                let pure = if bn.is_empty() {
                    wn.len() as i64
                } else if bp.is_empty() {
                    wp.len() as i64
                } else {
                    0
                };
                let prio = if pure > 0 { 0 } else { 1 };
                cand.push((prio, -pure, p, wn, wp, bn, bp));
            }
        }
        cand.sort_by(|x, y| (x.0, x.1, x.2).cmp(&(y.0, y.1, y.2)));
        let mut best: Option<Vec<Group>> = None;
        let mut lim = limit;
        for (_, _, _, wn, wp, bn, bp) in cand {
            if lim < 2 {
                break;
            }
            let lb_pos = if wp.is_empty() { 0 } else { 1 };
            let Some(m1) = self.mrg(wn, &bn, lim - lb_pos, true) else { continue };
            if m1.len() + lb_pos > lim {
                continue;
            }
            let Some(m2) = self.mrg(wp, &bp, lim - m1.len(), true) else { continue };
            let total = m1.len() + m2.len();
            if total <= lim {
                let mut all = m1;
                all.extend(m2);
                best = Some(all);
                lim = total.saturating_sub(1);
            }
        }
        best
    }
}

/// Convex polytope (rows) of a merged group: all hyperplanes that keep every cell on one side,
/// reduced to facets.
fn group_polytope(cells: &Cells, planes: &Planes, g: &Group) -> (Mat, Vec<f64>) {
    let n = planes.n;
    let (and, or) = envelope(cells, &g.cells);
    let mut verts: Vec<Vec<f64>> = vec![];
    for &c in &g.cells {
        for v in &cells.verts[c as usize] {
            if !verts.iter().any(|q| q.iter().zip(&v.x).all(|(a, b)| (a - b).abs() <= 1e-10)) {
                verts.push(v.x.clone());
            }
        }
    }
    let mut a = vec![];
    let mut b = vec![];
    for p in 0..planes.len() {
        if bit(&or, p) != bit(&and, p) {
            continue;
        }
        let pa = &planes.a[p * n..(p + 1) * n];
        // keep only planes that touch the polytope
        if bit(&and, p) {
            a.extend(pa.iter().map(|v| -v));
            b.push(-planes.b[p]);
        } else {
            a.extend_from_slice(pa);
            b.push(planes.b[p]);
        }
    }
    let keep = facet_rows(n, &a, &b, &verts);
    let mut e = Mat::zeros(0, n);
    let mut f = vec![];
    for k in keep {
        e.push_row(&a[k * n..(k + 1) * n]);
        f.push(b[k]);
    }
    (e, f)
}

/// Optimal merging of regions with the same law on the target components (Geyer et al. 2008,
/// Algorithms 3 and 1). The result is a [`Solution`] of disjoint convex regions covering the same
/// set, each carrying the shared law of the target components (`a` has one row per target
/// component, so `evaluate` returns the target components only). Active sets and multipliers are
/// not carried.
pub fn merge_regions(sol: &Solution, target: &ReduceTarget, opts: &ReduceOptions) -> (Solution, MergeReport) {
    let t0 = std::time::Instant::now();
    let n = sol.theta_dim();
    let r = sol.len();
    let mut rep = MergeReport { regions_before: r, exact: true, ..Default::default() };
    let nx = sol.critical_regions.first().map(|c| c.a.rows).unwrap_or(0);
    let idx = target.indices(nx);

    let unreduced = |rep: &mut MergeReport, t0: std::time::Instant, skipped: bool| -> (Solution, MergeReport) {
        let regs: Vec<CriticalRegion> = sol
            .critical_regions
            .iter()
            .map(|c| {
                let (a, b) = law_of(c, &idx);
                make_region(Mat::from_vec(idx.len(), n, a), b, c.e.clone(), c.f.clone())
            })
            .collect();
        rep.regions_after = regs.len();
        rep.skipped = skipped;
        rep.seconds = t0.elapsed().as_secs_f64();
        (Solution { critical_regions: regs, is_overlapping: false, ..sol.clone() }, rep.clone())
    };
    if r == 0 {
        return unreduced(&mut rep, t0, false);
    }

    // colours
    let laws: Vec<(Vec<f64>, Vec<f64>)> = sol.critical_regions.iter().map(|c| law_of(c, &idx)).collect();
    let mut reps: Vec<usize> = vec![];
    let mut colour_of = vec![0u32; r];
    for i in 0..r {
        match reps.iter().position(|&j| same_law(&laws[i], &laws[j], opts.law_tol)) {
            Some(c) => colour_of[i] = c as u32,
            None => {
                colour_of[i] = reps.len() as u32;
                reps.push(i);
            }
        }
    }
    rep.colours = reps.len();

    // hyperplane arrangement
    let mut rows: Vec<(Vec<f64>, f64)> = vec![];
    let mut row_start = vec![0usize];
    for c in &sol.critical_regions {
        for i in 0..c.e.rows {
            let nr = norm(c.e.row(i));
            rows.push((c.e.row(i).iter().map(|v| v / nr).collect(), c.f[i] / nr));
        }
        row_start.push(rows.len());
    }
    let (planes, row_plane) = build_planes(n, &rows);
    rep.planes = planes.len();
    let dbg = std::env::var("RED_DEBUG").is_ok();
    if dbg {
        eprintln!("planes {} at {:.3}s", planes.len(), t0.elapsed().as_secs_f64());
    }

    // cells: cut each region by the hyperplanes that cut its interior
    let counter = std::sync::atomic::AtomicUsize::new(0);
    let per_region: Vec<Option<Vec<Vec<Vert>>>> = (0..r)
        .into_par_iter()
        .map(|i| {
            let (s, t) = (row_start[i], row_start[i + 1]);
            let mut a = Mat::zeros(0, n);
            let mut b = vec![];
            for k in s..t {
                a.push_row(&rows[k].0);
                b.push(rows[k].1);
            }
            let vx = enumerate_vertices(&a, &b);
            if vx.is_empty() {
                return None;
            }
            let verts: Vec<Vert> = vx
                .into_iter()
                .map(|x| {
                    let mut tight: Vec<u32> = (s..t).filter(|&k| (dot(&rows[k].0, &x) - rows[k].1).abs() <= 1e-8).map(|k| row_plane[k].0 as u32).collect();
                    tight.sort();
                    tight.dedup();
                    Vert { x, tight }
                })
                .collect();
            let cands: Vec<u32> = (0..planes.len() as u32).filter(|&p| cuts(&verts, p as usize, &planes)).collect();
            split_region(verts, cands, &planes, &counter, opts.max_cells)
        })
        .collect();
    if per_region.iter().any(|x| x.is_none()) || per_region.iter().map(|x| x.as_ref().unwrap().len()).sum::<usize>() > opts.max_cells {
        // the arrangement is too large (Sec. 9.3 of the paper): greedy pairwise merging of regions
        let t1 = std::time::Instant::now();
        let out = greedy_merge(sol, &idx, &colour_of, &reps, &laws);
        rep.regions_after = out.len();
        rep.exact = false;
        rep.greedy = true;
        rep.seconds = t0.elapsed().as_secs_f64();
        if dbg {
            eprintln!("greedy merge {:.3}s", t1.elapsed().as_secs_f64());
        }
        return (Solution { critical_regions: out, is_overlapping: false, ..sol.clone() }, rep);
    }
    let words = planes.len().div_ceil(64).max(1);
    let mut cells = Cells { words, colour: vec![], verts: vec![], mark: vec![] };
    for (i, pr) in per_region.into_iter().enumerate() {
        for vs in pr.unwrap() {
            let mut cen = vec![0.0; n];
            for v in &vs {
                for k in 0..n {
                    cen[k] += v.x[k] / vs.len() as f64;
                }
            }
            let mut m = vec![0u64; words];
            for p in 0..planes.len() {
                if planes.side(p, &cen) > 0.0 {
                    m[p / 64] |= 1 << (p % 64);
                }
            }
            cells.mark.extend(m);
            cells.colour.push(colour_of[i]);
            cells.verts.push(vs);
        }
    }
    let nc = cells.colour.len();
    rep.cells = nc;
    if dbg {
        eprintln!("cells {} at {:.3}s", nc, t0.elapsed().as_secs_f64());
    }

    // Algorithm 1 per colour
    let results: Vec<(usize, Vec<Group>, bool)> = (0..reps.len())
        .into_par_iter()
        .map(|col| {
            let w: Vec<u32> = (0..nc as u32).filter(|&c| cells.colour[c as usize] == col as u32).collect();
            let bl: Vec<u32> = (0..nc as u32).filter(|&c| cells.colour[c as usize] != col as u32).collect();
            let bnb = Bnb { cells: &cells, budget: opts.bnb_node_budget, nodes: StdCell::new(0), exact: StdCell::new(true) };
            let lim = w.len();
            let groups = bnb.mrg(w.clone(), &bl, lim, true).unwrap_or_else(|| w.iter().map(|&c| Group { cells: vec![c] }).collect());
            (col, groups, bnb.exact.get())
        })
        .collect();
    rep.exact = results.iter().all(|x| x.2);
    if dbg {
        eprintln!("bnb done at {:.3}s", t0.elapsed().as_secs_f64());
    }

    let mut flat: Vec<(usize, &Group)> = vec![];
    for (col, gs, _) in &results {
        for g in gs {
            flat.push((*col, g));
        }
    }
    let regs: Vec<CriticalRegion> = flat
        .par_iter()
        .map(|&(col, g)| {
            let (e, f) = group_polytope(&cells, &planes, g);
            let (a, b) = &laws[reps[col]];
            make_region(Mat::from_vec(idx.len(), n, a.clone()), b.clone(), e, f)
        })
        .collect();
    rep.regions_after = regs.len();
    rep.seconds = t0.elapsed().as_secs_f64();
    (Solution { critical_regions: regs, is_overlapping: false, ..sol.clone() }, rep)
}

/// Greedy merging of regions (Sec. 9.2 of Geyer et al.): adjacent regions of the same colour are
/// replaced by their union whenever it is convex (Bemporad et al. 2001, Theorem 3: the union is
/// convex iff it equals its envelope, i.e. envelope minus union is empty). No hyperplane
/// arrangement, so no optimality; the number of regions never grows.
fn greedy_merge(sol: &Solution, idx: &[usize], colour_of: &[u32], reps: &[usize], laws: &[(Vec<f64>, Vec<f64>)]) -> Vec<CriticalRegion> {
    let geom = PartitionGeometry::new(sol);
    let n = geom.dim;
    let r = geom.num_regions;
    struct G {
        poly: Poly,
        verts: Vec<Vec<f64>>,
        colour: u32,
        ver: u32,
        alive: bool,
    }
    let mut groups: Vec<G> = (0..r)
        .map(|i| {
            let (e, f) = geom.region_rows(i);
            let mut p = Poly::new(n);
            for (k, fk) in f.iter().enumerate() {
                p.push(&e[k * n..(k + 1) * n], *fk);
            }
            G { poly: p, verts: geom.vertices[i].clone(), colour: colour_of[i], ver: 0, alive: true }
        })
        .collect();
    let mut gid: Vec<usize> = (0..r).collect();
    let mut pairs: Vec<(usize, usize)> = geom.adjacency_list().into_iter().filter(|&(i, j, _)| colour_of[i] == colour_of[j]).map(|(i, j, _)| (i, j)).collect();
    pairs.sort();
    pairs.dedup();
    fn find(g: &mut [usize], mut x: usize) -> usize {
        while g[x] != x {
            g[x] = g[g[x]];
            x = g[x];
        }
        x
    }
    let mut failed: std::collections::HashSet<(usize, usize, u32, u32)> = std::collections::HashSet::new();
    loop {
        let mut merged_any = false;
        for &(i, j) in &pairs {
            let (a, b) = (find(&mut gid, i), find(&mut gid, j));
            if a == b {
                continue;
            }
            let (lo, hi) = (a.min(b), a.max(b));
            let key = (lo, hi, groups[lo].ver, groups[hi].ver);
            if failed.contains(&key) {
                continue;
            }
            let mut verts = groups[a].verts.clone();
            verts.extend(groups[b].verts.iter().cloned());
            // envelope: rows of both groups that every vertex satisfies
            let mut env = Poly::new(n);
            for g in [&groups[a], &groups[b]] {
                for k in 0..g.poly.b.len() {
                    let row = &g.poly.a[k * n..(k + 1) * n];
                    if verts.iter().all(|v| dot(row, v) - g.poly.b[k] <= 1e-7) {
                        env.push(row, g.poly.b[k]);
                    }
                }
            }
            // bounded for the LPs: a box around the whole partition
            let mut boxed = env.clone();
            for k in 0..n {
                let mut row = vec![0.0; n];
                row[k] = 1.0;
                boxed.push(&row, geom.global_hi[k] + 1.0);
                row[k] = -1.0;
                boxed.push(&row, -(geom.global_lo[k] - 1.0));
            }
            // the union is convex iff envelope minus (group a, group b) is empty
            let mut pieces = vec![boxed];
            for g in [&groups[a], &groups[b]] {
                let mut next = vec![];
                for p in pieces {
                    if p.intersect(&g.poly).radius() > RAD_EPS {
                        next.extend(poly_diff(&p, &g.poly));
                    } else {
                        next.push(p);
                    }
                }
                pieces = next;
                if pieces.is_empty() {
                    break;
                }
            }
            if pieces.is_empty() {
                let keep = facet_rows(n, &env.a, &env.b, &verts);
                let mut p = Poly::new(n);
                for k in keep {
                    p.push(&env.a[k * n..(k + 1) * n], env.b[k]);
                }
                let mut uniq: Vec<Vec<f64>> = vec![];
                for v in verts {
                    if !uniq.iter().any(|u| u.iter().zip(&v).all(|(x, y)| (x - y).abs() <= 1e-10)) {
                        uniq.push(v);
                    }
                }
                groups[lo].poly = p;
                groups[lo].verts = uniq;
                groups[lo].ver += 1;
                groups[hi].alive = false;
                gid[hi] = lo;
                merged_any = true;
            } else {
                failed.insert(key);
            }
        }
        if !merged_any {
            break;
        }
    }
    let nx_idx = idx.len();
    groups
        .iter()
        .filter(|g| g.alive)
        .map(|g| {
            let (a, b) = &laws[reps[g.colour as usize]];
            let mut e = Mat::zeros(0, n);
            for k in 0..g.poly.b.len() {
                e.push_row(&g.poly.a[k * n..(k + 1) * n]);
            }
            make_region(Mat::from_vec(nx_idx, n, a.clone()), b.clone(), e, g.poly.b.clone())
        })
        .collect()
}

// ------------------------------------------------------------------------------------------------
// Clipping (Kvasnica and Fikar 2012)
// ------------------------------------------------------------------------------------------------

/// P \ Q as a list of disjoint polytopes (Q cut along its rows).
fn poly_diff(p: &Poly, q: &Poly) -> Vec<Poly> {
    let n = p.n;
    let mut out = vec![];
    let mut cur = p.clone();
    for j in 0..q.b.len() {
        let row = &q.a[j * n..(j + 1) * n];
        let neg: Vec<f64> = row.iter().map(|v| -v).collect();
        let piece = cur.with(&neg, -q.b[j]);
        if piece.radius() > RAD_EPS {
            out.push(piece);
            cur = cur.with(row, q.b[j]);
            if cur.radius() <= RAD_EPS {
                return out;
            }
        }
    }
    out
}

fn boxes_meet(a: &(Vec<f64>, Vec<f64>), b: &(Vec<f64>, Vec<f64>)) -> bool {
    (0..a.0.len()).all(|k| a.0[k] <= b.1[k] + 1e-9 && b.0[k] <= a.1[k] + 1e-9)
}

#[derive(Clone)]
pub struct ClipComponent {
    /// component of x
    pub index: usize,
    pub lo: Option<f64>,
    pub hi: Option<f64>,
    /// the augmented partition: one affine law (1 x n_theta) per region
    pub solution: Solution,
    pub original_regions: usize,
    pub saturated: usize,
    pub unsaturated: usize,
    /// Step 11 returned the original function (the augmentation was not smaller)
    pub fell_back: bool,
}

#[derive(Clone)]
pub struct ClippedLaw {
    pub components: Vec<ClipComponent>,
    pub seconds: f64,
}

impl ClippedLaw {
    /// Total number of regions over all components.
    pub fn num_regions(&self) -> usize {
        self.components.iter().map(|c| c.solution.len()).sum()
    }

    pub fn clip(&self, comp: usize, v: f64) -> f64 {
        let c = &self.components[comp];
        let v = match c.hi {
            Some(h) => v.min(h),
            None => v,
        };
        match c.lo {
            Some(l) => v.max(l),
            None => v,
        }
    }

    /// phi(law(theta)) of component `comp` given the region index found in its partition.
    pub fn value_in(&self, comp: usize, region: usize, theta: &[f64]) -> f64 {
        let r = &self.components[comp].solution.critical_regions[region];
        self.clip(comp, dot(r.a.row(0), theta) + r.b[0])
    }

    /// Clipped target components at theta (first containing region of each component partition,
    /// tolerance 1e-9), `None` if theta lies outside the feasible set.
    pub fn evaluate(&self, theta: &[f64]) -> Option<Vec<f64>> {
        let mut out = Vec::with_capacity(self.components.len());
        for (k, c) in self.components.iter().enumerate() {
            let i = c.solution.critical_regions.iter().position(|r| r.is_inside(theta, 1e-9))?;
            out.push(self.value_in(k, i, theta));
        }
        Some(out)
    }
}

fn single_component_solution(sol: &Solution, regs: Vec<(Vec<f64>, f64, Poly)>) -> Solution {
    let n = sol.theta_dim();
    let regions: Vec<CriticalRegion> = regs
        .into_iter()
        .map(|(a, b, p)| {
            let p = p.prune();
            let (e, f) = (p.mat(), p.b.clone());
            make_region(Mat::from_vec(1, n, a), vec![b], e, f)
        })
        .collect();
    Solution { program: std::sync::Arc::new(NoObjective(n)), continuous: None, critical_regions: regions, is_overlapping: false, point_location_tolerance: sol.point_location_tolerance }
}

/// Clipping-based complexity reduction. `indices` are the components of x to reduce and `bounds`
/// their saturation bounds (lo, hi) (from the input constraints; a missing side never saturates).
/// Each component is processed on its own (Sec. IV-B).
pub fn clip_reduce(sol: &Solution, indices: &[usize], bounds: &[(Option<f64>, Option<f64>)]) -> ClippedLaw {
    let t0 = std::time::Instant::now();
    let geom = PartitionGeometry::new(sol);
    if std::env::var("RED_DEBUG").is_ok() {
        eprintln!("clip geometry {:.3}s", t0.elapsed().as_secs_f64());
    }
    let comps = indices.iter().zip(bounds).map(|(&i, &(lo, hi))| clip_component(sol, &geom, i, lo, hi)).collect();
    ClippedLaw { components: comps, seconds: t0.elapsed().as_secs_f64() }
}

fn clip_component(sol: &Solution, geom: &PartitionGeometry, j: usize, lo: Option<f64>, hi: Option<f64>) -> ClipComponent {
    let n = geom.dim;
    let r = geom.num_regions;
    let law: Vec<(Vec<f64>, f64)> = sol.critical_regions.iter().map(|c| (c.a.row(j).to_vec(), c.b[j])).collect();
    // saturation: 0 unsat, 1 at max, 2 at min
    let sat: Vec<u8> = law
        .iter()
        .map(|(a, b)| {
            if a.iter().any(|v| v.abs() > 1e-9) {
                0
            } else if hi.map(|h| (b - h).abs() <= 1e-7 * (1.0 + h.abs())).unwrap_or(false) {
                1
            } else if lo.map(|l| (b - l).abs() <= 1e-7 * (1.0 + l.abs())).unwrap_or(false) {
                2
            } else {
                0
            }
        })
        .collect();
    let n_sat = sat.iter().filter(|&&s| s != 0).count();
    let original: Vec<(Vec<f64>, f64, Poly)> = (0..r)
        .map(|i| {
            let (e, f) = geom.region_rows(i);
            let mut p = Poly::new(n);
            for (k, fk) in f.iter().enumerate() {
                p.push(&e[k * n..(k + 1) * n], *fk);
            }
            (law[i].0.clone(), law[i].1, p)
        })
        .collect();
    let mk = |fell_back: bool, regs: Vec<(Vec<f64>, f64, Poly)>| ClipComponent {
        index: j,
        lo,
        hi,
        solution: single_component_solution(sol, regs),
        original_regions: r,
        saturated: n_sat,
        unsaturated: r - n_sat,
        fell_back,
    };
    if n_sat == 0 || n_sat == r {
        return mk(true, original);
    }
    // Omega
    let mut omega = Poly::new(n);
    for p in &geom.domain_planes {
        omega.push(&p[..n], p[n]);
    }
    let unsat_ids: Vec<usize> = (0..r).filter(|&i| sat[i] == 0).collect();
    let boxes: Vec<(Vec<f64>, Vec<f64>)> = (0..r).map(|i| (geom.bbox_lo[i].clone(), geom.bbox_hi[i].clone())).collect();

    // extensions of unsaturated regions (Steps 2-8)
    let ext: Vec<Option<Vec<Poly>>> = unsat_ids
        .par_iter()
        .map(|&i| {
            // facets whose neighbour is saturated
            let drop: Vec<usize> = geom.facets[i].iter().filter(|f| f.neighbors.iter().any(|&nb| sat[nb] != 0)).map(|f| f.row).collect();
            if drop.is_empty() {
                return Some(vec![original[i].2.clone()]);
            }
            let (e, f) = geom.region_rows(i);
            let mut rt = omega.clone();
            for k in 0..f.len() {
                if !drop.contains(&k) {
                    rt.push(&e[k * n..(k + 1) * n], f[k]);
                }
            }
            // saturated regions weakly adjacent to i (breadth first through saturated regions)
            let mut weak = vec![false; r];
            let mut q: VecDeque<usize> = VecDeque::new();
            for fc in &geom.facets[i] {
                for &nb in &fc.neighbors {
                    if sat[nb] != 0 && !weak[nb] {
                        weak[nb] = true;
                        q.push_back(nb);
                    }
                }
            }
            while let Some(s) = q.pop_front() {
                for fc in &geom.facets[s] {
                    for &nb in &fc.neighbors {
                        if sat[nb] != 0 && !weak[nb] {
                            weak[nb] = true;
                            q.push_back(nb);
                        }
                    }
                }
            }
            let rt = rt.prune();
            let rb = rt.bbox();
            let mut pieces = vec![rt];
            for c in 0..r {
                if c == i || weak[c] || !boxes_meet(&rb, &boxes[c]) {
                    continue;
                }
                let q = &original[c].2;
                let mut next = vec![];
                for p in pieces {
                    if p.intersect(q).radius() > RAD_EPS {
                        next.extend(poly_diff(&p, q));
                    } else {
                        next.push(p);
                    }
                }
                pieces = next;
                if pieces.len() > r {
                    // Remark 4.7: the set difference blew up; Step 11 will return the original
                    return None;
                }
            }
            Some(pieces)
        })
        .collect();
    if ext.iter().any(|e| e.is_none()) {
        return mk(true, original);
    }
    let ext: Vec<Vec<Poly>> = ext.into_iter().map(|e| e.unwrap()).collect();

    let mut regs: Vec<(Vec<f64>, f64, Poly)> = vec![];
    for (k, &i) in unsat_ids.iter().enumerate() {
        for p in &ext[k] {
            regs.push((law[i].0.clone(), law[i].1, p.clone()));
        }
    }
    // Steps 9-10: saturated regions not fully covered are added back
    let piece_boxes: Vec<(Vec<f64>, Vec<f64>)> = regs
        .par_iter()
        .map(|(_, _, p)| p.bbox())
        .collect();
    let nocover: Vec<usize> = (0..r)
        .into_par_iter()
        .filter(|&s| sat[s] != 0)
        .filter(|&s| {
            let mut rem = vec![original[s].2.clone()];
            for (pi, (_, _, p)) in regs.iter().enumerate() {
                if rem.is_empty() {
                    break;
                }
                if !boxes_meet(&piece_boxes[pi], &boxes[s]) {
                    continue;
                }
                let mut next = vec![];
                for x in rem {
                    if x.intersect(p).radius() > RAD_EPS {
                        next.extend(poly_diff(&x, p));
                    } else {
                        next.push(x);
                    }
                }
                rem = next;
                if rem.len() > 64 {
                    break; // conservatively treated as not covered (the region is added back)
                }
            }
            !rem.is_empty()
        })
        .collect();
    for s in nocover {
        regs.push((law[s].0.clone(), law[s].1, original[s].2.clone()));
    }
    // Step 11
    if regs.len() > r {
        return mk(true, original);
    }
    mk(false, regs)
}
