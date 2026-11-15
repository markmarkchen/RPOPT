//! Measures what removing duplicated constraints (rows of [A | F | b] that are positive multiples of another row) before the
//! solve would do to mplcp: the number of duplicate rows, the regions of the solve with and without them, and whether the
//! sorted active sets agree once every twin is replaced by the kept row of its group.
//!
//!   dedupe_check <problem.json>...
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::linalg::Mat;
use rpopt::program::MpProgram;
use std::collections::BTreeSet;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    for f in &args[1..] {
        let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(f)).unwrap());
        let m = p.a.rows;
        let nt = p.f.cols;
        let rows: Vec<Vec<f64>> = (0..m)
            .map(|i| {
                let mut v = p.a.row(i).to_vec();
                v.extend_from_slice(p.f.row(i));
                v.push(p.b[i]);
                v
            })
            .collect();
        let norm2: Vec<f64> = rows.iter().map(|v| v[..v.len() - 1].iter().map(|x| x * x).sum::<f64>()).collect();
        // group representative: the lowest index of the group (inequality rows only)
        let mut rep: Vec<usize> = (0..m).collect();
        for i in 0..m {
            if p.equality_indices.contains(&i) || rep[i] != i {
                continue;
            }
            for j in i + 1..m {
                if p.equality_indices.contains(&j) || rep[j] != j || !(norm2[i] > 0.0 && norm2[j] > 0.0) {
                    continue;
                }
                let (si, sj) = (norm2[i].sqrt(), norm2[j].sqrt());
                let c = rows[i][..rows[i].len() - 1].iter().zip(&rows[j]).map(|(a, b)| a * b).sum::<f64>() / norm2[j];
                if c > 0.0 && rows[i].iter().zip(&rows[j]).all(|(a, b)| (a - c * b).abs() <= 1e-9 * si.max(sj) * (1.0 + 0.0)) {
                    rep[j] = i;
                }
            }
        }
        let kept: Vec<usize> = (0..m).filter(|&i| rep[i] == i).collect();
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        let dups = m - kept.len();
        let t0 = std::time::Instant::now();
        let full = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
        let t_full = t0.elapsed().as_secs_f64();
        if dups == 0 {
            println!("{name} rows {m} duplicates 0 regions {} solve_s {t_full:.2}", full.critical_regions.len());
            continue;
        }
        let new_idx: Vec<usize> = {
            let mut v = vec![usize::MAX; m];
            for (k, &i) in kept.iter().enumerate() {
                v[i] = k;
            }
            v
        };
        let a = Mat::from_rows(&kept.iter().map(|&i| p.a.row(i).to_vec()).collect::<Vec<_>>());
        let b: Vec<f64> = kept.iter().map(|&i| p.b[i]).collect();
        let fm = Mat::from_rows(&kept.iter().map(|&i| p.f.row(i).to_vec()).collect::<Vec<_>>());
        let eq: Vec<usize> = p.equality_indices.iter().map(|&i| new_idx[i]).collect();
        let q = p.q.clone().expect("mpQP");
        let red = Arc::new(MpProgram::new(a, b, p.c.clone(), p.h.clone(), Some(q), p.a_t.clone(), p.b_t.clone(), fm, eq, false));
        let _ = nt;
        let t1 = std::time::Instant::now();
        let reduced = solve_mpqp(&red, MpqpAlgorithm::Mplcp);
        let t_red = t1.elapsed().as_secs_f64();
        let canon_full: BTreeSet<Vec<usize>> = full
            .critical_regions
            .iter()
            .map(|r| {
                let mut s: Vec<usize> = r.active_set.iter().map(|&i| rep[i]).collect();
                s.sort_unstable();
                s.dedup();
                s
            })
            .collect();
        let canon_red: BTreeSet<Vec<usize>> = reduced
            .critical_regions
            .iter()
            .map(|r| {
                let mut s: Vec<usize> = r.active_set.iter().map(|&k| kept[k]).collect();
                s.sort_unstable();
                s
            })
            .collect();
        // regions whose active set contains both of a twin pair (an unmerged double count)
        let both = full.critical_regions.iter().filter(|r| r.active_set.iter().any(|&i| rep[i] != i && r.active_set.contains(&rep[i]))).count();
        println!(
            "{name} rows {m} duplicates {dups} regions full {} reduced {} canonical sets full {} reduced {} only_full {} only_reduced {} regions_with_both_twins {both} solve_s full {t_full:.2} reduced {t_red:.2}",
            full.critical_regions.len(),
            reduced.critical_regions.len(),
            canon_full.len(),
            canon_red.len(),
            canon_full.difference(&canon_red).count(),
            canon_red.difference(&canon_full).count()
        );
    }
}
