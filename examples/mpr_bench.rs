//! MPR vs exact geometric: time, regions found, measured uncovered volume.
//! Usage: mpr_bench <problems_dir> [--runs R] [--samples S] [--batch B] [--only a,b]

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::mpr::{solve_mpr_with, MprOptions};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;
#[cfg(unix)]
unsafe extern "C" {
    fn clock() -> i64;
}
#[cfg(windows)]
#[path = "common/win_cpu.rs"]
mod win_cpu;

/// Process CPU time in seconds (the machine is shared, so wall time is not comparable).
struct Instant(f64);
impl Instant {
    #[cfg(unix)]
    fn now() -> Self {
        Instant(unsafe { clock() } as f64 / 1e6)
    }
    /// Windows: process CPU seconds from the cycle counter (see `common/win_cpu.rs`), as `clock()` on Unix.
    #[cfg(windows)]
    fn now() -> Self {
        Instant(win_cpu::process_cpu_seconds())
    }
    fn elapsed(&self) -> Dur {
        Dur(Instant::now().0 - self.0)
    }
}
struct Dur(f64);
impl Dur {
    fn as_secs_f64(&self) -> f64 {
        self.0
    }
}

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// (uncovered fraction of D, uncovered fraction of the feasible samples)
fn uncovered(p: &MpProgram, sol: &rpopt::solution::Solution, n: usize, seed: u64) -> (f64, f64) {
    let (lo, hi) = p.theta_bbox().unwrap().clone();
    let mut rng = StdRng::seed_from_u64(seed);
    let (mut feas, mut unc) = (0usize, 0usize);
    for _ in 0..n {
        let th: Vec<f64> = (0..lo.len()).map(|k| lo[k] + (hi[k] - lo[k]) * rng.r#gen::<f64>()).collect();
        if p.solve_theta_uncounted(&th).is_some() {
            feas += 1;
            if sol.get_region_index(&th).is_none() {
                unc += 1;
            }
        }
    }
    (unc as f64 / n as f64, unc as f64 / feas.max(1) as f64)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = std::path::PathBuf::from(&args[1]);
    let (mut runs, mut samples, mut batch) = (5usize, 100_000usize, 64usize);
    let mut only: Option<Vec<String>> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--runs" => runs = args[i + 1].parse().unwrap(),
            "--samples" => samples = args[i + 1].parse().unwrap(),
            "--batch" => batch = args[i + 1].parse().unwrap(),
            "--only" => only = Some(args[i + 1].split(',').map(String::from).collect()),
            o => panic!("unknown {o}"),
        }
        i += 2;
    }
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    files.sort();
    println!("{:<14} {:>3} {:>10} {:>7} {:>9} {:>8} {:>7} {:>9} {:>11} {:>11} {:>7} {:>6}", "problem", "m", "eps/beta", "N", "mpr_cpu_ms", "geo_cpu_ms", "speed", "regions", "unc_vol(D)", "unc_feas", "failed", "cuts");
    for f in files {
        let name = f.file_stem().unwrap().to_string_lossy().to_string();
        if only.as_ref().is_some_and(|o| !o.contains(&name)) {
            continue;
        }
        let p = Arc::new(MpProgram::from_json_file(&f).unwrap());
        let mut gt = vec![];
        let mut exact = None;
        for _ in 0..runs {
            let t = Instant::now();
            let s = solve_mpqp(&p, MpqpAlgorithm::Geometric);
            gt.push(t.elapsed().as_secs_f64() * 1e3);
            exact = Some(s);
        }
        let geo = median(gt);
        let nexact = exact.unwrap().len();
        for &(eps, beta) in &[(1e-2, 1e-3), (1e-3, 1e-6), (1e-4, 1e-6)] {
            let opts = MprOptions { batch, ..Default::default() };
            let mut ts = vec![];
            let mut last = None;
            for r in 0..runs {
                let t = Instant::now();
                let (s, st) = solve_mpr_with(&p, eps, beta, 100 + r as u64, &opts);
                ts.push(t.elapsed().as_secs_f64() * 1e3);
                last = Some((s, st));
            }
            let (s, st) = last.unwrap();
            let t = median(ts);
            let (u, uf) = uncovered(&p, &s, samples, 7);
            println!(
                "{:<14} {:>3} {:>5.0e}/{:<4.0e} {:>7} {:>9.1} {:>8.1} {:>6.2}x {:>4}/{:<4} {:>11.2e} {:>11.2e} {:>7} {:>6}",
                name,
                p.num_constraints(),
                eps,
                beta,
                st.n_required,
                t,
                geo,
                geo / t,
                s.len(),
                nexact,
                u,
                uf,
                st.failed_regions, st.outer_rows
            );
        }
    }
}
