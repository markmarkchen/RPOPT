//! MPR gates: every region is exact, and the uncovered volume respects (epsilon, beta).

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::mpr::{solve_mpqp_mpr, solve_mpr_with, MprOptions};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::problem_generator::{generate_mplp, generate_mpqp};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn problems() -> Vec<(String, Arc<MpProgram>)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/problems");
    let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    files.sort();
    assert_eq!(files.len(), 12);
    files.into_iter().map(|f| (f.file_stem().unwrap().to_string_lossy().to_string(), Arc::new(MpProgram::from_json_file(&f).unwrap()))).collect()
}

/// fraction of `n` uniform samples of the box D that are feasible and outside the regions
fn uncovered_volume(p: &MpProgram, sol: &rpopt::solution::Solution, n: usize, seed: u64) -> f64 {
    let (lo, hi) = p.theta_bbox().unwrap().clone();
    let mut rng = StdRng::seed_from_u64(seed);
    let mut unc = 0usize;
    for _ in 0..n {
        let th: Vec<f64> = (0..lo.len()).map(|k| lo[k] + (hi[k] - lo[k]) * rng.r#gen::<f64>()).collect();
        if p.solve_theta_uncounted(&th).is_some() && sol.get_region_index(&th).is_none() {
            unc += 1;
        }
    }
    unc as f64 / n as f64
}

#[test]
fn mpr_regions_exact_and_coverage_on_bench_problems() {
    let (eps, beta) = (1e-2, 1e-3);
    for (name, p) in problems() {
        let exact = solve_mpqp(&p, MpqpAlgorithm::Geometric);
        for seed in 0..3u64 {
            let s = solve_mpqp_mpr(&p, eps, beta, seed);
            assert!(!s.is_empty(), "{name}");
            for r in &s.critical_regions {
                let er = exact.critical_regions.iter().find(|e| e.active_set == r.active_set).unwrap_or_else(|| panic!("{name}: active set {:?} not exact", r.active_set));
                // same law: compare on points inside the MPR region
                let (c, rad) = r.chebyshev().unwrap();
                assert!(rad > 1e-9);
                let (xa, xb) = (r.evaluate(&c), er.evaluate(&c));
                for (a, b) in xa.iter().zip(&xb) {
                    assert!((a - b).abs() <= 1e-6 * (1.0 + a.abs()), "{name}: law mismatch");
                }
                // and the region agrees with the QP solution at its center
                let direct = p.solve_theta_uncounted(&c).unwrap();
                for (a, b) in xa.iter().zip(&direct.sol) {
                    assert!((a - b).abs() <= 1e-5 * (1.0 + a.abs()), "{name}: not optimal");
                }
            }
            let u = uncovered_volume(&p, &s, 100_000, 1000 + seed);
            assert!(u <= eps, "{name} seed {seed}: uncovered {u} > {eps}");
        }
    }
}

#[test]
fn mpr_tight_epsilon_and_serial_equals_batched() {
    let (name, p) = problems().into_iter().find(|(n, _)| n == "di_N4").unwrap();
    let (eps, beta) = (1e-3, 1e-6);
    let a = solve_mpr_with(&p, eps, beta, 5, &MprOptions { batch: 1, ..Default::default() }).0;
    let b = solve_mpr_with(&p, eps, beta, 5, &MprOptions { batch: 64, ..Default::default() }).0;
    let sig = |s: &rpopt::solution::Solution| s.critical_regions.iter().map(|r| r.active_set.clone()).collect::<Vec<_>>();
    assert_eq!(sig(&a), sig(&b), "{name}");
    assert!(uncovered_volume(&p, &a, 100_000, 77) <= eps);
}

#[test]
fn mpr_algorithm3_equals_algorithm1() {
    let sig = |s: &rpopt::solution::Solution| s.critical_regions.iter().map(|r| r.active_set.clone()).collect::<Vec<_>>();
    for (name, p) in problems().into_iter().filter(|(n, _)| n.starts_with("sys3x2") || n.starts_with("masses2_N4") || n == "di_N6") {
        let a1 = solve_mpr_with(&p, 1e-2, 1e-3, 11, &MprOptions { outer_approx: false, ..Default::default() }).0;
        let a3 = solve_mpr_with(&p, 1e-2, 1e-3, 11, &MprOptions { outer_approx: true, ..Default::default() }).0;
        assert_eq!(sig(&a1), sig(&a3), "{name}");
    }
}

#[test]
fn mpr_mplp_and_random_mpqp() {
    for (p, name) in [(generate_mplp(3, 2, 8, 1), "mplp"), (generate_mpqp(3, 2, 8, 2), "mpqp")] {
        let p = Arc::new(p);
        let exact = solve_mpqp(&p, MpqpAlgorithm::Combinatorial);
        let s = solve_mpqp_mpr(&p, 1e-2, 1e-3, 3);
        for r in &s.critical_regions {
            assert!(exact.critical_regions.iter().any(|e| e.active_set == r.active_set), "{name}");
        }
        if p.theta_bbox().is_some() {
            assert!(uncovered_volume(&p, &s, 20_000, 9) <= 1e-2, "{name}");
        }
    }
}
