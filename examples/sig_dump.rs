//! Prints the sorted region signature of geometric for each JSON problem (to diff across builds).
use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn main() {
    for f in std::env::args().skip(1) {
        let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(&f)).unwrap());
        let alg = if std::env::var("PAR").is_ok() { MpqpAlgorithm::GeometricParallel } else { MpqpAlgorithm::Geometric };
        let sol = solve_mpqp(&p, alg);
        println!("{f} {}", sol.len());
        for s in region_signature(&sol.critical_regions) {
            println!("{s:?}");
        }
    }
}
