//! Micro-benchmark of the KKT feasibility LPs of the combinatorial graph algorithm: collects the
//! (E, f) systems of supersets A + i of every region's active set, then times lp_feasible on them.
//!   cargo run --release --example lp_micro -- bench/problems/sys3x2_N5.json

use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).unwrap_or("bench/problems/sys3x2_N5.json".into());
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(&path)).unwrap());
    let sol = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
    let m = prog.num_constraints();
    let mut systems = vec![];
    for r in &sol.critical_regions {
        for i in 0..m {
            if r.active_set.contains(&i) {
                continue;
            }
            let mut a = r.active_set.clone();
            a.push(i);
            a.sort_unstable();
            if !prog.check_active_set_rank(&a) {
                continue;
            }
            if let Some(law) = prog.optimal_control_law_dual(&a) {
                let (e, f, _, _) = prog.region_rows(&a, &law);
                systems.push((e, f));
            }
        }
    }
    small_stats(&systems);
    let feasible = systems.iter().filter(|(e, f)| rpopt::solver::lp::lp_feasible(e, f, &[])).count();
    let reps = 5;
    let t0 = Instant::now();
    let mut cnt = 0;
    for _ in 0..reps {
        for (e, f) in &systems {
            cnt += rpopt::solver::lp::lp_feasible(e, f, &[]) as usize;
        }
    }
    let dt = t0.elapsed().as_secs_f64() / (reps * systems.len()) as f64;
    println!("{} LPs ({} feasible), rows {:.1} avg, {:.3} us per LP ({cnt})", systems.len(), feasible,
        systems.iter().map(|s| s.0.rows).sum::<usize>() as f64 / systems.len() as f64, dt * 1e6);
}

fn small_stats(systems: &[(rpopt::Mat, Vec<f64>)]) {
    use rpopt::solver::lp_small::{feasible, Small};
    let (mut f, mut i, mut u) = (0, 0, 0);
    for (e, b) in systems {
        match feasible(e, b) {
            Small::Feasible(_) => f += 1,
            Small::Infeasible => i += 1,
            Small::Undecided => u += 1,
        }
    }
    println!("fast path: feasible {f} infeasible {i} undecided {u}");
}
