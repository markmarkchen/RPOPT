//! Multiparametric linear complementarity (mp-LCP) solver for strictly convex mpQPs.
//!
//! Jones and Morari, "Multiparametric linear complementarity problems", CDC 2006 (Algorithms 1
//! and 2): the KKT system of the mpQP is the pLCP  w - M z = q + R θ  (z = multipliers,
//! w = slacks, M = A Q^{-1} A' positive semidefinite). Critical regions are enumerated
//! facet to facet. A region is a complementary basis B; its neighbour across a facet is found by
//! pivoting, with the lexicographic perturbation  q + R θ + ε,  ε = (ε0, ε0², ..., ε0^n),  of the
//! right hand side kept in the tableau (columns β = A_B^{-1}), so that every θ lies in exactly one
//! region and degenerate pivots are resolved consistently.
//!
//! Equality rows have free multipliers; they are eliminated before the pLCP is formed (Schur
//! complement of M on the equality block) and are always in the active set.
//!
//! Tableau form (compact, one column per nonbasic variable): x_B = ρ + T y_N, where the rows of ρ
//! are [constant | θ coefficients | β]. Variables: w_i = i, z_i = n + i, the facet parameter α = 2n.

use crate::critical_region::CriticalRegion;
use crate::fxhash::FxHashSet as HashSet;
use crate::linalg::{Lu, Mat};
use crate::mpqp_utils::gen_cr_from_active_set_lambda_first;
use crate::program::MpProgram;
use crate::utils::chebyshev_ball;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::cmp::Ordering;
use std::collections::VecDeque;

/// The reduced pLCP (equalities eliminated).
struct Plcp {
    n: usize,
    nt: usize,
    /// n x n
    m: Vec<f64>,
    q: Vec<f64>,
    /// n x nt
    r: Vec<f64>,
    /// original constraint index of each pLCP row
    ineq: Vec<usize>,
    eq: Vec<usize>,
}

impl Plcp {
    fn new(program: &MpProgram) -> Option<Plcp> {
        let kc = program.kkt_cache_pub()?;
        let nt = program.num_t();
        let mfull = kc.m.rows;
        let eq = program.equality_indices.clone();
        let ineq: Vec<usize> = (0..mfull).filter(|i| !eq.contains(i)).collect();
        let n = ineq.len();
        let mut m = vec![0.0; n * n];
        let mut q = vec![0.0; n];
        let mut r = vec![0.0; n * nt];
        for (a, &i) in ineq.iter().enumerate() {
            for (b, &j) in ineq.iter().enumerate() {
                m[a * n + b] = kc.m[(i, j)];
            }
            q[a] = kc.base_f[i];
            for t in 0..nt {
                r[a * nt + t] = -kc.base_e[(i, t)];
            }
        }
        if !eq.is_empty() {
            let ne = eq.len();
            let mee = Mat::from_vec(ne, ne, eq.iter().flat_map(|&i| eq.iter().map(move |&j| (i, j))).map(|(i, j)| kc.m[(i, j)]).collect());
            let lu = Lu::new(&mee)?;
            // X = M_EE^{-1} [M_EI | q_E | R_E]
            let w = n + 1 + nt;
            let mut rhs = Mat::zeros(ne, w);
            for (a, &i) in eq.iter().enumerate() {
                for (b, &j) in ineq.iter().enumerate() {
                    rhs[(a, b)] = kc.m[(i, j)];
                }
                rhs[(a, n)] = kc.base_f[i];
                for t in 0..nt {
                    rhs[(a, n + 1 + t)] = -kc.base_e[(i, t)];
                }
            }
            let x = lu.solve_mat(&rhs);
            for (a, &i) in ineq.iter().enumerate() {
                // M_IE X
                let mie: Vec<f64> = eq.iter().map(|&e| kc.m[(i, e)]).collect();
                for b in 0..n {
                    let mut s = 0.0;
                    for (k, mk) in mie.iter().enumerate() {
                        s += mk * x[(k, b)];
                    }
                    m[a * n + b] -= s;
                }
                let mut s = 0.0;
                for (k, mk) in mie.iter().enumerate() {
                    s += mk * x[(k, n)];
                }
                q[a] -= s;
                for t in 0..nt {
                    let mut s = 0.0;
                    for (k, mk) in mie.iter().enumerate() {
                        s += mk * x[(k, n + 1 + t)];
                    }
                    r[a * nt + t] -= s;
                }
            }
        }
        Some(Plcp { n, nt, m, q, r, ineq, eq })
    }

    /// Full active set (equalities first) of the z-basic rows.
    fn active_set(&self, z_rows: impl Iterator<Item = usize>) -> Vec<usize> {
        let mut v: Vec<usize> = z_rows.map(|k| self.ineq[k]).collect();
        v.sort_unstable();
        let mut out = self.eq.clone();
        out.extend(v);
        out
    }
}

/// Compact tableau x_B = ρ + T y_N with `nc` nonbasic columns and rows ρ of width `w`.
#[derive(Clone)]
struct Tab {
    n: usize,
    nc: usize,
    w: usize,
    t: Vec<f64>,
    rho: Vec<f64>,
    row_var: Vec<usize>,
    col_var: Vec<usize>,
}

impl Tab {
    /// Principal pivot (row r, column c).
    fn pivot(&mut self, r: usize, c: usize) {
        let (nc, w) = (self.nc, self.w);
        let p = self.t[r * nc + c];
        let inv = 1.0 / p;
        let mut trow = self.t[r * nc..(r + 1) * nc].to_vec();
        for (k, v) in trow.iter_mut().enumerate() {
            *v = if k == c { inv } else { -*v * inv };
        }
        let mut rrow = self.rho[r * w..(r + 1) * w].to_vec();
        for v in rrow.iter_mut() {
            *v *= -inv;
        }
        for i in 0..self.n {
            if i == r {
                continue;
            }
            let f = self.t[i * nc + c];
            if f == 0.0 {
                continue;
            }
            let ti = &mut self.t[i * nc..(i + 1) * nc];
            for (k, (o, s)) in ti.iter_mut().zip(&trow).enumerate() {
                if k == c {
                    *o = f * s;
                } else {
                    *o += f * s;
                }
            }
            let ri = &mut self.rho[i * w..(i + 1) * w];
            for (o, s) in ri.iter_mut().zip(&rrow) {
                *o += f * s;
            }
        }
        self.t[r * nc..(r + 1) * nc].copy_from_slice(&trow);
        self.rho[r * w..(r + 1) * w].copy_from_slice(&rrow);
        std::mem::swap(&mut self.row_var[r], &mut self.col_var[c]);
    }

    fn col_of(&self, var: usize) -> usize {
        self.col_var.iter().position(|&v| v == var).unwrap()
    }
}

/// Persistent tableau of the basis with z-basic rows `z`: rows [q | R | I], T = M, then one
/// principal pivot per z row (nonsingular when M_zz is positive definite).
fn tableau_for(p: &Plcp, z: &[usize]) -> Option<Tab> {
    let (n, nt) = (p.n, p.nt);
    let w = 1 + nt + n;
    let mut rho = vec![0.0; n * w];
    for i in 0..n {
        rho[i * w] = p.q[i];
        rho[i * w + 1..i * w + 1 + nt].copy_from_slice(&p.r[i * nt..(i + 1) * nt]);
        rho[i * w + 1 + nt + i] = 1.0;
    }
    let mut tab = Tab { n, nc: n, w, t: p.m.clone(), rho, row_var: (0..n).collect(), col_var: (n..2 * n).collect() };
    for &i in z {
        let piv = tab.t[i * n + i];
        if piv.abs() < 1e-11 {
            return None;
        }
        tab.pivot(i, i);
    }
    Some(tab)
}

const ZTOL: f64 = 1e-9;
const PTOL: f64 = 1e-10;

fn comp(var: usize, n: usize) -> usize {
    if var < n {
        var + n
    } else {
        var - n
    }
}

fn lex_cmp(a: &[f64], b: &[f64]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        let tol = 1e-12 * (1.0 + x.abs().max(y.abs()));
        if x - y > tol {
            return Ordering::Greater;
        }
        if y - x > tol {
            return Ordering::Less;
        }
    }
    Ordering::Equal
}

/// Is `v` (first entry the scalar part, then the perturbation columns) lexico-nonnegative?
fn lex_nonneg(v: &[f64]) -> bool {
    for x in v {
        if x.abs() > ZTOL {
            return *x > 0.0;
        }
    }
    true
}

struct Facet<'a> {
    /// facet-space rows of the polytope F (E_f θ_f <= f_f)
    ef: &'a Mat,
    ff: &'a [f64],
}

/// Algorithm 2 (`pivot`): from the ACLFB `tab` (α basic or entering as column `e`) returns the
/// z-row sets of the adjacent bases.
fn pivot_facet(p: &Plcp, mut tab: Tab, e: usize, fpoly: Facet, depth: usize, out: &mut Vec<Vec<usize>>) {
    let n = p.n;
    let d1 = p.nt - 1;
    let alpha = 2 * n;
    let mut e = e;
    for _ in 0..(8 * n + 50) {
        // P: rows decreasing in the entering variable
        let a: Vec<f64> = (0..n).map(|i| -tab.t[i * tab.nc + e]).collect();
        let w = tab.w;
        let rowmax = |i: usize| tab.rho[i * w..i * w + 1 + d1].iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let pset: Vec<usize> = (0..n).filter(|&i| a[i] > PTOL).collect();
        if pset.is_empty() {
            return;
        }
        let in_z = |i: usize| rowmax(i) <= ZTOL;
        let pz: Vec<usize> = pset.iter().cloned().filter(|&i| in_z(i)).collect();
        let alpha_row = tab.row_var.iter().position(|&v| v == alpha);
        if pz.is_empty() {
            if let Some(ra) = alpha_row {
                if tab.t[ra * tab.nc + e] > PTOL {
                    tab.pivot(ra, e);
                    out.push(z_rows(&tab, n));
                    return;
                }
            }
        }
        if !pz.is_empty() {
            // Case 1: leaving variable independent of θ_f
            let beta = |i: usize| &tab.rho[i * w + 1 + d1..(i + 1) * w];
            let mut l = pz[0];
            for &i in &pz[1..] {
                let u: Vec<f64> = beta(i).iter().map(|v| v / a[i]).collect();
                let v: Vec<f64> = beta(l).iter().map(|v| v / a[l]).collect();
                if lex_cmp(&u, &v) == Ordering::Less {
                    l = i;
                }
            }
            let leaving = tab.row_var[l];
            if leaving == alpha {
                return;
            }
            tab.pivot(l, e);
            e = tab.col_of(comp(leaving, n));
            continue;
        }
        // Case 2: the leaving variable depends on θ_f
        if depth > 6 {
            return;
        }
        for &l in &pset {
            let leaving = tab.row_var[l];
            // lexico-inequalities (16): Γ_j = a_l row_j - a_j row_l  ⪰ 0 for all j in P
            let mut ok = true;
            let mut ef = fpoly.ef.clone();
            let mut ff = fpoly.ff.to_vec();
            for &j in &pset {
                if j == l {
                    continue;
                }
                let g: Vec<f64> = (0..w).map(|k| a[l] * tab.rho[j * w + k] - a[j] * tab.rho[l * w + k]).collect();
                let gq_zero = g[1..1 + d1].iter().all(|v| v.abs() <= ZTOL);
                if gq_zero {
                    let mut v = vec![g[0]];
                    v.extend_from_slice(&g[1 + d1..]);
                    if !lex_nonneg(&v) {
                        ok = false;
                        break;
                    }
                } else {
                    // g0 + g_q θ_f >= 0  ->  -g_q θ_f <= g0
                    let nrm = g[1..1 + d1].iter().map(|v| v * v).sum::<f64>().sqrt();
                    let row: Vec<f64> = g[1..1 + d1].iter().map(|v| -v / nrm).collect();
                    ef.push_row(&row);
                    ff.push(g[0] / nrm);
                }
            }
            if !ok {
                continue;
            }
            if d1 > 0 {
                match chebyshev_ball(&ef, &ff, &[]) {
                    Some(s) if s.x[d1] > 1e-9 => {}
                    _ => continue,
                }
            }
            let mut next = tab.clone();
            next.pivot(l, e);
            if leaving == alpha {
                out.push(z_rows(&next, n));
                continue;
            }
            let ne = next.col_of(comp(leaving, n));
            pivot_facet(p, next, ne, Facet { ef: &ef, ff: &ff }, depth + 1, out);
        }
        return;
    }
}

/// z rows of the basis `tab` with the pair of row v toggled.
fn z_toggle(tab: &Tab, n: usize, v: usize) -> Vec<usize> {
    let mut z = z_rows(tab, n);
    let var = tab.row_var[v];
    if var < n {
        z.push(var);
        z.sort_unstable();
    } else {
        z.retain(|&x| x != var - n);
    }
    z
}

fn z_rows(tab: &Tab, n: usize) -> Vec<usize> {
    let mut v: Vec<usize> = tab.row_var.iter().filter(|&&x| x >= n && x < 2 * n).map(|&x| x - n).collect();
    v.sort_unstable();
    v
}

/// Orthonormal basis (columns, nt x (nt-1)) of the orthogonal complement of the unit vector g.
fn null_basis(g: &[f64]) -> Vec<Vec<f64>> {
    let nt = g.len();
    let mut basis: Vec<Vec<f64>> = vec![];
    let skip = (0..nt).max_by(|&a, &b| g[a].abs().partial_cmp(&g[b].abs()).unwrap()).unwrap();
    for k in 0..nt {
        if k == skip {
            continue;
        }
        let mut v = vec![0.0; nt];
        v[k] = 1.0;
        for u in std::iter::once(g).chain(basis.iter().map(|b| b.as_slice())) {
            let dp: f64 = v.iter().zip(u).map(|(a, b)| a * b).sum();
            for (x, y) in v.iter_mut().zip(u) {
                *x -= dp * y;
            }
        }
        let nrm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        for x in v.iter_mut() {
            *x /= nrm;
        }
        basis.push(v);
    }
    basis
}

/// Neighbouring z-row sets across the facet {e_k θ = f_k} of the region with tableau `tab`.
fn adjacent(p: &Plcp, tab: &Tab, erow: &[f64], frhs: f64, e_all: &Mat, f_all: &[f64]) -> Vec<Vec<usize>> {
    let (n, nt) = (p.n, p.nt);
    let nrm = erow.iter().map(|v| v * v).sum::<f64>().sqrt();
    let g: Vec<f64> = erow.iter().map(|v| v / nrm).collect();
    let b = -frhs / nrm; // γ'θ + b = 0
    let d1 = nt - 1;
    // rows identically zero on the facet (none: boundary of Θ), and the α column γ_i = ρθ_i · g
    let mut gam = vec![0.0; n];
    let mut zrows: Vec<usize> = vec![];
    for i in 0..n {
        let rr = &tab.rho[i * tab.w..(i + 1) * tab.w];
        let gi: f64 = (0..nt).map(|k| rr[1 + k] * g[k]).sum();
        gam[i] = gi;
        if (rr[0] - b * gi).abs() <= ZTOL && (0..nt).map(|k| (rr[1 + k] - gi * g[k]).powi(2)).sum::<f64>().sqrt() <= ZTOL {
            zrows.push(i);
        }
    }
    if zrows.is_empty() {
        return vec![];
    }
    // Fast path of Algorithm 2: a single leaving candidate v in P ∩ Z (Case 1 without ties), after
    // which α is basic and the entering variable comp(v) makes P' ∩ Z' empty with α rising:
    // the neighbour is the principal pivot (toggle) of v. Only column entries are needed.
    let pv: Vec<usize> = zrows.iter().cloned().filter(|&i| gam[i] < -PTOL).collect();
    if pv.len() == 1 {
        let v = pv[0];
        let var = tab.row_var[v];
        let kk = tab.col_of(comp(var, n));
        let pp = gam[v];
        let tve = -tab.t[v * n + kk] / pp;
        if tve > PTOL {
            let blocked = zrows.iter().any(|&i| i != v && -(tab.t[i * n + kk] + gam[i] * tve) > PTOL);
            if !blocked {
                return vec![z_toggle(tab, n, v)];
            }
        }
    }
    let nb = null_basis(&g);
    let w = 1 + d1 + n;
    // work tableau: n x (n+1) with the α column last
    let nc = n + 1;
    let mut t = vec![0.0; n * nc];
    let mut rho = vec![0.0; n * w];
    for i in 0..n {
        t[i * nc..i * nc + n].copy_from_slice(&tab.t[i * n..(i + 1) * n]);
        let rr = &tab.rho[i * tab.w..(i + 1) * tab.w];
        t[i * nc + n] = gam[i];
        rho[i * w] = rr[0] - b * gam[i];
        for (c, v) in nb.iter().enumerate() {
            rho[i * w + 1 + c] = (0..nt).map(|k| rr[1 + k] * v[k]).sum();
        }
        rho[i * w + 1 + d1..(i + 1) * w].copy_from_slice(&rr[1 + nt..]);
    }
    let mut col_var = tab.col_var.clone();
    col_var.push(2 * n);
    let work = Tab { n, nc, w, t, rho, row_var: tab.row_var.clone(), col_var };
    // F: region ∩ Θ restricted to the facet hyperplane, θ = N θ_f - γ b
    let mut ef = Mat::zeros(0, d1);
    let mut ff = vec![];
    if d1 > 0 {
        for i in 0..e_all.rows {
            let r = e_all.row(i);
            let row: Vec<f64> = nb.iter().map(|v| r.iter().zip(v).map(|(a, b)| a * b).sum()).collect();
            let nr = row.iter().map(|v| v * v).sum::<f64>().sqrt();
            if nr < 1e-12 {
                continue;
            }
            let eg: f64 = r.iter().zip(&g).map(|(a, b)| a * b).sum();
            ef.push_row(&row.iter().map(|v| v / nr).collect::<Vec<_>>());
            ff.push((f_all[i] + b * eg) / nr);
        }
    }
    let mut out = vec![];
    pivot_facet(p, work, n, Facet { ef: &ef, ff: &ff }, 0, &mut out);
    out
}

/// Solves the mpQP by enumerating the critical regions of its pLCP.
pub fn mplcp(program: &MpProgram, seed: u64) -> Vec<CriticalRegion> {
    let Some(plcp) = Plcp::new(program) else {
        panic!("mplcp needs a strictly convex mpQP (M = A Q^-1 A') with nonsingular equality block; mpLPs are not supported");
    };
    let mut rng = StdRng::seed_from_u64(seed);
    let Some(active_set) = program.gen_optimal_active_set(&mut rng) else { return vec![] };
    let Some(initial) = crate::mpqp_utils::gen_cr_from_active_set(program, &active_set, false) else { return vec![] };
    let mut regions = vec![initial.clone()];
    let mut seen: HashSet<Vec<usize>> = HashSet::default();
    seen.insert(initial.active_set.clone());
    let mut queue: VecDeque<CriticalRegion> = VecDeque::new();
    queue.push_back(initial);
    let neq = plcp.eq.len();
    while let Some(cur) = queue.pop_front() {
        // z rows of the region (position in the reduced LCP)
        let z: Vec<usize> = cur.active_set[neq..].iter().map(|c| plcp.ineq.iter().position(|x| x == c).unwrap()).collect();
        let Some(tab) = tableau_for(&plcp, &z) else { continue };
        for k in 0..cur.e.rows {
            for nz in adjacent(&plcp, &tab, cur.e.row(k), cur.f[k], &cur.e, &cur.f) {
                let aset = plcp.active_set(nz.into_iter());
                if !seen.insert(aset.clone()) {
                    continue;
                }
                if let Some(cr) = gen_cr_from_active_set_lambda_first(program, &aset, true) {
                    regions.push(cr.clone());
                    queue.push_back(cr);
                }
            }
        }
    }
    regions
}
