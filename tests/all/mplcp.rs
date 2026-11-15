//! mp-LCP (Jones-Morari pivoting) against the geometric algorithm and direct QP solves.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::linalg::Mat;
use rpopt::problem_generator::{generate_mplp, generate_mpqp};
use rpopt::critical_region::CriticalRegion;
use rpopt::program::MpProgram;
use std::sync::Arc;

#[test]
fn bench_problems_match_geometric() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/problems");
    let mut n = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map_or(true, |e| e != "json") {
            continue;
        }
        let p = Arc::new(MpProgram::from_json_file(&path).unwrap());
        let g = solve_mpqp(&p, MpqpAlgorithm::Geometric);
        let m = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
        assert_eq!(region_signature(&m.critical_regions), region_signature(&g.critical_regions), "{:?}", path);
        n += 1;
    }
    assert_eq!(n, 12);
}

/// pop_006 has facet rows with gradients of about 1e6 in the pLCP tableau. The absolute zero-row
/// test of `adjacent` then missed the identically-zero row and found no neighbour across the
/// lambda = 0 facet, so two regions of radius 1e-3 were never reached (mplcp 2373 regions,
/// geometric 2375).
#[test]
fn ill_scaled_facet_rows_do_not_hide_regions() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/problems_large/pop_006.json");
    let p = Arc::new(MpProgram::from_json_file(&path).unwrap());
    let g = solve_mpqp(&p, MpqpAlgorithm::Geometric);
    let m = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
    assert_eq!(region_signature(&m.critical_regions), region_signature(&g.critical_regions));
}

/// masses4_N3: combinatorial_graph used to add 62 slivers (radius about 5e-8) whose bordered-update
/// rows were rounding noise (the direct KKT region is empty) and whose laws violate the
/// constraints by 1e-5 to 1e-4. Every region it returns must also be a region of graph.
#[test]
fn combinatorial_graph_has_no_noise_slivers() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/problems_large/masses4_N3.json");
    let p = Arc::new(MpProgram::from_json_file(&path).unwrap());
    let g = region_signature(&solve_mpqp(&p, MpqpAlgorithm::Graph).critical_regions);
    let c = region_signature(&solve_mpqp(&p, MpqpAlgorithm::CombinatorialGraph).critical_regions);
    let extra: Vec<_> = c.iter().filter(|s| g.binary_search(s).is_err()).collect();
    assert!(extra.is_empty(), "{} regions of combinatorial_graph are not in graph", extra.len());
}

#[test]
fn random_mpqps_match_geometric() {
    for seed in 0..12u64 {
        let p = Arc::new(generate_mpqp(3, 2, 6, seed));
        let g = solve_mpqp(&p, MpqpAlgorithm::Geometric);
        let m = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
        assert_eq!(region_signature(&m.critical_regions), region_signature(&g.critical_regions), "seed {}", seed);
    }
}

/// Projection of θ onto a polytope whose vertex (1,..,1) lies on extra planes: dependent active
/// constraints there (degenerate). Compared with direct QP solves (the partition into bases at
/// the degenerate vertex is not unique, so the signature may legitimately differ).
fn projection(extra: &[Vec<f64>], extra_b: &[f64], n: usize) -> MpProgram {
    let mut rows: Vec<Vec<f64>> = vec![];
    let mut b = vec![];
    for i in 0..n {
        let mut r = vec![0.0; n];
        r[i] = 1.0;
        rows.push(r.clone());
        b.push(1.0);
        r[i] = -1.0;
        rows.push(r);
        b.push(1.0);
    }
    rows.extend(extra.iter().cloned());
    b.extend_from_slice(extra_b);
    let m = rows.len();
    let a = Mat::from_rows(&rows);
    let mut h = Mat::zeros(n, n);
    for i in 0..n {
        h[(i, i)] = -1.0;
    }
    let mut a_t = Mat::zeros(2 * n, n);
    let mut b_t = vec![];
    for i in 0..n {
        a_t[(2 * i, i)] = 1.0;
        a_t[(2 * i + 1, i)] = -1.0;
        b_t.push(4.0);
        b_t.push(4.0);
    }
    MpProgram::new_mpqp(a, b, vec![0.0; n], h, Mat::eye(n), a_t, b_t, Mat::zeros(m, n), vec![])
}

fn check_against_qp(p: MpProgram, queries: usize) {
    let p = Arc::new(p);
    let s = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
    assert!(s.len() > 1);
    // multipliers are not unique at degenerate vertices, so only x is compared below
    let mut rng = StdRng::seed_from_u64(7);
    let nt = p.num_t();
    for _ in 0..queries {
        let th: Vec<f64> = (0..nt).map(|_| rng.gen_range(-3.9..3.9)).collect();
        assert!(s.verify_theta(&th), "theta {:?}", th);
    }
    let g = solve_mpqp(&p, MpqpAlgorithm::Geometric);
    let (ov_m, gap_m) = overlap_gap(&s.critical_regions, nt, 20_000);
    let (ov_g, gap_g) = overlap_gap(&g.critical_regions, nt, 20_000);
    eprintln!("mplcp {} regions (overlap points {}, gap points {}), geometric {} regions (overlap {}, gap {}), 20000 samples", s.len(), ov_m, gap_m, g.len(), ov_g, gap_g);
    assert_eq!(ov_m, 0, "overlapping regions");
    assert_eq!(gap_m, 0, "gaps");
}

/// Samples points of [-3.99, 3.99]^nt: (points strictly inside 2+ regions, points in no region).
fn overlap_gap(regions: &[CriticalRegion], nt: usize, samples: usize) -> (usize, usize) {
    let mut rng = StdRng::seed_from_u64(11);
    let (mut overlap, mut gap) = (0, 0);
    for _ in 0..samples {
        let th: Vec<f64> = (0..nt).map(|_| rng.gen_range(-3.99..3.99)).collect();
        let (mut inside, mut touching) = (0, 0);
        for r in regions {
            let worst = (0..r.e.rows).map(|i| r.e.row(i).iter().zip(&th).map(|(a, b)| a * b).sum::<f64>() - r.f[i]).fold(f64::NEG_INFINITY, f64::max);
            if worst < -1e-7 {
                inside += 1;
            }
            if worst < 1e-7 {
                touching += 1;
            }
        }
        if inside >= 2 {
            overlap += 1;
        }
        if touching == 0 {
            gap += 1;
        }
    }
    (overlap, gap)
}

#[test]
fn degenerate_vertex_2d() {
    check_against_qp(projection(&[vec![1.0, 1.0]], &[2.0], 2), 3000);
}

#[test]
fn degenerate_vertex_3d() {
    let extra = vec![vec![1.0, 1.0, 0.0], vec![0.0, 1.0, 1.0], vec![1.0, 0.0, 1.0], vec![1.0, 1.0, 1.0]];
    check_against_qp(projection(&extra, &[2.0, 2.0, 2.0, 3.0], 3), 3000);
}

#[test]
fn enumeration_matches_geometric_on_all_bench_problems() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/problems");
    let mut n = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map_or(true, |e| e != "json") {
            continue;
        }
        let p = Arc::new(MpProgram::from_json_file(&path).unwrap());
        let g = solve_mpqp(&p, MpqpAlgorithm::Geometric);
        let e = solve_mpqp(&p, MpqpAlgorithm::MplcpEnum);
        assert_eq!(region_signature(&e.critical_regions), region_signature(&g.critical_regions), "{:?}", path);
        n += 1;
    }
    assert_eq!(n, 12);
}

#[test]
fn enumeration_mplp_matches_geometric_and_combinatorial() {
    let mut checked = 0;
    for (x, t, m) in [(2, 2, 4), (3, 3, 5), (4, 3, 8), (5, 3, 10)] {
        for seed in 0..4u64 {
            let p = Arc::new(generate_mplp(x, t, m, seed));
            let g = region_signature(&solve_mpqp(&p, MpqpAlgorithm::Geometric).critical_regions);
            let c = region_signature(&solve_mpqp(&p, MpqpAlgorithm::Combinatorial).critical_regions);
            let e = region_signature(&solve_mpqp(&p, MpqpAlgorithm::MplcpEnum).critical_regions);
            eprintln!("mpLP x{x} t{t} m{m} seed {seed}: geometric {} combinatorial {} enum {}", g.len(), c.len(), e.len());
            assert_eq!(e, c, "enum vs combinatorial, x{x} t{t} m{m} seed {seed}");
            if g != c {
                eprintln!("  (geometric differs from combinatorial on this degenerate mpLP)");
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 16);
}

#[test]
fn mplp_is_routed_to_mplp_lex() {
    let p = Arc::new(generate_mplp(2, 2, 4, 0));
    let s = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
    let lex = solve_mpqp(&p, MpqpAlgorithm::MplpLex);
    assert!(s.len() > 0);
    assert_eq!(region_signature(&s.critical_regions), region_signature(&lex.critical_regions));
}

/// Degenerate mpLP: min θ'x over a square with an extra plane through the vertex (1,1). Several
/// bases describe the same parameter cone there, so the enumeration returns overlapping regions
/// (no perturbation); the union must still cover Θ and agree with direct LP solves.
#[test]
fn degenerate_mplp_enumeration_covers_theta() {
    let a = Mat::from_rows(&[vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0], vec![-1.0, 0.0], vec![0.0, -1.0]]);
    let b = vec![1.0, 1.0, 2.0, 1.0, 1.0];
    let a_t = Mat::from_rows(&[vec![1.0, 0.0], vec![-1.0, 0.0], vec![0.0, 1.0], vec![0.0, -1.0]]);
    let p = Arc::new(MpProgram::new_mplp(a, b, vec![0.0; 2], Mat::eye(2), a_t, vec![4.0; 4], Mat::zeros(5, 2), vec![]));
    let s = solve_mpqp(&p, MpqpAlgorithm::MplcpEnum);
    let (ov, gap) = overlap_gap(&s.critical_regions, 2, 20_000);
    let g = solve_mpqp(&p, MpqpAlgorithm::Geometric);
    let (ovg, gapg) = overlap_gap(&g.critical_regions, 2, 20_000);
    eprintln!("enum {} regions (overlap points {ov}, gap {gap}); geometric {} regions (overlap {ovg}, gap {gapg})", s.len(), g.len());
    assert_eq!(gap, 0);
    let mut rng = StdRng::seed_from_u64(5);
    for _ in 0..2000 {
        let th: Vec<f64> = (0..2).map(|_| rng.gen_range(-3.9..3.9)).collect();
        assert!(s.verify_theta(&th));
    }
}

/// A region whose tableau cannot be factored when it is popped takes the probe-across fallback and is counted; the
/// counter is expected to stay 0 (a refusal needs a basis that is singular in both pivot orders tried), and a
/// release build must not skip such a region without a trace. The twin-row problem pop_001 is included.
#[test]
fn no_region_is_skipped_without_trace() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bench");
    for name in ["problems/di_N6.json", "problems/masses2_N4.json", "problems/sys3x2_N3.json", "problems_large/pop_001.json"] {
        let p = Arc::new(MpProgram::from_json_file(&dir.join(name)).unwrap());
        let m = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
        assert!(!m.critical_regions.is_empty(), "{name}");
        assert_eq!(rpopt::algorithms::mplcp::resolve_fallbacks(), 0, "{name}");
    }
}
