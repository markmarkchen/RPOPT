//! Kenefake and Pistikopoulos' classification pruning must give exactly the regions of
//! `combinatorial`, and its verdicts must be consistent with every region.

use rpopt::algorithms::combinatorial_kp::{classification, solve_with_stats, KpOptions, Pairs};
use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::problem_generator::{generate_mplp, generate_mpqp};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn load(path: &str) -> Arc<MpProgram> {
    Arc::new(MpProgram::from_json_file(std::path::Path::new(path)).unwrap())
}

fn names(dir: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path().file_stem().unwrap().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

fn sig(p: &MpProgram, par: bool, o: KpOptions) -> Vec<Vec<usize>> {
    region_signature(&solve_with_stats(p, par, o).0)
}

/// Options that exercise every path: classification from the start (lazy, eager), switched on
/// late (parents that already break a rule), and switched off by the audit.
fn variants() -> Vec<(&'static str, KpOptions)> {
    vec![
        ("lazy", KpOptions::always()),
        ("eager", KpOptions::paper()),
        ("late", KpOptions { activate_at: 60, ..KpOptions::always() }),
        ("late2", KpOptions { activate_at: 400, ..KpOptions::always() }),
        ("audit", KpOptions { min_benefit: 0.5, ..KpOptions::always() }),
    ]
}

#[test]
fn kp_matches_combinatorial_on_bench_problems() {
    let dir = format!("{}/bench/problems", env!("CARGO_MANIFEST_DIR"));
    let all = names(&dir);
    assert_eq!(all.len(), 12);
    for name in all {
        let prog = load(&format!("{dir}/{name}.json"));
        // the two ~10 s serial problems use the parallel reference
        let big = name == "di_N10" || name == "sys3x2_N5";
        let reference = region_signature(&solve_mpqp(&prog, if big { MpqpAlgorithm::CombinatorialParallel } else { MpqpAlgorithm::Combinatorial }).critical_regions);
        assert_eq!(reference, region_signature(&solve_mpqp(&prog, MpqpAlgorithm::CombinatorialKpParallel).critical_regions), "{name}: default parallel");
        if big {
            assert_eq!(reference, region_signature(&solve_mpqp(&prog, MpqpAlgorithm::CombinatorialKp).critical_regions), "{name}: default serial");
            continue;
        }
        // forcing the classification on the larger problems is slow (pointless queries), so they
        // get the late-activation variant only
        let large = ["di_N6", "di_N8", "dinv_N6", "dinv_N8", "masses2_N6"].contains(&name.as_str());
        if large {
            assert_eq!(reference, sig(&prog, true, KpOptions { activate_at: 400, ..KpOptions::always() }), "{name}: late parallel");
            continue;
        }
        for (v, o) in variants() {
            assert_eq!(reference, sig(&prog, false, o), "{name}: {v}");
        }
        assert_eq!(reference, sig(&prog, true, KpOptions::always()), "{name}: lazy parallel");
        assert_eq!(reference, sig(&prog, true, KpOptions::paper()), "{name}: eager parallel");
    }
}

#[test]
fn kp_matches_combinatorial_on_small_mplps() {
    let dir = format!("{}/bench/problems_lp", env!("CARGO_MANIFEST_DIR"));
    for name in ["diinf_N2", "diinf_N3", "dione_N2", "masses2inf_N2"] {
        let prog = load(&format!("{dir}/{name}.json"));
        let reference = region_signature(&solve_mpqp(&prog, MpqpAlgorithm::Combinatorial).critical_regions);
        assert_eq!(reference, region_signature(&solve_mpqp(&prog, MpqpAlgorithm::CombinatorialKp).critical_regions), "{name}: default");
        for (v, o) in variants() {
            assert_eq!(reference, sig(&prog, false, o), "{name}: {v}");
        }
    }
}

#[test]
fn kp_matches_combinatorial_on_random_mpqps() {
    let mut pruned = 0u64;
    for seed in 0..40u64 {
        let (x, t, m) = [(2, 1, 6), (3, 2, 8), (4, 2, 10), (3, 3, 9), (5, 2, 12), (4, 3, 14), (2, 2, 7), (6, 2, 10)][seed as usize % 8];
        let prog = generate_mpqp(x, t, m, seed);
        if prog.num_constraints() > 64 {
            continue;
        }
        let reference = region_signature(&solve_mpqp(&Arc::new(prog.clone()), MpqpAlgorithm::Combinatorial).critical_regions);
        for (v, o) in variants() {
            let (r, st) = solve_with_stats(&prog, false, o);
            assert_eq!(reference, region_signature(&r), "seed {seed} ({x},{t},{m}): {v}");
            pruned += st.pruned_e + st.pruned_w + st.pruned_parent;
        }
        assert_eq!(reference, sig(&prog, true, KpOptions::always()), "seed {seed}: parallel");
    }
    assert!(pruned > 0, "the classification pruned nothing on 40 random problems: the test is vacuous");
}

#[test]
fn kp_matches_combinatorial_on_random_mplps() {
    for seed in 0..12u64 {
        let (x, t, m) = [(2, 1, 6), (3, 2, 8), (3, 2, 10), (4, 2, 10)][seed as usize % 4];
        let prog = generate_mplp(x, t, m, seed);
        let reference = region_signature(&solve_mpqp(&Arc::new(prog.clone()), MpqpAlgorithm::Combinatorial).critical_regions);
        for (v, o) in variants() {
            assert_eq!(reference, sig(&prog, false, o), "mplp seed {seed}: {v}");
        }
    }
}

/// Every region of `combinatorial` must lie inside E and contain no pair of W, whatever the
/// verdicts were obtained with (this is the safety property of the pruning).
fn check_consistent(name: &str, prog: &MpProgram) {
    let reference = solve_mpqp(&Arc::new(prog.clone()), MpqpAlgorithm::Combinatorial);
    for pairs in [Pairs::Eager, Pairs::Lazy] {
        let (e, w) = classification(prog, pairs);
        for r in &reference.critical_regions {
            for &i in &r.active_set {
                assert!(e.contains(&i), "{name} {pairs:?}: constraint {i} of region {:?} excluded from E", r.active_set);
            }
            for &(i, j) in &w {
                assert!(!(r.active_set.contains(&i) && r.active_set.contains(&j)), "{name} {pairs:?}: pair ({i},{j}) in W lies in region {:?}", r.active_set);
            }
        }
    }
}

#[test]
fn classification_is_consistent_with_every_region() {
    let dir = format!("{}/bench/problems", env!("CARGO_MANIFEST_DIR"));
    for name in ["di_doc_N3", "di_N4", "dinv_N4", "masses2_N4", "sys3x2_N3", "di_N6"] {
        check_consistent(name, &load(&format!("{dir}/{name}.json")));
    }
    for seed in 0..30u64 {
        let (x, t, m) = [(2, 1, 6), (3, 2, 8), (4, 2, 10), (3, 3, 9), (5, 2, 12)][seed as usize % 5];
        check_consistent(&format!("random {seed}"), &generate_mpqp(x, t, m, 100 + seed));
    }
    for seed in 0..10u64 {
        check_consistent(&format!("mplp {seed}"), &generate_mplp(3, 2, 9, seed));
    }
}

/// The classification must find something to prune on a problem with known structure, and
/// `combinatorial_kp` must reduce the sets it tests.
#[test]
fn classification_prunes() {
    let dir = format!("{}/bench/problems", env!("CARGO_MANIFEST_DIR"));
    let prog = load(&format!("{dir}/sys3x2_N3.json"));
    let (_, plain) = solve_with_stats(&prog, false, KpOptions::plain());
    let (_, kp) = solve_with_stats(&prog, false, KpOptions::always());
    assert!(kp.w_pairs > 0);
    assert!(kp.tested < plain.tested, "tested {} vs {}", kp.tested, plain.tested);
    assert_eq!(kp.regions, plain.regions);
}
