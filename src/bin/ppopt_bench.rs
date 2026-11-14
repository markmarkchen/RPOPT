//! Benchmark runner: solves every JSON problem in a directory with every mpQP algorithm, checks
//! the solutions, then times all point location methods.
//!
//! Usage:
//!   ppopt_bench <problems_dir> <out.json> [--only a,b] [--algos x,y] [--queries N] [--no-pl] [--threads T]

use ppopt_rs::algorithms::{region_signature, solve_mpqp_with, MpqpAlgorithm, SolveOptions};
use ppopt_rs::point_location::*;
use ppopt_rs::program::MpProgram;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde_json::{json, Value};
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
            "--threads" => {
                let t: usize = args[i + 1].parse().unwrap();
                rayon::ThreadPoolBuilder::new().num_threads(t).build_global().unwrap();
                i += 1;
            }
            other => panic!("unknown argument {other}"),
        }
        i += 1;
    }
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map(|x| x == "json").unwrap_or(false)).collect();
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
        let mut reference_sig: Option<Vec<Vec<usize>>> = None;
        let mut reference_sol = None;
        for &algo in &algos {
            // repeat short solves to get a stable time
            let mut times = vec![];
            let mut sol;
            let start = Instant::now();
            let lp0 = ppopt_rs::solver::lp::LP_CALLS.load();
            let mut lp_per_run = 0u64;
            let (pl0, pq0) = ppopt_rs::stats::snapshot();
            let pr0 = ppopt_rs::stats::ranks();
            let mut ppopt_ops = (0u64, 0u64);
            let mut ppopt_ranks = 0u64;
            loop {
                let t0 = Instant::now();
                let s = solve_mpqp_with(&prog, algo, &opts);
                if times.is_empty() {
                    lp_per_run = ppopt_rs::solver::lp::LP_CALLS.load() - lp0;
                    let (pl1, pq1) = ppopt_rs::stats::snapshot();
                    ppopt_ops = (pl1 - pl0, pq1 - pq0);
                    ppopt_ranks = ppopt_rs::stats::ranks() - pr0;
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
            times.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let med = times[times.len() / 2];
            let sol = sol.unwrap();
            let sig = region_signature(&sol.critical_regions);
            let same = match &reference_sig {
                None => {
                    reference_sig = Some(sig.clone());
                    true
                }
                Some(r) => *r == sig,
            };
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
        let geom = PartitionGeometry::new(&sol);
        let geom_s = t0.elapsed().as_secs_f64();
        println!(
            "  geometry: {} planes, {} adjacencies, {} domain planes, {} facets with 2+ neighbours ({:.1} ms)",
            geom.planes.len(),
            geom.num_adjacencies(),
            geom.domain_planes.len(),
            geom.non_facet_to_facet(),
            geom_s * 1e3
        );
        let pts: Vec<Vec<f64>> = (0..queries).map(|_| (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect()).collect();
        let reference = FlatSequential::new(&sol);
        let truth: Vec<Option<usize>> = pts.iter().map(|p| reference.locate(p)).collect();
        let inside = truth.iter().filter(|t| t.is_some()).count();
        let mut locators: Vec<(Box<dyn PointLocator>, f64, Value)> = vec![];
        let timed = |f: &mut dyn FnMut() -> (Box<dyn PointLocator>, Value)| {
            let t0 = Instant::now();
            let (l, info) = f();
            (l, t0.elapsed().as_secs_f64(), info)
        };
        locators.push(timed(&mut || (Box::new(SequentialSearch::new(&sol)), json!({}))));
        locators.push(timed(&mut || (Box::new(FlatSequential::new(&sol)), json!({}))));
        let bst = BinarySearchTree::new(&geom);
        let bst_depth = bst.depth;
        let bst_nodes = bst.num_nodes();
        locators.push(timed(&mut || (Box::new(BinarySearchTree::new(&geom)), json!({"depth": bst_depth, "nodes": bst_nodes}))));
        let mut rows = vec![];
        for i in 0..geom.num_regions {
            let (e, f) = geom.region_rows(i);
            for k in 0..f.len() {
                rows.extend_from_slice(&e[k * nt..(k + 1) * nt]);
                rows.push(f[k]);
            }
        }
        for levels in [2usize, 3, 4] {
            let mw = MultiwayTree::from_bst(&bst, levels, rows.clone(), geom.offsets.clone());
            let h = mw.height;
            let order = mw.order();
            locators.push((Box::new(mw), 0.0, json!({"order": order, "height": h, "levels": levels})));
        }
        match LinearMachine::new(&geom) {
            Some(lm) => {
                let rep = lm.report.clone();
                println!(
                    "  linear machine check: centers {}/{} correct, max vertex violation {:.2e}, min center margin {:.2e}, certified {}",
                    rep.centers_correct, rep.num_regions, rep.max_vertex_violation, rep.min_center_margin, rep.certified
                );
                let info = json!({"certified": rep.certified, "max_vertex_violation": rep.max_vertex_violation, "min_center_margin": rep.min_center_margin, "centers_correct": rep.centers_correct});
                locators.push(timed(&mut || (Box::new(LinearMachine::new(&geom).unwrap()), info.clone())));
            }
            None => {
                let nff = geom.non_facet_to_facet();
                println!("  linear machine: LP infeasible ({nff} facets shared with 2+ neighbours: partition is not facet-to-facet, so not a regular subdivision)");
                results.insert(format!("{name}/linear_machine"), json!({"lp_feasible": false, "multi_neighbour_facets": nff}));
            }
        }
        let df = DescriptorFunction::new(&sol, &geom, 7);
        let dinfo = json!({"compared_facets": df.compared_facets, "explicit_rows": df.explicit_rows});
        locators.push(timed(&mut || (Box::new(DescriptorFunction::new(&sol, &geom, 7)), dinfo.clone())));
        for mult in [1usize, 4, 16] {
            let target = (geom.num_regions * mult).max(4);
            let ht = HashTable::new(&geom, target);
            let info = json!({"cells": ht.num_cells(), "mean_candidates": ht.mean_candidates(), "max_candidates": ht.max_candidates()});
            locators.push(timed(&mut || (Box::new(HashTable::new(&geom, target)), info.clone())));
        }
        locators.push(timed(&mut || (Box::new(Yann::new(&sol)), json!({}))));

        let mut pl_res = vec![];
        for (loc, build, info) in &locators {
            // warm up, then best of 3 passes
            let mut sink = 0usize;
            for p in pts.iter().take(1000) {
                sink += loc.locate(p).unwrap_or(0);
            }
            let mut best = f64::INFINITY;
            for _ in 0..3 {
                let t0 = Instant::now();
                for p in &pts {
                    sink = sink.wrapping_add(loc.locate(std::hint::black_box(p)).unwrap_or(usize::MAX));
                }
                best = best.min(t0.elapsed().as_secs_f64());
            }
            std::hint::black_box(sink);
            let mism = pts.iter().zip(&truth).filter(|(p, t)| loc.locate(p) != **t).count();
            let ns = best * 1e9 / pts.len() as f64;
            println!("  {:<26} {:>9.1} ns/query  storage {:>9} floats  build {:>9.2} ms  mismatches {}  {}", loc.name(), ns, loc.storage_floats(), build * 1e3, mism, info);
            pl_res.push(json!({"method": loc.name(), "ns_per_query": ns, "storage_floats": loc.storage_floats(), "build_s": build, "mismatches": mism, "info": info}));
        }
        results.insert(format!("{name}/point_location"), json!({"queries": queries, "inside_fraction": inside as f64 / queries as f64, "geometry_s": geom_s, "methods": pl_res}));
        std::fs::write(&out, serde_json::to_string_pretty(&Value::Object(results.clone())).unwrap()).unwrap();
    }
}

/// Sampled solution quality: (feasible points in no region, points strictly inside >= 2 regions,
/// points whose region law disagrees with a direct QP solve).
fn quality(prog: &MpProgram, sol: &ppopt_rs::Solution, n: usize) -> (usize, usize, usize) {
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
                if !ppopt_rs::linalg::allclose(&d.sol, &r.evaluate(&th), 1e-5, 1e-6) {
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
        if let Some(s) = ppopt_rs::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
            lo[k] = s.obj;
        }
        c[k] = -1.0;
        if let Some(s) = ppopt_rs::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
            hi[k] = -s.obj;
        }
    }
    (lo, hi)
}
