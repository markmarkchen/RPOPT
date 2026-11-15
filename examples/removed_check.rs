//! Accounts for every region that the tightness flag of `mplcp` removes: solves with the flag on,
//! then off, and classifies each active set that only the off run has by a direct QP at the
//! region's Chebyshev centre.
//!
//!   removed_check <problem.json>
//! Output: counts "bogus_qp_infeasible" (the centre is outside the feasible set), "bogus_qp_other"
//! (the QP's active set differs and the QP solution differs from the region's law), "real" (the QP
//! agrees: a genuine region the flag would lose), "no_centre" (empty); and "only_on" (sets only
//! the flagged-on run has, which must be 0).
use rpopt::algorithms::mplcp::set_tight_check;
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::critical_region::CriticalRegion;
use rpopt::program::MpProgram;
use std::collections::BTreeSet;
use std::sync::Arc;

fn sorted(r: &CriticalRegion) -> Vec<usize> {
    let mut a = r.active_set.clone();
    a.sort_unstable();
    a
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(&a[1])).unwrap());
    let nt = prog.num_t();
    set_tight_check(true);
    let on: BTreeSet<Vec<usize>> = solve_mpqp(&prog, MpqpAlgorithm::Mplcp).critical_regions.iter().map(sorted).collect();
    set_tight_check(false);
    let off = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
    let name = std::path::Path::new(&a[1]).file_stem().unwrap().to_string_lossy().to_string();
    let off_sets: BTreeSet<Vec<usize>> = off.critical_regions.iter().map(sorted).collect();
    let only_on = on.difference(&off_sets).count();
    let (mut infeasible, mut other, mut real, mut nocentre) = (0, 0, 0, 0);
    let mut reals: Vec<String> = vec![];
    for r in off.critical_regions.iter().filter(|r| !on.contains(&sorted(r))) {
        let Some(s) = rpopt::utils::chebyshev_ball(&r.e, &r.f, &[]) else {
            nocentre += 1;
            continue;
        };
        let c: Vec<f64> = s.x[..nt].to_vec();
        match prog.solve_theta(&c) {
            None => infeasible += 1,
            Some(o) => {
                let mut qa = o.active_set.clone();
                qa.sort_unstable();
                let x = r.evaluate(&c);
                let same_x = x.len() == o.sol.len() && x.iter().zip(&o.sol).all(|(p, q)| (p - q).abs() <= 1e-6 * (1.0 + q.abs()));
                if qa == sorted(r) || same_x {
                    real += 1;
                    reals.push(format!("{:?} radius {:.2e} qp {:?}", sorted(r), s.x[nt], qa));
                } else {
                    other += 1;
                }
            }
        }
    }
    println!(
        "{name} on {} off {} removed {} bogus_qp_infeasible {infeasible} bogus_qp_other {other} real {real} no_centre {nocentre} only_on {only_on}",
        on.len(),
        off_sets.len(),
        off_sets.len() - (off_sets.len() - off_sets.difference(&on).count())
    );
    for r in reals {
        println!("  real: {r}");
    }
}
