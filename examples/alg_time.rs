//! Serial solve time of one algorithm: median over `reps` solves after one warm-up solve.
//!
//!   alg_time <algorithm> <reps> <problem.json>...
//! Prints `name algorithm regions median_ms` per problem. Used for A/B timing of a change to a
//! shared builder on solvers other than mplcp.
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let alg = MpqpAlgorithm::from_name(&args[1]).expect("algorithm");
    let reps: usize = args[2].parse().unwrap();
    for f in &args[3..] {
        let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(f)).unwrap());
        let n = solve_mpqp(&p, alg).critical_regions.len();
        let mut ts: Vec<f64> = (0..reps)
            .map(|_| {
                let t = Instant::now();
                let _ = solve_mpqp(&p, alg);
                t.elapsed().as_secs_f64() * 1e3
            })
            .collect();
        ts.sort_by(|a, b| a.total_cmp(b));
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        println!("{name} {} {n} {:.3}", args[1], ts[ts.len() / 2]);
    }
}
