//! For the active sets listed in <sets.txt> (one `[i, j, ...]` per line): builds each region from its active set and reports the lexicographic
//! feasibility of its basis (rows with a vanishing theta part and their first significant lexicographic entry), the tightness residual of the law
//! at the Chebyshev centre, the radius, the QP's active set at the centre, which regions of the current mplcp solve contain the centre strictly, and
//! for every pair the largest violation of the other region's rows at each centre.
//!
//!   [MPLCP_REFINE=0] pair_check <problem.json> <sets.txt>
use rpopt::algorithms::mplcp::{lex_diagnostic, tight_diagnostic};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::critical_region::CriticalRegion;
use rpopt::program::MpProgram;
use std::sync::Arc;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if std::env::var("MPLCP_NO_DEDUPE").is_ok() {
        rpopt::algorithms::mplcp::set_dedupe(false);
    }
    if let Ok(v) = std::env::var("MPLCP_REFINE") {
        rpopt::algorithms::mplcp::set_refine_multipliers(v != "0");
    }
    if let Ok(v) = std::env::var("MPLCP_PIVDD") {
        rpopt::algorithms::mplcp::set_pivot_dd(v != "0");
    }
    rpopt::algorithms::mplcp::set_rank1_relaxations(std::env::var("MPLCP_ZCOL").map_or(true, |v| v != "0"), std::env::var("MPLCP_BANDREAL").map_or(true, |v| v != "0"));
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(&a[1])).unwrap());
    let sol = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
    println!("current mplcp: {} regions", sol.critical_regions.len());
    let nt = prog.num_t();
    let mut built: Vec<(Vec<usize>, CriticalRegion, Vec<f64>)> = vec![];
    for line in std::fs::read_to_string(&a[2]).unwrap().lines() {
        let aset: Vec<usize> = line.trim_matches(|c| c == '[' || c == ']').split(',').filter_map(|t| t.trim().parse().ok()).collect();
        if aset.is_empty() {
            continue;
        }
        let Some(cr) = rpopt::mpqp_utils::gen_cr_from_active_set(&prog, &aset, true) else {
            println!("{aset:?}: no full-dimensional region from the active set alone");
            // the region of the solve with this active set, if any
            let mut want = aset.clone();
            want.sort_unstable();
            for r in sol.critical_regions.iter().filter(|r| {
                let mut v = r.active_set.clone();
                v.sort_unstable();
                v == want
            }) {
                let s = rpopt::utils::chebyshev_ball(&r.e, &r.f, &[]).unwrap();
                let centre: Vec<f64> = s.x[..nt].to_vec();
                let (tight, agrees) = tight_diagnostic(&prog, r, &centre);
                let qp = prog.solve_theta(&centre).map(|o| {
                    let mut v = o.active_set.clone();
                    v.sort_unstable();
                    v
                });
                println!("   in the solve: rows {} radius {:.3e} (stored {:?}) tight residual {:.2e} qp agrees {} qp active set {:?}", r.e.rows, s.x[nt], r.cheb_radius, tight, agrees, qp);
            }
            continue;
        };
        let s = rpopt::utils::chebyshev_ball(&cr.e, &cr.f, &[]).unwrap();
        let centre: Vec<f64> = s.x[..nt].to_vec();
        let lex = lex_diagnostic(&prog, &cr);
        println!("   LICQ margin (smallest pivot of the unit-diagonal Gram matrix of {aset:?}): {:?}", rpopt::algorithms::mplcp::licq_margin(&prog, &aset));
        let (tight, qp_agrees) = tight_diagnostic(&prog, &cr, &centre);
        let inside: Vec<Vec<usize>> = sol
            .critical_regions
            .iter()
            .filter(|r| r.is_inside(&centre, -1e-9))
            .map(|r| {
                let mut v = r.active_set.clone();
                v.sort_unstable();
                v
            })
            .collect();
        let qp = prog.solve_theta(&centre).map(|o| {
            let mut v = o.active_set.clone();
            v.sort_unstable();
            v
        });
        println!(
            "{aset:?}: rows {} radius {:.3e} lex (feasible, rows with zero theta part, min signed first entry, min abs first entry) = {:?} tight residual {:.2e} qp agrees {} qp active set {:?} centre inside {:?}",
            cr.e.rows, s.x[nt], lex, tight, qp_agrees, qp, inside
        );
        // the QP at points inside the region (centre plus half the radius in 16 directions, kept only if every row of the region holds with margin)
        {
            let mut want = aset.clone();
            want.sort_unstable();
            let r = s.x[nt];
            let (mut inside_pts, mut feasible, mut same) = (0, 0, 0);
            let mut state = 0x9e3779b97f4a7c15u64;
            for _ in 0..16 {
                let mut u: Vec<f64> = (0..nt)
                    .map(|_| {
                        state ^= state << 13;
                        state ^= state >> 7;
                        state ^= state << 17;
                        (state >> 11) as f64 / (1u64 << 53) as f64 - 0.5
                    })
                    .collect();
                let nrm = u.iter().map(|x| x * x).sum::<f64>().sqrt();
                for x in u.iter_mut() {
                    *x *= 0.5 * r / nrm;
                }
                let pt: Vec<f64> = centre.iter().zip(&u).map(|(c, d)| c + d).collect();
                let ok = (0..cr.e.rows).all(|k| cr.f[k] - cr.e.row(k).iter().zip(&pt).map(|(x, y)| x * y).sum::<f64>() > 0.0);
                if !ok {
                    continue;
                }
                inside_pts += 1;
                if let Some(o) = prog.solve_theta(&pt) {
                    feasible += 1;
                    let mut v = o.active_set.clone();
                    v.sort_unstable();
                    if v == want {
                        same += 1;
                    }
                }
            }
            println!("   QP at half-radius points inside the region: {inside_pts} inside, {feasible} with a QP solution, {same} with exactly this active set");
        }
        built.push((aset.clone(), cr.clone(), centre));
    }
    for i in 0..built.len() {
        for j in 0..built.len() {
            if i != j {
                let crj = &built[j].1;
                let viol = (0..crj.e.rows).map(|k| crj.e.row(k).iter().zip(&built[i].2).map(|(x, y)| x * y).sum::<f64>() - crj.f[k]).fold(f64::NEG_INFINITY, f64::max);
                println!("centre of {:?} against the rows of {:?}: largest E theta - f = {:.3e}", built[i].0, built[j].0, viol);
            }
        }
    }
}
