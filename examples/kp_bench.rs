//! kp_bench <reps> <modes> <file.json>...
//! modes: comma list of comb (current combinatorial), lazy, eager (KP with pair classification
//! when needed / up front), base (the KP loop without classification, counts only).
//! Env: PAR=1 uses the parallel variants (thread count from RAYON_NUM_THREADS).
//! Modes alternate within a repetition; the median time is reported.
use rpopt::algorithms::combinatorial_kp::{solve_with_stats, KpOptions, KpStats};
use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use std::sync::Arc;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let reps: usize = a[1].parse().unwrap();
    let modes: Vec<&str> = a[2].split(',').collect();
    let par = std::env::var("PAR").is_ok();
    for f in &a[3..] {
        let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(f)).unwrap());
        let name = f.rsplit('/').next().unwrap().trim_end_matches(".json");
        let mut times: Vec<Vec<f64>> = vec![vec![]; modes.len()];
        let mut last: Vec<Option<(Vec<Vec<usize>>, Option<KpStats>, u64)>> = vec![None; modes.len()];
        for _ in 0..reps {
            for (k, mode) in modes.iter().enumerate() {
                let lps0 = rpopt::solver::lp::LP_CALLS.load();
                let t = std::time::Instant::now();
                let (sig, st) = match *mode {
                    "comb" => (region_signature(&solve_mpqp(&p, if par { MpqpAlgorithm::CombinatorialParallel } else { MpqpAlgorithm::Combinatorial }).critical_regions), None),
                    "auto" | "lazy" | "eager" | "base" => {
                        let o = match *mode {
                            "auto" => KpOptions::default(),
                            "lazy" => KpOptions::always(),
                            "eager" => KpOptions::paper(),
                            _ => KpOptions::plain(),
                        };
                        let (r, st) = solve_with_stats(&p, par, o);
                        (region_signature(&r), Some(st))
                    }
                    _ => panic!("mode"),
                };
                times[k].push(t.elapsed().as_secs_f64());
                last[k] = Some((sig, st, rpopt::solver::lp::LP_CALLS.load() - lps0));
            }
        }
        let ref_sig = last[0].as_ref().unwrap().0.clone();
        for (k, mode) in modes.iter().enumerate() {
            let mut t = times[k].clone();
            t.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let med = t[t.len() / 2];
            let (sig, st, lps) = last[k].as_ref().unwrap();
            let same = if *sig == ref_sig { "same" } else { "DIFF" };
            print!("{name:<12} {mode:<6} med {med:>9.4}s min {:>9.4}s regions {:>5} {same} lps {lps:>8}", t[0], sig.len());
            if let Some(s) = st {
                print!(" | m {} E {} W {} cls_lps {} q {} unk {}/{} cls_s {:.3} | gen {} size {} E {} W {} list {} par {} tested {} feas {} | on {:?} off {:?}", s.m, s.e_size, s.w_pairs, s.class_lps, s.class_queries, s.class_unknown, s.class_resolved, s.class_secs, s.generated, s.pruned_size, s.pruned_e, s.pruned_w, s.pruned_list, s.pruned_parent, s.tested, s.feasible, s.on_level, s.off_level);
            }
            println!();
        }
    }
}
