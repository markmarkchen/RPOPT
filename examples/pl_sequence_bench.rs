//! Point location on query sequences: subdivision walking and graph traversal against the hash
//! table (default and best storage limit), the BST and sequential search.
//!
//! Query modes: (a) uniform random in the parameter box (about a third lie outside the feasible set); (b) closed-loop trajectories x+ = A x + B u
//! (u from the explicit law) from random feasible starts, 1000 x 100 steps, for problems with an "mpc" key;
//! (c) random walks in theta with steps of 1%, 5% and 20% of the box width (steps that leave the feasible set are rejected).
//!
//! cargo run --release --example pl_sequence_bench -- [problem names...] [--reps 7]
//!
//! Timings are medians over interleaved repetitions. Run on a quiet machine for clean numbers.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::mpc::MpcProblem;
use rpopt::point_location::*;
use rpopt::program::MpProgram;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

fn theta_box(prog: &MpProgram) -> (Vec<f64>, Vec<f64>) {
    let n = prog.num_t();
    let (mut lo, mut hi) = (vec![-1.0; n], vec![1.0; n]);
    for k in 0..n {
        let mut c = vec![0.0; n];
        c[k] = 1.0;
        if let Some(s) = rpopt::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
            lo[k] = s.obj;
        }
        c[k] = -1.0;
        if let Some(s) = rpopt::solver::lp::solve_lp(Some(&c), &prog.a_t, &prog.b_t, &[]) {
            hi[k] = -s.obj;
        }
    }
    (lo, hi)
}

fn random_inside(lo: &[f64], hi: &[f64], rng: &mut StdRng, inside: &dyn Fn(&[f64]) -> bool) -> Vec<f64> {
    loop {
        let x: Vec<f64> = (0..lo.len()).map(|k| rng.gen_range(lo[k]..hi[k])).collect();
        if inside(&x) {
            return x;
        }
    }
}

fn random_walk(lo: &[f64], hi: &[f64], frac: f64, len: usize, rng: &mut StdRng, inside: &dyn Fn(&[f64]) -> bool) -> Vec<Vec<f64>> {
    let n = lo.len();
    let mut x = random_inside(lo, hi, rng, inside);
    let mut out = vec![];
    for _ in 0..len {
        out.push(x.clone());
        let mut y = x.clone();
        for k in 0..n {
            let w = (hi[k] - lo[k]) * frac;
            let mut v = x[k] + rng.gen_range(-w..w);
            if v < lo[k] {
                v = 2.0 * lo[k] - v;
            }
            if v > hi[k] {
                v = 2.0 * hi[k] - v;
            }
            y[k] = v.clamp(lo[k], hi[k]);
        }
        // steps that leave the feasible set are rejected
        if inside(&y) {
            x = y;
        }
    }
    out
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut reps = 7usize;
    let mut names: Vec<String> = vec![];
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--reps" {
            reps = args[i + 1].parse().unwrap();
            i += 1;
        } else {
            names.push(args[i].clone());
        }
        i += 1;
    }
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/problems");
    if names.is_empty() {
        let mut v: Vec<String> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map(|x| x == "json").unwrap_or(false)).map(|p| p.file_stem().unwrap().to_string_lossy().to_string()).collect();
        v.sort();
        names = v;
    }
    println!("# ns/query (median of {reps} interleaved reps, busy machine if others are running); walk/graph: avg steps per query, fallback rate among queries inside the feasible set (fb_in); uniform queries outside the set always fall back to answer None");
    for name in &names {
        let path = dir.join(format!("{name}.json"));
        let prog = Arc::new(MpProgram::from_json_file(&path).unwrap());
        let mpc = MpcProblem::from_json_file(&path).ok();
        let sol = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
        let geom = PartitionGeometry::new(&sol);
        let (lo, hi) = theta_box(&prog);
        let mut rng = StdRng::seed_from_u64(11);
        let nt = lo.len();
        let seqref = FlatSequential::new(&sol);

        let inside = |x: &[f64]| seqref.locate(x).is_some();
        let uniform: Vec<Vec<f64>> = (0..100_000).map(|_| (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect()).collect();
        let mut modes: Vec<(String, Vec<Vec<f64>>)> = vec![("uniform".into(), uniform.clone())];
        if let Some(m) = &mpc {
            let mut qs = vec![];
            for _ in 0..1000 {
                let mut x = random_inside(&lo, &hi, &mut rng, &inside);
                for _ in 0..100 {
                    qs.push(x.clone());
                    let Some(r) = seqref.locate(&x) else { break };
                    let u = sol.critical_regions[r].evaluate(&x);
                    x = (0..m.nx()).map(|i| (0..m.nx()).map(|j| m.a[i][j] * x[j]).sum::<f64>() + (0..m.nu()).map(|j| m.b[i][j] * u[j]).sum::<f64>()).collect();
                }
            }
            modes.push(("trajectory".into(), qs));
        }
        for (nm, f) in [("walk 1%", 0.01), ("walk 5%", 0.05), ("walk 20%", 0.2)] {
            modes.push((nm.into(), random_walk(&lo, &hi, f, 100_000, &mut rng, &inside)));
        }

        // locators: hash table best limit chosen on the uniform queries
        let ht_default = Arc::new(HashTable::with_default_limit(&geom));
        let mut best: Option<(f64, usize, Arc<HashTable>)> = None;
        for limit in [1usize << 16, 1 << 18, 1 << 20, 1 << 22, 1 << 24, 1 << 26] {
            let ht = Arc::new(HashTable::with_storage_limit(&geom, limit));
            let t0 = Instant::now();
            for q in uniform.iter().take(20_000) {
                black_box(ht.locate(q));
            }
            let t = t0.elapsed().as_secs_f64();
            if best.as_ref().map(|b| t < b.0).unwrap_or(true) {
                best = Some((t, limit, ht));
            }
        }
        let (_, best_limit, ht_best) = best.unwrap();
        let walk = SubdivisionWalk::with_fallback(&geom, ht_default.clone());
        let gt = GraphTraversal::with_fallback(&geom, ht_default.clone());
        let bst = BinarySearchTree::new(&geom);
        let seq = FlatSequential::new(&sol);
        let locs: Vec<(&str, &dyn PointLocator)> = vec![
            ("hash default", &*ht_default),
            ("hash best", &*ht_best),
            ("bst", &bst),
            ("sequential", &seq),
            ("walk", &walk),
            ("graph", &gt),
        ];
        println!(
            "\n== {name}: {} regions, {} facets with 2+ neighbours, {} adjacencies; hash best limit {} KiB",
            geom.num_regions,
            geom.non_facet_to_facet(),
            geom.num_adjacencies(),
            best_limit >> 10
        );
        print!("{:<12}", "mode");
        for (l, _) in &locs {
            print!("{:>14}", l);
        }
        println!("{:>20}{:>20}", "walk steps/fb_in%", "graph steps/fb_in%");
        for (mname, qs) in &modes {
            let mut times: Vec<Vec<f64>> = vec![vec![]; locs.len()];
            for _ in 0..reps {
                for (li, (ln, l)) in locs.iter().enumerate() {
                    // sequential search is O(R) per query and carries no state: time a prefix
                    let qq = if *ln == "sequential" { &qs[..qs.len().min(5000)] } else { &qs[..] };
                    let t0 = Instant::now();
                    for q in qq {
                        black_box(l.locate(black_box(q)));
                    }
                    times[li].push(t0.elapsed().as_secs_f64() * 1e9 / qq.len() as f64);
                }
            }
            // walk statistics over the whole sequence from a cold cache
            let (mut ws, mut wf, mut gs, mut gf) = (0u64, 0u64, 0u64, 0u64);
            let (mut last_w, mut last_g) = (0usize, 0usize);
            for q in qs {
                let o = walk.locate_from_stats(q, last_w);
                ws += o.steps as u64;
                wf += (o.fell_back && o.region.is_some()) as u64;
                if let Some(r) = o.region {
                    last_w = r;
                }
                let o = gt.locate_from_stats(q, last_g);
                gs += o.steps as u64;
                gf += (o.fell_back && o.region.is_some()) as u64;
                if let Some(r) = o.region {
                    last_g = r;
                }
            }
            let n = qs.len() as f64;
            print!("{:<12}", mname);
            for t in times {
                print!("{:>14.1}", median(t));
            }
            println!("{:>12.2}/{:<7.2}{:>12.2}/{:<7.2}", ws as f64 / n, 100.0 * wf as f64 / n, gs as f64 / n, 100.0 * gf as f64 / n);
        }
    }
}
