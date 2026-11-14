//! Times mplcp and mplcp_parallel (and checks them against geometric).
//!   mplcp_bench <threads,comma list> <reps> <files...>
//! Prints: name regions t_serial_ms [t_par_ms per thread count] same_geo par_eq_serial
//! Env NOGEO=1 skips the geometric check.
use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
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
    let args: Vec<String> = std::env::args().collect();
    let threads: Vec<usize> = args[1].split(',').filter(|s| !s.is_empty()).map(|s| s.parse().unwrap()).collect();
    let reps: usize = args[2].parse().unwrap();
    let geo = std::env::var("NOGEO").is_err();
    for path in &args[3..] {
        let p = std::path::Path::new(path);
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        let prog = Arc::new(MpProgram::from_json_file(p).unwrap());
        let ser = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
        let n = ser.critical_regions.len();
        let sig = region_signature(&ser.critical_regions);
        let same = if geo { region_signature(&solve_mpqp(&prog, MpqpAlgorithm::Geometric).critical_regions) == sig } else { true };
        let mut ts = vec![];
        for _ in 0..reps {
            let t = Instant::now();
            let _ = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
            ts.push(t.elapsed().as_secs_f64() * 1e3);
        }
        let mut line = format!("{:<14} {:>6} ser {:>9.3}", name, n, med(ts));
        let mut eq = true;
        for &th in &threads {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(th).build().unwrap();
            let r = pool.install(|| solve_mpqp(&prog, MpqpAlgorithm::MplcpParallel));
            eq &= region_signature(&r.critical_regions) == sig && r.critical_regions.iter().zip(&ser.critical_regions).all(|(a, b)| a.active_set == b.active_set);
            let ts = pool.install(|| {
                let mut ts = vec![];
                for _ in 0..reps {
                    let t = Instant::now();
                    let _ = solve_mpqp(&prog, MpqpAlgorithm::MplcpParallel);
                    ts.push(t.elapsed().as_secs_f64() * 1e3);
                }
                ts
            });
            line += &format!(" p{} {:>9.3}", th, med(ts));
        }
        println!("{} same={} order={}", line, same, eq);
    }
}
