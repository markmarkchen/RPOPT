//! One cold solve of a problem by any mpQP algorithm, with a sampled coverage check: the region count, the solve time, and
//! the number of sampled feasible parameters that lie in no region (gaps).
//!
//!   gapcheck <algorithm> <samples> <problem.json>...
//! Prints `name algorithm nt regions solve_ms feasible_samples gaps` per problem. A run with 0 regions or any gap is
//! a failure.
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let alg = MpqpAlgorithm::from_name(&a[1]).expect("algorithm");
    let ns: usize = a[2].parse().unwrap();
    for f in &a[3..] {
        let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(f)).unwrap());
        let nt = prog.num_t();
        let t0 = Instant::now();
        let sol = solve_mpqp(&prog, alg);
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        // print the timing first: the coverage check below can take longer than the solve
        println!("{name} {} {nt} {} {ms:.3} solved", a[1], sol.critical_regions.len());
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
        let (mut feas, mut gaps) = (0, 0);
        for _ in 0..ns {
            let th: Vec<f64> = (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect();
            if prog.solve_theta(&th).is_none() {
                continue;
            }
            feas += 1;
            if sol.get_region(&th).is_none() {
                gaps += 1;
            }
        }
        println!("{name} {} {nt} {} {ms:.3} checked {feas} {gaps}", a[1], sol.critical_regions.len());
    }
}
