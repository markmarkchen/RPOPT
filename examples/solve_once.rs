//! One timed solve of one algorithm (no warm-up, no repetition), for the problems that take minutes
//! to hours.
//!
//!   solve_once <algorithm> <problem.json>
//! Prints `name algorithm regions seconds`.
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let alg = MpqpAlgorithm::from_name(&args[1]).expect("algorithm");
    let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(&args[2])).unwrap());
    let t = Instant::now();
    let n = solve_mpqp(&p, alg).critical_regions.len();
    let name = std::path::Path::new(&args[2]).file_stem().unwrap().to_string_lossy().to_string();
    println!("{name} {} {n} {:.3}", args[1], t.elapsed().as_secs_f64());
}
