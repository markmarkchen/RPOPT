//! Low-rank storage of an explicit mpQP solution.
//!
//! Nielsen and Axehill, "Reduced memory footprint in multiparametric quadratic programming by
//! exploiting low rank structure", CDC 2016 (DOI 10.1109/CDC.2016.7798819).
//!
//! Crossing a facet between neighbouring critical regions adds or removes one active constraint, and
//! then the parametric solution changes by a rank-one term (paper Theorems 1 and 2):
//!   x_j(theta)       = x_i(theta) + f (c + v'theta),
//!   lambda_j(theta)  = lambda_i(theta) - d (c + v'theta),
//! and the value of every hyperplane of the neighbour's description changes by a scalar times the
//! same factor (Corollaries 1 and 2). A storage tree stores the law and the hyperplane values in a
//! root densely and, in every other node, only the modification (Theorem 3, Algorithms 1 and 2); the
//! memory count is the paper's (30): hyperplane j of node i needs the scalar entries of all nodes on
//! the path from the root, so node i stores them for the hyperplanes of all its descendants (29).
//!
//! Implementation notes (deviations/generalisations of the paper):
//!  * the modification is obtained numerically as a truncated SVD of the stacked change of
//!    [x-law; multiplier-law] between parent and child, so it also covers parents that differ by
//!    more than one constraint (rank r > 1) and equal active sets (rank 0). The tree edges are
//!    active-set neighbours (one constraint added or removed) wherever possible; components
//!    without such an edge get their own dense root (Remark 8). A node whose modification is not
//!    smaller than its dense description is stored densely (it starts a new tree).
//!  * hyperplanes are matched to the three kinds of facets of a critical region (primal feasibility
//!    of an inactive constraint, multiplier nonnegativity of an active one, parameter-set row) from
//!    the region's metadata. A facet of any other origin (for example a cut of the recursive
//!    partition of the approximate mpQP) is kept as a dense row, as is a stored offset when the
//!    multiplier condition was relaxed.
//!  * the stored facets are unit-norm rows, as in `CriticalRegion`; one scale factor per facet
//!    (counted) recovers the normalisation, so point location is identical to
//!    `Solution::evaluate`.
//!  * only the continuous solutions of `MpProgram` are supported (no binaries); the constraint
//!    matrices G, S, b and the parameter set A_t, b_t of the program are shared, not counted (the
//!    uncompressed count of the paper does not include them either).

use crate::linalg::{dot, norm, Mat};
use crate::program::MpProgram;
use crate::solution::Solution;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Primal,
    Dual,
    Omega,
}

#[derive(Clone, Debug)]
struct Facet {
    kind: Kind,
    idx: usize,
    /// norm of the unnormalised row, value = raw / nu - off
    nu: f64,
    off: f64,
}

#[derive(Clone, Debug, Default)]
struct Node {
    parent: Option<usize>,
    children: Vec<usize>,
    /// dense node (root of a tree): absolute x-law and absolute hyperplane rows
    dense: bool,
    x_a: Mat,
    x_b: Vec<f64>,
    /// stored hyperplane index sets S^p, S^d (paper (29))
    sp: Vec<usize>,
    sd: Vec<usize>,
    /// dense node: rows [slope | offset] of the values of the hyperplanes in sp / sd
    root_p: Mat,
    root_d: Mat,
    /// modification node: rank r, factors w (r x (nt+1)), u (r x nx), entries fp (|sp| x r), dd (|sd| x r)
    rank: usize,
    w: Mat,
    u: Mat,
    fp: Mat,
    dd: Mat,
    facets: Vec<Facet>,
    extra_e: Mat,
    extra_f: Vec<f64>,
}

/// Number of stored scalars, in the sense of the paper's (11) and (30).
#[derive(Clone, Debug, Default)]
pub struct StorageCount {
    /// reals of the compressed solution
    pub compressed_reals: usize,
    /// integers (indices, parent pointers) of the compressed solution
    pub compressed_ints: usize,
    /// paper (11): R n_x (n_t + 1) + sum_i facets_i (n_t + 1), the uncompressed solution
    pub baseline_reals: usize,
    /// baseline plus the multiplier law of every region (what `CriticalRegion` actually holds)
    pub baseline_with_duals: usize,
    pub regions: usize,
    pub dense_nodes: usize,
    pub max_depth: usize,
    pub rank_histogram: Vec<usize>,
}

pub struct CompressedSolution {
    program: Arc<MpProgram>,
    nodes: Vec<Node>,
    roots: Vec<usize>,
    overlapping: bool,
    tol: f64,
    nx: usize,
    nt: usize,
    m: usize,
    storage: StorageCount,
}

struct Bits(Vec<u64>);
impl Bits {
    fn new(n: usize) -> Self {
        Bits(vec![0; n.div_ceil(64).max(1)])
    }
    fn set(&mut self, i: usize) {
        self.0[i / 64] |= 1 << (i % 64);
    }
    fn or(&mut self, o: &Bits) {
        for (a, b) in self.0.iter_mut().zip(&o.0) {
            *a |= b;
        }
    }
    fn list(&self) -> Vec<usize> {
        let mut v = vec![];
        for (w, &x) in self.0.iter().enumerate() {
            let mut x = x;
            while x != 0 {
                let b = x.trailing_zeros() as usize;
                v.push(w * 64 + b);
                x &= x - 1;
            }
        }
        v
    }
}

/// m x (nt+1) multiplier law of a region embedded in the full constraint space (zero on inactive rows).
fn lambda_tilde(cr: &crate::critical_region::CriticalRegion, m: usize, nt: usize) -> Mat {
    let mut l = Mat::zeros(m, nt + 1);
    for (p, &k) in cr.active_set.iter().enumerate() {
        if p < cr.c.rows {
            l.row_mut(k)[..nt].copy_from_slice(cr.c.row(p));
            l[(k, nt)] = cr.d[p];
        }
    }
    l
}

fn x_law(cr: &crate::critical_region::CriticalRegion, nx: usize, nt: usize) -> Mat {
    let mut x = Mat::zeros(nx, nt + 1);
    for i in 0..nx {
        x.row_mut(i)[..nt].copy_from_slice(cr.a.row(i));
        x[(i, nt)] = cr.b[i];
    }
    x
}

/// Rows of the hyperplane values of the full constraint set for the x-law: P_j = G_j x - F_j theta - b_j.
fn primal_row(p: &MpProgram, xl: &Mat, j: usize, nt: usize) -> Vec<f64> {
    let g = p.a.row(j);
    let mut r = vec![0.0; nt + 1];
    for (i, &gi) in g.iter().enumerate() {
        if gi != 0.0 {
            for t in 0..=nt {
                r[t] += gi * xl[(i, t)];
            }
        }
    }
    for t in 0..nt {
        r[t] -= p.f[(j, t)];
    }
    r[nt] -= p.b[j];
    r
}

/// Compresses a solution of a continuous `MpProgram`; `None` for mixed-integer solutions.
pub fn compress(sol: &Solution) -> Option<CompressedSolution> {
    let program = sol.continuous.clone()?;
    let nx = program.num_x();
    let nt = program.num_t();
    let m = program.num_constraints();
    let regs = &sol.critical_regions;
    let n = regs.len();
    if regs.iter().any(|r| r.y_fixation.is_some()) {
        return None;
    }
    let omega_norm: Vec<f64> = (0..program.a_t.rows).map(|t| norm(program.a_t.row(t))).collect();
    let xl: Vec<Mat> = regs.iter().map(|r| x_law(r, nx, nt)).collect();

    // ---------------------------------------------------------------- facets of every region
    let mut nodes: Vec<Node> = vec![Node::default(); n];
    for (i, cr) in regs.iter().enumerate() {
        let mut cands: Vec<(Kind, usize, Vec<f64>, f64, f64)> = vec![]; // kind, idx, normalised row, rhs/nu, nu
        for &k in &cr.lambda_set {
            if let Some(p) = cr.active_set.iter().position(|&a| a == k) {
                if p < cr.c.rows {
                    let row: Vec<f64> = cr.c.row(p).iter().map(|v| -v).collect();
                    let nu = norm(&row);
                    if nu > 1e-300 {
                        cands.push((Kind::Dual, k, row.iter().map(|v| v / nu).collect(), cr.d[p] / nu, nu));
                    }
                }
            }
        }
        for &j in &cr.regular_set.1 {
            let r = primal_row(&program, &xl[i], j, nt);
            let nu = norm(&r[..nt]);
            if nu > 1e-300 {
                cands.push((Kind::Primal, j, r[..nt].iter().map(|v| v / nu).collect(), -r[nt] / nu, nu));
            }
        }
        for &t in &cr.omega_set {
            let nu = omega_norm[t];
            if nu > 1e-300 {
                cands.push((Kind::Omega, t, program.a_t.row(t).iter().map(|v| v / nu).collect(), program.b_t[t] / nu, nu));
            }
        }
        let mut facets = vec![];
        let mut extra_rows: Vec<Vec<f64>> = vec![];
        let mut extra_f = vec![];
        for s in 0..cr.e.rows {
            let es = cr.e.row(s);
            let hit = cands.iter().find(|c| c.2.iter().zip(es).all(|(a, b)| (a - b).abs() <= 1e-9));
            match hit {
                Some(c) => {
                    let off = cr.f[s] - c.3;
                    facets.push(Facet { kind: c.0, idx: c.1, nu: c.4, off: if off.abs() < 1e-13 { 0.0 } else { off } });
                }
                None => {
                    extra_rows.push(es.to_vec());
                    extra_f.push(cr.f[s]);
                }
            }
        }
        nodes[i].facets = facets;
        nodes[i].extra_e = if extra_rows.is_empty() { Mat::zeros(0, nt) } else { Mat::from_rows(&extra_rows) };
        nodes[i].extra_f = extra_f;
    }

    // ---------------------------------------------------------------- storage forest
    // active-set neighbours: sets differing by one constraint (or equal)
    let mut by_set: HashMap<&[usize], Vec<usize>> = HashMap::new();
    for (i, r) in regs.iter().enumerate() {
        by_set.entry(&r.active_set[..]).or_default().push(i);
    }
    let neighbours = |i: usize| -> Vec<usize> {
        let a = &regs[i].active_set;
        let mut out: Vec<usize> = vec![];
        if let Some(v) = by_set.get(&a[..]) {
            out.extend(v.iter().cloned());
        }
        for e in 0..a.len() {
            let mut s = a.clone();
            s.remove(e);
            if let Some(v) = by_set.get(&s[..]) {
                out.extend(v.iter().cloned());
            }
        }
        let mut s = vec![0usize; a.len() + 1];
        for k in 0..m {
            if a.binary_search(&k).is_ok() {
                continue;
            }
            let pos = a.partition_point(|&v| v < k);
            s[..pos].copy_from_slice(&a[..pos]);
            s[pos] = k;
            s[pos + 1..].copy_from_slice(&a[pos..]);
            if let Some(v) = by_set.get(&s[..]) {
                out.extend(v.iter().cloned());
            }
        }
        out
    };
    // BFS forest; the root of each component is the best (smallest depth) of a few trial starts
    let mut parent: Vec<Option<usize>> = vec![None; n];
    let mut visited = vec![false; n];
    let mut order: Vec<usize> = Vec::with_capacity(n);
    let bfs = |start: usize, allowed: &[bool]| -> (Vec<usize>, Vec<Option<usize>>, usize) {
        let mut seen: HashMap<usize, Option<usize>> = HashMap::new();
        let mut depth: HashMap<usize, usize> = HashMap::new();
        let mut q = std::collections::VecDeque::new();
        seen.insert(start, None);
        depth.insert(start, 0);
        q.push_back(start);
        let mut ord = vec![];
        let mut maxd = 0;
        while let Some(u) = q.pop_front() {
            ord.push(u);
            let du = depth[&u];
            maxd = maxd.max(du);
            for v in neighbours(u) {
                if !allowed[v] || seen.contains_key(&v) {
                    continue;
                }
                seen.insert(v, Some(u));
                depth.insert(v, du + 1);
                q.push_back(v);
            }
        }
        let pars = ord.iter().map(|u| seen[u]).collect();
        (ord, pars, maxd)
    };
    let mut allowed: Vec<bool> = vec![true; n];
    for s in 0..n {
        if visited[s] {
            continue;
        }
        let (comp, _, _) = bfs(s, &allowed);
        // trial roots: evenly spaced members of the component
        let tries = comp.len().min(6);
        let mut best: Option<(Vec<usize>, Vec<Option<usize>>, usize)> = None;
        for k in 0..tries {
            let cand = comp[k * comp.len() / tries];
            let res = bfs(cand, &allowed);
            if best.as_ref().map_or(true, |b| res.2 < b.2) {
                best = Some(res);
            }
        }
        let (ord, pars, _) = best.unwrap();
        for (u, p) in ord.iter().zip(&pars) {
            visited[*u] = true;
            allowed[*u] = false;
            parent[*u] = *p;
            order.push(*u);
        }
    }

    // ---------------------------------------------------------------- modifications
    // hyperplanes needed in each node's subtree (paper (29)); dense nodes cut the subtree
    for &u in &order {
        if let Some(p) = parent[u] {
            nodes[u].parent = Some(p);
            nodes[p].children.push(u);
        }
    }
    let mut svd_cache: Vec<Option<(usize, Mat, Mat, Mat)>> = vec![None; n]; // rank, w, u_x, u_lambda (m x r)
    for &u in &order {
        let Some(p) = parent[u] else { continue };
        let lt_u = lambda_tilde(&regs[u], m, nt);
        let lt_p = lambda_tilde(&regs[p], m, nt);
        let fro = |v: &[f64]| v.iter().map(|x| x * x).sum::<f64>().sqrt();
        let dx: Vec<f64> = (0..nx * (nt + 1)).map(|k| xl[u].data[k] - xl[p].data[k]).collect();
        let dl: Vec<f64> = (0..m * (nt + 1)).map(|k| lt_u.data[k] - lt_p.data[k]).collect();
        let (dxn, dln) = (fro(&dx), fro(&dl));
        let x_zero = dxn <= 1e-14 * (fro(&xl[u].data) + fro(&xl[p].data));
        let l_zero = dln <= 1e-14 * (fro(&lt_u.data) + fro(&lt_p.data));
        if x_zero && l_zero {
            svd_cache[u] = Some((0, Mat::zeros(0, nt + 1), Mat::zeros(nx, 0), Mat::zeros(m, 0)));
            continue;
        }
        // weight the multiplier block so that both blocks are resolved to the same relative accuracy
        let wl = if x_zero || l_zero || dln == 0.0 { 1.0 } else { dxn / dln };
        let mut d = Mat::zeros(nx + m, nt + 1);
        for i in 0..nx * (nt + 1) {
            d.data[i] = dx[i];
        }
        for i in 0..m * (nt + 1) {
            d.data[nx * (nt + 1) + i] = dl[i] * wl;
        }
        // rank-revealing factorisation D = sum_k q_k R_k (modified Gram-Schmidt with column pivoting
        // and one re-orthogonalisation); accurate to the discarded residual, unlike a Gram matrix
        let big_n = nx + m;
        let ncol = nt + 1;
        let mut res: Vec<Vec<f64>> = (0..ncol).map(|j| (0..big_n).map(|i| d[(i, j)]).collect()).collect();
        let cmax = res.iter().map(|c| fro(c)).fold(0.0, f64::max);
        let mut qs: Vec<Vec<f64>> = vec![];
        let mut rr: Vec<Vec<f64>> = vec![];
        loop {
            let (jbest, nbest) = res.iter().enumerate().map(|(j, c)| (j, fro(c))).fold((0, 0.0), |a, b| if b.1 > a.1 { b } else { a });
            if nbest <= 1e-13 * cmax || qs.len() >= ncol {
                break;
            }
            let q: Vec<f64> = res[jbest].iter().map(|v| v / nbest).collect();
            let mut row = vec![0.0; ncol];
            for j in 0..ncol {
                for _ in 0..2 {
                    let c = dot(&q, &res[j]);
                    row[j] += c;
                    for (rv, qv) in res[j].iter_mut().zip(&q) {
                        *rv -= c * qv;
                    }
                }
            }
            qs.push(q);
            rr.push(row);
        }
        let r = qs.len();
        let mut w = Mat::zeros(r, nt + 1);
        let mut ux = Mat::zeros(nx, r);
        let mut ul = Mat::zeros(m, r);
        for k in 0..r {
            w.row_mut(k).copy_from_slice(&rr[k]);
            for i in 0..nx {
                ux[(i, k)] = qs[k][i];
            }
            for i in 0..m {
                ul[(i, k)] = qs[k][nx + i] / wl;
            }
        }
        svd_cache[u] = Some((r, w, ux, ul));
    }

    // dense decision: iterate twice, S sets computed bottom-up ignoring subtrees below dense nodes
    let mut dense = vec![false; n];
    for &u in &order {
        if parent[u].is_none() {
            dense[u] = true;
        }
    }
    let mut sp_sets: Vec<Vec<usize>> = vec![vec![]; n];
    let mut sd_sets: Vec<Vec<usize>> = vec![vec![]; n];
    for pass in 0..3 {
        let mut bp: Vec<Bits> = (0..n).map(|_| Bits::new(m)).collect();
        let mut bd: Vec<Bits> = (0..n).map(|_| Bits::new(m)).collect();
        for &u in order.iter().rev() {
            for f in &nodes[u].facets {
                match f.kind {
                    Kind::Primal => bp[u].set(f.idx),
                    Kind::Dual => bd[u].set(f.idx),
                    Kind::Omega => {}
                }
            }
            if !dense[u] {
                if let Some(p) = parent[u] {
                    // u keeps its own sets for its record and also contributes them to the parent
                    let (ac, bc) = (Bits(bp[u].0.clone()), Bits(bd[u].0.clone()));
                    bp[p].or(&ac);
                    bd[p].or(&bc);
                }
            }
        }
        for u in 0..n {
            sp_sets[u] = bp[u].list();
            sd_sets[u] = bd[u].list();
        }
        let mut changed = false;
        for &u in &order {
            // the last pass only recomputes the sets for the final decisions
            if parent[u].is_none() || pass == 2 {
                continue;
            }
            let r = svd_cache[u].as_ref().unwrap().0;
            let sz = sp_sets[u].len() + sd_sets[u].len();
            let lr_cost = r * (nt + 1 + nx + sz);
            let dense_cost = nx * (nt + 1) + sz * (nt + 1);
            let want_dense = lr_cost >= dense_cost;
            if want_dense != dense[u] {
                dense[u] = want_dense;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // ---------------------------------------------------------------- fill nodes
    let mut st = StorageCount { regions: n, ..Default::default() };
    st.rank_histogram = vec![0; 8];
    let mut depth = vec![0usize; n];
    for &u in &order {
        if let Some(p) = parent[u] {
            depth[u] = depth[p] + 1;
        }
        st.max_depth = st.max_depth.max(depth[u]);
    }
    for &u in &order {
        let sp = sp_sets[u].clone();
        let sd = sd_sets[u].clone();
        let node = &mut nodes[u];
        node.dense = dense[u];
        node.sp = sp.clone();
        node.sd = sd.clone();
        if dense[u] {
            st.dense_nodes += 1;
            node.x_a = Mat::zeros(nx, nt);
            node.x_b = vec![0.0; nx];
            for i in 0..nx {
                node.x_a.row_mut(i).copy_from_slice(&xl[u].row(i)[..nt]);
                node.x_b[i] = xl[u][(i, nt)];
            }
            node.root_p = Mat::zeros(sp.len(), nt + 1);
            for (k, &j) in sp.iter().enumerate() {
                node.root_p.row_mut(k).copy_from_slice(&primal_row(&program, &xl[u], j, nt));
            }
            let lt = lambda_tilde(&regs[u], m, nt);
            node.root_d = Mat::zeros(sd.len(), nt + 1);
            for (k, &j) in sd.iter().enumerate() {
                for t in 0..=nt {
                    node.root_d[(k, t)] = -lt[(j, t)];
                }
            }
            st.compressed_reals += nx * (nt + 1) + (sp.len() + sd.len()) * (nt + 1);
        } else {
            let (r, w, ux, ul) = svd_cache[u].clone().unwrap();
            node.rank = r;
            node.w = w;
            node.u = Mat::zeros(r, nx);
            for k in 0..r {
                for i in 0..nx {
                    node.u[(k, i)] = ux[(i, k)];
                }
            }
            node.fp = Mat::zeros(sp.len(), r);
            for (a, &j) in sp.iter().enumerate() {
                let g = program.a.row(j);
                for k in 0..r {
                    let mut s = 0.0;
                    for (i, &gi) in g.iter().enumerate() {
                        s += gi * ux[(i, k)];
                    }
                    node.fp[(a, k)] = s;
                }
            }
            node.dd = Mat::zeros(sd.len(), r);
            for (a, &j) in sd.iter().enumerate() {
                for k in 0..r {
                    node.dd[(a, k)] = -ul[(j, k)];
                }
            }
            st.compressed_reals += r * (nt + 1 + nx + sp.len() + sd.len());
            let h = r.min(7);
            st.rank_histogram[h] += 1;
        }
        st.compressed_reals += node.facets.len() + node.facets.iter().filter(|f| f.off != 0.0).count() + node.extra_e.rows * (nt + 1);
        st.compressed_ints += node.facets.len() + sp.len() + sd.len() + 1;
        st.baseline_reals += nx * (nt + 1) + (node.facets.len() + node.extra_e.rows) * (nt + 1);
        st.baseline_with_duals += regs[u].active_set.len() * (nt + 1);
    }
    st.baseline_with_duals += st.baseline_reals;
    // dense nodes below a root are reached as children and reset the running state
    Some(CompressedSolution {
        program,
        nodes,
        roots: order.iter().cloned().filter(|&u| parent[u].is_none()).collect(),
        overlapping: sol.is_overlapping,
        tol: sol.point_location_tolerance,
        nx,
        nt,
        m,
        storage: st,
    })
}

/// Depth-first traversal state: one shared (x, P, D) with an undo log, so entering a node costs only
/// the entries it modifies.
enum Op {
    Enter(usize),
    Exit,
}

impl CompressedSolution {
    pub fn storage(&self) -> &StorageCount {
        &self.storage
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Index (in the original solution) of the region containing theta, as `Solution::get_region_index`.
    pub fn locate(&self, theta: &[f64]) -> Option<usize> {
        self.locate_with_x(theta).map(|(i, _)| i)
    }

    /// x*(theta) from the compressed form (paper Algorithm 1), `None` outside every region.
    pub fn evaluate(&self, theta: &[f64]) -> Option<Vec<f64>> {
        self.locate_with_x(theta).map(|(_, x)| x)
    }

    fn locate_with_x(&self, theta: &[f64]) -> Option<(usize, Vec<f64>)> {
        let (nx, nt, m) = (self.nx, self.nt, self.m);
        let omega: Vec<f64> = (0..self.program.a_t.rows).map(|t| dot(self.program.a_t.row(t), theta) - self.program.b_t[t]).collect();
        let mut hits: Vec<(usize, Vec<f64>, f64)> = vec![];
        let mut x = vec![0.0; nx];
        let mut pv = vec![0.0; m];
        let mut dv = vec![0.0; m];
        // undo log: (which array 0/1/2, index, old value); an Exit op rewinds to a saved length
        let mut log: Vec<(u8, u32, f64)> = Vec::new();
        let mut marks: Vec<usize> = Vec::new();
        let mut stack: Vec<Op> = self.roots.iter().rev().map(|&r| Op::Enter(r)).collect();
        let mut g: Vec<f64> = vec![];
        while let Some(op) = stack.pop() {
            let ni = match op {
                Op::Exit => {
                    let to = marks.pop().unwrap();
                    while log.len() > to {
                        let (w, i, v) = log.pop().unwrap();
                        match w {
                            0 => x[i as usize] = v,
                            1 => pv[i as usize] = v,
                            _ => dv[i as usize] = v,
                        }
                    }
                    continue;
                }
                Op::Enter(ni) => ni,
            };
            let node = &self.nodes[ni];
            marks.push(log.len());
            if node.dense {
                for i in 0..nx {
                    log.push((0, i as u32, x[i]));
                    x[i] = dot(node.x_a.row(i), theta) + node.x_b[i];
                }
                for (k, &j) in node.sp.iter().enumerate() {
                    let r = node.root_p.row(k);
                    log.push((1, j as u32, pv[j]));
                    pv[j] = dot(&r[..nt], theta) + r[nt];
                }
                for (k, &j) in node.sd.iter().enumerate() {
                    let r = node.root_d.row(k);
                    log.push((2, j as u32, dv[j]));
                    dv[j] = dot(&r[..nt], theta) + r[nt];
                }
            } else {
                g.clear();
                for k in 0..node.rank {
                    let w = node.w.row(k);
                    g.push(dot(&w[..nt], theta) + w[nt]);
                }
                if node.rank > 0 {
                    for i in 0..nx {
                        log.push((0, i as u32, x[i]));
                    }
                    for k in 0..node.rank {
                        let gk = g[k];
                        for (xi, ui) in x.iter_mut().zip(node.u.row(k)) {
                            *xi += gk * ui;
                        }
                    }
                    for (a, &j) in node.sp.iter().enumerate() {
                        log.push((1, j as u32, pv[j]));
                        pv[j] += dot(node.fp.row(a), &g);
                    }
                    for (a, &j) in node.sd.iter().enumerate() {
                        log.push((2, j as u32, dv[j]));
                        dv[j] += dot(node.dd.row(a), &g);
                    }
                }
            }
            // facets of this node's region
            let mut margin = f64::NEG_INFINITY;
            let mut inside = true;
            for f in &node.facets {
                let raw = match f.kind {
                    Kind::Primal => pv[f.idx],
                    Kind::Dual => dv[f.idx],
                    Kind::Omega => omega[f.idx],
                };
                let v = raw / f.nu - f.off;
                if v >= self.tol {
                    inside = false;
                    break;
                }
                margin = margin.max(v);
            }
            if inside {
                for r in 0..node.extra_e.rows {
                    let v = dot(node.extra_e.row(r), theta) - node.extra_f[r];
                    if v >= self.tol {
                        inside = false;
                        break;
                    }
                    margin = margin.max(v);
                }
            }
            if inside {
                if !self.overlapping && margin < -1e-4 {
                    return Some((ni, x));
                }
                hits.push((ni, x.clone(), margin));
            }
            stack.push(Op::Exit);
            for &c in node.children.iter().rev() {
                stack.push(Op::Enter(c));
            }
        }
        if hits.is_empty() {
            return None;
        }
        hits.sort_by_key(|h| h.0);
        if !self.overlapping {
            return hits.into_iter().next().map(|(a, b, _)| (a, b));
        }
        // Regions containing theta strictly (margin <= 0) take precedence over those within tol.
        let mut best = [f64::INFINITY; 2];
        let mut best_k: [Option<usize>; 2] = [None, None];
        for (k, (_, x, margin)) in hits.iter().enumerate() {
            let class = if *margin <= 0.0 { 0 } else { 1 };
            let obj = self.program.evaluate_objective(x, theta);
            if obj <= best[class] {
                best[class] = obj;
                best_k[class] = Some(k);
            }
        }
        let best_i = best_k[0].or(best_k[1]);
        best_i.map(|k| {
            let (a, b, _) = hits.swap_remove(k);
            (a, b)
        })
    }
}
