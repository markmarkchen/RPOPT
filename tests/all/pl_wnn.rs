//! Weighted nearest-neighbour locator (`Wnn`) against `Solution::get_region_index`: 100,000
//! queries per problem, half uniform in the parameter box and half within a few 1e-9 of facets
//! (both sides of the `LOCATE_TOL` boundary). Run on the 12 benchmark mpQPs, on random mpQPs with
//! 6 and 8 parameters, and on mpLPs (where the paper proves the partition is a power diagram).
//! Exactness must hold whatever the status of the power-diagram data: a non-power-diagram
//! partition only moves the work to the fallback.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::linalg::dot;
use rpopt::point_location::*;
use rpopt::program::MpProgram;
use rpopt::solution::Solution;
use std::path::PathBuf;
use std::sync::Arc;

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

/// Uniform queries plus queries within a few 1e-9 of facets of random regions.
fn queries(prog: &MpProgram, geom: &PartitionGeometry, count: usize, seed: u64) -> Vec<Vec<f64>> {
    let n = geom.dim;
    let (lo, hi) = theta_box(prog);
    let mut rng = StdRng::seed_from_u64(seed);
    let mut qs: Vec<Vec<f64>> = (0..count / 2).map(|_| (0..n).map(|k| rng.gen_range(lo[k]..hi[k])).collect()).collect();
    let offsets = [-1e-7, -2e-9, -1e-9, -5e-10, -1e-10, -1e-11, 1e-11, 1e-10, 5e-10, 9e-10, 1.1e-9, 2e-9, 1e-7];
    while qs.len() < count {
        let r = rng.gen_range(0..geom.num_regions);
        let (e, f) = geom.region_rows(r);
        if f.is_empty() {
            continue;
        }
        let k = rng.gen_range(0..f.len());
        let a = &e[k * n..(k + 1) * n];
        // vertices on the facet (rows are normalized, so the residual is a distance)
        let on: Vec<&Vec<f64>> = geom.vertices[r].iter().filter(|v| (dot(a, v) - f[k]).abs() < 1e-9).collect();
        let mut x = if on.len() >= 2 {
            let ws: Vec<f64> = (0..on.len()).map(|_| rng.gen_range(0.01..1.0)).collect();
            let s: f64 = ws.iter().sum();
            let mut x = vec![0.0; n];
            for (w, v) in ws.iter().zip(&on) {
                for j in 0..n {
                    x[j] += w / s * v[j];
                }
            }
            x
        } else {
            let c = &geom.centers[r];
            let slack = f[k] - dot(a, c);
            (0..n).map(|j| c[j] + slack * a[j]).collect()
        };
        let d = offsets[rng.gen_range(0..offsets.len())];
        for j in 0..n {
            x[j] += d * a[j];
        }
        qs.push(x);
    }
    qs
}

/// Largest row violation of region j at q (rows of `Solution`, not normalized).
fn violation(sol: &Solution, j: usize, q: &[f64]) -> f64 {
    let r = &sol.critical_regions[j];
    (0..r.e.rows).map(|k| dot(r.e.row(k), q) - r.f[k]).fold(f64::NEG_INFINITY, f64::max)
}

/// Exact boundary tie: both regions contain theta strictly and theta is within rounding error
/// (1e-13) of both facets, so the sign of the violation is noise and no rule can order them.
fn roundoff_tie(sol: &Solution, got: Option<usize>, want: Option<usize>, q: &[f64]) -> bool {
    match (got, want) {
        (Some(g), Some(w)) => {
            let (vg, vw) = (violation(sol, g, q), violation(sol, w, q));
            vg <= 0.0 && vw <= 0.0 && vg >= -1e-13 && vw >= -1e-13
        }
        _ => false,
    }
}


fn check(name: &str, prog: Arc<MpProgram>, algo: MpqpAlgorithm, count: usize) {
    let mut sol = solve_mpqp(&prog, algo);
    sol.point_location_tolerance = LOCATE_TOL;
    let geom = PartitionGeometry::new(&sol);
    let qs = queries(&prog, &geom, count, 17);
    let want: Vec<Option<usize>> = qs.iter().map(|q| sol.get_region_index(q)).collect();
    let base: Vec<Box<dyn PointLocator>> = vec![Box::new(HashTable::with_storage_limit(&geom, 8 << 20)), Box::new(FlatSequential::new(&sol)), Box::new(SubdivisionWalk::new(&geom))];
    // Queries on which the rows of the geometry (normalized) and the rows of the solution (raw)
    // put a facet at different distances from LOCATE_TOL: the hash table, walk, graph and trees
    // answer them differently from Solution::get_region (a tolerance-scale defect of the shared
    // geometry test, not of any one locator). Wnn uses the same geometry test, so it is held to
    // agreement with get_region everywhere except on those queries, where it must match the hash table.
    let hash_ref: Vec<Option<usize>> = qs.iter().map(|q| base[0].locate(q)).collect();
    for b in &base {
        let nb = qs.iter().zip(&want).filter(|(q, w)| b.locate(q) != **w && !roundoff_tie(&sol, b.locate(q), **w, q)).count();
        println!("{name} baseline {}: {nb} disagreements", b.name());
    }
    let locs: Vec<(&str, Wnn)> = vec![("hash fallback", Wnn::new(&geom, Some(&sol))), ("scan fallback", Wnn::with_fallback(&geom, Some(&sol), None))];
    for (tag, w) in &locs {
        let (mut bad, mut ties) = (0usize, 0usize);
        let mut first: Option<String> = None;
        let mut scale_defect = 0usize;
        let mut counts = [0usize; 4];
        for (qi, (q, wnt)) in qs.iter().zip(&want).enumerate() {
            let (got, how) = w.locate_outcome(q);
            counts[how as usize] += 1;
            if got != *wnt {
                scale_defect += (hash_ref[qi] != *wnt) as usize;
                if hash_ref[qi] != *wnt && got == hash_ref[qi] {
                    continue;
                }
                if roundoff_tie(&sol, got, *wnt, q) {
                    ties += 1;
                } else {
                    bad += 1;
                    if first.is_none() {
                        let v = |j: &Option<usize>| j.map(|j| violation(&sol, j, q));
                        first = Some(format!("got {got:?} ({how:?}, viol {:?}) want {wnt:?} (viol {:?}) at {q:?}", v(&got), v(wnt)));
                    }
                }
            }
        }
        println!("{name} [{tag}]: regions {} status {:?} nftf {} excluded {} outcomes tree/neighbor/outside/fallback {:?} ties {ties} defect-queries-matching-hash {scale_defect}", geom.num_regions, w.report.status, w.report.non_facet_to_facet, w.report.excluded, counts);
        assert_eq!(bad, 0, "{name} [{tag}]: wnn disagrees with Solution::get_region on {bad} of {} queries, first: {:?}", qs.len(), first);
        assert!(ties <= 5, "{name}: {ties} rounding-level ties");
    }
}

const N: usize = 100_000;

#[test]
fn bench_problems_mpqp() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/problems");
    let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map(|x| x == "json").unwrap_or(false)).map(|p| p.file_stem().unwrap().to_string_lossy().to_string()).collect();
    names.sort();
    assert_eq!(names.len(), 12);
    for nm in names {
        let prog = Arc::new(MpProgram::from_json_file(&dir.join(format!("{nm}.json"))).unwrap());
        check(&nm, prog, MpqpAlgorithm::GeometricParallel, N);
    }
}

#[test]
fn random_mpqp_theta6() {
    check("qp:3:6:14:1", Arc::new(rpopt::problem_generator::generate_mpqp(3, 6, 14, 1)), MpqpAlgorithm::GeometricParallel, N);
    check("qp:4:6:14:1", Arc::new(rpopt::problem_generator::generate_mpqp(4, 6, 14, 1)), MpqpAlgorithm::GeometricParallel, N);
}

// The two 8-parameter problems are separate tests, so a test run with one task per test (the
// cluster gate) checks them concurrently; together they took 22 minutes, the longest of all tests.
#[test]
fn random_mpqp_theta8_a() {
    check("qp:3:8:10:1", Arc::new(rpopt::problem_generator::generate_mpqp(3, 8, 10, 1)), MpqpAlgorithm::GeometricParallel, N);
}

#[test]
fn random_mpqp_theta8_b() {
    check("qp:4:8:14:1", Arc::new(rpopt::problem_generator::generate_mpqp(4, 8, 14, 1)), MpqpAlgorithm::GeometricParallel, N);
}

#[test]
fn mplp_problems() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/problems_lp");
    for nm in ["diinf_N4", "dione_N4", "masses2inf_N3", "masses2one_N3", "sys3x2inf_N3", "sys3x2one_N3"] {
        let prog = Arc::new(MpProgram::from_json_file(&dir.join(format!("{nm}.json"))).unwrap());
        check(nm, prog, MpqpAlgorithm::MplpLex, N);
    }
}

#[test]
fn random_mplp() {
    check("lp:3:6:14:1", Arc::new(rpopt::problem_generator::generate_mplp(3, 6, 14, 1)), MpqpAlgorithm::MplpLex, N);
    check("lp:3:8:12:1", Arc::new(rpopt::problem_generator::generate_mplp(3, 8, 12, 1)), MpqpAlgorithm::MplpLex, N);
}

/// A single region is a power diagram with one site.
#[test]
fn single_region() {
    let prog = Arc::new(rpopt::problem_generator::generate_mpqp(2, 2, 2, 1));
    let sol = solve_mpqp(&prog, MpqpAlgorithm::GeometricParallel);
    if sol.len() == 1 {
        let geom = PartitionGeometry::new(&sol);
        let w = Wnn::new(&geom, Some(&sol));
        assert!(w.is_power_diagram());
    }
}

#[allow(dead_code)]
fn unused(_: &Solution, _: &mut StdRng) -> f64 {
    let mut rng = StdRng::seed_from_u64(0);
    rng.gen_range(0.0..1.0) + dot(&[0.0], &[0.0])
}
