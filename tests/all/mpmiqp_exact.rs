//! Tests for the exact mp-MIQP solution (Oberdieck and Pistikopoulos 2015).
//!
//! Gates for every example: per-point agreement with a direct MIQP solve, coverage without gaps or
//! ambiguity, and regression against enumeration with min-selection.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::mpmiqp::{solve_mpmiqp_enumeration, MpmiProgram};
use rpopt::algorithms::mpmiqp_exact::*;
use rpopt::algorithms::{MpqpAlgorithm, SolveOptions};
use rpopt::linalg::Mat;
use std::sync::Arc;
use std::time::Instant;

fn rows(r: &[&[f64]]) -> Mat {
    Mat::from_rows(&r.iter().map(|x| x.to_vec()).collect::<Vec<_>>())
}

struct Report {
    name: String,
    enum_regions: usize,
    st: ExactStats,
    max_val_err: f64,
    feasible: usize,
}

/// Runs the three gates on `n` random thetas drawn from the box `[lo, hi]`.
fn gates(name: &str, p: &Arc<MpmiProgram>, lo: &[f64], hi: &[f64], n: usize, seed: u64) -> Report {
    let opts = SolveOptions::default();
    let t0 = Instant::now();
    let (ex, st, en) = solve_mpmiqp_exact(p, MpqpAlgorithm::Combinatorial, &opts);
    let total = t0.elapsed().as_secs_f64();
    println!(
        "[{name}] enumeration {} regions ({:.3}s), exact {} pieces ({} non-convex, max {} quadrics), compare {:.3}s, total {:.3}s, {} emptiness checks, {} bb nodes, {} capped (busy machine)",
        st.enumeration_regions, st.enum_secs, st.pieces, st.nonconvex_pieces, st.max_quads, st.compare_secs, total, st.emptiness_checks, st.bb_nodes, st.bb_capped
    );
    let mut rng = StdRng::seed_from_u64(seed);
    let mut max_err = 0.0f64;
    let mut feasible = 0;
    for _ in 0..n {
        let th: Vec<f64> = lo.iter().zip(hi).map(|(a, b)| rng.gen_range(*a..*b)).collect();
        let direct = p.solve_theta(&th);
        let loose = ex.count_containing(&th, 2e-6);
        let strict = ex.count_containing(&th, -1e-7);
        match direct {
            Some(d) => {
                feasible += 1;
                // coverage: no gap, and no ambiguity away from boundaries
                assert!(loose >= 1, "[{name}] gap at {th:?}");
                assert!(strict <= 1, "[{name}] ambiguity ({strict} pieces) at {th:?}");
                // per-point: value and optimizer
                let i = ex.locate_robust(&th).unwrap_or_else(|| panic!("[{name}] theta {th:?} not located"));
                let x = ex.pieces[i].region.evaluate(&th);
                let v = ex.pieces[i].z.value(&th);
                let err = (v - d.obj).abs() / d.obj.abs().max(1.0);
                max_err = max_err.max(err);
                assert!(err <= 1e-7, "[{name}] value {v} vs direct {} at {th:?}", d.obj);
                for (k, (a, b)) in x.iter().zip(&d.sol).enumerate() {
                    assert!((a - b).abs() <= 1e-6 * b.abs().max(1.0), "[{name}] x[{k}] {a} vs {b} at {th:?}");
                }
                // regression against enumeration with min-selection
                let ve = en.evaluate_objective(&th).expect("enumeration covers theta");
                assert!((ve - v).abs() <= 1e-7 * ve.abs().max(1.0), "[{name}] enumeration {ve} vs exact {v} at {th:?}");
            }
            None => assert!(strict == 0, "[{name}] piece contains infeasible theta {th:?}"),
        }
    }
    assert!(feasible > 0, "[{name}] no feasible sample");
    Report { name: name.to_string(), enum_regions: en.len(), st, max_val_err: max_err, feasible }
}

// ------------------------------------------------------------------ Example 1 of the paper

/// Oberdieck and Pistikopoulos (2015), Eqs. (23)-(25): z = (Q w + H theta + c)' w, w = (x, y).
fn paper_example_1() -> MpmiProgram {
    let q = rows(&[&[6., 0., 0., 0.], &[-1., 4., 0., 0.], &[0., 0., 6., 0.], &[0., 0., -1., 1.]]);
    let qs = q.add(&q.transpose()); // w'Qw = 1/2 w'(Q+Q')w
    let h = rows(&[&[5., 0.], &[0., -8.], &[0., 0.], &[3., 0.]]);
    let a = rows(&[
        &[1., 1., 0., 0.],
        &[1., 0., -1., 0.],
        &[0., 1., 0., -1.],
        &[-1., 0., 0., 0.],
        &[0., -1., 0., 0.],
    ]);
    let b = vec![1.5, 1., 1., 1., 1.];
    let f = rows(&[&[0., 0.], &[0., 0.], &[0., 0.], &[0., 1.], &[1., 0.]]);
    let a_t = rows(&[&[1., 0.], &[0., 1.], &[-1., 0.], &[0., -1.]]);
    MpmiProgram::new(a, b, vec![0.; 4], h, Some(qs), a_t, vec![1., 1., 0., 0.], f, vec![2, 3], vec![])
}

#[test]
fn paper_example_1_exact() {
    let p = Arc::new(paper_example_1());
    let r = gates("paper ex1", &p, &[0., 0.], &[1., 1.], 2000, 1);
    println!("paper ex1: pieces {} (paper Table 1 lists 4), max value error {:.2e}", r.st.pieces, r.max_val_err);
}

// ------------------------------------------------------------------ Example 2 of the paper

/// Hybrid MPC example (33), N = 2, Q = 0.01 I, R = 1, P = I, reformulated with big-M.
/// Variables: u0, u1, z0 (2), z1 (2), d0, d1 (binary switch, d = 1 iff x_1 >= 0). theta = x(0).
fn hybrid_mpc() -> MpmiProgram {
    hybrid_mpc_with(false)
}

/// `free_switch = true` drops the guard `d = 1 iff x_1 >= 0` so that the mode is a decision
/// variable: every combination is feasible everywhere and the regions overlap heavily.
fn hybrid_mpc_with(free_switch: bool) -> MpmiProgram {
    let big = 30.0;
    let (c, s) = ((std::f64::consts::PI / 3.0).cos(), (std::f64::consts::PI / 3.0).sin());
    let a1 = [[0.8 * c, -0.8 * s], [0.8 * s, 0.8 * c]]; // alpha = pi/3
    let a2 = [[0.8 * c, 0.8 * s], [-0.8 * s, 0.8 * c]]; // alpha = -pi/3
    let nx = 8;
    let (iu, iz, id) = ([0usize, 1], [[2usize, 3], [4, 5]], [6usize, 7]);
    let mut a: Vec<Vec<f64>> = vec![];
    let mut b: Vec<f64> = vec![];
    let mut f: Vec<Vec<f64>> = vec![];
    for k in 0..2 {
        for (mode, am) in [(1.0, &a1), (0.0, &a2)] {
            // sign * (z_k - A x_k - B u_k) <= big * (1 -+ d)
            for sign in [1.0, -1.0] {
                for r in 0..2 {
                    let mut row = vec![0.0; nx];
                    let mut frow = vec![0.0; 2];
                    row[iz[k][r]] = sign;
                    if r == 1 {
                        row[iu[k]] = -sign; // B = [0; 1]
                    }
                    for cidx in 0..2 {
                        let coef = -sign * am[r][cidx];
                        if k == 0 {
                            frow[cidx] = -coef; // coef * theta moved to the right
                        } else {
                            row[iz[0][cidx]] += coef;
                        }
                    }
                    // d = 1 selects mode 1: relaxed when d = 0 (row + big*d <= big); mode 0 relaxed when d = 1
                    if mode == 1.0 {
                        row[id[k]] = big;
                        b.push(big);
                    } else {
                        row[id[k]] = -big;
                        b.push(0.0);
                    }
                    a.push(row);
                    f.push(frow);
                }
            }
        }
        // d = 1 -> x_1 >= 0:  -x_1 + big d <= big ; d = 0 -> x_1 <= 0:  x_1 - big d <= 0
        let mut r1 = vec![0.0; nx];
        let mut f1 = vec![0.0; 2];
        let mut r2 = vec![0.0; nx];
        let mut f2 = vec![0.0; 2];
        if k == 0 {
            f1[0] = 1.0;
            f2[0] = -1.0;
        } else {
            r1[iz[0][0]] = -1.0;
            r2[iz[0][0]] = 1.0;
        }
        r1[id[k]] = big;
        r2[id[k]] = -big;
        if !free_switch {
            a.push(r1);
            b.push(big);
            f.push(f1);
            a.push(r2);
            b.push(0.0);
            f.push(f2);
        }
    }
    // bounds
    let push = |var: usize, sign: f64, rhs: f64, a: &mut Vec<Vec<f64>>, b: &mut Vec<f64>, f: &mut Vec<Vec<f64>>| {
        let mut r = vec![0.0; nx];
        r[var] = sign;
        a.push(r);
        b.push(rhs);
        f.push(vec![0.0; 2]);
    };
    for v in 0..2 {
        push(v, 1.0, 1.0, &mut a, &mut b, &mut f);
        push(v, -1.0, 1.0, &mut a, &mut b, &mut f);
    }
    for v in 2..6 {
        push(v, 1.0, 10.0, &mut a, &mut b, &mut f);
        push(v, -1.0, 10.0, &mut a, &mut b, &mut f);
    }
    for v in 6..8 {
        push(v, 1.0, 1.0, &mut a, &mut b, &mut f);
        push(v, -1.0, 0.0, &mut a, &mut b, &mut f);
    }
    let mut q = Mat::zeros(nx, nx);
    for (i, v) in [2.0, 2.0, 0.02, 0.02, 2.0, 2.0, 1e-6, 1e-6].iter().enumerate() {
        q[(i, i)] = *v;
    }
    let a_t = rows(&[&[1., 0.], &[0., 1.], &[-1., 0.], &[0., -1.]]);
    let mut p = MpmiProgram::new(
        Mat::from_rows(&a),
        b,
        vec![0.0; nx],
        Mat::zeros(nx, 2),
        Some(q),
        a_t,
        vec![10., 10., 10., 10.],
        Mat::from_rows(&f),
        vec![6, 7],
        vec![],
    );
    p.base.q_t = rows(&[&[0.02, 0.], &[0., 0.02]]);
    p
}

#[test]
fn paper_example_2_hybrid_mpc() {
    let p = Arc::new(hybrid_mpc());
    let r = gates("hybrid mpc", &p, &[-10., -10.], &[10., 10.], 2000, 2);
    println!("hybrid mpc: enumeration {} -> exact {} pieces (paper Table 2: 40 CRs for None, 48 for Exact)", r.enum_regions, r.st.pieces);
}

// ------------------------------------------------------------------ hand made

/// min (x - 1 - theta)^2/2 + 3 y ... two binaries compete with quadratics of different curvature.
fn two_branch() -> MpmiProgram {
    // x in [-3,3]; y in {0,1}. cost 1/2*(1+3y) x^2 + (y-2)*theta*x... via H; cost c_y on y
    let a = rows(&[&[1., 0.], &[-1., 0.], &[0., 1.], &[0., -1.]]);
    let b = vec![3., 3., 1., 0.];
    let h = rows(&[&[-1.], &[0.]]);
    let mut q = Mat::zeros(2, 2);
    q[(0, 0)] = 1.0;
    q[(0, 1)] = 1.5;
    q[(1, 0)] = 1.5;
    q[(1, 1)] = 3.0;
    let a_t = rows(&[&[1.], &[-1.]]);
    MpmiProgram::new(a, b, vec![0.5, -0.3], h, Some(q), a_t, vec![2., 2.], Mat::zeros(4, 1), vec![1], vec![])
}

#[test]
fn hand_made_two_branch() {
    let p = Arc::new(two_branch());
    gates("two branch", &p, &[-2.], &[2.], 2000, 3);
}

// ------------------------------------------------------------------ generated

fn generate(seed: u64, nc: usize, nb: usize, nt: usize, extra: usize) -> MpmiProgram {
    let mut rng = StdRng::seed_from_u64(seed);
    let nx = nc + nb;
    let mut a: Vec<Vec<f64>> = vec![];
    let mut b = vec![];
    let mut f: Vec<Vec<f64>> = vec![];
    for i in 0..nx {
        for s in [1.0, -1.0] {
            let mut r = vec![0.0; nx];
            r[i] = s;
            a.push(r);
            b.push(if i < nc { 4.0 } else if s > 0.0 { 1.0 } else { 0.0 });
            f.push(vec![0.0; nt]);
        }
    }
    for _ in 0..extra {
        let r: Vec<f64> = (0..nx).map(|_| rng.gen_range(-1.5..1.5)).collect();
        let fr: Vec<f64> = (0..nt).map(|_| rng.gen_range(-1.0..1.0)).collect();
        b.push(rng.gen_range(1.0..3.0));
        a.push(r);
        f.push(fr);
    }
    let bm = Mat::from_vec(nx, nx, (0..nx * nx).map(|_| rng.gen_range(-1.0..1.0)).collect());
    let mut q = bm.transpose().matmul(&bm);
    for i in 0..nx {
        q[(i, i)] += 0.5;
    }
    let h = Mat::from_vec(nx, nt, (0..nx * nt).map(|_| rng.gen_range(-2.0..2.0)).collect());
    let c: Vec<f64> = (0..nx).map(|i| if i < nc { rng.gen_range(-1.0..1.0) } else { rng.gen_range(-2.0..2.0) }).collect();
    let a_t = {
        let mut m = Mat::zeros(0, nt);
        for j in 0..nt {
            let mut r = vec![0.0; nt];
            r[j] = 1.0;
            m.push_row(&r);
            r[j] = -1.0;
            m.push_row(&r);
        }
        m
    };
    MpmiProgram::new(Mat::from_rows(&a), b, c, h, Some(q), a_t, vec![1.0; 2 * nt], Mat::from_rows(&f), (nc..nx).collect(), vec![])
}

fn gen_case(seed: u64, nc: usize, nb: usize, nt: usize) {
    let p = Arc::new(generate(seed, nc, nb, nt, 3));
    let r = gates(&format!("gen s{seed} {nc}c {nb}b {nt}t"), &p, &vec![-1.0; nt], &vec![1.0; nt], 2000, seed);
    println!("| {} | {} | {} | {} | {:.3} | {:.3} | err {:.1e}, feasible {} |", r.name, r.enum_regions, r.st.pieces, r.st.nonconvex_pieces, r.st.enum_secs, r.st.compare_secs, r.max_val_err, r.feasible);
}

#[test]
fn hybrid_mpc_free_switching() {
    let p = Arc::new(hybrid_mpc_with(true));
    let r = gates("hybrid mpc free switch", &p, &[-10., -10.], &[10., 10.], 2000, 4);
    println!("hybrid free switch: enumeration {} -> exact {} pieces", r.enum_regions, r.st.pieces);
}

#[test]
fn generated_s17() {
    gen_case(17, 3, 4, 2);
}
#[test]
fn generated_s18() {
    gen_case(18, 2, 5, 2);
}
#[test]
fn generated_s19() {
    gen_case(19, 3, 3, 3);
}

#[test]
fn generated_s11() {
    gen_case(11, 2, 2, 2);
}
#[test]
fn generated_s12() {
    gen_case(12, 2, 3, 2);
}
#[test]
fn generated_s13() {
    gen_case(13, 3, 2, 2);
}
#[test]
fn generated_s14() {
    gen_case(14, 2, 3, 1);
}
#[test]
fn generated_s15() {
    gen_case(15, 3, 3, 2);
}
#[test]
fn generated_s16() {
    gen_case(16, 2, 4, 2);
}

/// With identical Hessians the boundary is a hyperplane: no quadrics may appear.
#[test]
fn equal_curvature_gives_polyhedral_pieces() {
    // theta enters only through the linear cost of the binary: z differs by an affine function
    let a = rows(&[&[1., 0.], &[-1., 0.], &[0., 1.], &[0., -1.]]);
    let b = vec![3., 3., 1., 0.];
    let h = rows(&[&[0.], &[2.]]);
    let mut q = Mat::zeros(2, 2);
    q[(0, 0)] = 2.0;
    let a_t = rows(&[&[1.], &[-1.]]);
    let p = Arc::new(MpmiProgram::new(a, b, vec![0., 0.], h, Some(q), a_t, vec![2., 2.], Mat::zeros(4, 1), vec![1], vec![]));
    let en = solve_mpmiqp_enumeration(&p, MpqpAlgorithm::Combinatorial, &SolveOptions::default());
    let (ex, st) = compare_regions_exact(&p, &en);
    assert_eq!(st.nonconvex_pieces, 0);
    assert!(!ex.is_empty());
}
