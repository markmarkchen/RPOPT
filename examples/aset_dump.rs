//! Writes the sorted active sets of a solve, one per line, for diffing two solves.
//!
//!   aset_dump <algorithm> <problem.json> <out.txt>
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if let Ok(m) = std::env::var("RPOPT_CERT") {
        rpopt::algorithms::mplcp::set_certificate(m.parse().unwrap());
    }
    if std::env::var("MPLCP_NO_RANK1").is_ok() {
        rpopt::algorithms::mplcp::set_rank1(false);
    }
    let alg = MpqpAlgorithm::from_name(&a[1]).expect("algorithm");
    let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(&a[2])).unwrap());
    let sol = solve_mpqp(&p, alg);
    let mut lines: Vec<String> = sol
        .critical_regions
        .iter()
        .map(|r| {
            let mut s = r.active_set.clone();
            s.sort_unstable();
            format!("{s:?} rows {}", r.e.rows)
        })
        .collect();
    lines.sort();
    std::fs::write(&a[3], lines.join("\n") + "\n").unwrap();
    println!("{} {} regions", a[1], lines.len());
}
