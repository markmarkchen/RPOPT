//! Splits the serial cost of the tight check and of the facet certificate on one problem.
//! Configurations (certificate mode, tight check): A (0, off) baseline, B (0, on) tight check only,
//! C (5, off) vertices and hyperplane index only, D (1, off) certificate without the tight check,
//! E (1, on) the default. One solve per configuration per round, rounds alternate.
//!
//!   split_bench <problem.json> <rounds>
use rpopt::algorithms::mplcp::{set_certificate, set_tight_check};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(&a[1])).unwrap());
    let rounds: usize = a[2].parse().unwrap();
    let cfgs: [(&str, u8, bool); 6] = [("A base", 0, false), ("B tight", 0, true), ("C verts+index", 5, false), ("F verts only", 6, false), ("D cert", 1, false), ("E default", 1, true)];
    let mut times: Vec<Vec<f64>> = vec![vec![]; cfgs.len()];
    let mut regions = 0;
    for r in 0..rounds {
        for k in 0..cfgs.len() {
            let i = (k + r) % cfgs.len();
            let (_, mode, tight) = cfgs[i];
            set_certificate(mode);
            set_tight_check(tight);
            let t = Instant::now();
            let sol = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
            times[i].push(t.elapsed().as_secs_f64() * 1e3);
            regions = sol.critical_regions.len();
        }
    }
    let med = |v: &Vec<f64>| {
        let mut w = v.clone();
        w.sort_by(|x, y| x.total_cmp(y));
        w[w.len() / 2]
    };
    let base = med(&times[0]);
    let name = std::path::Path::new(&a[1]).file_stem().unwrap().to_string_lossy().to_string();
    print!("{name} regions {regions}");
    for (i, (n, _, _)) in cfgs.iter().enumerate() {
        let m = med(&times[i]);
        print!(" | {n} {m:.3} ms x{:.3}", m / base);
    }
    println!();
}
