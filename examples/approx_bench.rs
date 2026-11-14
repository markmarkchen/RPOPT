//! Approximate mpQP (Bemporad-Filippi) against the exact geometric algorithm.
//!
//!   cargo run --release --example approx_bench -- [bench/problems] [reps]
//!
//! Per problem: exact geometric time and regions; for several epsilon (fractions of the largest
//! sampled value function) the approximate time, regions and measured max error.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::approx::{solve_mpqp_approx_with, ApproxOptions};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[repr(C)]
struct Rusage {
    utime: [i64; 2],
    stime: [i64; 2],
    rest: [i64; 14],
}
unsafe extern "C" {
    fn getrusage(who: i32, usage: *mut Rusage) -> i32;
}

/// user + system CPU seconds of this process (the benchmark is single threaded, so this is the
/// solve's CPU time and excludes time spent waiting on a loaded machine)
fn cpu_seconds() -> f64 {
    let mut u = Rusage { utime: [0; 2], stime: [0; 2], rest: [0; 14] };
    unsafe { getrusage(0, &mut u) };
    let tv = |t: [i64; 2]| t[0] as f64 + ((t[1] & 0xFFFF_FFFF) as f64) * 1e-6;
    tv(u.utime) + tv(u.stime)
}

/// Median CPU time of `reps` runs (the machine is shared, wall time is not meaningful under load).
fn median_time<T>(reps: usize, mut f: impl FnMut() -> T) -> (f64, T) {
    let mut ts = vec![];
    let mut last = None;
    for _ in 0..reps {
        let t = cpu_seconds();
        let r = f();
        ts.push(cpu_seconds() - t);
        last = Some(r);
    }
    ts.sort_by(|a, b| a.total_cmp(b));
    (ts[ts.len() / 2], last.unwrap())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).cloned().unwrap_or("bench/problems".into());
    let reps: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5);
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().map_or(false, |e| e == "json")).collect();
    files.sort();
    println!("{:<12} {:>9} {:>8} | {:>9} {:>9} {:>8} {:>10} {:>9}", "problem", "exact ms", "regions", "eps", "approx ms", "regions", "max err", "speedup");
    for path in files {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        if let Some(f) = args.get(3) {
            if !name.contains(f.as_str()) {
                continue;
            }
        }
        let prog = Arc::new(MpProgram::from_json_file(&path).unwrap());
        let (te, exact) = median_time(reps, || solve_mpqp(&prog, MpqpAlgorithm::Geometric));
        let (lo, hi) = prog.theta_bbox().unwrap();
        let mut rng = StdRng::seed_from_u64(3);
        let pts: Vec<Vec<f64>> = (0..2000).map(|_| (0..prog.num_t()).map(|k| rng.gen_range(lo[k]..hi[k])).collect()).collect();
        let sols: Vec<_> = pts.iter().map(|t| prog.solve_theta(t)).collect();
        let vmax = sols.iter().flatten().map(|s| s.obj).fold(f64::MIN, f64::max) - sols.iter().flatten().map(|s| s.obj).fold(f64::MAX, f64::min);
        for frac in [1e-3, 1e-2, 1e-1, 0.3, 1.0] {
            let eps = frac * vmax;
            let mut o = ApproxOptions::abs(eps);
            if std::env::var("APPROX_MODE").map_or(false, |v| v == "clipped") {
                o.exploration = rpopt::algorithms::approx::Exploration::Clipped;
            }
            let (ta, (sol, _)) = median_time(reps, || solve_mpqp_approx_with(&prog, &o));
            let mut maxerr = 0.0f64;
            for (t, d) in pts.iter().zip(&sols) {
                if let (Some(d), Some(x)) = (d, sol.evaluate(t)) {
                    maxerr = maxerr.max(prog.evaluate_objective(&x, t) - d.obj);
                }
            }
            println!("{:<12} {:>9.3} {:>8} | {:>9.3e} {:>9.3} {:>8} {:>10.3e} {:>8.2}x", name, te * 1e3, exact.len(), eps, ta * 1e3, sol.len(), maxerr, te / ta);
        }
        // low-rank storage of the exact solution
        let (tc, comp) = median_time(reps, || rpopt::lowrank::compress(&exact).unwrap());
        let (t_plain, _) = median_time(reps, || pts.iter().filter_map(|t| exact.evaluate(t)).count());
        let (t_comp, _) = median_time(reps, || pts.iter().filter_map(|t| comp.evaluate(t)).count());
        let s = comp.storage();
        println!(
            "  lowrank {name}: storage {} -> {} reals (+{} ints) = {:.2}x, dense nodes {}, depth {}, compress {:.3} ms, evaluate 2000 pts: plain {:.2} ms, compressed {:.2} ms",
            s.baseline_reals, s.compressed_reals, s.compressed_ints, s.compressed_reals as f64 / s.baseline_reals as f64, s.dense_nodes, s.max_depth, tc * 1e3, t_plain * 1e3, t_comp * 1e3
        );
    }
}
