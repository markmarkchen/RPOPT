//! Where do the LP solves go? Runs (problem, algorithm) pairs with the LP/QP census on and prints
//! one table row per pair (see `src/census.rs`).
//!
//!   cargo run --release --features lp_census --example lp_census -- [--timeout S] [--detail] [--md]
//!   cargo run --release --features lp_census --example lp_census -- --pair <problem.json> <algorithm>
//!
//! Each pair runs in a child process (killed after the timeout, 60 s by default). A pair is solved
//! three times when the first solve takes under 5 s; the run with the median solve time is
//! reported (counts are deterministic, times are not). Times include the census bookkeeping
//! (two clock reads per LP call).

use rpopt::algorithms::{solve_mpqp_with, MpqpAlgorithm, SolveOptions};
use rpopt::census::{self, Counter, Stats, NAMES, NP};
use rpopt::program::MpProgram;
use serde_json::{json, Value};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const QP_ALGOS: [&str; 6] = ["geometric", "graph", "graph_exp", "combinatorial_graph", "mplcp", "combinatorial"];

fn child(path: &str, alg: &str) {
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(path)).unwrap());
    let algo = MpqpAlgorithm::from_name(alg).unwrap_or_else(|| panic!("unknown algorithm {alg}"));
    let opts = SolveOptions::default();
    let mut runs: Vec<(f64, usize, Stats)> = vec![];
    loop {
        census::reset();
        let t0 = Instant::now();
        let sol = solve_mpqp_with(&prog, algo, &opts);
        let dt = t0.elapsed().as_secs_f64();
        runs.push((dt, sol.critical_regions.len(), census::snapshot()));
        if dt > 5.0 || runs.len() == 3 {
            break;
        }
    }
    runs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (dt, regions, s) = &runs[runs.len() / 2];
    println!(
        "{}",
        json!({"regions": regions, "solve_s": dt, "runs": runs.len(), "calls": s.calls, "infeasible": s.infeasible, "nanos": s.nanos, "counters": s.counters})
    );
}

struct Row {
    problem: String,
    alg: String,
    regions: usize,
    solve_s: f64,
    s: Stats,
}

fn parse(v: &Value) -> (usize, f64, Stats) {
    let mut s = Stats::default();
    for k in 0..2 {
        for p in 0..NP {
            s.calls[k][p] = v["calls"][k][p].as_u64().unwrap();
            s.infeasible[k][p] = v["infeasible"][k][p].as_u64().unwrap();
            s.nanos[k][p] = v["nanos"][k][p].as_u64().unwrap();
        }
    }
    for (i, c) in v["counters"].as_array().unwrap().iter().enumerate() {
        s.counters[i] = c.as_u64().unwrap();
    }
    (v["regions"].as_u64().unwrap() as usize, v["solve_s"].as_f64().unwrap(), s)
}

fn pct(a: f64, b: f64) -> String {
    if b <= 0.0 { "-".into() } else { format!("{:.0}%", 100.0 * a / b) }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 4 && args[1] == "--pair" {
        child(&args[2], &args[3]);
        return;
    }
    let mut timeout = 60.0f64;
    let mut detail = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--timeout" => {
                timeout = args[i + 1].parse().unwrap();
                i += 1;
            }
            "--detail" => detail = true,
            "--md" => {}
            other => panic!("unknown argument {other}"),
        }
        i += 1;
    }
    let mut jobs: Vec<(String, String)> = vec![];
    for p in ["di_N10", "masses2_N6", "sys3x2_N5"] {
        for a in QP_ALGOS {
            jobs.push((format!("bench/problems/{p}.json"), a.into()));
        }
    }
    for (p, algs) in [("diinf_N4", vec!["geometric", "mplp_lex", "combinatorial"]), ("diinf_N6", vec!["geometric", "mplp_lex"])] {
        for a in algs {
            jobs.push((format!("bench/problems_lp/{p}.json"), a.into()));
        }
    }
    let exe = std::env::current_exe().unwrap();
    let mut rows: Vec<Row> = vec![];
    let mut skipped: Vec<(String, String)> = vec![];
    for (path, alg) in &jobs {
        let name = std::path::Path::new(path).file_stem().unwrap().to_string_lossy().to_string();
        let mut ch = Command::new(&exe).args(["--pair", path, alg]).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
        let start = Instant::now();
        let status = loop {
            if let Some(s) = ch.try_wait().unwrap() {
                break Some(s);
            }
            if start.elapsed() > Duration::from_secs_f64(timeout) {
                let _ = ch.kill();
                let _ = ch.wait();
                break None;
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        match status {
            Some(s) if s.success() => {
                let out = ch.wait_with_output().unwrap();
                let v: Value = serde_json::from_slice(&out.stdout).unwrap();
                let (regions, solve_s, s) = parse(&v);
                eprintln!("{name} {alg}: {regions} regions, {solve_s:.3} s");
                rows.push(Row { problem: name, alg: alg.clone(), regions, solve_s, s });
            }
            _ => {
                eprintln!("{name} {alg}: no result within {timeout} s");
                skipped.push((name, alg.clone()));
            }
        }
    }

    println!("### Overview\n");
    println!("| problem | algorithm | regions | solve ms | LPs | LPs/region | QPs | LP time / solve | build kept | build dup | build empty |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rows {
        let s = &r.s;
        let c = |k: Counter| s.counter(k);
        println!(
            "| {} | {} | {} | {:.1} | {} | {:.1} | {} | {} | {} | {} | {} |",
            r.problem,
            r.alg,
            r.regions,
            r.solve_s * 1e3,
            s.lp_calls(),
            s.lp_calls() as f64 / r.regions.max(1) as f64,
            s.qp_calls(),
            pct((s.lp_nanos() + s.qp_nanos()) as f64, r.solve_s * 1e9),
            c(Counter::Kept),
            c(Counter::Dup),
            c(Counter::Empty)
        );
    }
    for (p, a) in &skipped {
        println!("| {p} | {a} | > {timeout:.0} s, skipped | | | | | | | | |");
    }

    println!("\n### Share of LP count (and of LP time) by purpose\n");
    print!("| problem | algorithm |");
    for n in NAMES {
        print!(" {n} |");
    }
    println!();
    print!("|---|---|");
    for _ in 0..NP {
        print!("---:|");
    }
    println!();
    for r in &rows {
        let s = &r.s;
        print!("| {} | {} |", r.problem, r.alg);
        for p in 0..NP {
            if s.calls[0][p] == 0 {
                print!(" - |");
            } else {
                print!(" {} ({}) |", pct(s.calls[0][p] as f64, s.lp_calls() as f64), pct(s.nanos[0][p] as f64, s.lp_nanos() as f64));
            }
        }
        println!();
    }

    println!("\n### Share of total solve time by purpose (LP+QP time) and Amdahl bound 1/(1-f)\n");
    print!("| problem | algorithm | LP+QP / solve |");
    for n in NAMES {
        print!(" {n} |");
    }
    println!();
    print!("|---|---|---:|");
    for _ in 0..NP {
        print!("---:|");
    }
    println!();
    for r in &rows {
        let s = &r.s;
        let tot = r.solve_s * 1e9;
        print!("| {} | {} | {} |", r.problem, r.alg, pct((s.lp_nanos() + s.qp_nanos()) as f64, tot));
        for p in 0..NP {
            let t = (s.nanos[0][p] + s.nanos[1][p]) as f64;
            if t <= 0.0 {
                print!(" - |");
            } else {
                print!(" {:.0}% (x{:.2}) |", 100.0 * t / tot, 1.0 / (1.0 - (t / tot).min(0.999)));
            }
        }
        println!();
    }

    println!("\n### Region construction (inside gen_cr_*): share of solve time and Amdahl bound if its LPs are cut\n");
    println!("| problem | algorithm | construction / solve | of which LP | non-LP part | x if LP -50% | x if LP -80% | x if LP -100% | x if construction free |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rows {
        let s = &r.s;
        let tot = r.solve_s * 1e9;
        let b = s.counter(Counter::BuildNanos) as f64 / tot;
        let l = s.counter(Counter::BuildLpNanos) as f64 / tot;
        if b <= 0.0 {
            println!("| {} | {} | 0% | - | - | 1.00 | 1.00 | 1.00 | 1.00 |", r.problem, r.alg);
            continue;
        }
        let x = |cut: f64| 1.0 / (1.0 - cut * l);
        println!("| {} | {} | {:.0}% | {:.0}% | {:.0}% | {:.2} | {:.2} | {:.2} | {:.2} |", r.problem, r.alg, 100.0 * b, 100.0 * l, 100.0 * (b - l), x(0.5), x(0.8), x(1.0), 1.0 / (1.0 - b.min(0.999)));
    }

    println!("\n### QP solves by purpose\n");
    println!("| problem | algorithm | QPs | by purpose |");
    println!("|---|---|---:|---|");
    for r in rows.iter().filter(|r| r.s.qp_calls() > 0) {
        let parts: Vec<String> = (0..NP).filter(|&p| r.s.calls[1][p] > 0).map(|p| format!("{} {} ({} infeasible, {:.1} ms)", NAMES[p], r.s.calls[1][p], r.s.infeasible[1][p], r.s.nanos[1][p] as f64 / 1e6)).collect();
        println!("| {} | {} | {} | {} |", r.problem, r.alg, r.s.qp_calls(), parts.join("; "));
    }

    println!("\n### Redundancy removal (rows entering, kept, decided without an LP, tested by LP)\n");
    println!("| problem | algorithm | region builds | rows in | rows in / build | kept / build | no LP | LP | LP rows infeasible (redundant) |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.s.counter(Counter::RedRowsIn) > 0) {
        let s = &r.s;
        let built = s.counter(Counter::Kept).max(1) as f64;
        let rin = s.counter(Counter::RedRowsIn);
        let lp = s.calls[0][0];
        println!(
            "| {} | {} | {} | {} | {:.1} | {:.1} | {} ({}) | {} ({}) | {} |",
            r.problem,
            r.alg,
            s.counter(Counter::Kept),
            rin,
            rin as f64 / built,
            s.counter(Counter::RedRowsKept) as f64 / built,
            rin.saturating_sub(lp),
            pct(rin.saturating_sub(lp) as f64, rin as f64),
            lp,
            pct(lp as f64, rin as f64),
            s.infeasible[0][0]
        );
    }

    println!("\n### Combinatorial candidates\n");
    println!("| problem | algorithm | generated | pruned by list | rank deficient | accepted by witness | tested by LP | LP infeasible |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.s.counter(Counter::CombGen) > 0) {
        let c = |k: Counter| r.s.counter(k);
        println!("| {} | {} | {} | {} | {} | {} | {} | {} |", r.problem, r.alg, c(Counter::CombGen), c(Counter::CombPruned), c(Counter::CombRankFail), c(Counter::CombWitness), c(Counter::CombLp), c(Counter::CombInfeasible));
    }

    if detail {
        println!("\n### Detail: LP calls / infeasible / ms by purpose\n");
        print!("| problem | algorithm |");
        for n in NAMES {
            print!(" {n} |");
        }
        println!();
        print!("|---|---|");
        for _ in 0..NP {
            print!("---:|");
        }
        println!();
        for r in &rows {
            print!("| {} | {} |", r.problem, r.alg);
            for p in 0..NP {
                if r.s.calls[0][p] == 0 {
                    print!(" - |");
                } else {
                    print!(" {} / {} / {:.1} |", r.s.calls[0][p], r.s.infeasible[0][p], r.s.nanos[0][p] as f64 / 1e6);
                }
            }
            println!();
        }
    }
}
