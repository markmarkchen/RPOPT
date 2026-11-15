//! Builds the critical region of one active set and reports whether it exists: `gen_one <problem.json> i j k ...`
use rpopt::mpqp_utils::gen_cr_from_active_set_lambda_first;
use rpopt::program::MpProgram;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let p = MpProgram::from_json_file(std::path::Path::new(&args[1])).unwrap();
    let act: Vec<usize> = args[2..].iter().map(|s| s.parse().unwrap()).collect();
    println!("law: {}", p.optimal_control_law_dual(&act).is_some());
    match gen_cr_from_active_set_lambda_first(&p, &act, true) {
        Some(r) => println!("region rows {} radius {:?}", r.e.rows, r.cheb_radius),
        None => println!("gen_cr: None"),
    }
    match gen_cr_from_active_set_lambda_first(&p, &act, false) {
        Some(r) => {
            println!("without the full-dimension check: rows {} radius {:?}", r.e.rows, r.cheb_radius);
            let cold = rpopt::utils::chebyshev_ball(&r.e, &r.f, &[]);
            println!("cold Chebyshev LP on these rows: {:?}", cold.map(|b| b.x[b.x.len() - 1]));
            let feas = rpopt::solver::lp::lp_feasible_point(&r.e, &r.f, &[], None);
            println!("feasible point LP: {}", feas.is_some());
        }
        None => println!("without the full-dimension check: None"),
    }
}
