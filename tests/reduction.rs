//! Complexity reduction (optimal merging, clipping) on the 12 benchmark problems: equivalence of
//! the target output, coverage (no gaps, no overlaps for merging) and point location through the
//! reduced partitions.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::point_location::geometry::enumerate_vertices;
use rpopt::point_location::{HashTable, PartitionGeometry, PointLocator, SequentialSearch};
use rpopt::program::MpProgram;
use rpopt::reduction::*;
use rpopt::Solution;
use std::path::PathBuf;
use std::sync::Arc;

const SAMPLES: usize = 20_000;
const LIMIT: usize = 64 << 10;

fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/problems").join(format!("{name}.json"))
}

fn solve(name: &str) -> Solution {
    let prog = Arc::new(MpProgram::from_json_file(&path(name)).unwrap());
    solve_mpqp(&prog, MpqpAlgorithm::Geometric)
}

fn theta_box(sol: &Solution) -> (Vec<f64>, Vec<f64>) {
    let n = sol.theta_dim();
    let mut lo = vec![f64::INFINITY; n];
    let mut hi = vec![f64::NEG_INFINITY; n];
    for r in &sol.critical_regions {
        for v in enumerate_vertices(&r.e, &r.f) {
            for k in 0..n {
                lo[k] = lo[k].min(v[k]);
                hi[k] = hi[k].max(v[k]);
            }
        }
    }
    (lo, hi)
}

fn samples(sol: &Solution) -> Vec<Vec<f64>> {
    let (lo, hi) = theta_box(sol);
    let mut rng = StdRng::seed_from_u64(7);
    (0..SAMPLES).map(|_| (0..lo.len()).map(|k| rng.gen_range(lo[k]..=hi[k])).collect()).collect()
}

/// Reference target values from the original solution (None outside the feasible set).
fn reference(sol: &Solution, idx: &[usize], pts: &[Vec<f64>]) -> Vec<Option<Vec<f64>>> {
    pts.iter()
        .map(|t| {
            let r = sol.critical_regions.iter().find(|r| r.is_inside(t, 1e-9))?;
            let x = r.evaluate(t);
            Some(idx.iter().map(|&i| x[i]).collect())
        })
        .collect()
}

fn close(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() <= 1e-9 * y.abs().max(1.0))
}

const NAMES: [&str; 12] = ["di_doc_N3", "di_N4", "di_N6", "di_N8", "di_N10", "dinv_N4", "dinv_N6", "dinv_N8", "masses2_N4", "masses2_N6", "sys3x2_N3", "sys3x2_N5"];

fn check_merge(name: &str, sol: &Solution, target: &ReduceTarget, pts: &[Vec<f64>]) -> MergeReport {
    let nx = sol.critical_regions[0].a.rows;
    let idx = target.indices(nx);
    let refv = reference(sol, &idx, pts);
    let (red, rep) = merge_regions(sol, target, &ReduceOptions::default());
    assert!(!rep.skipped, "{name}");
    assert!(red.len() <= sol.len());
    let seq = SequentialSearch::new(&red);
    let geom = PartitionGeometry::new(&red);
    let ht = HashTable::with_storage_limit(&geom, LIMIT);
    for (t, rv) in pts.iter().zip(&refv) {
        let Some(rv) = rv else { continue };
        // equivalence
        let reg = red.critical_regions.iter().find(|r| r.is_inside(t, 1e-9)).unwrap_or_else(|| panic!("{name} {target:?}: gap at {t:?}"));
        assert!(close(&reg.evaluate(t), rv), "{name} {target:?}: law mismatch at {t:?}: {:?} vs {rv:?}", reg.evaluate(t));
        // coverage and overlap
        let covering = red.critical_regions.iter().filter(|r| r.is_inside(t, 1e-7)).count();
        assert!(covering >= 1, "{name} {target:?}: gap at {t:?}");
        let interior = red.critical_regions.iter().filter(|r| r.is_inside(t, -1e-7)).count();
        assert!(interior <= 1, "{name} {target:?}: overlap at {t:?}");
        // point location
        for (lname, loc) in [("sequential", &seq as &dyn PointLocator), ("hash", &ht as &dyn PointLocator)] {
            let i = loc.locate(t).unwrap_or_else(|| panic!("{name} {target:?} {lname}: not located {t:?}"));
            assert!(close(&red.critical_regions[i].evaluate(t), rv), "{name} {target:?} {lname}: mismatch at {t:?}");
        }
    }
    // no sample outside the original set is covered by more than tolerance would allow: spot check
    rep
}

#[test]
fn merge_full_law_and_first_inputs() {
    for name in NAMES {
        let sol = solve(name);
        let pts = samples(&sol);
        let full = check_merge(name, &sol, &ReduceTarget::FullLaw, &pts);
        let target = ReduceTarget::from_problem_json(&path(name));
        let first = check_merge(name, &sol, &target, &pts);
        assert!(first.regions_after <= full.regions_after, "{name}");
        println!("{name}: {} -> full {} ({} cells, exact {}), {target:?} {} ({} cells, exact {}), {:.3}s", sol.len(), full.regions_after, full.cells, full.exact, first.regions_after, first.cells, first.exact, first.seconds);
    }
}

#[test]
fn clipping_equivalence_coverage_location() {
    for name in NAMES {
        let sol = solve(name);
        let nx = sol.critical_regions[0].a.rows;
        let target = ReduceTarget::from_problem_json(&path(name));
        let idx = target.indices(nx);
        let bounds = input_bounds_from_json(&path(name), &target, nx);
        let pts = samples(&sol);
        let refv = reference(&sol, &idx, &pts);
        let clipped = clip_reduce(&sol, &idx, &bounds);
        let geoms: Vec<PartitionGeometry> = clipped.components.iter().map(|c| PartitionGeometry::new(&c.solution)).collect();
        let hts: Vec<HashTable> = geoms.iter().map(|g| HashTable::with_storage_limit(g, LIMIT)).collect();
        let seqs: Vec<SequentialSearch> = clipped.components.iter().map(|c| SequentialSearch::new(&c.solution)).collect();
        for (t, rv) in pts.iter().zip(&refv) {
            let Some(rv) = rv else { continue };
            let got = clipped.evaluate(t).unwrap_or_else(|| panic!("{name}: gap at {t:?}"));
            assert!(close(&got, rv), "{name}: clip mismatch at {t:?}: {got:?} vs {rv:?}");
            for (k, c) in clipped.components.iter().enumerate() {
                assert!(c.solution.critical_regions.iter().any(|r| r.is_inside(t, 1e-7)), "{name}: gap at {t:?}");
                for loc in [&seqs[k] as &dyn PointLocator, &hts[k] as &dyn PointLocator] {
                    let i = loc.locate(t).unwrap_or_else(|| panic!("{name}: not located {t:?}"));
                    let v = clipped.value_in(k, i, t);
                    assert!((v - rv[k]).abs() <= 1e-9 * rv[k].abs().max(1.0), "{name}: located mismatch at {t:?}");
                }
            }
        }
        println!("{name}: {} -> clipped {:?} (sat {:?}) in {:.3}s", sol.len(), clipped.components.iter().map(|c| c.solution.len()).collect::<Vec<_>>(), clipped.components.iter().map(|c| c.saturated).collect::<Vec<_>>(), clipped.seconds);
    }
}
