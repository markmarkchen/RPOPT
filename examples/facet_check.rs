//! For a facet of a region, the candidate neighbour sets from <sets.txt>: does each border the facet in a (d-1)-dimensional piece, and is it in the
//! solve? The facet is where the slack of constraint <c> vanishes under the law of the parent region (active set <parent>).
//!
//!   facet_check <problem.json> "<parent: i,j,k>" <c> <sets.txt>
//!
//! Prints, per set: the rows of its region, the Chebyshev radius of the region restricted to the facet hyperplane (alone, and together with the
//! parent's rows), whether the set is a region of the current mplcp solve, and the region's Chebyshev radius in theta.
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::linalg::Mat;
use rpopt::program::MpProgram;
use std::sync::Arc;

fn parse(s: &str) -> Vec<usize> {
    s.trim_matches(|c| c == '[' || c == ']').split(',').filter_map(|t| t.trim().parse().ok()).collect()
}

/// Chebyshev radius of {e theta <= f} restricted to the hyperplane hv.theta + h0 = 0 (None when the restriction is empty).
fn radius_on(hv: &[f64], h0: f64, rows: &[(Vec<f64>, f64)]) -> Option<f64> {
    let nt = hv.len();
    let nn: f64 = hv.iter().map(|v| v * v).sum();
    let t0: Vec<f64> = hv.iter().map(|v| -h0 * v / nn).collect();
    // orthonormal basis of the complement of hv
    let mut basis: Vec<Vec<f64>> = vec![];
    for k in 0..nt {
        let mut v = vec![0.0; nt];
        v[k] = 1.0;
        let d: f64 = v.iter().zip(hv).map(|(a, b)| a * b).sum::<f64>() / nn;
        for i in 0..nt {
            v[i] -= d * hv[i];
        }
        for b in &basis {
            let d: f64 = v.iter().zip(b).map(|(a, c)| a * c).sum();
            for i in 0..nt {
                v[i] -= d * b[i];
            }
        }
        let nv: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if nv > 1e-8 && basis.len() < nt - 1 {
            basis.push(v.iter().map(|x| x / nv).collect());
        }
    }
    let m = nt - 1;
    let mut e = Mat::zeros(rows.len(), m);
    let mut f = vec![0.0; rows.len()];
    for (i, (r, rhs)) in rows.iter().enumerate() {
        let off: f64 = r.iter().zip(&t0).map(|(a, b)| a * b).sum();
        f[i] = rhs - off;
        for (k, b) in basis.iter().enumerate() {
            e[(i, k)] = r.iter().zip(b).map(|(a, c)| a * c).sum();
        }
    }
    let s = rpopt::utils::chebyshev_ball(&e, &f, &[])?;
    Some(s.x[m])
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let prog = Arc::new(MpProgram::from_json_file_opts(std::path::Path::new(&a[1]), false).unwrap());
    let parent = parse(&a[2]);
    let c: usize = a[3].parse().unwrap();
    let nt = prog.num_t();
    let nx = prog.num_x();
    let sol = solve_mpqp(&prog, MpqpAlgorithm::Mplcp);
    println!("current mplcp: {} regions", sol.critical_regions.len());
    let par = rpopt::mpqp_utils::gen_cr_from_active_set(&prog, &parent, true).expect("parent region");
    // slack of constraint c under the parent's law: s(theta) = b_c + F_c theta - a_c x(theta), the facet is s = 0
    let ac = prog.a.row(c);
    let mut hv = vec![0.0; nt];
    for t in 0..nt {
        let mut v = prog.f[(c, t)];
        for j in 0..nx {
            v -= ac[j] * par.a[(j, t)];
        }
        hv[t] = v;
    }
    let h0: f64 = prog.b[c] - (0..nx).map(|j| ac[j] * par.b[j]).sum::<f64>();
    println!("facet hyperplane of constraint {c} in the region {parent:?}: |normal| {:.3e}, offset {:.3e}", hv.iter().map(|v| v * v).sum::<f64>().sqrt(), h0);
    let prow: Vec<(Vec<f64>, f64)> = (0..par.e.rows).map(|i| (par.e.row(i).to_vec(), par.f[i])).collect();
    println!("facet of the parent (parent rows on the hyperplane): radius {:?}", radius_on(&hv, h0, &prow));
    for line in std::fs::read_to_string(&a[4]).unwrap().lines() {
        let aset = parse(line);
        if aset.is_empty() {
            continue;
        }
        let mut want = aset.clone();
        want.sort_unstable();
        let in_solve = sol.critical_regions.iter().any(|r| {
            let mut v = r.active_set.clone();
            v.sort_unstable();
            v == want
        });
        let Some(cr) = rpopt::mpqp_utils::gen_cr_from_active_set(&prog, &aset, true) else {
            println!("{aset:?}: no full-dimensional region from the active set alone; in the solve: {in_solve}");
            continue;
        };
        let rows: Vec<(Vec<f64>, f64)> = (0..cr.e.rows).map(|i| (cr.e.row(i).to_vec(), cr.f[i])).collect();
        let alone = radius_on(&hv, h0, &rows);
        let mut both = rows.clone();
        both.extend(prow.iter().cloned());
        let together = radius_on(&hv, h0, &both);
        let full = rpopt::utils::chebyshev_ball(&cr.e, &cr.f, &[]).map(|s| s.x[nt]);
        println!("{aset:?}: rows {} region radius {:?}; on the facet hyperplane alone {:?}, with the parent's rows {:?}; in the solve: {in_solve}", cr.e.rows, full, alone, together);
    }
}
