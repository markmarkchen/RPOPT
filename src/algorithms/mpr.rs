//! MPR: randomized multi-parametric programming (Falsone, Bianchi, Prandini, Automatica 2026,
//! DOI 10.1016/j.automatica.2026.112873).
//!
//! Algorithm 1 of the paper: sample theta uniformly from the box D (the bounding box of the
//! parameter set); a sample that is feasible and outside every region found so far yields a new
//! critical region (optimal active set from the substituted QP/LP, then the region from that active
//! set); a sample that is infeasible or already covered increments a counter that resets whenever a
//! region is found. The loop stops after N consecutive such samples, with N from Theorem 1 /
//! Corollary 1: `N >= (ln beta - ln n_r) / ln(1 - eps)`. Then, with confidence at least `1 - beta`,
//! `vol((Theta_f \ Theta_C) ∩ D) / vol(D) <= eps`.
//!
//! Algorithm 3 (Section 3) is the default: infeasible samples are cut off by hyperplanes from their
//! projection onto Theta_f, so later infeasible samples are rejected without a QP. It returns the
//! same regions as Algorithm 1 for the same sample stream.
//!
//! The solver processes samples in rayon batches (pure speed-up: results are folded in sample
//! order, so the output equals the serial algorithm for the same sample stream).

use crate::critical_region::CriticalRegion;
use crate::linalg::Mat;
use crate::mpqp_utils::gen_cr_from_active_set;
use crate::program::MpProgram;
use crate::solution::Solution;
use crate::solver::qp::{solve_qp_factored, QpFactor};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use std::sync::Arc;


/// Options beyond the paper's (epsilon, beta).
#[derive(Clone, Debug)]
pub struct MprOptions {
    /// Upper bound on the number of critical regions n_r used in the stopping rule. `None`: the
    /// paper's worst case `sum_{i<=n_z} C(n_c, i)` (capped by `2^n_c`).
    pub nr_bound: Option<f64>,
    /// Samples examined per parallel batch (1 = serial). Does not change the result.
    pub batch: usize,
    /// Stop after this many regions (the paper's "maximum number of regions"); the guarantee
    /// then no longer applies.
    pub max_regions: Option<usize>,
    /// Algorithm 3 (outer approximation of Theta_f by projection cuts) instead of Algorithm 1.
    pub outer_approx: bool,
}

impl Default for MprOptions {
    fn default() -> Self {
        MprOptions { nr_bound: None, batch: 64, max_regions: None, outer_approx: true }
    }
}

/// Diagnostics of one run.
#[derive(Clone, Debug, Default)]
pub struct MprStats {
    pub n_required: usize,
    pub samples: usize,
    pub infeasible: usize,
    pub covered: usize,
    /// feasible, uncovered samples whose active set gave no full-dimensional region containing the sample
    pub failed_regions: usize,
    /// rows of the outer approximation of Theta_f at the end (including the rows of A_t)
    pub outer_rows: usize,
}

/// ln of the worst-case number of regions: ln sum_{i=0}^{nz} C(nc, i), at most nc ln 2.
pub fn ln_region_bound(nc: usize, nz: usize) -> f64 {
    let nz = nz.min(nc);
    let mut terms = Vec::with_capacity(nz + 1);
    let mut lc = 0.0f64; // ln C(nc, i)
    terms.push(0.0);
    for i in 1..=nz {
        lc += ((nc - i + 1) as f64).ln() - (i as f64).ln();
        terms.push(lc);
    }
    let mx = terms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    mx + terms.iter().map(|t| (t - mx).exp()).sum::<f64>().ln()
}

/// Number of consecutive non-productive samples required by Theorem 1.
pub fn required_samples(epsilon: f64, beta: f64, ln_nr: f64) -> usize {
    let n = (beta.ln() - ln_nr) / (1.0 - epsilon).ln();
    n.ceil().max(1.0) as usize
}

/// Point location over the regions found so far: a uniform grid over the sampling box D. Each cell
/// lists the regions that may meet it (a region is dropped from a cell when one of its rows is
/// violated at every point of the cell box), so a sample is tested against a few regions only.
/// Exactly the union test `any region: E th - f < 1e-9`, just with fewer candidates.
struct GridIndex {
    nt: usize,
    lo: Vec<f64>,
    inv_w: Vec<f64>,
    g: usize,
    /// per cell: ids of candidate regions
    cells: Vec<Vec<u32>>,
    /// per region: rows (flat) and rhs
    e: Vec<Vec<f64>>,
    f: Vec<Vec<f64>>,
}

impl GridIndex {
    fn new(lo: &[f64], hi: &[f64]) -> Self {
        let nt = lo.len();
        // about 1000 cells
        let g = ((1000f64).powf(1.0 / nt as f64).round() as usize).max(2);
        let ncell = g.pow(nt as u32);
        let inv_w = lo.iter().zip(hi).map(|(l, h)| g as f64 / (h - l).max(1e-300)).collect();
        GridIndex { nt, lo: lo.to_vec(), inv_w, g, cells: vec![vec![]; ncell], e: vec![], f: vec![] }
    }

    #[inline]
    fn cell_of(&self, th: &[f64]) -> usize {
        let mut c = 0usize;
        for k in 0..self.nt {
            let i = (((th[k] - self.lo[k]) * self.inv_w[k]) as isize).clamp(0, self.g as isize - 1) as usize;
            c = c * self.g + i;
        }
        c
    }

    fn len(&self) -> usize {
        self.e.len()
    }

    fn add(&mut self, r: &CriticalRegion) {
        let (nt, g) = (self.nt, self.g);
        let id = self.e.len() as u32;
        let rows = r.e.rows;
        let mut blo = vec![0.0f64; nt];
        let mut bhi = vec![0.0f64; nt];
        for cell in 0..self.cells.len() {
            // cell box
            let mut rem = cell;
            for k in (0..nt).rev() {
                let i = rem % g;
                rem /= g;
                blo[k] = self.lo[k] + i as f64 / self.inv_w[k];
                bhi[k] = self.lo[k] + (i + 1) as f64 / self.inv_w[k];
            }
            let mut disjoint = false;
            for i in 0..rows {
                let row = &r.e.data[i * nt..(i + 1) * nt];
                let mut mn = 0.0;
                for k in 0..nt {
                    mn += (row[k] * blo[k]).min(row[k] * bhi[k]);
                }
                // slack for the cell-boundary rounding of the clamped lookup
                if mn > r.f[i] + 1e-9 + 1e-9 * (1.0 + mn.abs()) {
                    disjoint = true;
                    break;
                }
            }
            if !disjoint {
                self.cells[cell].push(id);
            }
        }
        self.e.push(r.e.data.clone());
        self.f.push(r.f.clone());
    }

    /// Id of a region containing theta (up to tolerance).
    #[inline]
    fn locate(&self, th: &[f64]) -> Option<u32> {
        let nt = self.nt;
        for &id in &self.cells[self.cell_of(th)] {
            let (e, f) = (&self.e[id as usize], &self.f[id as usize]);
            let mut inside = true;
            for (i, &fi) in f.iter().enumerate() {
                let row = &e[i * nt..(i + 1) * nt];
                let mut s = 0.0;
                for k in 0..nt {
                    s += row[k] * th[k];
                }
                if s - fi >= 1e-9 {
                    inside = false;
                    break;
                }
            }
            if inside {
                return Some(id);
            }
        }
        None
    }
}


/// Polyhedral outer approximation of Theta_f (Algorithm 3, steps 14-15): the rows of
/// {A_t th <= b_t} plus one separating hyperplane per infeasible sample.
struct Outer {
    nt: usize,
    /// rows u (flat) and rhs: u'th <= rhs
    u: Vec<f64>,
    rhs: Vec<f64>,
}

impl Outer {
    #[inline]
    fn violated(&self, th: &[f64]) -> bool {
        let nt = self.nt;
        for (i, &r) in self.rhs.iter().enumerate() {
            let row = &self.u[i * nt..(i + 1) * nt];
            let mut s = 0.0;
            for k in 0..nt {
                s += row[k] * th[k];
            }
            if s > r {
                return true;
            }
        }
        false
    }

    fn push(&mut self, u: &[f64], r: f64) {
        self.u.extend_from_slice(u);
        self.rhs.push(r);
    }
}

/// Projection, Eq. (9): min 1/2 |th - th_bar|^2 over the (x, th) polytope. The x block gets a tiny
/// weight so the Hessian is positive definite for the dual active-set QP (x is the instrumental
/// variable of footnote 3); the cut is relaxed by a margin covering the perturbation this causes.
struct Projector {
    nx: usize,
    nt: usize,
    fac: Option<QpFactor>,
    q: Mat,
    big: Mat,
    bb: Vec<f64>,
    eq: Vec<usize>,
}

const PROJ_DELTA: f64 = 1e-9;
const CUT_MARGIN: f64 = 1e-6;

impl Projector {
    fn new(program: &MpProgram) -> Self {
        let (nx, nt) = (program.num_x(), program.num_t());
        let mut q = Mat::zeros(nx + nt, nx + nt);
        for i in 0..nx {
            q[(i, i)] = PROJ_DELTA;
        }
        for i in nx..nx + nt {
            q[(i, i)] = 1.0;
        }
        let (big, bb) = program.xtheta_polytope();
        let eq = program.equality_indices.clone();
        let fac = QpFactor::with_equalities(&q, &big, &eq);
        Projector { nx, nt, fac, q, big, bb, eq }
    }

    /// Separating cut (u, rhs) with u'th <= rhs valid on Theta_f and violated by `th_bar`.
    fn cut(&self, th_bar: &[f64]) -> Option<(Vec<f64>, f64)> {
        let fac = self.fac.as_ref()?;
        let mut c = vec![0.0; self.nx + self.nt];
        for k in 0..self.nt {
            c[self.nx + k] = -th_bar[k];
        }
        let sol = solve_qp_factored(fac, &self.q, Some(&c), &self.big, &self.bb, &self.eq)?;
        let th_hat = &sol.x[self.nx..];
        let mut u: Vec<f64> = (0..self.nt).map(|k| th_bar[k] - th_hat[k]).collect();
        let nrm = u.iter().map(|v| v * v).sum::<f64>().sqrt();
        if nrm < 1e-10 {
            return None;
        }
        u.iter_mut().for_each(|v| *v /= nrm);
        let scale = 1.0 + th_hat.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let rhs = u.iter().zip(th_hat).map(|(a, b)| a * b).sum::<f64>() + CUT_MARGIN * scale;
        // the sample must be cut off by the relaxed hyperplane as well
        let viol = u.iter().zip(th_bar).map(|(a, b)| a * b).sum::<f64>() > rhs;
        viol.then_some((u, rhs))
    }
}

enum Sample {
    /// outside the outer approximation of Theta_f (counts as infeasible)
    Outside,
    Covered,
    /// infeasible; carries a separating cut when the projection was computed
    Infeasible(Option<(Vec<f64>, f64)>),
    /// feasible and (as of the batch start) uncovered: a candidate region
    Candidate(Option<CriticalRegion>),
}

/// MPR with default options (Algorithm 3, uniform sampling over the bounding box of Theta).
pub fn solve_mpqp_mpr(program: &Arc<MpProgram>, epsilon: f64, beta: f64, seed: u64) -> Solution {
    solve_mpr_with(program, epsilon, beta, seed, &MprOptions::default()).0
}

/// MPR with options and diagnostics.
pub fn solve_mpr_with(program: &Arc<MpProgram>, epsilon: f64, beta: f64, seed: u64, opts: &MprOptions) -> (Solution, MprStats) {
    assert!(epsilon > 0.0 && epsilon < 1.0 && beta > 0.0 && beta < 1.0);
    let nt = program.num_t();
    let neq = program.num_equality_constraints();
    let nc = program.num_constraints() - neq;
    let nz = program.num_x().saturating_sub(neq);
    let ln_nr = match opts.nr_bound {
        Some(nr) => nr.max(1.0).ln(),
        None => ln_region_bound(nc, nz),
    };
    let n_req = required_samples(epsilon, beta, ln_nr);
    let mut stats = MprStats { n_required: n_req, ..Default::default() };

    let mut sol = Solution::new(program.clone(), vec![]);
    sol.is_overlapping = program.q.is_none();
    let Some((lo, hi)) = program.theta_bbox().cloned() else {
        return (sol, stats);
    };
    let projector = if opts.outer_approx { Some(Projector::new(program)).filter(|p| p.fac.is_some()) } else { None };
    let mut outer = Outer { nt, u: vec![], rhs: vec![] };
    for i in 0..program.a_t.rows {
        outer.push(program.a_t.row(i), program.b_t[i]);
    }

    let mut rng = StdRng::seed_from_u64(seed);
    let mut regions: Vec<CriticalRegion> = vec![];
    let mut index = GridIndex::new(&lo, &hi);
    let mut ns = 0usize;
    let batch = opts.batch.max(1);
    let mut thetas: Vec<f64> = vec![0.0; batch * nt];

    'outer: while ns < n_req {
        // never examine many more samples than can still be needed
        let b = batch.min(n_req - ns);
        for v in 0..b {
            for k in 0..nt {
                thetas[v * nt + k] = lo[k] + (hi[k] - lo[k]) * rng.r#gen::<f64>();
            }
        }
        let work = |v: usize| -> Sample {
            let th = &thetas[v * nt..(v + 1) * nt];
            if outer.violated(th) {
                return Sample::Outside;
            }
            if index.locate(th).is_some() {
                return Sample::Covered;
            }
            let sol = program.solve_theta_uncounted(th);
            match sol {
                None => {
                    let cut = projector.as_ref().and_then(|p| p.cut(th));
                    Sample::Infeasible(cut)
                }
                Some(out) => {
                    let cr = gen_cr_from_active_set(program, &out.active_set, true).filter(|r| r.is_inside(th, 1e-9));
                    Sample::Candidate(cr)
                }
            }
        };
        let results: Vec<Sample> = if b > 1 { (0..b).into_par_iter().map(work).collect() } else { vec![work(0)] };
        for (v, res) in results.into_iter().enumerate() {
            stats.samples += 1;
            let th = &thetas[v * nt..(v + 1) * nt];
            match res {
                Sample::Outside => {
                    stats.infeasible += 1;
                    ns += 1;
                }
                Sample::Covered => {
                    stats.covered += 1;
                    ns += 1;
                }
                Sample::Infeasible(cut) => {
                    if let Some((u, r)) = cut {
                        outer.push(&u, r);
                    }
                    stats.infeasible += 1;
                    ns += 1;
                }
                Sample::Candidate(cr) => {
                    // regions added earlier in this batch may now cover the sample
                    if index.locate(th).is_some() {
                        stats.covered += 1;
                        ns += 1;
                    } else if let Some(r) = cr {
                        index.add(&r);
                        regions.push(r);
                        ns = 0;
                        if opts.max_regions.is_some_and(|m| index.len() >= m) {
                            break 'outer;
                        }
                    } else {
                        stats.failed_regions += 1;
                        ns += 1;
                    }
                }
            }
            if ns >= n_req {
                break 'outer;
            }
        }
    }
    stats.outer_rows = outer.rhs.len();
    sol.critical_regions = regions;
    (sol, stats)
}
