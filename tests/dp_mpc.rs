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

/// The reduced LPs, the full LPs (5) of the paper, the global candidate order and the parallel
/// version all give the same active sets.
#[test]
fn dp_variants_agree() {
    for name in ["di_N4", "sys3x2_N3", "masses2_N4"] {
        let sym = DpOptions::symmetric();
        let (mpc, prog) = problem(name);
        let base = solve_mpc_dp(&mpc, &prog, &sym);
        let sig = region_signature(&base.solution.critical_regions);
        let full = solve_mpc_dp(&mpc, &prog, &DpOptions { reduced_lps: false, ..sym.clone() });
        assert_eq!(region_signature(&full.solution.critical_regions), sig, "{name} full LPs");
        let ordered = solve_mpc_dp(&mpc, &prog, &DpOptions { global_order: true, ..sym.clone() });
        let par = solve_mpc_dp(&mpc, &prog, &DpOptions { parallel: true, ..sym.clone() });
        assert_eq!(region_signature(&ordered.solution.critical_regions), sig, "{name} global order");
        assert_eq!(region_signature(&par.solution.critical_regions), sig, "{name} parallel");
        assert_eq!(par.stats.set_sizes, ordered.stats.set_sizes, "{name}: parallel run is deterministic");
        assert_eq!(par.stats.optimality_lps, ordered.stats.optimality_lps, "{name}: parallel run is deterministic");
    }
}

/// Terminal set = maximal admissible set of the LQR (the setting of the 2020 paper): Cor. 3 (copy
/// the sets without terminal constraints) and the early stop of Prop. 4 apply.
#[test]
fn dp_invariant_terminal_matches_geometric() {
    for name in ["dinv_N4", "dinv_N6", "dinv_N8"] {
        let inv = |o: DpOptions| DpOptions { terminal_invariant: true, ..o };
        check(name, &inv(DpOptions::paper2020()), "paper2020 Cor.3");
        check(name, &inv(DpOptions::cdc2022()), "cdc2022 Cor.3");
        check(name, &inv(DpOptions::symmetric()), "symmetric Cor.3");
        check(name, &DpOptions::symmetric(), "symmetric plain");
    }
}

#[test]
fn dp_matches_geometric_all() {
    for name in ["di_N6", "di_N8", "di_N10", "masses2_N4", "masses2_N6", "sys3x2_N5"] {
        check(name, &DpOptions::symmetric(), "symmetric");
    }
}
