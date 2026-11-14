//! sig_bench <alg> <file.json>...: regions, hash of the sorted active sets, time (single solve).
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let alg = *MpqpAlgorithm::all().iter().find(|x| x.name() == a[1]).unwrap();
    for f in &a[2..] {
        let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(f)).unwrap());
        let mut best = f64::MAX;
        let mut sol = None;
        for _ in 0..5 {
            let t = std::time::Instant::now();
            sol = Some(solve_mpqp(&p, alg));
            best = best.min(t.elapsed().as_secs_f64());
            if best > 3.0 {
                break;
            }
        }
        let mut v: Vec<Vec<usize>> = sol.unwrap().critical_regions.iter().map(|r| r.active_set.clone()).collect();
        v.sort();
        let mut h: u64 = 1469598103934665603;
        for s in &v {
            for &i in s.iter().chain(std::iter::once(&usize::MAX)) {
                h = (h ^ i as u64).wrapping_mul(1099511628211);
            }
        }
        println!("{:<24} {:<20} regions {:>5} hash {:016x} min_ms {:>10.3}", f.rsplit('/').next().unwrap(), a[1], v.len(), h, best * 1e3);
    }
}
