//! Gap test across region facets: from each region's Chebyshev center, shoot random rays to the
//! boundary, step `eps` past the facet, and check whether another region contains the point. An
//! uncovered point where the direct QP is feasible is a hole in the partition.
//!
//!   seamcheck <problem.json> <algo> [eps=1e-6] [rays_per_region=6]
use rand::{rngs::StdRng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::linalg::dot;
use rpopt::program::MpProgram;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(&args[1])).unwrap());
    let sol = solve_mpqp(&p, MpqpAlgorithm::from_name(&args[2]).unwrap());
    let eps: f64 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(1e-6);
    let rays: usize = args.get(4).map(|s| s.parse().unwrap()).unwrap_or(6);
    let prog = sol.continuous.clone().unwrap();
    let mut rng = StdRng::seed_from_u64(7);
    let (mut tested, mut covered, mut uncov_infeas, mut uncov_theta, mut holes) = (0, 0, 0, 0, 0);
    let mut shown = 0;
    for (ri, r) in sol.critical_regions.iter().enumerate() {
        let Some((c, rad)) = r.chebyshev() else { continue };
        if rad < 1e-6 {
            continue;
        }
        for _ in 0..rays {
            let dir: Vec<f64> = (0..c.len()).map(|_| rpopt::program::standard_normal(&mut rng)).collect();
            let (mut hi, mut kh) = (f64::INFINITY, usize::MAX);
            for k in 0..r.e.rows {
                let a = dot(r.e.row(k), &dir);
                if a > 1e-14 {
                    let t = (r.f[k] - dot(r.e.row(k), &c)) / a;
                    if t < hi {
                        hi = t;
                        kh = k;
                    }
                }
            }
            if kh == usize::MAX {
                continue;
            }
            let nrm = rpopt::linalg::norm(r.e.row(kh));
            let q: Vec<f64> = (0..c.len()).map(|i| c[i] + hi * dir[i] + eps * r.e[(kh, i)] / nrm).collect();
            tested += 1;
            let inside = sol.critical_regions.iter().any(|s| (0..s.e.rows).all(|k| dot(s.e.row(k), &q) - s.f[k] <= 0.0));
            if inside {
                covered += 1;
                continue;
            }
            if !prog.valid_parameter_realization(&q) {
                uncov_theta += 1;
                continue;
            }
            match prog.solve_theta(&q) {
                None => uncov_infeas += 1,
                Some(_) => {
                    holes += 1;
                    if shown < 5 {
                        shown += 1;
                        let mn = sol.critical_regions.iter().map(|s| (0..s.e.rows).map(|k| (dot(s.e.row(k), &q) - s.f[k]) / rpopt::linalg::norm(s.e.row(k))).fold(f64::NEG_INFINITY, f64::max)).fold(f64::INFINITY, f64::min);
                        println!("  hole near region {ri} (r={rad:.2e}) facet row {kh}: nearest region violation {mn:.2e}");
                    }
                }
            }
        }
    }
    println!(
        "{} {} eps={eps:.0e}: regions {}, rays {tested}: covered {covered}, outside Theta {uncov_theta}, uncovered+QP infeasible {uncov_infeas}, uncovered+QP feasible (holes) {holes}",
        args[1], args[2], sol.len()
    );
}
