//! Graph-family algorithms on degenerate mpLPs (infinity-norm and 1-norm MPC) must cover every
//! feasible parameter, like the combinatorial algorithm does. Same sampling check as
//! `examples/mplp_bench.rs`: random θ in the parameter box against a direct LP solve.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn gaps_and_errors(name: &str, alg: MpqpAlgorithm, samples: usize) -> (usize, usize, usize) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/problems_lp").join(name);
    let prog = Arc::new(MpProgram::from_json_file(&path).unwrap());
    let sol = solve_mpqp(&prog, alg);
    let (lo, hi) = prog.theta_bbox().cloned().expect("bounded parameter set");
    let mut rng = StdRng::seed_from_u64(7);
    let (mut gaps, mut errs, mut n) = (0, 0, 0);
    while n < samples {
        let th: Vec<f64> = lo.iter().zip(&hi).map(|(l, h)| l + (h - l) * rng.r#gen::<f64>()).collect();
        if !prog.valid_parameter_realization(&th) {
            continue;
        }
        n += 1;
        let Some(d) = prog.solve_theta_uncounted(&th) else { continue };
        match sol.get_region(&th) {
            None => gaps += 1,
            Some(r) => {
                let x = r.evaluate(&th);
                let ft = prog.f.matvec(&th);
                let viol = (0..prog.a.rows).map(|i| dot(prog.a.row(i), &x) - prog.b[i] - ft[i]).fold(f64::NEG_INFINITY, f64::max);
                let (o, od) = (prog.evaluate_objective(&x, &th), prog.evaluate_objective(&d.sol, &th));
                if viol > 1e-6 || (o - od).abs() > 1e-6 * (1.0 + od.abs()) {
                    errs += 1;
                }
            }
        }
    }
    (sol.critical_regions.len(), gaps, errs)
}

#[test]
fn graph_family_has_no_gaps_on_degenerate_mplps() {
    for name in ["diinf_N2.json", "dione_N2.json"] {
        for alg in [MpqpAlgorithm::Graph, MpqpAlgorithm::GraphExp, MpqpAlgorithm::GraphParallel, MpqpAlgorithm::CombinatorialGraph] {
            let (regions, gaps, errs) = gaps_and_errors(name, alg, 300);
            assert!(regions > 0, "{name} {}", alg.name());
            assert_eq!((gaps, errs), (0, 0), "{name} {}: {regions} regions", alg.name());
        }
    }
}
