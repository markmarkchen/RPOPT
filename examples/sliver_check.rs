//! Radius distribution of the regions of two algorithms, and whether each region of the first is also
//! in the second (same sorted active set), with the region's Chebyshev radius and the number of
//! facet rows.
//!
//!   sliver_check <problem.json> <algo_a> <algo_b>
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::program::MpProgram;
use rpopt::utils::chebyshev_center;
use std::collections::HashMap;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(&args[1])).unwrap());
    let a = solve_mpqp(&p, MpqpAlgorithm::from_name(&args[2]).unwrap());
    let b = solve_mpqp(&p, MpqpAlgorithm::from_name(&args[3]).unwrap());
    let key = |r: &rpopt::critical_region::CriticalRegion| {
        let mut s = r.active_set.clone();
        s.sort_unstable();
        s
    };
    let mut bm: HashMap<Vec<usize>, (f64, usize)> = HashMap::new();
    for r in &b.critical_regions {
        let rad = chebyshev_center(&r.e, &r.f).map(|x| x.1).unwrap_or(-1.0);
        bm.insert(key(r), (rad, r.e.rows));
    }
    println!("{}: {} regions {}, {} regions {}", args[1], args[2], a.critical_regions.len(), args[3], b.critical_regions.len());
    let (mut in_b, mut thin_total, mut thin_in_b) = (0, 0, 0);
    for (i, r) in a.critical_regions.iter().enumerate() {
        let rad = chebyshev_center(&r.e, &r.f).map(|x| x.1).unwrap_or(-1.0);
        let other = bm.get(&key(r));
        if other.is_some() {
            in_b += 1;
        }
        if rad < 1e-4 {
            thin_total += 1;
            if other.is_some() {
                thin_in_b += 1;
            }
        }
        // width of the thinnest slab: min over opposite row pairs of (f_i + f_j) (rows are unit norm)
        let mut width = f64::INFINITY;
        for x in 0..r.e.rows {
            for y in x + 1..r.e.rows {
                if (0..r.e.cols).all(|c| (r.e[(x, c)] + r.e[(y, c)]).abs() < 1e-9) {
                    width = width.min(r.f[x] + r.f[y]);
                }
            }
        }
        println!("region {i:3} slab_width {width:.3e} radius {rad:.3e} rows {:3}  in_b {}", r.e.rows, other.map(|o| format!("radius {:.3e} rows {}", o.0, o.1)).unwrap_or("-".into()));
    }
    println!("in {}: {} of {}; thin (<1e-4): {} of which in {}: {}", args[3], in_b, a.critical_regions.len(), thin_total, args[3], thin_in_b);
}
