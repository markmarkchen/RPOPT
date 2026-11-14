//! Writes the processed problem (after PPOPT-style constraint processing) and the active sets found
//! by `parametric_daqp`, for the comparison with the authors' ParametricDAQP.jl (python `pdaqp`):
//!   cargo run --release --example pdaqp_dump -- <problem.json> <out.json>

use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(&args[1])).unwrap());
    let sol = solve_mpqp(&p, MpqpAlgorithm::ParametricDaqp);
    let sig = region_signature(&sol.critical_regions);
    let q = p.q.as_ref().unwrap().to_rows();
    let out = serde_json::json!({
        "Q": q, "c": p.c, "H": p.h.to_rows(), "A": p.a.to_rows(), "b": p.b, "F": p.f.to_rows(),
        "A_t": p.a_t.to_rows(), "b_t": p.b_t, "eq": p.equality_indices, "active_sets": sig,
    });
    std::fs::write(&args[2], serde_json::to_string(&out).unwrap()).unwrap();
}
