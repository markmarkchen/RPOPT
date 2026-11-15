//! Prints, per problem, the region count and a hash of every region's active set and facet rows
//! (E, f rounded to 1e-9), sorted by active set. Used to check that a change to region building
//! leaves the regions identical.
//!
//!   region_hash <algorithm> <problem.json>...
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Ok(m) = std::env::var("RPOPT_CERT") {
        rpopt::algorithms::mplcp::set_certificate(m.parse().unwrap());
    }
    if let Ok(v) = std::env::var("MPLP_LEX_MODE") {
        rpopt::algorithms::mplp_lex::set_mode(v.parse().unwrap());
    }
    let alg = MpqpAlgorithm::from_name(&args[1]).expect("algorithm");
    // RPOPT_HASH_SCALE: rounding scale of E and f (default 1e9)
    let scale: f64 = std::env::var("RPOPT_HASH_SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(1e9);
    for f in &args[2..] {
        let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(f)).unwrap());
        let t0 = std::time::Instant::now();
        let sol = solve_mpqp(&p, alg);
        let dt = t0.elapsed().as_secs_f64();
        let mut recs: Vec<(Vec<usize>, Vec<i64>)> = sol
            .critical_regions
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
        let rows: usize = recs.iter().map(|r| r.1[0] as usize).sum();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        recs.hash(&mut h);
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        println!("{name} {} regions {} facet_rows {rows} hash {:016x} solve_s {dt:.3}", args[1], recs.len(), h.finish());
    }
}
