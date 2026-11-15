//! Measures what removing duplicated constraints (rows of [A | F | b] that are positive multiples of another row) before the
//! solve would do to mplcp: the number of duplicate rows, the regions of the solve with and without them, and whether the
//! sorted active sets agree once every twin is replaced by the kept row of its group.
//!
//!   dedupe_check <problem.json>...
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::linalg::Mat;
use rpopt::program::MpProgram;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::BTreeSet;
use std::sync::Arc;

#[cfg(feature = "lp_census")]
fn census_line(tag: &str, regions: usize, secs: f64) {
    use rpopt::census::{self, Counter};
    let st = census::snapshot();
    let c = |k: Counter| st.counter(k);
    println!(
        "  [{tag}] regions {regions} solve_s {secs:.3} adjacent {} none {} fast {} case2 {} cand_gen {} dup {} pf_calls {} pf_cheb {} r1_handled {} r1_fallback {} across {} across_fail {} lp_calls {} lp_ms {:.0} qp_calls {}",
        c(Counter::AdjCalls), c(Counter::AdjNone), c(Counter::AdjFast), c(Counter::AdjCase2), c(Counter::CandGen), c(Counter::Dup), c(Counter::PfCalls), c(Counter::PfCheb),
        c(Counter::R1Handled), c(Counter::R1Fallback), c(Counter::FallbackAcross), c(Counter::FallbackFail), st.lp_calls(), st.lp_nanos() as f64 / 1e6, st.qp_calls()
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    for f in &args[1..] {
        let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(f)).unwrap());
        let m = p.a.rows;
        let nt = p.f.cols;
        let rows: Vec<Vec<f64>> = (0..m)
            .map(|i| {
                let mut v = p.a.row(i).to_vec();
                v.extend_from_slice(p.f.row(i));
                v.push(p.b[i]);
                v
            })
            .collect();
        let norm2: Vec<f64> = rows.iter().map(|v| v[..v.len() - 1].iter().map(|x| x * x).sum::<f64>()).collect();
        // group representative: the lowest index of the group (inequality rows only)
        let mut rep: Vec<usize> = (0..m).collect();
        for i in 0..m {
            if p.equality_indices.contains(&i) || rep[i] != i {
                continue;
            }
            for j in i + 1..m {
                if p.equality_indices.contains(&j) || rep[j] != j || !(norm2[i] > 0.0 && norm2[j] > 0.0) {
                    continue;
                }
                let (si, sj) = (norm2[i].sqrt(), norm2[j].sqrt());
                let c = rows[i][..rows[i].len() - 1].iter().zip(&rows[j]).map(|(a, b)| a * b).sum::<f64>() / norm2[j];
                if c > 0.0 && rows[i].iter().zip(&rows[j]).all(|(a, b)| (a - c * b).abs() <= 1e-9 * si.max(sj) * (1.0 + 0.0)) {
                    rep[j] = i;
                }
            }
        }
        let kept: Vec<usize> = (0..m).filter(|&i| rep[i] == i).collect();
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        let dups = m - kept.len();
        if let Ok(which) = std::env::var("DD_ONLY") {
            // one solve per task for the problems that do not fit in memory twice; prints the hash of the canonical sets
            use std::hash::{Hash, Hasher};
            let t = std::time::Instant::now();
            let canon: BTreeSet<Vec<usize>> = if which == "full" {
                let sol = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
                sol.critical_regions
                    .iter()
                    .map(|r| {
                        let mut s: Vec<usize> = r.active_set.iter().map(|&i| rep[i]).collect();
                        s.sort_unstable();
                        s.dedup();
                        s
                    })
                    .collect()
            } else {
                let new_idx: Vec<usize> = {
                    let mut v = vec![usize::MAX; m];
                    for (k, &i) in kept.iter().enumerate() {
                        v[i] = k;
                    }
                    v
                };
                let a = Mat::from_rows(&kept.iter().map(|&i| p.a.row(i).to_vec()).collect::<Vec<_>>());
                let b: Vec<f64> = kept.iter().map(|&i| p.b[i]).collect();
                let fm = Mat::from_rows(&kept.iter().map(|&i| p.f.row(i).to_vec()).collect::<Vec<_>>());
                let eq: Vec<usize> = p.equality_indices.iter().map(|&i| new_idx[i]).collect();
                let red = Arc::new(MpProgram::from_raw_parts(a, b, p.c.clone(), p.h.clone(), p.q.clone(), p.a_t.clone(), p.b_t.clone(), fm, p.c_c, p.c_t.clone(), p.q_t.clone(), eq));
                let sol = solve_mpqp(&red, MpqpAlgorithm::Mplcp);
                sol.critical_regions
                    .iter()
                    .map(|r| {
                        let mut s: Vec<usize> = r.active_set.iter().map(|&k| kept[k]).collect();
                        s.sort_unstable();
                        s
                    })
                    .collect()
            };
            let mut h = std::collections::hash_map::DefaultHasher::new();
            canon.hash(&mut h);
            println!("{name} {which} rows {m} duplicates {dups} canonical_sets {} canon_hash {:016x} solve_s {:.2}", canon.len(), h.finish(), t.elapsed().as_secs_f64());
            continue;
        }
        let t0 = std::time::Instant::now();
        #[cfg(feature = "lp_census")]
        rpopt::census::reset();
        let full = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
        let t_full = t0.elapsed().as_secs_f64();
        #[cfg(feature = "lp_census")]
        census_line("full", full.critical_regions.len(), t_full);
        if dups == 0 {
            println!("{name} rows {m} duplicates 0 regions {} solve_s {t_full:.2}", full.critical_regions.len());
            continue;
        }
        let new_idx: Vec<usize> = {
            let mut v = vec![usize::MAX; m];
            for (k, &i) in kept.iter().enumerate() {
                v[i] = k;
            }
            v
        };
        let a = Mat::from_rows(&kept.iter().map(|&i| p.a.row(i).to_vec()).collect::<Vec<_>>());
        let b: Vec<f64> = kept.iter().map(|&i| p.b[i]).collect();
        let fm = Mat::from_rows(&kept.iter().map(|&i| p.f.row(i).to_vec()).collect::<Vec<_>>());
        let eq: Vec<usize> = p.equality_indices.iter().map(|&i| new_idx[i]).collect();
        let q = p.q.clone().expect("mpQP");
        let red = Arc::new(MpProgram::from_raw_parts(a, b, p.c.clone(), p.h.clone(), Some(q), p.a_t.clone(), p.b_t.clone(), fm, p.c_c, p.c_t.clone(), p.q_t.clone(), eq));
        let _ = nt;
        if std::env::var("DD_CONTROL").is_ok() {
            // the same construction with every row kept: separates the effect of `post_process = false` from the removal
            let all: Vec<usize> = (0..m).collect();
            let a = Mat::from_rows(&all.iter().map(|&i| p.a.row(i).to_vec()).collect::<Vec<_>>());
            let b: Vec<f64> = all.iter().map(|&i| p.b[i]).collect();
            let fm = Mat::from_rows(&all.iter().map(|&i| p.f.row(i).to_vec()).collect::<Vec<_>>());
            let ctl = Arc::new(MpProgram::from_raw_parts(a, b, p.c.clone(), p.h.clone(), p.q.clone(), p.a_t.clone(), p.b_t.clone(), fm, p.c_c, p.c_t.clone(), p.q_t.clone(), p.equality_indices.clone()));
            let tc = std::time::Instant::now();
            let sol = solve_mpqp(&ctl, MpqpAlgorithm::Mplcp);
            println!("  [control: all rows, from_raw_parts] regions {} solve_s {:.3}", sol.critical_regions.len(), tc.elapsed().as_secs_f64());
        }
        let t1 = std::time::Instant::now();
        #[cfg(feature = "lp_census")]
        rpopt::census::reset();
        let reduced = solve_mpqp(&red, MpqpAlgorithm::Mplcp);
        let t_red = t1.elapsed().as_secs_f64();
        #[cfg(feature = "lp_census")]
        census_line("reduced", reduced.critical_regions.len(), t_red);
        let canon_full: BTreeSet<Vec<usize>> = full
            .critical_regions
            .iter()
            .map(|r| {
                let mut s: Vec<usize> = r.active_set.iter().map(|&i| rep[i]).collect();
                s.sort_unstable();
                s.dedup();
                s
            })
            .collect();
        let canon_red: BTreeSet<Vec<usize>> = reduced
            .critical_regions
            .iter()
            .map(|r| {
                let mut s: Vec<usize> = r.active_set.iter().map(|&k| kept[k]).collect();
                s.sort_unstable();
                s
            })
            .collect();
        // the regions that exist in one solve only: their Chebyshev radius (are they slivers?) and active sets
        for (tag, sols, canon_other) in [("only in full", &full.critical_regions, &canon_red), ("only in reduced", &reduced.critical_regions, &canon_full)] {
            for r in sols.iter() {
                let mut set: Vec<usize> = if tag == "only in full" { r.active_set.iter().map(|&i| rep[i]).collect() } else { r.active_set.iter().map(|&k| kept[k]).collect() };
                set.sort_unstable();
                set.dedup();
                if !canon_other.contains(&set) {
                    let rad = rpopt::utils::chebyshev_ball(&r.e, &r.f, &[]).map_or(f64::NAN, |b| b.x[r.e.cols]);
                    println!("  {tag}: active set {:?} chebyshev radius {rad:.3e}", set);
                }
            }
        }
        // DD_LEX=<samples>: lexico-infeasible regions and sampled overlaps/gaps of both solves
        if let Some(ns) = std::env::var("DD_LEX").ok().and_then(|v| v.parse::<usize>().ok()).filter(|_| full.critical_regions.len() <= 30000) {
            for (tag, prog, sol) in [("full", &p, &full), ("reduced", &red, &reduced)] {
                let lexbad = rpopt::algorithms::mplcp::count_lex_infeasible(prog, &sol.critical_regions);
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
                let (mut feas, mut ov, mut gaps) = (0, 0, 0);
                for _ in 0..ns {
                    let th: Vec<f64> = (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect();
                    if prog.solve_theta(&th).is_none() {
                        continue;
                    }
                    feas += 1;
                    if sol.critical_regions.iter().filter(|r| r.is_inside(&th, -1e-7)).count() >= 2 {
                        ov += 1;
                    }
                    if sol.get_region(&th).is_none() {
                        gaps += 1;
                    }
                }
                println!("  lexcheck {tag}: lex_infeasible {lexbad} feasible_samples {feas} overlaps {ov} gaps {gaps}");
            }
        }
        // regions whose active set contains both of a twin pair (an unmerged double count)
        let both = full.critical_regions.iter().filter(|r| r.active_set.iter().any(|&i| rep[i] != i && r.active_set.contains(&rep[i]))).count();
        println!(
            "{name} rows {m} duplicates {dups} regions full {} reduced {} canonical sets full {} reduced {} only_full {} only_reduced {} regions_with_both_twins {both} solve_s full {t_full:.2} reduced {t_red:.2}",
            full.critical_regions.len(),
            reduced.critical_regions.len(),
            canon_full.len(),
            canon_red.len(),
            canon_full.difference(&canon_red).count(),
            canon_red.difference(&canon_full).count()
        );
    }
}
