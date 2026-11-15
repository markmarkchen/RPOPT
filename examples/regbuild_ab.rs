//! Paired timing of the chained bounding-box LPs of the mplcp LP-path builder, in one binary:
//! per problem, each mode is solved `reps` times in a shuffled order (seeded by the first argument),
//! the per-mode median is taken, and the ratio is computed from the printed medians.
//!
//!   regbuild_ab <seed> <reps> <modes> <problem.json>...
//! modes: a comma list of `off` (shared builder, the previous behaviour), `on` (chain) and `daqp`
//! (parametric_daqp), e.g. `off,on,off,daqp` (the second `off` is the A/A pair). Prints one JSON line per problem.
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn med(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: u64 = a[1].parse().unwrap();
    let reps: usize = a[2].parse().unwrap();
    let modes: Vec<&str> = a[3].split(',').collect();
    for f in &a[4..] {
        let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(f)).unwrap());
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        let mut order: Vec<usize> = (0..modes.len()).collect();
        let mut x = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        for i in (1..order.len()).rev() {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            order.swap(i, ((x >> 33) as usize) % (i + 1));
        }
        let mut times: Vec<Vec<f64>> = vec![vec![]; modes.len()];
        let mut regions = 0;
        for _ in 0..reps {
            for &mi in &order {
                rpopt::algorithms::mplcp::set_box_chain(modes[mi] == "on");
                let alg = if modes[mi] == "daqp" { MpqpAlgorithm::from_name("parametric_daqp").expect("parametric_daqp") } else { MpqpAlgorithm::Mplcp };
                let t = Instant::now();
                let sol = solve_mpqp(&prog, alg);
                times[mi].push(t.elapsed().as_secs_f64() * 1e3);
                regions = sol.critical_regions.len();
            }
        }
        // one more chained solve for the fallback statistics
        rpopt::algorithms::mplcp::set_box_chain(true);
        let _ = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
        let (creg, cfb, ceng, coff) = rpopt::algorithms::mplcp::box_chain_stats();
        let meds: Vec<f64> = times.iter().map(|v| med(v.clone())).collect();
        println!("{}", serde_json::json!({"name": name, "regions": regions, "modes": modes, "order": order, "median_ms": meds, "times": times, "chain_regions": creg, "chain_fallbacks": cfb, "chain_engine_lps": ceng, "chain_off": coff}));
    }
}
