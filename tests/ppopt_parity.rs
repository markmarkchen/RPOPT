//! Parity tests ported from PPOPT's own test suite (tests/test_fixtures.py, other_tests/test_solve_mpqp.py,
//! mpmiqp_solver_tests/test_mpmiqp.py), plus cross-checks of the point location methods.

use rpopt::algorithms::mpmiqp::{solve_mpmiqp_enumeration, MpmiProgram};
use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm, SolveOptions};
use rpopt::linalg::{allclose, Mat};
use rpopt::point_location::*;
use rpopt::program::MpProgram;
use rpopt::problem_generator::generate_mpqp;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;

fn rows(v: &[&[f64]]) -> Mat {
    Mat::from_rows(&v.iter().map(|r| r.to_vec()).collect::<Vec<_>>())
}

/// The factory problem from the mp book (PPOPT fixture `qp_problem`): 4 critical regions.
fn factory() -> MpProgram {
    let a = rows(&[
        &[1., 1., 0., 0.],
        &[0., 0., 1., 1.],
        &[-1., 0., -1., 0.],
        &[0., -1., 0., -1.],
        &[-1., 0., 0., 0.],
        &[0., -1., 0., 0.],
        &[0., 0., -1., 0.],
        &[0., 0., 0., -1.],
    ]);
    let b = vec![350., 600., 0., 0., 0., 0., 0., 0.];
    let c = vec![25.; 4];
    let f = rows(&[&[0., 0.], &[0., 0.], &[-1., 0.], &[0., -1.], &[0., 0.], &[0., 0.], &[0., 0.], &[0., 0.]]);
    let mut q = Mat::zeros(4, 4);
    for (i, v) in [153., 162., 162., 126.].iter().enumerate() {
        q[(i, i)] = 2.0 * v;
    }
    let a_t = rows(&[&[1., 0.], &[0., 1.], &[-1., 0.], &[0., -1.]]);
    let b_t = vec![1000., 1000., 0., 0.];
    MpProgram::new_mpqp(a, b, c, Mat::zeros(4, 2), q, a_t, b_t, f, vec![])
}

#[test]
fn factory_all_algorithms_four_regions() {
    let p = Arc::new(factory());
    let reference = region_signature(&solve_mpqp(&p, MpqpAlgorithm::Combinatorial).critical_regions);
    assert_eq!(reference.len(), 4);
    for &algo in MpqpAlgorithm::all() {
        let s = solve_mpqp(&p, algo);
        assert_eq!(s.len(), 4, "{:?}", algo);
        assert_eq!(region_signature(&s.critical_regions), reference, "{:?}", algo);
    }
}

#[test]
fn factory_solution_matches_direct_qp() {
    let p = Arc::new(factory());
    let s = solve_mpqp(&p, MpqpAlgorithm::Combinatorial);
    assert!(s.verify_solution());
    let mut rng = StdRng::seed_from_u64(3);
    for _ in 0..500 {
        let th = vec![rng.gen_range(0.0..1000.0), rng.gen_range(0.0..1000.0)];
        assert!(s.verify_theta(&th));
    }
}

/// PPOPT fixture `portfolio_problem_analog` (equality constraints, post_process=False):
/// geometric and combinatorial must agree.
#[test]
fn portfolio_geometric_equals_combinatorial() {
    let n = 8;
    let mu = [0.09551451, 0.00317183, 0.06799116, 0.12334409, 0.10235298, 0.0754139, 0.00730871, 0.11324299];
    let mut a = Mat::zeros(n + 2, n);
    for j in 0..n {
        a[(0, j)] = 1.0;
        a[(1, j)] = mu[j];
        a[(2 + j, j)] = -1.0;
    }
    let mut b = vec![0.0; n + 2];
    b[0] = 1.0;
    let mut f = Mat::zeros(n + 2, 1);
    f[(1, 0)] = 1.0;
    let mut q = Mat::zeros(n, n);
    for i in 0..n {
        q[(i, i)] = (i + 1) as f64;
    }
    let mn = mu.iter().cloned().fold(f64::INFINITY, f64::min);
    let mx = mu.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let a_t = rows(&[&[-1.], &[1.]]);
    let p = Arc::new(MpProgram::new(a, b, vec![0.0; n], Mat::zeros(n, 1), Some(q), a_t, vec![-mn, mx], f, vec![0, 1], false));
    let geo = solve_mpqp(&p, MpqpAlgorithm::Geometric);
    let combi = solve_mpqp(&p, MpqpAlgorithm::Combinatorial);
    assert!(combi.len() > 1);
    assert_eq!(geo.len(), combi.len());
    for cr in &geo.critical_regions {
        let (c, _) = cr.chebyshev().unwrap();
        assert!(allclose(&geo.evaluate(&c).unwrap(), &combi.evaluate(&c).unwrap(), 1e-5, 1e-8));
    }
}

fn simple_mi(q: Option<Mat>) -> MpmiProgram {
    let a = rows(&[&[0., 1., 1.], &[1., 0., 0.], &[-1., 0., 0.], &[1., -1., 0.], &[1., 0., -1.]]);
    let b = vec![1., 0., 0., 0., 0.];
    let f = rows(&[&[0.], &[1.], &[0.], &[0.], &[0.]]);
    let c = vec![-3., 0., 0.];
    let a_t = rows(&[&[1.], &[1.]]);
    MpmiProgram::new(a, b, c, Mat::zeros(3, 1), q, a_t, vec![2., 2.], f, vec![1, 2], vec![])
}

#[test]
fn mpmilp_partial_feasibility_and_subproblem() {
    for q in [None, Some(Mat::eye(3))] {
        let p = simple_mi(q);
        assert!(p.check_bin_feasibility(&[0., 0.]));
        assert!(p.check_bin_feasibility(&[1., 0.]));
        assert!(p.check_bin_feasibility(&[0., 1.]));
        assert!(!p.check_bin_feasibility(&[1., 1.]));
    }
    let p = simple_mi(None);
    let sub = p.generate_substituted_problem(&[0., 1.]);
    assert_eq!((sub.a.rows, sub.a.cols), (2, 1));
    assert_eq!(sub.equality_indices, vec![0]);
}

fn market(q: bool) -> MpmiProgram {
    let a = rows(&[
        &[1., 1., 0., 0., 0.],
        &[0., 0., 1., 1., 0.],
        &[-1., 0., -1., 0., 0.],
        &[0., -1., 0., -1., -500.],
        &[-1., 0., 0., 0., 0.],
        &[0., -1., 0., 0., 0.],
        &[0., 0., -1., 0., 0.],
        &[0., 0., 0., -1., 0.],
        &[0., 0., 0., 0., -1.],
        &[0., 0., 0., 0., 1.],
    ]);
    let b = vec![350., 600., 0., 0., 0., 0., 0., 0., 0., 1.];
    let f = rows(&[&[0., 0.], &[0., 0.], &[-1., 0.], &[0., -1.], &[0., 0.], &[0., 0.], &[0., 0.], &[0., 0.], &[0., 0.], &[0., 0.]]);
    let a_t = rows(&[&[1., 0.], &[0., 1.], &[-1., 0.], &[0., -1.]]);
    let b_t = vec![1000., 1000., 0., 0.];
    if q {
        let mut qm = Mat::zeros(5, 5);
        for (i, v) in [153., 162., 162., 126., 1.].iter().enumerate() {
            qm[(i, i)] = *v;
        }
        MpmiProgram::new(a, b, vec![25., 25., 25., 25., 7.6e6], Mat::zeros(5, 2), Some(qm), a_t, b_t, f, vec![4], vec![])
    } else {
        MpmiProgram::new(a, b, vec![178., 187., 187., 151., 50000.], Mat::zeros(5, 2), None, a_t, b_t, f, vec![4], vec![])
    }
}

#[test]
fn mpmilp_market_evaluate_matches_milp() {
    let p = Arc::new(market(false));
    let sol = solve_mpmiqp_enumeration(&p, MpqpAlgorithm::Combinatorial, &SolveOptions::default());
    assert!(!sol.is_empty());
    let theta = [0.0, 500.0];
    let x = sol.evaluate(&theta).expect("theta inside");
    let obj = sol.evaluate_objective(&theta).unwrap();
    let det = p.solve_theta(&theta).unwrap();
    assert!((det.obj - obj).abs() <= 1e-6 * det.obj.abs().max(1.0), "{} vs {}", det.obj, obj);
    assert!(allclose(&det.sol, &x, 1e-5, 1e-6), "{:?} vs {:?}", det.sol, x);
}

#[test]
fn mpmiqp_market_enumeration() {
    let p = Arc::new(market(true));
    let sol = solve_mpmiqp_enumeration(&p, MpqpAlgorithm::Combinatorial, &SolveOptions::default());
    assert!(!sol.is_empty());
    let mut rng = StdRng::seed_from_u64(9);
    for _ in 0..50 {
        let th = [rng.gen_range(0.0..1000.0), rng.gen_range(0.0..1000.0)];
        if let (Some(det), Some(obj)) = (p.solve_theta(&th), sol.evaluate_objective(&th)) {
            assert!((det.obj - obj).abs() <= 1e-5 * det.obj.abs().max(1.0), "{} vs {}", det.obj, obj);
        }
    }
}

/// Every point location method must return the same region as sequential search.
#[test]
fn point_location_methods_agree() {
    for seed in [1u64, 2, 3] {
        let p = Arc::new(generate_mpqp(4, 2, 12, seed));
        let sol = solve_mpqp(&p, MpqpAlgorithm::Combinatorial);
        if sol.len() < 3 {
            continue;
        }
        let geom = PartitionGeometry::new(&sol);
        let reference = FlatSequential::new(&sol);
        let bst = BinarySearchTree::new(&geom);
        let mut rowsv = vec![];
        for i in 0..geom.num_regions {
            let (e, f) = geom.region_rows(i);
            for k in 0..f.len() {
                rowsv.extend_from_slice(&e[k * 2..(k + 1) * 2]);
                rowsv.push(f[k]);
            }
        }
        let locators: Vec<Box<dyn PointLocator>> = vec![
            Box::new(SequentialSearch::new(&sol)),
            Box::new(MultiwayTree::from_bst(&bst, 3, rowsv.clone(), geom.offsets.clone())),
            Box::new(bst),
            Box::new(LinearMachine::new(&geom).expect("linear machine")),
            Box::new(DescriptorFunction::new(&sol, &geom, 1)),
            Box::new(HashTable::new(&geom, 4 * geom.num_regions)),
            Box::new(Yann::new(&sol)),
        ];
        let (lo, hi) = (geom.global_lo.clone(), geom.global_hi.clone());
        let mut rng = StdRng::seed_from_u64(seed);
        for _ in 0..5000 {
            let th: Vec<f64> = (0..2).map(|k| rng.gen_range(lo[k] - 0.1 * (hi[k] - lo[k])..hi[k] + 0.1 * (hi[k] - lo[k]))).collect();
            let truth = reference.locate(&th);
            for l in &locators {
                let got = l.locate(&th);
                match (got, truth) {
                    (None, None) => {}
                    (Some(g), Some(t)) if g == t => {}
                    // degenerate problems can have overlapping regions: any region containing θ
                    // with the same optimiser is a correct answer
                    (Some(g), Some(t)) => {
                        let rg = &sol.critical_regions[g];
                        let rt = &sol.critical_regions[t];
                        assert!(rg.is_inside(&th, 1e-7), "{} returned a region not containing θ", l.name());
                        assert!(allclose(&rg.evaluate(&th), &rt.evaluate(&th), 1e-6, 1e-6), "{} at {:?}", l.name(), th);
                    }
                    _ => panic!("{} at {:?} (seed {seed}): {:?} vs {:?}", l.name(), th, got, truth),
                }
            }
        }
        // YANN output equals the explicit law
        let y = Yann::new(&sol);
        for cr in &sol.critical_regions {
            let Some((c, rad)) = cr.chebyshev() else { continue };
            if rad < 1e-6 {
                continue; // numerically thin region
            }
            assert!(allclose(&y.evaluate(&c), &cr.evaluate(&c), 1e-9, 1e-7));
        }
    }
}
