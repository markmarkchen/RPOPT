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
