//! The rows of every region of `mplcp` with the chained bounding-box LPs against the rows of the same region with the shared
//! builder (`set_box_chain(false)`): for the regions with the same active set, the rows (unit normal and offset, matched to 1e-7)
//! that only the shared builder kept (a row the chain screened out although it bounds the region: expected 0) and the rows only
//! the chain kept (rows that graze the region, kept because the widened box could not reject them).
//!
//!   box_rows_diff <problem.json>...
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::critical_region::CriticalRegion;
use rpopt::program::MpProgram;
use std::collections::HashMap;
use std::sync::Arc;

fn has(r: &CriticalRegion, e: &[f64], f: f64) -> bool {
    (0..r.e.rows).any(|k| (r.f[k] - f).abs() <= 1e-7 * (1.0 + f.abs()) && r.e.row(k).iter().zip(e).all(|(a, b)| (a - b).abs() <= 1e-7))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    for f in &args[1..] {
        let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(f)).unwrap());
        rpopt::algorithms::mplcp::set_box_chain(true);
        let on = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
        rpopt::algorithms::mplcp::set_box_chain(false);
        let off = solve_mpqp(&p, MpqpAlgorithm::Mplcp);
        let key = |r: &CriticalRegion| {
            let mut a = r.active_set.clone();
            a.sort_unstable();
            a
        };
        let map: HashMap<Vec<usize>, &CriticalRegion> = on.critical_regions.iter().map(|r| (key(r), r)).collect();
        let (mut both, mut only_off_regions, mut dropped, mut extra) = (0usize, 0usize, 0usize, 0usize);
        for r in off.critical_regions.iter() {
            match map.get(&key(r)) {
                None => only_off_regions += 1,
                Some(c) => {
                    both += 1;
                    for k in 0..r.e.rows {
                        if !has(c, r.e.row(k), r.f[k]) {
                            dropped += 1;
                        }
                    }
                    for k in 0..c.e.rows {
                        if !has(r, c.e.row(k), c.f[k]) {
                            extra += 1;
                        }
                    }
                }
            }
        }
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        println!(
            "{name} regions on {} off {} compared {both} only_in_off_run {only_off_regions} rows_dropped_by_chain {dropped} rows_only_in_chain {extra}",
            on.critical_regions.len(),
            off.critical_regions.len()
        );
    }
}
