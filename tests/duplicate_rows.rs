//! Duplicate constraint rows. `bench/pop_convert.py` appends the variable bounds Xmin <= x <= Xmax
//! as extra rows even where A already contains them, so several POP problems hold exact duplicate
//! rows. Before duplicates were merged in `base_constraint_processing`, the active-set
//! enumerations reported a region once per copy (overlaps on up to 90% of sampled parameters) and
//! `mplcp` and `geometric` returned no region on some of them.

use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::linalg::Mat;
use rpopt::program::MpProgram;
use std::sync::Arc;

/// (problem, region count found by POP v1.5 and by geometric after the fix)
const POP: [(&str, usize); 5] = [("pop_010", 36), ("pop_052", 3), ("pop_071", 5), ("pop_081", 4), ("pop_093", 7)];

#[test]
fn pop_problems_with_duplicate_rows_agree_across_algorithms() {
    let algos = [
        MpqpAlgorithm::Geometric,
        MpqpAlgorithm::Mplcp,
        MpqpAlgorithm::Combinatorial,
        MpqpAlgorithm::Graph,
        MpqpAlgorithm::CombinatorialGraph,
        MpqpAlgorithm::ParametricDaqp,
    ];
    for (name, regions) in POP {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("bench/problems_large/{name}.json"));
        let p = Arc::new(MpProgram::from_json_file(&path).unwrap());
        let reference = region_signature(&solve_mpqp(&p, MpqpAlgorithm::Geometric).critical_regions);
        assert_eq!(reference.len(), regions, "{name}: geometric");
        for alg in algos {
            let sig = region_signature(&solve_mpqp(&p, alg).critical_regions);
            assert_eq!(sig, reference, "{name}: {alg:?}");
        }
    }
}

/// min 1/2 x^2 + theta x over |theta| <= 1 with x <= 1 + theta given three times (once scaled by 2)
/// and -x <= 1 once: the copies collapse to one row.
#[test]
fn duplicate_and_scaled_rows_are_merged() {
    let a = Mat::from_rows(&[vec![1.0], vec![2.0], vec![-1.0], vec![1.0]]);
    let b = vec![1.0, 2.0, 1.0, 1.0];
    let f = Mat::from_rows(&[vec![1.0], vec![2.0], vec![0.0], vec![1.0]]);
    let a_t = Mat::from_rows(&[vec![1.0], vec![-1.0]]);
    let p = MpProgram::new(a, b, vec![0.0], Mat::from_rows(&[vec![1.0]]), Some(Mat::from_rows(&[vec![1.0]])), a_t, vec![1.0, 1.0], f, vec![], false);
    assert_eq!(p.num_constraints(), 2);
    assert!(p.equality_indices.is_empty());
}

/// An inequality that duplicates an equality is dropped and the equality is kept, whichever comes
/// first in the input.
#[test]
fn equality_replaces_duplicated_inequality() {
    for eq_first in [true, false] {
        let rows = if eq_first { [vec![1.0], vec![1.0], vec![-1.0]] } else { [vec![1.0], vec![-1.0], vec![1.0]] };
        let frows = if eq_first { [vec![1.0], vec![1.0], vec![0.0]] } else { [vec![1.0], vec![0.0], vec![1.0]] };
        let eq = if eq_first { 0 } else { 2 };
        let a_t = Mat::from_rows(&[vec![1.0], vec![-1.0]]);
        let p = MpProgram::new(
            Mat::from_rows(&rows),
            vec![0.0, if eq_first { 0.0 } else { 2.0 }, if eq_first { 2.0 } else { 0.0 }],
            vec![0.0],
            Mat::from_rows(&[vec![0.0]]),
            Some(Mat::from_rows(&[vec![1.0]])),
            a_t,
            vec![1.0, 1.0],
            Mat::from_rows(&frows),
            vec![eq],
            false,
        );
        assert_eq!(p.num_constraints(), 2, "eq_first = {eq_first}");
        assert_eq!(p.equality_indices, vec![0], "eq_first = {eq_first}");
        assert_eq!(p.a.row(0)[0], 1.0 / 2f64.sqrt(), "eq_first = {eq_first}");
    }
}
