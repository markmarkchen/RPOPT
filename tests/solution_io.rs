//! Save / load of solutions and hash tables: bit-exact round trips on all benchmark problems,
//! deterministic bytes, and errors (never panics) on malformed input.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::io::Error;
use rpopt::point_location::*;
use rpopt::program::MpProgram;
use rpopt::solution::Solution;
use rpopt::Format;
use std::path::PathBuf;
use std::sync::Arc;

fn problems() -> Vec<(String, Solution)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/problems");
    let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map(|x| x == "json").unwrap_or(false)).map(|p| p.file_stem().unwrap().to_string_lossy().to_string()).collect();
    names.sort();
    assert_eq!(names.len(), 12);
    names
        .into_iter()
        .map(|n| {
            let prog = Arc::new(MpProgram::from_json_file(&dir.join(format!("{n}.json"))).unwrap());
            let sol = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
            (n, sol)
        })
        .collect()
}

fn theta_box(sol: &Solution) -> (Vec<f64>, Vec<f64>) {
    let g = PartitionGeometry::new(sol);
    (g.global_lo.clone(), g.global_hi.clone())
}

fn random_points(lo: &[f64], hi: &[f64], count: usize, seed: u64) -> Vec<Vec<f64>> {
    let mut rng = StdRng::seed_from_u64(seed);
    (0..count).map(|_| (0..lo.len()).map(|k| rng.gen_range(lo[k]..hi[k])).collect()).collect()
}

fn same_eval(a: &Option<Vec<f64>>, b: &Option<Vec<f64>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| p.to_bits() == q.to_bits()),
        _ => false,
    }
}

#[test]
fn binary_round_trip_bit_exact_and_deterministic() {
    for (name, sol) in problems() {
        let bytes = sol.to_bytes().unwrap();
        assert_eq!(bytes, sol.to_bytes().unwrap(), "{name}: bytes not deterministic");
        assert_eq!(&bytes[..8], b"RPOPTSOL");
        let back = Solution::from_bytes(&bytes).unwrap();
        assert!(sol.bit_identical(&back, true), "{name}: binary round trip differs");
        assert_eq!(bytes, back.to_bytes().unwrap(), "{name}: re-encoding differs");
        let (lo, hi) = theta_box(&sol);
        for p in random_points(&lo, &hi, 2000, 7) {
            assert!(same_eval(&sol.evaluate(&p), &back.evaluate(&p)), "{name}: evaluate differs");
        }
        // the loaded solution carries its program: verification and locator rebuild work without the problem file
        assert_eq!(back.theta_dim(), sol.theta_dim());
        let _ = HashTable::with_default_limit(&PartitionGeometry::new(&back));
    }
}

#[test]
fn json_round_trip_and_files() {
    let dir = std::env::temp_dir().join(format!("rpopt_io_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, sol) in problems().into_iter().take(4) {
        let pj = dir.join(format!("{name}.json"));
        let pb = dir.join(format!("{name}.bin"));
        sol.save(&pj, Format::Json).unwrap();
        sol.save(&pb, Format::Binary).unwrap();
        let j = Solution::load(&pj).unwrap();
        let b = Solution::load(&pb).unwrap();
        assert!(sol.bit_identical(&j, false), "{name}: json differs");
        assert!(sol.bit_identical(&b, true), "{name}: binary file differs");
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn solution_errors_not_panics() {
    let (_, sol) = problems().into_iter().next().unwrap();
    let bytes = sol.to_bytes().unwrap();
    // wrong magic
    let mut b = bytes.clone();
    b[0] ^= 0xff;
    assert!(matches!(Solution::from_bytes(&b), Err(Error::BadMagic)));
    // wrong version
    let mut b = bytes.clone();
    b[8] = 99;
    assert!(matches!(Solution::from_bytes(&b), Err(Error::UnsupportedVersion(99))));
    // empty and tiny
    assert!(Solution::from_bytes(&[]).is_err());
    assert!(Solution::from_bytes(b"RPOPT").is_err());
    // every truncation point
    for cut in (0..bytes.len()).step_by((bytes.len() / 400).max(1)) {
        assert!(Solution::from_bytes(&bytes[..cut]).is_err(), "truncation at {cut} accepted");
    }
    // bit flips anywhere are caught by the checksum
    let mut rng = StdRng::seed_from_u64(3);
    for _ in 0..300 {
        let mut b = bytes.clone();
        let i = rng.gen_range(0..b.len());
        b[i] ^= 1 << rng.gen_range(0..8);
        assert!(Solution::from_bytes(&b).is_err(), "flip at {i} accepted");
    }
    // trailing garbage
    let mut b = bytes.clone();
    b.push(0);
    assert!(Solution::from_bytes(&b).is_err());
    // missing file, garbage JSON
    assert!(matches!(Solution::load("/nonexistent/dir/x.bin"), Err(Error::Io(_))));
    assert!(matches!(Solution::from_any_bytes(b"{not json"), Err(Error::Json(_))));
}

/// Re-signs a payload so that only the structural validation can catch the damage.
fn resign(mut b: Vec<u8>) -> Vec<u8> {
    let n = b.len() - 8;
    let c = rpopt::io::checksum(&b[..n]);
    b[n..].copy_from_slice(&c.to_le_bytes());
    b
}

#[test]
fn dimension_mismatch_is_an_error() {
    let (_, sol) = problems().into_iter().next().unwrap();
    // JSON whose region has the wrong parameter dimension
    let mut bad = sol.clone();
    let r = &mut bad.critical_regions[0];
    r.e.cols += 1;
    r.e.data.extend(std::iter::repeat(0.0).take(r.e.rows));
    assert!(matches!(bad.to_bytes(), Err(Error::DimensionMismatch(_))));
    // a binary file whose F matrix is altered to a different parameter count but is otherwise well-formed
    let bytes = sol.to_bytes().unwrap();
    let mut b = bytes.clone();
    // flip the first u32 of the payload after the flags and tolerance: A.rows
    let off = 20 + 4 + 8;
    b[off] = b[off].wrapping_add(1);
    let b = resign(b);
    assert!(Solution::from_bytes(&b).is_err());
    // hash table against a different partition
    let g = PartitionGeometry::new(&sol);
    let ht = HashTable::with_default_limit(&g);
    assert!(ht.check_compatible(g.dim, g.num_regions).is_ok());
    assert!(matches!(ht.check_compatible(g.dim + 1, g.num_regions), Err(Error::DimensionMismatch(_))));
    assert!(matches!(ht.check_compatible(g.dim, g.num_regions + 1), Err(Error::DimensionMismatch(_))));
}

#[test]
fn hash_table_round_trip_matches_sequential() {
    for (name, sol) in problems() {
        let geom = PartitionGeometry::new(&sol);
        let ht = HashTable::with_default_limit(&geom);
        let bytes = ht.to_bytes().unwrap();
        assert_eq!(bytes, ht.to_bytes().unwrap());
        let back = HashTable::from_bytes(&bytes).unwrap();
        assert!(ht.bit_identical(&back), "{name}: hash table differs after round trip");
        assert_eq!(bytes, back.to_bytes().unwrap());
        back.check_compatible(geom.dim, geom.num_regions).unwrap();
        let seq = FlatSequential::new(&sol);
        let (lo, hi) = theta_box(&sol);
        let mut mismatches = 0;
        for p in random_points(&lo, &hi, 100_000, 11) {
            let truth = seq.locate(&p);
            let got = back.locate(&p);
            assert_eq!(got, ht.locate(&p));
            let ok = match (truth, got) {
                (None, None) => true,
                (Some(_), Some(g)) => seq.region_contains(g, &p),
                _ => false,
            };
            mismatches += !ok as usize;
        }
        assert_eq!(mismatches, 0, "{name}: {mismatches} mismatches against sequential search");
    }
}

#[test]
fn hash_table_errors_not_panics() {
    let (_, sol) = problems().into_iter().next().unwrap();
    let ht = HashTable::with_default_limit(&PartitionGeometry::new(&sol));
    let bytes = ht.to_bytes().unwrap();
    let mut b = bytes.clone();
    b[1] ^= 1;
    assert!(matches!(HashTable::from_bytes(&b), Err(Error::BadMagic)));
    let mut b = bytes.clone();
    b[8] = 42;
    assert!(matches!(HashTable::from_bytes(&b), Err(Error::UnsupportedVersion(42))));
    for cut in (0..bytes.len()).step_by((bytes.len() / 300).max(1)) {
        assert!(HashTable::from_bytes(&bytes[..cut]).is_err());
    }
    let mut rng = StdRng::seed_from_u64(5);
    for _ in 0..300 {
        let mut b = bytes.clone();
        let i = rng.gen_range(0..b.len());
        b[i] ^= 1 << rng.gen_range(0..8);
        let r = HashTable::from_bytes(&b);
        assert!(r.is_err(), "flip at {i} accepted");
        // same flip with a valid checksum must be rejected by validation or still load safely
        if let Ok(h) = HashTable::from_bytes(&resign(b)) {
            let p = vec![0.1; h.dim()];
            let _ = h.locate(&p);
        }
    }
    // a Solution file is not a hash table and vice versa
    assert!(matches!(HashTable::from_bytes(&sol.to_bytes().unwrap()), Err(Error::BadMagic)));
    assert!(matches!(Solution::from_bytes(&bytes), Err(Error::BadMagic)));
}
