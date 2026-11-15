//! One mplcp solve; writes the sorted active sets, one `[i, j, ...]` per line, sorted.
//!   [AUDIT_RANK1=0] audit_solve <problem.json> <out.txt>
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if std::env::var("AUDIT_RANK1").map(|v| v == "0").unwrap_or(false) {
        rpopt::algorithms::mplcp::set_rank1(false);
    }
    let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(&a[1])).unwrap());
    let sol = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
    let mut lines: Vec<String> = sol
        .critical_regions
        .iter()
        .map(|r| {
            let mut s = r.active_set.clone();
            s.sort_unstable();
            format!("{s:?}")
        })
        .collect();
    if let Ok(path) = std::env::var("AUDIT_DUMP") {
        // set, Chebyshev radius and centre of the solver's own region (for sets that cannot be rebuilt from the active set)
        let nt = p.num_t();
        let mut reg: Vec<String> = sol
            .critical_regions
            .iter()
            .map(|r| {
                let mut s = r.active_set.clone();
                s.sort_unstable();
                match rpopt::utils::chebyshev_ball(&r.e, &r.f, &[]) {
                    Some(b) => format!("{s:?}\t{:e}\t{}", b.x[nt], b.x[..nt].iter().map(|v| format!("{v:e}")).collect::<Vec<_>>().join(",")),
                    None => format!("{s:?}\tNONE\t"),
                }
            })
            .collect();
        reg.sort();
        std::fs::write(path, reg.join("\n") + "\n").unwrap();
    }
    lines.sort();
    std::fs::write(&a[2], lines.join("\n") + "\n").unwrap();
    println!("{} regions", lines.len());
}
