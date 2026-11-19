//! Times the active-set dynamic programming algorithms against the geometric and combinatorial
//! algorithms of RPOPT on the benchmark MPC problems, and the growth of the DP cost with the
//! horizon.
//!
//! cargo run --release --example dp_bench -- [problem ...] [--scaling N,N,...] [--limit SECONDS] [--threads T] [--dp-only]
//!
//! Times are medians over repeated runs (at least 5 runs and 0.5 s, a single run above `limit`
//! seconds); one thread for the serial algorithms and the DP variants without `+par`; `+par` uses `--threads` (default 4) rayon workers.

use rpopt::algorithms::dp::{enumerate_active_sets, solve_mpc_dp, DpOptions, DpStats};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::mpc::MpcProblem;
use rpopt::program::MpProgram;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(unix)]
#[repr(C)]
struct Timespec {
    sec: i64,
    nsec: i64,
}
#[cfg(unix)]
unsafe extern "C" {
    fn clock_gettime(clk: i32, ts: *mut Timespec) -> i32;
}

#[cfg(windows)]
#[path = "common/win_cpu.rs"]
mod win_cpu;

/// Windows: CPU seconds of the calling thread from the cycle counter (see `common/win_cpu.rs`).
#[cfg(windows)]
fn thread_cpu() -> f64 {
    win_cpu::thread_cpu_seconds()
}

/// CPU time of the calling thread (the machine is shared and loaded; wall time is noisy).
#[cfg(unix)]
fn thread_cpu() -> f64 {
    let mut ts = Timespec { sec: 0, nsec: 0 };
    #[cfg(target_os = "macos")]
    let clk = 16; // CLOCK_THREAD_CPUTIME_ID
    #[cfg(not(target_os = "macos"))]
    let clk = 3;
    unsafe { clock_gettime(clk, &mut ts) };
    ts.sec as f64 + ts.nsec as f64 * 1e-9
}

fn median_time<T>(limit: f64, mut f: impl FnMut() -> T) -> (f64, T) {
    if std::env::var("DP_CPU").is_ok() {
        let mut times = vec![];
        let mut total = 0.0;
        let mut last;
        loop {
            let t0 = thread_cpu();
            last = f();
            let dt = thread_cpu() - t0;
            times.push(dt);
            total += dt;
            if dt > limit || (times.len() >= 15 && total >= 0.5) || times.len() >= 200 {
                break;
            }
        }
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = if std::env::var("DP_MIN").is_ok() { 0 } else { times.len() / 2 };
        return (times[idx], last);
    }
    let mut times = vec![];
    let mut total = 0.0;
    let mut last;
    loop {
        let t0 = Instant::now();
        last = f();
        let dt = t0.elapsed().as_secs_f64();
        times.push(dt);
        total += dt;
        if dt > limit || (times.len() >= 5 && total >= 0.5) || times.len() >= 200 {
            break;
        }
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (times[times.len() / 2], last)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut names = vec![];
    let mut scaling: Vec<usize> = vec![];
    let mut limit = 2.0;
    let mut only_dp = false;
    let mut threads = 4usize;
    let mut only_variant: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scaling" => {
                scaling = args[i + 1].split(',').map(|s| s.parse().unwrap()).collect();
                i += 1;
            }
            "--limit" => {
                limit = args[i + 1].parse().unwrap();
                i += 1;
            }
            "--dp-only" => only_dp = true,
            "--variant" => {
                only_variant = Some(args[i + 1].clone());
                i += 1;
            }
            "--threads" => {
                threads = args[i + 1].parse().unwrap();
                i += 1;
            }
            n => names.push(n.to_string()),
        }
        i += 1;
    }
    rayon::ThreadPoolBuilder::new().num_threads(threads).build_global().unwrap();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/problems");
    if names.is_empty() && scaling.is_empty() {
        names = ["di_doc_N3", "di_N4", "di_N6", "di_N8", "di_N10", "masses2_N4", "masses2_N6", "sys3x2_N3", "sys3x2_N5"].iter().map(|s| s.to_string()).collect();
    }
    let variants: Vec<(&str, DpOptions)> = vec![
        ("dp2020", DpOptions::paper2020()),
        ("dp2022", DpOptions::cdc2022()),
        ("dp2023sym", DpOptions::symmetric()),
        ("dp2023sym+order", DpOptions { global_order: true, ..DpOptions::symmetric() }),
        ("dp2023sym+par", DpOptions { parallel: true, ..DpOptions::symmetric() }),
    ];
    println!("{:<12} {:<18} {:>10} {:>8} {:>10} {:>9} {:>9}", "problem", "algorithm", "time [s]", "regions", "candidates", "opt LPs", "feas LPs");
    for name in &names {
        let path = dir.join(format!("{name}.json"));
        let mpc = MpcProblem::from_json_file(&path).unwrap();
        let prog = Arc::new(MpProgram::from_json_file(&path).unwrap());
        if !only_dp {
            let algos: &[MpqpAlgorithm] = if std::env::var("DP_GEO").is_ok() { &[MpqpAlgorithm::Geometric] } else { &[MpqpAlgorithm::Geometric, MpqpAlgorithm::Combinatorial, MpqpAlgorithm::CombinatorialGraph] };
            for &algo in algos {
                let (t, s) = median_time(limit, || solve_mpqp(&prog, algo));
                println!("{:<12} {:<18} {:>10.4} {:>8}", name, algo.name(), t, s.critical_regions.len());
            }
        }
        let mut vs = variants.clone();
        if name.starts_with("dinv") {
            // terminal set = admissible set of the LQR: Cor. 3 and Prop. 4 of the 2020 paper apply
            vs.push(("dp2020+cor3", DpOptions { terminal_invariant: true, ..DpOptions::paper2020() }));
            vs.push(("dp2022+cor3", DpOptions { terminal_invariant: true, ..DpOptions::cdc2022() }));
            vs.push(("dp2023sym+cor3", DpOptions { terminal_invariant: true, ..DpOptions::symmetric() }));
        }
        for (label, opts) in &vs {
            if std::env::var("DP_GEO").is_ok() {
                break;
            }
            if only_variant.as_deref().is_some_and(|v| v != *label) {
                continue;
            }
            let (t, r) = median_time(limit, || solve_mpc_dp(&mpc, &prog, opts));
            let s = &r.stats;
            println!("{:<12} {:<18} {:>10.4} {:>8} {:>10} {:>9} {:>9}  (horizon {}, outcomes opt/infeas/other {:?}, lp s {:.3}+{:.3}, enum {:.3}, regions {:.3})", name, label, t, r.solution.critical_regions.len(), s.candidates, s.optimality_lps, s.feasibility_lps, s.horizon_reached, s.outcomes, s.seconds_opt_lp, s.seconds_feas_lp, s.seconds_enumeration, s.seconds_regions);
        }
    }
    if !scaling.is_empty() {
        let path = dir.join("di_N10.json");
        let mpc = MpcProblem::from_json_file(&path).unwrap();
        println!("\nhorizon scaling, double integrator (enumeration of the optimal active sets only)");
        println!("{:<4} {:<18} {:>10} {:>8} {:>10} {:>9} {:>9}", "N", "algorithm", "time [s]", "|S_N|", "candidates", "opt LPs", "feas LPs");
        for &n in &scaling {
            for (label, opts) in &variants {
                let o = DpOptions { horizon: Some(n), ..opts.clone() };
                let (t, st): (f64, DpStats) = median_time(limit, || {
                    let mut st = DpStats::default();
                    enumerate_active_sets(&mpc, &o, &mut st);
                    st
                });
                println!("{:<4} {:<18} {:>10.4} {:>8} {:>10} {:>9} {:>9}", n, label, t, st.set_sizes.last().unwrap(), st.candidates, st.optimality_lps, st.feasibility_lps);
            }
        }
    }
}
