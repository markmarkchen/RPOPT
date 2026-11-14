//! LP solver benchmark on captured instances: records every call to the `solver::lp` entry points
//! during a real solve, then replays the calls in isolation (old vs new solver: build this example
//! at both versions and run each on the same capture files).
//!
//!   lp_replay capture <out.bin> <problem.json> <algorithm>
//!   lp_replay run <reps> <results.bin|-> <capture.bin>...
//!   lp_replay compare <old.res> <new.res> <capture.bin>...

use rpopt::linalg::Mat;
use rpopt::solver::lp::capture::{self, Call, Log, Rec};
use std::hint::black_box;
use std::io::{Read, Write};
use std::time::Instant;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn put_u32(w: &mut Vec<u8>, v: u32) {
    w.extend_from_slice(&v.to_le_bytes());
}

fn put_f64s(w: &mut Vec<u8>, v: &[f64]) {
    for x in v {
        w.extend_from_slice(&x.to_le_bytes());
    }
}

struct Rd<'a> {
    b: &'a [u8],
    p: usize,
}

impl Rd<'_> {
    fn u8(&mut self) -> u8 {
        self.p += 1;
        self.b[self.p - 1]
    }
    fn u32(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.b[self.p..self.p + 4].try_into().unwrap());
        self.p += 4;
        v
    }
    fn f64s(&mut self, n: usize) -> Vec<f64> {
        let v = (0..n).map(|i| f64::from_le_bytes(self.b[self.p + 8 * i..self.p + 8 * i + 8].try_into().unwrap())).collect();
        self.p += 8 * n;
        v
    }
}

fn save(log: &Log, path: &str) {
    let mut w = vec![];
    put_u32(&mut w, log.mats.len() as u32);
    for m in &log.mats {
        put_u32(&mut w, m.rows as u32);
        put_u32(&mut w, m.cols as u32);
        put_f64s(&mut w, &m.data);
    }
    put_u32(&mut w, log.vecs.len() as u32);
    for v in &log.vecs {
        put_u32(&mut w, v.len() as u32);
        put_f64s(&mut w, v);
    }
    put_u32(&mut w, log.recs.len() as u32);
    for r in &log.recs {
        w.push(r.call as u8);
        put_u32(&mut w, r.a);
        put_u32(&mut w, r.b);
        put_u32(&mut w, r.c.unwrap_or(u32::MAX));
        put_u32(&mut w, r.x0.unwrap_or(u32::MAX));
        put_u32(&mut w, r.eq.len() as u32);
        for &e in &r.eq {
            put_u32(&mut w, e);
        }
    }
    std::fs::File::create(path).unwrap().write_all(&w).unwrap();
}

struct Inst {
    mats: Vec<Mat>,
    vecs: Vec<Vec<f64>>,
    recs: Vec<(Rec, Vec<usize>)>,
}

fn load(path: &str) -> Inst {
    let mut buf = vec![];
    std::fs::File::open(path).unwrap().read_to_end(&mut buf).unwrap();
    let mut r = Rd { b: &buf, p: 0 };
    let nm = r.u32() as usize;
    let mats = (0..nm)
        .map(|_| {
            let rows = r.u32() as usize;
            let cols = r.u32() as usize;
            Mat::from_vec(rows, cols, r.f64s(rows * cols))
        })
        .collect();
    let nv = r.u32() as usize;
    let vecs = (0..nv)
        .map(|_| {
            let l = r.u32() as usize;
            r.f64s(l)
        })
        .collect();
    let nr = r.u32() as usize;
    let opt = |v: u32| if v == u32::MAX { None } else { Some(v) };
    let recs = (0..nr)
        .map(|_| {
            let call = Call::from_u8(r.u8());
            let a = r.u32();
            let b = r.u32();
            let c = opt(r.u32());
            let x0 = opt(r.u32());
            let ne = r.u32() as usize;
            let eq: Vec<u32> = (0..ne).map(|_| r.u32()).collect();
            let equ = eq.iter().map(|&e| e as usize).collect();
            (Rec { call, a, b, c, x0, eq }, equ)
        })
        .collect();
    Inst { mats, vecs, recs }
}

/// Result of one replayed call: Some(x) when the entry point returns a point (empty for bool
/// entry points that answered feasible), None when infeasible.
fn replay_one(inst: &Inst, rec: &Rec, eq: &[usize]) -> Option<Vec<f64>> {
    let a = &inst.mats[rec.a as usize];
    let b = &inst.vecs[rec.b as usize][..];
    let c = rec.c.map(|i| &inst.vecs[i as usize][..]);
    let x0 = rec.x0.map(|i| &inst.vecs[i as usize][..]);
    use rpopt::solver::lp::*;
    match rec.call {
        Call::SolveLp => solve_lp(c, a, b, eq).map(|s| s.x),
        Call::Feasible => lp_feasible(a, b, eq).then(Vec::new),
        Call::FeasibleFrom => lp_feasible_from(a, b, eq, x0.unwrap()).then(Vec::new),
        Call::FeasiblePoint => lp_feasible_point(a, b, eq, x0),
        Call::FeasiblePointShifted => lp_feasible_point_shifted(a, b, eq, x0.unwrap()),
        Call::PointFrom => lp_point_from(c, a, b, eq, x0.unwrap()),
        Call::Point => lp_point(c, a, b, eq),
    }
}

/// Replay classes: entry point, with/without equality rows, theta-space (n <= 4) or not.
fn class_of(inst: &Inst, rec: &Rec) -> String {
    let n = inst.mats[rec.a as usize].cols;
    let name = match rec.call {
        Call::SolveLp => "solve_lp",
        Call::Feasible => "feasible",
        Call::FeasibleFrom => "feasible_from",
        Call::FeasiblePoint => "feasible_point",
        Call::FeasiblePointShifted => "feasible_point_shifted",
        Call::PointFrom => "point_from",
        Call::Point => "point",
    };
    let obj = if rec.c.is_some() { "+obj" } else { "" };
    let eq = if rec.eq.is_empty() { "" } else { "+eq" };
    let size = if n <= 4 { "n<=4" } else if n <= 8 { "n<=8" } else { "n>8" };
    format!("{name}{obj}{eq} {size}")
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args[1].as_str() {
        "capture" => {
            let prog = std::sync::Arc::new(rpopt::program::MpProgram::from_json_file(std::path::Path::new(&args[3])).unwrap());
            let algo = rpopt::algorithms::MpqpAlgorithm::from_name(&args[4]).unwrap();
            capture::start();
            let s = rpopt::algorithms::solve_mpqp(&prog, algo);
            let log = capture::stop();
            save(&log, &args[2]);
            println!("{} regions, {} LP calls, {} matrices, {} vectors", s.critical_regions.len(), log.recs.len(), log.mats.len(), log.vecs.len());
        }
        "run" => {
            let reps: usize = args[2].parse().unwrap();
            let res_path = &args[3];
            let insts: Vec<(String, Inst)> = args[4..].iter().map(|p| (p.clone(), load(p))).collect();
            // results (and warm up)
            let mut out = vec![];
            for (_, inst) in &insts {
                for (rec, eq) in &inst.recs {
                    match replay_one(inst, rec, eq) {
                        None => out.push(0u8),
                        Some(x) => {
                            out.push(1u8);
                            put_u32(&mut out, x.len() as u32);
                            put_f64s(&mut out, &x);
                        }
                    }
                }
            }
            if res_path != "-" {
                std::fs::File::create(res_path).unwrap().write_all(&out).unwrap();
            }
            // classes
            let mut classes: Vec<String> = vec![];
            let mut members: Vec<Vec<(usize, usize)>> = vec![];
            for (fi, (_, inst)) in insts.iter().enumerate() {
                for (ri, (rec, _)) in inst.recs.iter().enumerate() {
                    let c = class_of(inst, rec);
                    let k = match classes.iter().position(|x| *x == c) {
                        Some(k) => k,
                        None => {
                            classes.push(c);
                            members.push(vec![]);
                            classes.len() - 1
                        }
                    };
                    members[k].push((fi, ri));
                }
            }
            let mut file_t = vec![vec![]; insts.len()];
            let mut class_t = vec![vec![]; classes.len()];
            for _ in 0..reps {
                for (fi, (_, inst)) in insts.iter().enumerate() {
                    let t0 = Instant::now();
                    for (rec, eq) in &inst.recs {
                        black_box(replay_one(inst, black_box(rec), eq));
                    }
                    file_t[fi].push(t0.elapsed().as_secs_f64());
                }
                for (k, mem) in members.iter().enumerate() {
                    let t0 = Instant::now();
                    for &(fi, ri) in mem {
                        let inst = &insts[fi].1;
                        let (rec, eq) = &inst.recs[ri];
                        black_box(replay_one(inst, black_box(rec), eq));
                    }
                    class_t[k].push(t0.elapsed().as_secs_f64());
                }
            }
            let mut total = 0.0;
            for (fi, (p, inst)) in insts.iter().enumerate() {
                let t = median(&mut file_t[fi]);
                total += t;
                println!("file  {:<40} {:>8} LPs {:>10.3} ms {:>8.3} us/LP", p.rsplit('/').next().unwrap(), inst.recs.len(), t * 1e3, t * 1e6 / inst.recs.len() as f64);
            }
            let mut ord: Vec<usize> = (0..classes.len()).collect();
            ord.sort_by(|&a, &b| classes[a].cmp(&classes[b]));
            for k in ord {
                let t = median(&mut class_t[k]);
                let (mut rows, mut cols) = (0usize, 0usize);
                for &(fi, ri) in &members[k] {
                    let m = &insts[fi].1.mats[insts[fi].1.recs[ri].0.a as usize];
                    rows += m.rows;
                    cols += m.cols;
                }
                let cnt = members[k].len();
                println!(
                    "class {:<40} {:>8} LPs {:>10.3} ms {:>8.3} us/LP  (avg {:.1} rows x {:.1} cols)",
                    classes[k], cnt, t * 1e3, t * 1e6 / cnt as f64, rows as f64 / cnt as f64, cols as f64 / cnt as f64
                );
            }
            println!("TOTAL {:.3} ms", total * 1e3);
        }
        "compare" => {
            let ra = std::fs::read(&args[2]).unwrap();
            let rb = std::fs::read(&args[3]).unwrap();
            let insts: Vec<Inst> = args[4..].iter().map(|p| load(p)).collect();
            let (mut pa, mut pb) = (Rd { b: &ra, p: 0 }, Rd { b: &rb, p: 0 });
            let (mut n, mut flag_diff, mut obj_diff, mut x_diff, mut infeasible_pts) = (0usize, 0usize, 0usize, 0usize, 0usize);
            let mut max_obj_rel: f64 = 0.0;
            for inst in &insts {
                for (rec, eq) in &inst.recs {
                    n += 1;
                    let sa = pa.u8();
                    let xa = if sa == 1 {
                        let l = pa.u32() as usize;
                        pa.f64s(l)
                    } else {
                        vec![]
                    };
                    let sb = pb.u8();
                    let xb = if sb == 1 {
                        let l = pb.u32() as usize;
                        pb.f64s(l)
                    } else {
                        vec![]
                    };
                    if sa != sb {
                        flag_diff += 1;
                        if flag_diff <= 5 {
                            println!("decision differs: record {} {:?} old {} new {}", n - 1, rec.call, sa, sb);
                        }
                        continue;
                    }
                    if xa.iter().zip(&xb).any(|(u, v)| (u - v).abs() > 1e-9 * (1.0 + u.abs())) {
                        x_diff += 1;
                    }
                    if sb == 1 && !xb.is_empty() {
                        let a = &inst.mats[rec.a as usize];
                        let b = &inst.vecs[rec.b as usize];
                        // shifted calls carry b - A x0: the point is x0 + z, check A (x - x0) <= bs
                        let shift = if rec.call == Call::FeasiblePointShifted { rec.x0.map(|i| &inst.vecs[i as usize]) } else { None };
                        for i in 0..a.rows {
                            let mut ax = 0.0;
                            let mut mag = 0.0;
                            for k in 0..a.cols {
                                let xk = xb[k] - shift.map(|s| s[k]).unwrap_or(0.0);
                                ax += a[(i, k)] * xk;
                                mag += (a[(i, k)] * xk).abs();
                            }
                            let s = b[i] - ax;
                            let viol = if eq.contains(&i) { s.abs() } else { -s };
                            if viol > 1e-7 * (1.0 + b[i].abs() + mag) {
                                infeasible_pts += 1;
                                break;
                            }
                        }
                        if let Some(ci) = rec.c {
                            let c = &inst.vecs[ci as usize];
                            let oa: f64 = c.iter().zip(&xa).map(|(u, v)| u * v).sum();
                            let ob: f64 = c.iter().zip(&xb).map(|(u, v)| u * v).sum();
                            let rel = (oa - ob).abs() / (1.0 + oa.abs().max(ob.abs()));
                            max_obj_rel = max_obj_rel.max(rel);
                            if rel > 1e-9 {
                                obj_diff += 1;
                            }
                        }
                    }
                }
            }
            println!(
                "{n} records: decisions differ {flag_diff}, objectives differ (>1e-9 rel) {obj_diff} (max rel {max_obj_rel:.2e}), points differ {x_diff}, new points infeasible {infeasible_pts}"
            );
        }
        _ => panic!("unknown mode"),
    }
}
