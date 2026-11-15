//! For each mpLP: size, number of subsets, dual lex-feasible bases, regions of the enumeration path, and
//! the times of the enumeration pass, the enumeration path and the walk.
//!
//!   enum_stats <problem.json>...
use rpopt::algorithms::mplp_lex;
use rpopt::program::MpProgram;
use std::time::Instant;

fn main() {
    mplp_lex::set_mode(1);
    for f in std::env::args().skip(1) {
        let p = MpProgram::from_json_file(std::path::Path::new(&f)).unwrap();
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        if !mplp_lex::enumeration_applies(&p) {
            println!("{name}: enumeration does not apply");
            continue;
        }
        let subsets = mplp_lex::binom(p.num_constraints(), p.num_x() - p.num_equality_constraints());
        if subsets > 3e7 {
            println!("{name}: m {} nx {} nt {} subsets {subsets:.2e} (too many, not enumerated)", p.num_constraints(), p.num_x(), p.num_t());
            continue;
        }
        let t = Instant::now();
        let (bases, visited) = mplp_lex::dual_lex_feasible_bases(&p);
        let t_pass = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        let re = mplp_lex::enumerate_regions(&p);
        let t_enum = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        let rw = mplp_lex::regions(&p, 0);
        let t_walk = t.elapsed().as_secs_f64() * 1e3;
        if std::env::var("ENUM_DIFF").is_ok() {
            let key = |r: &rpopt::critical_region::CriticalRegion| {
                let mut a = r.active_set.clone();
                a.sort_unstable();
                a
            };
            let wk: std::collections::HashSet<Vec<usize>> = rw.iter().map(key).collect();
            let ek: std::collections::HashSet<Vec<usize>> = re.iter().map(key).collect();
            for r in &re {
                if !wk.contains(&key(r)) {
                    println!("  only enumeration: {:?} radius {:?} rows {}", key(r), r.cheb_radius, r.e.rows);
                    if let Some((c, _)) = rpopt::utils::chebyshev_center(&r.e, &r.f) {
                        let inside = |o: &rpopt::critical_region::CriticalRegion| (0..o.e.rows).all(|k| rpopt::linalg::dot(o.e.row(k), &c) <= o.f[k] + 1e-9);
                        println!("    centre in {} walk regions, {} enumeration regions", rw.iter().filter(|o| inside(o)).count(), re.iter().filter(|o| inside(o)).count());
                    }
                }
            }
            for r in &rw {
                if !ek.contains(&key(r)) {
                    println!("  only walk: {:?} radius {:?}", key(r), r.cheb_radius);
                }
            }
        }
        println!(
            "{name} m {} nx {} nt {} visited {visited} dual_feasible {} regions_enum {} regions_walk {} pass_ms {t_pass:.2} enum_ms {t_enum:.2} walk_ms {t_walk:.2}",
            p.num_constraints(),
            p.num_x(),
            p.num_t(),
            bases.len(),
            re.len(),
            rw.len()
        );
    }
}
