//! Tests for the comparison step and branch and bound on mpMILP.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::mpmilp_bb::*;
use rpopt::algorithms::mpmiqp::{solve_mpmiqp_enumeration, MpmiProgram};
use rpopt::algorithms::{MpqpAlgorithm, SolveOptions};
use rpopt::linalg::Mat;
use rpopt::solution::{Objective, Solution};
use std::sync::Arc;
use std::time::Instant;

fn rows(r: &[&[f64]]) -> Mat {
    Mat::from_rows(&r.iter().map(|x| x.to_vec()).collect::<Vec<_>>())
}

fn simple_mi() -> MpmiProgram {
    let a = rows(&[&[0., 1., 1.], &[1., 0., 0.], &[-1., 0., 0.], &[1., -1., 0.], &[1., 0., -1.]]);
    let b = vec![1., 0., 0., 0., 0.];
    let f = rows(&[&[0.], &[1.], &[0.], &[0.], &[0.]]);
    MpmiProgram::new(a, b, vec![-3., 0., 0.], Mat::zeros(3, 1), None, rows(&[&[1.], &[-1.]]), vec![2., 2.], f, vec![1, 2], vec![])
}

fn market() -> MpmiProgram {
    let a = rows(&[
        &[1., 1., 0., 0., 0.],
        &[0., 0., 1., 1., 0.],
        &[-1., 0., -1., 0., 0.],
        &[0., -1., 0., -1., -500.],
        &[-1., 0., 0., 0., 0.],
        &[0., -1., 0., 0., 0.],
        &[0., 0., -1., 0., 0.],
        &[0., 0., 0., -1., 0.],
        &[0., 0., 0., 0., -1.],
        &[0., 0., 0., 0., 1.],
    ]);
    let b = vec![350., 600., 0., 0., 0., 0., 0., 0., 0., 1.];
    let f = rows(&[&[0., 0.], &[0., 0.], &[-1., 0.], &[0., -1.], &[0., 0.], &[0., 0.], &[0., 0.], &[0., 0.], &[0., 0.], &[0., 0.]]);
    let a_t = rows(&[&[1., 0.], &[0., 1.], &[-1., 0.], &[0., -1.]]);
    MpmiProgram::new(a, b, vec![178., 187., 187., 151., 50000.], Mat::zeros(5, 2), None, a_t, vec![1000., 1000., 0., 0.], f, vec![4], vec![])
}

/// 1-D fixed-charge example: two plants (big-M capacities 6 and 8), recourse x3 at a high price.
fn one_d() -> MpmiProgram {
    let a = rows(&[
        &[1., 0., 0., -6., 0.],
        &[0., 1., 0., 0., -8.],
        &[-1., -1., -1., 0., 0.],
        &[0., 0., 1., 0., 0.],
        &[-1., 0., 0., 0., 0.],
        &[0., -1., 0., 0., 0.],
        &[0., 0., -1., 0., 0.],
        &[0., 0., 0., 1., 0.],
        &[0., 0., 0., 0., 1.],
        &[0., 0., 0., -1., 0.],
        &[0., 0., 0., 0., -1.],
    ]);
    let b = vec![0., 0., 0., 10., 0., 0., 0., 1., 1., 0., 0.];
    let mut f = Mat::zeros(11, 1);
    f[(2, 0)] = -1.0;
    MpmiProgram::new(a, b, vec![1.0, 1.5, 3.0, 4.0, 6.0], Mat::zeros(5, 1), None, rows(&[&[1.], &[-1.]]), vec![16., 0.], f, vec![3, 4], vec![])
}

fn theta_box(p: &MpmiProgram) -> (Vec<f64>, Vec<f64>) {
    let nt = p.num_t();
    let mut lo = vec![f64::NEG_INFINITY; nt];
    let mut hi = vec![f64::INFINITY; nt];
    for i in 0..p.base.a_t.rows {
        let r = p.base.a_t.row(i);
        let nz: Vec<usize> = (0..nt).filter(|&j| r[j] != 0.0).collect();
        if nz.len() == 1 {
            let j = nz[0];
            if r[j] > 0.0 {
                hi[j] = hi[j].min(p.base.b_t[i] / r[j]);
            } else {
                lo[j] = lo[j].max(p.base.b_t[i] / r[j]);
            }
        }
    }
    (lo, hi)
}

fn slack(sol: &Solution, ri: usize, th: &[f64]) -> f64 {
    let r = &sol.critical_regions[ri];
    (0..r.e.rows).map(|i| rpopt::linalg::dot(r.e.row(i), th) - r.f[i]).fold(f64::NEG_INFINITY, f64::max)
}

/// per-point MILP check, non-overlap and no-gap check; returns the number of feasible points.
fn check(p: &MpmiProgram, sol: &Solution, n: usize, seed: u64, label: &str) -> usize {
    let (lo, hi) = theta_box(p);
    let mut rng = StdRng::seed_from_u64(seed);
    let mut feas = 0;
    for _ in 0..n {
        let th: Vec<f64> = lo.iter().zip(&hi).map(|(l, h)| rng.gen_range(*l..*h)).collect();
        let direct = p.solve_theta(&th);
        let sl: Vec<f64> = (0..sol.critical_regions.len()).map(|i| slack(sol, i, &th)).collect();
        let strict: Vec<usize> = (0..sl.len()).filter(|&i| sl[i] <= -1e-7).collect();
        let loose = sl.iter().any(|&s| s <= 1e-7);
        assert!(strict.len() <= 1, "{label}: theta {th:?} in {} regions", strict.len());
        match direct {
            Some(d) => {
                feas += 1;
                assert!(loose, "{label}: gap at theta {th:?}");
                if let Some(&ri) = strict.first() {
                    let r = &sol.critical_regions[ri];
                    let x = r.evaluate(&th);
                    let obj = p.evaluate_objective(&x, &th);
                    assert!((obj - d.obj).abs() <= 1e-7 * d.obj.abs().max(1.0), "{label}: theta {th:?} explicit {obj} vs milp {}", d.obj);
                }
            }
            None => assert!(strict.is_empty(), "{label}: region at infeasible theta {th:?}"),
        }
    }
    feas
}

fn bb_alg() -> MpqpAlgorithm {
    match std::env::var("BBALG").as_deref() {
        Ok("comb") => MpqpAlgorithm::Combinatorial,
        Ok("geo") => MpqpAlgorithm::Geometric,
        Ok("cg") => MpqpAlgorithm::CombinatorialGraph,
        Ok("graph") => MpqpAlgorithm::Graph,
        _ => MpqpAlgorithm::MplpLex,
    }
}

struct Row {
    name: String,
    nb: usize,
    nt: usize,
    en_t: f64,
    en_regions: usize,
    en_nodes: usize,
    cmp_regions: usize,
    cmp_t: f64,
    bb_t: f64,
    bb_nodes: usize,
    bb_regions: usize,
    feas: usize,
}

fn run(name: &str, p: MpmiProgram, n: usize) -> Row {
    let p = Arc::new(p);
    let opts = SolveOptions::default();
    let alg = MpqpAlgorithm::Combinatorial;
    let t = Instant::now();
    let en = solve_mpmiqp_enumeration(&p, alg, &opts);
    let en_t = t.elapsed().as_secs_f64();
    let nodes = p.feasible_binary_combinations().len();
    let t = Instant::now();
    let cmp = compare_regions(&p, &en).unwrap();
    let cmp_t = t.elapsed().as_secs_f64();
    eprintln!("{name}: enum {en_t:.2}s {} regions, compare {cmp_t:.2}s {} regions", en.len(), cmp.len());
    let (bb, st) = solve_mpmilp_bb(&p, bb_alg(), &opts).unwrap();
    eprintln!("{name}: bb {:.2}s nodes {} regions {} relaxed {} pruned {}", st.seconds, st.nodes, bb.len(), st.relaxed_regions, st.pruned_regions);
    // regression: same value as the enumeration everywhere
    let (lo, hi) = theta_box(&p);
    let mut rng = StdRng::seed_from_u64(77);
    for _ in 0..300 {
        let th: Vec<f64> = lo.iter().zip(&hi).map(|(l, h)| rng.gen_range(*l..*h)).collect();
        // enumeration reference: lowest objective among regions containing theta with margin 1e-7
        let e = (0..en.len())
            .filter(|&i| slack(&en, i, &th) <= -1e-7)
            .map(|i| p.evaluate_objective(&en.critical_regions[i].evaluate(&th), &th))
            .fold(f64::INFINITY, f64::min);
        if e.is_finite() {
            let strict = |s: &Solution| (0..s.critical_regions.len()).any(|i| slack(s, i, &th) <= -1e-7);
            for (s, tag) in [(&cmp, "compare"), (&bb, "bb")] {
                if strict(s) {
                    let o = s.evaluate_objective(&th).unwrap();
                    assert!((o - e).abs() <= 1e-7 * e.abs().max(1.0), "{name} {tag}: {o} vs enumeration {e}");
                }
            }
        }
    }
    let feas = check(&p, &cmp, n, 1, &format!("{name}/compare"));
    check(&p, &bb, n, 2, &format!("{name}/bb"));
    let (dc, dst) = solve_mpmilp_decomposition(&p, alg, &opts).unwrap();
    check(&p, &dc, n, 4, &format!("{name}/decomp"));
    eprintln!("{name}: decomposition mpLPs {} MILPs {} regions {} t {:.2}s", dst.mplps, dst.milps, dc.len(), dst.seconds);
    let (bb2, st2) = solve_mpmilp_bb_with(&p, bb_alg(), &opts, BbOptions { restrict_domain: false, reduce_final: true }).unwrap();
    check(&p, &bb2, n, 3, &format!("{name}/bb_full"));
    eprintln!(
        "{name}: bb_restricted mpLPs {} fixations {} | bb_fulldomain mpLPs {} fixations {} regions {} t {:.2}s | enumeration mpLPs {nodes}",
        st.nodes, st.distinct_fixations, st2.nodes, st2.distinct_fixations, bb2.len(), st2.seconds
    );
    Row {
        name: name.into(),
        nb: p.binary_indices.len(),
        nt: p.num_t(),
        en_t,
        en_regions: en.len(),
        en_nodes: nodes,
        cmp_regions: cmp.len(),
        cmp_t,
        bb_t: st.seconds,
        bb_nodes: st.nodes,
        bb_regions: bb.len(),
        feas,
    }
}

#[test]
fn mpmilp_suite() {
    let mut rows_out = vec![];
    rows_out.push(run("simple_mi", simple_mi(), 2000));
    rows_out.push(run("one_d", one_d(), 2000));
    rows_out.push(run("market", market(), 2000));
    for (i, (nb, nd, t)) in [(3, 2, 1), (3, 2, 2), (4, 2, 2), (5, 2, 2), (6, 3, 2), (8, 3, 2)].iter().enumerate() {
        let name = format!("fc{i}_{nb}b{nd}d{t}t");
        rows_out.push(run(&name, generate_mpmilp(*nb, *nd, *t, 100 + i as u64), 2000));
    }
    println!("busy machine timings (s)");
    println!("{:<18}{:>3}{:>3} | enum t/regions/combos | compare t/regions | bb t/nodes/regions | feasible/2000", "problem", "nb", "nt");
    for r in rows_out {
        println!(
            "{:<18}{:>3}{:>3} | {:>8.3} {:>6} {:>5} | {:>8.3} {:>6} | {:>8.3} {:>5} {:>6} | {}",
            r.name, r.nb, r.nt, r.en_t, r.en_regions, r.en_nodes, r.cmp_t, r.cmp_regions, r.bb_t, r.bb_nodes, r.bb_regions, r.feas
        );
    }
}

/// Root relaxation (all binaries in [0,1]) of a program as an mpLP.
fn root_relaxation(p: &MpmiProgram) -> rpopt::program::MpProgram {
    let b = &p.base;
    let nx = b.num_x();
    let mut a = b.a.clone();
    let mut rhs = b.b.clone();
    for &j in &p.binary_indices {
        let mut up = vec![0.0; nx];
        up[j] = 1.0;
        a.push_row(&up);
        rhs.push(1.0);
        let mut lo = vec![0.0; nx];
        lo[j] = -1.0;
        a.push_row(&lo);
        rhs.push(0.0);
    }
    let f = Mat::vstack(&[&b.f, &Mat::zeros(2 * p.binary_indices.len(), b.num_t())]);
    rpopt::program::MpProgram::new(a, rhs, b.c.clone(), b.h.clone(), None, b.a_t.clone(), b.b_t.clone(), f, b.equality_indices.clone(), true)
}

/// Probe: which algorithms leave gaps on the root relaxation of small fixed-charge programs.
#[test]
#[ignore]
fn probe_relaxation_gaps() {
    use rpopt::algorithms::solve_mpqp;
    for (nb, nd, t) in [(1, 1, 1), (2, 1, 1), (2, 2, 1), (3, 1, 1), (3, 2, 1), (3, 2, 2), (4, 2, 2)] {
        for seed in 100..110u64 {
            let p = generate_mpmilp(nb, nd, t, seed);
            let r = Arc::new(root_relaxation(&p));
            for (name, alg) in [("graph", MpqpAlgorithm::Graph), ("geometric", MpqpAlgorithm::Geometric), ("combinatorial", MpqpAlgorithm::Combinatorial)] {
                let sol = solve_mpqp(&r, alg);
                let mut rng = StdRng::seed_from_u64(5);
                let mut gaps = 0;
                for _ in 0..300 {
                    let th: Vec<f64> = (0..t).map(|_| rng.gen_range(0.0..1.0)).collect();
                    if r.solve_theta(&th).is_some() && !sol.critical_regions.iter().any(|c| (0..c.e.rows).all(|i| rpopt::linalg::dot(c.e.row(i), &th) - c.f[i] <= 1e-7)) {
                        gaps += 1;
                    }
                }
                if gaps > 0 {
                    println!("GAP {name} nb={nb} nd={nd} t={t} seed={seed}: {gaps}/300 gaps, {} regions", sol.len());
                }
            }
        }
    }
}

/// Does `process_region_constraints` change the polytope of a region?
#[test]
#[ignore]
fn probe_process_region_constraints() {
    let mut worst: Option<(usize, Mat, Vec<f64>, Mat, Vec<f64>)> = None;
    let mut bad = 0;
    let mut total = 0;
    for (nb, nd, t) in [(3, 2, 1), (3, 2, 2), (4, 2, 2), (5, 2, 2), (6, 3, 2)] {
        for seed in 100..104u64 {
            let p = Arc::new(generate_mpmilp(nb, nd, t, seed));
            let (sol, _) = solve_mpmilp_bb_with(&p, MpqpAlgorithm::MplpLex, &SolveOptions::default(), BbOptions { restrict_domain: true, reduce_final: false }).unwrap();
            let mut rng = StdRng::seed_from_u64(1);
            for r in &sol.critical_regions {
                total += 1;
                let (e2, f2) = rpopt::utils::process_region_constraints(&r.e, &r.f);
                let mut differs = false;
                for _ in 0..4000 {
                    let th: Vec<f64> = (0..t).map(|_| rng.gen_range(-0.5..1.5)).collect();
                    let a = (0..r.e.rows).all(|i| rpopt::linalg::dot(r.e.row(i), &th) <= r.f[i]);
                    let b = (0..e2.rows).all(|i| rpopt::linalg::dot(e2.row(i), &th) <= f2[i]);
                    if a != b {
                        differs = true;
                        break;
                    }
                }
                if differs {
                    bad += 1;
                    if worst.as_ref().map_or(true, |w| r.e.rows < w.0) {
                        worst = Some((r.e.rows, r.e.clone(), r.f.clone(), e2, f2));
                    }
                }
            }
        }
    }
    println!("process_region_constraints changed the polytope of {bad} of {total} regions");
    if let Some((_, e, f, e2, f2)) = worst {
        println!("smallest: E={:?}\nf={:?}\nreduced E={:?}\nf={:?}", e.to_rows(), f, e2.to_rows(), f2);
    }
}

/// Minimal reproducer: two rows equal up to 1 ulp are both dropped by `process_region_constraints`.
#[test]
#[ignore]
fn process_region_constraints_near_duplicates() {
    let e = rows(&[
        &[0.1414213562373095, 0.9899494936611665],
        &[0.8574929257125442, -0.5144957554275266],
        &[0.14142135623730953, 0.9899494936611666],
        &[-1.0, 0.0],
        &[0.0, -1.0],
    ]);
    let f = vec![0.7071067811865475, 7.187505722312114e-17, 0.7071067811865476, 0.0, 0.0];
    let (e2, f2) = rpopt::utils::process_region_constraints(&e, &f);
    println!("reduced rows {:?} rhs {:?}", e2.to_rows(), f2);
}

#[test]
#[ignore]
fn dump_repro() {
    use rpopt::algorithms::solve_mpqp;
    let p = generate_mpmilp(3, 1, 1, 101);
    let r = root_relaxation(&p);
    println!("A={:?}\nb={:?}\nc={:?}\nF={:?}\nA_t={:?}\nb_t={:?}\neq={:?}", r.a.to_rows(), r.b, r.c, r.f.to_rows(), r.a_t.to_rows(), r.b_t, r.equality_indices);
    let r = Arc::new(r);
    for th in [0.1, 0.5, 0.9] {
        println!("theta {th}: lp obj {:?}", r.solve_theta(&[th]).map(|s| s.obj));
    }
    for (n, alg) in [("graph", MpqpAlgorithm::Graph), ("graph_exp", MpqpAlgorithm::GraphExp), ("combinatorial", MpqpAlgorithm::Combinatorial)] {
        println!("{n}: {} regions", solve_mpqp(&r, alg).len());
    }
    // the same program without the duplicated bound rows
    let b = &p.base;
    let nodup = Arc::new(rpopt::program::MpProgram::new(b.a.clone(), b.b.clone(), b.c.clone(), b.h.clone(), None, b.a_t.clone(), b.b_t.clone(), b.f.clone(), b.equality_indices.clone(), true));
    println!("graph without duplicate bound rows: {} regions", solve_mpqp(&nodup, MpqpAlgorithm::Graph).len());
    let p6 = generate_mpmilp(6, 3, 2, 104);
    let r6 = Arc::new(root_relaxation(&p6));
    let t = Instant::now();
    println!("geometric nb=6 nd=3 t=2 seed=104: {} regions, {:.1}s", solve_mpqp(&r6, MpqpAlgorithm::Geometric).len(), t.elapsed().as_secs_f64());
    let b6 = &p6.base;
    let nodup6 = Arc::new(rpopt::program::MpProgram::new(b6.a.clone(), b6.b.clone(), b6.c.clone(), b6.h.clone(), None, b6.a_t.clone(), b6.b_t.clone(), b6.f.clone(), b6.equality_indices.clone(), true));
    let t = Instant::now();
    println!("geometric without duplicate rows: {} regions, {:.1}s", solve_mpqp(&nodup6, MpqpAlgorithm::Geometric).len(), t.elapsed().as_secs_f64());
}

/// 1-D case: the explicit solution agrees with a fine grid of MILP solves.
#[test]
fn single_parameter_grid() {
    for p in [one_d(), simple_mi()] {
        let p = Arc::new(p);
        let (bb, _) = solve_mpmilp_bb(&p, MpqpAlgorithm::Combinatorial, &SolveOptions::default()).unwrap();
        let cmp = solve_mpmilp_enumeration_compared(&p, MpqpAlgorithm::Combinatorial, &SolveOptions::default()).unwrap();
        let (lo, hi) = theta_box(&p);
        for s in [&bb, &cmp] {
            for i in 0..=2000 {
                let th = [lo[0] + (hi[0] - lo[0]) * (i as f64 + 0.5) / 2001.0];
                let d = p.solve_theta(&th);
                let hit = (0..s.len()).find(|&r| slack(s, r, &th) <= -1e-7);
                match (d, hit) {
                    (Some(d), Some(r)) => {
                        let o = p.evaluate_objective(&s.critical_regions[r].evaluate(&th), &th);
                        assert!((o - d.obj).abs() <= 1e-7 * d.obj.abs().max(1.0));
                    }
                    (Some(_), None) => assert!((0..s.len()).any(|r| slack(s, r, &th) <= 1e-7), "gap at {th:?}"),
                    (None, Some(_)) => panic!("region at infeasible theta"),
                    _ => {}
                }
            }
        }
    }
}

/// General integers: native branching, binary expansion and direct integer enumeration agree.
#[test]
fn general_integers() {
    // x continuous, y1 in {0..3}, y2 in {0..2}: x <= 2 y1 + 3 y2, x >= theta, y1 + y2 <= 4
    let a = rows(&[&[1., -2., -3.], &[-1., 0., 0.], &[1., 0., 0.], &[0., 1., 0.], &[0., 0., 1.], &[0., -1., 0.], &[0., 0., -1.], &[0., 1., 1.]]);
    let b = vec![0., 0., 12., 3., 2., 0., 0., 4.];
    let mut f = Mat::zeros(8, 1);
    f[(1, 0)] = -1.0;
    let c = vec![0.5, 2.0, 3.5];
    let a_t = rows(&[&[1.], &[-1.]]);
    let b_t = vec![10., 0.];
    let p = Arc::new(MpmiProgram::new_unprocessed(a.clone(), b.clone(), c.clone(), Mat::zeros(3, 1), None, a_t.clone(), b_t.clone(), f.clone(), vec![1, 2], vec![]));
    let up = [3u32, 2];
    let opts = SolveOptions::default();
    let (nat, st) = solve_mpmilp_bb_general(&p, &up, MpqpAlgorithm::Combinatorial, &opts).unwrap();
    let (ep, _groups) = binary_expansion(&a, &b, &c, &f, &a_t, &b_t, &[1, 2], &up, &[]);
    let ep = Arc::new(ep);
    let (exp, st2) = solve_mpmilp_bb(&ep, MpqpAlgorithm::Combinatorial, &opts).unwrap();
    println!("general ints: native nodes {} regions {} | binary expansion ({} bits) nodes {} regions {}", st.nodes, nat.len(), ep.binary_indices.len(), st2.nodes, exp.len());
    for i in 0..400 {
        let th = [10.0 * (i as f64 + 0.5) / 400.0];
        let mut best = f64::INFINITY;
        for y1 in 0..=3 {
            for y2 in 0..=2 {
                if y1 + y2 > 4 {
                    continue;
                }
                let x = th[0];
                if x <= 2.0 * y1 as f64 + 3.0 * y2 as f64 {
                    best = best.min(0.5 * x + 2.0 * y1 as f64 + 3.5 * y2 as f64);
                }
            }
        }
        for (s, q) in [(&nat, &p), (&exp, &ep)] {
            let hit = (0..s.len()).find(|&r| slack(s, r, &th) <= -1e-7);
            if let Some(r) = hit {
                let o = q.evaluate_objective(&s.critical_regions[r].evaluate(&th), &th);
                assert!((o - best).abs() <= 1e-7 * best.abs().max(1.0), "theta {th:?}: {o} vs {best}");
            } else {
                assert!((0..s.len()).any(|r| slack(s, r, &th) <= 1e-7) || best.is_infinite(), "gap at {th:?}");
            }
        }
    }
}
