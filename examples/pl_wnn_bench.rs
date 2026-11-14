//! Weighted nearest-neighbour point location against the other locators.
//!
//! cargo run --release --example pl_wnn_bench -- <spec> [--reps 5] [--nq 100000]
//!
//! specs:  qp:<x>:<t>:<m>:<seed>      random mpQP (problem_generator)
//!         lp:<x>:<t>:<m>:<seed>      random mpLP
//!         mass:<masses>:<horizon>    chain of unit masses and springs, one force input
//!         file:<name>                bench/problems/<name>.json (mpQP)
//!         lpfile:<name>              bench/problems_lp/<name>.json (mpLP)
//! Prints one TSV row per (problem, method) and WNN summary lines. Run each spec in its own
//! process under `timeout`, with RAYON_NUM_THREADS=2 for the solve. Timings are medians of
//! interleaved repetitions of single-thread loops; label them "busy machine" when others run.
//! Query sets: uniform in the parameter box, closed-loop trajectories (problems with an mpc key),
//! and a random walk with steps of 2% of the box width.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::linalg::Mat;
use rpopt::mpc::MpcProblem;
use rpopt::point_location::opt_tree::{OptTreeOptions, OptimizedTree};
use rpopt::point_location::*;
use rpopt::program::MpProgram;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

fn expm(a: &Mat) -> Mat {
    let n = a.rows;
    let nrm = a.max_abs() * n as f64;
    let s = (nrm.log2().ceil().max(0.0) as i32 + 1).max(0);
    let a = a.scale(0.5f64.powi(s));
    let mut term = Mat::eye(n);
    let mut sum = Mat::eye(n);
    for k in 1..20 {
        term = term.matmul(&a).scale(1.0 / k as f64);
        sum = sum.add(&term);
    }
    for _ in 0..s {
        sum = sum.matmul(&sum);
    }
    sum
}

/// Solves the discrete Riccati equation by fixed-point iteration.
fn dare(a: &Mat, b: &Mat, q: &Mat, r: &Mat) -> Mat {
    let mut p = q.clone();
    for _ in 0..5000 {
        let bt = b.transpose();
        let s = r.add(&bt.matmul(&p).matmul(b));
        let g = rpopt::linalg::Lu::new(&s).unwrap().solve_mat(&bt.matmul(&p).matmul(a));
        let pn = q.add(&a.transpose().matmul(&p).matmul(a)).sub(&a.transpose().matmul(&p).matmul(b).matmul(&g));
        let d = pn.sub(&p).max_abs();
        p = pn;
        if d < 1e-12 {
            break;
        }
    }
    p
}

fn box_poly(bound: &[f64]) -> rpopt::mpc::Polytope {
    let n = bound.len();
    let mut h = vec![];
    for s in [1.0, -1.0] {
        for i in 0..n {
            let mut r = vec![0.0; n];
            r[i] = s;
            h.push(r);
        }
    }
    rpopt::mpc::Polytope { h, k: [bound, bound].concat() }
}

/// Condensed explicit MPC of a chain of `nm` unit masses (state dimension 2 nm), as in bench/mpc_problems.py.
fn mass_chain(nm: usize, horizon: usize) -> (MpProgram, MpcProblem) {
    let ts = 0.5;
    let nx = 2 * nm;
    let mut ac = Mat::zeros(nx + 1, nx + 1);
    for i in 0..nm {
        ac[(i, nm + i)] = 1.0;
        ac[(nm + i, i)] = -2.0;
        if i + 1 < nm {
            ac[(nm + i, i + 1)] = 1.0;
            ac[(nm + i + 1, i)] = 1.0;
        }
    }
    ac[(nm, nx)] = 1.0;
    let m = expm(&ac.scale(ts));
    let mut a = Mat::zeros(nx, nx);
    let mut b = Mat::zeros(nx, 1);
    for i in 0..nx {
        for j in 0..nx {
            a[(i, j)] = m[(i, j)];
        }
        b[(i, 0)] = m[(i, nx)];
    }
    let q = Mat::eye(nx);
    let r = Mat::eye(1);
    let p = dare(&a, &b, &q, &r);
    let (xb, ub) = (vec![4.0; nx], vec![0.5]);
    let mpc = MpcProblem {
        a: a.to_rows(),
        b: b.to_rows(),
        q: q.to_rows(),
        r: r.to_rows(),
        p: p.to_rows(),
        n: horizon,
        u_poly: box_poly(&ub),
        x_poly: box_poly(&xb),
        t_poly: box_poly(&xb),
        row_map: vec![],
        original: None,
    };
    let qp = mpc.horizon_qp(horizon);
    let keep: Vec<usize> = (0..qp.g.rows).filter(|&i| qp.g.row(i).iter().any(|v| v.abs() > 1e-12)).collect();
    let g = qp.g.select_rows(&keep);
    let e = qp.e.select_rows(&keep);
    let w: Vec<f64> = keep.iter().map(|&i| qp.w[i]).collect();
    let mut a_t = Mat::zeros(2 * nx, nx);
    for i in 0..nx {
        a_t[(i, i)] = 1.0;
        a_t[(nx + i, i)] = -1.0;
    }
    let prog = MpProgram::new_mpqp(g, w, vec![0.0; qp.nm], qp.fm.clone(), qp.h.clone(), a_t, vec![4.0; 2 * nx], e, vec![]);
    (prog, mpc)
}

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

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}


struct ArcLoc<T: PointLocator>(Arc<T>);
impl<T: PointLocator> PointLocator for ArcLoc<T> {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn locate(&self, theta: &[f64]) -> Option<usize> {
        self.0.locate(theta)
    }
    fn storage_floats(&self) -> usize {
        self.0.storage_floats()
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let spec = args.first().expect("spec").clone();
    let mut reps = 5usize;
    let mut nq = 100_000usize;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--reps" => {
                reps = args[i + 1].parse().unwrap();
                i += 1;
            }
            "--nq" => {
                nq = args[i + 1].parse().unwrap();
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    let parts: Vec<&str> = spec.split(':').collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench");
    let mut algo = MpqpAlgorithm::GeometricParallel;
    let (prog, mpc): (MpProgram, Option<MpcProblem>) = match parts[0] {
        "qp" => (rpopt::problem_generator::generate_mpqp(parts[1].parse().unwrap(), parts[2].parse().unwrap(), parts[3].parse().unwrap(), parts[4].parse().unwrap()), None),
        "lp" => {
            algo = MpqpAlgorithm::MplpLex;
            (rpopt::problem_generator::generate_mplp(parts[1].parse().unwrap(), parts[2].parse().unwrap(), parts[3].parse().unwrap(), parts[4].parse().unwrap()), None)
        }
        "mass" => {
            let (p, m) = mass_chain(parts[1].parse().unwrap(), parts[2].parse().unwrap());
            (p, Some(m))
        }
        "file" => {
            let p = root.join("problems").join(format!("{}.json", parts[1]));
            (MpProgram::from_json_file(&p).unwrap(), MpcProblem::from_json_file(&p).ok())
        }
        "lpfile" => {
            algo = MpqpAlgorithm::MplpLex;
            let p = root.join("problems_lp").join(format!("{}.json", parts[1]));
            (MpProgram::from_json_file(&p).unwrap(), MpcProblem::from_json_file(&p).ok())
        }
        _ => panic!("bad spec"),
    };
    let prog = Arc::new(prog);
    let t0 = Instant::now();
    let sol = solve_mpqp(&prog, algo);
    println!("# {spec}: theta {} regions {} solve {:.1}s", prog.num_t(), sol.len(), t0.elapsed().as_secs_f64());
    let geom = PartitionGeometry::new(&sol);
    eprintln!("# geometry done");
    let nt = prog.num_t();
    let nreg = geom.num_regions;
    if std::env::var("WNN_LP_ONLY").is_ok() {
        // power-diagram data from the adjacency LP alone (no value-function route)
        let adj = geom.adjacency_list();
        let mut comp: Vec<usize> = (0..nreg).collect();
        fn find(c: &mut Vec<usize>, x: usize) -> usize {
            let mut r = x;
            while c[r] != r {
                r = c[r];
            }
            let mut y = x;
            while c[y] != r {
                let nx = c[y];
                c[y] = r;
                y = nx;
            }
            r
        }
        for &(a, b, _) in &adj {
            let (ra, rb) = (find(&mut comp, a), find(&mut comp, b));
            comp[ra] = rb;
        }
        let ncomp = (0..nreg).filter(|&i| find(&mut comp, i) == i).count();
        println!("# adjacency graph: {} adjacencies, {} connected components, thin regions {}", adj.len(), ncomp, geom.thin.iter().filter(|&&t| t).count());
        let w = Wnn::with_fallback(&geom, None, None);
        let r = &w.report;
        println!("# WNN-LP-ONLY {spec}: regions {nreg} status {:?} nftf {} excluded {} max_vertex_violation {:.2e} lp_s {:.2}", r.status, r.non_facet_to_facet, r.excluded, r.max_vertex_violation, r.lp_seconds);
        return;
    }
    let (lo, hi) = theta_box(&prog);
    let seq = FlatSequential::new(&sol);
    let mut rng = StdRng::seed_from_u64(11);
    let uniform: Vec<Vec<f64>> = (0..nq).map(|_| (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect()).collect();
    let ht8 = Arc::new(HashTable::with_storage_limit(&geom, 8 << 20));
    let t_hash = Instant::now();
    let _ = t_hash;
    let walk_of = |len: usize, seed: u64| -> Vec<Vec<f64>> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut x = geom.centers[rng.gen_range(0..nreg)].clone();
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            out.push(x.clone());
            let y: Vec<f64> = (0..nt)
                .map(|k| {
                    let w = (hi[k] - lo[k]) * 0.02;
                    (x[k] + rng.gen_range(-w..w)).clamp(lo[k], hi[k])
                })
                .collect();
            if ht8.locate(&y).is_some() {
                x = y;
            }
        }
        out
    };
    let mut modes: Vec<(&str, Vec<Vec<f64>>)> = vec![("uniform", uniform)];
    if let Some(m) = &mpc {
        if m.nx() == nt {
            let mut qs = vec![];
            let mut tries = 0;
            while qs.len() < nq && tries < 100_000 {
                tries += 1;
                let mut x: Vec<f64> = (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect();
                if ht8.locate(&x).is_none() {
                    continue;
                }
                for _ in 0..100 {
                    qs.push(x.clone());
                    let Some(r) = ht8.locate(&x) else { break };
                    let u = sol.critical_regions[r].evaluate(&x);
                    x = (0..m.nx()).map(|i| (0..m.nx()).map(|j| m.a[i][j] * x[j]).sum::<f64>() + (0..m.nu()).map(|j| m.b[i][j] * u[j]).sum::<f64>()).collect();
                }
            }
            if qs.len() > 100 {
                modes.push(("trajectory", qs));
            }
        }
    }
    modes.push(("walk", walk_of(nq, 21)));
    let refs: Vec<Vec<Option<usize>>> = modes.iter().map(|(_, qs)| qs.iter().map(|q| seq.locate(q)).collect()).collect();

    struct M {
        name: String,
        loc: Box<dyn PointLocator>,
        build_s: f64,
        bytes: usize,
    }
    let mut ms: Vec<M> = vec![];
    let tsec = |t: Instant| t.elapsed().as_secs_f64();
    ms.push(M { name: "sequential".into(), loc: Box::new(FlatSequential::new(&sol)), build_s: 0.0, bytes: seq.storage_floats() * 8 });
    let t = Instant::now();
    let bst = BinarySearchTree::with_options(&geom, &rpopt::point_location::bst::BstOptions { max_nodes: (64usize << 20) / (8 * (nt + 3)), ..Default::default() });
    let b = tsec(t);
    eprintln!("# bst {b:.1}s");
    ms.push(M { name: if bst.truncated { "bst (truncated)".into() } else { "bst".into() }, build_s: b, bytes: bst.storage_floats() * 8, loc: Box::new(bst) });
    let t = Instant::now();
    let tr = OptimizedTree::with_options(&geom, &OptTreeOptions { max_bytes: 64 << 20, max_seconds: 30.0, ..Default::default() });
    let b = tsec(t);
    eprintln!("# opt tree {b:.1}s");
    ms.push(M { name: if tr.truncated { "opt tree (truncated)".into() } else { "opt tree".into() }, build_s: b, bytes: tr.storage_bytes(), loc: Box::new(tr) });
    let t = Instant::now();
    let ht_timed = HashTable::with_storage_limit(&geom, 8 << 20);
    let b = tsec(t);
    ms.push(M { name: "hash 8MB".into(), build_s: b, bytes: ht_timed.storage_bytes(), loc: Box::new(ht_timed) });
    let t = Instant::now();
    let walk = SubdivisionWalk::with_fallback(&geom, ht8.clone());
    ms.push(M { name: "walk".into(), build_s: tsec(t), bytes: walk.storage_floats() * 8, loc: Box::new(walk) });
    let t = Instant::now();
    let gt = GraphTraversal::with_fallback(&geom, ht8.clone());
    ms.push(M { name: "graph".into(), build_s: tsec(t), bytes: gt.storage_floats() * 8, loc: Box::new(gt) });
    let t = Instant::now();
    let wnn = Arc::new(Wnn::with_fallback(&geom, Some(&sol), Some(ht8.clone())));
    let wb = tsec(t);
    eprintln!("# wnn {wb:.1}s");
    let r = &wnn.report;
    println!(
        "# WNN {spec}: status {:?} nftf {} excluded {} duplicate_sites {} max_vertex_violation {:.2e} centers_correct {}/{} lp_s {:.2} tree_s {:.3} build_s_excl_hash {:.2} bytes_excl_fallback {} bytes_incl_fallback {}",
        r.status,
        r.non_facet_to_facet,
        r.excluded,
        r.duplicate_sites,
        r.max_vertex_violation,
        r.centers_correct,
        r.num_regions,
        r.lp_seconds,
        r.tree_seconds,
        wb,
        wnn.storage_floats_excl_fallback() * 8,
        wnn.storage_floats() * 8
    );
    ms.push(M { name: "wnn".into(), build_s: wb, bytes: wnn.storage_floats() * 8, loc: Box::new(ArcLoc(wnn.clone())) });
    // outcome statistics of wnn per mode, split by inside/outside the feasible set
    for (k, (mn, qs)) in modes.iter().enumerate() {
        let (mut nodes, mut scored, mut cnt) = (0usize, 0usize, 0usize);
        for (q, rf) in qs.iter().zip(&refs[k]).take(20_000) {
            if rf.is_some() {
                let (a, b) = wnn.search_cost(q);
                nodes += a;
                scored += b;
                cnt += 1;
            }
        }
        if cnt > 0 && nodes > 0 {
            println!("# WNNCOST {spec} {mn}: kd nodes visited {:.1} regions scored {:.1} of {nreg}", nodes as f64 / cnt as f64, scored as f64 / cnt as f64);
        }
        let mut c = [[0usize; 4]; 2];
        for (q, rf) in qs.iter().zip(&refs[k]) {
            let (_, how) = wnn.locate_outcome(q);
            c[rf.is_some() as usize][how as usize] += 1;
        }
        let ins: usize = c[1].iter().sum();
        let outs: usize = c[0].iter().sum();
        let pc = |a: usize, b: usize| if b == 0 { f64::NAN } else { 100.0 * a as f64 / b as f64 };
        println!(
            "# WNNOUT {spec} {mn}: inside {} (tree {:.2}% neighbor {:.2}% fallback {:.3}%), outside {} (outside-exit {:.2}% fallback {:.3}%)",
            ins,
            pc(c[1][0], ins),
            pc(c[1][1], ins),
            pc(c[1][3], ins),
            outs,
            pc(c[0][2], outs),
            pc(c[0][3], outs)
        );
    }
    print!("spec\ttheta\tregions\tmethod\tbuild_s\tbytes");
    for (mn, _) in &modes {
        print!("\tns_{mn}");
    }
    println!("\tmismatch");
    let mut times: Vec<Vec<Vec<f64>>> = vec![vec![vec![]; modes.len()]; ms.len()];
    let mut mism = vec![0usize; ms.len()];
    for (mi, m) in ms.iter().enumerate() {
        for (k, (_, qs)) in modes.iter().enumerate() {
            for (q, r) in qs.iter().zip(&refs[k]) {
                let got = m.loc.locate(q);
                let bad = match (got, r) {
                    (None, None) => false,
                    (Some(j), Some(_)) => !seq.region_contains(j, q),
                    _ => true,
                };
                mism[mi] += bad as usize;
            }
        }
    }
    for _ in 0..reps {
        for (k, (_, qs)) in modes.iter().enumerate() {
            for (mi, m) in ms.iter().enumerate() {
                let qq = if m.name == "sequential" { &qs[..qs.len().min(2000)] } else { &qs[..] };
                let t0 = Instant::now();
                for q in qq {
                    black_box(m.loc.locate(black_box(q)));
                }
                times[mi][k].push(t0.elapsed().as_secs_f64() * 1e9 / qq.len() as f64);
            }
        }
    }
    for (mi, m) in ms.iter().enumerate() {
        print!("{spec}\t{nt}\t{nreg}\t{}\t{:.2}\t{}", m.name, m.build_s, m.bytes);
        for k in 0..modes.len() {
            print!("\t{:.1}", median(times[mi][k].clone()));
        }
        println!("\t{}", mism[mi]);
    }
}
