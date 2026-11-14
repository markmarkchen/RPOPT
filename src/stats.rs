//! Operation counters for the PPOPT comparison.
//!
//! `PPOPT_LPS` / `PPOPT_QPS` count the deterministic-solver calls PPOPT makes for the same
//! algorithm: they are incremented wherever PPOPT calls `solver.solve_lp` / `solve_qp`, whether or
//! not the Rust port answers that question with an LP (it sometimes skips the call or uses a
//! cheaper exact test). `PPOPT_RANKS` counts PPOPT's `numpy.linalg.matrix_rank` tests. Multiplying
//! them by the cost of one Python call gives a lower bound on PPOPT's run time.
//!
//! The counters are sharded per thread (one cache line each) so the parallel algorithms do not
//! contend on a shared atomic.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

const SHARDS: usize = 64;

#[repr(align(128))]
pub struct Padded(AtomicU64);

pub struct Sharded([Padded; SHARDS]);

impl Sharded {
    pub const fn new() -> Self {
        Sharded([const { Padded(AtomicU64::new(0)) }; SHARDS])
    }

    #[inline]
    pub fn add(&self, n: u64) {
        self.0[shard()].0.fetch_add(n, Ordering::Relaxed);
    }

    pub fn load(&self) -> u64 {
        self.0.iter().map(|p| p.0.load(Ordering::Relaxed)).sum()
    }
}

impl Default for Sharded {
    fn default() -> Self {
        Self::new()
    }
}

static NEXT_SHARD: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static SHARD: usize = NEXT_SHARD.fetch_add(1, Ordering::Relaxed) % SHARDS;
}

#[inline]
fn shard() -> usize {
    SHARD.with(|s| *s)
}

pub static PPOPT_LPS: Sharded = Sharded::new();
pub static PPOPT_QPS: Sharded = Sharded::new();
pub static PPOPT_RANKS: Sharded = Sharded::new();

#[inline]
pub fn lp(n: u64) {
    PPOPT_LPS.add(n);
}

#[inline]
pub fn qp() {
    PPOPT_QPS.add(1);
}

#[inline]
pub fn rank() {
    PPOPT_RANKS.add(1);
}

pub fn ranks() -> u64 {
    PPOPT_RANKS.load()
}

pub fn snapshot() -> (u64, u64) {
    (PPOPT_LPS.load(), PPOPT_QPS.load())
}

/// Region discovery log for time-to-coverage measurements (`examples/coverage.rs`): when enabled,
/// every critical region the construction returns is logged with its active set and the time it
/// was built. Off by default (one relaxed load per built region).
static RECORDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static EVENTS: std::sync::Mutex<Vec<(Vec<usize>, std::time::Instant)>> = std::sync::Mutex::new(Vec::new());

/// Turns the region log on (clearing it) or off.
pub fn record_regions(on: bool) {
    if on {
        EVENTS.lock().unwrap().clear();
    }
    RECORDING.store(on, Ordering::SeqCst);
}

#[inline]
pub fn region_built(active_set: &[usize]) {
    if RECORDING.load(Ordering::Relaxed) {
        EVENTS.lock().unwrap().push((active_set.to_vec(), std::time::Instant::now()));
    }
}

/// The logged (active set, build time) pairs, emptying the log.
pub fn take_region_events() -> Vec<(Vec<usize>, std::time::Instant)> {
    std::mem::take(&mut *EVENTS.lock().unwrap())
}
