//! Bytes held by the stored regions of a solution, per field, by capacity (what the allocator was asked for, not the
//! length), next to the peak and current resident size of the process.
//!
//!   region_bytes <algorithm> <problem.json>
use rpopt::algorithms::{solve_mpqp, MpqpAlgorithm};
use rpopt::linalg::Mat;
use rpopt::program::MpProgram;
use std::sync::Arc;

#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn malloc_trim(pad: usize) -> i32;
}

fn status_mb(key: &str) -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|t| t.lines().find(|l| l.starts_with(key)).and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse::<f64>().ok())))
        .map_or(0.0, |kb| kb / 1024.0)
}

fn mat(m: &Mat) -> (usize, usize) {
    (m.data.capacity() * 8, m.data.len() * 8)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let alg = MpqpAlgorithm::from_name(&args[1]).expect("algorithm");
    let p = Arc::new(MpProgram::from_json_file(std::path::Path::new(&args[2])).unwrap());
    if let Some(b) = std::env::var("MPLCP_TAB_BUDGET").ok().and_then(|v| v.parse().ok()) {
        rpopt::algorithms::mplcp::set_tab_budget(b);
    }
    let hwm0 = status_mb("VmHWM");
    let rss0 = status_mb("VmRSS");
    let sol = solve_mpqp(&p, alg);
    let regs = &sol.critical_regions;
    let n = regs.len().max(1) as f64;
    let (hwm, rss) = (status_mb("VmHWM"), status_mb("VmRSS"));
    // resident size after handing every free page of the allocator back to the system: the difference is memory that was
    // free but held (fragmentation between the long-lived regions and the transient search data)
    #[cfg(target_os = "linux")]
    let rss_trim = {
        unsafe { malloc_trim(0) };
        status_mb("VmRSS")
    };
    #[cfg(not(target_os = "linux"))]
    let rss_trim = 0.0;
    // (name, capacity bytes, length bytes)
    let mut rows: Vec<(&str, usize, usize)> = Vec::new();
    let mut add = |name: &'static str, cap: usize, len: usize| rows.push((name, cap, len));
    let sum = |f: &dyn Fn(&rpopt::critical_region::CriticalRegion) -> (usize, usize)| regs.iter().fold((0usize, 0usize), |a, r| {
        let (c, l) = f(r);
        (a.0 + c, a.1 + l)
    });
    let v = |x: &Vec<f64>| (x.capacity() * 8, x.len() * 8);
    let u = |x: &Vec<usize>| (x.capacity() * 8, x.len() * 8);
    let (c, l) = sum(&|r| mat(&r.a));
    add("primal law A", c, l);
    let (c, l) = sum(&|r| v(&r.b));
    add("primal law b", c, l);
    let (c, l) = sum(&|r| mat(&r.c));
    add("dual law C", c, l);
    let (c, l) = sum(&|r| v(&r.d));
    add("dual law d", c, l);
    let (c, l) = sum(&|r| mat(&r.e));
    add("facet rows E", c, l);
    let (c, l) = sum(&|r| v(&r.f));
    add("facet rhs f", c, l);
    let (c, l) = sum(&|r| u(&r.active_set));
    add("active_set", c, l);
    let (c, l) = sum(&|r| u(&r.omega_set));
    add("omega_set", c, l);
    let (c, l) = sum(&|r| u(&r.lambda_set));
    add("lambda_set", c, l);
    let (c, l) = sum(&|r| (r.regular_set.0.capacity() * 8 + r.regular_set.1.capacity() * 8, r.regular_set.0.len() * 8 + r.regular_set.1.len() * 8));
    add("regular_set", c, l);
    let (c, l) = sum(&|r| (r.y_fixation.as_ref().map_or(0, |x| x.capacity() * 8) + r.y_indices.as_ref().map_or(0, |x| x.capacity() * 8) + r.x_indices.as_ref().map_or(0, |x| x.capacity() * 8), 0));
    add("mixed-integer fields", c, l);
    let shell = std::mem::size_of::<rpopt::critical_region::CriticalRegion>() * regs.capacity();
    let total_cap: usize = rows.iter().map(|r| r.1).sum::<usize>() + shell;
    let total_len: usize = rows.iter().map(|r| r.2).sum::<usize>() + std::mem::size_of::<rpopt::critical_region::CriticalRegion>() * regs.len();
    println!("{} {} regions {} nt {} nx {}", args[1], args[2].rsplit('/').next().unwrap(), regs.len(), p.num_t(), p.num_x());
    println!("| field | capacity B/region | length B/region |");
    println!("|---|---:|---:|");
    for (name, c, l) in &rows {
        println!("| {name} | {:.0} | {:.0} |", *c as f64 / n, *l as f64 / n);
    }
    println!("| struct shell (Vec of regions, {} B each) | {:.0} | {:.0} |", std::mem::size_of::<rpopt::critical_region::CriticalRegion>(), shell as f64 / n, (shell as f64) / n);
    println!("| total | {:.0} | {:.0} |", total_cap as f64 / n, total_len as f64 / n);
    println!("rss_before_mb {rss0:.0} hwm_before_mb {hwm0:.0} rss_after_mb {rss:.0} rss_after_trim_mb {rss_trim:.0} hwm_after_mb {hwm:.0}");
    println!("regions_cap_mb {:.0} rss_growth_per_region_B {:.0} hwm_growth_per_region_B {:.0}", total_cap as f64 / 1048576.0, (rss - rss0) * 1048576.0 / n, (hwm - hwm0) * 1048576.0 / n);
}
