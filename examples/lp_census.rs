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
static GLOBAL: census::CountingAlloc<mimalloc::MiMalloc> = census::CountingAlloc(mimalloc::MiMalloc);

const QP_ALGOS: [&str; 6] = ["geometric", "graph", "graph_exp", "combinatorial_graph", "mplcp", "combinatorial"];

fn child(path: &str, alg: &str) {
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(path)).unwrap());
    let algo = MpqpAlgorithm::from_name(alg).unwrap_or_else(|| panic!("unknown algorithm {alg}"));
    let opts = SolveOptions::default();
    // MPLCP_TAB_BUDGET=<bytes>: the tableau budget of the serial mplcp search (0 = none); a small value forces the rebuilds
    if let Some(b) = std::env::var("MPLCP_TAB_BUDGET").ok().and_then(|v| v.parse().ok()) {
        rpopt::algorithms::mplcp::set_tab_budget(b);
    }
    // RPOPT_BOXCHAIN=0: mplcp builds its regions with the shared redundancy step (the build study counters need it)
    if std::env::var("RPOPT_BOXCHAIN").map_or(false, |v| v == "0") {
        rpopt::algorithms::mplcp::set_box_chain(false);
    }
    // MPLCP_NO_DEDUPE=1: solve with the duplicated constraints
    if std::env::var("MPLCP_NO_DEDUPE").is_ok() {
        rpopt::algorithms::mplcp::set_dedupe(false);
    }
    // RPOPT_CERT=0|1|2|3|4: mplcp facet certificate off (default), on (3 to 5 parameters), audit, on at 2 to 6, audit at 2 to 6
    if std::env::var("MPLCP_MU_AUDIT").is_ok() {
        rpopt::algorithms::mplcp::set_mu_audit(true);
    }
    if std::env::var("MPLCP_REFINE").is_ok() {
        rpopt::algorithms::mplcp::set_refine_multipliers(true);
    }
    if let Ok(m) = std::env::var("RPOPT_CERT") {
        rpopt::algorithms::mplcp::set_certificate(m.parse().unwrap());
    }
    // MPLCP_TAB_BUDGET=<bytes>: limit on live parent tableaux (default 256 MiB, 0 = none)
    if let Some(b) = std::env::var("MPLCP_TAB_BUDGET").ok().and_then(|v| v.parse().ok()) {
        rpopt::algorithms::mplcp::set_tab_budget(b);
    }
    let mut runs: Vec<(f64, usize, Stats)> = vec![];
    let lap_ns = census::calibrate();
    loop {
        census::reset();
        rpopt::algorithms::mplcp::census_reset_local();
        census::set_calib(lap_ns);
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
        json!({"regions": regions, "solve_s": dt, "runs": runs.len(), "calls": s.calls, "infeasible": s.infeasible, "nanos": s.nanos, "counters": s.counters.to_vec()})
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
    let mut problems: Option<Vec<String>> = None;
    let mut algs: Option<Vec<String>> = None;
    let mut highdim = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--timeout" => {
                timeout = args[i + 1].parse().unwrap();
                i += 1;
            }
            "--detail" => detail = true,
            "--md" => {}
            "--problems" => {
                problems = Some(args[i + 1].split(',').map(|x| x.to_string()).collect());
                i += 1;
            }
            "--algs" => {
                algs = Some(args[i + 1].split(',').map(|x| x.to_string()).collect());
                i += 1;
            }
            "--highdim" => highdim = true,
            other => panic!("unknown argument {other}"),
        }
        i += 1;
    }
    let mut jobs: Vec<(String, String)> = vec![];
    if let Some(ps) = &problems {
        let al: Vec<String> = algs.clone().unwrap_or_else(|| QP_ALGOS.iter().map(|x| x.to_string()).collect());
        for p in ps {
            let path = ["bench/problems_large", "bench/problems_highdim", "bench/problems"].iter().map(|d| format!("{d}/{p}.json")).find(|f| std::path::Path::new(f).exists()).unwrap_or_else(|| panic!("no problem {p}"));
            for a in &al {
                jobs.push((path.clone(), a.clone()));
            }
        }
    } else {
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

    if highdim {
        highdim_tables(&rows, &skipped);
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

fn highdim_tables(rows: &[Row], skipped: &[(String, String)]) {
    let c = |r: &Row, k: Counter| r.s.counter(k) as f64;
    let per = |a: f64, b: f64| if b <= 0.0 { "-".to_string() } else { format!("{:.1}", a / b) };
    println!("\n### High-dimension 1: cost per region\n");
    println!("| problem | algorithm | regions | solve ms | us/region | LP/region | QP/region | LP+QP share | facets/region | build ms/region |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows {
        let n = r.regions.max(1) as f64;
        println!(
            "| {} | {} | {} | {:.0} | {:.0} | {:.1} | {:.1} | {} | {} | {:.2} |",
            r.problem, r.alg, r.regions, r.solve_s * 1e3, r.solve_s * 1e6 / n, r.s.lp_calls() as f64 / n, r.s.qp_calls() as f64 / n,
            pct((r.s.lp_nanos() + r.s.qp_nanos()) as f64, r.solve_s * 1e9), per(c(r, Counter::FacetRows), n), (c(r, Counter::BuildNanos) + c(r, Counter::MplcpBuildNanos)) / 1e6 / n
        );
    }
    println!("\n### High-dimension 2: LP calls per region by purpose (time share of solve in brackets)\n");
    print!("| problem | algorithm |");
    for nm in NAMES {
        print!(" {nm} |");
    }
    println!();
    print!("|---|---|");
    for _ in 0..NP {
        print!("---:|");
    }
    println!();
    for r in rows {
        let n = r.regions.max(1) as f64;
        print!("| {} | {} |", r.problem, r.alg);
        for p in 0..NP {
            let (calls, t) = (r.s.calls[0][p] + r.s.calls[1][p], (r.s.nanos[0][p] + r.s.nanos[1][p]) as f64);
            if calls == 0 {
                print!(" - |");
            } else {
                print!(" {:.1} ({}) |", calls as f64 / n, pct(t, r.solve_s * 1e9));
            }
        }
        println!();
    }
    println!("\n### High-dimension 3: mplcp neighbour search (per `adjacent` call) and time split\n");
    println!("| problem | regions | adjacent/region | no zero row | Case 1 fast | Case 2 | us/call none | us/call fast | us/call Case 2 | Case 2 share of adjacent time | neighbour search | region build | tableau update | rest |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::AdjCalls) > 0.0) {
        let (n, a) = (r.regions.max(1) as f64, c(r, Counter::AdjCalls));
        let us = |t: Counter, k: Counter| if c(r, k) > 0.0 { format!("{:.1}", c(r, t) / 1e3 / c(r, k)) } else { "-".into() };
        let tot = r.solve_s * 1e9;
        let (nb, bd, tb) = (c(r, Counter::MplcpNbrNanos), c(r, Counter::MplcpBuildNanos), c(r, Counter::MplcpTabNanos));
        let adj_t = c(r, Counter::AdjNoneNanos) + c(r, Counter::AdjFastNanos) + c(r, Counter::AdjCase2Nanos);
        println!(
            "| {} | {} | {:.1} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            r.problem, r.regions, a / n, pct(c(r, Counter::AdjNone), a), pct(c(r, Counter::AdjFast), a), pct(c(r, Counter::AdjCase2), a),
            us(Counter::AdjNoneNanos, Counter::AdjNone), us(Counter::AdjFastNanos, Counter::AdjFast), us(Counter::AdjCase2Nanos, Counter::AdjCase2),
            pct(c(r, Counter::AdjCase2Nanos), adj_t), pct(nb, tot), pct(bd, tot), pct(tb, tot), pct((tot - nb - bd - tb).max(0.0), tot)
        );
    }
    println!("\n### High-dimension 3b: mplcp candidates that end at a known region\n");
    println!("| problem | candidates/region | fast returned | fast already seen | slow returned | slow already seen | adjacent calls with candidates | all seen (fast) | all seen (slow) | clip rows sharing a hyperplane/region |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::AdjCalls) > 0.0) {
        let n = r.regions.max(1) as f64;
        let (fr, sr) = (c(r, Counter::NbrFastRet), c(r, Counter::NbrSlowRet));
        println!(
            "| {} | {:.1} | {} | {} | {} | {} | {:.1}/region | {} | {} | {} |",
            r.problem, (fr + sr) / n, fr, pct(c(r, Counter::NbrFastSeen), fr), sr, pct(c(r, Counter::NbrSlowSeen), sr),
            (c(r, Counter::GrpFast) + c(r, Counter::GrpSlow)) / n, pct(c(r, Counter::GrpFastAllSeen), c(r, Counter::GrpFast)), pct(c(r, Counter::GrpSlowAllSeen), c(r, Counter::GrpSlow)),
            per(c(r, Counter::ClipDupRows), n)
        );
    }
    println!("\n### High-dimension 3c: mplcp `pivot_facet` (degenerate slow path)\n");
    println!("| problem | calls/region | Case-1 iters | Case-2 iters | branches | cut by lex_nonneg | Chebyshev LPs | empty facet | repeated basis | depth 0/1/2/3/4/5/6/7+ | depth>6 returns | iteration-limit exits |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::AdjCalls) > 0.0) {
        let n = r.regions.max(1) as f64;
        let ds: Vec<String> = [Counter::PfDepth0, Counter::PfDepth1, Counter::PfDepth2, Counter::PfDepth3, Counter::PfDepth4, Counter::PfDepth5, Counter::PfDepth6, Counter::PfDepth7].iter().map(|&k| format!("{}", c(r, k) as u64)).collect();
        let br = c(r, Counter::PfBranches);
        println!(
            "| {} | {:.1} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            r.problem, c(r, Counter::PfCalls) / n, c(r, Counter::PfCase1), c(r, Counter::PfCase2), br, pct(c(r, Counter::PfLexCut), br), c(r, Counter::PfCheb),
            pct(c(r, Counter::PfChebEmpty), c(r, Counter::PfCheb)), pct(c(r, Counter::PfRepeatBasis), br), ds.join("/"), c(r, Counter::PfDepthLimit), c(r, Counter::PfIterLimit)
        );
    }
    println!("\n### High-dimension 3d: mplcp facet certificate (RPOPT_CERT=1 skips, 2 audits; asked only for Case-2 facets)\n");
    println!("| problem | regions | Case-2 facets/region | certified | search skipped | audit: search also found a new region | check us/facet | check share of solve |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::CertFacets) > 0.0) {
        let n = r.regions.max(1) as f64;
        let (fe, hit) = (c(r, Counter::CertFacets), c(r, Counter::CertHit));
        println!(
            "| {} | {} | {:.1} | {} | {} | {} | {:.2} | {} |",
            r.problem, r.regions, fe / n, pct(hit, fe), c(r, Counter::AdjCertSkip), c(r, Counter::CertExtra),
            c(r, Counter::CertNanos) / 1e3 / fe, pct(c(r, Counter::CertNanos), r.solve_s * 1e9)
        );
    }
    println!("\n### High-dimension 3e: mplcp Case-2 leaf pruning, sub-facet hints and walk limits\n");
    println!("| problem | regions | LP/region | Chebyshev LPs/region in pivot_facet | leaf branches skipped | leaves kept | leaves with own centre | old hint inside / outside the leaf sub-facet | cycle stops | budget stops | fallbacks found / failed |");
    println!("|---|---:|---:|---:|---:|---:|---:|---|---:|---:|---|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::AdjCalls) > 0.0) {
        let n = r.regions.max(1) as f64;
        let (sk, kp) = (c(r, Counter::LeafSkip), c(r, Counter::LeafKept));
        println!(
            "| {} | {} | {:.1} | {:.1} | {} ({}) | {} | {} | {} / {} | {} | {} | {} / {} |",
            r.problem, r.regions, r.s.lp_calls() as f64 / n, c(r, Counter::PfCheb) / n, sk, pct(sk, sk + kp), kp, c(r, Counter::HintCentre),
            c(r, Counter::HintInside), c(r, Counter::HintOutside), c(r, Counter::WalkCycle), c(r, Counter::WalkBudget), c(r, Counter::FallbackAcross), c(r, Counter::FallbackFail)
        );
    }
    println!("\n### High-dimension 3f: mplcp sub-facet witnesses (non-leaf Case-2 branches accepted without a Chebyshev LP)\n");
    println!("| problem | regions | Chebyshev LPs/region in pivot_facet | branches accepted by a witness | of which piece = parent | candidates tried, none cleared | LPs found empty |");
    println!("|---|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::AdjCalls) > 0.0) {
        let n = r.regions.max(1) as f64;
        let (acc, fail) = (c(r, Counter::WitnessAccept), c(r, Counter::WitnessFail));
        println!(
            "| {} | {} | {:.1} | {} ({}) | {} | {} | {} |",
            r.problem, r.regions, c(r, Counter::PfCheb) / n, acc, pct(acc, acc + c(r, Counter::PfCheb)), c(r, Counter::WitnessNoCuts), fail, c(r, Counter::PfChebEmpty)
        );
    }
    println!("\n### High-dimension 3g: mplcp Case-2 calls by class (rank-1 census)\n");
    println!("| problem | Case-2 calls | one slack row, zero pivot, slack columns zero | one slack row, zero pivot, slack columns not zero | one slack row, other | one multiplier row | two rows | three or more rows | time share of the same classes (ms) |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::AdjCase2) > 0.0) {
        let cs = [Counter::R1Calls0, Counter::R1Calls1, Counter::R1Calls2, Counter::R1Calls3, Counter::R1Calls4, Counter::R1Calls5];
        let ns = [Counter::R1Nanos0, Counter::R1Nanos1, Counter::R1Nanos2, Counter::R1Nanos3, Counter::R1Nanos4, Counter::R1Nanos5];
        let tot = c(r, Counter::AdjCase2);
        let times: Vec<String> = ns.iter().map(|&k| format!("{:.0}", c(r, k) / 1e6)).collect();
        println!(
            "| {} | {} | {} | {} |",
            r.problem, tot, cs.iter().map(|&k| format!("{} ({})", c(r, k), pct(c(r, k), tot))).collect::<Vec<_>>().join(" | "), times.join(" / ")
        );
    }
    println!("\n### High-dimension 3h: mplcp rank-1 calls (one slack row, zero pivot, slack columns zero): |P-|, the candidate set S, depth of the current walk\n");
    println!("| problem | calls | |P-| = 0 / 1 / 2 / 3 / 4 / 5 / 6 / 7+ | members of S | already in Seen | calls with all of S in Seen | unseen members with an empty sub-facet | unseen with a nonempty sub-facet | calls with a structural tie | current walk depth 0 / 1 / 2 / 3-5 / 6+ |");
    println!("|---|---:|---|---:|---:|---:|---:|---:|---:|---|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::R1Calls0) > 0.0) {
        let h: Vec<String> = [Counter::R1P0, Counter::R1P1, Counter::R1P2, Counter::R1P3, Counter::R1P4, Counter::R1P5, Counter::R1P6, Counter::R1P7].iter().map(|&k| format!("{}", c(r, k))).collect();
        let d: Vec<String> = [Counter::R1D0, Counter::R1D1, Counter::R1D2, Counter::R1D3to5, Counter::R1D6p].iter().map(|&k| format!("{}", c(r, k))).collect();
        let (m, u) = (c(r, Counter::R1Members), c(r, Counter::R1Unseen));
        println!(
            "| {} | {} | {} | {} | {} ({}) | {} ({}) | {} | {} | {} | {} |",
            r.problem, c(r, Counter::R1Calls0), h.join(" / "), m, m - u, pct(m - u, m), c(r, Counter::R1AllSeen), pct(c(r, Counter::R1AllSeen), c(r, Counter::R1Calls0)),
            c(r, Counter::R1UnseenEmpty), c(r, Counter::R1UnseenNonEmpty), c(r, Counter::R1Ties), d.join(" / ")
        );
    }
    println!("\n### High-dimension 3i: mplcp rank-1 enumeration of Case 2\n");
    println!("| problem | Case-2 calls | answered by the enumeration | handed to the walk | candidates emitted | members dropped as known | emitted on a witness | sub-facet LPs | members with an empty sub-facet | neighbour LPs/region | distinct active sets tested | repeated tests | of which empty | fast path by relative pivot | pivot measure in the gap | fast path demoted to rank-1 |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::AdjCase2) > 0.0) {
        let n = r.regions.max(1) as f64;
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.1} | {} | {} | {} | {} | {} | {} |",
            r.problem, c(r, Counter::AdjCase2), c(r, Counter::R1Handled), c(r, Counter::R1Fallback), c(r, Counter::R1Emitted), c(r, Counter::R1SkipSeen),
            c(r, Counter::R1WitnessAcc), c(r, Counter::R1LpRun), c(r, Counter::R1EmptyMember), c(r, Counter::PfCheb) / n,
            c(r, Counter::R1TestDistinct), c(r, Counter::R1TestRepeat), c(r, Counter::R1EmptyRepeat), c(r, Counter::FastRel), c(r, Counter::PivotGap), c(r, Counter::FastDemoted)
        );
    }
    println!("\n### High-dimension 3j: parametric_daqp active-set tests and region builds\n");
    println!("| problem | regions | solve ms | tests/region | LICQ violated | empty | nonempty | empty before any DAQP solve | DAQP solves (stage 1 / stage 2) | LP fallbacks | ms in the tests | of which DAQP | of which LDL | ms in gen_cr_from_kkt | us per test | us per DAQP solve | other ms |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "parametric_daqp" && c(r, Counter::PdTests) > 0.0) {
        let n = r.regions.max(1) as f64;
        let (t, tn, dn, bn) = (c(r, Counter::PdTests), c(r, Counter::PdTestNanos), c(r, Counter::PdDaqpNanos), c(r, Counter::PdBuildNanos));
        let ds = c(r, Counter::PdDaqp1) + c(r, Counter::PdDaqp2);
        println!(
            "| {} | {} | {:.0} | {:.1} | {} | {} | {} | {} | {} / {} | {} | {:.0} | {:.0} | {:.0} | {:.0} | {:.2} | {:.2} | {:.0} |",
            r.problem, r.regions, r.solve_s * 1e3, t / n, c(r, Counter::PdLicq), c(r, Counter::PdEmpty), c(r, Counter::PdRegion), c(r, Counter::PdEarlyEmpty),
            c(r, Counter::PdDaqp1), c(r, Counter::PdDaqp2), c(r, Counter::PdLpFallback), tn / 1e6, dn / 1e6, c(r, Counter::PdLdlNanos) / 1e6, bn / 1e6, tn / t / 1e3, dn / ds.max(1.0) / 1e3,
            r.solve_s * 1e3 - tn / 1e6 - bn / 1e6
        );
    }
    println!("\n### High-dimension 3k: mplcp sub-facet emptiness tests answered by DAQP\n");
    println!("| problem | regions | solve ms | DAQP empty / region | sent on to the LP / region | share answered by DAQP | us per DAQP test | LPs / region |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::DqEmpty) + c(r, Counter::DqFallback) > 0.0) {
        let n = r.regions.max(1) as f64;
        let (b, d) = (c(r, Counter::DqEmpty), c(r, Counter::DqFallback));
        println!(
            "| {} | {} | {:.0} | {:.2} | {:.2} | {:.0}% | {:.2} | {:.1} |",
            r.problem, r.regions, r.solve_s * 1e3, b / n, d / n, 100.0 * b / (b + d), c(r, Counter::DqNanos) / (b + d) / 1e3, c(r, Counter::PfCheb) / n
        );
    }
    println!("\n### High-dimension 3l: mplcp Case-2 cost by component (rank-1 = case 2, walk = case 3)\n");
    println!("Per `adjacent` call that ended in the case. `us` is microseconds per call, `al` allocations per call. The census overhead (basis_seen, rank-1 probe, tested map, hint check) is taken out of the total and of the rest. Timer cost per lap is in the last column; every lap costs two clock reads and counter updates, so components with many laps are inflated by about that much per lap.\n");
    for r in rows.iter().filter(|r| r.alg == "mplcp") {
        let mut any = false;
        for ci in 0..2usize {
            let g = |comp: usize, f: usize| r.s.counters[census::c2_slot(comp, ci, f)] as f64;
            let calls = g(0, 1);
            if calls == 0.0 {
                continue;
            }
            if !any {
                println!("**{}** ({} regions, solve {:.0} ms, {} Case-2 calls/region, lap cost {:.0} ns)\n", r.problem, r.regions, r.solve_s * 1e3, per(calls / r.regions.max(1) as f64, 1.0), r.s.counters[census::CALIB] as f64 / 1000.0);
                println!("| case | component | us/call | al/call | KB/call | laps/call | share of solve |");
                println!("|---|---|---:|---:|---:|---:|---:|");
                any = true;
            }
            let cname = if ci == 0 { "rank-1 (2)" } else { "walk (3)" };
            let cen = g(census::Comp::Census as usize, 0) + g(census::Comp::HintCensus as usize, 0);
            let cen_al = g(census::Comp::Census as usize, 2) + g(census::Comp::HintCensus as usize, 2);
            let tot = g(0, 0) - cen;
            let solve_ns = r.solve_s * 1e9;
            let mut sum = 0.0;
            let mut sum_al = 0.0;
            let mut sum_by = 0.0;
            let mut laps = 0.0;
            println!("| {cname} | **total** ({} calls, census removed) | {:.2} | {:.1} | {:.1} | - | {} |", calls, tot / calls / 1e3, (g(0, 2) - cen_al) / calls, g(0, 3) / calls / 1024.0, pct(tot, solve_ns));
            for comp in 1..census::NCOMP {
                if comp == census::Comp::Census as usize || comp == census::Comp::HintCensus as usize {
                    continue;
                }
                let (mut ns, mut al, mut by, lp) = (g(comp, 0), g(comp, 2), g(comp, 3), g(comp, 1));
                if comp == census::Comp::Leaf as usize {
                    ns -= g(census::Comp::HintCensus as usize, 0);
                    al -= g(census::Comp::HintCensus as usize, 2);
                    by -= g(census::Comp::HintCensus as usize, 3);
                }
                if lp == 0.0 {
                    continue;
                }
                sum += ns;
                sum_al += al;
                sum_by += by;
                laps += lp;
                println!("| {cname} | {} | {:.2} | {:.1} | {:.1} | {:.1} | {} |", census::COMP_NAMES[comp], ns / calls / 1e3, al / calls, by / calls / 1024.0, lp / calls, pct(ns, solve_ns));
            }
            println!("| {cname} | rest (untimed code, timer cost) | {:.2} | {:.1} | {:.1} | - | {} |", (tot - sum) / calls / 1e3, (g(0, 2) - cen_al - sum_al) / calls, (g(0, 3) - sum_by) / calls / 1024.0, pct(tot - sum, solve_ns));
            let _ = laps;
        }
        if any {
            println!();
        }
    }
    println!("\n### High-dimension 3m: mplcp walk Chebyshev LPs against the parent piece (warm-start potential)\n");
    println!("| problem | walk LPs | polytope rows (child) | rows added: 0 / 1 / 2 / 3+ | cold pivots/LP | with a parent point | child rows the point violates (all / added) | point inside child | cold pivots on those | shifted-start pivots/LP | shifted us/LP | cold us/LP | same decision and radius |");
    println!("|---|---:|---:|---|---:|---:|---|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::ChebRows) > 0.0) {
        let n = c(r, Counter::PfCheb);
        let hp = c(r, Counter::ChebHasPt).max(1.0);
        let lpn = r.s.counters[census::c2_slot(census::Comp::Lp as usize, 1, 0)] as f64;
        println!(
            "| {} | {} | {:.1} (d1 {:.1}) | {} / {} / {} / {} | {:.1} | {} | {:.2} / {:.2} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {} |",
            r.problem, n, c(r, Counter::ChebRows) / n, c(r, Counter::ChebDims) / n, c(r, Counter::ChebAdd0), c(r, Counter::ChebAdd1), c(r, Counter::ChebAdd2), c(r, Counter::ChebAdd3p),
            c(r, Counter::ChebPivots) / n, pct(c(r, Counter::ChebHasPt), n), c(r, Counter::ChebViolAll) / hp, c(r, Counter::ChebViolNew) / hp, pct(c(r, Counter::ChebPtInside), c(r, Counter::ChebHasPt)),
            c(r, Counter::ChebPivotsPt) / hp, c(r, Counter::ChebShiftPivots) / c(r, Counter::ChebShiftCalls).max(1.0), c(r, Counter::ChebShiftNs) / c(r, Counter::ChebShiftCalls).max(1.0) / 1e3, lpn / n / 1e3,
            pct(c(r, Counter::ChebShiftSame), c(r, Counter::ChebShiftCalls))
        );
    }
    println!("\n### High-dimension 3n: mplcp walk calls by class of vanishing rows (time without census work)\n");
    println!("| problem | class | walk calls | share of calls | us/call | share of solve | LP share of class time | DAQP share | Gamma share |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::AdjCase2) > 0.0) {
        let tot: f64 = (0..census::NWK).map(|k| r.s.counters[census::WK_BASE + k * 5] as f64).sum();
        for k in 0..census::NWK {
            let g = |f: usize| r.s.counters[census::WK_BASE + k * 5 + f] as f64;
            if g(0) == 0.0 {
                continue;
            }
            println!("| {} | {} | {} | {} | {:.0} | {} | {} | {} | {} |", r.problem, census::WK_NAMES[k], g(0), pct(g(0), tot), g(1) / g(0) / 1e3, pct(g(1), r.solve_s * 1e9), pct(g(2), g(1)), pct(g(3), g(1)), pct(g(4), g(1)));
        }
    }
    println!("\n### High-dimension 3o: walk Chebyshev LPs, leaf against non-leaf, and the phase-1 feasibility LP for non-leaf branches\n");
    println!("| problem | leaf LPs | leaf us/LP | leaf pivots/LP | non-leaf LPs | non-leaf us/LP | non-leaf pivots/LP | non-leaf share of LP time | phase-1 pivots/LP | phase-1 us/LP | phase-1 same verdict |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::ChebRows) > 0.0) {
        let (lc, nc) = (c(r, Counter::LpLeafCalls), c(r, Counter::LpNlCalls));
        println!(
            "| {} | {} | {:.1} | {:.1} | {} | {:.1} | {:.1} | {} | {:.1} | {:.1} | {} |",
            r.problem, lc, c(r, Counter::LpLeafNs) / lc.max(1.0) / 1e3, c(r, Counter::LpLeafPiv) / lc.max(1.0), nc, c(r, Counter::LpNlNs) / nc.max(1.0) / 1e3, c(r, Counter::LpNlPiv) / nc.max(1.0),
            pct(c(r, Counter::LpNlNs), c(r, Counter::LpNlNs) + c(r, Counter::LpLeafNs)), c(r, Counter::P1Pivots) / c(r, Counter::P1Calls).max(1.0), c(r, Counter::P1Ns) / c(r, Counter::P1Calls).max(1.0) / 1e3,
            pct(c(r, Counter::P1Same), c(r, Counter::P1Calls))
        );
    }
    println!("\n### High-dimension 3q: walk Chebyshev LPs by outcome, cold against shifted start from the parent point and against the phase-1 feasibility LP\n");
    println!("| problem | outcome | LPs | share of LPs | share of LP time | cold pivots/LP | cold us/LP | shifted pivots/LP | shifted us/LP | phase-1 pivots/LP | phase-1 us/LP |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::ChebRows) > 0.0) {
        let g = |o: usize, f: usize| r.s.counters[census::EO_BASE + o * 8 + f] as f64;
        let (tc, tt) = (g(0, 0) + g(1, 0), g(0, 2) + g(1, 2));
        for (o, nm) in [(0usize, "empty"), (1, "nonempty")] {
            let n = g(o, 0).max(1.0);
            let sn = g(o, 3).max(1.0);
            println!(
                "| {} | {} | {} | {} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} |",
                r.problem, nm, g(o, 0), pct(g(o, 0), tc), pct(g(o, 2), tt), g(o, 1) / n, g(o, 2) / n / 1e3, g(o, 4) / sn, g(o, 5) / sn / 1e3, g(o, 6) / n, g(o, 7) / n / 1e3
            );
        }
    }
    println!("\n### High-dimension 3p: region-box prefilter on the pieces that reach the DAQP / LP stage, exact row redundancy, repeated hyperplanes, DAQP cost split\n");
    println!("| problem | tests | box proves empty | of those: DAQP empty / LP empty / LP nonempty (must be 0) | rows/piece | rows redundant over the box | cut rows/piece | redundant cut rows | non-redundant rows (exact, sample of walk LPs) | walks | walks on a hyperplane walked before | from the other side | share of walk time in repeats | DAQP tests | build us | solve us | certificate us |");
    println!("|---|---:|---:|---|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::ChebRows) > 0.0) {
        let t = c(r, Counter::BoxTests).max(1.0);
        let dq = c(r, Counter::DqTests).max(1.0);
        println!(
            "| {} | {} | {} | {} / {} / {} | {:.1} | {:.1} | {:.2} | {:.2} | {:.1} of {:.1} | {} | {} | {} | {} | {} | {:.2} | {:.2} | {:.2} |",
            r.problem, c(r, Counter::BoxTests), pct(c(r, Counter::BoxEmpty), t), c(r, Counter::BoxEmptyDaqp), c(r, Counter::BoxEmptyLp), c(r, Counter::BoxEmptyNonempty),
            c(r, Counter::BoxRows) / t, c(r, Counter::BoxRedRows) / t, c(r, Counter::BoxCutRows) / t, c(r, Counter::BoxRedCut) / t,
            c(r, Counter::RedNonRed) / c(r, Counter::RedSamples).max(1.0), c(r, Counter::RedRows) / c(r, Counter::RedSamples).max(1.0),
            c(r, Counter::HyperWalks), pct(c(r, Counter::HyperRepeat), c(r, Counter::HyperWalks)), pct(c(r, Counter::HyperOther), c(r, Counter::HyperWalks)), pct(c(r, Counter::HyperRepeatNs), c(r, Counter::HyperNs)),
            c(r, Counter::DqTests), c(r, Counter::DqBuildNs) / dq / 1e3, c(r, Counter::DqSolveNs) / dq / 1e3, c(r, Counter::DqCertNs) / dq / 1e3
        );
    }
    println!("\n### High-dimension 3s: rank-1 multipliers under the scale-free rule (nu = |mu_j| sqrt(M_jj / M_cc), zero below 1e-9, nonzero above 1e-6)\n");
    println!("| problem | Case-2 calls | answered by the enumeration | handed to the walk | members the old guard refused, now in P- | refused by the old guard, now zero | tiny (<= PTOL) now in P- | in the gap | mismatch refusals now passed |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::AdjCase2) > 0.0) {
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            r.problem, c(r, Counter::AdjCase2), c(r, Counter::R1Handled), c(r, Counter::R1Fallback), c(r, Counter::MuAccepted), c(r, Counter::MuNoiseZero), c(r, Counter::MuNewPositive), c(r, Counter::MuGap), c(r, Counter::MuMismatchFreed)
        );
    }
    println!("\n### High-dimension 3j: mplcp tableau rebuilds refused\n");
    println!("| problem | regions | `tableau_for` refusals | of which pivot / M_ii above 1e-6 | regions popped without a tableau (neighbour search skipped) | `candidate_law`: fresh tableau refused | `candidate_law`: one-pivot step below 1e-9| bases accepted in the second pivot order | tableaux rebuilt for popped children (tableau budget) | entries set to zero as rounding noise |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp") {
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            r.problem, r.regions, c(r, Counter::TabRefuse), c(r, Counter::TabRefuseRel), c(r, Counter::ResolveNone), c(r, Counter::CandLawNoTab), c(r, Counter::CandLawSmallPiv), c(r, Counter::TabRescued), c(r, Counter::SnapRebuilds), c(r, Counter::SnapEntries)
        );
    }
    println!("\n### High-dimension 3t: gap-band multipliers recomputed from the original data (Gram solve plus 2 refinement steps)\n");
    println!("| problem | gap-band calls | entries in the band | refined nu <= 1e-12 (noise) | refined nu >= 1e-6 (real) | still between | calls with every entry decided | us per recomputation | large entries compared | disagreeing | tableau zeros not zero when refined | refined nu decade of the band entries (1e0 ... 1e-14, below) |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && r.s.counters[census::MU2_BASE] > 0) {
        let g = |k: usize| r.s.counters[census::MU2_BASE + k] as f64;
        let h: Vec<String> = (0..16).map(|d| format!("{}", r.s.counters[census::MU2_BASE + 9 + d])).collect();
        println!(
            "| {} | {} | {} | {} | {} | {} | {} ({}) | {:.1} | {} | {} | {} | {} |",
            r.problem, g(0), g(1), g(2), g(3), g(4), g(5), pct(g(5), g(0)), g(6) / g(0).max(1.0) / 1e3, g(7), g(8), g(25), h.join(" / ")
        );
    }
    println!("\n### High-dimension 3u: refined multipliers in the enumeration, and the audit against the walk\n");
    println!("| problem | band entries decided zero | decided real | still between (walk) | audited calls | same neighbour sets | walk has extra sets, all linearly dependent (no region) | walk has other extra sets, no real positive multiplier (walk wandered on a noise multiplier) | walk has other extra sets, a real positive multiplier exists | enumeration has a set the walk lacks | walk incomplete | enumeration undecided (back to the walk) | P- empty with a positive multiplier decided zero | of those handed to the walk | walk calls left |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::AdjCase2) > 0.0) {
        println!("| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |", r.problem, c(r, Counter::MuRefinedZero), c(r, Counter::MuRefinedReal), c(r, Counter::MuRefinedBand), c(r, Counter::AuditCalls), c(r, Counter::AuditSame), c(r, Counter::AuditExplained), c(r, Counter::AuditNoPositive), c(r, Counter::AuditExtraWalk), c(r, Counter::AuditDiff), c(r, Counter::AuditSkip), c(r, Counter::AuditAltNone), c(r, Counter::MuEmptyGuard), c(r, Counter::MuEmptyToWalk), c(r, Counter::R1Fallback));
    }
    println!("\n### High-dimension 4: vertex clipping (`clip_flags`) outcomes per call\n");
    println!("| problem | algorithm | clip calls/region | ok | nt outside 2..=6 | more than 128 hyperplanes | infinite box | more than 600 vertices | too few vertices | vertex violates a row | redundancy LPs/region |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| c(r, Counter::ClipCalls) > 0.0 || r.alg != "combinatorial") {
        let (n, k) = (r.regions.max(1) as f64, c(r, Counter::ClipCalls));
        if k == 0.0 && r.s.calls[0][0] == 0 {
            continue;
        }
        println!(
            "| {} | {} | {:.2} | {} | {} | {} | {} | {} | {} | {} | {:.1} |",
            r.problem, r.alg, k / n, pct(c(r, Counter::ClipOk), k), pct(c(r, Counter::ClipNtOut), k), pct(c(r, Counter::ClipManyHyp), k), pct(c(r, Counter::ClipInfBox), k),
            pct(c(r, Counter::ClipManyVerts), k), pct(c(r, Counter::ClipFewVerts), k), pct(c(r, Counter::ClipViolate), k), r.s.calls[0][0] as f64 / n
        );
    }
    println!("\n### High-dimension 5: candidates and facet steps per region\n");
    println!("| problem | algorithm | regions | candidates tried/region | duplicates/region | neighbour LP+QP/region | feasibility LP/region |");
    println!("|---|---|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| matches!(r.alg.as_str(), "graph" | "combinatorial_graph" | "geometric" | "mplcp")) {
        let n = r.regions.max(1) as f64;
        println!(
            "| {} | {} | {} | {} | {:.1} | {:.1} | {:.1} |",
            r.problem, r.alg, r.regions, per(c(r, Counter::CandGen), n), c(r, Counter::Dup) / n, (r.s.calls[0][2] + r.s.calls[1][2]) as f64 / n, r.s.calls[0][1] as f64 / n
        );
    }
    build_study(rows);
    for (p, a) in skipped {
        println!("\n(skipped: {p} {a})");
    }
}

/// mplcp region-build study (needs `RPOPT_DIAG=1`; see `src/census_diag.rs`).
fn build_study(rows: &[Row]) {
    use rpopt::census_diag::{s as d, slot};
    let rs: Vec<&Row> = rows.iter().filter(|r| r.alg == "mplcp" && slot(&r.s, d::REG) > 0.0).collect();
    if rs.is_empty() {
        return;
    }
    let g = |r: &Row, i: usize| slot(&r.s, i);
    let pc = |a: f64, b: f64| if b > 0.0 { format!("{:.1}%", 100.0 * a / b) } else { "-".into() };
    println!("\n### Build study 1: time split of the mplcp region build (diagnostic time removed)\n");
    println!("| problem | regions | solve ms | build share | law | rows (KKT + normalise) | Chebyshev stage | gen_cr_from_prepared | of which box LPs | per-row LPs | tab rows (diag) us/region | KKT rows us/region |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rs {
        let tot = r.solve_s * 1e9 - g(r, d::DIAG_NS);
        let dg = g(r, d::DIAG_NS);
        let n = r.regions.max(1) as f64;
        let nano = |p: usize| (r.s.nanos[0][p] + r.s.nanos[1][p]) as f64;
        println!(
            "| {} | {} | {:.1} | {} | {} | {} | {} | {} | {} | {} | {:.2} | {:.2} |",
            r.problem, r.regions, tot / 1e6, pc(g(r, d::BUILD_NS) - dg, tot), pc(g(r, d::LAW_NS), tot), pc(g(r, d::ROWS_NS), tot), pc(g(r, d::CHEB_NS), tot), pc(g(r, d::RED_NS) - dg, tot),
            pc(nano(4), tot), pc(nano(0), tot), g(r, d::TAB_ROWS_NS) / 1e3 / n, g(r, d::ROWS_NS) / 1e3 / n
        );
    }
    println!("\n### Build study 2: what decides each row (per region), and the entry facet\n");
    println!("| problem | rows | facets | propagated box | exact box | box vertex | projection | LP kept | LP dropped | children with entry | entry located | entry by box vertex | by projection | by LP (saved LP) |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rs {
        let n = g(r, d::REG);
        let ps = |i: usize| format!("{:.1}", g(r, i) / n);
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            r.problem, ps(d::ROWS), ps(d::FACETS), ps(d::PROP_REJ), ps(d::BOX_REJ), ps(d::VERT_KEEP), ps(d::PROJ_KEEP), ps(d::LP_KEEP), ps(d::LP_DROP),
            pc(g(r, d::HAS_ENTRY), n), pc(g(r, d::ENTRY_FOUND), g(r, d::HAS_ENTRY)), pc(g(r, d::ENTRY_VERT), g(r, d::ENTRY_FOUND)), pc(g(r, d::ENTRY_PROJ), g(r, d::ENTRY_FOUND)), pc(g(r, d::ENTRY_LPKEEP), g(r, d::ENTRY_FOUND))
        );
    }
    println!("\n### Build study 3: parent verdicts as hints\n");
    println!("| problem | children with parent | rows with counterpart | agree | parent kept, child kept | parent kept, child dropped | parent dropped, child kept | parent dropped, child dropped | rows at LP stage with counterpart (per region) | agree | LP-dropped where parent dropped | LP-kept where parent kept |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rs {
        let n = g(r, d::REG);
        let cr = g(r, d::CMP_ROWS);
        println!(
            "| {} | {} | {:.1} | {} | {} | {} | {} | {} | {:.2} | {} | {} | {} |",
            r.problem, pc(g(r, d::HAS_PARENT), n), cr / g(r, d::HAS_PARENT).max(1.0), pc(g(r, d::PK_CK) + g(r, d::PD_CD), cr), pc(g(r, d::PK_CK), cr), pc(g(r, d::PK_CD), cr), pc(g(r, d::PD_CK), cr), pc(g(r, d::PD_CD), cr),
            g(r, d::LPS_CMP) / g(r, d::HAS_PARENT).max(1.0), pc(g(r, d::LPS_AGREE), g(r, d::LPS_CMP)), pc(g(r, d::LPS_DROP_PD), g(r, d::LP_DROP)), pc(g(r, d::LPS_KEEP_PK), g(r, d::LP_KEEP))
        );
    }
    println!("\n### Build study 4: the bounding box (child against parent's exact box, and rows an outer box decides)\n");
    println!("| problem | box bounds equal to parent's | parent box points feasible in child | of which optimal for the child | vertex-kept rows hit by a feasible parent point | propagated box (6 rounds) rejects /region | 30 rounds rejects /region | exact box rejects /region |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rs {
        let n = g(r, d::REG);
        println!(
            "| {} | {} | {} | {} | {} | {:.1} | {:.1} | {:.1} |",
            r.problem, pc(g(r, d::BOX_EQ_PARENT), g(r, d::BOX_COORDS)), pc(g(r, d::PT_FEAS), g(r, d::PT_TOTAL)), pc(g(r, d::PT_OPT), g(r, d::PT_TOTAL)), pc(g(r, d::VK_COVERED), g(r, d::VK_ROWS)),
            g(r, d::PROP_REJ) / n, g(r, d::PROP30_REJ) / n, g(r, d::BOX_REJ) / n
        );
    }
    println!("\n### Build study 5: tableau rows against KKT rows, hints, and a Clarkson-style redundancy removal (per region)\n");
    println!("| problem | regions compared | max row difference above 1e-10 | 1e-8 | 1e-6 | 1e-4 | rows missing | hint margin > 0 | > 1e-9 | > 1e-6 | > 1e-4 |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rs {
        let t = g(r, d::TABC_REG);
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            r.problem, t, pc(g(r, d::TABC_T0 + 1), t), pc(g(r, d::TABC_T0 + 2), t), pc(g(r, d::TABC_T0 + 3), t), pc(g(r, d::TABC_T0 + 4), t), pc(g(r, d::TABC_MISSING), g(r, d::TABC_ROWS)),
            pc(g(r, d::HINT_M0), g(r, d::HINT_N)), pc(g(r, d::HINT_M1), g(r, d::HINT_N)), pc(g(r, d::HINT_M2), g(r, d::HINT_N)), pc(g(r, d::HINT_M3), g(r, d::HINT_N))
        );
    }
    println!("\n### Build study 6: Clarkson-style removal, per region, against the present LPs (A seed = parameter-set rows, B = A + entry + parent's kept rows)\n");
    println!("| problem | present: box LPs | per-row LPs | Chebyshev | present LP rows (live) | A: LPs | avg LP rows | rays | decided by ray alone | LP time us | ray time us | final S | verdict mismatches | B: LPs | avg LP rows | rays | LP time us | final S | mismatches | present LP time us (box+row+Cheb) |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rs {
        let n = g(r, d::REG);
        let nano = |p: usize| (r.s.nanos[0][p] + r.s.nanos[1][p]) as f64;
        let a = d::SIM_A;
        let b = d::SIM_B;
        let live = (g(r, d::ROWS) - g(r, d::PROP_REJ)) / n;
        println!(
            "| {} | {:.1} | {:.1} | 1 | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {} | {:.1} |",
            r.problem, 2.0 * 0.0 + (r.s.calls[0][4] + r.s.calls[1][4]) as f64 / n, (g(r, d::LP_KEEP) + g(r, d::LP_DROP)) / n, live,
            g(r, a + d::O_LPS) / n, g(r, a + d::O_LPROWS) / g(r, a + d::O_LPS).max(1.0), g(r, a + d::O_RAYS) / n, g(r, a + d::O_RAYPROOF) / n, g(r, a + d::O_LP_NS) / 1e3 / n, g(r, a + d::O_RAY_NS) / 1e3 / n, g(r, a + d::O_SFINAL) / n, g(r, a + d::O_MISMATCH),
            g(r, b + d::O_LPS) / n, g(r, b + d::O_LPROWS) / g(r, b + d::O_LPS).max(1.0), g(r, b + d::O_RAYS) / n, g(r, b + d::O_LP_NS) / 1e3 / n, g(r, b + d::O_SFINAL) / n, g(r, b + d::O_MISMATCH),
            (nano(4) + nano(0) + nano(3)) / 1e3 / n
        );
    println!("\n### Build study 7: Clarkson-style removal with a ray phase first (rays along every row normal and +-e_k, then LPs for the rest), per region\n");
    println!("| problem | LPs | avg LP rows | rays | LP time us | ray time us | final S | mismatches |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rs {
        let n = g(r, d::REG);
        let cc = d::SIM_C;
        println!("| {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {} |", r.problem, g(r, cc + d::O_LPS) / n, g(r, cc + d::O_LPROWS) / g(r, cc + d::O_LPS).max(1.0), g(r, cc + d::O_RAYS) / n, g(r, cc + d::O_LP_NS) / 1e3 / n, g(r, cc + d::O_RAY_NS) / 1e3 / n, g(r, cc + d::O_SFINAL) / n, g(r, cc + d::O_MISMATCH));
    }
    println!("\n### Build study 8: pivots and time of the 2n box LPs, cold (engine, from the centre) against a warm chain (own primal simplex, one shared vertex)\n");
    println!("| problem | n | engine pivots/LP | engine us/LP | own cold steps/LP | own cold us/LP | warm order 1 steps/LP | us/LP | warm order 2 steps/LP | us/LP | bounds differing from engine: own cold | warm 1 | warm 2 | rows tight at a box point | tight-set differences: warm 1 | warm 2 | failures/region |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rs {
        let lp = g(r, d::ENG_LPS).max(1.0);
        let bb = g(r, d::BOX_BOUNDS).max(1.0);
        let reg = g(r, d::BOX_REGIONS).max(1.0);
        println!(
            "| {} | {:.0} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {} | {} | {} | {:.1} | {:.2} | {:.2} | {:.3} |",
            r.problem, bb / reg / 2.0, g(r, d::ENG_PIV) / lp, g(r, d::ENG_NS) / 1e3 / lp, g(r, d::COLD_STEPS) / bb, g(r, d::COLD_NS) / 1e3 / bb,
            g(r, d::WARM1_STEPS) / bb, g(r, d::WARM1_NS) / 1e3 / bb, g(r, d::WARM2_STEPS) / bb, g(r, d::WARM2_NS) / 1e3 / bb,
            pc(g(r, d::COLD_VALDIFF), bb), pc(g(r, d::WARM1_VALDIFF), bb), pc(g(r, d::WARM2_VALDIFF), bb),
            g(r, d::TIGHT_ROWS_COLD) / reg, g(r, d::TIGHT_DIFF_W1) / reg, g(r, d::TIGHT_DIFF_W2) / reg, g(r, d::WARM_FAIL) / reg
        );
    }
    println!("\n### Build study 9: the engine's box LPs chained (each starts at the previous optimum), per box LP\n");
    println!("| problem | cold pivots | cold us | chain order 2, vertex start: pivots | us | speed-up | bounds differing | tight-set diffs/region | chain order 2, start pushed 1e-4: pivots | us | speed-up | bounds differing | tight-set diffs/region | chain order 1, vertex start: pivots | us | speed-up | failures/region |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rs {
        let lp = g(r, d::ENG_LPS).max(1.0);
        let bb = g(r, d::BOX_BOUNDS).max(1.0);
        let reg = g(r, d::BOX_REGIONS).max(1.0);
        let cold = g(r, d::ENG_NS) / lp;
        let sp = |slot: usize| format!("{:.2}x", cold / (g(r, slot) / bb).max(1e-9));
        println!(
            "| {} | {:.2} | {:.2} | {:.2} | {:.2} | {} | {} | {:.2} | {:.2} | {:.2} | {} | {} | {:.2} | {:.2} | {:.2} | {} | {:.3} |",
            r.problem, g(r, d::ENG_PIV) / lp, cold / 1e3,
            g(r, d::E_V2_PIV) / bb, g(r, d::E_V2_NS) / 1e3 / bb, sp(d::E_V2_NS), pc(g(r, d::E_V2_VALDIFF), bb), g(r, d::E_V2_TD) / reg,
            g(r, d::E_P2_PIV) / bb, g(r, d::E_P2_NS) / 1e3 / bb, sp(d::E_P2_NS), pc(g(r, d::E_P2_VALDIFF), bb), g(r, d::E_P2_TD) / reg,
            g(r, d::E_V1_PIV) / bb, g(r, d::E_V1_NS) / 1e3 / bb, sp(d::E_V1_NS), g(r, d::E_FAIL) / reg
        );
    }
    println!("\n### Build study 10: `boxlp::box_chain` (first LP by the engine, the rest by a verified rank-one primal simplex) against the engine's 2n cold box LPs\n");
    println!("| problem | engine us/region | chain us/region | speed-up | steps/LP | engine LPs/region | fallbacks/region | bounds differing | tight-set diffs/region | None/region |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rs {
        let bb = g(r, d::BOX_BOUNDS).max(1.0);
        let reg = g(r, d::BOX_REGIONS).max(1.0);
        let eng = g(r, d::ENG_NS) / reg;
        let ch = g(r, d::TC_NS) / reg;
        println!(
            "| {} | {:.1} | {:.1} | {:.2}x | {:.2} | {:.2} | {:.3} | {} | {:.2} | {:.3} |",
            r.problem, eng / 1e3, ch / 1e3, eng / ch.max(1e-9), g(r, d::TC_STEPS) / (bb - g(r, d::TC_ENGINE_LPS)).max(1.0), g(r, d::TC_ENGINE_LPS) / reg, g(r, d::TC_FALLBACK) / reg,
            pc(g(r, d::TC_VALDIFF), bb), g(r, d::TC_TD) / reg, g(r, d::TC_NONE) / reg
        );
    }
    println!("\n### Build study 11: box LPs started at a parent vertex on the shared (entry) facet\n");
    println!("| problem | children with entry and parent box | parent box points tight on the entry facet (per child) | of which feasible in the child | children with a shared vertex | engine cold us/region | chain from engine LP us | speed-up | chain from shared vertex us | speed-up | engine LPs/region (chain / shared) | steps/LP (shared) | fallbacks/region | bounds differing | tight-set diffs/region | rows kept by projection or LP | of which tight at a shared vertex | bytes per queued child (shared points) |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rs {
        let nn = g(r, d::SH_N).max(1.0);
        let any = g(r, d::SH_ANY).max(1.0);
        let bb = (g(r, d::BOX_BOUNDS) / g(r, d::BOX_REGIONS).max(1.0)).max(1.0);
        let eng = g(r, d::SH_ENG_NS) / any;
        let ch = g(r, d::SH_CH_NS) / any;
        let sv = g(r, d::SH_SV_NS) / any;
        let nvar = bb / 2.0;
        println!(
            "| {} | {} | {:.2} | {:.2} | {} | {:.1} | {:.1} | {:.2}x | {:.1} | {:.2}x | {:.2} / {:.2} | {:.2} | {:.3} | {} | {:.2} | {:.1} | {} | {:.0} |",
            r.problem, g(r, d::SH_N), g(r, d::SH_PTS_TIGHT) / nn, g(r, d::SH_PTS_FEAS) / nn, pc(g(r, d::SH_ANY), g(r, d::SH_N)), eng / 1e3, ch / 1e3, eng / ch.max(1e-9), sv / 1e3, eng / sv.max(1e-9),
            g(r, d::SH_CH_ENGLP) / any, g(r, d::SH_SV_ENGLP) / any, g(r, d::SH_SV_STEPS) / (any * bb).max(1.0), g(r, d::SH_SV_FB) / any, pc(g(r, d::SH_SV_VALDIFF), any * bb), g(r, d::SH_SV_TD) / any,
            g(r, d::SH_LPKEEP_ROWS) / any, pc(g(r, d::SH_SETTLED), g(r, d::SH_LPKEEP_ROWS)), g(r, d::SH_PTS_FEAS) / nn * nvar * 8.0 + 24.0
        );
    }
    println!("\n### Build study 12: small Farkas certificates for the redundant rows (share caught, by how the row is decided today)\n");
    println!("| problem | variant | rows decided by the propagated box (per region) | caught | rows decided by the exact box (per region) | caught | rows decided by a per-row LP (per region) | caught | us per test |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|");
    let names = ["d nearest kept rows (square solve)", "2d nearest kept rows (LP)", "parent's kept rows (LP)", "all kept rows (LP)"];
    for r in &rs {
        let reg = g(r, d::REG).max(1.0);
        for (v, nm) in names.iter().enumerate() {
            let b = d::FK_BASE + v * d::FK_STRIDE;
            println!(
                "| {} | {} | {:.1} | {} | {:.1} | {} | {:.1} | {} | {:.2} |",
                r.problem, nm, g(r, b) / reg, pc(g(r, b + 1), g(r, b)), g(r, b + 2) / reg, pc(g(r, b + 3), g(r, b + 2)), g(r, b + 4) / reg, pc(g(r, b + 5), g(r, b + 4)), g(r, b + 6) / 1e3 / g(r, b + 7).max(1.0)
            );
        }
    }
    println!("\n### Build study 13: ridge rule on the shared-facet vertices (parent box vertices on the entry facet)\n");
    println!("| problem | children with shared vertices | shared vertices per child | children with fewer than d - 1 | facets (not the entry) per child | rows with >= d - 1 affinely independent shared vertices tight | of which unique | unique and a facet | unique and NOT a facet | not unique | not unique and not a facet |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rs {
        let c = g(r, d::RG_CHILDREN).max(1.0);
        println!(
            "| {} | {} | {:.2} | {} | {:.1} | {:.2} | {:.2} | {:.2} | {} | {:.2} | {} |",
            r.problem, g(r, d::RG_CHILDREN), g(r, d::RG_PTS) / c, pc(g(r, d::RG_NOENOUGH), g(r, d::RG_CHILDREN)), g(r, d::RG_FACETS) / c, g(r, d::RG_RIDGE) / c, g(r, d::RG_UNIQUE) / c, g(r, d::RG_UNIQUE_FACET) / c,
            g(r, d::RG_UNIQUE_WRONG), g(r, d::RG_AMBIG) / c, g(r, d::RG_AMBIG_WRONG)
        );
    }
    }
}
