//! `geometric` / `geometric_parallel` on degenerate programs: no gaps and no overlaps.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::critical_region::CriticalRegion;
use rpopt::linalg::Mat;
use rpopt::program::MpProgram;
use std::sync::Arc;

/// min 1/2 |x|^2 - theta'x (projection of theta) s.t. |x_i| <= 1 (rhs shifted by `shift` * theta_i
/// on the upper bounds) plus `extra` rows (rhs `extra_b` + `shift` * sum of theta over the support).
fn projection(extra: &[Vec<f64>], extra_b: &[f64], n: usize, shift: f64) -> MpProgram {
    let mut rows: Vec<Vec<f64>> = vec![];
    let mut b = vec![];
    let mut f_rows: Vec<Vec<f64>> = vec![];
    for i in 0..n {
        let mut r = vec![0.0; n];
        r[i] = 1.0;
        rows.push(r.clone());
        b.push(1.0);
        let mut fr = vec![0.0; n];
        fr[i] = shift;
        f_rows.push(fr);
        r[i] = -1.0;
        rows.push(r);
        b.push(1.0);
        f_rows.push(vec![0.0; n]);
    }
    for (r, &eb) in extra.iter().zip(extra_b) {
        rows.push(r.clone());
        b.push(eb);
        f_rows.push(r.iter().map(|v| if *v > 0.0 { shift * v } else { 0.0 }).collect());
    }
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
    MpProgram::new_mpqp(a, b, vec![0.0; n], h, Mat::eye(n), a_t, b_t, Mat::from_rows(&f_rows), vec![])
}

fn overlap_gap(regions: &[CriticalRegion], nt: usize, samples: usize) -> (usize, usize) {
    let mut rng = StdRng::seed_from_u64(11);
    let (mut overlap, mut gap) = (0, 0);
    for _ in 0..samples {
        let th: Vec<f64> = (0..nt).map(|_| rng.gen_range(-3.99..3.99)).collect();
        let (mut inside, mut touching) = (0, 0);
        for r in regions {
            let worst = (0..r.e.rows).map(|i| r.e.row(i).iter().zip(&th).map(|(a, b)| a * b).sum::<f64>() - r.f[i]).fold(f64::NEG_INFINITY, f64::max);
            if worst < -1e-7 {
                inside += 1;
            }
            if worst < 1e-7 {
                touching += 1;
            }
        }
        if inside >= 2 {
            overlap += 1;
        }
        if touching == 0 {
            gap += 1;
        }
    }
    (overlap, gap)
}

fn check(name: &str, p: MpProgram) {
    let p = Arc::new(p);
    let nt = p.num_t();
    let mut sigs = vec![];
    for alg in [MpqpAlgorithm::Geometric, MpqpAlgorithm::GeometricParallel] {
        let s = solve_mpqp(&p, alg);
        let (ov, gap) = overlap_gap(&s.critical_regions, nt, 20_000);
        eprintln!("{name} {:?}: {} regions, overlap {ov}, gap {gap}", alg, s.len());
        assert_eq!(gap, 0, "{name} {:?}: gaps", alg);
        assert_eq!(ov, 0, "{name} {:?}: overlaps", alg);
        let mut rng = StdRng::seed_from_u64(3);
        for _ in 0..1000 {
            let th: Vec<f64> = (0..nt).map(|_| rng.gen_range(-3.9..3.9)).collect();
            assert!(s.verify_theta(&th), "{name} theta {:?}", th);
        }
        sigs.push(region_signature(&s.critical_regions));
    }
    assert_eq!(sigs[0], sigs[1], "{name}: serial and parallel differ");
}

#[test]
fn vertex_2d() {
    check("vertex_2d", projection(&[vec![1.0, 1.0]], &[2.0], 2, 0.0));
}

#[test]
fn vertex_3d() {
    let extra = vec![vec![1.0, 1.0, 0.0], vec![0.0, 1.0, 1.0], vec![1.0, 0.0, 1.0], vec![1.0, 1.0, 1.0]];
    check("vertex_3d", projection(&extra, &[2.0, 2.0, 2.0, 3.0], 3, 0.0));
}

#[test]
fn vertex_3d_two_planes() {
    check("vertex_3d_two_planes", projection(&[vec![1.0, 1.0, 0.0], vec![1.0, 1.0, 1.0]], &[2.0, 3.0], 3, 0.0));
}

/// Degenerate on a full-dimensional set of theta although the right-hand side depends on theta.
#[test]
fn vertex_2d_parametric_rhs() {
    check("vertex_2d_parametric_rhs", projection(&[vec![1.0, 1.0]], &[2.0], 2, 0.3));
}

#[test]
fn vertex_3d_parametric_rhs() {
    check("vertex_3d_parametric_rhs", projection(&[vec![1.0, 1.0, 0.0], vec![0.0, 1.0, 1.0]], &[2.0, 2.0], 3, 0.25));
}

/// Duplicate constraint rows: the active set at the vertex is rank deficient (LICQ violated).
#[test]
fn duplicate_rows_licq_violation() {
    check("duplicate_rows", projection(&[vec![1.0, 0.0], vec![0.0, 1.0]], &[1.0, 1.0], 2, 0.0));
}

/// A non-binding-in-the-interior plane touching the box along an edge (weakly active constraint:
/// its multiplier is zero on a full-dimensional set of theta).
#[test]
fn weakly_active_edge() {
    check("weakly_active_edge", projection(&[vec![1.0, 0.0]], &[1.0], 2, 0.0));
}

/// A plane through a vertex that cuts nothing off but is dependent with two facets (non-facet-to-facet).
#[test]
fn vertex_2d_scaled_row() {
    check("vertex_2d_scaled_row", projection(&[vec![2.0, 2.0], vec![0.0, -3.0]], &[4.0, 3.0], 2, 0.0));
}

/// Random planes through the vertex (1, 1, 1) (integer coefficients), with and without a
/// theta-dependent right-hand side.
#[test]
fn random_vertex_planes_3d() {
    let mut rng = StdRng::seed_from_u64(2024);
    for case in 0..12 {
        let k = rng.gen_range(1..=4);
        let mut extra = vec![];
        let mut eb = vec![];
        for _ in 0..k {
            let r: Vec<f64> = (0..3).map(|_| rng.gen_range(0..=3) as f64).collect();
            if r.iter().all(|v| *v == 0.0) {
                continue;
            }
            eb.push(r.iter().sum::<f64>());
            extra.push(r);
        }
        let shift = if case % 2 == 0 { 0.0 } else { 0.2 };
        check(&format!("random3d_{case}"), projection(&extra, &eb, 3, shift));
    }
}

#[test]
fn nondegenerate_unchanged_by_completion() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/problems");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map_or(true, |e| e != "json") {
            continue;
        }
        let p = Arc::new(MpProgram::from_json_file(&path).unwrap());
        let g = solve_mpqp(&p, MpqpAlgorithm::Geometric);
        let gp = solve_mpqp(&p, MpqpAlgorithm::GeometricParallel);
        assert_eq!(region_signature(&g.critical_regions), region_signature(&gp.critical_regions), "{:?}", path);
    }
}
