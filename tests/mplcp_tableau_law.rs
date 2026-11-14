//! The multiplier law read from the tableau agrees with the KKT-solve law (A/B switch), the regions
//! and their order are identical, and mplcp_parallel returns them in the serial order.
//! One test function only: the switch is process-global.

use rpopt::algorithms::mplcp::{set_tableau_law, solve, Method};
use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / 1.0f64.max(a.abs()).max(b.abs())
}

#[test]
fn tableau_law_matches_kkt_solve_law() {
    let mut names: Vec<_> = std::fs::read_dir("bench/problems").unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map_or(false, |x| x == "json")).collect();
    names.sort();
    assert!(!names.is_empty());
    for path in names {
        let prog = Arc::new(MpProgram::from_json_file(&path).unwrap());
        set_tableau_law(false);
        let old = solve(&prog, 0, Method::Pivot).unwrap();
        set_tableau_law(true);
        let new = solve(&prog, 0, Method::Pivot).unwrap();
        let par = solve(&prog, 0, Method::PivotParallel).unwrap();
        let geo = region_signature(&solve_mpqp(&prog, MpqpAlgorithm::Geometric).critical_regions);
        assert_eq!(region_signature(&new), geo, "{}: signature differs from geometric", path.display());
        assert_eq!(old.len(), new.len(), "{}", path.display());
        assert_eq!(par.len(), new.len(), "{}", path.display());
        for ((o, n), p) in old.iter().zip(&new).zip(&par) {
            assert_eq!(o.active_set, n.active_set, "{}: region order", path.display());
            assert_eq!(p.active_set, n.active_set, "{}: parallel order", path.display());
            assert_eq!((&o.omega_set, &o.lambda_set, &o.regular_set), (&n.omega_set, &n.lambda_set, &n.regular_set));
            assert_eq!(o.e.rows, n.e.rows);
            let pairs = o
                .a
                .data
                .iter()
                .zip(&n.a.data)
                .chain(o.b.iter().zip(&n.b))
                .chain(o.c.data.iter().zip(&n.c.data))
                .chain(o.d.iter().zip(&n.d))
                .chain(o.e.data.iter().zip(&n.e.data))
                .chain(o.f.iter().zip(&n.f));
            for (x, y) in pairs {
                assert!(rel(*x, *y) < 1e-8, "{}: {} vs {}", path.display(), x, y);
            }
        }
    }
    set_tableau_law(true);
}
