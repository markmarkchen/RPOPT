//! Per problem: mplcp region count, number of lexico-infeasible regions, and a sampled coverage
//! check (gaps: feasible θ in no region; overlaps: θ strictly inside two regions).
//!
//!   [LEX_ALG=<algorithm>] lexcheck <samples> <problem.json>...
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Same hash as `region_hash` (active sets and facet rows rounded to 1e-9), so one solve gives hash and coverage.
fn region_hash(regs: &[rpopt::critical_region::CriticalRegion]) -> u64 {
    use std::hash::{Hash, Hasher};
    let scale = 1e9;
    let mut recs: Vec<(Vec<usize>, Vec<i64>)> = regs
        .iter()
        .map(|r| {
            let mut a = r.active_set.clone();
            a.sort_unstable();
            let mut v = vec![r.e.rows as i64];
            for k in 0..r.e.rows {
                v.extend(r.e.row(k).iter().map(|x| (x * scale).round() as i64));
                v.push((r.f[k] * scale).round() as i64);
            }
            (a, v)
        })
        .collect();
    recs.sort();
    let mut h = std::collections::hash_map::DefaultHasher::new();
    recs.hash(&mut h);
    h.finish()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let ns: usize = a[1].parse().unwrap();
    // RPOPT_BOXCHAIN=0: no chained bounding-box LPs in the mplcp LP-path builder (the shared builder instead)
    if let Ok(v) = std::env::var("RPOPT_BOXCHAIN") {
        rpopt::algorithms::mplcp::set_box_chain(v != "0");
    }
    // MPLCP_TAB_BUDGET=<bytes>: limit on live parent tableaux (default 256 MiB, 0 = none)
    if let Some(b) = std::env::var("MPLCP_TAB_BUDGET").ok().and_then(|v| v.parse().ok()) {
        rpopt::algorithms::mplcp::set_tab_budget(b);
    }
    // LEX_ALG: algorithm to solve with (default mplcp)
    let alg = MpqpAlgorithm::from_name(&std::env::var("LEX_ALG").unwrap_or_else(|_| "mplcp".into())).expect("algorithm");
    for f in &a[2..] {
        let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(f)).unwrap());
        let t0 = std::time::Instant::now();
        let sol = solve_mpqp(&prog, alg);
        let dt = t0.elapsed().as_secs_f64();
        let hash = region_hash(&sol.critical_regions);
        let lexbad = rpopt::algorithms::mplcp::count_lex_infeasible(&prog, &sol.critical_regions);
        let nt = prog.num_t();
        let (mut lo, mut hi) = (vec![-1.0; nt], vec![1.0; nt]);
        for k in 0..nt {
            let mut c = vec![0.0; nt];
            c[k] = 1.0;
            if let Some(s) = rpopt::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
                lo[k] = s.obj;
            }
            c[k] = -1.0;
            if let Some(s) = rpopt::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
                hi[k] = -s.obj;
            }
        }
        let mut rng = StdRng::seed_from_u64(17);
        let (mut feas, mut ov, mut gaps) = (0, 0, 0);
        for _ in 0..ns {
            let th: Vec<f64> = (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect();
            if prog.solve_theta(&th).is_none() {
                continue;
            }
            feas += 1;
            if sol.critical_regions.iter().filter(|r| r.is_inside(&th, -1e-7)).count() >= 2 {
                ov += 1;
            }
            if sol.get_region(&th).is_none() {
                gaps += 1;
            }
        }
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        // hash of the sorted active sets alone, and the number of facet rows: unchanged when only the last digits of the rows move
        let mut sets: Vec<Vec<usize>> = sol.critical_regions.iter().map(|r| { let mut a = r.active_set.clone(); a.sort_unstable(); a }).collect();
        sets.sort();
        let mut hs = std::collections::hash_map::DefaultHasher::new();
        sets.hash(&mut hs);
        let rows: usize = sol.critical_regions.iter().map(|r| r.e.rows).sum();
        println!("{name} regions {} lex_infeasible {lexbad} feasible_samples {feas} overlaps {ov} gaps {gaps} hash {hash:016x} sets {:016x} facet_rows {rows} solve_s {dt:.3}", sol.critical_regions.len(), hs.finish());
    }
}
