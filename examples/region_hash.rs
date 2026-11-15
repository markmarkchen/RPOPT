//! Prints, per problem, the region count and a hash of every region's active set and facet rows
//! (E, f rounded to 1e-9), sorted by active set. Used to check that a change to region building
//! leaves the regions identical. RPOPT_HASH_SCALE=1e6 hashes the rows coarser; `sets` hashes only the sorted active sets.
//!
//!   region_hash <algorithm> <problem.json>...
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // RPOPT_BOXCHAIN=0: no chained bounding-box LPs in the mplcp LP-path builder (the shared builder instead)
    if let Ok(v) = std::env::var("RPOPT_BOXCHAIN") {
        rpopt::algorithms::mplcp::set_box_chain(v != "0");
    }
    if let Ok(m) = std::env::var("RPOPT_CERT") {
        rpopt::algorithms::mplcp::set_certificate(m.parse().unwrap());
    }
    if let Ok(v) = std::env::var("MPLP_LEX_MODE") {
        rpopt::algorithms::mplp_lex::set_mode(v.parse().unwrap());
    }
    let alg = MpqpAlgorithm::from_name(&args[1]).expect("algorithm");
    // RPOPT_INDEX_FIRST=0|1|2: index-first lookup off, on (default), audit
    if let Ok(m) = std::env::var("RPOPT_INDEX_FIRST") {
        rpopt::algorithms::mplcp::set_index_first(m.parse().unwrap());
    }
    // MPLCP_NO_RANK1=1: the Case-2 walk only (the oracle of the region-set gates)
    if std::env::var("MPLCP_NO_RANK1").is_ok() {
        rpopt::algorithms::mplcp::set_rank1(false);
    }
    // MPLCP_REFINE=1: gap-band multipliers are recomputed from the original data (off by default)
    if std::env::var("MPLCP_REFINE").is_ok() {
        rpopt::algorithms::mplcp::set_refine_multipliers(true);
    }
    // MPLCP_NO_DEDUPE=1: solve with the duplicated constraints (no removal)
    if std::env::var("MPLCP_NO_DEDUPE").is_ok() {
        rpopt::algorithms::mplcp::set_dedupe(false);
    }
    // MPLCP_NO_DAQP=1: answer every sub-facet nonemptiness test with the LP
    if std::env::var("MPLCP_NO_DAQP").is_ok() {
        rpopt::algorithms::mplcp::set_daqp_tests(false);
    }
    // MPLCP_TAB_BUDGET=<bytes>: limit on live parent tableaux (default 256 MiB, 0 = none)
    if let Some(b) = std::env::var("MPLCP_TAB_BUDGET").ok().and_then(|v| v.parse().ok()) {
        rpopt::algorithms::mplcp::set_tab_budget(b);
    }
    // RPOPT_HASH_SCALE: rounding scale of E and f (default 1e9)
    let scale: f64 = std::env::var("RPOPT_HASH_SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(1e9);
    for f in &args[2..] {
        let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(f)).unwrap());
        // RPOPT_CANON=1: active sets are mapped to the lowest-index row of each group of duplicated constraints (so that the
        // solve with and without MPLCP_NO_DEDUPE compare); RPOPT_SETS_OUT=<file>: the sorted sets are also written there
        let rep = if std::env::var("RPOPT_CANON").is_ok() { Some(rpopt::algorithms::mplcp::duplicate_rows(&p)) } else { None };
        let t0 = std::time::Instant::now();
        let sol = solve_mpqp(&p, alg);
        let dt = t0.elapsed().as_secs_f64();
        if std::env::var("RPOPT_INDEX_FIRST").is_ok() {
            eprintln!("index_first facets_skipped/audit_checks/audit_violations/regions_skipped/regions_would_skip {:?}", rpopt::algorithms::mplcp::index_first_stats());
        }
        let mut recs: Vec<(Vec<usize>, Vec<i64>)> = sol
            .critical_regions
            .iter()
            .map(|r| {
                let mut a: Vec<usize> = match &rep {
                    Some(rep) => r.active_set.iter().map(|&i| rep[i]).collect(),
                    None => r.active_set.clone(),
                };
                a.sort_unstable();
                a.dedup();
                let mut v = vec![r.e.rows as i64];
                for k in 0..r.e.rows {
                    v.extend(r.e.row(k).iter().map(|x| (x * scale).round() as i64));
                    v.push((r.f[k] * scale).round() as i64);
                }
                (a, v)
            })
            .collect();
        recs.sort();
        if let Ok(path) = std::env::var("RPOPT_SETS_OUT") {
            let text: String = recs.iter().map(|r| format!("{:?}\n", r.0)).collect();
            std::fs::write(path, text).unwrap();
        }
        let rows: usize = recs.iter().map(|r| r.1[0] as usize).sum();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        recs.hash(&mut h);
        // hash of the sorted active sets alone: stays the same when a rebuilt tableau changes the last digits of the rows
        let mut hs = std::collections::hash_map::DefaultHasher::new();
        recs.iter().map(|r| &r.0).collect::<Vec<_>>().hash(&mut hs);
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        let (peak_parents, tab_bytes) = rpopt::algorithms::mplcp::tab_stats();
        let hwm_mb = std::fs::read_to_string("/proc/self/status").ok().and_then(|t| t.lines().find(|l| l.starts_with("VmHWM")).and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse::<f64>().ok()))).map_or(0.0, |kb| kb / 1024.0);
        let (seen_len, seen_bytes, qlen, qbytes) = rpopt::algorithms::mplcp::search_stats();
        println!("{name} seen_entries {seen_len} seen_mb {:.0} peak_queue {qlen} peak_queue_mb {:.0}", seen_bytes as f64 / 1048576.0, qbytes as f64 / 1048576.0);
        println!("{name} {} regions {} facet_rows {rows} hash {:016x} sets {:016x} solve_s {dt:.3} peak_parents {peak_parents} tab_bytes {tab_bytes} peak_parent_mb {:.0} rss_peak_mb {hwm_mb:.0}", args[1], recs.len(), h.finish(), hs.finish(), (peak_parents * tab_bytes) as f64 / 1048576.0);
    }
}
