//! Overlap test: the Chebyshev center of every region must lie in that region only.
//!
//!   overlap_check <problem.json> <algo>
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::linalg::dot;
use rpopt::program::MpProgram;
use rpopt::utils::chebyshev_center;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(&args[1])).unwrap());
    let sol = solve_mpqp(&p, MpqpAlgorithm::from_name(&args[2]).unwrap());
    let regs = &sol.critical_regions;
    let (mut bad, mut tiny) = (0, 0);
    for (i, r) in regs.iter().enumerate() {
        let Some((c, rad)) = chebyshev_center(&r.e, &r.f) else { continue };
        if rad < 1e-7 {
            tiny += 1;
        }
        let n = regs.iter().enumerate().filter(|(j, o)| *j != i && (0..o.e.rows).all(|k| dot(o.e.row(k), &c) <= o.f[k] + 1e-9)).count();
        if n > 0 {
            bad += 1;
            if bad <= 5 {
                println!("region {i} radius {rad:.3e} center also in {n} other regions");
            }
        }
    }
    println!("{} regions {} with overlapping center, {} with radius < 1e-7", args[1], bad, tiny);
}
