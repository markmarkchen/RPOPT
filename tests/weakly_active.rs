//! A constraint that is weakly active on a whole region (active with λ ≡ 0) gives two labels, A and
//! A \ {i}, for one region. pop_031: x <= 0 is weakly active where the unconstrained optimum is
//! x = 0, and the enumerating methods returned the region as both {} and {3} (5 regions, overlaps on
//! 31% of the samples). An independent grid check with quadprog on the raw POP data finds 4
//! full-dimensional active sets, as do POP v1.5 and `geometric`.

use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

#[test]
fn weakly_active_region_is_reported_once() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/problems_large/pop_031.json");
    let p = Arc::new(MpProgram::from_json_file(&path).unwrap());
    let reference = region_signature(&solve_mpqp(&p, MpqpAlgorithm::Geometric).critical_regions);
    assert_eq!(reference.len(), 4);
    for alg in [
        MpqpAlgorithm::Mplcp,
        MpqpAlgorithm::MplcpEnum,
        MpqpAlgorithm::Combinatorial,
        MpqpAlgorithm::Graph,
        MpqpAlgorithm::CombinatorialGraph,
        MpqpAlgorithm::Ahmadi,
        MpqpAlgorithm::ParametricDaqp,
    ] {
        assert_eq!(region_signature(&solve_mpqp(&p, alg).critical_regions), reference, "{alg:?}");
    }
}
