//! Checks active sets that one mplcp run has and another lacks. For each active set listed in
//! <extra.txt> (one `[i, j, ...]` per line): build its region, report whether its basis is
//! lexico-feasible, its Chebyshev radius, how many regions of the current mplcp solve contain its
//! Chebyshev center (strictly), and whether the QP at that center has this active set.
//!
//!   extra_regions <problem.json> <extra.txt>
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(&a[1])).unwrap());
    let sol = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
    println!("current mplcp: {} regions", sol.critical_regions.len());
    for line in std::fs::read_to_string(&a[2]).unwrap().lines() {
        let aset: Vec<usize> = line.trim_matches(|c| c == '[' || c == ']').split(',').filter_map(|t| t.trim().parse().ok()).collect();
        let Some(cr) = rpopt::mpqp_utils::gen_cr_from_active_set(&prog, &aset, true) else {
            println!("{line}: no full-dimensional region");
            continue;
        };
        let lexbad = rpopt::algorithms::mplcp::count_lex_infeasible(&prog, std::slice::from_ref(&cr));
        let nt = prog.num_t();
        let s = rpopt::utils::chebyshev_ball(&cr.e, &cr.f, &[]).unwrap();
        let center: Vec<f64> = s.x[..nt].to_vec();
        let inside: Vec<Vec<usize>> = sol
            .critical_regions
            .iter()
            .filter(|r| r.is_inside(&center, -1e-9))
            .map(|r| {
                let mut v = r.active_set.clone();
                v.sort_unstable();
                v
            })
            .collect();
        let qp = prog.solve_theta(&center).map(|o| {
            let mut v = o.active_set.clone();
            v.sort_unstable();
            v
        });
        println!("{line}: lex_infeasible={} radius={:.3e} contained_in_current={:?} qp_active_set_at_center={:?}", lexbad == 1, s.x[nt], inside, qp);
    }
}
