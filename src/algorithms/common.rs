//! Port of `ppopt.mp_solvers.solver_utils`.

use crate::critical_region::CriticalRegion;
use crate::linalg::{rank, Mat};
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
        let mut projected = active;
        if projected.len() > program.num_x() {
            projected = find_sub_active_set(program, &projected);
        }
        if projected == current_active_set {
            continue;
        }
        if indexed.contains(&projected) {
            return None;
        }
        crate::stats::rank();
        if !program.check_active_set_rank(&projected) {
            continue;
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
