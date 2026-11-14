//! Seam measurement for the point location tolerance.
//!
//! cargo run --release --example pl_seam -- <path to problem json> [--samples 40]
//! For points on region facets (centres, random facet points, vertices) it records
//!   1. the smallest violation over all regions (0 or less: strictly inside some region),
//!   2. the gap between neighbours: smallest violation over the other regions at points of
//!      interior facets,
//!   3. harm: for points pushed outward from boundary facets by d, the constraint violation of the
//!      extrapolated law of the region returned under a 1e-5 tolerance.
//! Violations are measured on unit-normalized rows (distances in theta space).

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::linalg::dot;
use rpopt::point_location::*;
use rpopt::program::MpProgram;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args[0].clone();
    let samples: usize = args.iter().position(|a| a == "--samples").map(|i| args[i + 1].parse().unwrap()).unwrap_or(40);
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(&path)).unwrap());
    let sol = solve_mpqp(&prog, MpqpAlgorithm::GeometricParallel);
    let geom = PartitionGeometry::new(&sol);
    let n = geom.dim;
    let nr = geom.num_regions;
    let viol = |j: usize, p: &[f64]| -> f64 {
        let (e, f) = geom.region_rows(j);
        (0..f.len()).map(|k| dot(&e[k * n..(k + 1) * n], p) - f[k]).fold(f64::NEG_INFINITY, f64::max)
    };
    let mut rng = StdRng::seed_from_u64(5);
    let thr = [0.0, 1e-12, 1e-9, 1e-7, 1e-5];
    let mut miss = [0usize; 5];
    let mut total = 0usize;
    let mut max_min = f64::NEG_INFINITY;
    let mut gaps: Vec<f64> = vec![];
    let mut isolated = 0usize;
    let mut boundary_pts: Vec<(Vec<f64>, Vec<f64>)> = vec![];
    let mut row_norm = (f64::INFINITY, 0.0f64);
    for r in &sol.critical_regions {
        for k in 0..r.e.rows {
            let nm = rpopt::linalg::norm(r.e.row(k));
            row_norm = (row_norm.0.min(nm), row_norm.1.max(nm));
        }
    }
    for i in 0..nr {
        if geom.thin[i] {
            continue;
        }
        let (e, f) = geom.region_rows(i);
        for k in 0..f.len() {
            let a = &e[k * n..(k + 1) * n];
            let on: Vec<&Vec<f64>> = geom.vertices[i].iter().filter(|v| (dot(a, v) - f[k]).abs() < 1e-9).collect();
            if on.len() < n {
                continue;
            }
            for s in 0..samples {
                let p: Vec<f64> = if s == 0 {
                    let mut x = vec![0.0; n];
                    for v in &on {
                        for j in 0..n {
                            x[j] += v[j] / on.len() as f64;
                        }
                    }
                    x
                } else if s == 1 {
                    on[0].clone()
                } else {
                    let ws: Vec<f64> = (0..on.len()).map(|_| rng.gen_range(0.01..1.0)).collect();
                    let sm: f64 = ws.iter().sum();
                    let mut x = vec![0.0; n];
                    for (w, v) in ws.iter().zip(&on) {
                        for j in 0..n {
                            x[j] += w / sm * v[j];
                        }
                    }
                    x
                };
                total += 1;
                let mn = (0..nr).map(|j| viol(j, &p)).fold(f64::INFINITY, f64::min);
                max_min = max_min.max(mn);
                for (t, th) in thr.iter().enumerate() {
                    if mn > *th {
                        miss[t] += 1;
                    }
                }
                let mut nb = f64::INFINITY;
                for j in 0..nr {
                    if j == i {
                        continue;
                    }
                    // neighbour: shares the facet plane (a row of j with opposite normal through p)
                    let (ej, fj) = geom.region_rows(j);
                    let shares = (0..fj.len()).any(|kk| {
                        let b = &ej[kk * n..(kk + 1) * n];
                        dot(a, b) < -1.0 + 1e-6 && (dot(b, &p) - fj[kk]).abs() < 1e-6
                    });
                    if shares {
                        nb = nb.min(viol(j, &p));
                    }
                }
                if nb.is_finite() {
                    gaps.push(nb);
                    if nb > 1e-9 {
                        isolated += 1;
                    }
                } else if s == 0 {
                    boundary_pts.push((p.clone(), a.to_vec()));
                }
            }
        }
    }
    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |p: f64| if gaps.is_empty() { f64::NAN } else { gaps[((gaps.len() - 1) as f64 * p) as usize] };
    println!("PROBLEM {} theta={} regions={} thin={} rows_norm=[{:.2e},{:.2e}]", path.rsplit('/').next().unwrap(), n, nr, geom.thin.iter().filter(|t| **t).count(), row_norm.0, row_norm.1);
    println!("  facet points {total}: min-violation>0: {} >1e-12: {} >1e-9: {} >1e-7: {} >1e-5: {}  (max over points of min violation {:.2e})", miss[0], miss[1], miss[2], miss[3], miss[4], max_min);
    println!("  neighbour gap (interior facet points {}): min {:.2e} p50 {:.2e} p99 {:.2e} max {:.2e}; points with no neighbour within 1e-9: {}", gaps.len(), q(0.0), q(0.5), q(0.99), q(1.0), isolated);
    let mut sol2 = sol.clone();
    sol2.point_location_tolerance = 1e-5;
    for d in [1e-9, 1e-8, 1e-7, 1e-6, 5e-6, 9e-6] {
        let (mut cnt, mut some, mut worst, mut worst_ratio, mut tin) = (0usize, 0usize, 0.0f64, 0.0f64, 0usize);
        for (p, a) in &boundary_pts {
            let th: Vec<f64> = (0..n).map(|j| p[j] + d * a[j]).collect();
            if (0..nr).any(|j| viol(j, &th) <= 0.0) {
                continue;
            }
            cnt += 1;
            let at = prog.a_t.matvec(&th);
            if at.iter().zip(&prog.b_t).all(|(u, v)| *u <= *v + 1e-12) {
                tin += 1;
            }
            if let Some(ri) = sol2.get_region_index(&th) {
                some += 1;
                let x = sol2.critical_regions[ri].evaluate(&th);
                let ax = prog.a.matvec(&x);
                let ft = prog.f.matvec(&th);
                let v = (0..ax.len()).map(|k| ax[k] - prog.b[k] - ft[k]).fold(f64::NEG_INFINITY, f64::max);
                worst = worst.max(v);
                worst_ratio = worst_ratio.max(v / d);
            }
        }
        println!("  harm d={d:.0e}: outside points {cnt} (inside theta-set {tin}), Some under 1e-5: {some}, worst constraint violation of extrapolated x {worst:.2e}, worst violation/d {worst_ratio:.2e}");
    }
}
