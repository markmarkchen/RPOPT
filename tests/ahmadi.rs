//! The Ahmadi-Moshkenani et al. algorithm finds the same regions as the geometric algorithm.

use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

#[test]
fn ahmadi_matches_geometric_on_bench_problems() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/problems");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().map(|e| e == "json").unwrap_or(false))
        .collect();
    files.sort();
    assert_eq!(files.len(), 9);
    for f in files {
        let p = Arc::new(MpProgram::from_json_file(&f).unwrap());
        let g = region_signature(&solve_mpqp(&p, MpqpAlgorithm::Geometric).critical_regions);
        let a = region_signature(&solve_mpqp(&p, MpqpAlgorithm::Ahmadi).critical_regions);
        assert_eq!(a, g, "{}", f.display());
    }
}

#[test]
fn ahmadi_matches_combinatorial_on_random_problems() {
    use rpopt::problem_generator::generate_mpqp;
    for (x, t, m) in [(3usize, 2usize, 10usize), (4, 2, 12), (4, 3, 14), (5, 2, 14), (3, 1, 9), (2, 2, 8)] {
        for seed in 1..=8u64 {
            let p = Arc::new(generate_mpqp(x, t, m, seed));
            let c = region_signature(&solve_mpqp(&p, MpqpAlgorithm::Combinatorial).critical_regions);
            let a = region_signature(&solve_mpqp(&p, MpqpAlgorithm::Ahmadi).critical_regions);
            assert_eq!(a, c, "x={x} t={t} m={m} seed={seed}");
        }
    }
}
