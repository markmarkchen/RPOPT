//! Writes random mpQPs (src/problem_generator.rs) with many parameters as JSON for the large benchmark.
//!
//! cargo run --release --example gen_random -- <out_dir> [--lp] [t:x:m:seed ...]
//! Without specs a default grid (theta dimension 6, 8, 10) is written. The JSON holds the processed
//! program (what `MpProgram::from_json_file` would build from it).

use rpopt::problem_generator::{generate_mplp, generate_mpqp};
use serde_json::json;

fn col(v: &[f64]) -> Vec<Vec<f64>> {
    v.iter().map(|&x| vec![x]).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = std::path::PathBuf::from(&args[1]);
    std::fs::create_dir_all(&out).unwrap();
    let lp = args.iter().any(|a| a == "--lp");
    let mut specs: Vec<(usize, usize, usize, u64)> = vec![];
    for s in args[2..].iter().filter(|a| *a != "--lp") {
        let p: Vec<u64> = s.split(':').map(|v| v.parse().unwrap()).collect();
        specs.push((p[0] as usize, p[1] as usize, p[2] as usize, p[3]));
    }
    if specs.is_empty() {
        for &(t, x, m) in &[(6, 3, 6), (6, 4, 8), (8, 3, 6), (8, 4, 8), (10, 3, 6), (10, 4, 8), (10, 5, 10)] {
            for seed in 0..3u64 {
                specs.push((t, x, m, seed));
            }
        }
    }
    for (t, x, m, seed) in specs {
        // `--lp`: a random mpLP (no "Q"), file name prefix `randlp`
        let p = if lp { generate_mplp(x, t, m, seed) } else { generate_mpqp(x, t, m, seed) };
        let mut doc = json!({
            "A": p.a.to_rows(), "b": col(&p.b), "c": col(&p.c), "H": p.h.to_rows(),
            "A_t": p.a_t.to_rows(), "b_t": col(&p.b_t), "F": p.f.to_rows(), "equality_indices": p.equality_indices,
        });
        if let Some(q) = p.q.as_ref() {
            doc["Q"] = json!(q.to_rows());
        }
        let name = format!("{}_t{t}_x{x}_m{m}_s{seed}", if lp { "randlp" } else { "rand" });
        std::fs::write(out.join(format!("{name}.json")), doc.to_string()).unwrap();
        println!("{name}: x={} t={} constraints={}", p.num_x(), p.num_t(), p.num_constraints());
    }
}
