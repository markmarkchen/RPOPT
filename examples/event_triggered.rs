//! Event-triggered mp (`rpopt::event_triggered`) on a Benders-like query pattern, against a direct
//! solve of every query.
//!
//! The first third of θ (at least one entry) is a "master" part shared by all scenarios that drifts
//! a little every iteration; the rest is fixed per scenario. Every scenario is a stream. Reported:
//! hit rate, how hits were found (safe ball, stream region after the sparse update, learned
//! successor, hot-list scan), regions tested per query, cached regions, time per query.
//!
//!   cargo run --profile gate --example event_triggered [-- problem.json ...] [--scenarios S] [--iters T]

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::event_triggered::{EtCache, EtOptions, Status};
use rpopt::program::MpProgram;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut files = vec![];
    let mut scenarios = 200usize;
    let mut iters = 50usize;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scenarios" => {
                scenarios = args[i + 1].parse().unwrap();
                i += 1;
            }
            "--iters" => {
                iters = args[i + 1].parse().unwrap();
                i += 1;
            }
            f => files.push(f.to_string()),
        }
        i += 1;
    }
    if files.is_empty() {
        let root = env!("CARGO_MANIFEST_DIR");
        for f in ["bench/problems/di_N8.json", "bench/problems/masses2_N6.json", "bench/problems_lp/diinf_N6.json", "bench/problems_lp/sys3x2inf_N4.json"] {
            files.push(format!("{root}/{f}"));
        }
    }
    println!("{scenarios} scenarios x {iters} iterations\n");
    println!(
        "{:<22} {:>3} {:>5} {:>8} {:>7} {:>6} {:>6} {:>6} {:>6} {:>7} {:>9} {:>9} {:>7}",
        "problem", "nt", "kind", "queries", "hit %", "ball", "strm", "succ", "scan", "tested", "direct us", "et us", "regions"
    );
    for f in files {
        run(&f, scenarios, iters);
    }
}

fn run(file: &str, scenarios: usize, iters: usize) {
    let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(file)).unwrap());
    let nt = p.num_t();
    let (lo, hi) = p.theta_bbox().unwrap().clone();
    let n_master = (nt / 3).max(1);
    let mut rng = StdRng::seed_from_u64(1);
    // scenario parts: random points of the box (only feasible combinations are kept below)
    let mid: Vec<f64> = lo.iter().zip(&hi).map(|(l, h)| 0.5 * (l + h)).collect();
    let scen: Vec<Vec<f64>> = (0..scenarios).map(|_| (0..nt).map(|k| mid[k] + 0.3 * (hi[k] - lo[k]) * (2.0 * rng.r#gen::<f64>() - 1.0)).collect()).collect();
    let mut master: Vec<f64> = mid[..n_master].to_vec();
    let mut thetas = Vec::with_capacity(scenarios * iters * nt);
    let mut streams = vec![];
    for _ in 0..iters {
        for k in 0..n_master {
            master[k] = (master[k] + 0.02 * (hi[k] - lo[k]) * (2.0 * rng.r#gen::<f64>() - 1.0)).clamp(lo[k], hi[k]);
        }
        for (s, sc) in scen.iter().enumerate() {
            thetas.extend_from_slice(&master);
            thetas.extend_from_slice(&sc[n_master..]);
            streams.push(s as u64);
        }
    }
    let n = streams.len();

    let t0 = Instant::now();
    let mut feasible = 0;
    for t in thetas.chunks(nt) {
        if p.solve_theta_uncounted(t).is_some() {
            feasible += 1;
        }
    }
    let direct = t0.elapsed().as_secs_f64();

    // one batch per iteration (all scenarios), as a Benders loop would query
    let mut c = EtCache::new(p.clone(), EtOptions::default());
    let t0 = Instant::now();
    let mut bad = 0;
    for it in 0..iters {
        let a = it * scenarios;
        let ans = c.query_batch(&thetas[a * nt..(a + scenarios) * nt], Some(&streams[a..a + scenarios]));
        bad += ans.iter().filter(|x| !matches!(x.status, Status::Hit | Status::Built | Status::Infeasible | Status::Solved)).count();
    }
    let et = t0.elapsed().as_secs_f64();
    let s = c.stats();
    let name = std::path::Path::new(file).file_stem().unwrap().to_string_lossy().to_string();
    let kind = if p.q.is_some() { "mpQP" } else { "mpLP" };
    let pct = |v: u64| 100.0 * v as f64 / s.hits.max(1) as f64;
    println!(
        "{:<22} {:>3} {:>5} {:>8} {:>7.2} {:>5.0}% {:>5.0}% {:>5.0}% {:>5.0}% {:>7.2} {:>9.2} {:>9.2} {:>7}",
        name,
        nt,
        kind,
        n,
        100.0 * s.hits as f64 / feasible.max(1) as f64,
        pct(s.ball_hits),
        pct(s.stream_hits),
        pct(s.successor_hits),
        pct(s.scan_hits),
        s.regions_tested as f64 / n as f64,
        1e6 * direct / n as f64,
        1e6 * et / n as f64,
        c.len()
    );
    println!(
        "    {} built, {} duplicate solves, {} infeasible, {} solved without caching, {} build failures, {} reductions",
        s.built, s.duplicates, s.infeasible, s.solved, s.build_failures, s.reductions
    );
    if bad > 0 {
        println!("    unexpected statuses: {bad}");
    }
}
