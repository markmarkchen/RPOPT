//! Writes {problem: {algorithm: {"n": regions, "active_sets": [...sorted...]}}} for the Python binding tests.
//!
//! cargo run --release --example dump_reference -- <algorithm> out.json bench/problems/*.json

use rpopt::{solve_mpqp, MpProgram, MpqpAlgorithm};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let alg = MpqpAlgorithm::from_name(&args[0]).expect("algorithm");
    let mut out = serde_json::Map::new();
    for path in &args[2..] {
        let p = std::path::Path::new(path);
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        let prog = Arc::new(MpProgram::from_json_file(p).unwrap());
        let sol = solve_mpqp(&prog, alg);
        let mut sets: Vec<Vec<usize>> = sol.critical_regions.iter().map(|r| r.active_set.clone()).collect();
        sets.sort();
        out.insert(name, serde_json::json!({ "n": sol.len(), "active_sets": sets }));
    }
    std::fs::write(&args[1], serde_json::to_string(&out).unwrap()).unwrap();
}
