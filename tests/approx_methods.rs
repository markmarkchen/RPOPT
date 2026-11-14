//! Tests for the approximate mpQP (Bemporad-Filippi), constraint trimming (Hou-You) and low-rank
//! storage (Nielsen-Axehill).

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::approx::{solve_mpqp_approx, solve_mpqp_approx_with, ApproxOptions};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::path::PathBuf;
use std::sync::Arc;

fn problems() -> Vec<(String, Arc<MpProgram>)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/problems");
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().map_or(false, |e| e == "json")).collect();
    v.sort();
    v.into_iter().map(|p| (p.file_stem().unwrap().to_string_lossy().to_string(), Arc::new(MpProgram::from_json_file(&p).unwrap()))).collect()
}

fn samples(prog: &MpProgram, n: usize, seed: u64) -> Vec<Vec<f64>> {
    let (lo, hi) = prog.theta_bbox().unwrap();
    let mut rng = StdRng::seed_from_u64(seed);
    (0..n).map(|_| (0..prog.num_t()).map(|k| rng.gen_range(lo[k]..hi[k])).collect()).collect()
}

#[test]
fn approx_bound_feasibility_and_cover() {
    for (name, prog) in problems() {
        let exact = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
        for &eps in &[1e-3, 1e-1, 10.0] {
            let t0 = std::time::Instant::now();
            let (sol, st) = solve_mpqp_approx_with(&prog, &ApproxOptions::abs(eps));
            let el = t0.elapsed().as_secs_f64();
            let (mut gaps, mut infeas, mut over, mut under, mut maxerr) = (0, 0, 0, 0, 0.0f64);
            let mut feasible = 0;
            for th in samples(&prog, 2000, 5) {
                let Some(d) = prog.solve_theta(&th) else { continue };
                feasible += 1;
                let Some(x) = sol.evaluate(&th) else {
                    gaps += 1;
                    continue;
                };
                let ax = prog.a.matvec(&x);
                let ft = prog.f.matvec(&th);
                for i in 0..prog.num_constraints() {
                    let rhs = prog.b[i] + ft[i];
                    let viol = if prog.equality_indices.contains(&i) { (ax[i] - rhs).abs() } else { ax[i] - rhs };
                    if viol > 1e-7 * (1.0 + rhs.abs()) {
                        infeas += 1;
                        break;
                    }
                }
                let err = prog.evaluate_objective(&x, &th) - d.obj;
                maxerr = maxerr.max(err);
                if err > eps * (1.0 + 1e-6) + 1e-8 {
                    over += 1;
                }
                if err < -1e-7 * (1.0 + d.obj.abs()) {
                    under += 1;
                }
            }
            println!("{name} eps {eps:e}: exact {} approx {} regions, {:.3}s, qps {}, dropped {}, feasible samples {feasible}, gaps {gaps}, infeasible {infeas}, over-eps {over}, below-opt {under}, max err {maxerr:.3e}", exact.len(), sol.len(), el, st.qp_solves, st.dropped_pieces);
            assert_eq!(gaps, 0, "{name} eps {eps}: sampled gaps");
            assert_eq!(infeas, 0, "{name} eps {eps}: infeasible optimizer");
            assert_eq!(over, 0, "{name} eps {eps}: error above epsilon");
            assert_eq!(under, 0, "{name} eps {eps}: below the optimum");
        }
    }
}

#[test]
fn approx_subtract_mode_small_problems() {
    use rpopt::algorithms::approx::Exploration;
    for (name, prog) in problems() {
        let eps = if name.starts_with("di_") {
            0.1
        } else if name == "sys3x2_N3" {
            31.86
        } else {
            continue;
        };
        let mut o = ApproxOptions::abs(eps);
        o.exploration = Exploration::Subtract;
        let (sol, st) = solve_mpqp_approx_with(&prog, &o);
        let exact = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
        let (mut bad, mut feasible) = (0, 0);
        for th in samples(&prog, 2000, 15) {
            let Some(d) = prog.solve_theta(&th) else { continue };
            feasible += 1;
            match sol.evaluate(&th) {
                None => bad += 1,
                Some(x) => {
                    let err = prog.evaluate_objective(&x, &th) - d.obj;
                    let ax = prog.a.matvec(&x);
                    let ft = prog.f.matvec(&th);
                    let infeas = (0..prog.num_constraints()).any(|i| {
                        let rhs = prog.b[i] + ft[i];
                        let v = if prog.equality_indices.contains(&i) { (ax[i] - rhs).abs() } else { ax[i] - rhs };
                        v > 1e-7 * (1.0 + rhs.abs())
                    });
                    if err > eps * (1.0 + 1e-6) + 1e-8 || err < -1e-7 * (1.0 + d.obj.abs()) || infeas {
                        bad += 1;
                    }
                }
            }
        }
        println!("{name} subtract eps {eps}: exact {} approx {} regions ({} qps), {feasible} feasible samples, bad {bad}", exact.len(), sol.len(), st.qp_solves);
        assert_eq!(bad, 0, "{name}");
    }
}

#[test]
fn approx_wrapper() {
    let (_, prog) = problems().remove(1);
    let s = solve_mpqp_approx(&prog, 1.0);
    assert!(s.len() > 0);
}

fn same_x(a: &[f64], b: &[f64]) -> bool {
    a.iter().zip(b).all(|(u, v)| (u - v).abs() <= 1e-7 * (1.0 + u.abs()))
}

#[test]
fn trimming_pointwise_and_region() {
    use rpopt::algorithms::region_signature;
    use rpopt::trimming::*;
    for (name, prog) in problems() {
        let Some(kappa) = lipschitz_by_enumeration(&prog, 3_000_000) else {
            println!("{name}: too many subsets for the exact Lipschitz constant, skipped");
            continue;
        };
        let m = prog.num_constraints();
        let mut tr = Trimmer::new(kappa);
        let all = samples(&prog, 400, 21);
        let mut learned = 0;
        let mut line = format!("{name}: kappa {kappa:.4}, m {m}; kept fraction after n learned instances:");
        let test_pts: Vec<Vec<f64>> = samples(&prog, 150, 77);
        for target in [1usize, 10, 100, 400] {
            for th in all.iter().take(target).skip(learned) {
                tr.learn(&prog, th);
            }
            learned = target;
            let (mut kept_sum, mut cnt, mut bad) = (0usize, 0usize, 0);
            for th in &test_pts {
                let Some(full) = prog.solve_theta(th) else { continue };
                let kept = tr.kept_at(&prog, th);
                let (tp, _) = trimmed_program(&prog, &kept);
                let t = tp.solve_theta(th);
                match t {
                    Some(t) if same_x(&full.sol, &t.sol) => {}
                    _ => bad += 1,
                }
                kept_sum += kept.len();
                cnt += 1;
            }
            line += &format!(" {target}: {:.3}", kept_sum as f64 / (cnt * m).max(1) as f64);
            assert_eq!(bad, 0, "{name}: pointwise trimmed QP differs from the full QP");
        }
        println!("{line}");

        // one learned instance, query at growing distance (average kept fraction over 40 centers)
        let mut dl = String::new();
        for dist in [1e-3, 1e-2, 1e-1, 1.0] {
            let (mut ks, mut cnt) = (0usize, 0usize);
            for c in samples(&prog, 40, 3) {
                let Some(inst) = solve_instance(&prog, &c) else { continue };
                let mut q = c.clone();
                q[0] += dist;
                if prog.solve_theta(&q).is_none() {
                    continue;
                }
                let kept = trim_point(&prog, &q, &[inst], kappa);
                let (tp, _) = trimmed_program(&prog, &kept);
                assert!(same_x(&prog.solve_theta(&q).unwrap().sol, &tp.solve_theta(&q).expect("trimmed feasible").sol), "{name}: single instance trim differs");
                ks += kept.len();
                cnt += 1;
            }
            dl += &format!(" d={dist}: {:.3}", ks as f64 / (cnt * m).max(1) as f64);
        }
        println!("  {name}: one learned instance, kept fraction vs distance:{dl}");

        // region version: solve the mpQP on a small box with the constraints kept for the ball
        let (lo, hi) = prog.theta_bbox().unwrap();
        let mut rng = StdRng::seed_from_u64(5);
        let mut done = 0;
        for _ in 0..40 {
            if done == 3 {
                break;
            }
            let c: Vec<f64> = (0..prog.num_t()).map(|k| rng.gen_range(lo[k]..hi[k])).collect();
            if prog.solve_theta(&c).is_none() {
                continue;
            }
            let h: Vec<f64> = (0..prog.num_t()).map(|k| 0.02 * (hi[k] - lo[k])).collect();
            let blo: Vec<f64> = c.iter().zip(&h).map(|(a, b)| a - b).collect();
            let bhi: Vec<f64> = c.iter().zip(&h).map(|(a, b)| a + b).collect();
            let radius = h.iter().map(|v| v * v).sum::<f64>().sqrt();
            let kept = tr.kept_in_ball(&prog, &c, radius);
            let (tp, map) = trimmed_program(&prog, &kept);
            let full_box = restrict_theta_box(&prog, &blo, &bhi);
            let trim_box = restrict_theta_box(&tp, &blo, &bhi);
            let s_full = solve_mpqp(&Arc::new(full_box), MpqpAlgorithm::Geometric);
            let s_trim = solve_mpqp(&Arc::new(trim_box), MpqpAlgorithm::Geometric);
            let mapped: Vec<Vec<usize>> = {
                let mut v: Vec<Vec<usize>> = s_trim.critical_regions.iter().map(|r| r.active_set.iter().map(|&i| map[i]).collect()).collect();
                v.sort();
                v
            };
            assert_eq!(region_signature(&s_full.critical_regions), mapped, "{name}: region signature differs after ball trimming");
            println!("  {name}: box half-width 2% of the range, kept {}/{m} constraints, {} regions both", kept.len(), s_full.len());
            done += 1;
        }
    }
}

#[test]
fn lowrank_matches_uncompressed() {
    use rpopt::lowrank::compress;
    for (name, prog) in problems() {
        let exact = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
        // the approximate solution (relaxed multiplier facets, partition cuts) must compress as well
        if name != "masses2_N6" {
            let (ap, _) = solve_mpqp_approx_with(&prog, &ApproxOptions::abs(1.0));
            let ac = compress(&ap).unwrap();
            let s = ac.storage();
            let mut worst = 0.0f64;
            let mut miss = 0;
            for th in &samples(&prog, 2000, 4) {
                match (ap.evaluate(th), ac.evaluate(th)) {
                    (None, None) => {}
                    (Some(x), Some(y)) => {
                        for (u, v) in x.iter().zip(&y) {
                            worst = worst.max((u - v).abs() / (1.0 + u.abs()));
                        }
                    }
                    _ => miss += 1,
                }
            }
            println!("{name} approx(eps=1): R {} baseline {} -> compressed {} reals (+{} ints), dense nodes {}, worst rel err {worst:.2e}, miss {miss}", s.regions, s.baseline_reals, s.compressed_reals, s.compressed_ints, s.dense_nodes);
            assert!(worst <= 1e-9 && miss == 0, "{name}: approximate solution compression mismatch");
        }
        let t0 = std::time::Instant::now();
        let comp = compress(&exact).unwrap();
        let tc = t0.elapsed().as_secs_f64();
        let st = comp.storage().clone();
        let (mut bad, mut worst, mut loc_bad) = (0, 0.0f64, 0);
        for th in &samples(&prog, 2000, 9) {
            match (exact.evaluate(th), comp.evaluate(th)) {
                (None, None) => {}
                (Some(x), Some(y)) => {
                    let mut e = 0.0f64;
                    for (u, v) in x.iter().zip(&y) {
                        e = e.max((u - v).abs() / (1.0 + u.abs()));
                    }
                    worst = worst.max(e);
                    if e > 1e-9 {
                        bad += 1;
                    }
                }
                _ => bad += 1,
            }
            if exact.get_region_index(th) != comp.locate(th) {
                loc_bad += 1;
            }
        }
        println!(
            "{name}: R {} baseline {} (with duals {}) -> compressed {} reals + {} ints ({:.2}x of baseline), dense nodes {}, depth {}, ranks {:?}, compress {:.3}s, worst rel err {worst:.2e}, region mismatches {loc_bad}",
            st.regions, st.baseline_reals, st.baseline_with_duals, st.compressed_reals, st.compressed_ints,
            st.compressed_reals as f64 / st.baseline_reals as f64, st.dense_nodes, st.max_depth, st.rank_histogram, tc
        );
        assert_eq!(bad, 0, "{name}: evaluate mismatch");
        assert_eq!(loc_bad, 0, "{name}: region index mismatch");
    }
}
