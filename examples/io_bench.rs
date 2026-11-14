//! Save/load benchmark for explicit solutions and hash tables.
//!
//! cargo run --release --example io_bench -- [problem names...] [--reps 9] [--large]
//!
//! For each problem (Geometric solve): JSON and binary size, in-memory estimate, save and load
//! medians (to/from files in the temp dir, page cache warm), hash table save/load vs rebuild,
//! and the round-trip checks (binary bit-exact, evaluate on 10,000 random points, JSON bit-exact?).
//! Timings are medians over repetitions; label them "busy machine" when the machine is shared.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::point_location::*;
use rpopt::program::MpProgram;
use rpopt::solution::Solution;
use rpopt::Format;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

fn median_ms<T>(reps: usize, mut f: impl FnMut() -> T) -> f64 {
    let mut v: Vec<f64> = (0..reps)
        .map(|_| {
            let t = Instant::now();
            black_box(f());
            t.elapsed().as_secs_f64() * 1e3
        })
        .collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn names_in(dir: &PathBuf) -> Vec<String> {
    let mut v: Vec<String> = match std::fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map(|x| x == "json").unwrap_or(false)).map(|p| p.file_stem().unwrap().to_string_lossy().to_string()).collect(),
        Err(_) => vec![],
    };
    v.sort();
    v
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut reps = 9usize;
    let mut want: Vec<String> = vec![];
    let mut large = true;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--reps" => {
                reps = args[i + 1].parse().unwrap();
                i += 1;
            }
            "--no-large" => large = false,
            "--large" => large = true,
            a => want.push(a.to_string()),
        }
        i += 1;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench");
    let mut jobs: Vec<(String, PathBuf)> = vec![];
    for n in names_in(&root.join("problems")) {
        jobs.push((n.clone(), root.join("problems").join(format!("{n}.json"))));
    }
    if large {
        let big = names_in(&root.join("problems_large"));
        if big.is_empty() {
            eprintln!("bench/problems_large not found, skipping large problems");
        }
        for n in big {
            jobs.push((format!("L:{n}"), root.join("problems_large").join(format!("{n}.json"))));
        }
    }
    if !want.is_empty() {
        jobs.retain(|(n, _)| want.iter().any(|w| n == w || n.ends_with(&format!(":{w}"))));
    }
    let tmp = std::env::temp_dir().join(format!("rpopt_io_bench_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    println!("busy machine (shared): medians of {reps} runs, release build, files in {}", tmp.display());
    println!(
        "{:<16} {:>7} {:>9} {:>9} {:>9} | {:>8} {:>8} {:>8} {:>8} | {:>7} {:>7} {:>7} {:>7} {:>7} | {:>6} {:>6}",
        "problem", "regions", "mem KB", "json KB", "bin KB", "jsonSave", "jsonLoad", "binSave", "binLoad", "geomMs", "htBuild", "htSave", "htLoad", "ht KB", "binOK", "jsonOK"
    );
    for (name, path) in jobs {
        let prog = Arc::new(MpProgram::from_json_file(&path).unwrap());
        let sol = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
        let pj = tmp.join("s.json");
        let pb = tmp.join("s.bin");
        sol.save(&pj, Format::Json).unwrap();
        sol.save(&pb, Format::Binary).unwrap();
        let (jkb, bkb) = (std::fs::metadata(&pj).unwrap().len() as f64 / 1024.0, std::fs::metadata(&pb).unwrap().len() as f64 / 1024.0);
        let json_save = median_ms(reps, || sol.save(&pj, Format::Json).unwrap());
        let json_load = median_ms(reps, || Solution::load(&pj).unwrap());
        let bin_save = median_ms(reps, || sol.save(&pb, Format::Binary).unwrap());
        let bin_load = median_ms(reps, || Solution::load(&pb).unwrap());

        // round trip checks
        let back_b = Solution::load(&pb).unwrap();
        let back_j = Solution::load(&pj).unwrap();
        let bin_ok = sol.bit_identical(&back_b, true);
        let json_ok = sol.bit_identical(&back_j, false);
        let geom = PartitionGeometry::new(&sol);
        let mut rng = StdRng::seed_from_u64(1);
        let mut eval_diff = 0usize;
        for _ in 0..10_000 {
            let p: Vec<f64> = (0..geom.dim).map(|k| rng.gen_range(geom.global_lo[k]..geom.global_hi[k])).collect();
            let (a, b) = (sol.evaluate(&p), back_b.evaluate(&p));
            let same = match (&a, &b) {
                (None, None) => true,
                (Some(x), Some(y)) => x.iter().zip(y).all(|(u, v)| u.to_bits() == v.to_bits()),
                _ => false,
            };
            eval_diff += !same as usize;
        }

        // hash table
        let geom_ms = median_ms(3.min(reps), || PartitionGeometry::new(&sol));
        let ht = HashTable::with_default_limit(&geom);
        let ht_build = median_ms(3.min(reps), || HashTable::with_default_limit(&geom));
        let ph = tmp.join("h.bin");
        ht.save(&ph).unwrap();
        let ht_save = median_ms(reps, || ht.save(&ph).unwrap());
        let ht_load = median_ms(reps, || HashTable::load(&ph).unwrap());
        let ht2 = HashTable::load(&ph).unwrap();
        let ht_ok = ht.bit_identical(&ht2);
        let ht_kb = std::fs::metadata(&ph).unwrap().len() as f64 / 1024.0;

        println!(
            "{:<16} {:>7} {:>9.1} {:>9.1} {:>9.1} | {:>8.2} {:>8.2} {:>8.2} {:>8.2} | {:>7.1} {:>7.1} {:>7.2} {:>7.2} {:>7.1} | {:>6} {:>6}",
            name,
            sol.len(),
            sol.memory_bytes() as f64 / 1024.0,
            jkb,
            bkb,
            json_save,
            json_load,
            bin_save,
            bin_load,
            geom_ms,
            ht_build,
            ht_save,
            ht_load,
            ht_kb,
            if bin_ok && eval_diff == 0 && ht_ok { "exact" } else { "DIFF" },
            if json_ok { "exact" } else { "DIFF" }
        );
        if !(bin_ok && eval_diff == 0 && ht_ok) {
            eprintln!("  {name}: binary round trip differs (regions {bin_ok}, eval diffs {eval_diff}, hash table {ht_ok})");
        }
    }
    std::fs::remove_dir_all(&tmp).ok();
}
