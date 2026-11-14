//! Every point location method must return exactly the region of `Solution::get_region_index`
//! (strict-first rule: the first region that contains theta with tolerance 0, otherwise the first
//! within the point location tolerance). The tolerance is the one default of the crate
//! (`Solution::new` gives `LOCATE_TOL`), with no special casing: locators are built from the
//! solution as it comes out of the solver. Checked on theta dimension 6 and 8 with at least
//! 20,000 queries each: uniform points in the parameter box and points within +-2 tolerances of
//! a facet (both sides of the tolerance boundary), which is where first-within-tolerance and
//! strict-first rules differ. A second run repeats this at PPOPT's tolerance 1e-5.
//!
//! Also holds the regression tests for the point location defects found by
//! `examples/pl_dim_sweep.rs` (thin regions dropped by an unreliable LP radius check).

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::linalg::dot;
use rpopt::point_location::bst::BstOptions;
use rpopt::point_location::*;
use rpopt::program::MpProgram;
use rpopt::solution::Solution;
use std::sync::Arc;

fn theta_box(prog: &MpProgram) -> (Vec<f64>, Vec<f64>) {
    let n = prog.num_t();
    let (mut lo, mut hi) = (vec![-1.0; n], vec![1.0; n]);
    for k in 0..n {
        let mut c = vec![0.0; n];
        c[k] = 1.0;
        if let Some(s) = rpopt::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
            lo[k] = s.obj;
        }
        c[k] = -1.0;
        if let Some(s) = rpopt::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
            hi[k] = -s.obj;
        }
    }
    (lo, hi)
}

/// Uniform queries plus queries within a few tolerances of facets of random regions.
fn queries(prog: &MpProgram, geom: &PartitionGeometry, count: usize, seed: u64) -> Vec<Vec<f64>> {
    let t = geom.tol;
    let n = geom.dim;
    let (lo, hi) = theta_box(prog);
    let mut rng = StdRng::seed_from_u64(seed);
    let mut qs: Vec<Vec<f64>> = (0..count / 2).map(|_| (0..n).map(|k| rng.gen_range(lo[k]..hi[k])).collect()).collect();
    let offsets = [-100.0 * t, -2.0 * t, -t, -0.5 * t, -0.1 * t, -0.01 * t, 0.01 * t, 0.1 * t, 0.5 * t, 0.9 * t, 1.1 * t, 2.0 * t, 100.0 * t];
    while qs.len() < count {
        let r = rng.gen_range(0..geom.num_regions);
        let (e, f) = geom.region_rows(r);
        if f.is_empty() {
            continue;
        }
        let k = rng.gen_range(0..f.len());
        let a = &e[k * n..(k + 1) * n];
        // vertices on the facet (rows are normalized, so the residual is a distance)
        let on: Vec<&Vec<f64>> = geom.vertices[r].iter().filter(|v| (dot(a, v) - f[k]).abs() < 1e-9).collect();
        let mut x = if on.len() >= 2 {
            let ws: Vec<f64> = (0..on.len()).map(|_| rng.gen_range(0.01..1.0)).collect();
            let s: f64 = ws.iter().sum();
            let mut x = vec![0.0; n];
            for (w, v) in ws.iter().zip(&on) {
                for j in 0..n {
                    x[j] += w / s * v[j];
                }
            }
            x
        } else {
            let c = &geom.centers[r];
            let slack = f[k] - dot(a, c);
            (0..n).map(|j| c[j] + slack * a[j]).collect()
        };
        let d = offsets[rng.gen_range(0..offsets.len())];
        for j in 0..n {
            x[j] += d * a[j];
        }
        qs.push(x);
    }
    qs
}

/// Largest row violation of region j at q (rows of `Solution`, not normalized).
fn violation(sol: &Solution, j: usize, q: &[f64]) -> f64 {
    let r = &sol.critical_regions[j];
    (0..r.e.rows).map(|k| dot(r.e.row(k), q) - r.f[k]).fold(f64::NEG_INFINITY, f64::max)
}

/// Exact boundary tie: both regions contain theta strictly and theta is within rounding error
/// (1e-13) of both facets, so the sign of the violation is noise and no rule can order them.
fn roundoff_tie(sol: &Solution, got: Option<usize>, want: Option<usize>, q: &[f64]) -> bool {
    match (got, want) {
        (Some(g), Some(w)) => {
            let (vg, vw) = (violation(sol, g, q), violation(sol, w, q));
            vg <= 0.0 && vw <= 0.0 && vg >= -1e-13 && vw >= -1e-13
        }
        _ => false,
    }
}

fn agree(name: &str, sol: &Solution, locs: &[Box<dyn PointLocator>], qs: &[Vec<f64>]) {
    let want: Vec<Option<usize>> = qs.iter().map(|q| sol.get_region_index(q)).collect();
    let inside = want.iter().filter(|w| w.is_some()).count();
    assert!(inside > qs.len() / 4, "{name}: only {inside} of {} queries inside the feasible set", qs.len());
    for loc in locs {
        let mut bad = vec![];
        let mut ties = 0usize;
        for (q, w) in qs.iter().zip(&want) {
            let got = loc.locate(q);
            if got != *w {
                if roundoff_tie(sol, got, *w, q) {
                    ties += 1;
                } else {
                    bad.push((got, *w, q.clone()));
                }
            }
        }
        if !bad.is_empty() {
            let (got, want, q) = &bad[0];
            let v = |j: &Option<usize>| j.map(|j| violation(sol, j, q));
            panic!("{name}: {} disagrees with Solution::get_region on {} of {} queries, first: got {:?} (violation {:?}) want {:?} (violation {:?}) at {:?}", loc.name(), bad.len(), qs.len(), got, v(got), want, v(want), q);
        }
        assert!(ties <= 5, "{name}: {} has {ties} rounding-level ties", loc.name());
    }
}

fn all_methods(sol: &Solution, geom: &PartitionGeometry) -> Vec<Box<dyn PointLocator>> {
    let mut locs: Vec<Box<dyn PointLocator>> = vec![
        Box::new(SequentialSearch::new(sol)),
        Box::new(FlatSequential::new(sol)),
        Box::new(BinarySearchTree::with_options(geom, &BstOptions { max_nodes: 100_000, ..Default::default() })),
        Box::new(OptimizedTree::with_options(geom, &OptTreeOptions { max_seconds: 8.0, ..Default::default() })),
        Box::new(OptimizedTree::with_options(geom, &OptTreeOptions { max_seconds: 8.0, candidates: Candidates::Facets, ..Default::default() })),
        Box::new(OptimizedTree::with_options(geom, &OptTreeOptions { max_seconds: 4.0, candidates: Candidates::Axes, leaf_size: 3, ..Default::default() })),
        Box::new(HashTable::with_storage_limit(geom, 8 << 20)),
        Box::new(SubdivisionWalk::new(geom)),
        Box::new(GraphTraversal::new(geom)),
    ];
    if let Some(lm) = LinearMachine::new(geom) {
        locs.push(Box::new(lm));
    }
    let bst = BinarySearchTree::with_options(geom, &BstOptions { max_nodes: 100_000, ..Default::default() });
    let mut rows = vec![];
    let n = geom.dim;
    for i in 0..geom.num_regions {
        let (e, f) = geom.region_rows(i);
        for k in 0..f.len() {
            rows.extend_from_slice(&e[k * n..(k + 1) * n]);
            rows.push(f[k]);
        }
    }
    locs.push(Box::new(MultiwayTree::from_bst(&bst, 3, rows, geom.offsets.clone())));
    locs
}

fn run(name: &str, prog: Arc<MpProgram>, count: usize) {
    run_tol(name, prog, count, None);
}

/// `tol`: None keeps the default of `Solution::new`.
fn run_tol(name: &str, prog: Arc<MpProgram>, count: usize, tol: Option<f64>) {
    let mut sol = solve_mpqp(&prog, MpqpAlgorithm::GeometricParallel);
    match tol {
        None => assert_eq!(sol.point_location_tolerance, LOCATE_TOL),
        Some(t) => sol.point_location_tolerance = t,
    }
    let geom = PartitionGeometry::new(&sol);
    let qs = queries(&prog, &geom, count, 17);
    let locs = all_methods(&sol, &geom);
    agree(name, &sol, &locs, &qs);
    // PPOPT's first-within-tolerance rule stays available: it must still return a region that
    // contains theta within the tolerance
    let ppopt = SequentialSearch::ppopt(&sol);
    let flat = FlatSequential::new(&sol);
    for q in &qs {
        match (ppopt.locate(q), sol.get_region_index(q)) {
            (None, None) => {}
            (Some(j), Some(_)) => assert!(flat.region_contains(j, q), "{name}: PPOPT-rule search returned a region that does not contain theta"),
            (got, want) => panic!("{name}: PPOPT-rule search {got:?} vs {want:?}"),
        }
    }
}

#[test]
fn all_methods_agree_with_solution_theta6() {
    run("qp:3:6:14:1", Arc::new(rpopt::problem_generator::generate_mpqp(3, 6, 14, 1)), 24_000);
}

/// PPOPT's own tolerance, set on the solution: every locator follows it.
#[test]
fn all_methods_agree_with_solution_at_ppopt_tolerance() {
    run_tol("qp:3:6:14:1 tol 1e-5", Arc::new(rpopt::problem_generator::generate_mpqp(3, 6, 14, 1)), 24_000, Some(1e-5));
}

#[test]
fn all_methods_agree_with_solution_theta8() {
    run("qp:3:8:10:1", Arc::new(rpopt::problem_generator::generate_mpqp(3, 8, 10, 1)), 24_000);
}

/// qp:3:8:10:1 has a region of width 2.5e-4 at coordinates of magnitude 38 (slab along the upper
/// bound of theta). The Chebyshev LP that decides "does this region meet the cell with positive
/// volume" returned radius 0 for some cells that contain interior points of it, and the hash table
/// dropped the region from those cells: `locate` returned None for points strictly inside it.
#[test]
fn hash_table_keeps_thin_region_theta8() {
    let prog = Arc::new(rpopt::problem_generator::generate_mpqp(3, 8, 10, 1));
    let sol = solve_mpqp(&prog, MpqpAlgorithm::GeometricParallel);
    let geom = PartitionGeometry::new(&sol);
    let thetas: [[f64; 8]; 4] = [
        [34.80534466451739, 0.4775783571946002, 37.819863556682556, -4.094729937941457, 20.93323009248409, -7.019987216301264, 0.5104307985038137, 3.075497883138876],
        [26.77623638227597, 0.7021604983991092, 27.67175396181211, -2.7021788165957616, 10.593281716498993, -16.20900754309671, -6.76167713639348, 9.231040402895328],
        [4.139624372726168, 2.209461823653964, 21.968896927705792, 2.693726778218621, 2.3226761780582534, -37.81986355668256, 24.0968119574092, 17.250191055335087],
        [26.527349801148617, 2.9814144400704015, 33.26088831570242, 3.3744347710692972, -1.668756671399801, 7.713471641930731, -15.779196366438088, -9.097791073267405],
    ];
    let tables = [HashTable::with_storage_limit(&geom, 64 << 20), HashTable::with_grid(&geom, &[4; 8])];
    for th in &thetas {
        let want = sol.get_region_index(th);
        assert!(want.is_some());
        for ht in &tables {
            assert_eq!(ht.locate(th), want, "hash table lost a thin region at {th:?}");
        }
    }
}

/// The hash table used to panic above 8 parameters.
#[test]
fn hash_table_supports_more_than_eight_parameters() {
    let prog = Arc::new(rpopt::problem_generator::generate_mpqp(3, 10, 10, 1));
    let sol = solve_mpqp(&prog, MpqpAlgorithm::GeometricParallel);
    let geom = PartitionGeometry::new(&sol);
    assert!(HashTable::supports_dim(geom.dim));
    let ht = HashTable::try_with_storage_limit(&geom, 8 << 20).unwrap();
    let qs = queries(&prog, &geom, 4000, 3);
    let bad = qs.iter().filter(|q| ht.locate(q) != sol.get_region_index(q)).count();
    assert_eq!(bad, 0);
    assert!(!HashTable::supports_dim(17));
    assert!(!HashTable::supports_dim(0));
}
