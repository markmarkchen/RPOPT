//! Classifies the differences between the active-set files old.<name>.txt and new.<name>.txt in <dir>
//! (also reads head.<name>.txt and oracle.<name>.txt if present). Every set of the union of the files is built
//! into a region once; each differing set is tested for lexico-feasibility, QP agreement at its Chebyshev centre
//! and containment of the centre in the regions of each file. Every line of every file must parse (else panic).
//! A set that cannot be rebuilt full-dimensionally is judged at the centre of the solver's own region, read from
//! <dir>/<old|new>.<name>.reg (written by audit_solve with AUDIT_DUMP); that region is not tested for overlaps
//! with unbuildable regions of the other solve.
//!
//! Classes: a_lost, b_gained (genuine; "_unbuildable" if judged at the solver's centre), d_dup (the same region
//! under another set), and the spurious ones: c1_overlap (the centre lies inside another region of the set's own
//! solve; depth and both radii are printed), c2_qp_other (the QP returns another set), c3_qp_infeasible,
//! c4_unusable (solver radius zero or no dump entry).
//!   set_audit <problem.json> <dir> <name>
use rpopt::algorithms::mplcp::{lex_diagnostic, tight_diagnostic};
use rpopt::program::MpProgram;
use std::collections::BTreeSet;
use std::sync::Arc;

type Set = Vec<usize>;

fn read(path: &str) -> Option<Vec<Set>> {
    let txt = std::fs::read_to_string(path).ok()?;
    let mut out = vec![];
    let mut nonempty = 0;
    for line in txt.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        nonempty += 1;
        assert!(l.starts_with('[') && l.ends_with(']'), "bad line {l:?} in {path}");
        let inner = &l[1..l.len() - 1];
        let mut s: Set = vec![];
        if !inner.trim().is_empty() {
            for t in inner.split(',') {
                s.push(t.trim().parse().unwrap_or_else(|_| panic!("unparsable token {t:?} in line {l:?} of {path}")));
            }
        }
        s.sort_unstable();
        out.push(s);
    }
    assert_eq!(out.len(), nonempty);
    Some(out)
}

/// Solver-side record of a region: (radius, centre), from <dir>/<old|new>.<name>.reg (written by audit_solve with AUDIT_DUMP).
fn read_reg(path: &str) -> std::collections::HashMap<Set, (f64, Vec<f64>)> {
    let mut m = std::collections::HashMap::new();
    let Ok(txt) = std::fs::read_to_string(path) else { return m };
    for l in txt.lines().filter(|l| !l.trim().is_empty()) {
        let mut it = l.split('\t');
        let set = read_line(it.next().unwrap());
        let r = it.next().unwrap();
        if r == "NONE" {
            continue;
        }
        let c: Vec<f64> = it.next().unwrap_or("").split(',').filter(|t| !t.is_empty()).map(|t| t.parse().unwrap()).collect();
        m.insert(set, (r.parse().unwrap(), c));
    }
    m
}

fn read_line(l: &str) -> Set {
    let l = l.trim();
    assert!(l.starts_with('[') && l.ends_with(']'), "bad line {l:?}");
    let inner = &l[1..l.len() - 1];
    let mut s: Set = vec![];
    if !inner.trim().is_empty() {
        for t in inner.split(',') {
            s.push(t.trim().parse().unwrap_or_else(|_| panic!("unparsable token {t:?} in line {l:?}")));
        }
    }
    s.sort_unstable();
    s
}

struct Diff {
    set: Set,
    side: &'static str,
    buildable: bool,
    /// a centre is available (own Chebyshev centre, or the solver's for unbuildable sets)
    has_centre: bool,
    radius: f64,
    centre: Vec<f64>,
    lex: Option<(bool, usize, f64, f64)>,
    tight: f64,
    qp_agrees: bool,
    qp_set: Option<Set>,
    inside: [Vec<Set>; 4],
    /// regions of the set's own solve (other than the set) that contain the centre: (set, depth of the centre below their boundary, their radius)
    overlaps: Vec<(Set, f64, f64)>,
}

fn qp_set_at(prog: &MpProgram, c: &[f64]) -> Option<Set> {
    prog.solve_theta(c).map(|o| {
        let mut v = o.active_set.clone();
        v.sort_unstable();
        v
    })
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let prog = Arc::new(MpProgram::from_json_file(std::path::Path::new(&a[1])).unwrap());
    let (dir, name) = (&a[2], &a[3]);
    let nt = prog.num_t();
    let names = ["old", "new", "head", "oracle"];
    let files: Vec<Option<Vec<Set>>> = names.iter().map(|n| read(&format!("{dir}/{n}.{name}.txt"))).collect();
    let sets: Vec<Option<BTreeSet<Set>>> = files.iter().map(|f| f.as_ref().map(|v| v.iter().cloned().collect())).collect();
    let (old, new) = (sets[0].as_ref().unwrap(), sets[1].as_ref().unwrap());
    println!(
        "{name} PARSED old {} new {} head {} oracle {}",
        files[0].as_ref().unwrap().len(),
        files[1].as_ref().unwrap().len(),
        files[2].as_ref().map_or(-1, |v| v.len() as i64),
        files[3].as_ref().map_or(-1, |v| v.len() as i64)
    );
    if let Some(h) = &sets[2] {
        println!("{name} HEADvsNEW only_new {} only_head {}", new.difference(h).count(), h.difference(new).count());
    }
    if let Some(o) = &sets[3] {
        println!("{name} ORACLEvsNEW only_new {} only_oracle {}", new.difference(o).count(), o.difference(new).count());
    }
    let reg_old = read_reg(&format!("{dir}/old.{name}.reg"));
    let reg_new = read_reg(&format!("{dir}/new.{name}.reg"));
    let mut diffs: Vec<Diff> = vec![];
    for (set, side) in old.difference(new).map(|s| (s, "lost")).chain(new.difference(old).map(|s| (s, "gained"))) {
        let built = rpopt::mpqp_utils::gen_cr_from_active_set(&prog, set, true).and_then(|cr| rpopt::utils::chebyshev_ball(&cr.e, &cr.f, &[]).map(|s| (cr, s)));
        let mut d = Diff { set: set.clone(), side, buildable: false, has_centre: false, radius: 0.0, centre: vec![], lex: None, tight: f64::NAN, qp_agrees: false, qp_set: None, inside: Default::default(), overlaps: vec![] };
        match built {
            Some((cr, s)) => {
                d.buildable = true;
                d.has_centre = true;
                d.centre = s.x[..nt].to_vec();
                d.radius = s.x[nt];
                d.lex = lex_diagnostic(&prog, &cr);
                let (tight, qp_agrees) = tight_diagnostic(&prog, &cr, &d.centre);
                d.tight = tight;
                d.qp_agrees = qp_agrees;
                d.qp_set = qp_set_at(&prog, &d.centre);
            }
            None => {
                let reg = if side == "lost" { &reg_old } else { &reg_new };
                if let Some((r, c)) = reg.get(set) {
                    d.radius = *r;
                    if *r > 1e-12 {
                        d.has_centre = true;
                        d.centre = c.clone();
                        d.qp_set = qp_set_at(&prog, c);
                        d.qp_agrees = d.qp_set.as_ref() == Some(set);
                    }
                }
            }
        }
        diffs.push(d);
    }
    if diffs.iter().any(|d| d.has_centre) {
        let mut union: BTreeSet<&Set> = BTreeSet::new();
        for s in sets.iter().flatten() {
            union.extend(s.iter());
        }
        for u in union {
            let Some(cr) = rpopt::mpqp_utils::gen_cr_from_active_set(&prog, u, true) else { continue };
            for d in diffs.iter_mut().filter(|d| d.has_centre) {
                if cr.is_inside(&d.centre, -1e-9) {
                    for (k, s) in sets.iter().enumerate() {
                        if s.as_ref().map_or(false, |s| s.contains(u)) {
                            d.inside[k].push(u.clone());
                        }
                    }
                    let own = if d.side == "lost" { old } else { new };
                    if own.contains(u) && *u != d.set {
                        let depth = (0..cr.e.rows)
                            .map(|k| {
                                let row = cr.e.row(k);
                                let nrm = row.iter().map(|x| x * x).sum::<f64>().sqrt();
                                (cr.f[k] - row.iter().zip(&d.centre).map(|(x, y)| x * y).sum::<f64>()) / nrm
                            })
                            .fold(f64::INFINITY, f64::min);
                        let ru = rpopt::utils::chebyshev_ball(&cr.e, &cr.f, &[]).map_or(f64::NAN, |s| s.x[nt]);
                        d.overlaps.push((u.clone(), depth, ru));
                    }
                }
            }
        }
    }
    let mut counts = std::collections::BTreeMap::new();
    for d in &diffs {
        let other = if d.side == "lost" { 1 } else { 0 };
        let own_set: &BTreeSet<Set> = if d.side == "lost" { old } else { new };
        let lacking: &BTreeSet<Set> = if d.side == "lost" { new } else { old };
        let class = if !d.has_centre {
            "c4_unusable"
        } else if !d.overlaps.is_empty() {
            "c1_overlap"
        } else if d.qp_set.is_none() {
            "c3_qp_infeasible"
        } else if !d.qp_agrees {
            if d.qp_set.as_ref().map_or(false, |q| lacking.contains(q)) {
                "d_dup"
            } else {
                "c2_qp_other"
            }
        } else if d.inside[other].iter().any(|s| !own_set.contains(s)) {
            "d_dup"
        } else if d.side == "lost" {
            if d.buildable { "a_lost" } else { "a_lost_unbuildable" }
        } else if d.buildable {
            "b_gained"
        } else {
            "b_gained_unbuildable"
        };
        *counts.entry(class).or_insert(0usize) += 1;
        let orc = sets[3].as_ref().map(|o| (o.contains(&d.set), d.inside[3].clone()));
        let hd = sets[2].as_ref().map(|h| h.contains(&d.set));
        println!(
            "{name} DIFF {class} {} {:?} radius {:.3e} lex(feas,zero_rows,min_signed,min_abs)={:?} tight {:.2e} qp_agrees {} qp_set {:?} inside_old {:?} inside_new {:?} oracle(has,inside)={:?} head_has {:?} buildable {} overlaps {:?}",
            d.side, d.set, d.radius, d.lex, d.tight, d.qp_agrees, d.qp_set, d.inside[0], d.inside[1], orc, hd, d.buildable, d.overlaps
        );
    }
    println!("{name} COUNTS {:?} total_diffs {}", counts, diffs.len());
}
