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
    // RPOPT_CERT=0|1|2|3|4: mplcp facet certificate off (default), on (3 to 5 parameters), audit, on at 2 to 6, audit at 2 to 6
    if let Ok(m) = std::env::var("RPOPT_CERT") {
        rpopt::algorithms::mplcp::set_certificate(m.parse().unwrap());
    }
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
    println!("| problem | Case-2 calls | answered by the enumeration | handed to the walk | candidates emitted | members dropped as known | emitted on a witness | sub-facet LPs | members with an empty sub-facet | neighbour LPs/region | distinct active sets tested | repeated tests | of which empty |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows.iter().filter(|r| r.alg == "mplcp" && c(r, Counter::AdjCase2) > 0.0) {
        let n = r.regions.max(1) as f64;
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.1} | {} | {} | {} |",
            r.problem, c(r, Counter::AdjCase2), c(r, Counter::R1Handled), c(r, Counter::R1Fallback), c(r, Counter::R1Emitted), c(r, Counter::R1SkipSeen),
            c(r, Counter::R1WitnessAcc), c(r, Counter::R1LpRun), c(r, Counter::R1EmptyMember), c(r, Counter::PfCheb) / n,
            c(r, Counter::R1TestDistinct), c(r, Counter::R1TestRepeat), c(r, Counter::R1EmptyRepeat)
        );
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
    for (p, a) in skipped {
        println!("\n(skipped: {p} {a})");
    }
}
