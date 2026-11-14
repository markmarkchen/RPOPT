//! Splits the graph algorithm's time into its initialisation (random QP sampling) and the rest.
use ppopt_rs::algorithms::{solve_mpqp, MpqpAlgorithm};
use ppopt_rs::program::MpProgram;
use rand::SeedableRng;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(&args[1])).unwrap());
    let iters = 2000;
    let t0 = Instant::now();
    let mut n = 0;
    for _ in 0..iters {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        n += prog.sample_theta_space(100, &mut rng).len();
    }
    let ts = t0.elapsed().as_secs_f64() / iters as f64;
    let t1 = Instant::now();
    for _ in 0..iters {
        n += solve_mpqp(&prog, MpqpAlgorithm::Graph).len();
    }
    let tg = t1.elapsed().as_secs_f64() / iters as f64;
    let t2 = Instant::now();
    for _ in 0..iters {
        n += prog.feasible_space_chebychev_ball().map(|s| s.x.len()).unwrap_or(0);
    }
    let tc = t2.elapsed().as_secs_f64() / iters as f64;
    println!("sample_theta_space {:.1} us, chebyshev {:.1} us, graph total {:.1} us ({n})", ts * 1e6, tc * 1e6, tg * 1e6);
}
