//! Exports closed-loop trajectories and the condensed QP of explicit MPC problems for
//! `bench/online/online_bench.cpp` (DAQP and qpOASES solving the same QP online), and times the
//! explicit law on the same trajectories: hash-table point location plus the first input block.
//!
//!   cargo run --release --example export_online -- <out_dir> <problem.json>... [--traj N,L] [--hash-mb M]
//!
//! Plant: the "mpc" key of the problem (bench/problems) or the sidecar `<name>.model.json`
//! (bench/problems_large). Trajectories: x+ = A x + B u(x) from uniform random feasible starts; a
//! trajectory ends when it leaves the partition. Explicit time per step: the minimum over 5
//! repetitions of a batch of 64 calls at that step, the same protocol as the C harness.
//!
//! Output `<name>.bin`, little endian: 7 u64 (magic 0x4f4e4c494e45, nx, nt, m, nu, T, regions),
//! then f64 arrays Q (nx*nx), c (nx), H (nx*nt), A (m*nx), b (m), F (m*nt) (row major, the
//! processed program: ½x'Qx + (c + Hθ)'x s.t. Ax <= b + Fθ), θ (T*nt), u_ref (T*nu),
//! explicit_ns (T), start flags (T, 1.0 at the first step of a trajectory), and one f64 with the
//! hash table size in bytes.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::point_location::{HashTable, PartitionGeometry, PointLocator};
use rpopt::program::MpProgram;
use std::hint::black_box;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

struct Plant {
    a: Vec<Vec<f64>>,
    b: Vec<Vec<f64>>,
    u_index: usize,
}

fn mat(v: &serde_json::Value) -> Vec<Vec<f64>> {
    v.as_array().unwrap().iter().map(|r| r.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()).collect()
}

fn load_plant(path: &std::path::Path) -> Option<Plant> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    if let Some(m) = v.get("mpc") {
        return Some(Plant { a: mat(&m["A"]), b: mat(&m["B"]), u_index: 0 });
    }
    let side: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path.with_extension("model.json")).ok()?).ok()?;
    Some(Plant { a: mat(&side["A"]), b: mat(&side["B"]), u_index: side["u_index"].as_u64().unwrap_or(0) as usize })
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let (mut ntraj, mut len, mut hash_mb) = (200usize, 50usize, 64usize);
    let mut k = 0;
    while k < args.len() {
        match args[k].as_str() {
            "--traj" => {
                let p: Vec<usize> = args[k + 1].split(',').map(|s| s.parse().unwrap()).collect();
                (ntraj, len) = (p[0], p[1]);
                args.drain(k..k + 2);
            }
            "--hash-mb" => {
                hash_mb = args[k + 1].parse().unwrap();
                args.drain(k..k + 2);
            }
            _ => k += 1,
        }
    }
    let out = std::path::PathBuf::from(&args[0]);
    std::fs::create_dir_all(&out).unwrap();
    for file in &args[1..] {
        let path = std::path::Path::new(file);
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let prog = Arc::new(MpProgram::from_json_file(path).unwrap());
        if !prog.equality_indices.is_empty() {
            println!("{name}: skipped (equality constraints)");
            continue;
        }
        let Some(plant) = load_plant(path) else {
            println!("{name}: skipped (no plant model)");
            continue;
        };
        let t0 = Instant::now();
        let sol = solve_mpqp(&prog, MpqpAlgorithm::Geometric);
        let solve_s = t0.elapsed().as_secs_f64();
        let t1 = Instant::now();
        let geom = PartitionGeometry::new(&sol);
        let table = HashTable::with_storage_limit(&geom, hash_mb << 20);
        let build_s = t1.elapsed().as_secs_f64();
        let (nx, nt, m) = (prog.num_x(), prog.num_t(), prog.num_constraints());
        let nu = plant.b[0].len();
        let (u0, u1) = (plant.u_index * nu, plant.u_index * nu + nu);

        // closed-loop trajectories from uniform random feasible starts
        let bbox = prog.theta_bbox().cloned().unwrap();
        let mut rng = StdRng::seed_from_u64(7);
        let (mut th, mut start) = (vec![], vec![]);
        for _ in 0..ntraj {
            let mut x: Option<Vec<f64>> = None;
            for _ in 0..1000 {
                let c: Vec<f64> = (0..nt).map(|j| rng.gen_range(bbox.0[j]..bbox.1[j])).collect();
                if table.locate(&c).is_some() {
                    x = Some(c);
                    break;
                }
            }
            let Some(mut x) = x else { continue };
            for step in 0..len {
                let Some(r) = table.locate(&x) else { break };
                th.push(x.clone());
                start.push(if step == 0 { 1.0 } else { 0.0 });
                let law = sol.critical_regions[r].evaluate(&x);
                let u = &law[u0..u1];
                x = (0..nt).map(|i| (0..nt).map(|j| plant.a[i][j] * x[j]).sum::<f64>() + (0..nu).map(|j| plant.b[i][j] * u[j]).sum::<f64>()).collect();
            }
        }
        let t = th.len();

        // explicit law: locate + first input block, min over 5 reps of 64 calls per step
        let first_input = |r: usize, x: &[f64], u: &mut [f64]| {
            let cr = &sol.critical_regions[r];
            for (k, row) in (u0..u1).enumerate() {
                u[k] = cr.b[row] + cr.a.row(row).iter().zip(x).map(|(p, q)| p * q).sum::<f64>();
            }
        };
        let mut uref = vec![0.0; t * nu];
        let mut ns = vec![0.0; t];
        let mut u = vec![0.0; nu];
        for (s, x) in th.iter().enumerate() {
            let r = table.locate(x).unwrap();
            first_input(r, x, &mut uref[s * nu..(s + 1) * nu]);
            let mut best = f64::INFINITY;
            for _ in 0..5 {
                let t2 = Instant::now();
                for _ in 0..64 {
                    let r = table.locate(black_box(x)).unwrap();
                    first_input(r, x, &mut u);
                    black_box(&u);
                }
                best = best.min(t2.elapsed().as_nanos() as f64 / 64.0);
            }
            ns[s] = best;
        }

        let mut f = std::io::BufWriter::new(std::fs::File::create(out.join(format!("{name}.bin"))).unwrap());
        for v in [0x4f4e4c494e45u64, nx as u64, nt as u64, m as u64, nu as u64, t as u64, sol.critical_regions.len() as u64] {
            f.write_all(&v.to_le_bytes()).unwrap();
        }
        let mut put = |vals: &mut dyn Iterator<Item = f64>| {
            for v in vals {
                f.write_all(&v.to_le_bytes()).unwrap();
            }
        };
        let q = prog.q.as_ref().unwrap();
        put(&mut (0..nx).flat_map(|i| q.row(i).to_vec()));
        put(&mut prog.c.iter().cloned());
        put(&mut (0..nx).flat_map(|i| prog.h.row(i).to_vec()));
        put(&mut (0..m).flat_map(|i| prog.a.row(i).to_vec()));
        put(&mut prog.b.iter().cloned());
        put(&mut (0..m).flat_map(|i| prog.f.row(i).to_vec()));
        put(&mut th.iter().flat_map(|x| x.clone()));
        put(&mut uref.iter().cloned());
        put(&mut ns.iter().cloned());
        put(&mut start.iter().cloned());
        put(&mut std::iter::once(table.storage_bytes() as f64));
        drop(f);
        let mut sorted = ns.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        println!(
            "{name}: nx {nx} nt {nt} m {m} nu {nu} regions {} steps {t} | solve {solve_s:.2} s, table {build_s:.2} s, {:.2} MB | explicit median {:.1} ns max {:.1} ns",
            sol.critical_regions.len(),
            table.storage_bytes() as f64 / 1048576.0,
            sorted[t / 2],
            sorted[t - 1]
        );
    }
}
