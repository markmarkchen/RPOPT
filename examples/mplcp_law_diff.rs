//! A/B check of the tableau law against the KKT-solve law in mplcp.
//!   mplcp_law_diff <files...>
//! Per problem: same regions in the same order (serial old, serial new, parallel new), signature
//! equal to geometric, and the largest difference of the law (A, b of x; C, D of lambda) and of the
//! region rows (E, f), measured as |a-b| / max(1, |a|, |b|).
use rpopt::algorithms::mplcp::{set_tableau_law, solve, Method};
use rpopt::algorithms::{region_signature, solve_mpqp, MpqpAlgorithm};
use rpopt::critical_region::CriticalRegion;
use rpopt::program::MpProgram;

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / 1.0f64.max(a.abs()).max(b.abs())
}

/// Largest relative KKT residual of the multiplier law of a region: w_AS = q + R theta + M lambda = 0.
fn residual(prog: &MpProgram, r: &CriticalRegion) -> f64 {
    let kc = prog.kkt_cache_pub().unwrap();
    let nt = r.c.cols;
    let mut worst = 0.0f64;
    for &i in &r.active_set {
        for t in 0..=nt {
            let (mut s, mut sc) = if t < nt { (-kc.base_e[(i, t)], kc.base_e[(i, t)].abs()) } else { (kc.base_f[i], kc.base_f[i].abs()) };
            for (j, &aj) in r.active_set.iter().enumerate() {
                let l = if t < nt { r.c[(j, t)] } else { r.d[j] };
                s += kc.m[(i, aj)] * l;
                sc += (kc.m[(i, aj)] * l).abs();
            }
            worst = worst.max(s.abs() / sc.max(1.0));
        }
    }
    // primal feasibility of the active rows: A_AS x(theta) = b_AS + F_AS theta
    for &i in &r.active_set {
        for t in 0..=nt {
            let (mut s, mut sc) = if t < nt { (-prog.f[(i, t)], prog.f[(i, t)].abs()) } else { (-prog.b[i], prog.b[i].abs()) };
            for k in 0..prog.a.cols {
                let x = if t < nt { r.a[(k, t)] } else { r.b[k] };
                s += prog.a[(i, k)] * x;
                sc += (prog.a[(i, k)] * x).abs();
            }
            worst = worst.max(s.abs() / sc.max(1.0));
        }
    }
    worst
}

fn main() {
    let mut worst = [0.0f64; 2];
    let nogeo = std::env::var("NOGEO").is_ok();
    for path in std::env::args().skip(1) {
        let p = std::path::Path::new(&path);
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        let prog = std::sync::Arc::new(MpProgram::from_json_file(p).unwrap());
        set_tableau_law(false);
        let old = solve(&*prog, 0, Method::Pivot).unwrap();
        set_tableau_law(true);
        let new = solve(&*prog, 0, Method::Pivot).unwrap();
        let newp = solve(&*prog, 0, Method::PivotParallel).unwrap();
        let geo_ok = nogeo || {
            let geo = region_signature(&solve_mpqp(&prog, MpqpAlgorithm::Geometric).critical_regions);
            region_signature(&new) == geo && region_signature(&old) == geo
        };
        let same_order = old.len() == new.len() && old.iter().zip(&new).all(|(a, b)| a.active_set == b.active_set);
        let par_order = newp.len() == new.len() && newp.iter().zip(&new).all(|(a, b)| a.active_set == b.active_set);
        let (mut dl, mut de, mut dx) = (0.0f64, 0.0f64, 0.0f64);
        let mut sets = true;
        let (mut ro, mut rn) = (0.0f64, 0.0f64);
        for r in &old { ro = ro.max(residual(&prog, r)); }
        for r in &new { rn = rn.max(residual(&prog, r)); }
        let mut cmp = |a: &CriticalRegion, b: &CriticalRegion| {
            if a.e.rows != b.e.rows || a.omega_set != b.omega_set || a.lambda_set != b.lambda_set || a.regular_set != b.regular_set {
                sets = false;
                return;
            }
            for i in 0..a.a.data.len() {
                dx = dx.max(rel(a.a.data[i], b.a.data[i]));
            }
            for i in 0..a.b.len() {
                dx = dx.max(rel(a.b[i], b.b[i]));
            }
            for i in 0..a.c.data.len() {
                dl = dl.max(rel(a.c.data[i], b.c.data[i]));
            }
            for i in 0..a.d.len() {
                dl = dl.max(rel(a.d[i], b.d[i]));
            }
            for i in 0..a.e.data.len() {
                de = de.max(rel(a.e.data[i], b.e.data[i]));
            }
            for i in 0..a.f.len() {
                de = de.max(rel(a.f[i], b.f[i]));
            }
            let mut ab = 0.0f64;
            let mut mag = 0.0f64;
            for i in 0..a.e.data.len() {
                ab = ab.max((a.e.data[i] - b.e.data[i]).abs());
            }
            for i in 0..a.f.len() {
                ab = ab.max((a.f[i] - b.f[i]).abs());
                mag = mag.max(a.f[i].abs());
            }
            if std::env::var("VERB").is_ok() && ab > 1e-12 {
                let lm = a.d.iter().chain(a.c.data.iter()).fold(0.0f64, |m, v| m.max(v.abs()));
                println!("  abs rows diff {:.2e} max|f| {:.1e} max|lambda coef| {:.1e} k={}", ab, mag, lm, a.active_set.len());
            }
        };
        if same_order {
            for (a, b) in old.iter().zip(&new) {
                cmp(a, b);
            }
        }
        worst[0] = worst[0].max(dx);
        worst[1] = worst[1].max(de);
        println!("{:<14} regions {:>6} order={} par_order={} geo={} index_sets={} x_maxdiff={:.2e} lambda_maxdiff={:.2e} rows_maxdiff={:.2e} resid_old={:.1e} resid_new={:.1e}", name, new.len(), same_order, par_order, geo_ok, sets, dx, dl, de, ro, rn);
    }
    println!("WORST x-law {:.3e} rows {:.3e}", worst[0], worst[1]);
}
