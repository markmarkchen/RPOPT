//! Repeats one solve for profiling:  cargo run --profile profiling --example profile_solve -- <problem.json> <algorithm> <iterations>

use ppopt_rs::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use ppopt_rs::program::MpProgram;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(&args[1])).unwrap());
    let algo = MpqpAlgorithm::from_name(&args[2]).unwrap();
    let iters: usize = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(10);
    let mut sig = vec![];
    let (l0, q0) = ppopt_rs::stats::snapshot();
    let r0 = ppopt_rs::stats::ranks();
    let t0 = Instant::now();
    for _ in 0..iters {
        let s = solve_mpqp(&prog, algo);
        sig = region_signature(&s.critical_regions);
    }
    let dt = t0.elapsed().as_secs_f64() / iters as f64;
    let lps = ppopt_rs::solver::lp::LP_CALLS.load() / iters as u64;
    let (l1, q1) = ppopt_rs::stats::snapshot();
    let (pl, pq) = ((l1 - l0) / iters as u64, (q1 - q0) / iters as u64);
    println!("PPOPT-equivalent rank tests per solve: {}", (ppopt_rs::stats::ranks() - r0) / iters as u64);
    println!(
        "{} regions, {:.3} ms per solve, {} LPs ({:.3} us each); PPOPT-equivalent {} LPs + {} QPs -> {:.3} us per PPOPT call",
        sig.len(), dt * 1e3, lps, dt * 1e6 / lps as f64, pl, pq, dt * 1e6 / (pl + pq) as f64
    );
}
