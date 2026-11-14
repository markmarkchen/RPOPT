//! Lexicographic perturbation mpLP (Jones, Kerrigan, Maciejowski 2007): same regions as the
//! reference algorithms on nondegenerate mpLPs; partition without gaps or overlaps, optimal
//! values equal to direct LP solves and a continuous optimizer on degenerate ones.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::critical_region::CriticalRegion;
use rpopt::linalg::Mat;
use rpopt::problem_generator::{generate_mplp, generate_mpqp};
use rpopt::program::MpProgram;
use std::sync::Arc;

/// (points strictly inside 2+ regions, points in no region) among samples of [-3.99, 3.99]^nt
/// that the program can solve (feasible), margin 1e-7.
fn overlap_gap(p: &MpProgram, regions: &[CriticalRegion], samples: usize) -> (usize, usize) {
    let nt = p.num_t();
    let mut rng = StdRng::seed_from_u64(11);
    let (mut overlap, mut gap) = (0, 0);
    for _ in 0..samples {
        let th: Vec<f64> = (0..nt).map(|_| rng.gen_range(-3.99..3.99)).collect();
        if !p.valid_parameter_realization(&th) {
            continue;
        }
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
        if touching == 0 && p.solve_theta_uncounted(&th).is_some() {
            gap += 1;
        }
    }
    (overlap, gap)
}

/// Largest relative objective difference to direct LP solves over `queries` samples, and the
/// number of feasibility disagreements.
fn value_error(p: &Arc<MpProgram>, regions: Vec<CriticalRegion>, queries: usize) -> (f64, usize) {
    let mut s = rpopt::solution::Solution::new(p.clone(), regions);
    s.is_overlapping = false;
    s.point_location_tolerance = 1e-9;
    let mut rng = StdRng::seed_from_u64(5);
    let (mut worst, mut miss) = (0.0f64, 0usize);
    for _ in 0..queries {
        let th: Vec<f64> = (0..p.num_t()).map(|_| rng.gen_range(-3.9..3.9)).collect();
        match (p.solve_theta_uncounted(&th), s.evaluate_objective(&th)) {
            (Some(d), Some(o)) => worst = worst.max((d.obj - o).abs() / (1.0 + d.obj.abs())),
            (None, None) => {}
            _ => miss += 1,
        }
    }
    (worst, miss)
}

/// x*(θ) jumps along random short segments, relative to the largest slope of any region.
fn continuity(regions: &[CriticalRegion], nt: usize) -> f64 {
    let slope = regions.iter().map(|r| r.a.data.iter().map(|v| v * v).sum::<f64>().sqrt()).fold(0.0, f64::max);
    let mut rng = StdRng::seed_from_u64(9);
    let h = 1e-4;
    let mut worst = 0.0f64;
    for _ in 0..4000 {
        let th: Vec<f64> = (0..nt).map(|_| rng.gen_range(-3.9..3.9)).collect();
        let dir: Vec<f64> = (0..nt).map(|_| rng.gen_range(-1.0..1.0)).collect();
        let th2: Vec<f64> = th.iter().zip(&dir).map(|(a, d)| a + h * d).collect();
        let loc = |t: &[f64]| regions.iter().find(|r| r.is_inside(t, 1e-9)).map(|r| r.evaluate(t));
        if let (Some(x1), Some(x2)) = (loc(&th), loc(&th2)) {
            let jump = x1.iter().zip(&x2).map(|(a, b)| (a - b).powi(2)).sum::<f64>().sqrt();
            let step = h * dir.iter().map(|d| d * d).sum::<f64>().sqrt();
            worst = worst.max(jump / (slope * step + 1e-12));
        }
    }
    worst
}

fn theta_box(nt: usize, r: f64) -> (Mat, Vec<f64>) {
    let mut a_t = Mat::zeros(2 * nt, nt);
    let mut b_t = vec![];
    for i in 0..nt {
        a_t[(2 * i, i)] = 1.0;
        a_t[(2 * i + 1, i)] = -1.0;
        b_t.push(r);
        b_t.push(r);
    }
    (a_t, b_t)
}

struct Report {
    lex: usize,
    cont: f64,
}

fn check_degenerate(name: &str, p: MpProgram, continuous: bool) -> Report {
    let p = Arc::new(p);
    let t0 = std::time::Instant::now();
    let lex = solve_mpqp(&p, MpqpAlgorithm::MplpLex);
    let t_lex = t0.elapsed();
    let comb = solve_mpqp(&p, MpqpAlgorithm::Combinatorial);
    let geo = solve_mpqp(&p, MpqpAlgorithm::Geometric);
    let enm = solve_mpqp(&p, MpqpAlgorithm::MplcpEnum);
    let lex_og = overlap_gap(&p, &lex.critical_regions, 20_000);
    let comb_og = overlap_gap(&p, &comb.critical_regions, 20_000);
    let geo_og = overlap_gap(&p, &geo.critical_regions, 20_000);
    let enm_og = overlap_gap(&p, &enm.critical_regions, 20_000);
    let value = value_error(&p, lex.critical_regions.clone(), 2000);
    let cont = if continuous { continuity(&lex.critical_regions, p.num_t()) } else { f64::NAN };
    eprintln!(
        "{:<24} regions lex {:>3} comb {:>3} geo {:>3} enum {:>3} | ov/gap lex {:?} comb {:?} geo {:?} enum {:?} | value err {:.1e} miss {} | jump/slope {:.3} | lex {:.1?}",
        name,
        lex.len(),
        comb.len(),
        geo.len(),
        enm.len(),
        lex_og,
        comb_og,
        geo_og,
        enm_og,
        value.0,
        value.1,
        cont,
        t_lex
    );
    assert!(lex.len() > 0, "{name}: no regions");
    assert_eq!(lex_og.0, 0, "{name}: overlapping regions");
    assert_eq!(lex_og.1, 0, "{name}: gaps");
    assert!(value.0 < 1e-6 && value.1 == 0, "{name}: value mismatch {:?}", value);
    if continuous {
        assert!(cont < 1.01, "{name}: optimizer jumps, ratio {cont}");
    }
    Report { lex: lex.len(), cont }
}

#[test]
fn nondegenerate_matches_combinatorial_and_geometric() {
    let mut n = 0;
    for (x, t, m) in [(2, 2, 4), (3, 2, 6), (3, 3, 5), (4, 3, 8), (4, 4, 8), (5, 3, 10)] {
        for seed in 0..5u64 {
            let p = Arc::new(generate_mplp(x, t, m, seed));
            let l = region_signature(&solve_mpqp(&p, MpqpAlgorithm::MplpLex).critical_regions);
            let c = region_signature(&solve_mpqp(&p, MpqpAlgorithm::Combinatorial).critical_regions);
            let g = region_signature(&solve_mpqp(&p, MpqpAlgorithm::Geometric).critical_regions);
            eprintln!("mpLP x{x} t{t} m{m} seed {seed}: lex {} comb {} geo {}", l.len(), c.len(), g.len());
            if c == g {
                assert_eq!(l, c, "lex vs combinatorial, x{x} t{t} m{m} seed {seed}");
                n += 1;
            }
        }
    }
    assert!(n >= 20, "only {n} problems where combinatorial and geometric agree");
}

/// Larger generated mpLPs: timings against combinatorial and geometric (printed), and the same
/// regions as the combinatorial algorithm whenever it agrees with the geometric one.
#[test]
fn larger_mplps_timing() {
    for (x, t, m, seed) in [(6, 3, 12, 0u64), (6, 4, 14, 1), (8, 4, 16, 2), (8, 3, 18, 3), (10, 4, 20, 4)] {
        let p = Arc::new(generate_mplp(x, t, m, seed));
        let time = |a: MpqpAlgorithm| {
            let t0 = std::time::Instant::now();
            let s = solve_mpqp(&p, a);
            (region_signature(&s.critical_regions), t0.elapsed())
        };
        let (l, tl) = time(MpqpAlgorithm::MplpLex);
        let (g, tg) = time(MpqpAlgorithm::Geometric);
        let (c, tc) = time(MpqpAlgorithm::Combinatorial);
        eprintln!("mpLP x{x} t{t} m{m}: regions lex {} geo {} comb {} | lex {:.1?} geo {:.1?} comb {:.1?}", l.len(), g.len(), c.len(), tl, tg, tc);
        if c == g {
            assert_eq!(l, c);
        }
    }
}

#[test]
fn mpqp_delegates_without_panicking() {
    let p = Arc::new(generate_mpqp(3, 2, 6, 1));
    let l = region_signature(&solve_mpqp(&p, MpqpAlgorithm::MplpLex).critical_regions);
    let g = region_signature(&solve_mpqp(&p, MpqpAlgorithm::Geometric).critical_regions);
    assert_eq!(l, g);
}

/// min θ'x over a square with an extra plane through the vertex (1,1) (tests/mplcp.rs).
fn square_with_plane() -> MpProgram {
    let a = Mat::from_rows(&[vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0], vec![-1.0, 0.0], vec![0.0, -1.0]]);
    let b = vec![1.0, 1.0, 2.0, 1.0, 1.0];
    let (a_t, b_t) = theta_box(2, 4.0);
    MpProgram::new_mplp(a, b, vec![0.0; 2], Mat::eye(2), a_t, b_t, Mat::zeros(5, 2), vec![])
}

#[test]
fn degenerate_square_plane_vertex() {
    check_degenerate("square+plane", square_with_plane(), false);
}

/// Cost theta1 * x1 only: for every theta the optimal x2 is non-unique (dual degenerate).
#[test]
fn dual_degenerate_free_direction() {
    let a = Mat::from_rows(&[vec![1.0, 0.0], vec![-1.0, 0.0], vec![0.0, 1.0], vec![0.0, -1.0], vec![1.0, 1.0]]);
    let b = vec![1.0, 1.0, 1.0, 1.0, 1.5];
    let h = Mat::from_rows(&[vec![1.0, 0.0], vec![0.0, 0.0]]);
    let (a_t, b_t) = theta_box(2, 4.0);
    check_degenerate("dual-degenerate", MpProgram::new_mplp(a, b, vec![0.0; 2], h, a_t, b_t, Mat::zeros(5, 2), vec![]), false);
}

/// Cost over a cube whose corners have extra planes through them (3 variables).
#[test]
fn dual_and_primal_degenerate_3d() {
    let mut rows = vec![];
    let mut b = vec![];
    for i in 0..3 {
        for s in [1.0, -1.0] {
            let mut r = vec![0.0; 3];
            r[i] = s;
            rows.push(r);
            b.push(1.0);
        }
    }
    rows.push(vec![1.0, 1.0, 0.0]);
    b.push(2.0);
    rows.push(vec![0.0, 1.0, 1.0]);
    b.push(2.0);
    rows.push(vec![1.0, 1.0, 1.0]);
    b.push(3.0);
    let m = rows.len();
    let h = Mat::from_rows(&[vec![1.0, 0.0], vec![0.0, 1.0], vec![0.0, 0.0]]);
    let (a_t, b_t) = theta_box(2, 4.0);
    check_degenerate("cube-3d", MpProgram::new_mplp(Mat::from_rows(&rows), b, vec![0.0; 3], h, a_t, b_t, Mat::zeros(m, 2), vec![]), false);
}

/// Parameter in the right hand side only; three constraints always meet at (1+θ1, 1+θ2):
/// primal degenerate for every θ. The optimizer must be continuous.
#[test]
fn primal_degenerate_rhs_continuous() {
    let a = Mat::from_rows(&[vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0], vec![-1.0, 0.0], vec![0.0, -1.0]]);
    let b = vec![1.0, 1.0, 2.0, 1.0, 1.0];
    let f = Mat::from_rows(&[vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0], vec![0.5, 0.0], vec![0.0, 0.5]]);
    let (a_t, b_t) = theta_box(2, 3.0);
    let r = check_degenerate("primal-degenerate", MpProgram::new_mplp(a, b, vec![-1.0, -1.0], Mat::zeros(2, 2), a_t, b_t, f, vec![]), true);
    assert!(r.cont.is_finite() && r.lex > 0);
}

/// Primal and dual degenerate with the parameter in the right hand side: zero cost on x2 (the
/// optimal x2 is a segment), continuous x*.
#[test]
fn dual_and_primal_degenerate_rhs_continuous() {
    let a = Mat::from_rows(&[vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0], vec![-1.0, 0.0], vec![0.0, -1.0]]);
    let b = vec![1.0, 1.0, 2.0, 1.0, 1.0];
    let f = Mat::from_rows(&[vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0], vec![0.0, 0.0], vec![0.0, 0.0]]);
    let (a_t, b_t) = theta_box(2, 3.0);
    check_degenerate("rhs-both-degenerate", MpProgram::new_mplp(a, b, vec![-1.0, 0.0], Mat::zeros(2, 2), a_t, b_t, f, vec![]), true);
}

/// Equality constraint x1 + x2 + x3 = 1 + θ1 over a degenerate polytope.
#[test]
fn equality_constraints() {
    let mut rows = vec![vec![1.0, 1.0, 1.0]];
    let mut b = vec![1.0];
    let mut f = vec![vec![1.0, 0.0]];
    for i in 0..3 {
        let mut r = vec![0.0; 3];
        r[i] = -1.0;
        rows.push(r);
        b.push(0.0);
        f.push(vec![0.0, 0.0]);
    }
    rows.push(vec![1.0, 1.0, 0.0]);
    b.push(1.0);
    f.push(vec![1.0, 0.0]);
    rows.push(vec![0.0, 1.0, 1.0]);
    b.push(1.0);
    f.push(vec![1.0, 0.0]);
    let (mut a_t, mut b_t) = theta_box(2, 3.0);
    // keep theta1 above -0.9 so that the program stays feasible
    a_t.push_row(&[-1.0, 0.0]);
    b_t.push(0.9);
    let h = Mat::from_rows(&[vec![0.0, 1.0], vec![0.0, 0.0], vec![0.0, -1.0]]);
    check_degenerate("equality", MpProgram::new_mplp(Mat::from_rows(&rows), b, vec![0.0, 0.1, 0.0], h, a_t, b_t, Mat::from_rows(&f), vec![0]), false);
}

/// Random small-integer mpLPs with the parameter only in the right hand side (H = 0): the
/// optimizer x*(θ) is continuous (Theorem 16).
#[test]
fn random_integer_rhs_only_continuous() {
    for seed in 0..25u64 {
        let mut rng = StdRng::seed_from_u64(5000 + seed);
        let nx = rng.gen_range(2..=3usize);
        let nt = rng.gen_range(2..=3usize);
        let mut rows = vec![];
        let mut b = vec![];
        let mut f = vec![];
        for i in 0..nx {
            for s in [1.0, -1.0] {
                let mut r = vec![0.0; nx];
                r[i] = s;
                rows.push(r);
                b.push(2.0);
                f.push((0..nt).map(|_| if rng.gen_bool(0.5) { 0.0 } else { rng.gen_range(-1..=1i32) as f64 * 0.5 }).collect::<Vec<f64>>());
            }
        }
        for _ in 0..rng.gen_range(2..=4usize) {
            let r: Vec<f64> = (0..nx).map(|_| rng.gen_range(-1..=1i32) as f64).collect();
            if r.iter().all(|v| *v == 0.0) {
                continue;
            }
            rows.push(r);
            b.push(rng.gen_range(1..=3i32) as f64);
            f.push((0..nt).map(|_| if rng.gen_bool(0.5) { 0.0 } else { rng.gen_range(-1..=1i32) as f64 * 0.5 }).collect());
        }
        let c: Vec<f64> = (0..nx).map(|_| if rng.gen_bool(0.4) { 0.0 } else { rng.gen_range(-2..=2i32) as f64 }).collect();
        let (a_t, b_t) = theta_box(nt, 3.0);
        let p = MpProgram::new_mplp(Mat::from_rows(&rows), b, c, Mat::zeros(nx, nt), a_t, b_t, Mat::from_rows(&f), vec![]);
        check_degenerate(&format!("rhs-only seed {seed}"), p, true);
    }
}

/// Random small-integer mpLPs (many ties): every seed must partition without gaps or overlaps.
#[test]
fn random_integer_degenerate() {
    for seed in 0..40u64 {
        let mut rng = StdRng::seed_from_u64(1000 + seed);
        let nx = rng.gen_range(2..=3usize);
        let nt = rng.gen_range(2..=3usize);
        let extra = rng.gen_range(2..=5usize);
        let mut rows = vec![];
        let mut b = vec![];
        let mut f = vec![];
        for i in 0..nx {
            for s in [1.0, -1.0] {
                let mut r = vec![0.0; nx];
                r[i] = s;
                rows.push(r);
                b.push(2.0);
                f.push(vec![0.0; nt]);
            }
        }
        for _ in 0..extra {
            let r: Vec<f64> = (0..nx).map(|_| rng.gen_range(-1..=1i32) as f64).collect();
            if r.iter().all(|v| *v == 0.0) {
                continue;
            }
            rows.push(r);
            b.push(rng.gen_range(1..=3i32) as f64);
            f.push((0..nt).map(|_| if rng.gen_bool(0.5) { 0.0 } else { rng.gen_range(-1..=1i32) as f64 * 0.5 }).collect());
        }
        let hm: Vec<Vec<f64>> = (0..nx).map(|_| (0..nt).map(|_| if rng.gen_bool(0.4) { 0.0 } else { rng.gen_range(-2..=2i32) as f64 }).collect()).collect();
        let c: Vec<f64> = (0..nx).map(|_| if rng.gen_bool(0.5) { 0.0 } else { rng.gen_range(-1..=1i32) as f64 }).collect();
        let (a_t, b_t) = theta_box(nt, 3.0);
        let p = MpProgram::new_mplp(Mat::from_rows(&rows), b, c, Mat::from_rows(&hm), a_t, b_t, Mat::from_rows(&f), vec![]);
        check_degenerate(&format!("random-int seed {seed}"), p, false);
    }
}
