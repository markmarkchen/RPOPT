//! Prints the sorted active set of every region of an algorithm, one per line (for exact checks).
//!
//!   dump_bases <algorithm> <problem.json>      (DUMP_PROGRAM=1 first prints the processed program as JSON)
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(&args[2])).unwrap());
    if std::env::var("DUMP_PROGRAM").is_ok() {
        let doc = serde_json::json!({"A": p.a.to_rows(), "b": p.b, "c": p.c, "H": p.h.to_rows(), "A_t": p.a_t.to_rows(), "b_t": p.b_t, "F": p.f.to_rows()});
        println!("PROGRAM {doc}");
    }
    let sol = solve_mpqp(&p, MpqpAlgorithm::from_name(&args[1]).unwrap());
    for r in &sol.critical_regions {
        let mut a = r.active_set.clone();
        a.sort_unstable();
        println!("{}", a.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" "));
    }
}
