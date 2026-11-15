//! Regression for the Case-2 sign test of `pivot_facet` (scale-free tolerances).
//!
//! pop_001 has twin rows 4 and 23 (the same hyperplane, scaled by about 1e-10). At the parent active
//! set [3,5,11,14,15,18], facet k=9, the unscaled test saw the beta part of Gamma at 1.2e-10, below
//! the zero tolerance: both directions read as "zero, so nonnegative", and the lexico-infeasible
//! basis [3,4,5,11,14,18,20] was returned next to the good [3,5,11,14,18,20,23]. An end filter
//! (`drop_lex_infeasible`) used to hide it; the scale-free test removes it at the source. Every
//! region returned must be lexico-feasible, because the entry-facet skip is proved for such
//! parents only.

use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::path::Path;
use std::sync::Arc;

#[test]
fn mplcp_pop_001_twin_rows_lex_feasible() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/problems_large/pop_001.json");
    let prog = Arc::new(MpProgram::from_json_file(&path).unwrap());
    let sol = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
    let sets: Vec<Vec<usize>> = sol
        .critical_regions
        .iter()
        .map(|r| {
            let mut a = r.active_set.clone();
            a.sort_unstable();
            a
        })
        .collect();
    assert!(!sets.contains(&vec![3, 4, 5, 11, 14, 18, 20]), "lexico-infeasible twin returned");
    assert!(sets.contains(&vec![3, 5, 11, 14, 18, 20, 23]), "lexico-feasible twin missing");
    assert_eq!(rpopt::algorithms::mplcp::count_lex_infeasible(&prog, &sol.critical_regions), 0);
    // the end filter's output had 3535 regions: 6 were overlapping twins (same parameter set under
    // the other twin row), see bench/results/mplcp_case2_tolerance.md
    assert_eq!(sol.critical_regions.len(), 3529);
}
