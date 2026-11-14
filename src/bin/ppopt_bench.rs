//! Benchmark runner: solves every JSON problem in a directory with every mpQP algorithm, checks
//! the solutions, then times all point location methods.
//!
//! Usage:
//!   ppopt_bench <problems_dir> <out.json> [--only a,b] [--algos x,y] [--queries N] [--no-pl] [--threads T]
//!               [--max-seconds S] [--resume] [--traj N,L] [--no-traj] [--geom-max-regions R] [--bst-max-regions R]
//!
//! `--geom-max-regions R` skips the partition geometry (and every locator built on it) above R regions (default 15000):
//! building it took 120 s at 10k regions. `--bst-max-regions R` (default 3000) skips the binary search tree and the multiway
//! trees (1.7M nodes for 955 regions).
//! `--max-seconds S` caps every first solve at S seconds (the run is recorded as timed out and the process
//! re-executes itself with `--resume` so that the runaway solver thread dies). `--resume` skips (problem, algorithm)
//! pairs already in the output file. Files named `*.model.json` are closed-loop model sidecars (A, B, u_index) of
//! `<name>.json`, not problems. Point location is timed on uniform queries and on closed-loop trajectories
//! (`--traj N,L`: N trajectories of L steps; x+ = A x + B u with the explicit law for MPC problems, a random walk
//! in theta otherwise).

use rpopt::algorithms::{region_signature, solve_mpqp_with, MpqpAlgorithm, SolveOptions};
use rpopt::point_location::*;
use rpopt::program::MpProgram;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde_json::{json, Value};
use rayon::prelude::*;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: ppopt_bench <problems_dir> <out.json> [--only a,b] [--algos x,y] [--queries N] [--no-pl]");
        std::process::exit(2);
    }
    let dir = std::path::PathBuf::from(&args[1]);
    let out = std::path::PathBuf::from(&args[2]);
    let mut only: Option<Vec<String>> = None;
    let mut algos: Vec<MpqpAlgorithm> = MpqpAlgorithm::all().to_vec();
    let mut queries = 100_000usize;
    let mut do_pl = true;
    let mut max_seconds = 0.0f64;
    let mut resume = false;
    let mut do_traj = true;
    let mut geom_max_regions = 15_000usize;
    let mut bst_max_regions = 3_000usize;
    let (mut traj_n, mut traj_len) = (1000usize, 100usize);
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--only" => {
                only = Some(args[i + 1].split(',').map(String::from).collect());
                i += 1;
            }
            "--algos" => {
                algos = args[i + 1].split(',').map(|s| MpqpAlgorithm::from_name(s).unwrap_or_else(|| panic!("unknown algorithm {s}"))).collect();
                i += 1;
            }
            "--queries" => {
                queries = args[i + 1].parse().unwrap();
                i += 1;
            }
            "--no-pl" => do_pl = false,
            "--no-traj" => do_traj = false,
            "--resume" => resume = true,
            "--bst-max-regions" => {
                bst_max_regions = args[i + 1].parse().unwrap();
                i += 1;
            }
            "--geom-max-regions" => {
                geom_max_regions = args[i + 1].parse().unwrap();
                i += 1;
            }
            "--max-seconds" => {
                max_seconds = args[i + 1].parse().unwrap();
                i += 1;
            }
            "--traj" => {
                let p: Vec<usize> = args[i + 1].split(',').map(|v| v.parse().unwrap()).collect();
                traj_n = p[0];
                traj_len = p[1];
                i += 1;
            }
            "--threads" => {
                let t: usize = args[i + 1].parse().unwrap();
                rayon::ThreadPoolBuilder::new().num_threads(t).build_global().unwrap();
                i += 1;
            }
            other => panic!("unknown argument {other}"),
        }
        i += 1;
    }
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map(|x| x == "json").unwrap_or(false)).filter(|p| !p.to_string_lossy().ends_with(".model.json")).collect();
    files.sort();
    let mut results = serde_json::Map::new();
    if out.exists() {
        if let Ok(Value::Object(m)) = serde_json::from_str::<Value>(&std::fs::read_to_string(&out).unwrap_or_default()) {
            results = m;
        }
    }
    for path in files {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        if let Some(o) = &only {
            if !o.contains(&name) {
                continue;
            }
        }
        // build (constraint processing) time, best of 5
        let mut build_s = f64::INFINITY;
        let mut prog = None;
        for _ in 0..5 {
            let t0 = Instant::now();
            let p = MpProgram::from_json_file(&path).unwrap();
            build_s = build_s.min(t0.elapsed().as_secs_f64());
            prog = Some(p);
        }
        let prog = Arc::new(prog.unwrap());
        println!("== {name}: x={} t={} constraints={} (eq {}) build {:.3} ms", prog.num_x(), prog.num_t(), prog.num_constraints(), prog.num_equality_constraints(), build_s * 1e3);
        let opts = SolveOptions::default();
        let mut reference_sol = None;
        let mut best_algo: Option<(f64, MpqpAlgorithm)> = None;
        results.insert(format!("{name}/info"), json!({"x": prog.num_x(), "t": prog.num_t(), "constraints": prog.num_constraints(), "equalities": prog.num_equality_constraints()}));
        for &algo in &algos {
            if resume && results.contains_key(&format!("{name}/{}", algo.name())) {
                let rec = &results[&format!("{name}/{}", algo.name())];
                if rec["timed_out"] != true {
                    let t = rec["solve_s"].as_f64().unwrap_or(f64::INFINITY);
                    if best_algo.map(|(b, _)| t < b).unwrap_or(true) {
                        best_algo = Some((t, algo));
                    }
                }
                continue;
            }
            // repeat short solves to get a stable time
            let mut times = vec![];
            let mut sol = None;
            let start = Instant::now();
            let lp0 = rpopt::solver::lp::LP_CALLS.load();
            let mut lp_per_run = 0u64;
            let (pl0, pq0) = rpopt::stats::snapshot();
            let pr0 = rpopt::stats::ranks();
            let mut ppopt_ops = (0u64, 0u64);
            let mut ppopt_ranks = 0u64;
            let mut timed_out = false;
            loop {
                let t0 = Instant::now();
                let s = if times.is_empty() && max_seconds > 0.0 {
                    match solve_capped(&prog, algo, &opts, max_seconds) {
                        Some(s) => s,
                        None => {
                            timed_out = true;
                            break;
                        }
                    }
                } else {
                    solve_mpqp_with(&prog, algo, &opts)
                };
                if times.is_empty() {
                    lp_per_run = rpopt::solver::lp::LP_CALLS.load() - lp0;
                    let (pl1, pq1) = rpopt::stats::snapshot();
                    ppopt_ops = (pl1 - pl0, pq1 - pq0);
                    ppopt_ranks = rpopt::stats::ranks() - pr0;
                }
                times.push(t0.elapsed().as_secs_f64());
                sol = Some(s);
                // median of many repetitions for fast solves (>= 7 runs and >= 0.3 s, at most 2000),
                // a single run once a solve takes more than 2 s
                let el = start.elapsed().as_secs_f64();
                if (times.len() >= 7 && el >= 0.3) || times.len() >= 2000 || el > 2.0 {
                    break;
                }
            }
            if timed_out {
                println!("  {:<28} TIMED OUT after {:.0} s (cap {:.0} s)", algo.name(), start.elapsed().as_secs_f64(), max_seconds);
                results.insert(format!("{name}/{}", algo.name()), json!({"build_s": build_s, "timed_out": true, "cap_s": max_seconds}));
                std::fs::write(&out, serde_json::to_string_pretty(&Value::Object(results.clone())).unwrap()).unwrap();
                // the solver thread cannot be cancelled: restart the process, skipping finished runs
                let mut a: Vec<String> = args[1..].to_vec();
                if !resume {
                    a.push("--resume".into());
                }
                use std::os::unix::process::CommandExt;
                let err = std::process::Command::new(std::env::current_exe().unwrap()).args(&a).exec();
                panic!("re-exec failed: {err}");
            }
            times.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let med = times[times.len() / 2];
            let sol = sol.unwrap();
            let sig = region_signature(&sol.critical_regions);
            let sig_hash = {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                sig.hash(&mut h);
                format!("{:016x}", h.finish())
            };
            let ref_key = format!("{name}/ref_signature");
            let same = match results.get(&ref_key).and_then(|v| v.as_str()) {
                Some(r) => r == sig_hash,
                None => {
                    results.insert(ref_key, json!(sig_hash));
                    true
                }
            };
            if best_algo.map(|(b, _)| med < b).unwrap_or(true) {
                best_algo = Some((med, algo));
            }
            let q = quality(&prog, &sol, 2000);
            println!(
                "  {:<28} {:>6} regions  {:>10.3} ms  (min {:.3}, runs {})  LPs {} (PPOPT-equivalent LPs {} QPs {})  same-as-first: {}  gaps {} overlaps {} wrong {}",
                algo.name(), sol.len(), med * 1e3, times[0] * 1e3, times.len(), lp_per_run, ppopt_ops.0, ppopt_ops.1, same, q.0, q.1, q.2
            );
            results.insert(
                format!("{name}/{}", algo.name()),
                json!({"build_s": build_s, "solve_s": med, "solve_min_s": times[0], "runs": times.len(), "regions": sol.len(), "lp_calls": lp_per_run, "ppopt_lps": ppopt_ops.0, "ppopt_qps": ppopt_ops.1, "ppopt_ranks": ppopt_ranks, "same_regions_as_first": same,
                       "sample_gaps": q.0, "sample_overlaps": q.1, "sample_wrong": q.2}),
            );
            if reference_sol.is_none() {
                reference_sol = Some(sol);
            }
            std::fs::write(&out, serde_json::to_string_pretty(&Value::Object(results.clone())).unwrap()).unwrap();
        }
        if reference_sol.is_none() {
            // every algorithm was finished in an earlier pass: re-solve with the fastest one
            if let Some((_, a)) = best_algo {
                reference_sol = Some(solve_mpqp_with(&prog, a, &opts));
            }
        }
        let Some(sol) = reference_sol else { continue };

        // correctness of the explicit solution against direct QP solves
        let nt = prog.num_t();
        let (lo, hi) = theta_box(&prog);
        let mut rng = StdRng::seed_from_u64(1);
        let mut bad = 0;
        let checks = 2000;
        for _ in 0..checks {
            let th: Vec<f64> = (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect();
            if !sol.verify_theta(&th) {
                bad += 1;
            }
        }
        println!("  verify_theta vs direct QP: {}/{} agree", checks - bad, checks);
        results.insert(format!("{name}/verify"), json!({"checked": checks, "mismatch": bad}));

        if !do_pl || sol.is_empty() {
            continue;
        }
        // point location
        let t0 = Instant::now();
        let geom_opt = if sol.len() <= geom_max_regions { Some(PartitionGeometry::new(&sol)) } else { None };
        let geom_s = t0.elapsed().as_secs_f64();
        match &geom_opt {
            Some(geom) => println!(
                "  geometry: {} planes, {} adjacencies, {} domain planes, {} facets with 2+ neighbours ({:.1} ms)",
                geom.planes.len(),
                geom.num_adjacencies(),
                geom.domain_planes.len(),
                geom.non_facet_to_facet(),
                geom_s * 1e3
            ),
            None => println!("  geometry skipped: {} regions > --geom-max-regions {} (only geometry-free locators are timed)", sol.len(), geom_max_regions),
        }
        let pts: Vec<Vec<f64>> = (0..queries).map(|_| (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect()).collect();
        let reference = FlatSequential::new(&sol);
        let truth: Vec<Option<usize>> = pts.par_iter().map(|p| reference.locate(p)).collect();
        let inside = truth.iter().filter(|t| t.is_some()).count();
        // closed-loop query sequences
        let (traj_pts, traj_mode) = if do_traj {
            let model = load_model(&path);
            let mode = if model.is_some() { "closed_loop_mpc" } else { "random_walk" };
            (make_trajectories(&sol, &reference, model.as_ref(), &lo, &hi, traj_n, traj_len), mode)
        } else {
            (vec![], "none")
        };
        let traj_truth: Vec<Option<usize>> = traj_pts.par_iter().map(|p| reference.locate(p)).collect();
        println!("  trajectories ({traj_mode}): {} points from {} x {} steps", traj_pts.len(), traj_n, traj_len);
        let mut locators: Vec<(Box<dyn PointLocator>, f64, Value)> = vec![];
        fn timed<L: PointLocator + 'static>(f: impl FnOnce() -> (L, Value)) -> (Box<dyn PointLocator>, f64, Value) {
            let t0 = Instant::now();
            let (l, info) = f();
            let el = t0.elapsed().as_secs_f64();
            println!("  built {} in {:.1} ms", l.name(), el * 1e3);
            (Box::new(l), el, info)
        }
        locators.push(timed(|| (SequentialSearch::new(&sol), json!({}))));
        locators.push(timed(|| (FlatSequential::new(&sol), json!({}))));
        if let Some(geom) = &geom_opt {
            if sol.len() <= bst_max_regions {
                let t0 = Instant::now();
                let bst = BinarySearchTree::new(geom);
                let bst_build = t0.elapsed().as_secs_f64();
                let (bst_depth, bst_nodes) = (bst.depth, bst.num_nodes());
                let mut rows = vec![];
                for i in 0..geom.num_regions {
                    let (e, f) = geom.region_rows(i);
                    for k in 0..f.len() {
                        rows.extend_from_slice(&e[k * nt..(k + 1) * nt]);
                        rows.push(f[k]);
                    }
                }
                let mut mws = vec![];
                for levels in [2usize, 3, 4] {
                    let t0 = Instant::now();
                    let mw = MultiwayTree::from_bst(&bst, levels, rows.clone(), geom.offsets.clone());
                    let (h, order) = (mw.height, mw.order());
                    mws.push((Box::new(mw) as Box<dyn PointLocator>, 0.0, json!({"order": order, "height": h, "levels": levels})));
                    let _ = t0;
                }
                locators.push((Box::new(bst), bst_build, json!({"depth": bst_depth, "nodes": bst_nodes})));
                locators.extend(mws);
            } else {
                println!("  BST and multiway trees skipped: {} regions > --bst-max-regions {} (node count explodes)", sol.len(), bst_max_regions);
            }
            let t0 = Instant::now();
            match LinearMachine::new(geom) {
                Some(lm) => {
                    let rep = lm.report.clone();
                    println!(
                        "  linear machine check: centers {}/{} correct, max vertex violation {:.2e}, min center margin {:.2e}, certified {}",
                        rep.centers_correct, rep.num_regions, rep.max_vertex_violation, rep.min_center_margin, rep.certified
                    );
                    let info = json!({"certified": rep.certified, "max_vertex_violation": rep.max_vertex_violation, "min_center_margin": rep.min_center_margin, "centers_correct": rep.centers_correct});
                    locators.push((Box::new(lm), t0.elapsed().as_secs_f64(), info));
                }
                None => {
                    let nff = geom.non_facet_to_facet();
                    println!("  linear machine: LP infeasible ({nff} facets shared with 2+ neighbours: partition is not facet-to-facet, so not a regular subdivision)");
                    results.insert(format!("{name}/linear_machine"), json!({"lp_feasible": false, "multi_neighbour_facets": nff}));
                }
            }
            locators.push(timed(|| {
                let df = DescriptorFunction::new(&sol, geom, 7);
                let info = json!({"compared_facets": df.compared_facets, "explicit_rows": df.explicit_rows});
                (df, info)
            }));
            for mult in [1usize, 4, 16] {
                let target = (geom.num_regions * mult).max(4);
                locators.push(timed(|| {
                    let ht = HashTable::new(geom, target);
                    let info = json!({"cells": ht.num_cells(), "mean_candidates": ht.mean_candidates(), "max_candidates": ht.max_candidates(), "mixed_cells": ht.mixed_cells(), "rows_per_mixed": ht.mean_rows_per_mixed_cell()});
                    (ht, info)
                }));
            }
            // storage-limited hash tables: multiples of the original uniform 16x-regions table, and fixed byte limits
            let old16: usize = match name.as_str() {
                "di_doc_N3" => 405,
                "di_N4" => 505,
                "di_N6" | "di_N10" | "di_N8" => 591,
                "masses2_N4" => 34647,
                "masses2_N6" => 112707,
                "sys3x2_N3" => 18938,
                "sys3x2_N5" => 84650,
                _ => HashTable::new(geom, (geom.num_regions * 16).max(4)).storage_floats(),
            };
            let mut limits: Vec<(String, usize)> = [1usize, 4, 16, 64].iter().map(|m| (format!("{m}x old16"), m * old16 * 8)).collect();
            limits.extend([(1usize << 20, "1 MB"), (8 << 20, "8 MB"), (64 << 20, "64 MB")].iter().map(|&(b, l)| (l.to_string(), b)));
            for (label, bytes) in limits {
                let label: &'static str = Box::leak(format!("hash table (limit {label})").into_boxed_str());
                locators.push(timed(|| {
                    let ht = HashTable::with_storage_limit(geom, bytes).with_name(label);
                    let info = json!({"limit_bytes": bytes, "storage_bytes": ht.storage_bytes(), "fits": ht.storage_bytes() <= bytes, "base_cells": ht.num_cells(), "mixed_leaves": ht.mixed_cells(), "mean_candidates": ht.mean_candidates(), "rows_per_mixed": ht.mean_rows_per_mixed_cell()});
                    (ht, info)
                }));
            }
        }
        locators.push(timed(|| (Yann::new(&sol), json!({}))));

        let mut pl_res = vec![];
        for (loc, build, info) in &locators {
            let (ns, timed_n, mism) = time_locator(loc.as_ref(), &pts, &truth);
            let (ns_t, timed_nt, mism_t) = if traj_pts.is_empty() { (f64::NAN, 0, 0) } else { time_locator(loc.as_ref(), &traj_pts, &traj_truth) };
            println!(
                "  {:<26} uniform {:>10.1} ns/q  trajectory {:>10.1} ns/q  storage {:>9} floats  build {:>9.2} ms  mismatches {}/{}  {}",
                loc.name(), ns, ns_t, loc.storage_floats(), build * 1e3, mism, mism_t, info
            );
            pl_res.push(json!({"method": loc.name(), "ns_per_query": ns, "queries_timed": timed_n, "ns_per_query_traj": ns_t, "queries_timed_traj": timed_nt,
                               "storage_floats": loc.storage_floats(), "build_s": build, "mismatches": mism, "mismatches_traj": mism_t, "info": info}));
        }
        results.insert(format!("{name}/point_location"), json!({"queries": queries, "inside_fraction": inside as f64 / queries as f64, "trajectory_mode": traj_mode, "trajectory_points": traj_pts.len(), "geometry_s": geom_s, "methods": pl_res}));
        std::fs::write(&out, serde_json::to_string_pretty(&Value::Object(results.clone())).unwrap()).unwrap();
    }
}

/// Sampled solution quality: (feasible points in no region, points strictly inside >= 2 regions,
/// points whose region law disagrees with a direct QP solve).
fn quality(prog: &MpProgram, sol: &rpopt::Solution, n: usize) -> (usize, usize, usize) {
    let nt = prog.num_t();
    let (lo, hi) = theta_box(prog);
    let mut rng = StdRng::seed_from_u64(11);
    let (mut gaps, mut overlaps, mut wrong) = (0, 0, 0);
    for _ in 0..n {
        let th: Vec<f64> = (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect();
        let direct = prog.solve_theta(&th);
        let inside: Vec<usize> = (0..sol.len()).filter(|&i| sol.critical_regions[i].is_inside(&th, -1e-7)).collect();
        if inside.len() >= 2 {
            overlaps += 1;
        }
        match (sol.get_region(&th), &direct) {
            (None, Some(_)) => gaps += 1,
            (Some(r), Some(d)) => {
                if !rpopt::linalg::allclose(&d.sol, &r.evaluate(&th), 1e-5, 1e-6) {
                    wrong += 1;
                }
            }
            (Some(_), None) => wrong += 1,
            (None, None) => {}
        }
    }
    (gaps, overlaps, wrong)
}

/// Box bounds of the parameter set A_t θ <= b_t.
fn theta_box(prog: &MpProgram) -> (Vec<f64>, Vec<f64>) {
    let n = prog.num_t();
    let mut lo = vec![-1.0; n];
    let mut hi = vec![1.0; n];
    for k in 0..n {
        let mut c = vec![0.0; n];
        c[k] = 1.0;
        if let Some(s) = rpopt::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
            lo[k] = s.obj;
        }
        c[k] = -1.0;
        if let Some(s) = rpopt::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
            hi[k] = -s.obj;
        }
    }
    (lo, hi)
}

/// Times `loc` on `pts` (ns per query). Slow locators are timed on a prefix so that a pass takes a few seconds at most;
/// returns (ns/query, number of queries timed, mismatches against `truth` on that prefix).
fn time_locator(loc: &dyn PointLocator, pts: &[Vec<f64>], truth: &[Option<usize>]) -> (f64, usize, usize) {
    let mut sink = 0usize;
    // probe: estimate the cost per query
    let probe = pts.len().min(200);
    let t0 = Instant::now();
    for p in &pts[..probe] {
        sink += loc.locate(p).unwrap_or(0);
    }
    let est = t0.elapsed().as_secs_f64() / probe.max(1) as f64;
    let n = if est * pts.len() as f64 > 3.0 { ((3.0 / est) as usize).clamp(probe, pts.len()) } else { pts.len() };
    let pts = &pts[..n];
    let passes = if est * n as f64 > 1.0 { 1 } else { 3 };
    // warm up, then best of `passes`
    for p in pts.iter().take(1000) {
        sink += loc.locate(p).unwrap_or(0);
    }
    let mut best = f64::INFINITY;
    for _ in 0..passes {
        let t0 = Instant::now();
        for p in pts {
            sink = sink.wrapping_add(loc.locate(std::hint::black_box(p)).unwrap_or(usize::MAX));
        }
        best = best.min(t0.elapsed().as_secs_f64());
    }
    std::hint::black_box(sink);
    let mism = pts.iter().zip(truth).filter(|(p, t)| loc.locate(p) != **t).count();
    (best * 1e9 / n as f64, n, mism)
}

/// Solves with a wall-clock cap; the worker thread is abandoned on timeout (the caller restarts the process).
fn solve_capped(prog: &Arc<MpProgram>, algo: MpqpAlgorithm, opts: &SolveOptions, secs: f64) -> Option<rpopt::Solution> {
    let (tx, rx) = std::sync::mpsc::channel();
    let (p, o) = (prog.clone(), opts.clone());
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let _ = tx.send(solve_mpqp_with(&p, algo, &o));
        })
        .unwrap();
    rx.recv_timeout(std::time::Duration::from_secs_f64(secs)).ok()
}

/// Closed-loop model sidecar `<name>.model.json`: x+ = A x + B u, the applied input is u_index-th block of the variables.
struct Model {
    a: Vec<Vec<f64>>,
    b: Vec<Vec<f64>>,
    u_index: usize,
}

fn load_model(problem: &std::path::Path) -> Option<Model> {
    let p = problem.with_extension("model.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()?;
    let mat = |k: &str| -> Vec<Vec<f64>> { v[k].as_array().unwrap().iter().map(|r| r.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()).collect() };
    Some(Model { a: mat("A"), b: mat("B"), u_index: v["u_index"].as_u64().unwrap_or(0) as usize })
}

/// `n` trajectories of up to `len` points inside the partition. With a model: x+ = A x + B u(x) from uniform random
/// initial states (a trajectory ends when it leaves the partition). Without: a random walk in theta with steps of
/// 2% of the box width per coordinate (a step leaving the partition is redrawn). Generated in parallel, one seed per trajectory.
fn make_trajectories(sol: &rpopt::Solution, loc: &FlatSequential, model: Option<&Model>, lo: &[f64], hi: &[f64], n: usize, len: usize) -> Vec<Vec<f64>> {
    let nt = lo.len();
    let trajs: Vec<Vec<Vec<f64>>> = (0..n)
        .into_par_iter()
        .map(|k| {
            let mut rng = StdRng::seed_from_u64(1000 + k as u64);
            let mut x: Option<Vec<f64>> = None;
            for _ in 0..200 {
                let c: Vec<f64> = (0..nt).map(|j| rng.gen_range(lo[j]..hi[j])).collect();
                if loc.locate(&c).is_some() {
                    x = Some(c);
                    break;
                }
            }
            let mut out = vec![];
            let Some(mut x) = x else { return out };
            for _ in 0..len {
                let Some(r) = loc.locate(&x) else { break };
                out.push(x.clone());
                match model {
                    Some(m) => {
                        let nu = m.b[0].len();
                        let law = sol.critical_regions[r].evaluate(&x);
                        let u = &law[m.u_index * nu..(m.u_index + 1) * nu];
                        x = (0..nt).map(|i| (0..nt).map(|j| m.a[i][j] * x[j]).sum::<f64>() + (0..nu).map(|j| m.b[i][j] * u[j]).sum::<f64>()).collect();
                    }
                    None => {
                        for _ in 0..20 {
                            let c: Vec<f64> = (0..nt).map(|j| (x[j] + 0.02 * (hi[j] - lo[j]) * rng.gen_range(-1.0..1.0)).clamp(lo[j], hi[j])).collect();
                            if loc.locate(&c).is_some() {
                                x = c;
                                break;
                            }
                        }
                    }
                }
            }
            out
        })
        .collect();
    trajs.into_iter().flatten().collect()
}
