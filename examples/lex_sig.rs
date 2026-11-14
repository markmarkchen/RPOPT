//! lex_sig <problem> [sigfile] [reps]: time mplp_lex (median of reps) and dump sorted region signatures.
use rpopt::algorithms::mplp_lex;
use rpopt::problem_generator::generate_mplp;
use rpopt::program::MpProgram;
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let spec = &a[1];
    let prog = if let Some(r) = spec.strip_prefix("rand:") {
        let v: Vec<u64> = r.split(',').map(|s| s.parse().unwrap()).collect();
        generate_mplp(v[0] as usize, v[1] as usize, v[2] as usize, v[3])
    } else {
        MpProgram::from_json_file(std::path::Path::new(spec)).unwrap()
    };
    let reps: usize = a.get(3).map_or(1, |s| s.parse().unwrap());
    let mut ts = vec![];
    let mut regs = vec![];
    for _ in 0..reps {
        let t = Instant::now();
        regs = mplp_lex::regions(&prog, 0);
        ts.push(t.elapsed().as_secs_f64() * 1e3);
    }
    ts.sort_by(|x, y| x.total_cmp(y));
    println!("regions {} median_ms {:.3} min_ms {:.3}", regs.len(), ts[ts.len() / 2], ts[0]);
    if let Some(f) = a.get(2) {
        if f != "-" {
            let mut sigs: Vec<String> = regs
                .iter()
                .map(|r| {
                    let mut s = r.active_set.clone();
                    s.sort();
                    format!("{:?}", s)
                })
                .collect();
            sigs.sort();
            std::fs::write(f, sigs.join("\n")).unwrap();
        }
    }
}
