//! Subdivision walking and graph traversal must agree with sequential search: on uniform random
//! queries (cached and with random start hints), on closed-loop MPC trajectories and on random
//! walks in theta, for all benchmark problems, including the ones that are not facet-to-facet.
//! Agreement means: same inside/outside answer, and the returned region passes the f64 halfspace
//! test (facet points may legitimately be assigned to either neighbour).

#![allow(non_snake_case)]

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::mpc::MpcProblem;
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

fn trajectories(sol: &Solution, mpc: &MpcProblem, lo: &[f64], hi: &[f64], truth: &FlatSequential, rng: &mut StdRng, count: usize, steps: usize) -> Vec<Vec<f64>> {
    let (a, b) = (&mpc.a, &mpc.b);
    let (nx, nu) = (mpc.nx(), mpc.nu());
    let mut out = vec![];
    for _ in 0..count {
        let mut x = random_inside(lo, hi, rng, &|p: &[f64]| truth.locate(p).is_some());
        for _ in 0..steps {
            out.push(x.clone());
            let Some(r) = truth.locate(&x) else { break };
            let u = sol.critical_regions[r].evaluate(&x);
            x = (0..nx).map(|i| (0..nx).map(|j| a[i][j] * x[j]).sum::<f64>() + (0..nu).map(|j| b[i][j] * u[j]).sum::<f64>()).collect();
        }
    }
    out
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
            // reflect into the box
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

fn check(name: &str) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/problems");
    let path = dir.join(format!("{name}.json"));
    let prog = Arc::new(MpProgram::from_json_file(&path).unwrap());
    let mpc = MpcProblem::from_json_file(&path).unwrap();
    let sol = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
    let geom = PartitionGeometry::new(&sol);
    let truth = FlatSequential::new(&sol);
    let hash = Arc::new(HashTable::with_default_limit(&geom));
    let walk = SubdivisionWalk::with_fallback(&geom, hash.clone());
    let gt = GraphTraversal::with_fallback(&geom, hash);
    let (lo, hi) = theta_box(&prog);
    let mut rng = StdRng::seed_from_u64(7);
    let r = sol.len();

    let verify = |what: &str, th: &[f64], got: Option<usize>, want: Option<usize>, who: &str| {
        assert_eq!(got.is_some(), want.is_some(), "{name} {who} {what}: inside/outside differs at {th:?}");
        if let Some(g) = got {
            assert!(truth.region_contains(g, th) || geom.contains(g, th, 1e-9), "{name} {who} {what}: region {g} does not contain {th:?}");
        }
    };

    // uniform random queries: cached, and from random start hints
    let uniform: Vec<Vec<f64>> = (0..100_000).map(|_| (0..lo.len()).map(|k| rng.gen_range(lo[k]..hi[k])).collect()).collect();
    for th in &uniform {
        let want = truth.locate(th);
        verify("uniform", th, walk.locate(th), want, "walk");
        verify("uniform", th, gt.locate(th), want, "graph");
        let s = rng.gen_range(0..r);
        verify("uniform+hint", th, walk.locate_from(th, s), want, "walk");
        verify("uniform+hint", th, gt.locate_from(th, s), want, "graph");
    }
    // out-of-range hint must fall back, not panic
    verify("bad hint", &uniform[0], walk.locate_from(&uniform[0], r + 5), truth.locate(&uniform[0]), "walk");
    verify("bad hint", &uniform[0], gt.locate_from(&uniform[0], r + 5), truth.locate(&uniform[0]), "graph");

    let mut seqs: Vec<(&str, Vec<Vec<f64>>)> = vec![("trajectory", trajectories(&sol, &mpc, &lo, &hi, &truth, &mut rng, 1000, 100))];
    for (nm, frac) in [("walk1%", 0.01), ("walk5%", 0.05), ("walk20%", 0.2)] {
        seqs.push((nm, random_walk(&lo, &hi, frac, 100_000, &mut rng, &|p: &[f64]| truth.locate(p).is_some())));
    }
    for (nm, seq) in &seqs {
        for th in seq {
            let want = truth.locate(th);
            verify(nm, th, walk.locate(th), want, "walk");
            verify(nm, th, gt.locate(th), want, "graph");
        }
    }
    eprintln!("{name}: {r} regions, {} facets with 2+ neighbours: ok", geom.non_facet_to_facet());
}

macro_rules! problems {
    ($($id:ident),*) => { $( #[test] fn $id() { check(stringify!($id)); } )* };
}

problems!(di_doc_N3, di_N4, di_N6, di_N8, di_N10, dinv_N4, dinv_N6, dinv_N8, masses2_N4, masses2_N6, sys3x2_N3, sys3x2_N5);
