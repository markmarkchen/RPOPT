//! Lists the overlapping region pairs that sampling finds: for each distinct pair, the active
//! sets, whether each basis is lexico-feasible, the Chebyshev radius, whether the two laws agree
//! at the sample, and the active set of a direct QP solve there.
//!
//!   overlap_pairs <samples> <problem.json> [max pairs]
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::collections::BTreeSet;
use std::sync::Arc;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let ns: usize = a[1].parse().unwrap();
    let maxp: usize = a.get(3).map_or(8, |s| s.parse().unwrap());
    if let Ok(m) = std::env::var("RPOPT_CERT") {
        rpopt::algorithms::mplcp::set_certificate(m.parse().unwrap());
    }
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(&a[2])).unwrap());
    let sol = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
    let nt = prog.num_t();
    let (mut lo, mut hi) = (vec![-1.0; nt], vec![1.0; nt]);
    for k in 0..nt {
        let mut c = vec![0.0; nt];
        c[k] = 1.0;
        if let Some(s) = rpopt::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
            lo[k] = s.obj;
        }
        c[k] = -1.0;
        if let Some(s) = rpopt::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
            hi[k] = -s.obj;
        }
    }
    let mut rng = StdRng::seed_from_u64(17);
    let mut seen: BTreeSet<(usize, usize)> = BTreeSet::new();
    let (mut samples_ov, mut agree, mut differ) = (0, 0, 0);
    for _ in 0..ns {
        let th: Vec<f64> = (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect();
        if prog.solve_theta(&th).is_none() {
            continue;
        }
        let ins: Vec<usize> = (0..sol.critical_regions.len()).filter(|&i| sol.critical_regions[i].is_inside(&th, -1e-7)).collect();
        if ins.len() < 2 {
            continue;
        }
        samples_ov += 1;
        let (i, j) = (ins[0], ins[1]);
        let (ri, rj) = (&sol.critical_regions[i], &sol.critical_regions[j]);
        let (xi, xj) = (ri.evaluate(&th), rj.evaluate(&th));
        let same = xi.iter().zip(&xj).all(|(p, q)| (p - q).abs() <= 1e-6 * (1.0 + p.abs()));
        if same {
            agree += 1;
        } else {
            differ += 1;
        }
        if seen.insert((i, j)) && seen.len() <= maxp {
            let lf = |r: &rpopt::critical_region::CriticalRegion| rpopt::algorithms::mplcp::count_lex_infeasible(&prog, std::slice::from_ref(r)) == 0;
            let mut ai = ri.active_set.clone();
            ai.sort_unstable();
            let mut aj = rj.active_set.clone();
            aj.sort_unstable();
            let qp = prog.solve_theta(&th).map(|o| {
                let mut v = o.active_set.clone();
                v.sort_unstable();
                v
            });
            println!(
                "pair {i} {ai:?} lexfeas={} r={:.2e} | {j} {aj:?} lexfeas={} r={:.2e} | laws agree={same} | qp {qp:?}",
                lf(ri),
                ri.cheb_radius.unwrap_or(f64::NAN),
                lf(rj),
                rj.cheb_radius.unwrap_or(f64::NAN)
            );
        }
    }
    println!("regions {} samples_in_two {samples_ov} laws_agree {agree} laws_differ {differ} distinct_pairs {}", sol.critical_regions.len(), seen.len());
}
