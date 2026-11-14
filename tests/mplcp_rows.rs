//! The regions `mplcp` builds (hinted construction with vertex clipping) have the same rows and
//! index sets as the reference construction `gen_cr_from_active_set_lambda_first`.

use rpopt::algorithms::mplcp::{solve, Method};
use rpopt::mpqp_utils::gen_cr_from_active_set_lambda_first;
use rpopt::program::MpProgram;

#[test]
fn parallel_finds_the_same_regions() {
    for entry in std::fs::read_dir("bench/problems").unwrap().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map_or(true, |x| x != "json") {
            continue;
        }
        let prog = MpProgram::from_json_file(&path).unwrap();
        let sig = |m: Method| {
            let mut v: Vec<Vec<usize>> = solve(&prog, 0, m).unwrap().into_iter().map(|r| r.active_set).collect();
            v.sort();
            v
        };
        assert_eq!(sig(Method::Pivot), sig(Method::PivotParallel), "{}", path.display());
    }
}

#[test]
fn mplcp_rows_match_reference_construction() {
    let mut names: Vec<_> = std::fs::read_dir("bench/problems").unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map_or(false, |x| x == "json")).collect();
    names.sort();
    assert!(!names.is_empty());
    for path in names {
        let prog = MpProgram::from_json_file(&path).unwrap();
        let regions = solve(&prog, 0, Method::Pivot).unwrap();
        let mut bad = 0;
        for r in &regions {
            let reference = gen_cr_from_active_set_lambda_first(&prog, &r.active_set, true).expect("reference region");
            let same_sets = reference.omega_set == r.omega_set && reference.lambda_set == r.lambda_set && reference.regular_set == r.regular_set;
            let mut same_rows = reference.e.rows == r.e.rows;
            if same_rows {
                for i in 0..r.e.rows {
                    for k in 0..r.e.cols {
                        if (reference.e[(i, k)] - r.e[(i, k)]).abs() > 1e-12 {
                            same_rows = false;
                        }
                    }
                    if (reference.f[i] - r.f[i]).abs() > 1e-12 {
                        same_rows = false;
                    }
                }
            }
            if !(same_sets && same_rows) {
                bad += 1;
            }
        }
        assert_eq!(bad, 0, "{}: {} of {} regions differ from the reference construction", path.display(), bad, regions.len());
    }
}
