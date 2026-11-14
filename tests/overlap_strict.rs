//! Point location on overlapping (mpLP) solutions prefers regions that contain theta strictly:
//! a region holding theta only within the tolerance extrapolates its law and can beat the optimum.

use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;

fn check(alg: MpqpAlgorithm) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/problems_lp/diinf_N6.json");
    let p = std::sync::Arc::new(MpProgram::from_json_file(&path).unwrap());
    let mut sol = solve_mpqp(&p, alg);
    sol.is_overlapping = true;
    let theta = [2.2108375971726453, -3.1729839036073884];
    let x = sol.evaluate(&theta).expect("theta is feasible");
    let prog = sol.continuous.as_ref().unwrap();
    let direct = prog.solve_theta(&theta).unwrap();
    let obj = sol.program.evaluate_objective(&x, &theta);
    let direct_obj = sol.program.evaluate_objective(&direct.sol, &theta);
    assert!((obj - direct_obj).abs() < 1e-9, "objective {obj} against the LP optimum {direct_obj}");
}

#[test]
fn overlap_prefers_strict_containment_mplp_lex() {
    check(MpqpAlgorithm::MplpLex);
}

#[test]
fn overlap_prefers_strict_containment_geometric() {
    check(MpqpAlgorithm::Geometric);
}
