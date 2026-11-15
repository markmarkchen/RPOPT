//! Looks at one region of the mplcp solution: the multiplier law of each active constraint
//! (norm of the θ part, constant), which facets bound it, and, at the Chebyshev centre, the
//! multipliers and the direct QP active set.
//!
//!   stray <problem.json> "1,3,8,11,18"
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(&a[1])).unwrap());
    let want: Vec<usize> = a[2].split(',').filter_map(|t| t.trim().parse().ok()).collect();
    let sol = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
    let nt = prog.num_t();
    for (idx, r) in sol.critical_regions.iter().enumerate() {
        let mut s = r.active_set.clone();
        s.sort_unstable();
        if s != want {
            continue;
        }
        println!("region {idx} active {:?} lambda_set {:?} regular_set {:?} omega {:?} rows {}", r.active_set, r.lambda_set, r.regular_set.1, r.omega_set, r.e.rows);
        let sc = rpopt::utils::chebyshev_ball(&r.e, &r.f, &[]).unwrap();
        let c: Vec<f64> = sc.x[..nt].to_vec();
        println!("centre {:?} radius {:.3e}", c, sc.x[nt]);
        for (k, &i) in r.active_set.iter().enumerate() {
            let th: f64 = (0..nt).map(|t| r.c[(k, t)].powi(2)).sum::<f64>().sqrt();
            let lam: f64 = (0..nt).map(|t| r.c[(k, t)] * c[t]).sum::<f64>() + r.d[k];
            println!("  active {i}: lambda law |theta part| {:.3e} const {:.3e}; lambda at centre {:.3e}; in lambda_set {}", th, r.d[k], lam, r.lambda_set.contains(&i));
        }
        let qp = prog.solve_theta(&c).map(|o| {
            let mut v = o.active_set.clone();
            v.sort_unstable();
            (v, o.dual)
        });
        println!("qp at centre: {qp:?}");
    }
}
