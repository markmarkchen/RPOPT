//! Region-set diff across exact mpQP algorithms: for every region in one algorithm's set and not
//! another's (by active-set signature), reports Chebyshev radius, degeneracy (LICQ failure or a
//! multiplier identically / nearly zero), coverage of its interior by the other algorithm's regions,
//! and agreement of its law with a direct QP solve.
//!
//!   regdiff <problem.json> <algo,algo,...> [samples_per_region=8]
use rand::{rngs::StdRng, Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::critical_region::CriticalRegion;
use rpopt::linalg::{dot, rank, Mat};
use rpopt::program::MpProgram;
use rpopt::solution::Solution;
use std::collections::HashMap;
use std::sync::Arc;

fn viol(r: &CriticalRegion, th: &[f64]) -> f64 {
    (0..r.e.rows).map(|k| dot(r.e.row(k), th) - r.f[k]).fold(f64::NEG_INFINITY, f64::max)
}

/// Random interior points: hit-and-run from the Chebyshev center.
fn interior_points(r: &CriticalRegion, c: &[f64], rng: &mut StdRng, n: usize) -> Vec<Vec<f64>> {
    let d = c.len();
    let mut out = vec![c.to_vec()];
    let mut cur = c.to_vec();
    for _ in 0..n {
        for _ in 0..3 {
            let dir: Vec<f64> = (0..d).map(|_| rpopt::program::standard_normal(rng)).collect();
            let (mut lo, mut hi) = (f64::NEG_INFINITY, f64::INFINITY);
            for k in 0..r.e.rows {
                let a = dot(r.e.row(k), &dir);
                let s = r.f[k] - dot(r.e.row(k), &cur);
                if a > 1e-14 {
                    hi = hi.min(s / a);
                } else if a < -1e-14 {
                    lo = lo.max(s / a);
                }
            }
            if !(lo.is_finite() && hi.is_finite()) || hi <= lo {
                continue;
            }
            let t = lo + (hi - lo) * rng.gen_range(0.05..0.95);
            for i in 0..d {
                cur[i] += t * dir[i];
            }
        }
        out.push(cur.clone());
    }
    out
}

fn law_ok(x: &[f64], y: &[f64]) -> bool {
    x.iter().zip(y).all(|(u, v)| (u - v).abs() <= 1e-5 + 1e-6 * v.abs())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(&args[1])).unwrap());
    let algos: Vec<&str> = args[2].split(',').collect();
    let ns: usize = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(8);
    let mut sols: Vec<(String, Solution)> = vec![];
    for a in &algos {
        let t = std::time::Instant::now();
        let s = solve_mpqp(&p, MpqpAlgorithm::from_name(a).unwrap());
        eprintln!("{a}: {} regions, {:.1}s", s.len(), t.elapsed().as_secs_f64());
        sols.push((a.to_string(), s));
    }
    let prog = sols[0].1.continuous.clone().unwrap();
    let mut rng = StdRng::seed_from_u64(1);
    for i in 0..sols.len() {
        for j in 0..sols.len() {
            if i == j {
                continue;
            }
            let (na, a) = (&sols[i].0, &sols[i].1);
            let (nb, b) = (&sols[j].0, &sols[j].1);
            let sigs_b: std::collections::HashSet<&Vec<usize>> = b.critical_regions.iter().map(|r| &r.active_set).collect();
            let only: Vec<&CriticalRegion> = a.critical_regions.iter().filter(|r| !sigs_b.contains(&r.active_set)).collect();
            let mut cat: HashMap<String, usize> = HashMap::new();
            let mut radii: Vec<f64> = vec![];
            for r in &only {
                let Some((c, rad)) = r.chebyshev() else {
                    *cat.entry("empty".into()).or_default() += 1;
                    continue;
                };
                radii.push(rad);
                let m = r.active_set.len();
                let rows = Mat { rows: m, cols: prog.a.cols, data: r.active_set.iter().flat_map(|&k| prog.a.row(k).to_vec()).collect() };
                let licq_fail = m > 0 && rank(&rows) < m;
                let lam_c = r.lagrange_multipliers(&c);
                let lam_zero = (0..r.c.rows).any(|k| r.c.row(k).iter().all(|v| v.abs() < 1e-9) && r.d[k].abs() < 1e-9) || lam_c.iter().any(|v| v.abs() < 1e-9);
                // radius of the unreduced, unit-normalised KKT region (no redundancy screening)
                let full_r = rpopt::mpqp_utils::kkt_region(&prog, &r.active_set).and_then(|k| {
                    let keep = rpopt::utils::numerically_nonzero_rows(&k.e);
                    let mut e = k.e.select_rows(&keep);
                    let mut f: Vec<f64> = keep.iter().map(|&i| k.f[i]).collect();
                    for i in 0..e.rows {
                        let n = rpopt::linalg::norm(e.row(i));
                        for v in e.row_mut(i) {
                            *v /= n;
                        }
                        f[i] /= n;
                    }
                    rpopt::utils::chebyshev_ball(&e, &f, &[]).map(|s| s.x[s.x.len() - 1])
                });
                let pts = interior_points(r, &c, &mut rng, ns);
                let (mut cov, mut own_ok, mut other_ok) = (0, 0, 0);
                let (mut worst_dx, mut worst_inf, mut worst_gap) = (0.0f64, 0.0f64, 0.0f64);
                for th in &pts {
                    let vmin = b.critical_regions.iter().map(|q| viol(q, th)).fold(f64::INFINITY, f64::min);
                    if vmin <= 1e-7 {
                        cov += 1;
                    }
                    if let Some(s) = prog.solve_theta(th) {
                        let x = r.evaluate(th);
                        if law_ok(&x, &s.sol) {
                            own_ok += 1;
                        } else {
                            worst_dx = worst_dx.max(x.iter().zip(&s.sol).map(|(u, v)| (u - v).abs()).fold(0.0, f64::max));
                            let ax = prog.a.matvec(&x);
                            let ft = prog.f.matvec(th);
                            let inf = (0..ax.len()).map(|k| if prog.equality_indices.contains(&k) { (ax[k] - prog.b[k] - ft[k]).abs() } else { ax[k] - prog.b[k] - ft[k] }).fold(0.0, f64::max);
                            worst_inf = worst_inf.max(inf);
                            worst_gap = worst_gap.max(prog.evaluate_objective(&x, th) - s.obj);
                        }
                        if let Some(q) = b.critical_regions.iter().find(|q| viol(q, th) <= 1e-7) {
                            if law_ok(&q.evaluate(th), &s.sol) {
                                other_ok += 1;
                            }
                        }
                    }
                }
                let n = pts.len();
                let key = format!(
                    "{}|{}|{}|{}|own {}|other-cover {}",
                    if rad < 1e-6 { "thin(r<1e-6)" } else { "fulldim" },
                    match full_r { Some(fr) if fr > 1e-8 => "fullR>1e-8", Some(_) => "fullR<=1e-8", None => "fullEmpty" },
                    if licq_fail { "LICQfail" } else if lam_zero { "lam0" } else { "nondegenerate" },
                    if cov == n { "covered" } else if cov == 0 { "UNCOVERED" } else { "partly" },
                    if own_ok == n { "lawOK" } else { "LAWBAD" },
                    if other_ok == cov { "lawOK" } else { "LAWBAD" },
                );
                if cov == 0 && rad >= 1e-6 {
                    println!("      UNCOVERED r={rad:.2e} active_set={:?} licq_fail={licq_fail} lam_zero={lam_zero} center={:?}", r.active_set, c);
                }
                if own_ok != n && std::env::var("VERBOSE").is_ok() {
                    println!("      LAWBAD r={rad:.2e} |A|={} dx={worst_dx:.2e} infeas={worst_inf:.2e} objgap={worst_gap:.2e}", r.active_set.len());
                }
                *cat.entry(key).or_default() += 1;
            }
            radii.sort_by(|x, y| x.partial_cmp(y).unwrap());
            let q = |f: f64| radii.get(((radii.len() as f64 - 1.0) * f) as usize).copied().unwrap_or(f64::NAN);
            println!("{na} \\ {nb}: {} of {} regions; radius min/med/max = {:.2e}/{:.2e}/{:.2e}", only.len(), a.len(), q(0.0), q(0.5), q(1.0));
            let mut v: Vec<_> = cat.into_iter().collect();
            v.sort();
            for (k, c) in v {
                println!("    {c:6}  {k}");
            }
        }
    }
}
