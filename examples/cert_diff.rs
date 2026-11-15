//! Solves with the facet certificate off and on and compares the two region sets. For every active
//! set in only one of them: lexico-feasibility, Chebyshev radius, how many regions of the other
//! solve contain its Chebyshev center, and the active set of a direct QP solve there.
//!
//!   cert_diff <problem.json>
use rpopt::algorithms::mplcp::{count_lex_infeasible, set_certificate};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::critical_region::CriticalRegion;
use rpopt::program::MpProgram;
use std::collections::BTreeMap;
use std::sync::Arc;

fn sorted(r: &CriticalRegion) -> Vec<usize> {
    let mut a = r.active_set.clone();
    a.sort_unstable();
    a
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(&a[1])).unwrap());
    set_certificate(0);
    let off = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
    set_certificate(3);
    let on = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
    println!("off {} regions, on {} regions", off.critical_regions.len(), on.critical_regions.len());
    let mo: BTreeMap<Vec<usize>, &CriticalRegion> = off.critical_regions.iter().map(|r| (sorted(r), r)).collect();
    let mn: BTreeMap<Vec<usize>, &CriticalRegion> = on.critical_regions.iter().map(|r| (sorted(r), r)).collect();
    let nt = prog.num_t();
    for (tag, only, other) in [("only OFF", &mo, &on), ("only ON", &mn, &off)] {
        let o: &BTreeMap<Vec<usize>, &CriticalRegion> = if tag == "only OFF" { &mn } else { &mo };
        for (k, r) in only.iter().filter(|(k, _)| !o.contains_key(*k)) {
            let s = rpopt::utils::chebyshev_ball(&r.e, &r.f, &[]);
            let (rad, inside, qp) = match s {
                Some(s) => {
                    let c: Vec<f64> = s.x[..nt].to_vec();
                    let ins: Vec<Vec<usize>> = other.critical_regions.iter().filter(|q| q.is_inside(&c, -1e-9)).map(sorted).collect();
                    let qp = prog.solve_theta(&c).map(|o| {
                        let mut v = o.active_set.clone();
                        v.sort_unstable();
                        v
                    });
                    (s.x[nt], ins, qp)
                }
                None => (f64::NAN, vec![], None),
            };
            println!("{tag} {k:?}: lex_infeasible={} radius={rad:.3e} centre_in_other={inside:?} qp={qp:?}", count_lex_infeasible(&prog, std::slice::from_ref(*r)) == 1);
        }
    }
}
