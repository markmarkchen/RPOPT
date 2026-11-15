//! Per problem: mplcp region count, number of lexico-infeasible regions, and a sampled coverage
//! check (gaps: feasible θ in no region; overlaps: θ strictly inside two regions).
//!
//!   lexcheck <samples> <problem.json>...
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let ns: usize = a[1].parse().unwrap();
    for f in &a[2..] {
        let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(f)).unwrap());
        let sol = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
        let lexbad = rpopt::algorithms::mplcp::count_lex_infeasible(&prog, &sol.critical_regions);
        let nt = prog.num_t();
        let (mut lo, mut hi) = (vec![-1.0; nt], vec![1.0; nt]);
        for k in 0..nt {
            let mut c = vec![0.0; nt];
            c[k] = 1.0;
            if let Some(s) = rpopt::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
                lo[k] = s.obj;
            }
            c[k] = -1.0;
            if let Some(s) = rpopt::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
                hi[k] = -s.obj;
            }
        }
        let mut rng = StdRng::seed_from_u64(17);
        let (mut feas, mut ov, mut gaps) = (0, 0, 0);
        for _ in 0..ns {
            let th: Vec<f64> = (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect();
            if prog.solve_theta(&th).is_none() {
                continue;
            }
            feas += 1;
            if sol.critical_regions.iter().filter(|r| r.is_inside(&th, -1e-7)).count() >= 2 {
                ov += 1;
            }
            if sol.get_region(&th).is_none() {
                gaps += 1;
            }
        }
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        println!("{name} regions {} lex_infeasible {lexbad} feasible_samples {feas} overlaps {ov} gaps {gaps}", sol.critical_regions.len());
    }
}
