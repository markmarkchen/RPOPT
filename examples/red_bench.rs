//! Redundancy-removal benchmark: red_bench <problem.json>.. --algos a,b [--dump out.txt] [--reps N]
//! Prints per run: regions, total LP calls, redundancy-test LP calls, median time. With --dump,
//! writes the sorted normalized irredundant rows of every region.
use rpopt::algorithms::{solve_mpqp_with, MpqpAlgorithm, SolveOptions};
use rpopt::program::MpProgram;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut files = vec![];
    let mut algos = vec![];
    let mut dump: Option<String> = None;
    let mut reps = 5usize;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--algos" => {
                algos = args[i + 1].split(',').map(|s| MpqpAlgorithm::from_name(s).unwrap()).collect();
                i += 1;
            }
            "--dump" => {
                dump = Some(args[i + 1].clone());
                i += 1;
            }
            "--reps" => {
                reps = args[i + 1].parse().unwrap();
                i += 1;
            }
            f => files.push(f.to_string()),
        }
        i += 1;
    }
    let mut out = dump.map(|d| std::io::BufWriter::new(std::fs::File::create(d).unwrap()));
    for f in &files {
        let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(f)).unwrap());
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        for &algo in &algos {
            let opts = SolveOptions::default();
            let (l0, _) = rpopt::stats::snapshot();
            let c0 = rpopt::solver::lp::LP_CALLS.load();
            let r0 = rpopt::mpqp_utils::RED_LPS.load();
            let g0 = rpopt::mpqp_utils::RED_REGIONS.load();
            let mut sol = solve_mpqp_with(&prog, algo, &opts);
            let (l1, _) = rpopt::stats::snapshot();
            let c1 = rpopt::solver::lp::LP_CALLS.load();
            let r1 = rpopt::mpqp_utils::RED_LPS.load();
            let g1 = rpopt::mpqp_utils::RED_REGIONS.load();
            let mut times = vec![];
            for _ in 0..reps {
                let t = Instant::now();
                sol = solve_mpqp_with(&prog, algo, &opts);
                times.push(t.elapsed().as_secs_f64());
            }
            times.sort_by(|a, b| a.partial_cmp(b).unwrap());
            println!(
                "{name} {} regions={} ppopt_lps={} lp_calls={} red_lps={} red_regions={} median_ms={:.2}",
                algo.name(),
                sol.critical_regions.len(),
                l1 - l0,
                c1 - c0,
                r1 - r0,
                g1 - g0,
                times[times.len() / 2] * 1e3
            );
            if let Some(w) = out.as_mut() {
                let mut regs: Vec<(Vec<usize>, Vec<Vec<i64>>)> = sol
                    .critical_regions
                    .iter()
                    .map(|r| {
                        let mut rows: Vec<Vec<i64>> = (0..r.e.rows)
                            .map(|k| {
                                let n = rpopt::linalg::norm(r.e.row(k));
                                let mut v: Vec<i64> = r.e.row(k).iter().map(|x| (x / n * 1e9).round() as i64).collect();
                                v.push((r.f[k] / n * 1e9).round() as i64);
                                v
                            })
                            .collect();
                        rows.sort();
                        (r.active_set.clone(), rows)
                    })
                    .collect();
                regs.sort();
                writeln!(w, "## {name} {} {}", algo.name(), regs.len()).unwrap();
                for (a, rows) in &regs {
                    writeln!(w, "R {:?}", a).unwrap();
                    for r in rows {
                        writeln!(w, "  {:?}", r).unwrap();
                    }
                }
            }
        }
    }
}
