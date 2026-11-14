//! Speed and correctness of every mpLP-capable exact algorithm.
//!
//!   cargo run --release --example mplp_bench -- <timeout_seconds> <problem> ...
//!
//! A problem is a JSON file (`bench/mplp_problems.py`) or `rand:x,t,m,seed` (PPOPT's random mpLP
//! generator). Each (problem, algorithm) runs in a child process killed after `timeout_seconds`;
//! an algorithm that times out is skipped for the larger members of the family (name before
//! "_N", or "rand"). Time is the median of repeated solves (at least 3 runs and 0.3 s, one run
//! above 2 s).
//!
//! Correctness: 2,000 uniform θ in the parameter box, each solved directly as an LP. A "gap" is a
//! feasible θ in no region; an "error" is a θ whose region law gives an infeasible x or an
//! objective off by more than 1e-6 (relative). mpLP optima need not be unique, so x itself is not
//! compared, and the number of regions can legitimately differ between algorithms.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::problem_generator::generate_mplp;
use rpopt::program::MpProgram;
use std::collections::HashSet;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const ALGOS: [MpqpAlgorithm; 10] = [
    MpqpAlgorithm::MplpLex,
    MpqpAlgorithm::Geometric,
    MpqpAlgorithm::Graph,
    MpqpAlgorithm::GraphExp,
    MpqpAlgorithm::CombinatorialGraph,
    MpqpAlgorithm::MplcpEnum,
    MpqpAlgorithm::Combinatorial,
    MpqpAlgorithm::GeometricParallel,
    MpqpAlgorithm::GraphParallel,
    MpqpAlgorithm::CombinatorialParallel,
];

fn median_time<T>(mut f: impl FnMut() -> T) -> (f64, T) {
    let t0 = Instant::now();
    let mut out = f();
    let first = t0.elapsed().as_secs_f64();
    if first > 2.0 {
        return (first, out);
    }
    let mut times = vec![first];
    let start = Instant::now();
    while (times.len() < 3 || start.elapsed().as_secs_f64() < 0.3) && times.len() < 200 {
        let t = Instant::now();
        out = f();
        times.push(t.elapsed().as_secs_f64());
    }
    times.sort_by(|a, b| a.total_cmp(b));
    (times[times.len() / 2], out)
}

fn load(spec: &str) -> MpProgram {
    if let Some(r) = spec.strip_prefix("rand:") {
        let v: Vec<u64> = r.split(',').map(|s| s.parse().unwrap()).collect();
        generate_mplp(v[0] as usize, v[1] as usize, v[2] as usize, v[3])
    } else {
        MpProgram::from_json_file(std::path::Path::new(spec)).unwrap()
    }
}

fn label(spec: &str) -> String {
    match spec.strip_prefix("rand:") {
        Some(r) => {
            let v: Vec<&str> = r.split(',').collect();
            format!("rand_x{}t{}m{}", v[0], v[1], v[2])
        }
        None => std::path::Path::new(spec).file_stem().unwrap().to_string_lossy().to_string(),
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Child: solve once (timed) and check; prints "regions time_ms gaps errors samples".
fn run_one(alg_name: &str, spec: &str) {
    let alg = *MpqpAlgorithm::all().iter().find(|a| a.name() == alg_name).unwrap();
    let prog = Arc::new(load(spec));
    let (t, sol) = median_time(|| solve_mpqp(&prog, alg));
    let (lo, hi) = prog.theta_bbox().cloned().expect("bounded parameter set");
    let mut rng = StdRng::seed_from_u64(7);
    let (mut gaps, mut errs, mut n) = (0, 0, 0);
    while n < 2000 {
        let th: Vec<f64> = lo.iter().zip(&hi).map(|(l, h)| l + (h - l) * rng.r#gen::<f64>()).collect();
        if !prog.valid_parameter_realization(&th) {
            continue;
        }
        n += 1;
        let direct = prog.solve_theta_uncounted(&th);
        match (direct, sol.get_region(&th)) {
            (None, _) => {}
            (Some(_), None) => gaps += 1,
            (Some(d), Some(r)) => {
                let x = r.evaluate(&th);
                let ft = prog.f.matvec(&th);
                let viol = (0..prog.a.rows).map(|i| dot(prog.a.row(i), &x) - prog.b[i] - ft[i]).fold(f64::NEG_INFINITY, f64::max);
                let (o, od) = (prog.evaluate_objective(&x, &th), prog.evaluate_objective(&d.sol, &th));
                if viol > 1e-6 || (o - od).abs() > 1e-6 * (1.0 + od.abs()) {
                    errs += 1;
                }
            }
        }
    }
    println!("{} {:.4} {} {} {}", sol.critical_regions.len(), t * 1e3, gaps, errs, n);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args[1] == "--one" {
        return run_one(&args[2], &args[3]);
    }
    let timeout = Duration::from_secs_f64(args[1].parse().unwrap());
    let exe = std::env::current_exe().unwrap();
    let mut dropped: HashSet<(String, &str)> = HashSet::new();
    println!("{:<18} {:>3} {:>4} {:>4}  {:<22} {:>8} {:>12} {:>5} {:>5}", "problem", "nt", "nx", "m", "algorithm", "regions", "time [ms]", "gaps", "errs");
    for spec in &args[2..] {
        let name = label(spec);
        let family = if name.starts_with("rand") { "rand".to_string() } else { name.split("_N").next().unwrap().to_string() };
        let p = load(spec);
        let (nt, nx, m) = (p.num_t(), p.num_x(), p.a.rows);
        for alg in ALGOS {
            let key = (family.clone(), alg.name());
            let head = format!("{:<18} {:>3} {:>4} {:>4}  {:<22}", name, nt, nx, m, alg.name());
            if dropped.contains(&key) {
                println!("{head} {:>8} {:>12}", "-", "skipped");
                continue;
            }
            let mut child = Command::new(&exe).args(["--one", alg.name(), spec]).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
            let start = Instant::now();
            let status = loop {
                if let Some(s) = child.try_wait().unwrap() {
                    break Some(s);
                }
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(20));
            };
            match status {
                Some(s) if s.success() => {
                    let out = child.wait_with_output().unwrap();
                    let f: Vec<String> = String::from_utf8_lossy(&out.stdout).split_whitespace().map(String::from).collect();
                    println!("{head} {:>8} {:>12} {:>5} {:>5}", f[0], f[1], f[2], f[3]);
                }
                Some(_) => {
                    println!("{head} {:>8} {:>12}", "-", "crashed");
                    dropped.insert(key);
                }
                None => {
                    println!("{head} {:>8} {:>12}", "-", format!(">{:.0} s", timeout.as_secs_f64()));
                    dropped.insert(key);
                }
            }
        }
    }
}
