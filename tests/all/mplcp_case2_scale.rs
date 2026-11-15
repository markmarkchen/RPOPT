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
    // the program with its twin rows: the preprocessing merge of duplicate rows is off, so the test
    // keeps testing mplcp's own handling of twins (with the merge on, the twins are gone before mplcp runs)
    let prog = Arc::new(MpProgram::from_json_file_opts(&path, false).unwrap());
    // the solve with the twin rows in the program (the invariant this test is about)
    let regions = rpopt::algorithms::mplcp::solve_with_dedupe(&prog, 0, rpopt::algorithms::mplcp::Method::Pivot, false).unwrap();
    let sets: Vec<Vec<usize>> = regions
        .iter()
        .map(|r| {
            let mut a = r.active_set.clone();
            a.sort_unstable();
            a
        })
        .collect();
    assert!(!sets.contains(&vec![3, 4, 5, 11, 14, 18, 20]), "lexico-infeasible twin returned");
    assert!(sets.contains(&vec![3, 5, 11, 14, 18, 20, 23]), "lexico-feasible twin missing");
    assert_eq!(rpopt::algorithms::mplcp::count_lex_infeasible(&prog, &regions), 0);
    // the end filter's output had 3535 regions: 6 were overlapping twins (same parameter set under
    // the other twin row), see bench/results/mplcp_case2_tolerance.md
    assert_eq!(regions.len(), 3529);
    // the default solve removes the duplicated rows: the same 3529 regions, each reported with the lowest-index row of a
    // twin group (row 4, not 23)
    let sol = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
    assert_eq!(sol.critical_regions.len(), 3529);
    let dsets: Vec<Vec<usize>> = sol
        .critical_regions
        .iter()
        .map(|r| {
            let mut a = r.active_set.clone();
            a.sort_unstable();
            a
        })
        .collect();
    assert!(dsets.contains(&vec![3, 4, 5, 11, 14, 18, 20]), "the region of the twin pair must carry the lowest-index row 4");
    assert!(!dsets.iter().any(|s| s.contains(&23)), "row 23 is a removed twin of row 4");
}
