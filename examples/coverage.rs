//! Time to coverage: how fast an algorithm's regions cover the feasible parameter set.
//!
//!   cargo run --release --example coverage -- <problem.json> <algorithm> [ppopt_events.jsonl|-] [samples]
//!
//! Coverage at time t is the fraction of the feasible parameter set (estimated by uniform samples of
//! Θ that fall in the final, complete partition) lying in final regions whose active set had been
//! built by t. The Rust solve is repeated (at least 5 runs and 0.3 s, a single run above 2 s) and the
//! median time per coverage level is reported. With a PPOPT event log (`coverage_ppopt.py`: one JSON
//! line per built region, {"t": seconds since solve start, "as": active set}, and a final
//! {"done": ..., "solve_s": ..., "final": [...]} line if the run finished), the same levels are
//! computed for PPOPT against the same samples. Prints one JSON object.

use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const LEVELS: [f64; 5] = [0.5, 0.9, 0.99, 0.999, 1.0];

/// Times at which coverage reaches each level, given the first build time of each active set.
/// None for a level not reached.
fn level_times(weights: &HashMap<Vec<usize>, usize>, total: usize, first: &HashMap<Vec<usize>, f64>) -> Vec<Option<f64>> {
    let mut found: Vec<(f64, usize)> = weights.iter().filter_map(|(a, &w)| first.get(a).map(|&t| (t, w))).collect();
    found.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut out = vec![None; LEVELS.len()];
    let mut acc = 0usize;
    for (t, w) in found {
        acc += w;
        for (l, lv) in LEVELS.iter().enumerate() {
            if out[l].is_none() && acc as f64 >= lv * total as f64 - 1e-9 {
                out[l] = Some(t);
            }
        }
    }
    out
}

fn first_times(events: &[(Vec<usize>, f64)]) -> HashMap<Vec<usize>, f64> {
    let mut first: HashMap<Vec<usize>, f64> = HashMap::new();
    for (a, t) in events {
        let mut k = a.clone();
        k.sort_unstable();
        let e = first.entry(k).or_insert(*t);
        if *t < *e {
            *e = *t;
        }
    }
    first
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

fn json_opt(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.9}")).unwrap_or_else(|| "null".into())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(&args[1])).unwrap());
    let algo = MpqpAlgorithm::from_name(&args[2]).unwrap();
    let ppopt_path = args.get(3).filter(|s| s.as_str() != "-").cloned();
    let n_samples: usize = args.get(4).map(|s| s.parse().unwrap()).unwrap_or(20000);

    // Rust runs with the region log on
    let mut runs: Vec<(f64, Vec<(Vec<usize>, f64)>)> = vec![];
    let mut sol = None;
    let t_all = Instant::now();
    loop {
        rpopt::stats::record_regions(true);
        let t0 = Instant::now();
        let s = solve_mpqp(&prog, algo);
        let dt = t0.elapsed().as_secs_f64();
        rpopt::stats::record_regions(false);
        let ev = rpopt::stats::take_region_events().into_iter().map(|(a, t)| (a, t.duration_since(t0).as_secs_f64())).collect();
        runs.push((dt, ev));
        if sol.is_none() {
            sol = Some(s);
        }
        let n = runs.len();
        if runs[0].0 > 2.0 || (n >= 5 && t_all.elapsed().as_secs_f64() >= 0.3) || n >= 2000 {
            break;
        }
    }
    let sol = sol.unwrap();

    // samples of Θ located in the final partition (complete and overlap-free, checked by ppopt_bench)
    let (lo, hi) = prog.theta_bbox().cloned().expect("bounded parameter set");
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut weights: HashMap<Vec<usize>, usize> = HashMap::new();
    let mut in_theta = 0usize;
    let mut covered = 0usize;
    for _ in 0..n_samples {
        let th: Vec<f64> = lo.iter().zip(&hi).map(|(l, h)| rng.gen_range(*l..=*h)).collect();
        if !(0..prog.a_t.rows).all(|i| rpopt::linalg::dot(prog.a_t.row(i), &th) <= prog.b_t[i]) {
            continue;
        }
        in_theta += 1;
        if let Some(i) = sol.critical_regions.iter().position(|r| r.is_inside(&th, 1e-9)) {
            covered += 1;
            let mut k = sol.critical_regions[i].active_set.clone();
            k.sort_unstable();
            *weights.entry(k).or_insert(0) += 1;
        }
    }
    let finals: Vec<Vec<usize>> = sol
        .critical_regions
        .iter()
        .map(|r| {
            let mut k = r.active_set.clone();
            k.sort_unstable();
            k
        })
        .collect();

    // Rust: median over runs of each level time and of the time all final regions were built
    let mut per_level: Vec<Vec<f64>> = vec![vec![]; LEVELS.len()];
    let mut all_built = vec![];
    for (dt, ev) in &runs {
        let first = first_times(ev);
        for (l, t) in level_times(&weights, covered, &first).into_iter().enumerate() {
            per_level[l].push(t.unwrap_or(*dt));
        }
        all_built.push(finals.iter().map(|a| first.get(a).copied().unwrap_or(*dt)).fold(0.0, f64::max));
    }
    let rust_levels: Vec<f64> = per_level.into_iter().map(median).collect();
    let rust_total = median(runs.iter().map(|r| r.0).collect());
    let rust_all = median(all_built);

    // PPOPT event log
    let mut ppopt_json = "null".to_string();
    if let Some(path) = ppopt_path {
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let mut ev: Vec<(Vec<usize>, f64)> = vec![];
        let mut done: Option<(f64, Vec<Vec<usize>>)> = None;
        let mut last_t = 0.0f64;
        for line in text.lines() {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
            if let (Some(t), Some(a)) = (v.get("t").and_then(|x| x.as_f64()), v.get("as").and_then(|x| x.as_array())) {
                ev.push((a.iter().map(|x| x.as_u64().unwrap() as usize).collect(), t));
                last_t = last_t.max(t);
            } else if v.get("done").is_some() {
                let fin = v["final"].as_array().unwrap().iter().map(|r| {
                    let mut k: Vec<usize> = r.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect();
                    k.sort_unstable();
                    k
                });
                done = Some((v["solve_s"].as_f64().unwrap(), fin.collect()));
            }
            if let Some(e) = v.get("elapsed").and_then(|x| x.as_f64()) {
                last_t = last_t.max(e);
            }
        }
        let first = first_times(&ev);
        let levels = level_times(&weights, covered, &first);
        let found_final = finals.iter().filter(|a| first.contains_key(*a)).count();
        let all_built = if found_final == finals.len() { Some(finals.iter().map(|a| first[a]).fold(0.0, f64::max)) } else { None };
        // PPOPT's final active sets that are not in the Rust partition (should be none)
        let foreign = done.as_ref().map(|(_, f)| f.iter().filter(|a| !finals.contains(a)).count());
        ppopt_json = format!(
            "{{\"finished\": {}, \"solve_s\": {}, \"observed_s\": {:.6}, \"levels\": [{}], \"all_regions_built_s\": {}, \"final_regions_built\": {}, \"final_regions_not_in_rust\": {}}}",
            done.is_some(),
            json_opt(done.as_ref().map(|d| d.0)),
            last_t,
            levels.iter().map(|t| json_opt(*t)).collect::<Vec<_>>().join(", "),
            json_opt(all_built),
            found_final,
            foreign.map(|x| x.to_string()).unwrap_or_else(|| "null".into()),
        );
    }
    println!(
        "{{\"problem\": \"{}\", \"algorithm\": \"{}\", \"levels\": [{}], \"samples_in_theta\": {}, \"samples_covered\": {}, \"regions\": {}, \"regions_hit\": {}, \"rust\": {{\"runs\": {}, \"solve_s\": {:.9}, \"levels\": [{}], \"all_regions_built_s\": {:.9}}}, \"ppopt\": {}}}",
        std::path::Path::new(&args[1]).file_stem().unwrap().to_string_lossy(),
        algo.name(),
        LEVELS.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(", "),
        in_theta,
        covered,
        finals.len(),
        weights.len(),
        runs.len(),
        rust_total,
        rust_levels.iter().map(|t| format!("{t:.9}")).collect::<Vec<_>>().join(", "),
        rust_all,
        ppopt_json
    );
}
