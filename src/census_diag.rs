//! Counters for the mplcp region-build study (feature `lp_census`, switched on by the environment
//! variable `RPOPT_DIAG=1`): where the redundancy step spends its rows, how much a child shares with
//! its parent, and what an output-sensitive (Clarkson style) redundancy removal would need. Counts
//! only: nothing here changes a decision of the solver.
//!
//! The slots live in `census::MD_BASE..` (read them with `slot`). The diagnostic's own work is
//! timed in `DIAG_NS` so that it can be subtracted from the solve time.

use crate::census::{bump_slot, MD_BASE};
use crate::linalg::{dot, Mat};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

pub mod s {
    pub const REG: usize = 0;
    pub const ROWS: usize = 1;
    pub const FACETS: usize = 2;
    pub const PROP_REJ: usize = 3; // rows rejected by the propagated parameter box (free)
    pub const BOX_REJ: usize = 4; // rejected by the exact box (2n LPs)
    pub const VERT_KEEP: usize = 5; // kept by an exact-box vertex
    pub const PROJ_KEEP: usize = 6; // kept by the projection test
    pub const LP_KEEP: usize = 7; // kept by a per-row LP
    pub const LP_DROP: usize = 8; // dropped by a per-row LP
    pub const HAS_ENTRY: usize = 9; // children built with an entry facet
    pub const ENTRY_FOUND: usize = 10; // entry row located among the rows
    pub const ENTRY_BOXREJ: usize = 11;
    pub const ENTRY_VERT: usize = 12;
    pub const ENTRY_PROJ: usize = 13;
    pub const ENTRY_LPKEEP: usize = 14;
    pub const HAS_PARENT: usize = 16;
    pub const CMP_ROWS: usize = 17; // rows with a counterpart (same kind and constraint) in the parent
    pub const PK_CK: usize = 18;
    pub const PK_CD: usize = 19;
    pub const PD_CK: usize = 20;
    pub const PD_CD: usize = 21;
    pub const LPS_CMP: usize = 22; // rows that reach the per-row LP stage and have a counterpart
    pub const LPS_AGREE: usize = 23;
    pub const LPS_DROP_PD: usize = 24; // child LP-dropped, parent dropped
    pub const LPS_KEEP_PK: usize = 25; // child LP-kept, parent kept
    pub const LPS_ALL: usize = 26; // rows that reach the per-row LP stage
    pub const LAW_NS: usize = 27;
    pub const ROWS_NS: usize = 28;
    pub const CHEB_NS: usize = 29;
    pub const RED_NS: usize = 30;
    pub const BUILD_NS: usize = 31;
    pub const DIAG_NS: usize = 32;
    pub const BOX_COORDS: usize = 35;
    pub const BOX_EQ_PARENT: usize = 36;
    pub const PT_TOTAL: usize = 37;
    pub const PT_FEAS: usize = 38;
    pub const PT_OPT: usize = 39;
    pub const VK_ROWS: usize = 40;
    pub const VK_COVERED: usize = 41;
    pub const PROP30_REJ: usize = 42;
    pub const ROWS_REMAINING_AFTER_PROP: usize = 43;
    pub const HINT_N: usize = 44;
    pub const HINT_M0: usize = 45; // margin > 0
    pub const HINT_M1: usize = 46; // > 1e-9
    pub const HINT_M2: usize = 47; // > 1e-6
    pub const HINT_M3: usize = 48; // > 1e-4
    pub const TIGHT_NS: usize = 49;
    pub const TABC_REG: usize = 75;
    pub const TABC_T0: usize = 76; // 6 slots: regions whose max row difference exceeds 1e-12, 1e-10, 1e-8, 1e-6, 1e-4, 1e-2
    pub const TABC_ROWS: usize = 82;
    pub const TABC_MISSING: usize = 83;
    pub const TAB_ROWS_NS: usize = 84;
    pub const TAB_NOPIV: usize = 85;
    pub const ENG_LPS: usize = 104;
    pub const ENG_PIV: usize = 105;
    pub const ENG_NS: usize = 106;
    pub const COLD_STEPS: usize = 107;
    pub const COLD_NS: usize = 108;
    pub const WARM1_STEPS: usize = 109; // pairs order: min t1, max t1, min t2, ...
    pub const WARM1_NS: usize = 110;
    pub const WARM2_STEPS: usize = 111; // all +e_k then all -e_k reversed
    pub const WARM2_NS: usize = 112;
    pub const WARM_FAIL: usize = 113;
    pub const COLD_VALDIFF: usize = 114; // bounds where my cold optimum differs from the engine's
    pub const WARM1_VALDIFF: usize = 115;
    pub const WARM2_VALDIFF: usize = 116;
    pub const BOX_BOUNDS: usize = 117;
    pub const TIGHT_ROWS_COLD: usize = 118; // rows tight at some engine box point
    pub const TIGHT_DIFF_W1: usize = 119; // symmetric difference of the tight-row sets, warm order 1 against engine
    pub const TIGHT_DIFF_W2: usize = 120;
    pub const BOX_REGIONS: usize = 121;
    pub const E_V2_PIV: usize = 122; // engine chain, order 2, start at the previous vertex
    pub const E_V2_NS: usize = 123;
    pub const E_P2_PIV: usize = 124; // engine chain, order 2, start pushed 1e-4 towards the centre
    pub const E_P2_NS: usize = 125;
    pub const E_V2_VALDIFF: usize = 126;
    pub const E_V2_TD: usize = 127;
    pub const E_P2_VALDIFF: usize = 128;
    pub const E_P2_TD: usize = 129;
    pub const E_V1_PIV: usize = 130; // engine chain, order 1, vertex start
    pub const E_V1_NS: usize = 131;
    pub const E_V1_VALDIFF: usize = 132;
    pub const E_FAIL: usize = 133;
    pub const E_V1_TD: usize = 134;
    pub const TC_NS: usize = 136;
    pub const TC_STEPS: usize = 137;
    pub const TC_FALLBACK: usize = 138;
    pub const TC_ENGINE_LPS: usize = 139;
    pub const TC_VALDIFF: usize = 140;
    pub const TC_TD: usize = 141;
    pub const TC_NONE: usize = 142;
    pub const SH_N: usize = 143;
    pub const SH_PTS_TIGHT: usize = 144;
    pub const SH_PTS_FEAS: usize = 145;
    pub const SH_ANY: usize = 146;
    pub const SH_ENG_NS: usize = 147;
    pub const SH_ENG_PIV: usize = 148;
    pub const SH_CH_NS: usize = 149;
    pub const SH_CH_STEPS: usize = 150;
    pub const SH_CH_ENGLP: usize = 151;
    pub const SH_SV_NS: usize = 152;
    pub const SH_SV_STEPS: usize = 153;
    pub const SH_SV_ENGLP: usize = 154;
    pub const SH_SV_FB: usize = 155;
    pub const SH_SV_VALDIFF: usize = 156;
    pub const SH_SV_TD: usize = 157;
    pub const SH_LPKEEP_ROWS: usize = 158;
    pub const SH_SETTLED: usize = 159;
    // Farkas certificates: 4 variants, 8 slots each
    pub const FK_BASE: usize = 170;
    pub const FK_STRIDE: usize = 8;
    // offsets: 0 tested prop-box rows, 1 caught, 2 tested exact-box rows, 3 caught, 4 tested LP-dropped rows, 5 caught, 6 ns, 7 calls
    // ridge rule
    pub const RG_CHILDREN: usize = 205;
    pub const RG_PTS: usize = 206;
    pub const RG_RIDGE: usize = 207;
    pub const RG_UNIQUE: usize = 208;
    pub const RG_UNIQUE_FACET: usize = 209;
    pub const RG_UNIQUE_WRONG: usize = 210;
    pub const RG_FACETS: usize = 211;
    pub const RG_AMBIG: usize = 212;
    pub const RG_AMBIG_WRONG: usize = 213;
    pub const RG_NOENOUGH: usize = 214; // children whose shared points are fewer than d - 1
    // time profile of the LP-path builder (always counted under lp_census)
    pub const PF_BOX_NS: usize = 265; // box_chain, whole call
    pub const PF_PERROW_NS: usize = 266; // projection tests and per-row LPs
    pub const PF_SETUP_NS: usize = 267; // Plcp::new
    pub const PF_INIT_NS: usize = 268; // initial_region
    pub const PF_LOOP_NS: usize = 269; // the exploration loop of pivot_solve
    pub const PF_CLIP_NS: usize = 270; // clip_flags, all calls (nt <= 5)
    pub const PF_CLIP_FAIL_NS: usize = 271; // clip_flags calls that returned None
    pub const PF_CLIP_FAILS: usize = 272;
    pub const PF_WORK_OK: usize = 273; // 28 log2 buckets of the work of successful clips (nt <= 5)
    pub const PF_WORK_FAIL: usize = 302; // the same for clips that returned None
    pub const PF_CLIP_BUDGET_ABORTS: usize = 331;
    pub const PF_REGSTAT_NS: usize = 332; // census-only work of the per-region records (subtracted from the solve)
    // Clarkson simulation, variant A (seed: parameter-set rows) at SIM_A, variant B (A + entry + parent's kept rows) at SIM_B
    pub const SIM_A: usize = 50;
    pub const SIM_B: usize = 62;
    pub const SIM_C: usize = 90;
    // offsets inside a variant
    pub const O_LPS: usize = 0;
    pub const O_LPROWS: usize = 1;
    pub const O_RAYS: usize = 2;
    pub const O_LP_NS: usize = 3;
    pub const O_RAY_NS: usize = 4;
    pub const O_MISMATCH: usize = 5;
    pub const O_SFINAL: usize = 6;
    pub const O_RAYPROOF: usize = 7; // rows decided by the projection ray alone (no LP)
    pub const O_FACET_RAY: usize = 8; // rows proven kept by an LP-violating point + ray
    pub const O_REDUND: usize = 9; // rows dropped (LP)
    pub const O_LPS_PER_KEEP: usize = 10; // LPs spent on rows that end up kept
}

pub fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("RPOPT_DIAG").map_or(false, |v| v == "1"))
}

pub fn add(i: usize, n: u64) {
    bump_slot(MD_BASE + i, n);
}

pub fn t0() -> Instant {
    Instant::now()
}
pub fn lap(i: usize, t: Instant) {
    add(i, t.elapsed().as_nanos() as u64);
}

pub fn eng_begin() -> (u64, Instant) {
    (crate::census::piv_get(), Instant::now())
}
pub fn eng_end(m: (u64, Instant)) {
    if !on() {
        return;
    }
    add(s::ENG_LPS, 1);
    add(s::ENG_PIV, crate::census::piv_get() - m.0);
    add(s::ENG_NS, m.1.elapsed().as_nanos() as u64);
}

/// Primal simplex from a point `x` of the polytope with tight rows `t` (independent), maximizing
/// c'x; returns the number of ratio-test steps, or None on failure (unbounded or singular).
fn opt(e: &Mat, f: &[f64], x: &mut Vec<f64>, t: &mut Vec<usize>, c: &[f64]) -> Option<u64> {
    let (m, n) = (e.rows, e.cols);
    let mut steps = 0u64;
    for _ in 0..(10 * (n + m) + 50) {
        let k = t.len();
        let mut d = c.to_vec();
        let mut y = vec![];
        if k > 0 {
            let mut g = Mat::zeros(k, k);
            let mut rhs = vec![0.0; k];
            for i in 0..k {
                rhs[i] = dot(e.row(t[i]), c);
                for j in 0..k {
                    g[(i, j)] = dot(e.row(t[i]), e.row(t[j]));
                }
            }
            let lu = crate::linalg::Lu::new(&g)?;
            y = lu.solve(&rhs);
            for i in 0..k {
                for q in 0..n {
                    d[q] -= y[i] * e[(t[i], q)];
                }
            }
        }
        let dn = crate::linalg::norm(&d);
        let cn = crate::linalg::norm(c);
        if dn > 1e-9 * cn {
            // move along d until a row blocks
            let mut best = (usize::MAX, f64::INFINITY);
            for j in 0..m {
                if t.contains(&j) {
                    continue;
                }
                let ad = dot(e.row(j), &d);
                if ad > 1e-12 * dn {
                    let tt = (f[j] - dot(e.row(j), x)).max(0.0) / ad;
                    if tt < best.1 {
                        best = (j, tt);
                    }
                }
            }
            if best.0 == usize::MAX {
                return None;
            }
            for q in 0..n {
                x[q] += best.1 * d[q];
            }
            t.push(best.0);
            steps += 1;
        } else {
            // c in the span of the tight rows: optimal if all multipliers are nonnegative
            let (mut wi, mut wv) = (usize::MAX, -1e-9 * cn);
            for i in 0..k {
                if y[i] < wv {
                    wv = y[i];
                    wi = i;
                }
            }
            if wi == usize::MAX {
                return Some(steps);
            }
            t.remove(wi);
        }
    }
    None
}

/// The engine's box LPs chained: each starts at the previous optimum (pushed `push` of the way to the centre).
fn engine_chain(e: &Mat, f: &[f64], center: &[f64], order: &[usize], push: f64, target: &dyn Fn(usize) -> f64, slots: (usize, usize, usize, usize), cold_t: &[bool]) {
    let n = e.cols;
    let t = Instant::now();
    let p0 = crate::census::piv_get();
    let mut x = center.to_vec();
    let mut pts: Vec<Vec<f64>> = vec![];
    let mut diff = 0u64;
    for &j in order {
        let mut obj = vec![0.0; n];
        obj[j / 2] = if j % 2 == 0 { 1.0 } else { -1.0 };
        let x0: Vec<f64> = x.iter().zip(center).map(|(a, c)| a + push * (c - a)).collect();
        match crate::solver::lp::lp_point_from(Some(&obj), e, f, &[], &x0) {
            Some(y) => {
                if (y[j / 2] - target(j)).abs() > 1e-9 * (1.0 + target(j).abs()) {
                    diff += 1;
                }
                x = y.clone();
                pts.push(y);
            }
            None => {
                add(s::E_FAIL, 1);
                return;
            }
        }
    }
    let ns = t.elapsed().as_nanos() as u64;
    let piv = crate::census::piv_get() - p0;
    add(slots.0, piv);
    add(slots.1, ns);
    add(slots.2, diff);
    let wt: Vec<bool> = (0..e.rows).map(|i| pts.iter().any(|x| (dot(e.row(i), x) - f[i]).abs() <= 1e-9 * (1.0 + f[i].abs()))).collect();
    add(slots.3, wt.iter().zip(cold_t).filter(|(a, b)| a != b).count() as u64);
}

fn box_study(e: &Mat, f: &[f64], center: &[f64], bx: &(Vec<f64>, Vec<f64>, Vec<Vec<f64>>), entry_p: Option<usize>, parent_pts: Option<&[Vec<f64>]>, classes: &[u8]) {
    let n = e.cols;
    let (lo, hi, pts) = bx;
    // objective j = 2k (min theta_k = max -e_k), 2k+1 (max theta_k = max +e_k), in the engine's order
    let obj = |j: usize| -> Vec<f64> {
        let mut c = vec![0.0; n];
        c[j / 2] = if j % 2 == 0 { -1.0 } else { 1.0 };
        c
    };
    let target = |j: usize| -> f64 { if j % 2 == 0 { lo[j / 2] } else { hi[j / 2] } };
    let order1: Vec<usize> = (0..2 * n).collect();
    let mut order2: Vec<usize> = (0..n).map(|k| 2 * k + 1).collect();
    order2.extend((0..n).rev().map(|k| 2 * k));
    add(s::BOX_REGIONS, 1);
    add(s::BOX_BOUNDS, 2 * n as u64);
    // cold: every LP from the centre
    let t = Instant::now();
    let mut diff = 0u64;
    let mut steps = 0u64;
    let mut fail = false;
    for j in 0..2 * n {
        let (mut x, mut tt) = (center.to_vec(), vec![]);
        match opt(e, f, &mut x, &mut tt, &obj(j)) {
            Some(st) => {
                steps += st;
                if (x[j / 2] - target(j)).abs() > 1e-9 * (1.0 + target(j).abs()) {
                    diff += 1;
                }
            }
            None => fail = true,
        }
    }
    add(s::COLD_NS, t.elapsed().as_nanos() as u64);
    add(s::COLD_STEPS, steps);
    add(s::COLD_VALDIFF, diff);
    if fail {
        add(s::WARM_FAIL, 1);
    }
    // rows tight at some engine point
    let tight_set = |ps: &[Vec<f64>]| -> Vec<bool> { (0..e.rows).map(|i| ps.iter().any(|x| (dot(e.row(i), x) - f[i]).abs() <= 1e-9 * (1.0 + f[i].abs()))).collect() };
    let cold_t = tight_set(pts);
    add(s::TIGHT_ROWS_COLD, cold_t.iter().filter(|&&v| v).count() as u64);
    engine_chain(e, f, center, &order2, 0.0, &target, (s::E_V2_PIV, s::E_V2_NS, s::E_V2_VALDIFF, s::E_V2_TD), &cold_t);
    engine_chain(e, f, center, &order2, 1e-4, &target, (s::E_P2_PIV, s::E_P2_NS, s::E_P2_VALDIFF, s::E_P2_TD), &cold_t);
    engine_chain(e, f, center, &order1, 0.0, &target, (s::E_V1_PIV, s::E_V1_NS, s::E_V1_VALDIFF, s::E_V1_TD), &cold_t);
    {
        let mut st = crate::boxlp::Stats::default();
        let t = Instant::now();
        let r = crate::boxlp::box_chain(e, f, center, None, &mut st);
        add(s::TC_NS, t.elapsed().as_nanos() as u64);
        match r {
            None => add(s::TC_NONE, 1),
            Some((clo, chi, cpts)) => {
                add(s::TC_STEPS, st.steps);
                add(s::TC_FALLBACK, st.fallbacks);
                add(s::TC_ENGINE_LPS, st.engine_lps);
                let mut dv = 0;
                for k in 0..n {
                    if (clo[k] - lo[k]).abs() > 1e-9 * (1.0 + lo[k].abs()) {
                        dv += 1;
                    }
                    if (chi[k] - hi[k]).abs() > 1e-9 * (1.0 + hi[k].abs()) {
                        dv += 1;
                    }
                }
                add(s::TC_VALDIFF, dv);
                let wt = tight_set(&cpts);
                add(s::TC_TD, wt.iter().zip(&cold_t).filter(|(a, b)| a != b).count() as u64);
            }
        }
    }
    for (ord, st_slot, ns_slot, vd_slot, td_slot) in [(&order1, s::WARM1_STEPS, s::WARM1_NS, s::WARM1_VALDIFF, s::TIGHT_DIFF_W1), (&order2, s::WARM2_STEPS, s::WARM2_NS, s::WARM2_VALDIFF, s::TIGHT_DIFF_W2)] {
        let t = Instant::now();
        let (mut x, mut tt) = (center.to_vec(), vec![]);
        let (mut steps, mut diff, mut fail) = (0u64, 0u64, false);
        let mut wp: Vec<Vec<f64>> = vec![];
        for &j in ord.iter() {
            match opt(e, f, &mut x, &mut tt, &obj(j)) {
                Some(st) => {
                    steps += st;
                    if (x[j / 2] - target(j)).abs() > 1e-9 * (1.0 + target(j).abs()) {
                        diff += 1;
                    }
                    wp.push(x.clone());
                }
                None => {
                    fail = true;
                    break;
                }
            }
        }
        add(ns_slot, t.elapsed().as_nanos() as u64);
        add(st_slot, steps);
        add(vd_slot, diff);
        if fail {
            add(s::WARM_FAIL, 1);
        } else {
            let wt = tight_set(&wp);
            add(td_slot, wt.iter().zip(&cold_t).filter(|(a, b)| a != b).count() as u64);
        }
    }
    if let (Some(ep), Some(pp)) = (entry_p, parent_pts) {
        add(s::SH_N, 1);
        let mut feas: Vec<&Vec<f64>> = vec![];
        for x in pp {
            if (dot(e.row(ep), x) - f[ep]).abs() <= 1e-9 * (1.0 + f[ep].abs()) {
                add(s::SH_PTS_TIGHT, 1);
                if (0..e.rows).all(|i| dot(e.row(i), x) <= f[i] + 1e-9 * (1.0 + f[i].abs())) {
                    add(s::SH_PTS_FEAS, 1);
                    feas.push(x);
                }
            }
        }
        if !feas.is_empty() {
            add(s::SH_ANY, 1);
            SHARED.with(|c| *c.borrow_mut() = feas.iter().map(|x| (*x).clone()).collect());
            let t = Instant::now();
            let p0 = crate::census::piv_get();
            for j in 0..2 * n {
                let mut o = vec![0.0; n];
                o[j / 2] = if j % 2 == 0 { 1.0 } else { -1.0 };
                let _ = crate::solver::lp::lp_point_from(Some(&o), e, f, &[], center);
            }
            add(s::SH_ENG_NS, t.elapsed().as_nanos() as u64);
            add(s::SH_ENG_PIV, crate::census::piv_get() - p0);
            let mut st = crate::boxlp::Stats::default();
            let t = Instant::now();
            let _ = crate::boxlp::box_chain(e, f, center, None, &mut st);
            add(s::SH_CH_NS, t.elapsed().as_nanos() as u64);
            add(s::SH_CH_STEPS, st.steps);
            add(s::SH_CH_ENGLP, st.engine_lps);
            let mut st = crate::boxlp::Stats::default();
            let t = Instant::now();
            let r = crate::boxlp::box_chain(e, f, center, Some(feas[0]), &mut st);
            add(s::SH_SV_NS, t.elapsed().as_nanos() as u64);
            add(s::SH_SV_STEPS, st.steps);
            add(s::SH_SV_ENGLP, st.engine_lps);
            add(s::SH_SV_FB, st.fallbacks);
            if let Some((clo, chi, cpts)) = r {
                let mut dv = 0;
                for k in 0..n {
                    if (clo[k] - lo[k]).abs() > 1e-9 * (1.0 + lo[k].abs()) {
                        dv += 1;
                    }
                    if (chi[k] - hi[k]).abs() > 1e-9 * (1.0 + hi[k].abs()) {
                        dv += 1;
                    }
                }
                add(s::SH_SV_VALDIFF, dv);
                let wt = tight_set(&cpts);
                add(s::SH_SV_TD, wt.iter().zip(&cold_t).filter(|(a, b)| a != b).count() as u64);
            }
            if classes.len() == e.rows {
                for p in 0..e.rows {
                    if classes[p] == 3 || classes[p] == 4 {
                        add(s::SH_LPKEEP_ROWS, 1);
                        if feas.iter().any(|x| (dot(e.row(p), x) - f[p]).abs() <= 1e-9 * (1.0 + f[p].abs())) {
                            add(s::SH_SETTLED, 1);
                        }
                    }
                }
            }
        }
    }
}

struct Rec {
    verdict: HashMap<(u8, usize), bool>,
    bx: Option<(Vec<f64>, Vec<f64>, Vec<Vec<f64>>)>,
}

static STORE: OnceLock<Mutex<HashMap<Vec<usize>, Rec>>> = OnceLock::new();

thread_local! {
    static PARENT: RefCell<Option<Vec<usize>>> = const { RefCell::new(None) };
    static ENTRY: RefCell<Option<(Vec<f64>, f64)>> = const { RefCell::new(None) };
    static HINT: RefCell<Option<Vec<f64>>> = const { RefCell::new(None) };
    static CLASSES: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static TABROWS: RefCell<Option<HashMap<(u8, usize), (Vec<f64>, f64)>>> = const { RefCell::new(None) };
    static SHARED: RefCell<Vec<Vec<f64>>> = const { RefCell::new(Vec::new()) };
    static BOX: RefCell<Option<(Vec<f64>, Vec<f64>, Vec<Vec<f64>>)>> = const { RefCell::new(None) };
}

pub fn set_tab_rows(m: Option<HashMap<(u8, usize), (Vec<f64>, f64)>>) {
    TABROWS.with(|p| *p.borrow_mut() = m);
}
/// Per-region neighbour-search record (mplcp, census only), 12 numbers per popped region:
/// [nbr_ns, fast_ns, none_ns, case2_ns, rank1_ns, fast_calls, none_calls, case2_calls, rank1_emitted, fresh_case1, fresh_case2, one_index_unseen]
pub const REGREC: usize = 12;
thread_local! {
    static REGS: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
}
pub fn regs_reset() {
    REGS.with(|r| r.borrow_mut().clear());
}
pub fn regs_push(rec: [f64; REGREC]) {
    REGS.with(|r| r.borrow_mut().extend_from_slice(&rec));
}
/// Writes the records of this thread as little-endian f64 to `path`.
pub fn regs_dump(path: &str) {
    REGS.with(|r| {
        let bytes: Vec<u8> = r.borrow().iter().flat_map(|v| v.to_le_bytes()).collect();
        let _ = std::fs::write(path, bytes);
    });
}
/// Snapshot of the counters whose per-region deltas split the neighbour search.
pub fn reg_snap() -> [u64; 9] {
    use crate::census::{cur, Counter as C};
    let r1: u64 = [C::R1Nanos0, C::R1Nanos1, C::R1Nanos2, C::R1Nanos3, C::R1Nanos4, C::R1Nanos5].iter().map(|&k| cur(k)).sum();
    [cur(C::AdjFastNanos), cur(C::AdjNoneNanos), cur(C::AdjCase2Nanos), r1, cur(C::AdjFast), cur(C::AdjNone), cur(C::AdjCase2), cur(C::R1Emitted), 0]
}

// ------------------------------------------------------------------ Case-2 degeneracy and connectivity (mplcp)

/// `RPOPT_CONN=1`: the exploration passes no `Seen` map to the neighbour search, so that crossings to known regions are
/// returned too (the main loop drops them as before); the regions found do not change.
pub fn conn_on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("RPOPT_CONN").map_or(false, |v| v == "1"))
}

#[derive(Default)]
struct Conn {
    uf1: Vec<usize>,
    uf2: Vec<usize>,
    edges: Vec<(usize, usize, u8)>,
    cur: [u32; 5],
    tot: [u64; 5],
    flag_any: u64,
    flag_subst: u64,
    isolated: u64,
    unclassified: u64,
    regions: u64,
}

thread_local! {
    static CONN: RefCell<Conn> = RefCell::new(Conn::default());
}

fn uf_find(v: &mut Vec<usize>, mut x: usize) -> usize {
    while v[x] != x {
        v[x] = v[v[x]];
        x = v[x];
    }
    x
}
fn uf_union(v: &mut Vec<usize>, a: usize, b: usize) {
    let n = a.max(b) + 1;
    while v.len() < n {
        let l = v.len();
        v.push(l);
    }
    let (ra, rb) = (uf_find(v, a), uf_find(v, b));
    if ra != rb {
        v[rb] = ra;
    }
}

pub fn conn_reset() {
    CONN.with(|c| *c.borrow_mut() = Conn::default());
}

/// Facet classes of one region: 0 boundary (no neighbour), 1 Case 1 (also the entry facet), 2 single valid swap (rank-1, |P-| = 1),
/// 3 several candidates (rank-1 with |P-| >= 2, walks, fallbacks), 4 other (nothing counted).
pub fn conn_facet(class: u8) {
    CONN.with(|c| c.borrow_mut().cur[class as usize] += 1);
}

/// Counter snapshot for classifying one `adjacent` call: [AdjNone, AdjFast, AdjCase2, R1Handled, R1Single, 0].
pub fn facet_snap() -> [u64; 6] {
    use crate::census::{cur, Counter as C};
    [cur(C::AdjNone), cur(C::AdjFast), cur(C::AdjCase2), cur(C::R1Handled), cur(C::R1Single), 0]
}

pub fn facet_class(a: &[u64; 6], b: &[u64; 6]) -> u8 {
    let d = |i: usize| b[i] - a[i];
    if d(0) > 0 {
        0
    } else if d(1) > 0 {
        1
    } else if d(2) > 0 {
        if d(3) > 0 && d(4) > 0 && d(5) == 0 {
            2
        } else {
            3
        }
    } else {
        4
    }
}

/// A crossing of region `a` to region `b` (kind 1 = Case 1, 2 = single swap): variant 1 joins Case 1 only, variant 2 both.
pub fn conn_union(a: usize, b: usize, kind: u8) {
    CONN.with(|c| {
        let c = &mut *c.borrow_mut();
        if kind >= 2 {
            c.edges.push((a, b, kind));
        }
        if kind == 1 {
            uf_union(&mut c.uf1, a, b);
        }
        if kind == 1 || kind == 2 {
            uf_union(&mut c.uf2, a, b);
        }
    });
}

/// The facet classes of the region just explored are folded into the totals and the flags.
pub fn conn_region_done(idx: usize) {
    CONN.with(|c| {
        let c = &mut *c.borrow_mut();
        for v in [&mut c.uf1, &mut c.uf2] {
            while v.len() <= idx {
                let l = v.len();
                v.push(l);
            }
        }
        c.regions += 1;
        let k = c.cur;
        c.cur = [0; 5];
        if k.iter().sum::<u32>() == 0 {
            c.unclassified += 1;
            return;
        }
        for i in 0..5 {
            c.tot[i] += k[i] as u64;
        }
        let interior = k[1] + k[2] + k[3];
        if interior == 0 {
            c.isolated += 1;
        } else if k[1] == 0 {
            c.flag_any += 1;
            if k[2] == 0 {
                c.flag_subst += 1;
            }
        }
    });
}

/// (name, value) pairs for the census JSON: regions explored, facet totals by class, flagged regions, components of both variants.
pub fn conn_report(regions: usize) -> Vec<(&'static str, f64)> {
    CONN.with(|c| {
        let c = &mut *c.borrow_mut();
        let mut out = vec![("regions_explored", c.regions as f64), ("facets_boundary", c.tot[0] as f64), ("facets_case1", c.tot[1] as f64), ("facets_single_swap", c.tot[2] as f64), ("facets_several", c.tot[3] as f64), ("facets_other", c.tot[4] as f64), ("flag_any", c.flag_any as f64), ("flag_substantive", c.flag_subst as f64), ("isolated", c.isolated as f64), ("unclassified", c.unclassified as f64)];
        let edges = c.edges.clone();
        for (name, which) in [("v1", 1), ("v2", 2)] {
            let v = if which == 1 { &mut c.uf1 } else { &mut c.uf2 };
            while v.len() < regions {
                let l = v.len();
                v.push(l);
            }
            let mut size = std::collections::HashMap::new();
            for i in 0..regions {
                let r = uf_find(v, i);
                *size.entry(r).or_insert(0usize) += 1;
            }
            let r0 = uf_find(v, 0);
            let start = *size.get(&r0).unwrap_or(&0);
            let mut sizes: Vec<usize> = size.values().copied().collect();
            sizes.sort_unstable();
            let med = if sizes.is_empty() { 0 } else { sizes[sizes.len() / 2] };
            let singles = sizes.iter().filter(|&&s| s == 1).count();
            // Case-2 crossings that join two different components (variant 1: single swaps and several-candidate facets;
            // variant 2: several-candidate facets only), and the distinct pairs of components they join
            let mut cross = 0usize;
            let mut pairs = std::collections::HashSet::new();
            for &(a, b, k) in &edges {
                if which == 2 && k != 3 {
                    continue;
                }
                let (ra, rb) = (uf_find(v, a), uf_find(v, b));
                if ra != rb {
                    cross += 1;
                    pairs.insert((ra.min(rb), ra.max(rb)));
                }
            }
            let names: [&'static str; 6] = if which == 1 { ["median_size_v1", "max_size_v1", "singletons_v1", "case2_cross_v1", "case2_pairs_v1", "x"] } else { ["median_size_v2", "max_size_v2", "singletons_v2", "case2_cross_v2", "case2_pairs_v2", "x"] };
            out.push((names[0], med as f64));
            out.push((names[1], *sizes.last().unwrap_or(&0) as f64));
            out.push((names[2], singles as f64));
            out.push((names[3], cross as f64));
            out.push((names[4], pairs.len() as f64));
            let (a, b, cc) = match name {
                "v1" => ("components_v1", "start_size_v1", "outside_v1"),
                _ => ("components_v2", "start_size_v2", "outside_v2"),
            };
            out.push((a, size.len() as f64));
            out.push((b, start as f64));
            out.push((cc, (regions - start) as f64));
        }
        out
    })
}

pub fn set_parent(a: &[usize]) {
    PARENT.with(|p| *p.borrow_mut() = Some(a.to_vec()));
}
pub fn set_entry(e: Option<&(Vec<f64>, f64)>, hint: Option<&[f64]>) {
    ENTRY.with(|p| *p.borrow_mut() = e.cloned());
    HINT.with(|p| *p.borrow_mut() = hint.map(|h| h.to_vec()));
}
/// Called by `lp_flags`: the decision class of every row (0 propagated-box reject, 1 exact-box reject,
/// 2 box vertex, 3 projection, 4 LP kept, 5 LP dropped) and the exact box with its 2n points.
pub fn set_classes(c: Vec<u8>, bx: Option<(Vec<f64>, Vec<f64>, Vec<Vec<f64>>)>) {
    CLASSES.with(|p| *p.borrow_mut() = c);
    BOX.with(|p| *p.borrow_mut() = bx);
}

fn rank_aff(pts: &[&Vec<f64>]) -> usize {
    if pts.is_empty() {
        return 0;
    }
    let n = pts[0].len();
    let scale = pts.iter().flat_map(|p| p.iter()).fold(1e-300f64, |a, v| a.max(v.abs()));
    let mut q: Vec<Vec<f64>> = vec![];
    for p in &pts[1..] {
        let mut v: Vec<f64> = (0..n).map(|k| p[k] - pts[0][k]).collect();
        for u in &q {
            let c = dot(&v, u);
            for k in 0..n {
                v[k] -= c * u[k];
            }
        }
        let nv = crate::linalg::norm(&v);
        if nv > 1e-7 * scale {
            for k in 0..n {
                v[k] /= nv;
            }
            q.push(v);
        }
    }
    q.len() + 1
}

/// Farkas certificates for the redundant rows and the ridge rule (count only).
fn farkas_ridge(e: &Mat, f: &[f64], flags: &[bool], classes: &[u8], entry_p: Option<usize>, parent_kept: &[usize]) {
    let (m, n) = (e.rows, e.cols);
    let kept: Vec<usize> = (0..m).filter(|&p| flags[p]).collect();
    let have = classes.len() == m;
    for k in 0..m {
        if flags[k] {
            continue;
        }
        let cls = if have { classes[k] } else { 5 };
        let slot = match cls {
            0 => 0,
            1 => 2,
            _ => 4,
        };
        let ak = e.row(k);
        // candidate supports
        let mut by_cos: Vec<usize> = kept.clone();
        by_cos.sort_by(|&a, &b| dot(e.row(b), ak).partial_cmp(&dot(e.row(a), ak)).unwrap_or(std::cmp::Ordering::Equal));
        let sup1: Vec<usize> = by_cos.iter().copied().take(n).collect();
        let sup2: Vec<usize> = by_cos.iter().copied().take(2 * n).collect();
        let sup3: Vec<usize> = parent_kept.iter().copied().filter(|&p| p != k && flags[p]).collect();
        let sup4: Vec<usize> = kept.clone();
        for (v, sup) in [sup1, sup2, sup3, sup4].iter().enumerate() {
            let base = s::FK_BASE + v * s::FK_STRIDE;
            add(base + slot, 1);
            if sup.is_empty() {
                continue;
            }
            let t = Instant::now();
            let caught = if v == 0 && sup.len() == n {
                // square system A_S' lambda = a_k
                let mut a = Mat::zeros(n, n);
                for (j, &r) in sup.iter().enumerate() {
                    for q in 0..n {
                        a[(q, j)] = e[(r, q)];
                    }
                }
                match crate::linalg::Lu::new(&a) {
                    Some(lu) => {
                        let lam = lu.solve(ak);
                        let bsum: f64 = sup.iter().zip(&lam).map(|(&r, l)| l * f[r]).sum();
                        lam.iter().all(|&l| l >= -1e-12) && bsum <= f[k] + 1e-9 * (1.0 + f[k].abs())
                    }
                    None => false,
                }
            } else {
                // min b' lambda s.t. A_S' lambda = a_k, lambda >= 0
                let q = sup.len();
                let mut a = Mat::zeros(q + n, q);
                let mut b = vec![0.0; q + n];
                for j in 0..q {
                    a[(j, j)] = -1.0;
                }
                for d in 0..n {
                    for (j, &r) in sup.iter().enumerate() {
                        a[(q + d, j)] = e[(r, d)];
                    }
                    b[q + d] = ak[d];
                }
                let eq: Vec<usize> = (q..q + n).collect();
                let c: Vec<f64> = sup.iter().map(|&r| f[r]).collect();
                match crate::solver::lp::solve_lp(Some(&c), &a, &b, &eq) {
                    Some(sol) => sol.obj <= f[k] + 1e-9 * (1.0 + f[k].abs()),
                    None => false,
                }
            };
            add(base + 6, t.elapsed().as_nanos() as u64);
            add(base + 7, 1);
            if caught {
                add(base + slot + 1, 1);
            }
        }
    }
    // ridge rule
    let shared = SHARED.with(|c| std::mem::take(&mut *c.borrow_mut()));
    if let Some(ep) = entry_p {
        if !shared.is_empty() {
            add(s::RG_CHILDREN, 1);
            add(s::RG_PTS, shared.len() as u64);
            if shared.len() + 1 < n {
                add(s::RG_NOENOUGH, 1);
            }
            let tight: Vec<Vec<usize>> = (0..m).map(|r| (0..shared.len()).filter(|&i| (dot(e.row(r), &shared[i]) - f[r]).abs() <= 1e-9 * (1.0 + f[r].abs())).collect()).collect();
            add(s::RG_FACETS, (0..m).filter(|&r| r != ep && flags[r]).count() as u64);
            for r in 0..m {
                if r == ep || tight[r].len() + 1 < n {
                    continue;
                }
                let pts: Vec<&Vec<f64>> = tight[r].iter().map(|&i| &shared[i]).collect();
                if rank_aff(&pts) + 1 < n {
                    continue;
                }
                add(s::RG_RIDGE, 1);
                // unique: no other row (besides the entry) is tight at the same affinely independent set, tested by any other row tight at all of these points
                let dup = (0..m).any(|o| o != r && o != ep && tight[o].len() >= tight[r].len() && tight[r].iter().all(|i| tight[o].contains(i)));
                if dup {
                    add(s::RG_AMBIG, 1);
                    if !flags[r] {
                        add(s::RG_AMBIG_WRONG, 1);
                    }
                } else {
                    add(s::RG_UNIQUE, 1);
                    if flags[r] {
                        add(s::RG_UNIQUE_FACET, 1);
                    } else {
                        add(s::RG_UNIQUE_WRONG, 1);
                    }
                }
            }
        }
    }
}

fn first_hit(e: &Mat, f: &[f64], c: &[f64], d: &[f64]) -> Option<(usize, f64, bool)> {
    let mut best = (usize::MAX, f64::INFINITY);
    let mut second = f64::INFINITY;
    for i in 0..e.rows {
        let a = e.row(i);
        let ad = dot(a, d);
        if ad <= 1e-12 {
            continue;
        }
        let t = (f[i] - dot(a, c)) / ad;
        if t < best.1 {
            second = best.1;
            best = (i, t);
        } else if t < second {
            second = t;
        }
    }
    if best.0 == usize::MAX {
        return None;
    }
    Some((best.0, best.1, second - best.1 > 1e-9 * (1.0 + best.1.abs())))
}

/// Clarkson-style redundancy removal from the interior point `c`, with the given seed rows; returns
/// the verdicts and bumps the counters of the variant at `base`.
fn sim(e: &Mat, f: &[f64], c: &[f64], seed: &[usize], flags: &[bool], base: usize, pre: bool) {
    let m = e.rows;
    let n = e.cols;
    let mut in_s = vec![false; m];
    let mut s_list: Vec<usize> = vec![];
    for &p in seed {
        if !in_s[p] {
            in_s[p] = true;
            s_list.push(p);
        }
    }
    let mut known = vec![false; m];
    let slack: Vec<f64> = (0..m).map(|i| f[i] - dot(e.row(i), c)).collect();
    let mut order: Vec<usize> = (0..m).collect();
    order.sort_by(|&a, &b| slack[a].partial_cmp(&slack[b]).unwrap_or(std::cmp::Ordering::Equal));
    let (mut lps, mut lprows, mut rays, mut lpns, mut rayns, mut mism) = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let (mut rayproof, mut facet_ray, mut redund, mut lps_keep) = (0u64, 0u64, 0u64, 0u64);
    if pre {
        // phase 1: rays along every row normal and along +-e_k find facets without an LP
        let mut dirs: Vec<Vec<f64>> = (0..m).map(|i| e.row(i).to_vec()).collect();
        for k in 0..n {
            for sg in [1.0, -1.0] {
                let mut d = vec![0.0; n];
                d[k] = sg;
                dirs.push(d);
            }
        }
        for d in &dirs {
            let t = Instant::now();
            let hit = first_hit(e, f, c, d);
            rays += 1;
            rayns += t.elapsed().as_nanos() as u64;
            if let Some((j, _, true)) = hit {
                known[j] = true;
                if !in_s[j] {
                    in_s[j] = true;
                    s_list.push(j);
                }
            }
        }
    }
    for &r in &order {
        let mut row_lps = 0u64;
        let verdict: bool;
        if known[r] {
            verdict = true;
            rayproof += 1;
        } else {
            let t = Instant::now();
            let hit = first_hit(e, f, c, e.row(r));
            rays += 1;
            rayns += t.elapsed().as_nanos() as u64;
            let mut decided = None;
            if let Some((j, _, uniq)) = hit {
                if uniq {
                    known[j] = true;
                    if j == r {
                        decided = Some(true);
                        rayproof += 1;
                    } else if !in_s[j] {
                        in_s[j] = true;
                        s_list.push(j);
                    }
                }
            }
            verdict = match decided {
                Some(v) => v,
                None => {
                    let mut out = false;
                    for _ in 0..(m + 2) {
                        let rows: Vec<usize> = s_list.iter().copied().filter(|&i| i != r).collect();
                        let mut mat = Mat::zeros(rows.len() + 1, n);
                        let mut b = Vec::with_capacity(rows.len() + 1);
                        for (k, &i) in rows.iter().enumerate() {
                            mat.row_mut(k).copy_from_slice(e.row(i));
                            b.push(f[i]);
                        }
                        mat.row_mut(rows.len()).copy_from_slice(e.row(r));
                        b.push(f[r] + 1.0);
                        let obj: Vec<f64> = e.row(r).iter().map(|v| -v).collect();
                        let t = Instant::now();
                        let x = crate::solver::lp::lp_point_from(Some(&obj), &mat, &b, &[], c);
                        lpns += t.elapsed().as_nanos() as u64;
                        lps += 1;
                        row_lps += 1;
                        lprows += mat.rows as u64;
                        let Some(x) = x else {
                            out = false;
                            break;
                        };
                        if dot(e.row(r), &x) <= f[r] + 1e-9 * (1.0 + f[r].abs()) {
                            redund += 1;
                            out = false;
                            break;
                        }
                        let viol = (0..m).any(|i| i != r && dot(e.row(i), &x) > f[i] + 1e-9 * (1.0 + f[i].abs()));
                        if !viol {
                            facet_ray += 1;
                            out = true;
                            break;
                        }
                        let d: Vec<f64> = x.iter().zip(c).map(|(a, b)| a - b).collect();
                        let t = Instant::now();
                        let hit = first_hit(e, f, c, &d);
                        rays += 1;
                        rayns += t.elapsed().as_nanos() as u64;
                        match hit {
                            Some((j, _, _)) if j == r => {
                                facet_ray += 1;
                                out = true;
                                break;
                            }
                            Some((j, _, uniq)) if !in_s[j] => {
                                in_s[j] = true;
                                s_list.push(j);
                                if uniq {
                                    known[j] = true;
                                }
                            }
                            _ => {
                                out = true; // cannot happen unless degenerate; counted as a mismatch if wrong
                                break;
                            }
                        }
                    }
                    out
                }
            };
        }
        if verdict {
            lps_keep += row_lps;
        }
        if verdict != flags[r] {
            mism += 1;
        }
    }
    let b = base;
    add(b + s::O_LPS, lps);
    add(b + s::O_LPROWS, lprows);
    add(b + s::O_RAYS, rays);
    add(b + s::O_LP_NS, lpns);
    add(b + s::O_RAY_NS, rayns);
    add(b + s::O_MISMATCH, mism);
    add(b + s::O_SFINAL, s_list.len() as u64);
    add(b + s::O_RAYPROOF, rayproof);
    add(b + s::O_FACET_RAY, facet_ray);
    add(b + s::O_REDUND, redund);
    add(b + s::O_LPS_PER_KEEP, lps_keep);
}

/// Called at the end of a region build on the LP path.
pub fn on_built(program: &crate::program::MpProgram, active_set: &[usize], e: &Mat, f: &[f64], center: &[f64], pos: &[usize], n_lambda: usize, inactive: &[usize], flags: &[bool]) {
    if !on() {
        return;
    }
    crate::census_purpose!(Other);
    let t_all = Instant::now();
    let m = e.rows;
    let n = e.cols;
    let neq = program.num_equality_constraints();
    let n_inactive = inactive.len();
    add(s::REG, 1);
    add(s::ROWS, m as u64);
    add(s::FACETS, flags.iter().filter(|&&v| v).count() as u64);
    let classes = CLASSES.with(|c| std::mem::take(&mut *c.borrow_mut()));
    let bx = BOX.with(|c| c.borrow_mut().take());
    if let Some(b) = &bx {
        let entry0 = ENTRY.with(|c| c.borrow().clone());
        let mut ep0 = None;
        if let Some((ea, ef)) = &entry0 {
            for p in 0..m {
                let a = e.row(p);
                let same = a.iter().zip(ea).all(|(x, y)| (x - y).abs() < 1e-7) && (f[p] - ef).abs() < 1e-7 * (1.0 + ef.abs());
                let rev = a.iter().zip(ea).all(|(x, y)| (x + y).abs() < 1e-7) && (f[p] + ef).abs() < 1e-7 * (1.0 + ef.abs());
                if same || rev {
                    ep0 = Some(p);
                    break;
                }
            }
        }
        let pkey = PARENT.with(|c| c.borrow().clone());
        let ppts: Option<Vec<Vec<f64>>> = pkey.and_then(|k| STORE.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap().get(&k).and_then(|r| r.bx.as_ref().map(|b| b.2.clone())));
        box_study(e, f, center, b, ep0, ppts.as_deref(), &classes);
    }
    let have_classes = classes.len() == m;
    // row p -> (kind, id)
    let mut key: Vec<(u8, usize)> = vec![(9, 0); m];
    for (r, &p) in pos.iter().enumerate() {
        if p == usize::MAX {
            continue;
        }
        key[p] = if r < n_lambda {
            (0, active_set[neq + r])
        } else if r < n_lambda + n_inactive {
            (1, inactive[r - n_lambda])
        } else {
            (2, r - n_lambda - n_inactive)
        };
    }
    if have_classes {
        let mut cnt = [0u64; 6];
        for &c in &classes {
            cnt[c as usize] += 1;
        }
        add(s::PROP_REJ, cnt[0]);
        add(s::BOX_REJ, cnt[1]);
        add(s::VERT_KEEP, cnt[2]);
        add(s::PROJ_KEEP, cnt[3]);
        add(s::LP_KEEP, cnt[4]);
        add(s::LP_DROP, cnt[5]);
        add(s::LPS_ALL, cnt[4] + cnt[5]);
        add(s::ROWS_REMAINING_AFTER_PROP, m as u64 - cnt[0]);
        // propagated box with 30 rounds
        if let Some((l, h)) = program.theta_bbox() {
            let (mut lo, mut hi) = (l.clone(), h.clone());
            crate::mpqp_utils::propagate_box_pub(e, f, &mut lo, &mut hi, 30);
            let rej = (0..m).filter(|&p| crate::mpqp_utils::box_max_pub(e.row(p), &lo, &hi) < f[p] - 1e-7 * (1.0 + f[p].abs())).count();
            add(s::PROP30_REJ, rej as u64);
        }
    }
    // entry facet
    let entry = ENTRY.with(|c| c.borrow().clone());
    let mut entry_p = None;
    if let Some((ea, ef)) = &entry {
        add(s::HAS_ENTRY, 1);
        for p in 0..m {
            let a = e.row(p);
            let same = a.iter().zip(ea).all(|(x, y)| (x - y).abs() < 1e-7) && (f[p] - ef).abs() < 1e-7 * (1.0 + ef.abs());
            let rev = a.iter().zip(ea).all(|(x, y)| (x + y).abs() < 1e-7) && (f[p] + ef).abs() < 1e-7 * (1.0 + ef.abs());
            if same || rev {
                entry_p = Some(p);
                break;
            }
        }
        if let Some(p) = entry_p {
            add(s::ENTRY_FOUND, 1);
            if have_classes {
                match classes[p] {
                    1 => add(s::ENTRY_BOXREJ, 1),
                    2 => add(s::ENTRY_VERT, 1),
                    3 => add(s::ENTRY_PROJ, 1),
                    4 => add(s::ENTRY_LPKEEP, 1),
                    _ => {}
                }
            }
        }
    }
    // tableau rows against the KKT rows
    if let Some(tm) = TABROWS.with(|c| c.borrow_mut().take()) {
        add(s::TABC_REG, 1);
        let mut worst = 0.0f64;
        for p in 0..m {
            if key[p].0 > 1 {
                continue;
            }
            add(s::TABC_ROWS, 1);
            match tm.get(&key[p]) {
                None => add(s::TABC_MISSING, 1),
                Some((te, tf)) => {
                    let mut d = (f[p] - tf).abs() / (1.0 + f[p].abs());
                    for k in 0..n {
                        d = d.max((e[(p, k)] - te[k]).abs());
                    }
                    worst = worst.max(d);
                }
            }
        }
        for (i, th) in [1e-12, 1e-10, 1e-8, 1e-6, 1e-4, 1e-2].iter().enumerate() {
            if worst > *th {
                add(s::TABC_T0 + i, 1);
            }
        }
    }
    {
        let pk: Vec<usize> = if let Some(pkey) = PARENT.with(|c| c.borrow().clone()) {
            let st = STORE.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
            match st.get(&pkey) {
                Some(pr) => (0..m).filter(|&p| pr.verdict.get(&key[p]).copied().unwrap_or(false)).collect(),
                None => vec![],
            }
        } else {
            vec![]
        };
        farkas_ridge(e, f, flags, &classes, entry_p, &pk);
    }
    // hint margin
    if let Some(h) = HINT.with(|c| c.borrow().clone()) {
        if h.len() == n {
            let mg = (0..m).map(|i| f[i] - dot(e.row(i), &h)).fold(f64::INFINITY, f64::min);
            add(s::HINT_N, 1);
            if mg > 0.0 {
                add(s::HINT_M0, 1);
            }
            if mg > 1e-9 {
                add(s::HINT_M1, 1);
            }
            if mg > 1e-6 {
                add(s::HINT_M2, 1);
            }
            if mg > 1e-4 {
                add(s::HINT_M3, 1);
            }
        }
    }
    // parent comparison
    let store = STORE.get_or_init(|| Mutex::new(HashMap::new()));
    let parent_key = PARENT.with(|c| c.borrow().clone());
    let mut parent_kept_rows: Vec<usize> = vec![];
    {
        let st = store.lock().unwrap();
        if let Some(pr) = parent_key.as_ref().and_then(|k| st.get(k)) {
            add(s::HAS_PARENT, 1);
            for p in 0..m {
                let Some(&pv) = pr.verdict.get(&key[p]) else { continue };
                let cv = flags[p];
                add(s::CMP_ROWS, 1);
                match (pv, cv) {
                    (true, true) => add(s::PK_CK, 1),
                    (true, false) => add(s::PK_CD, 1),
                    (false, true) => add(s::PD_CK, 1),
                    (false, false) => add(s::PD_CD, 1),
                }
                if pv {
                    parent_kept_rows.push(p);
                }
                if have_classes && classes[p] >= 4 {
                    add(s::LPS_CMP, 1);
                    if pv == cv {
                        add(s::LPS_AGREE, 1);
                    }
                    if !pv && !cv {
                        add(s::LPS_DROP_PD, 1);
                    }
                    if pv && cv {
                        add(s::LPS_KEEP_PK, 1);
                    }
                }
            }
            if let (Some((lo, hi, _)), Some((plo, phi, ppts))) = (&bx, &pr.bx) {
                for k in 0..n {
                    add(s::BOX_COORDS, 2);
                    if (lo[k] - plo[k]).abs() <= 1e-9 * (1.0 + lo[k].abs()) {
                        add(s::BOX_EQ_PARENT, 1);
                    }
                    if (hi[k] - phi[k]).abs() <= 1e-9 * (1.0 + hi[k].abs()) {
                        add(s::BOX_EQ_PARENT, 1);
                    }
                }
                let mut feas_pts: Vec<&Vec<f64>> = vec![];
                for (j, x) in ppts.iter().enumerate() {
                    add(s::PT_TOTAL, 1);
                    let ok = (0..m).all(|i| dot(e.row(i), x) <= f[i] + 1e-9 * (1.0 + f[i].abs()));
                    if ok {
                        add(s::PT_FEAS, 1);
                        feas_pts.push(x);
                        let k = j / 2;
                        let tgt = if j % 2 == 0 { lo[k] } else { hi[k] };
                        if (x[k] - tgt).abs() <= 1e-9 * (1.0 + tgt.abs()) {
                            add(s::PT_OPT, 1);
                        }
                    }
                }
                if have_classes {
                    for p in 0..m {
                        if classes[p] == 2 {
                            add(s::VK_ROWS, 1);
                            if feas_pts.iter().any(|x| (dot(e.row(p), x) - f[p]).abs() <= 1e-9 * (1.0 + f[p].abs())) {
                                add(s::VK_COVERED, 1);
                            }
                        }
                    }
                }
            }
        }
    }
    // Clarkson simulation
    let omega: Vec<usize> = (0..m).filter(|&p| key[p].0 == 2).collect();
    sim(e, f, center, &omega, flags, MD_SIM(s::SIM_A), false);
    sim(e, f, center, &omega, flags, MD_SIM(s::SIM_C), true);
    let mut seed_b = omega.clone();
    if let Some(p) = entry_p {
        seed_b.push(p);
    }
    seed_b.extend(parent_kept_rows);
    sim(e, f, center, &seed_b, flags, MD_SIM(s::SIM_B), false);
    // store this region as a possible parent
    let mut verdict = HashMap::new();
    for p in 0..m {
        if key[p].0 != 9 {
            verdict.insert(key[p], flags[p]);
        }
    }
    store.lock().unwrap().insert(active_set.to_vec(), Rec { verdict, bx });
    lap(s::DIAG_NS, t_all);
}

#[allow(non_snake_case)]
fn MD_SIM(b: usize) -> usize {
    b
}

/// Reads a slot of a snapshot.
pub fn slot(st: &crate::census::Stats, i: usize) -> f64 {
    st.counters[MD_BASE + i] as f64
}
