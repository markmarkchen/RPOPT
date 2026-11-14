//! Debug: are region vertex sets complete? Compares projections onto random directions with LP optima.
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::point_location::PartitionGeometry;
use std::sync::Arc;

fn main() {
    let prog = Arc::new(rpopt::problem_generator::generate_mpqp(4, 6, 14, 1));
    let sol = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
    let geom = PartitionGeometry::new(&sol);
    let n = geom.dim;
    let mut rng = StdRng::seed_from_u64(3);
    let mut shown = 0;
    for i in 0..geom.num_regions {
        let (a, b) = geom.region_mat(i);
        let al: Vec<f64> = (0..n).map(|_| rng.gen_range(-1.0..1.0)).collect();
        let vmax = geom.vertices[i].iter().map(|v| rpopt::linalg::dot(&al, v)).fold(f64::NEG_INFINITY, f64::max);
        let neg: Vec<f64> = al.iter().map(|x| -x).collect();
        if let Some(s) = rpopt::solver::lp::solve_lp(Some(&neg), &a, &b, &[]) {
            let lp = -s.obj;
            if (lp - vmax).abs() > 1e-6 && shown < 6 {
                shown += 1;
                let bmax = b.iter().fold(0.0f64, |m, v| m.max(v.abs()));
                // LP point feasibility
                let viol = (0..a.rows).map(|k| rpopt::linalg::dot(a.row(k), &s.x[..n]) - b[k]).fold(f64::NEG_INFINITY, f64::max);
                let bv = geom.vertices[i].iter().max_by(|u, v| rpopt::linalg::dot(&al, u).partial_cmp(&rpopt::linalg::dot(&al, v)).unwrap()).unwrap();
                let vv = (0..a.rows).map(|k| rpopt::linalg::dot(a.row(k), bv) - b[k]).fold(f64::NEG_INFINITY, f64::max);
                println!("best vertex violation {vv:.2e}");
                println!("region {i}: rows {} verts {} lp {lp:.6} vmax {vmax:.6} max|b| {bmax:.3e} lp-point violation {viol:.2e} x {:?}", a.rows, geom.vertices[i].len(), &s.x[..n]);
            }
        }
    }
}
