//! Optimized decision trees (and the probability-weighted BST) must agree with sequential search:
//! same inside/outside answer, and the returned region passes the f64 halfspace test. Checked on
//! uniform queries, points inside regions and random walks, for every benchmark problem and a few
//! random mpQPs of higher parameter dimension.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::point_location::bst::BstOptions;
use rpopt::point_location::*;
use rpopt::program::MpProgram;
use std::path::PathBuf;
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

fn check(name: &str, prog: Arc<MpProgram>) {
    let sol = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
    let geom = PartitionGeometry::new(&sol);
    let seq = FlatSequential::new(&sol);
    let (lo, hi) = theta_box(&prog);
    let nt = lo.len();
    let mut rng = StdRng::seed_from_u64(5);
    let mut qs: Vec<Vec<f64>> = (0..4000).map(|_| (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect()).collect();
    for _ in 0..4000 {
        let r = rng.gen_range(0..geom.num_regions);
        let vs = &geom.vertices[r];
        let mut x = geom.centers[r].clone();
        if !vs.is_empty() {
            let v = &vs[rng.gen_range(0..vs.len())];
            let t: f64 = rng.gen_range(0.0..1.0);
            for k in 0..nt {
                x[k] += t * (v[k] - x[k]);
            }
        }
        qs.push(x);
    }
    let mut x = geom.centers[0].clone();
    for _ in 0..2000 {
        let y: Vec<f64> = (0..nt).map(|k| (x[k] + rng.gen_range(-0.02..0.02) * (hi[k] - lo[k])).clamp(lo[k], hi[k])).collect();
        if seq.locate(&y).is_some() {
            x = y;
        }
        qs.push(x.clone());
    }
    let weights: Vec<f64> = (0..geom.num_regions).map(|i| 1.0 + (i % 7) as f64).collect();
    let trees: Vec<Box<dyn PointLocator>> = vec![
        Box::new(OptimizedTree::new(&geom)),
        Box::new(OptimizedTree::with_options(&geom, &OptTreeOptions { weights: Some(weights.clone()), ..Default::default() })),
        Box::new(OptimizedTree::with_options(&geom, &OptTreeOptions { candidates: Candidates::Facets, ..Default::default() })),
        Box::new(OptimizedTree::with_options(&geom, &OptTreeOptions { candidates: Candidates::Axes, leaf_size: 3, ..Default::default() })),
        Box::new(OptimizedTree::with_options(&geom, &OptTreeOptions { criterion: Criterion::ExpDepth, exact_refinement: false, ..Default::default() })),
        Box::new(BinarySearchTree::with_options(&geom, &BstOptions { weights: Some(weights), ..Default::default() })),
    ];
    for t in &trees {
        let mut bad = 0;
        let mut inside = 0;
        for q in &qs {
            let want = seq.locate(q);
            inside += want.is_some() as usize;
            let ok = match (t.locate(q), want) {
                (None, None) => true,
                (Some(j), Some(_)) => seq.region_contains(j, q),
                _ => false,
            };
            bad += !ok as usize;
        }
        assert_eq!(bad, 0, "{name}: {} disagrees with sequential on {bad} of {} queries", t.name(), qs.len());
        assert!(inside > 100, "{name}: too few queries inside the feasible set");
    }
}

#[test]
fn opt_tree_matches_sequential_on_benchmark_problems() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/problems");
    let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map(|x| x == "json").unwrap_or(false)).map(|p| p.file_stem().unwrap().to_string_lossy().to_string()).collect();
    names.sort();
    assert_eq!(names.len(), 12);
    for n in names {
        check(&n, Arc::new(MpProgram::from_json_file(&dir.join(format!("{n}.json"))).unwrap()));
    }
}

#[test]
fn opt_tree_matches_sequential_on_random_mpqps() {
    for (x, t, m, seed) in [(3, 5, 12, 1u64), (4, 6, 14, 1), (3, 8, 10, 1)] {
        check(&format!("qp:{x}:{t}:{m}:{seed}"), Arc::new(rpopt::problem_generator::generate_mpqp(x, t, m, seed)));
    }
}
