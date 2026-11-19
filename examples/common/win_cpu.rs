//! CPU-time clock for Windows, shared by the benchmark examples (`#[path = "common/win_cpu.rs"] mod win_cpu;`).
//!
//! Unix has fine-grained CPU clocks (`getrusage`, `clock_gettime`). Windows' `GetThreadTimes` / `GetProcessTimes`
//! only advance every 15.6 ms (the scheduler tick), too coarse for solves of a few milliseconds. The cycle counters
//! `QueryThreadCycleTime` / `QueryProcessCycleTime` count the cycles the thread or process actually ran, with fine
//! resolution, and exclude time spent waiting for other programs. Cycles are converted to seconds with a
//! cycles-per-second figure measured once on first use, so the seconds are approximate (a few percent, more if the
//! CPU changes clock speed during a run); use them for comparing timings on the same machine, not as exact values.

use std::sync::OnceLock;
use std::time::Instant;

unsafe extern "system" {
    fn GetCurrentProcess() -> isize;
    fn GetCurrentThread() -> isize;
    fn QueryProcessCycleTime(handle: isize, cycles: *mut u64) -> i32;
    fn QueryThreadCycleTime(handle: isize, cycles: *mut u64) -> i32;
}

fn thread_cycles() -> u64 {
    let mut c = 0u64;
    unsafe { QueryThreadCycleTime(GetCurrentThread(), &mut c) };
    c
}

fn process_cycles() -> u64 {
    let mut c = 0u64;
    unsafe { QueryProcessCycleTime(GetCurrentProcess(), &mut c) };
    c
}

/// Cycles per second of CPU time, measured by spinning this thread: a thread that is descheduled during a trial
/// only lowers its cycles-per-wall-second, so the largest of several short trials is the best estimate.
fn cycles_per_second() -> f64 {
    static HZ: OnceLock<f64> = OnceLock::new();
    *HZ.get_or_init(|| {
        let mut best = 0.0f64;
        for _ in 0..5 {
            let t0 = Instant::now();
            let c0 = thread_cycles();
            while t0.elapsed().as_millis() < 20 {
                std::hint::spin_loop();
            }
            let hz = (thread_cycles() - c0) as f64 / t0.elapsed().as_secs_f64();
            best = best.max(hz);
        }
        best
    })
}

/// CPU seconds used so far by the calling thread (like `CLOCK_THREAD_CPUTIME_ID`).
#[allow(dead_code)]
pub fn thread_cpu_seconds() -> f64 {
    let hz = cycles_per_second();
    thread_cycles() as f64 / hz
}

/// CPU seconds used so far by the whole process, all threads (like `getrusage` user + system, or `clock()`).
#[allow(dead_code)]
pub fn process_cpu_seconds() -> f64 {
    let hz = cycles_per_second();
    process_cycles() as f64 / hz
}
