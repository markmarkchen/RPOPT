//! Reports the size of every benchmark problem: type, number of parameters (theta), variables (x),
//! constraints before and after preprocessing, and the number of critical regions.
//!
//! Read-only: it parses the problem JSON files and builds each `MpProgram` (constraint preprocessing only,
//! nothing is solved). Region counts come from the solution archive manifest, one row per instance:
//! `bench/results/solution_archive_manifest_<commit>.csv`.
//!
//! Usage (from the repository root):
//!   cargo run --profile gate --example problem_sizes -- [bench_dir] [manifest.csv] [out_prefix]
//! Defaults: `bench`, the newest `bench/results/solution_archive_manifest_*.csv` with 200 or more rows, and
//! `bench/results/problem_sizes` (writes `<out_prefix>.md` and `<out_prefix>.csv`).
//!
//! "Raw" counts are the rows in the JSON file. "Processed" counts are what the solvers see: `MpProgram` merges
//! duplicate rows and drops redundant ones when it loads a problem (`process_constraints`), so di_N4 has 24 raw
//! and 18 processed constraints. The result files of `ppopt_bench` (`<name>/info`) record the processed count.

use rpopt::program::{MpProgram, ProblemData};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The five problem directories, in report order.
const SUITES: [&str; 5] = ["problems", "problems_highdim", "problems_large", "problems_lp", "problems_lp_highdim"];

struct Row {
    suite: &'static str,
    name: String,
    kind: &'static str,
    theta: usize,
    x: usize,
    m_raw: usize,
    m: usize,
    eq_raw: usize,
    eq: usize,
    at_raw: usize,
    at: usize,
    /// (regions, solver, status) from the archive manifest, if the instance is listed
    regions: Option<(usize, String, String)>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bench = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("bench"));
    let manifest = args.get(2).map(PathBuf::from).or_else(|| newest_manifest(&bench.join("results")));
    let out_prefix = PathBuf::from(args.get(3).map(String::as_str).unwrap_or("bench/results/problem_sizes"));

    let regions = manifest.as_ref().map(|p| read_manifest(p)).unwrap_or_default();
    match &manifest {
        Some(p) => println!("regions from {} ({} instances)", p.display(), regions.len()),
        None => println!("no archive manifest found: the regions column will be empty"),
    }

    let mut rows: Vec<Row> = vec![];
    let mut failed: Vec<String> = vec![];
    for suite in SUITES {
        let dir = bench.join(suite);
        let mut files: Vec<PathBuf> = match std::fs::read_dir(&dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
                .filter(|p| !p.to_string_lossy().ends_with(".model.json"))
                .collect(),
            Err(e) => {
                failed.push(format!("{}: {e}", dir.display()));
                continue;
            }
        };
        files.sort_by_key(|p| natural_key(&p.file_stem().unwrap().to_string_lossy()));
        for path in files {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            match size_row(suite, &name, &path, &regions) {
                Ok(r) => rows.push(r),
                Err(e) => failed.push(format!("{suite}/{name}: {e}")),
            }
        }
        println!("{suite}: {} problems", rows.iter().filter(|r| r.suite == suite).count());
    }

    std::fs::write(out_prefix.with_extension("csv"), csv(&rows)).expect("write csv");
    std::fs::write(out_prefix.with_extension("md"), markdown(&rows, manifest.as_deref())).expect("write md");
    println!("wrote {}.md and {}.csv ({} problems)", out_prefix.display(), out_prefix.display(), rows.len());
    if !failed.is_empty() {
        eprintln!("FAILED to read {} problem(s):", failed.len());
        for f in &failed {
            eprintln!("  {f}");
        }
        std::process::exit(1);
    }
}

fn size_row(suite: &'static str, name: &str, path: &Path, regions: &HashMap<(String, String), (usize, String, String)>) -> Result<Row, String> {
    let txt = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let d: ProblemData = serde_json::from_str(&txt).map_err(|e| e.to_string())?;
    let prog = MpProgram::from_problem_data_opts(&d, true);
    Ok(Row {
        suite,
        name: name.to_string(),
        kind: if d.q.is_some() { "mpQP" } else { "mpLP" },
        theta: prog.num_t(),
        x: prog.num_x(),
        m_raw: d.a.len(),
        m: prog.num_constraints(),
        eq_raw: d.equality_indices.len(),
        eq: prog.num_equality_constraints(),
        at_raw: d.a_t.len(),
        at: prog.a_t.rows,
        regions: regions.get(&(suite.to_string(), name.to_string())).cloned(),
    })
}

/// "Natural" sort key: digit runs compare as numbers (masses2_N4 before masses2_N10).
fn natural_key(s: &str) -> Vec<(u8, u64, String)> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut digit = false;
    let flush = |cur: &mut String, digit: bool, out: &mut Vec<(u8, u64, String)>| {
        if cur.is_empty() {
            return;
        }
        if digit {
            out.push((0, cur.parse().unwrap_or(0), String::new()));
        } else {
            out.push((1, 0, cur.clone()));
        }
        cur.clear();
    };
    for ch in s.chars() {
        if ch.is_ascii_digit() != digit {
            flush(&mut cur, digit, &mut out);
            digit = ch.is_ascii_digit();
        }
        cur.push(ch);
    }
    flush(&mut cur, digit, &mut out);
    out
}

/// The newest archive manifest that covers (nearly) every instance; a partial manifest is skipped.
fn newest_manifest(results: &Path) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for e in std::fs::read_dir(results).ok()?.filter_map(|e| e.ok()) {
        let p = e.path();
        let n = p.file_name()?.to_string_lossy().to_string();
        if !(n.starts_with("solution_archive_manifest_") && n.ends_with(".csv")) {
            continue;
        }
        let lines = std::fs::read_to_string(&p).ok()?.lines().count();
        if lines < 200 {
            continue;
        }
        let t = e.metadata().ok()?.modified().ok()?;
        if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
            best = Some((t, p));
        }
    }
    best.map(|(_, p)| p)
}

/// (dir, problem) -> (regions, solver, status). The manifest has no quoted fields.
fn read_manifest(path: &Path) -> HashMap<(String, String), (usize, String, String)> {
    let mut map = HashMap::new();
    let txt = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines = txt.lines();
    let header: Vec<&str> = lines.next().unwrap_or("").split(',').collect();
    let col = |n: &str| header.iter().position(|h| *h == n);
    let (Some(cd), Some(cp), Some(cs), Some(cst), Some(cr)) = (col("dir"), col("problem"), col("solver"), col("status"), col("regions")) else {
        eprintln!("manifest {} lacks the dir/problem/solver/status/regions columns", path.display());
        return map;
    };
    for l in lines {
        let f: Vec<&str> = l.split(',').collect();
        if f.len() <= cd.max(cp).max(cs).max(cst).max(cr) {
            continue;
        }
        if let Ok(r) = f[cr].parse::<usize>() {
            map.insert((f[cd].to_string(), f[cp].to_string()), (r, f[cs].to_string(), f[cst].to_string()));
        }
    }
    map
}

fn csv(rows: &[Row]) -> String {
    let mut s = String::from("suite,problem,type,theta,x,constraints_raw,constraints,equalities_raw,equalities,theta_constraints_raw,theta_constraints,regions,region_solver,region_status\n");
    for r in rows {
        let (reg, sol, st) = match &r.regions {
            Some((n, sol, st)) => (n.to_string(), sol.as_str(), st.as_str()),
            None => (String::new(), "", ""),
        };
        let _ = writeln!(s, "{},{},{},{},{},{},{},{},{},{},{},{},{},{}", r.suite, r.name, r.kind, r.theta, r.x, r.m_raw, r.m, r.eq_raw, r.eq, r.at_raw, r.at, reg, sol, st);
    }
    s
}

/// "24 -> 18" when preprocessing changed the count, otherwise just the number.
fn raw_to(raw: usize, proc_: usize) -> String {
    if raw == proc_ {
        raw.to_string()
    } else {
        format!("{raw} → {proc_}")
    }
}

fn range(it: impl Iterator<Item = usize>) -> String {
    let v: Vec<usize> = it.collect();
    match (v.iter().min(), v.iter().max()) {
        (Some(a), Some(b)) if a == b => a.to_string(),
        (Some(a), Some(b)) => format!("{a} to {b}"),
        _ => "-".into(),
    }
}

fn markdown(rows: &[Row], manifest: Option<&Path>) -> String {
    let mut s = String::new();
    s.push_str("# Benchmark problem sizes\n\n");
    s.push_str("Generated by `cargo run --profile gate --example problem_sizes` (`examples/problem_sizes.rs`); nothing is solved or timed.\n\n");
    s.push_str("- **theta**: parameters. **x**: variables (the same before and after preprocessing).\n");
    s.push_str("- **constraints**: rows of `A x <= b + F theta` (equalities included). `a → b` means the JSON file has `a` rows and the solvers see `b`: ");
    s.push_str("loading a problem merges duplicate rows and drops redundant ones. The result files of `ppopt_bench` (`<name>/info`) record the processed count.\n");
    s.push_str("- **theta rows**: rows of the parameter-space constraints `A_t theta <= b_t`, same notation.\n");
    s.push_str("- **regions**: critical regions of the archived solution of the solver in the last column, from ");
    match manifest {
        Some(p) => {
            let _ = write!(s, "`{}`", p.file_name().unwrap().to_string_lossy());
        }
        None => s.push_str("(no manifest found)"),
    }
    s.push_str(". For the degenerate mpLPs the partition is not unique, so another valid solver can report a different count.\n\n");

    s.push_str("## Summary by suite\n\n");
    s.push_str("| suite | type | problems | theta | x | constraints | regions |\n|---|---|---:|---:|---:|---:|---:|\n");
    for suite in SUITES {
        let rs: Vec<&Row> = rows.iter().filter(|r| r.suite == suite).collect();
        if rs.is_empty() {
            continue;
        }
        let kinds: std::collections::BTreeSet<&str> = rs.iter().map(|r| r.kind).collect();
        let reg = range(rs.iter().filter_map(|r| r.regions.as_ref().map(|g| g.0)));
        let _ = writeln!(
            s,
            "| `{suite}` | {} | {} | {} | {} | {} | {reg} |",
            kinds.into_iter().collect::<Vec<_>>().join(", "),
            rs.len(),
            range(rs.iter().map(|r| r.theta)),
            range(rs.iter().map(|r| r.x)),
            range(rs.iter().map(|r| r.m))
        );
    }
    for suite in SUITES {
        let rs: Vec<&Row> = rows.iter().filter(|r| r.suite == suite).collect();
        if rs.is_empty() {
            continue;
        }
        let _ = write!(s, "\n## `{suite}` ({} problems)\n\n", rs.len());
        s.push_str("| problem | type | theta | x | constraints | equalities | theta rows | regions | solver |\n|---|---|---:|---:|---:|---:|---:|---:|---|\n");
        for r in rs {
            let (reg, sol) = match &r.regions {
                Some((n, sol, st)) if st == "ok" => (n.to_string(), sol.clone()),
                Some((n, sol, st)) => (format!("{n} ({st})"), sol.clone()),
                None => ("-".into(), "-".into()),
            };
            let _ = writeln!(
                s,
                "| {} | {} | {} | {} | {} | {} | {} | {reg} | {sol} |",
                r.name,
                r.kind,
                r.theta,
                r.x,
                raw_to(r.m_raw, r.m),
                raw_to(r.eq_raw, r.eq),
                raw_to(r.at_raw, r.at)
            );
        }
    }
    s
}
