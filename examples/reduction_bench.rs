//! Region counts, reduction time and hash-table cost before and after complexity reduction.
//!
//! cargo run --release --example reduction_bench [problem ...]
//! Timings depend on machine load; the hash table limit is the same before and after.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::point_location::geometry::enumerate_vertices;
use rpopt::point_location::{HashTable, PartitionGeometry, PointLocator};
use rpopt::program::MpProgram;
use rpopt::reduction::*;
use rpopt::Solution;

const LIMIT: usize = 64 << 10;

fn ns_per_query(loc: &dyn PointLocator, pts: &[Vec<f64>]) -> f64 {
    let mut best = f64::INFINITY;
    for _ in 0..7 {
        let t = std::time::Instant::now();
        let mut acc = 0usize;
        for p in pts {
            acc += loc.locate(p).unwrap_or(usize::MAX) & 1;
        }
        std::hint::black_box(acc);
        best = best.min(t.elapsed().as_secs_f64() * 1e9 / pts.len() as f64);
    }
    best
}

fn ht_stats(sol: &Solution, pts: &[Vec<f64>]) -> (f64, usize) {
    let geom = PartitionGeometry::new(sol);
    let ht = HashTable::with_storage_limit(&geom, LIMIT);
    (ns_per_query(&ht, pts), ht.storage_bytes())
}

fn main() {
    let only: Vec<String> = std::env::args().skip(1).collect();
    let mut files: Vec<_> = std::fs::read_dir("bench/problems").unwrap().map(|e| e.unwrap().path()).collect();
    files.sort();
    println!("busy machine; hash table limit {LIMIT} bytes; ns/query best of 7 over 20000 uniform points");
    println!("{:<14} {:<22} {:>6} {:>7} {:>9} {:>9} {:>10} {:>10}", "problem", "method", "regs", "red_ms", "ht_ns", "ht_bytes", "method_kind", "exact");
    for p in files {
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        if !only.is_empty() && !only.contains(&name) {
            continue;
        }
        let prog = std::sync::Arc::new(MpProgram::from_json_file(&p).unwrap());
        let sol = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
        let n = sol.theta_dim();
        let mut lo = vec![f64::INFINITY; n];
        let mut hi = vec![f64::NEG_INFINITY; n];
        for r in &sol.critical_regions {
            for v in enumerate_vertices(&r.e, &r.f) {
                for k in 0..n {
                    lo[k] = lo[k].min(v[k]);
                    hi[k] = hi[k].max(v[k]);
                }
            }
        }
        let mut rng = StdRng::seed_from_u64(3);
        let pts: Vec<Vec<f64>> = (0..20_000).map(|_| (0..n).map(|k| rng.gen_range(lo[k]..=hi[k])).collect()).collect();
        let (ns0, b0) = ht_stats(&sol, &pts);
        println!("{:<14} {:<22} {:>6} {:>7} {:>9.1} {:>9} {:>10} {:>10}", name, "original", sol.len(), "-", ns0, b0, "-", "-");
        let target = ReduceTarget::from_problem_json(&p);
        for (label, t) in [("merge FullLaw", ReduceTarget::FullLaw), ("merge first inputs", target.clone())] {
            let (red, rep) = merge_regions(&sol, &t, &ReduceOptions::default());
            let (ns, b) = ht_stats(&red, &pts);
            println!("{:<14} {:<22} {:>6} {:>7.1} {:>9.1} {:>9} {:>10} {:>10}", name, label, red.len(), rep.seconds * 1e3, ns, b, if rep.greedy { "greedy" } else { "arrangement" }, rep.exact);
        }
        let nx = sol.critical_regions[0].a.rows;
        let idx = target.indices(nx);
        let bounds = input_bounds_from_json(&p, &target, nx);
        let c = clip_reduce(&sol, &idx, &bounds);
        let stats: Vec<(f64, usize)> = c.components.iter().map(|k| ht_stats(&k.solution, &pts)).collect();
        let ns: f64 = stats.iter().map(|s| s.0).sum();
        let bytes: usize = stats.iter().map(|s| s.1).sum();
        println!("{:<14} {:<22} {:>6} {:>7.1} {:>9.1} {:>9} {:>10} {:>10}", name, "clip first inputs", c.num_regions(), c.seconds * 1e3, ns, bytes, format!("{:?}", c.components.iter().map(|k| k.solution.len()).collect::<Vec<_>>()), "-");
    }
}
