//! `mplcp` and `geometric` on programs whose sampled start has dependent active rows.
//!
//! Regression: `gen_optimal_active_set` accepted an active set with at most n_x rows even when
//! those rows were linearly dependent (two copies of the same bound binding together, or an
//! over-determined vertex), so the start region could not be built and both algorithms returned
//! zero regions. The POP programs below (bench/problems_large) triggered it.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::path::Path;
use std::sync::Arc;

fn check(name: &str, algo: MpqpAlgorithm) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/problems_large").join(format!("{name}.json"));
    let prog = Arc::new(MpProgram::from_json_file(&path).unwrap());
    let sol = solve_mpqp(&prog, algo);
    assert!(!sol.critical_regions.is_empty(), "{name} {algo:?}: no critical regions");
    // every feasible sampled parameter lies in a region and the law matches a direct solve
    let mut rng = StdRng::seed_from_u64(5);
    let (mut feasible, mut bad) = (0, 0);
    for _ in 0..1000 {
        let th: Vec<f64> = (0..prog.num_t()).map(|_| rng.gen_range(-9.9..9.9)).collect();
        if prog.solve_theta(&th).is_some() {
            feasible += 1;
            if !sol.verify_theta(&th) {
                bad += 1;
            }
        }
    }
    assert!(feasible > 100, "{name}: too few feasible samples");
    assert!(bad == 0, "{name} {algo:?}: {bad} of {feasible} feasible samples uncovered or wrong");
}

#[test]
fn mplcp_dependent_start() {
    for n in ["pop_031", "pop_052", "pop_071"] {
        check(n, MpqpAlgorithm::Mplcp);
    }
}

#[test]
fn geometric_dependent_start() {
    for n in ["pop_031", "pop_052", "pop_071"] {
        check(n, MpqpAlgorithm::Geometric);
    }
}

/// `mplcp` must not return the same parameter set under two index sets (rows that bind together:
/// a basis with an identically zero multiplier or slack is lexico-infeasible). Sampled check:
/// no sampled parameter lies strictly inside two regions, and none lies in no region.
fn no_overlap(name: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/problems_large").join(format!("{name}.json"));
    let prog = Arc::new(MpProgram::from_json_file(&path).unwrap());
    let sol = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
    let mut rng = StdRng::seed_from_u64(11);
    let (mut feasible, mut overlaps, mut gaps) = (0, 0, 0);
    // box around the parameter set A_t θ <= b_t
    let nt = prog.num_t();
    let (mut lo, mut hi) = (vec![-1.0; nt], vec![1.0; nt]);
    for k in 0..nt {
        let mut c = vec![0.0; nt];
        c[k] = 1.0;
        if let Some(s) = rpopt::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
            lo[k] = s.obj;
        }
        c[k] = -1.0;
        if let Some(s) = rpopt::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
            hi[k] = -s.obj;
        }
    }
    for _ in 0..1000 {
        let th: Vec<f64> = (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect();
        if prog.solve_theta(&th).is_none() {
            continue;
        }
        feasible += 1;
        let inside = sol.critical_regions.iter().filter(|r| r.is_inside(&th, -1e-7)).count();
        if inside >= 2 {
            overlaps += 1;
        }
        if sol.get_region(&th).is_none() {
            gaps += 1;
        }
    }
    assert!(feasible > 100, "{name}: too few feasible samples");
    assert_eq!(overlaps, 0, "{name}: {overlaps} of {feasible} feasible samples inside two regions");
    assert_eq!(gaps, 0, "{name}: {gaps} of {feasible} feasible samples in no region");
}

#[test]
fn mplcp_no_overlap_dependent_rows() {
    for n in ["pop_019", "pop_031", "pop_052", "pop_071"] {
        no_overlap(n);
    }
}

/// pop_001 (about 3,500 regions): a facet pivot, not the start, lands on the lexico-infeasible twin.
#[test]
fn mplcp_no_overlap_pop_001() {
    no_overlap("pop_001");
}
