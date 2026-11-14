//! Per-region law quality versus Chebyshev radius: for each region of a solution, the largest
//! primal violation of its law at its Chebyshev center, bucketed by radius.
//!
//!   lawcheck <problem.json> <algo>
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(&args[1])).unwrap());
    let sol = solve_mpqp(&p, MpqpAlgorithm::from_name(&args[2]).unwrap());
    let prog = sol.continuous.clone().unwrap();
    let edges = [1e-8, 1e-7, 1e-6, 1e-5, 1e-4, 1e-3, f64::INFINITY];
    // per bucket: count, count with violation > 1e-7, worst violation, count with x != QP (1e-5)
    let mut b = vec![(0usize, 0usize, 0.0f64, 0usize); edges.len()];
    for r in &sol.critical_regions {
        let Some((c, rad)) = r.chebyshev() else { continue };
        let k = edges.iter().position(|&e| rad < e).unwrap();
        let x = r.evaluate(&c);
        let ax = prog.a.matvec(&x);
        let ft = prog.f.matvec(&c);
        let v = (0..ax.len())
            .map(|i| if prog.equality_indices.contains(&i) { (ax[i] - prog.b[i] - ft[i]).abs() } else { ax[i] - prog.b[i] - ft[i] })
            .fold(f64::NEG_INFINITY, f64::max);
        b[k].0 += 1;
        if v > 1e-7 {
            b[k].1 += 1;
        }
        b[k].2 = b[k].2.max(v);
        if let Some(s) = prog.solve_theta(&c) {
            if x.iter().zip(&s.sol).any(|(u, w)| (u - w).abs() > 1e-5 + 1e-6 * w.abs()) {
                b[k].3 += 1;
            }
        }
    }
    println!("{} {}: {} regions", args[1], args[2], sol.len());
    let mut lo = 0.0;
    for (k, e) in edges.iter().enumerate() {
        println!("  r in [{lo:.0e},{e:.0e}): {:6} regions, viol>1e-7: {:5}, worst viol {:.2e}, law!=QP at center: {}", b[k].0, b[k].1, b[k].2, b[k].3);
        lo = *e;
    }
}
