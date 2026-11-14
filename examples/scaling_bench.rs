//! Horizon scaling of the exact algorithms on MPC problems of growing size.
//!
//!   cargo run --release --example scaling_bench -- <limit_seconds> <problem.json> ...
//!
//! For each problem (in the order given), times every exact algorithm (median of repeated
//! solves: at least 3 runs and 0.3 s, one run above 2 s) and, when the JSON carries the "mpc"
//! key, the active-set DP (2023 symmetric variant). An algorithm is skipped for the rest of a
//! family (same name prefix before "_N") once one solve exceeds `limit_seconds`. Every result is
//! checked against geometric's region signature. Prints one line per (problem, algorithm).

use rpopt::algorithms::dp::{solve_mpc_dp, DpOptions};
use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::mpc::MpcProblem;
use rpopt::program::MpProgram;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn median_time<T>(mut f: impl FnMut() -> T) -> (f64, T) {
    let t0 = Instant::now();
    let mut out = f();
    let first = t0.elapsed().as_secs_f64();
    if first > 2.0 {
        return (first, out);
    }
    let mut times = vec![first];
    let start = Instant::now();
    while times.len() < 3 || start.elapsed().as_secs_f64() < 0.3 {
        let t = Instant::now();
        out = f();
        times.push(t.elapsed().as_secs_f64());
        if times.len() >= 200 {
            break;
        }
    }
    times.sort_by(|a, b| a.total_cmp(b));
    (times[times.len() / 2], out)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let limit: f64 = args[1].parse().unwrap();
    let algos = [
        MpqpAlgorithm::Geometric,
        MpqpAlgorithm::Mplcp,
        MpqpAlgorithm::GeometricParallel,
        MpqpAlgorithm::MplcpParallel,
        MpqpAlgorithm::Graph,
        MpqpAlgorithm::CombinatorialGraph,
        MpqpAlgorithm::ParametricDaqp,
        MpqpAlgorithm::Ahmadi,
        MpqpAlgorithm::Combinatorial,
    ];
    let mut dropped: HashSet<(String, String)> = HashSet::new();
    println!("{:<14} {:>5} {:>4} {:>8}  {:<20} {:>12} {:>6}", "problem", "N", "nx", "regions", "algorithm", "time [ms]", "same");
    for path in &args[2..] {
        let p = std::path::Path::new(path);
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        let family = name.split("_N").next().unwrap().to_string();
        let horizon: usize = name.rsplit("_N").next().unwrap().parse().unwrap_or(0);
        let prog = Arc::new(MpProgram::from_json_file(p).unwrap());
        let (tg, geo) = median_time(|| solve_mpqp(&prog, MpqpAlgorithm::Geometric));
        let sig = region_signature(&geo.critical_regions);
        let regions = geo.critical_regions.len();
        let nx = prog.num_x();
        for alg in algos {
            let key = (family.clone(), alg.name().to_string());
            if dropped.contains(&key) {
                println!("{:<14} {:>5} {:>4} {:>8}  {:<20} {:>12} {:>6}", name, horizon, nx, regions, alg.name(), "skipped", "-");
                continue;
            }
            let (t, same) = if alg == MpqpAlgorithm::Geometric {
                (tg, true)
            } else {
                let (t, s) = median_time(|| solve_mpqp(&prog, alg));
                (t, region_signature(&s.critical_regions) == sig)
            };
            println!("{:<14} {:>5} {:>4} {:>8}  {:<20} {:>12.3} {:>6}", name, horizon, nx, regions, alg.name(), t * 1e3, same);
            if t > limit {
                dropped.insert(key);
            }
        }
        // active-set DP, if the problem carries its MPC model
        let key = (family.clone(), "dp".to_string());
        if let Ok(mpc) = MpcProblem::from_json_file(p) {
            if dropped.contains(&key) {
                println!("{:<14} {:>5} {:>4} {:>8}  {:<20} {:>12} {:>6}", name, horizon, nx, regions, "dp (2023 sym)", "skipped", "-");
            } else {
                let opts = DpOptions::symmetric();
                // the DP encodes its constraint rows in 128-bit masks; larger problems are out of scope
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| median_time(|| solve_mpc_dp(&mpc, &prog, &opts))));
                match res {
                    Ok((t, r)) => {
                        let same = region_signature(&r.solution.critical_regions) == sig;
                        println!("{:<14} {:>5} {:>4} {:>8}  {:<20} {:>12.3} {:>6}", name, horizon, nx, regions, "dp (2023 sym)", t * 1e3, same);
                        if t > limit {
                            dropped.insert(key);
                        }
                    }
                    Err(_) => {
                        println!("{:<14} {:>5} {:>4} {:>8}  {:<20} {:>12} {:>6}", name, horizon, nx, regions, "dp (2023 sym)", "failed", "-");
                        dropped.insert(key);
                    }
                }
            }
        }
    }
}
