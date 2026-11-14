//! ns per query of every locator on interior points of a bench problem (tie-rule overhead check).
//!
//! cargo run --release --example pl_tie_bench -- <problem name in bench/problems> [--nq 20000] [--reps 7]
//! Uses only API that exists before and after the strict-first change, so the same file builds on
//! both trees. Queries are strictly interior points (convex combinations of region vertices pulled
//! towards the Chebyshev center), the usual case that must cost the same under both rules.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::point_location::*;
use rpopt::program::MpProgram;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let name = args[0].clone();
    let mut nq = 20_000usize;
    let mut reps = 7usize;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--nq" => {
                nq = args[i + 1].parse().unwrap();
                i += 1;
            }
            "--reps" => {
                reps = args[i + 1].parse().unwrap();
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/problems").join(format!("{name}.json"));
    let prog = Arc::new(MpProgram::from_json_file(&p).unwrap());
    let sol = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
    let geom = PartitionGeometry::new(&sol);
    let nt = geom.dim;
    let mut rng = StdRng::seed_from_u64(3);
    let qs: Vec<Vec<f64>> = (0..nq)
        .map(|_| {
            let r = rng.gen_range(0..geom.num_regions);
            let vs = &geom.vertices[r];
            let ws: Vec<f64> = (0..4).map(|_| -rng.gen_range(1e-9f64..1.0).ln()).collect();
            let s: f64 = ws.iter().sum();
            let mut x: Vec<f64> = geom.centers[r].iter().map(|c| 0.05 * c).collect();
            for w in &ws {
                let v = &vs[rng.gen_range(0..vs.len())];
                for k in 0..nt {
                    x[k] += 0.95 * w / s * v[k];
                }
            }
            x
        })
        .collect();
    let mut locs: Vec<(String, Box<dyn PointLocator>)> = vec![
        ("sequential".into(), Box::new(SequentialSearch::new(&sol))),
        ("sequential early exit".into(), Box::new(FlatSequential::new(&sol))),
        ("bst".into(), Box::new(BinarySearchTree::new(&geom))),
        ("hash 8MB".into(), Box::new(HashTable::with_storage_limit(&geom, 8 << 20))),
        ("walk".into(), Box::new(SubdivisionWalk::new(&geom))),
        ("graph traversal".into(), Box::new(GraphTraversal::new(&geom))),
    ];
    if let Some(lm) = LinearMachine::new(&geom) {
        locs.push(("linear machine".into(), Box::new(lm)));
    }
    let bst = BinarySearchTree::new(&geom);
    let n = geom.dim;
    let mut rows = vec![];
    for i in 0..geom.num_regions {
        let (e, f) = geom.region_rows(i);
        for k in 0..f.len() {
            rows.extend_from_slice(&e[k * n..(k + 1) * n]);
            rows.push(f[k]);
        }
    }
    locs.push(("multiway".into(), Box::new(MultiwayTree::from_bst(&bst, 3, rows, geom.offsets.clone()))));
    println!("# {name}: theta {nt} regions {}", geom.num_regions);
    for (lname, loc) in &locs {
        let mut t = vec![];
        let mut found = 0usize;
        for _ in 0..reps {
            let t0 = Instant::now();
            for q in &qs {
                black_box(loc.locate(black_box(q)));
            }
            t.push(t0.elapsed().as_secs_f64() * 1e9 / qs.len() as f64);
        }
        for q in &qs {
            found += loc.locate(q).is_some() as usize;
        }
        println!("{name}\t{lname}\t{:.1}\t{}/{}", median(t), found, qs.len());
    }
}
