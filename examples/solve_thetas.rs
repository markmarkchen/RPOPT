//! Direct QP solves of a JSON problem at given parameter values (used by bench/pop_verify.py).
//!
//! cargo run --release --example solve_thetas -- <problem.json> <thetas.json> <out.json>
//! thetas.json is a list of theta vectors; out.json a list of {"x": [...], "obj": f} or null (infeasible).

use rpopt::program::MpProgram;
use serde_json::{json, Value};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let prog = MpProgram::from_json_file(std::path::Path::new(&args[1])).unwrap();
    let thetas: Vec<Vec<f64>> = serde_json::from_str(&std::fs::read_to_string(&args[2]).unwrap()).unwrap();
    let out: Vec<Value> = thetas
        .iter()
        .map(|t| match prog.solve_theta(t) {
            Some(s) => json!({"x": s.sol, "obj": s.obj}),
            None => Value::Null,
        })
        .collect();
    std::fs::write(&args[3], serde_json::to_string(&out).unwrap()).unwrap();
}
