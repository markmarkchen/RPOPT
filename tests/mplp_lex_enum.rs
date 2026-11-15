//! The enumeration path of `mplp_lex` against its walk: same regions (active sets, facet rows).
//!
//! Own test binary: it switches the process-wide path selection (`mplp_lex::set_mode`).
use rpopt::algorithms::mplp_lex;
use rpopt::problem_generator::generate_mplp;
use rpopt::program::MpProgram;
use std::collections::BTreeMap;
use std::path::Path;

fn signature(regions: &[rpopt::critical_region::CriticalRegion]) -> BTreeMap<Vec<usize>, (usize, Vec<i64>)> {
    regions
        .iter()
        .map(|r| {
            let mut a = r.active_set.clone();
            a.sort_unstable();
            let mut v = vec![];
            for k in 0..r.e.rows {
                v.extend(r.e.row(k).iter().map(|x| (x * 1e9).round() as i64));
                v.push((r.f[k] * 1e9).round() as i64);
            }
            (a, (r.e.rows, v))
        })
        .collect()
}

fn compare(p: &MpProgram, label: &str) {
    mplp_lex::set_mode(1);
    let walk = mplp_lex::solve(p, 0).unwrap();
    let en = mplp_lex::enumerate_regions(p);
    assert_eq!(walk.len(), en.len(), "{label}: region counts");
    assert_eq!(signature(&walk), signature(&en), "{label}: regions");
}

fn random_mplps() {
    for seed in 0..6 {
        let p = generate_mplp(3, 3, 9, seed);
        assert!(mplp_lex::enumeration_applies(&p));
        compare(&p, &format!("random x3 t3 m9 seed {seed}"));
    }
}

fn degenerate_mpc() {
    // inf-norm MPC: primal and dual degenerate (zero slack rows, tied multipliers)
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("bench");
    for f in ["problems_lp_highdim/masses5inf_N1.json", "problems_lp/diinf_N2.json", "problems_lp/dione_N2.json"] {
        let p = MpProgram::from_json_file(&dir.join(f)).unwrap();
        assert!(mplp_lex::enumeration_applies(&p));
        compare(&p, f);
    }
}

fn automatic_choice() {
    mplp_lex::set_mode(0);
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("bench");
    let small = MpProgram::from_json_file(&dir.join("problems_lp_highdim/masses6inf_N1.json")).unwrap();
    assert!(mplp_lex::use_enumeration(&small));
    let large = MpProgram::from_json_file(&dir.join("problems_lp/diinf_N2.json")).unwrap();
    assert!(!mplp_lex::use_enumeration(&large));
}

/// One test function: the three parts share the process-wide mode switch.
#[test]
fn enumeration_path() {
    random_mplps();
    degenerate_mpc();
    automatic_choice();
}
