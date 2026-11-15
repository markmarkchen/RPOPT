//! Per problem: region count, how many regions have the same primal law (x = A θ + b) as a region sharing a facet
//! with them (pieces of one true critical region split by the lexicographic perturbation), the region count after
//! merging such pieces (connected components), and a sampled overlap/gap check (θ strictly inside two regions, or in
//! none although feasible).
//!
//!   splitcheck <algorithm> <samples> <problem.json>...
//!
//! Facets and laws are matched on values rounded to 1e-6, so a few matches can be missed or invented by rounding.
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::collections::HashMap;
use std::sync::Arc;

fn rnd(x: f64) -> i64 {
    (x * std::env::var("SC_SCALE").ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(1e6)).round() as i64
}

fn find(p: &mut Vec<usize>, mut i: usize) -> usize {
    while p[i] != i {
        p[i] = p[p[i]];
        i = p[i];
    }
    i
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let alg = MpqpAlgorithm::from_name(&a[1]).expect("algorithm");
    let ns: usize = a[2].parse().unwrap();
    for f in &a[3..] {
        let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(f)).unwrap());
        let sol = solve_mpqp(&prog, alg);
        let regs = &sol.critical_regions;
        let law: Vec<Vec<i64>> = regs.iter().map(|r| r.a.data.iter().map(|&x| rnd(x)).chain(r.b.iter().map(|&x| rnd(x))).collect()).collect();
        // unit-normalised rows keyed by rounded values
        let key = |e: &[f64], f: f64, sign: f64| -> Vec<i64> {
            let nrm = e.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-300);
            e.iter().map(|&v| rnd(sign * v / nrm)).chain(std::iter::once(rnd(sign * f / nrm))).collect()
        };
        let mut by_row: HashMap<Vec<i64>, Vec<usize>> = HashMap::new();
        for (i, r) in regs.iter().enumerate() {
            for k in 0..r.e.rows {
                by_row.entry(key(r.e.row(k), r.f[k], 1.0)).or_default().push(i);
            }
        }
        let mut parent: Vec<usize> = (0..regs.len()).collect();
        let mut has_same = vec![false; regs.len()];
        for (i, r) in regs.iter().enumerate() {
            for k in 0..r.e.rows {
                if let Some(js) = by_row.get(&key(r.e.row(k), r.f[k], -1.0)) {
                    for &j in js {
                        if j != i && law[j] == law[i] {
                            has_same[i] = true;
                            let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                            parent[ri] = rj;
                        }
                    }
                }
            }
        }
        let mut lawcount: HashMap<&Vec<i64>, usize> = HashMap::new();
        for l in &law {
            *lawcount.entry(l).or_default() += 1;
        }
        let dup_laws = lawcount.values().filter(|&&c| c > 1).map(|&c| c).sum::<usize>();
        let same = has_same.iter().filter(|&&b| b).count();
        let comps = (0..regs.len()).filter(|&i| find(&mut parent, i) == i).count();
        if let Ok(other) = std::env::var("SC_OTHER") {
            // regions of this algorithm whose law no region of the other algorithm has, with their Chebyshev radii
            let o = solve_mpqp(&prog, MpqpAlgorithm::from_name(&other).expect("other"));
            let olaws: std::collections::HashSet<Vec<i64>> = o.critical_regions.iter().map(|r| r.a.data.iter().map(|&x| rnd(x)).chain(r.b.iter().map(|&x| rnd(x))).collect()).collect();
            let mut radii: Vec<f64> = regs
                .iter()
                .zip(&law)
                .filter(|(_, l)| !olaws.contains(*l))
                .map(|(r, _)| rpopt::utils::chebyshev_ball(&r.e, &r.f, &[]).map_or(0.0, |s| s.x[r.e.cols]))
                .collect();
            radii.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let q = |f: f64| radii.get(((radii.len() as f64 - 1.0) * f).max(0.0) as usize).copied().unwrap_or(0.0);
            println!("  {} regions with a law {other} lacks: {} (radius min {:.1e} median {:.1e} max {:.1e}); {other} has {} regions, {} laws", a[1], radii.len(), q(0.0), q(0.5), q(1.0), o.critical_regions.len(), olaws.len());
        }
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
            if regs.iter().filter(|r| r.is_inside(&th, -1e-7)).count() >= 2 {
                ov += 1;
            }
            if sol.get_region(&th).is_none() {
                gaps += 1;
            }
        }
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        println!("{name} {} regions {} same_law_as_neighbour {same} regions_sharing_a_law {dup_laws} after_merge {comps} feasible_samples {feas} overlaps {ov} gaps {gaps}", a[1], regs.len());
    }
}
