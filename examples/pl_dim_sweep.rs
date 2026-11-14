//! Dimension sweep for point location: tree vs hash table as the parameter dimension grows.
//!
//! cargo run --release --example pl_dim_sweep -- <spec> [--reps 5] [--nq 100000] [--solve-only]
//!
//! specs:  qp:<x>:<t>:<m>:<seed>      random mpQP (problem_generator)
//!         mass:<masses>:<horizon>    chain of unit masses and springs, one force input
//!         file:<name>                bench/problems/<name>.json (or bench/problems_large)
//! Prints one TSV row per (problem, method); run each spec in its own process under `timeout`.
//! Timings are medians of interleaved repetitions; label them "busy machine" when others run.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::linalg::Mat;
use rpopt::mpc::MpcProblem;
use rpopt::point_location::*;
use rpopt::point_location::opt_tree::{Candidates, Criterion, OptTreeOptions, OptimizedTree};
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

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let spec = args.first().expect("spec").clone();
    let mut reps = 5usize;
    let mut nq = 100_000usize;
    let mut solve_only = false;
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
            "--solve-only" => solve_only = true,
            _ => {}
        }
        i += 1;
    }
    let parts: Vec<&str> = spec.split(':').collect();
    let (prog, mpc): (MpProgram, Option<MpcProblem>) = match parts[0] {
        "qp" => (rpopt::problem_generator::generate_mpqp(parts[1].parse().unwrap(), parts[2].parse().unwrap(), parts[3].parse().unwrap(), parts[4].parse().unwrap()), None),
        "mass" => {
            let (p, m) = mass_chain(parts[1].parse().unwrap(), parts[2].parse().unwrap());
            (p, Some(m))
        }
        "file" => {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench");
            let mut p = root.join("problems").join(format!("{}.json", parts[1]));
            if !p.exists() {
                p = root.join("problems_large").join(format!("{}.json", parts[1]));
            }
            (MpProgram::from_json_file(&p).unwrap(), MpcProblem::from_json_file(&p).ok())
        }
        _ => panic!("bad spec"),
    };
    let _ = mpc;
    let prog = Arc::new(prog);
    let t0 = Instant::now();
    let sol = solve_mpqp(&prog, MpqpAlgorithm::GeometricParallel);
    let solve_s = t0.elapsed().as_secs_f64();
    println!("# {spec}: theta {} regions {} solve {:.1}s", prog.num_t(), sol.len(), solve_s);
    if solve_only {
        return;
    }
    let t0 = Instant::now();
    let geom = PartitionGeometry::new(&sol);
    let geom_s = t0.elapsed().as_secs_f64();
    println!("# geometry {:.1}s", geom_s);
    let nt = prog.num_t();
    let nreg = geom.num_regions;
    let (lo, hi) = theta_box(&prog);
    let seq = FlatSequential::new(&sol);
    let mut rng = StdRng::seed_from_u64(11);

    // ---- query sets
    let uniform: Vec<Vec<f64>> = (0..nq).map(|_| (0..nt).map(|k| rng.gen_range(lo[k]..hi[k])).collect()).collect();
    let inside_q: Vec<Vec<f64>> = (0..nq)
        .map(|_| {
            let r = rng.gen_range(0..nreg);
            let vs = &geom.vertices[r];
            if vs.is_empty() {
                return geom.centers[r].clone();
            }
            let ws: Vec<f64> = (0..4).map(|_| -rng.gen_range(1e-9f64..1.0).ln()).collect();
            let s: f64 = ws.iter().sum();
            // strictly interior: 2% of the Chebyshev center keeps the point off the facets
            let mut x: Vec<f64> = geom.centers[r].iter().map(|c| 0.02 * c).collect();
            for w in &ws {
                let v = &vs[rng.gen_range(0..vs.len())];
                for k in 0..nt {
                    x[k] += 0.98 * w / s * v[k];
                }
            }
            x
        })
        .collect();
    let ht_gen: Box<dyn PointLocator> = if HashTable::supports_dim(nt) { Box::new(HashTable::with_storage_limit(&geom, 8 << 20)) } else { Box::new(FlatSequential::new(&sol)) };
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
            if ht_gen.locate(&y).is_some() {
                x = y;
            }
        }
        out
    };
    let walk = walk_of(nq, 21);
    // training distribution for the probability-weighted trees (independent walk)
    let train = walk_of(20_000.min(nq), 33);
    let mut wts = vec![0.5f64; nreg];
    for q in &train {
        if let Some(j) = ht_gen.locate(q) {
            wts[j] += 1.0;
        }
    }
    drop(ht_gen);
    let modes: Vec<(&str, &Vec<Vec<f64>>)> = vec![("uniform", &uniform), ("inside", &inside_q), ("walk", &walk)];
    let feas_frac = uniform.iter().take(20_000).filter(|q| seq.locate(q).is_some()).count() as f64 / uniform.len().min(20_000) as f64;
    println!("# uniform queries inside the feasible set: {:.4}", feas_frac);
    let refs: Vec<Vec<Option<usize>>> = modes.iter().map(|(_, qs)| qs.iter().map(|q| seq.locate(q)).collect()).collect();

    // ---- build methods
    struct M {
        name: String,
        loc: Box<dyn PointLocator>,
        build_s: f64,
        bytes: usize,
        depth_max: String,
        depth_of: Option<Box<dyn Fn(&[f64]) -> (usize, usize)>>,
    }
    let mut ms: Vec<M> = vec![];
    let tsec = |t: Instant| t.elapsed().as_secs_f64();
    ms.push(M { name: "sequential".into(), loc: Box::new(FlatSequential::new(&sol)), build_s: 0.0, bytes: seq.storage_floats() * 8, depth_max: format!("{}", nreg), depth_of: None });
    let skip_bst = std::env::var("SWEEP_SKIP_BST").is_ok();
    // node cap: about the 64 MB hash table budget (a node costs at least n + 3 floats)
    let bst_cap: usize = if skip_bst { 1 } else { (64usize << 20) / (8 * (nt + 3)) };
    let t = Instant::now();
    let bst = Arc::new(BinarySearchTree::with_options(&geom, &rpopt::point_location::bst::BstOptions { max_nodes: bst_cap, ..Default::default() }));
    let b = tsec(t);
    eprintln!("# bst built {b:.1}s");
    let bst_depth = |tree: &BinarySearchTree, th: &[f64]| -> (usize, usize) {
        let mut node = 0;
        let mut d = 0;
        loop {
            match &tree.nodes[node] {
                rpopt::point_location::bst::BstNode::Split { plane, below, above } => {
                    node = if rpopt::linalg::dot(&plane[..tree.dim], th) <= plane[tree.dim] { *below } else { *above };
                    d += 1;
                }
                rpopt::point_location::bst::BstNode::Leaf { candidates } => return (d, candidates.len()),
            }
        }
    };
    if !skip_bst {
        let bb = bst.clone();
        let bytes = bst.storage_floats() * 8;
        let dm = bst.depth;
        let wrapper = ArcLoc(bst.clone());
        ms.push(M { name: if bst.truncated { "bst (truncated)".into() } else { "bst".into() }, loc: Box::new(wrapper), build_s: b, bytes, depth_max: format!("{dm}"), depth_of: Some(Box::new(move |th| bst_depth(&bb, th))) });
    }
    let t = Instant::now();
    let bstw = Arc::new(BinarySearchTree::with_options(&geom, &rpopt::point_location::bst::BstOptions { weights: Some(wts.clone()), max_nodes: bst_cap, ..Default::default() }));
    if !skip_bst {
        let b = tsec(t);
        let bb = bstw.clone();
        let bytes = bstw.storage_floats() * 8;
        let dm = bstw.depth;
        ms.push(M { name: if bstw.truncated { "bst weighted (truncated)".into() } else { "bst weighted".into() }, loc: Box::new(ArcLoc(bstw.clone())), build_s: b, bytes, depth_max: format!("{dm}"), depth_of: Some(Box::new(move |th| bst_depth(&bb, th))) });
    }
    let opt_variants: Vec<(&str, OptTreeOptions)> = vec![
        ("facet tree", OptTreeOptions { candidates: Candidates::Facets, ..Default::default() }),
        ("opt tree", OptTreeOptions::default()),
        ("opt tree weighted", OptTreeOptions { weights: Some(wts.clone()), ..Default::default() }),
    ];
    let skip_opt = std::env::var("SWEEP_SKIP_OPT").is_ok();
    // SWEEP_EXPLORE=1: tuning grid of tree options instead of the standard variants
    let opt_variants: Vec<(&str, OptTreeOptions)> = if std::env::var("SWEEP_EXPLORE").is_ok() {
        vec![
            ("opt w1", OptTreeOptions::default()),
            ("opt w0.6", OptTreeOptions { omega: 0.6, ..Default::default() }),
            ("opt w1.5", OptTreeOptions { omega: 1.5, ..Default::default() }),
            ("opt expd", OptTreeOptions { criterion: Criterion::ExpDepth, ..Default::default() }),
            ("facet tree", OptTreeOptions { candidates: Candidates::Facets, ..Default::default() }),
            ("kd tree", OptTreeOptions { candidates: Candidates::Axes, ..Default::default() }),
        ]
    } else {
        opt_variants
    };
    let tree_bytes: usize = std::env::var("SWEEP_TREE_MB").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(64) << 20;
    let tree_secs: f64 = std::env::var("SWEEP_TREE_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(150.0);
    for (nm, o) in opt_variants {
        if skip_opt {
            break;
        }
        let t = Instant::now();
        let o = OptTreeOptions { max_bytes: tree_bytes, max_seconds: tree_secs, exact_refinement: o.exact_refinement && std::env::var("SWEEP_NOREFINE").is_err(), ..o };
        let tr = Arc::new(OptimizedTree::with_options(&geom, &o));
        let b = tsec(t);
        eprintln!("# {nm} built {b:.1}s depth {} nodes {}", tr.depth, tr.num_nodes());
        let t2 = tr.clone();
        let bytes = tr.storage_bytes();
        let dm = tr.depth;
        ms.push(M { name: if tr.truncated { format!("{nm} (truncated)") } else { nm.to_string() }, loc: Box::new(ArcLoc(tr.clone())), build_s: b, bytes, depth_max: format!("{dm}"), depth_of: Some(Box::new(move |th| {
            let (_, d, c) = t2.locate_counted(th);
            (d, c)
        })) });
    }
    // the hash table supports 1 to MAX_DIM parameters
    let hash_limits: Vec<(&str, usize)> = if HashTable::supports_dim(nt) { vec![("hash 1MB", 1usize << 20), ("hash 8MB", 8 << 20), ("hash 64MB", 64 << 20)] } else { vec![] };
    for (nm, lim) in hash_limits {
        let t = Instant::now();
        let ht = Arc::new(HashTable::with_storage_limit(&geom, lim));
        let b = tsec(t);
        let bytes = ht.storage_bytes();
        let dm = format!("{}/c{}", ht.max_depth(), ht.max_candidates());
        ms.push(M { name: nm.into(), loc: Box::new(ArcLoc(ht)), build_s: b, bytes, depth_max: dm, depth_of: None });
    }

    // ---- mismatches, depth statistics
    println!("spec\ttheta\tregions\tmethod\tbuild_s\tbytes\tns_uniform\tns_inside\tns_walk\tdepth_max\tdepth_avg_walk\tcand_avg_walk\tmismatch");
    let mut times: Vec<Vec<Vec<f64>>> = vec![vec![vec![]; modes.len()]; ms.len()];
    let mut mism = vec![0usize; ms.len()];
    let dump = std::env::var("SWEEP_DUMP").is_ok();
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
                if bad && dump {
                    let viol = |j: usize| seq.region_violation(j, q);
                    let strict: Vec<usize> = (0..nreg).filter(|&j| geom_viol(&geom, j, q) <= 0.0).collect();
                    let near: Vec<usize> = (0..nreg).filter(|&j| geom_viol(&geom, j, q) <= LOCATE_TOL).collect();
                    let gv = got.map(|j| geom_viol(&geom, j, q));
                    let _ = viol;
                    if let Some(jr) = r {
                        let (a, b) = geom.region_mat(*jr);
                        let rv = rpopt::point_location::opt_tree::robust_vertices(&a, &b, 1e-7);
                        let (mut mx, mut mn) = (f64::NEG_INFINITY, f64::INFINITY);
                        let _ = (&mut mx, &mut mn);
                        println!("  region {} rows {} radius {:e} verts {} robust_verts {}", jr, a.rows, geom.radii[*jr], geom.vertices[*jr].len(), rv.len());
                    }
                    println!("MISMATCH\t{}\t{}\tgot={:?} got_viol={:?} ref={:?} strict={:?} near={:?}\ttheta={:?}", spec, m.name, got, gv, r, strict, near, q);
                }
            }
        }
    }
    for _ in 0..reps {
        for (k, (_, qs)) in modes.iter().enumerate() {
            for (mi, m) in ms.iter().enumerate() {
                // sequential search is O(R) per query: time a prefix
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
        let (mut da, mut ca) = (f64::NAN, f64::NAN);
        if let Some(f) = &m.depth_of {
            let (mut ds, mut cs) = (0usize, 0usize);
            for q in &walk {
                let (d, c) = f(q);
                ds += d;
                cs += c;
            }
            da = ds as f64 / walk.len() as f64;
            ca = cs as f64 / walk.len() as f64;
        }
        println!(
            "{spec}\t{nt}\t{nreg}\t{}\t{:.2}\t{}\t{:.1}\t{:.1}\t{:.1}\t{}\t{:.2}\t{:.2}\t{}",
            m.name,
            m.build_s,
            m.bytes,
            median(times[mi][0].clone()),
            median(times[mi][1].clone()),
            median(times[mi][2].clone()),
            m.depth_max,
            da,
            ca,
            mism[mi]
        );
    }
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

/// Largest row violation of region j at q (normalized rows, unscaled; matches the locators' scale only up to normalization).
fn geom_viol(geom: &PartitionGeometry, j: usize, q: &[f64]) -> f64 {
    let (e, f) = geom.region_rows(j);
    let n = geom.dim;
    (0..f.len()).map(|k| rpopt::linalg::dot(&e[k * n..(k + 1) * n], q) - f[k]).fold(f64::NEG_INFINITY, f64::max)
}
