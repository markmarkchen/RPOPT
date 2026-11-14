//! mp-LCP (Jones-Morari pivoting) against the geometric algorithm and direct QP solves.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::linalg::Mat;
use rpopt::problem_generator::generate_mpqp;
use rpopt::program::MpProgram;
use std::sync::Arc;

#[test]
fn bench_problems_match_geometric() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/problems");
    let mut n = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map_or(true, |e| e != "json") {
            continue;
        }
        let p = Arc::new(MpProgram::from_json_file(&path).unwrap());
        let g = solve_mpqp(&p, MpqpAlgorithm::Geometric);
        let m = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
        assert_eq!(region_signature(&m.critical_regions), region_signature(&g.critical_regions), "{:?}", path);
        n += 1;
    }
    assert_eq!(n, 9);
}

#[test]
fn random_mpqps_match_geometric() {
    for seed in 0..12u64 {
        let p = Arc::new(generate_mpqp(3, 2, 6, seed));
        let g = solve_mpqp(&p, MpqpAlgorithm::Geometric);
        let m = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
        assert_eq!(region_signature(&m.critical_regions), region_signature(&g.critical_regions), "seed {}", seed);
    }
}

/// Projection of θ onto a polytope whose vertex (1,..,1) lies on extra planes: dependent active
/// constraints there (degenerate). Compared with direct QP solves (the partition into bases at
/// the degenerate vertex is not unique, so the signature may legitimately differ).
fn projection(extra: &[Vec<f64>], extra_b: &[f64], n: usize) -> MpProgram {
    let mut rows: Vec<Vec<f64>> = vec![];
    let mut b = vec![];
    for i in 0..n {
        let mut r = vec![0.0; n];
        r[i] = 1.0;
        rows.push(r.clone());
        b.push(1.0);
        r[i] = -1.0;
        rows.push(r);
        b.push(1.0);
    }
    rows.extend(extra.iter().cloned());
    b.extend_from_slice(extra_b);
    let m = rows.len();
    let a = Mat::from_rows(&rows);
    let mut h = Mat::zeros(n, n);
    for i in 0..n {
        h[(i, i)] = -1.0;
    }
    let mut a_t = Mat::zeros(2 * n, n);
    let mut b_t = vec![];
    for i in 0..n {
        a_t[(2 * i, i)] = 1.0;
        a_t[(2 * i + 1, i)] = -1.0;
        b_t.push(4.0);
        b_t.push(4.0);
    }
    MpProgram::new_mpqp(a, b, vec![0.0; n], h, Mat::eye(n), a_t, b_t, Mat::zeros(m, n), vec![])
}

fn check_against_qp(p: MpProgram, queries: usize) {
    let p = Arc::new(p);
    let s = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
    assert!(s.len() > 1);
    // multipliers are not unique at degenerate vertices, so only x is compared below
    let mut rng = StdRng::seed_from_u64(7);
    let nt = p.num_t();
    for _ in 0..queries {
        let th: Vec<f64> = (0..nt).map(|_| rng.gen_range(-3.9..3.9)).collect();
        assert!(s.verify_theta(&th), "theta {:?}", th);
    }
    let g = solve_mpqp(&p, MpqpAlgorithm::Geometric);
    eprintln!("mplcp {} regions, geometric {}", s.len(), g.len());
}

#[test]
fn degenerate_vertex_2d() {
    check_against_qp(projection(&[vec![1.0, 1.0]], &[2.0], 2), 3000);
}

#[test]
fn degenerate_vertex_3d() {
    let extra = vec![vec![1.0, 1.0, 0.0], vec![0.0, 1.0, 1.0], vec![1.0, 0.0, 1.0], vec![1.0, 1.0, 1.0]];
    check_against_qp(projection(&extra, &[2.0, 2.0, 2.0, 3.0], 3), 3000);
}
