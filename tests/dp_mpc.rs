//! The DP algorithms (Mitze and Moennigmann) must find exactly the active sets of the condensed
//! mpQP, as found by the geometric algorithm.
//!
//! Index mapping: the DP numbers constraints in the stagewise order of the MPC problem; each row is
//! mapped to a row of the condensed `MpProgram` through the `row_map` of the problem JSON (and the
//! geometric match of the processed rows with the original ones); the active set handed to
//! `gen_cr_from_active_set` is the program's equality rows plus the mapped rows, sorted.

use rpopt::algorithms::dp::{solve_mpc_dp, DpOptions};
use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::mpc::MpcProblem;
use rpopt::program::MpProgram;
use std::path::PathBuf;
use std::sync::Arc;

fn problem(name: &str) -> (MpcProblem, Arc<MpProgram>) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/problems").join(format!("{name}.json"));
    (MpcProblem::from_json_file(&path).unwrap(), Arc::new(MpProgram::from_json_file(&path).unwrap()))
}

fn check(name: &str, opts: &DpOptions, label: &str) {
    let (mpc, prog) = problem(name);
    let reference = region_signature(&solve_mpqp(&prog, MpqpAlgorithm::Geometric).critical_regions);
    let r = solve_mpc_dp(&mpc, &prog, opts);
    let got = region_signature(&r.solution.critical_regions);
    assert_eq!(got.len(), reference.len(), "{name} {label}: region count");
    assert_eq!(got, reference, "{name} {label}");
    assert!(r.solution.verify_solution(), "{name} {label}: verify");
}

#[test]
fn dp_matches_geometric_small() {
    for name in ["di_doc_N3", "di_N4", "sys3x2_N3"] {
        check(name, &DpOptions::paper2020(), "paper2020");
        check(name, &DpOptions::cdc2022(), "cdc2022");
        check(name, &DpOptions::symmetric(), "symmetric");
    }
}

#[test]
fn dp_matches_geometric_all() {
    for name in ["di_N6", "di_N8", "di_N10", "masses2_N4", "masses2_N6", "sys3x2_N5"] {
        check(name, &DpOptions::symmetric(), "symmetric");
    }
}
