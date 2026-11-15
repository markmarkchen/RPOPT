//! The bordered update of `algorithms::incremental` must reproduce the KKT rows of `region_rows`.

use rpopt::algorithms::incremental::ParentInfo;
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

#[test]
fn bordered_update_matches_direct_rows() {
    for name in ["masses2_N4", "sys3x2_N3"] {
        let path = format!("{}/bench/problems/{name}.json", env!("CARGO_MANIFEST_DIR"));
        let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(&path)).unwrap());
        let sol = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
        let m = prog.num_constraints();
        let mut checked = 0;
        let mut worst: f64 = 0.0;
        for r in sol.critical_regions.iter().take(40) {
            let law = prog.optimal_control_law_dual(&r.active_set).unwrap();
            let Some(info) = ParentInfo::new(&prog, &r.active_set, &law) else { continue };
            for i in 0..m {
                if r.active_set.contains(&i) {
                    continue;
                }
                let Some(child) = info.child(&prog, i) else { continue };
                // λ-infeasible children carry no rows; compare the others
                let rpopt::algorithms::incremental::ChildKkt::Region(child) = child else { continue };
                let mut b = r.active_set.clone();
                b.push(i);
                b.sort_unstable();
                let direct_law = prog.optimal_control_law_dual(&b).unwrap();
                let (e, f, nl, inactive) = prog.region_rows(&b, &direct_law);
                assert_eq!(nl, child.n_lambda);
                assert_eq!(inactive, child.inactive);
                assert_eq!((e.rows, e.cols), (child.e.rows, child.e.cols));
                for k in 0..e.data.len() {
                    let d = (e.data[k] - child.e.data[k]).abs() / (1.0 + e.data[k].abs());
                    worst = worst.max(d);
                }
                for k in 0..f.len() {
                    let d = (f[k] - child.f[k]).abs() / (1.0 + f[k].abs());
                    worst = worst.max(d);
                }
                checked += 1;
            }
        }
        assert!(checked > 100, "{name}: only {checked} children checked");
        assert!(worst < 1e-8, "{name}: worst relative difference {worst:e}");
        println!("{name}: {checked} children, worst relative difference {worst:.2e}");
    }
}
