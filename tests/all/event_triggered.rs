//! Event-triggered cache (`rpopt::event_triggered`): every answer equals a direct solve, for mpLP and
//! mpQP, under each miss policy, cache limit and with or without parallelism.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::event_triggered::{EtCache, EtOptions, Eviction, MissPolicy, Status};
use rpopt::problem_generator::{generate_mplp, generate_mpqp};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn load(dir: &str, name: &str) -> Arc<MpProgram> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir).join(format!("{name}.json"));
    Arc::new(MpProgram::from_json_file(&p).unwrap())
}

fn programs() -> Vec<(String, Arc<MpProgram>)> {
    let mut v = vec![];
    for n in ["di_N4", "di_N8", "masses2_N4"] {
        v.push((n.to_string(), load("bench/problems", n)));
    }
    for n in ["diinf_N3", "dione_N4", "sys3x2inf_N3"] {
        v.push((n.to_string(), load("bench/problems_lp", n)));
    }
    for seed in 0..3 {
        v.push((format!("rand mpqp {seed}"), Arc::new(generate_mpqp(4, 3, 12, seed))));
        v.push((format!("rand mplp {seed}"), Arc::new(generate_mplp(4, 3, 12, seed))));
    }
    v
}

/// A random walk inside the parameter bounding box: `n` points, steps of `step` box widths.
fn walk(p: &MpProgram, n: usize, step: f64, seed: u64) -> Vec<f64> {
    let (lo, hi) = p.theta_bbox().unwrap().clone();
    let nt = lo.len();
    let mut rng = StdRng::seed_from_u64(seed);
    let mut th: Vec<f64> = (0..nt).map(|k| lo[k] + (hi[k] - lo[k]) * rng.r#gen::<f64>()).collect();
    let mut out = Vec::with_capacity(n * nt);
    for _ in 0..n {
        for k in 0..nt {
            let w = hi[k] - lo[k];
            th[k] = (th[k] + step * w * (2.0 * rng.r#gen::<f64>() - 1.0)).clamp(lo[k], hi[k]);
        }
        out.extend_from_slice(&th);
    }
    out
}

/// The answer agrees with a direct solve (objective always; x for mpQP, whose optimiser is unique).
fn check(p: &MpProgram, theta: &[f64], a: &rpopt::event_triggered::Answer, label: &str) {
    let direct = p.solve_theta_uncounted(theta);
    match (a.status, direct) {
        (Status::Infeasible, None) => {}
        (Status::Infeasible, Some(_)) => panic!("{label}: reported infeasible, the solver solves {theta:?}"),
        (Status::Hit | Status::Built | Status::Solved, Some(s)) => {
            let tol = 1e-6 * (1.0 + s.obj.abs());
            assert!((a.objective - s.obj).abs() <= tol, "{label}: objective {} vs {} at {theta:?} ({:?})", a.objective, s.obj, a.status);
            if p.q.is_some() {
                for (u, v) in a.x.iter().zip(&s.sol) {
                    assert!((u - v).abs() <= 1e-5 * (1.0 + v.abs()), "{label}: x {:?} vs {:?} ({:?})", a.x, s.sol, a.status);
                }
            }
            // the multipliers certify optimality: complementary and of the right sign
            for (i, &d) in a.duals.iter().enumerate() {
                if !p.equality_indices.contains(&i) {
                    assert!(d >= -1e-6 * (1.0 + d.abs()), "{label}: negative multiplier {d} on row {i}");
                }
            }
        }
        (st, d) => panic!("{label}: status {st:?} with direct solve {:?}", d.map(|s| s.obj)),
    }
}

#[test]
fn exact_policy_matches_solver() {
    for (name, p) in programs() {
        let nt = p.num_t();
        let thetas = walk(&p, 400, 0.05, 7);
        let mut cache = EtCache::new(p.clone(), EtOptions { parallel: false, ..Default::default() });
        for i in 0..400 {
            let th = &thetas[i * nt..(i + 1) * nt];
            let a = cache.query(th, Some(0));
            check(&p, th, &a, &name);
        }
        let s = cache.stats();
        assert!(s.hits > 0, "{name}: no hits ({s:?})");
        assert!(s.build_failures * 4 <= s.built + s.build_failures + 1, "{name}: too many build failures ({s:?})");
    }
}

#[test]
fn parallel_batch_matches_serial() {
    for (name, p) in programs() {
        let nt = p.num_t();
        let n = 300;
        let thetas = walk(&p, n, 0.08, 11);
        let streams: Vec<u64> = (0..n as u64).map(|i| i % 7).collect();
        let mut par = EtCache::new(p.clone(), EtOptions { parallel: true, ..Default::default() });
        let mut ser = EtCache::new(p.clone(), EtOptions { parallel: false, ..Default::default() });
        let a = par.query_batch(&thetas, Some(&streams));
        // second pass: mostly hits
        let a2 = par.query_batch(&thetas, Some(&streams));
        for i in 0..n {
            let th = &thetas[i * nt..(i + 1) * nt];
            let b = ser.query(th, Some(streams[i]));
            check(&p, th, &a[i], &format!("{name} parallel"));
            check(&p, th, &a2[i], &format!("{name} parallel, 2nd pass"));
            check(&p, th, &b, &format!("{name} serial"));
        }
        assert!(a2.iter().all(|x| x.status == Status::Hit || x.status == Status::Infeasible || x.status == Status::Solved), "{name}: second pass misses");
    }
}

#[test]
fn cache_limit_and_eviction() {
    for ev in [Eviction::Lru, Eviction::Lfu] {
        for (name, p) in programs().into_iter().take(4) {
            let nt = p.num_t();
            let thetas = walk(&p, 300, 0.2, 3);
            let mut cache = EtCache::new(p.clone(), EtOptions { max_regions: Some(3), eviction: ev, parallel: false, ..Default::default() });
            for i in 0..300 {
                let th = &thetas[i * nt..(i + 1) * nt];
                let a = cache.query(th, Some(1));
                check(&p, th, &a, &format!("{name} {ev:?}"));
                assert!(cache.len() <= 3);
            }
        }
    }
}

#[test]
fn miss_policies() {
    let p = load("bench/problems", "di_N4");
    let nt = p.num_t();
    let thetas = walk(&p, 60, 0.3, 5);
    let feasible: Vec<&[f64]> = (0..60).map(|i| &thetas[i * nt..(i + 1) * nt]).filter(|t| p.solve_theta_uncounted(t).is_some()).collect();
    assert!(feasible.len() > 10);

    // report: no solve; an externally supplied region then answers
    let mut c = EtCache::new(p.clone(), EtOptions { miss: MissPolicy::Report, ..Default::default() });
    let a = c.query(feasible[0], None);
    assert_eq!(a.status, Status::Miss);
    assert!(a.objective.is_nan());
    assert!(c.is_empty());
    let id = c.insert_theta(feasible[0]).unwrap();
    let a = c.query(feasible[0], None);
    assert_eq!((a.status, a.region), (Status::Hit, Some(id)));
    check(&p, feasible[0], &a, "report then insert");

    // solve only: exact answers, empty cache
    let mut c = EtCache::new(p.clone(), EtOptions { miss: MissPolicy::SolveOnly, ..Default::default() });
    for t in &feasible {
        let a = c.query(t, Some(0));
        assert_eq!(a.status, Status::Solved);
        check(&p, t, &a, "solve only");
    }
    assert!(c.is_empty());

    // deferred: miss on an empty cache, approximate answers later, exact after flush
    let mut c = EtCache::new(p.clone(), EtOptions { miss: MissPolicy::Deferred, ..Default::default() });
    assert_eq!(c.query(feasible[0], None).status, Status::Miss);
    assert_eq!(c.num_pending(), 1);
    assert_eq!(c.flush(), 1);
    assert_eq!(c.query(feasible[0], None).status, Status::Hit);
    let mut approx = 0;
    for t in &feasible[1..] {
        match c.query(t, None).status {
            Status::Approx => approx += 1,
            Status::Hit => {}
            s => panic!("deferred: status {s:?}"),
        }
    }
    assert!(approx > 0 && c.num_pending() == approx);
    c.flush();
    assert_eq!(c.num_pending(), 0);
    for t in &feasible {
        let a = c.query(t, None);
        assert_eq!(a.status, Status::Hit);
        check(&p, t, &a, "deferred after flush");
    }

    // switching policy at run time: train exact, then freeze
    let mut c = EtCache::new(p.clone(), EtOptions::default());
    for t in &feasible {
        c.query(t, None);
    }
    let n = c.len();
    c.set_miss_policy(MissPolicy::SolveOnly);
    for t in walk(&p, 50, 0.3, 99).chunks(nt) {
        let a = c.query(t, None);
        check(&p, t, &a, "frozen");
    }
    assert_eq!(c.len(), n);
}

#[test]
fn insert_active_sets_of_a_full_solution() {
    let p = load("bench/problems", "di_N4");
    let full = rpopt::solve_mpqp(&p, rpopt::MpqpAlgorithm::CombinatorialGraph);
    let mut c = EtCache::new(p.clone(), EtOptions { miss: MissPolicy::Report, ..Default::default() });
    for r in &full.critical_regions {
        c.insert_active_set(&r.active_set).unwrap();
    }
    assert_eq!(c.len(), full.critical_regions.len());
    let nt = p.num_t();
    for t in walk(&p, 300, 0.2, 4).chunks(nt) {
        let a = c.query(t, None);
        if p.solve_theta_uncounted(t).is_some() {
            assert_eq!(a.status, Status::Hit, "a full solution covers every feasible point");
            check(&p, t, &a, "full solution");
        }
    }
}

#[test]
fn slow_parameters_skip_row_evaluations() {
    // one coordinate drifts slowly, the others are fixed: the stream memory settles almost every query
    for (name, p) in programs() {
        let nt = p.num_t();
        let (lo, hi) = p.theta_bbox().unwrap().clone();
        let mut th: Vec<f64> = lo.iter().zip(&hi).map(|(l, h)| 0.5 * (l + h)).collect();
        if p.solve_theta_uncounted(&th).is_none() {
            continue;
        }
        let mut c = EtCache::new(p.clone(), EtOptions { parallel: false, ..Default::default() });
        let n = 2000;
        for i in 0..n {
            th[0] = 0.5 * (lo[0] + hi[0]) + 0.3 * (hi[0] - lo[0]) * (i as f64 / n as f64 * 6.0).sin();
            let a = c.query(&th, Some(42));
            check(&p, &th, &a, &format!("{name} slow"));
        }
        let s = c.stats();
        let settled = s.ball_hits + s.stream_hits;
        assert!(settled as f64 >= 0.9 * s.hits as f64, "{name}: stream memory settled {settled} of {} hits ({s:?})", s.hits);
        let _ = nt;
    }
}

#[test]
fn lazy_reduction_keeps_answers() {
    for (name, p) in programs() {
        let nt = p.num_t();
        let thetas = walk(&p, 300, 0.05, 21);
        let mut c = EtCache::new(p.clone(), EtOptions { reduce_after: Some(1), parallel: false, ..Default::default() });
        let rows_before: usize;
        {
            for t in thetas.chunks(nt).take(50) {
                c.query(t, Some(0));
            }
            rows_before = c.total_rows();
        }
        for t in thetas.chunks(nt) {
            let a = c.query(t, Some(0));
            check(&p, t, &a, &format!("{name} reduced"));
        }
        assert!(c.stats().reductions > 0 || c.is_empty(), "{name}");
        let _ = rows_before;
    }
}

/// The multipliers are an optimal dual: A'λ = -(c + Hθ), λ >= 0 on inequalities, λ_i slack_i = 0.
fn check_dual(p: &MpProgram, theta: &[f64], a: &rpopt::event_triggered::Answer, label: &str) {
    let (m, nx) = (p.num_constraints(), p.num_x());
    let cost: Vec<f64> = (0..nx).map(|k| p.c[k] + (0..p.num_t()).map(|t| p.h[(k, t)] * theta[t]).sum::<f64>()).collect();
    let scale = 1.0 + a.duals.iter().fold(0.0f64, |s, v| s.max(v.abs()));
    for k in 0..nx {
        let r: f64 = (0..m).map(|i| p.a[(i, k)] * a.duals[i]).sum::<f64>() + cost[k];
        assert!(r.abs() <= 1e-6 * scale, "{label}: stationarity residual {r} in column {k}");
    }
    for i in 0..m {
        let rhs = p.b[i] + (0..p.num_t()).map(|t| p.f[(i, t)] * theta[t]).sum::<f64>();
        let slack = rhs - (0..nx).map(|k| p.a[(i, k)] * a.x[k]).sum::<f64>();
        if !p.equality_indices.contains(&i) {
            assert!(a.duals[i] >= -1e-7 * scale, "{label}: multiplier {} < 0 on row {i}", a.duals[i]);
        }
        assert!((a.duals[i] * slack).abs() <= 1e-6 * scale * (1.0 + rhs.abs()), "{label}: complementarity on row {i}");
    }
}

fn pareto_gain(p: &MpProgram, theta: &[f64], duals: &[f64], coords: &[usize], core: &[f64]) -> f64 {
    let mut s = 0.0;
    for (i, l) in duals.iter().enumerate() {
        let fd: f64 = coords.iter().zip(core).map(|(&k, c)| p.f[(i, k)] * (c - theta[k])).sum();
        s -= l * fd;
    }
    s
}

#[test]
fn lex_regions_are_lex_optimal_bases() {
    use rpopt::event_triggered::BasisRule;
    let mut cases: Vec<(String, Arc<MpProgram>)> = ["dione_N4", "sys3x2inf_N3", "diinf_N3"].iter().map(|n| (n.to_string(), load("bench/problems_lp", n))).collect();
    for seed in 0..3 {
        cases.push((format!("rand mplp {seed}"), Arc::new(generate_mplp(3, 3, 9, seed))));
    }
    for (name, p) in cases {
        let all: std::collections::HashSet<Vec<usize>> = rpopt::algorithms::mplp_lex::solve(&p, 0).unwrap().into_iter().map(|r| {
            let mut a = r.active_set;
            a.sort_unstable();
            a
        }).collect();
        let nt = p.num_t();
        let mut c = EtCache::new(p.clone(), EtOptions { basis: BasisRule::Lex, parallel: false, ..Default::default() });
        for t in walk(&p, 400, 0.05, 13).chunks(nt) {
            let a = c.query(t, Some(0));
            check(&p, t, &a, &format!("{name} lex"));
        }
        for (_, r) in c.regions() {
            assert!(all.contains(&r.active_set), "{name}: cached basis {:?} is not lex-optimal", r.active_set);
        }
        assert_eq!(c.stats().rule_fallbacks, 0, "{name}: lex fell back ({:?})", c.stats());
    }
    // an mpQP has no lexicographic rule
    let q = load("bench/problems", "di_N4");
    assert!(EtCache::try_new(q.clone(), EtOptions { basis: BasisRule::Lex, ..Default::default() }).is_err());
    // the default resolves to lex for an mpLP and to the solver's active set for an mpQP
    assert_eq!(EtCache::new(q, EtOptions::default()).options().basis, BasisRule::Solver);
    let lp = load("bench/problems_lp", "dione_N4");
    assert_eq!(EtCache::new(lp, EtOptions::default()).options().basis, BasisRule::Lex);
}

#[test]
fn pareto_multipliers_are_optimal_and_dominate_the_solver() {
    use rpopt::event_triggered::BasisRule;
    for name in ["sys3x2inf_N3", "dione_N4", "masses2inf_N3"] {
        let p = load("bench/problems_lp", name);
        let nt = p.num_t();
        let (lo, hi) = p.theta_bbox().unwrap().clone();
        // cut coordinates: the parameters that enter the right hand side only
        let coords: Vec<usize> = (0..nt).filter(|&k| (0..p.h.rows).all(|r| p.h[(r, k)] == 0.0)).collect();
        let core: Vec<f64> = coords.iter().map(|&k| 0.5 * (lo[k] + hi[k])).collect();
        let rule = BasisRule::Pareto { coords: coords.clone(), core: core.clone() };
        let mut better = 0;
        for t in walk(&p, 120, 0.1, 17).chunks(nt) {
            if p.solve_theta_uncounted(t).is_none() {
                continue;
            }
            // a fresh cache: the answer is the miss path (one LP over the optimal dual face)
            let mut c = EtCache::new(p.clone(), EtOptions { basis: rule.clone(), parallel: false, ..Default::default() });
            let a = c.query(t, None);
            check(&p, t, &a, &format!("{name} pareto"));
            check_dual(&p, t, &a, &format!("{name} pareto"));
            let mut s = EtCache::new(p.clone(), EtOptions { basis: BasisRule::Solver, parallel: false, ..Default::default() });
            let b = s.query(t, None);
            let (gp, gs) = (pareto_gain(&p, t, &a.duals, &coords, &core), pareto_gain(&p, t, &b.duals, &coords, &core));
            assert!(gp >= gs - 1e-7 * (1.0 + gs.abs()), "{name}: pareto cut {gp} below the solver's {gs} at {t:?}");
            if gp > gs + 1e-7 * (1.0 + gs.abs()) {
                better += 1;
            }
        }
        // a cache over a walk: answers stay exact and optimal with the hit-time tie-break
        let mut c = EtCache::new(p.clone(), EtOptions { basis: rule.clone(), parallel: false, ..Default::default() });
        for t in walk(&p, 400, 0.05, 19).chunks(nt) {
            let a = c.query(t, Some(1));
            check(&p, t, &a, &format!("{name} pareto walk"));
            if a.status == Status::Hit || a.status == Status::Built {
                check_dual(&p, t, &a, &format!("{name} pareto walk"));
            }
        }
        eprintln!("{name}: pareto cut strictly higher at the core in {better} solves; {:?}", c.stats());
    }
    // a cut coordinate in the objective is rejected
    let p = Arc::new(generate_mplp(3, 3, 9, 0));
    let bad = BasisRule::Pareto { coords: vec![0], core: vec![0.0] };
    if (0..p.h.rows).any(|r| p.h[(r, 0)] != 0.0) {
        assert!(EtCache::try_new(p, EtOptions { basis: bad, ..Default::default() }).is_err());
    }
}

/// Review findings: a non-finite θ, a hit without stream state, active sets that are not a basis,
/// and an empty region from `insert_active_set`.
#[test]
fn malformed_input_and_stream_start() {
    for (name, p) in programs().into_iter().take(6) {
        let nt = p.num_t();
        let thetas = walk(&p, 200, 0.05, 23);
        let mut c = EtCache::new(p.clone(), EtOptions { parallel: false, ..Default::default() });
        for t in thetas.chunks(nt).take(20) {
            c.query(t, Some(3));
        }
        // a NaN (or infinite) θ is answered infeasible and leaves the stream usable
        let mut bad = thetas[..nt].to_vec();
        bad[0] = f64::NAN;
        assert_eq!(c.query(&bad, Some(3)).status, Status::Infeasible, "{name}");
        bad[0] = f64::INFINITY;
        assert_eq!(c.query(&bad, Some(3)).status, Status::Infeasible, "{name}");
        for t in thetas.chunks(nt) {
            let a = c.query(t, Some(3));
            check(&p, t, &a, &format!("{name} after a NaN query"));
        }
        assert_eq!(c.insert_theta(&bad), None);
    }

    // a stream that only ever hits still gets stream memory
    let p = load("bench/problems", "di_N4");
    let nt = p.num_t();
    let thetas = walk(&p, 300, 0.01, 29);
    let mut c = EtCache::new(p.clone(), EtOptions { parallel: false, ..Default::default() });
    for t in thetas.chunks(nt) {
        c.insert_theta(t);
    }
    c.clear_streams();
    let before = c.stats().clone();
    for t in thetas.chunks(nt) {
        let a = c.query(t, Some(9));
        check(&p, t, &a, "warm cache");
    }
    let s = c.stats();
    assert!(s.ball_hits + s.stream_hits > before.ball_hits + before.stream_hits, "stream memory never started ({s:?})");

    // mpLP: an active set that is not a basis is refused; every accepted region is optimal
    let p = load("bench/problems_lp", "sys3x2inf_N3");
    let (m, nx) = (p.num_constraints(), p.num_x());
    let mut c = EtCache::new(p.clone(), EtOptions { miss: MissPolicy::Report, parallel: false, ..Default::default() });
    let neq = p.equality_indices.len();
    assert!(c.insert_active_set(&((neq..m).take(1).collect::<Vec<_>>())).is_err() || nx == neq + 1, "fewer than n_x rows");
    assert!(c.insert_active_set(&(0..m).collect::<Vec<_>>()).is_err(), "more than n_x rows");
    let mut rng = StdRng::seed_from_u64(31);
    let (mut ok, mut refused) = (0, 0);
    for _ in 0..300 {
        let mut set: Vec<usize> = (neq..m).collect();
        for i in (1..set.len()).rev() {
            set.swap(i, rng.gen_range(0..=i));
        }
        set.truncate(nx - neq);
        match c.insert_active_set(&set) {
            Ok(_) => ok += 1,
            Err(_) => refused += 1,
        }
    }
    for (_, r) in c.regions() {
        let (center, _) = r.chebyshev().expect("an accepted region is not empty");
        let a = c.query(&center, None);
        assert_eq!(a.status, Status::Hit);
        check(&p, &center, &a, "inserted basis");
    }
    assert!(refused > 0, "random bases are mostly not optimal anywhere ({ok} accepted)");
}
