//! The facet certificate of `mplcp` (a known single neighbour across a facet, see `certify`).
//!
//! The mode is a process-wide switch, so this file holds one test (own process).

use rpopt::algorithms::mplcp::{certificate_audit, set_certificate};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::path::Path;
use std::sync::Arc;

fn load(dir: &str, name: &str) -> Arc<MpProgram> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir).join(format!("{name}.json"));
    Arc::new(MpProgram::from_json_file(&path).unwrap())
}

fn sets(p: &Arc<MpProgram>, alg: MpqpAlgorithm) -> Vec<Vec<usize>> {
    solve_mpqp(p, alg).critical_regions.iter().map(|r| r.active_set.clone()).collect()
}

/// With the certificate on, the regions are the same set as with it off (a skipped facet only
/// had candidates that are found anyway), the wave-parallel method returns them in the serial
/// order, and the audit finds certified facets.
#[test]
fn certificate_changes_no_region() {
    let probs = [("bench/problems", "sys3x2_N5"), ("bench/problems", "masses2_N6"), ("bench/problems_large", "masses3_N3")];
    let mut certified_total = 0;
    for (dir, name) in probs {
        let p = load(dir, name);
        set_certificate(0);
        let mut off = sets(&p, MpqpAlgorithm::Mplcp);
        set_certificate(3);
        let on = sets(&p, MpqpAlgorithm::Mplcp);
        let mut on_sorted = on.clone();
        on_sorted.sort();
        off.sort();
        assert_eq!(on_sorted, off, "{name}: skipping certified facets changed the set of regions");
        let pool = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
        let par = pool.install(|| sets(&p, MpqpAlgorithm::MplcpParallel));
        assert_eq!(par, on, "{name}: parallel order differs from serial with the certificate");
        let (c0, _) = certificate_audit();
        set_certificate(4);
        let mut audited = sets(&p, MpqpAlgorithm::Mplcp);
        audited.sort();
        assert_eq!(audited, off, "{name}: audit mode changed the regions");
        certified_total += certificate_audit().0 - c0;
    }
    set_certificate(1); // the default mode
    assert!(certified_total > 0, "no facet was ever certified");
}
