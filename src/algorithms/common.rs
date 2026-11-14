//! Port of `ppopt.mp_solvers.solver_utils`.

use crate::critical_region::CriticalRegion;
use crate::linalg::{dot, is_full_rank_rows, rank, Cholesky, Mat};
use crate::mpqp_utils::gen_cr_from_active_set;
use crate::program::MpProgram;
use crate::solution::Solution;
use crate::utils::chebyshev_ball;
use crate::fxhash::FxHashSet as HashSet;

/// Fixed-width bitset over constraint indices.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BitSet(pub Vec<u64>);

impl BitSet {
    pub fn from_indices(idx: &[usize], m: usize) -> Self {
        let mut w = vec![0u64; m.div_ceil(64).max(1)];
        for &i in idx {
            w[i / 64] |= 1u64 << (i % 64);
        }
        BitSet(w)
    }

    #[inline]
    pub fn is_subset_of(&self, other: &BitSet) -> bool {
        self.0.iter().zip(&other.0).all(|(a, b)| a & !b == 0)
    }
}

/// Keeps every infeasible active set and rejects any superset of one (PPOPT `CombinationTester`).
/// Combos are also indexed by their largest element: a child `parent ∪ {i}` (i above every
/// element of a feasible parent) can only contain a combo whose largest element is `i`, so it is
/// tested against those alone, as "rest" masks (the combo without `i`) that must fit in `parent`.
#[derive(Clone, Default)]
pub struct CombinationTester {
    m: usize,
    combos: Vec<BitSet>,
    /// the same combos as u128 masks when m <= 128 (allocation-free checks)
    combos128: Vec<u128>,
    by_max: Vec<Vec<u32>>,
    /// per largest element, the other elements of each combo as a mask (m <= 64)
    rest64: Vec<Vec<u64>>,
    by_min: Vec<Vec<u32>>,
    has_empty: bool,
    seen: HashSet<Vec<usize>>,
}

impl CombinationTester {
    pub fn new(m: usize) -> Self {
        CombinationTester {
            m,
            combos: vec![],
            combos128: vec![],
            by_max: vec![vec![]; m],
            rest64: vec![vec![]; if m <= 64 { m } else { 0 }],
            by_min: vec![vec![]; m],
            has_empty: false,
            seen: HashSet::default(),
        }
    }

    /// false if `active_set` is a superset of a recorded infeasible set.
    pub fn check(&self, active_set: &[usize]) -> bool {
        if active_set.is_empty() {
            return true;
        }
        if self.has_empty {
            return false;
        }
        // a recorded combo inside `active_set` has its smallest element in `active_set`
        if self.m <= 128 {
            let key = active_set.iter().fold(0u128, |k, &i| k | (1u128 << i));
            return !active_set.iter().any(|&e| self.by_min[e].iter().any(|&c| self.combos128[c as usize] & !key == 0));
        }
        let bs = BitSet::from_indices(active_set, self.m);
        !active_set.iter().any(|&e| self.by_min[e].iter().any(|&c| self.combos[c as usize].is_subset_of(&bs)))
    }

    /// Same as `check` for `child = parent ∪ {new}` with `new` above every element of `parent`,
    /// when `parent` itself passed the check against all smaller combos and is not a recorded
    /// combo (true for children of feasible sets).
    #[inline]
    pub fn check_child(&self, child: &BitSet, new: usize) -> bool {
        !self.by_max[new].iter().any(|&c| self.combos[c as usize].is_subset_of(child))
    }

    /// `check_child` for m <= 64 with the parent as a mask.
    #[inline]
    pub fn check_child64(&self, parent: u64, new: usize) -> bool {
        let out = !parent;
        // branch-free OR over blocks (vectorizes), early exit between blocks
        for block in self.rest64[new].chunks(32) {
            let mut hit = 0u64;
            for &r in block {
                hit |= ((r & out) == 0) as u64;
            }
            if hit != 0 {
                return false;
            }
        }
        true
    }

    pub fn is_small(&self) -> bool {
        self.m <= 64
    }

    pub fn add_combo(&mut self, active_set: &[usize]) {
        let mut s = active_set.to_vec();
        s.sort_unstable();
        if self.seen.insert(s.clone()) {
            let id = self.combos.len() as u32;
            self.combos.push(BitSet::from_indices(&s, self.m));
            if self.m <= 128 {
                self.combos128.push(s.iter().fold(0u128, |k, &i| k | (1u128 << i)));
            }
            if let Some(&last) = s.last() {
                self.by_max[last].push(id);
                if self.m <= 64 {
                    self.rest64[last].push(s[..s.len() - 1].iter().fold(0u64, |k, &i| k | (1u64 << i)));
                }
            }
            match s.first() {
                Some(&first) => self.by_min[first].push(id),
                None => self.has_empty = true,
            }
        }
    }

    pub fn len(&self) -> usize {
        self.combos.len()
    }

    pub fn is_empty(&self) -> bool {
        self.combos.is_empty()
    }
}

/// All supersets [*active_set, i] with i > last element that survive the pruning list.
/// `active_set` must be a feasible set (or the equality set), as in the combinatorial algorithm.
pub fn generate_children_sets(active_set: &[usize], num_constraints: usize, murder: Option<&CombinationTester>) -> Vec<Vec<usize>> {
    let start = active_set.last().map(|v| v + 1).unwrap_or(0);
    let mut out = vec![];
    let mut child = active_set.to_vec();
    child.push(0);
    if let Some(m) = murder.filter(|m| m.is_small()) {
        let key = active_set.iter().fold(0u64, |k, &i| k | (1u64 << i));
        for i in start..num_constraints {
            if m.check_child64(key, i) {
                *child.last_mut().unwrap() = i;
                out.push(child.clone());
            }
        }
        return out;
    }
    let mut bits = BitSet::from_indices(active_set, num_constraints);
    for i in start..num_constraints {
        *child.last_mut().unwrap() = i;
        let ok = match murder {
            Some(m) => {
                bits.0[i / 64] |= 1u64 << (i % 64);
                let ok = m.check_child(&bits, i);
                bits.0[i / 64] &= !(1u64 << (i % 64));
                ok
            }
            None => true,
        };
        if ok {
            out.push(child.clone());
        }
    }
    out
}

/// Subsets with one element removed (equality indices must stay).
pub fn generate_reduce(
    candidate: &[usize],
    murder: Option<&CombinationTester>,
    attempted: Option<&HashSet<Vec<usize>>>,
    equality_set: &[usize],
) -> Vec<Vec<usize>> {
    let mut out = vec![];
    for &i in candidate {
        let possible: Vec<usize> = candidate.iter().cloned().filter(|&j| j != i).collect();
        let ok_attempt = attempted.map(|a| !a.contains(&possible)).unwrap_or(true);
        let ok_murder = murder.map(|m| m.check(&possible)).unwrap_or(true);
        if ok_attempt && ok_murder && equality_set.iter().all(|e| possible.contains(e)) {
            out.push(possible);
        }
    }
    out
}

/// Supersets with one regular constraint added.
pub fn generate_extra(
    candidate: &[usize],
    expansion: &[usize],
    murder: Option<&CombinationTester>,
    attempted: Option<&HashSet<Vec<usize>>>,
) -> Vec<Vec<usize>> {
    let mut out = vec![];
    for &r in expansion {
        let mut child = candidate.to_vec();
        child.push(r);
        child.sort_unstable();
        let ok_attempt = attempted.map(|a| !a.contains(&child)).unwrap_or(true);
        let ok_murder = murder.map(|m| m.check(&child)).unwrap_or(true);
        if ok_attempt && ok_murder {
            out.push(child);
        }
    }
    out
}

/// mpLP neighbours of a vertex active set `candidate` (size n_x): the bases reached by one pivot.
/// `entering` are constraints that become active across a primal facet (any basis member may
/// leave); `leaving` are members whose multiplier reaches zero across a dual facet (any
/// constraint of the program may enter). PPOPT reaches the first kind through the over-determined
/// set `candidate + r` and its reductions, but its pruning list rejects that set whenever it
/// contains a recorded dependent pair (for example the two bounds of one variable), which
/// disconnects degenerate mpLPs. Swaps are generated directly instead.
pub fn generate_lp_swaps(
    candidate: &[usize],
    entering: &[usize],
    leaving: &[usize],
    num_constraints: usize,
    equality: &[usize],
    murder: Option<&CombinationTester>,
    attempted: Option<&HashSet<Vec<usize>>>,
) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = vec![];
    let mut push = |base: &[usize], drop: usize, add: usize| {
        let mut child: Vec<usize> = base.iter().cloned().filter(|&j| j != drop).collect();
        child.push(add);
        child.sort_unstable();
        let ok_attempt = attempted.map(|a| !a.contains(&child)).unwrap_or(true);
        let ok_murder = murder.map(|m| m.check(&child)).unwrap_or(true);
        if ok_attempt && ok_murder && !out.contains(&child) {
            out.push(child);
        }
    };
    for &r in entering {
        for &i in candidate {
            if !equality.contains(&i) {
                push(candidate, i, r);
            }
        }
    }
    for &i in leaving {
        for j in 0..num_constraints {
            if !candidate.contains(&j) && !equality.contains(&i) {
                push(candidate, i, j);
            }
        }
    }
    out
}

/// For an over-determined active set, the greedy full-rank subset keeping the equalities.
pub fn find_sub_active_set(program: &MpProgram, active_set: &[usize]) -> Vec<usize> {
    let eq = &program.equality_indices;
    let ineq: Vec<usize> = active_set.iter().cloned().filter(|i| !eq.contains(i)).collect();
    let mut kept: Vec<usize> = vec![];
    let mut current_rank = if eq.is_empty() { 0 } else { rank(&program.a.select_rows(eq)) };
    for i in ineq {
        let mut trial: Vec<usize> = eq.clone();
        trial.extend(kept.iter().cloned());
        trial.push(i);
        let r = rank(&program.a.select_rows(&trial));
        if r > current_rank {
            kept.push(i);
            current_rank = r;
        }
        if current_rank == program.num_x() {
            break;
        }
    }
    let mut out = eq.clone();
    out.extend(kept);
    out
}

/// (Chebyshev center, normal, radius) of every facet of {x : A x <= b}.
pub fn get_facet_centers(a: &Mat, b: &[f64]) -> Vec<(Vec<f64>, Vec<f64>, f64)> {
    get_facet_centers_degen(a, b, None)
}

/// Is there another row of [A | b] on the same hyperplane as row `k` (a duplicated facet)?
fn has_coincident_row(a: &Mat, b: &[f64], k: usize) -> bool {
    let nk = crate::linalg::norm(a.row(k));
    (0..a.rows).any(|j| {
        let nj = crate::linalg::norm(a.row(j));
        j != k && (0..a.cols).all(|c| (a[(k, c)] / nk - a[(j, c)] / nj).abs() < 1e-9) && (b[k] / nk - b[j] / nj).abs() < 1e-9
    })
}

/// `get_facet_centers` that flags in `degen` a facet skipped because another row of the region
/// lies on the same hyperplane (two constraints becoming active together: degenerate).
pub fn get_facet_centers_degen(a: &Mat, b: &[f64], degen: Option<&std::sync::atomic::AtomicBool>) -> Vec<(Vec<f64>, Vec<f64>, f64)> {
    let mut out = vec![];
    for k in 0..a.rows {
        let (theta, radius) = if a.cols == 1 {
            (vec![b[k] / a[(k, 0)]], 1.0)
        } else {
            crate::stats::lp(1);
            match chebyshev_ball(a, b, &[k]) {
                Some(s) => (s.x[..a.cols].to_vec(), s.x[a.cols]),
                None => continue,
            }
        };
        if radius.abs() <= 1e-12 {
            if let Some(d) = degen {
                if has_coincident_row(a, b, k) {
                    d.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            continue;
        }
        out.push((theta, a.row(k).to_vec(), radius));
    }
    out
}

/// Steps out of a facet until a new critical region is found (PPOPT `fathem_facet`).
pub fn fathem_facet(
    center: &[f64],
    normal: &[f64],
    radius: f64,
    program: &MpProgram,
    indexed: &HashSet<Vec<usize>>,
    current_active_set: &[usize],
    cand_sol: Option<&Solution>,
) -> Option<CriticalRegion> {
    fathem_facet_claim(center, normal, radius, program, indexed, None, current_active_set, cand_sol)
}

/// `fathem_facet` for parallel waves: `claimed` holds active sets another facet of the same wave
/// is already building, which are treated like indexed regions (the region is built once).
#[allow(clippy::too_many_arguments)]
pub fn fathem_facet_claim(
    center: &[f64],
    normal: &[f64],
    radius: f64,
    program: &MpProgram,
    indexed: &HashSet<Vec<usize>>,
    claimed: Option<&std::sync::Mutex<HashSet<Vec<usize>>>>,
    current_active_set: &[usize],
    cand_sol: Option<&Solution>,
) -> Option<CriticalRegion> {
    fathem_facet_degen(center, normal, radius, program, indexed, claimed, current_active_set, cand_sol, None)
}

/// Deterministic pseudo-random weight of constraint `j` (splitmix64), in (0, 1). Generic weights
/// select a regular triangulation of the multiplier cone at a degenerate vertex.
fn weight(j: usize) -> f64 {
    let mut z = (j as u64).wrapping_add(0x9E37_79B9_7F4A_7C15).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 30;
    z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 27;
    ((z >> 11) as f64 + 0.5) / (1u64 << 53) as f64
}

fn combos_into(items: &[usize], k: usize, start: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>, cap: usize) {
    if out.len() >= cap {
        return;
    }
    if cur.len() == k {
        out.push(cur.clone());
        return;
    }
    for i in start..items.len() {
        if items.len() - i < k - cur.len() {
            break;
        }
        cur.push(items[i]);
        combos_into(items, k, i + 1, cur, out, cap);
        cur.pop();
    }
}

/// Canonical basis of a degenerate (dependent or over-determined) active set at `theta`.
///
/// The optimal multipliers at a degenerate point are the polyhedron {l >= 0 : A_S' l = g} and the
/// critical region of a basis B is the set of theta with an optimal multiplier supported on B, so
/// different bases give overlapping regions (the facet-to-facet property fails; Spjotvold, Tondel,
/// Johansen 2006). The basis is therefore the optimal basis of min w'l over that polyhedron for a
/// fixed generic weight w (Bemporad 2015 / Jones-Morari 2006 use the same idea in lexicographic
/// form): it is primal feasible at theta (l_B >= 0) and dual feasible (reduced costs
/// w_j - w_B' A_B^{-T} a_j >= 0, independent of theta). Dual feasible bases are the cells of a
/// regular triangulation of the cone, so every theta has exactly one and the regions are disjoint.
pub fn canonical_basis(program: &MpProgram, active: &[usize], theta: &[f64]) -> Option<Vec<usize>> {
    let eq = &program.equality_indices;
    let ineq: Vec<usize> = active.iter().cloned().filter(|i| !eq.contains(i)).collect();
    let mut all: Vec<usize> = eq.clone();
    all.extend(ineq.iter().cloned());
    let r = rank(&program.a.select_rows(&all));
    if r < eq.len() {
        return None;
    }
    let k = r - eq.len();
    let mut cands = vec![];
    combos_into(&ineq, k, 0, &mut vec![], &mut cands, 20_000);
    let neq = eq.len();
    for c in cands {
        let mut b: Vec<usize> = eq.clone();
        b.extend(c);
        if !is_full_rank_rows(&program.a, &b) {
            continue;
        }
        // reduced costs of the dependent active constraints (independent of theta)
        let ab = program.a.select_rows(&b);
        let g = ab.matmul(&ab.transpose());
        let Some(ch) = Cholesky::new(&g) else { continue };
        let wb: Vec<f64> = b.iter().map(|&i| weight(i)).collect();
        let mut ok = true;
        for &j in &ineq {
            if b.contains(&j) {
                continue;
            }
            let aj = program.a.row(j);
            let y = ch.solve(&ab.matvec(aj));
            let rc = weight(j) - dot(&wb, &y);
            if rc < -1e-10 {
                ok = false;
                break;
            }
        }
        if !ok {
            continue;
        }
        let Some(law) = program.optimal_control_law(&b) else { continue };
        let lam_ok = (neq..law.a_l.rows).all(|i| dot(law.a_l.row(i), theta) + law.b_l[i] >= -1e-9);
        if lam_ok {
            return Some(b);
        }
    }
    None
}

/// `fathem_facet_claim` that records in `degen` that a degenerate (over-determined or
/// rank-deficient) active set was met, and resolves such sets with `canonical_basis`.
#[allow(clippy::too_many_arguments)]
pub fn fathem_facet_degen(
    center: &[f64],
    normal: &[f64],
    radius: f64,
    program: &MpProgram,
    indexed: &HashSet<Vec<usize>>,
    claimed: Option<&std::sync::Mutex<HashSet<Vec<usize>>>>,
    current_active_set: &[usize],
    cand_sol: Option<&Solution>,
    degen: Option<&std::sync::atomic::AtomicBool>,
) -> Option<CriticalRegion> {
    let mut dist = radius * 1e-6;
    while dist < radius {
        dist *= 2.0;
        let test: Vec<f64> = center.iter().zip(normal).map(|(c, n)| c + n * dist).collect();
        // PPOPT first scans every found region for the test point; that scan is only decisive when
        // the step would otherwise build a new region, so it is done there (below). In the other
        // outcomes (infeasible, known active set) PPOPT also returns None.
        // counted for PPOPT only where PPOPT certainly solves it too (infeasible, or a new region)
        let count = || if program.is_qp() { crate::stats::qp() } else { crate::stats::lp(1) };
        let Some(active) = program.active_set_at(&test) else {
            if program.valid_parameter_realization(&test) {
                count();
            }
            return None;
        };
        let degenerate = active.len() > program.num_x();
        let mut projected = active.clone();
        if degenerate {
            projected = find_sub_active_set(program, &projected);
            if let Some(d) = degen {
                d.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            if let Some(b) = canonical_basis(program, &active, &test) {
                projected = b;
            }
        }
        if projected == current_active_set {
            continue;
        }
        if indexed.contains(&projected) {
            return None;
        }
        crate::stats::rank();
        if !program.check_active_set_rank(&projected) {
            if degenerate {
                continue;
            }
            // rank-deficient (LICQ violating) active set with at most n_x constraints
            if let Some(d) = degen {
                d.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            match canonical_basis(program, &active, &test) {
                Some(b) if b != current_active_set => {
                    if indexed.contains(&b) {
                        return None;
                    }
                    projected = b;
                }
                _ => continue,
            }
        }
        if let Some(s) = cand_sol {
            if s.critical_regions.iter().any(|r| r.is_inside(&test, 0.0)) {
                return None;
            }
        }
        count();
        if let Some(c) = claimed {
            if !c.lock().unwrap().insert(projected.clone()) {
                return None; // another facet of this wave is building it
            }
        }
        if let Some(cr) = gen_cr_from_active_set(program, &projected, true) {
            return Some(cr);
        }
        if let Some(c) = claimed {
            c.lock().unwrap().remove(&projected);
        }
    }
    None
}
