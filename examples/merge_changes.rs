//! For each problem file: the constraint count of the processed program with and without the duplicate-row merge of the
//! preprocessing. Prints `<name> <rows_without> <rows_with>`; the instances where the two differ are the ones whose
//! stored solutions change index set between a build with and a build without the merge (used by the solution archive).
//!
//!   merge_changes <problem.json>...
use rpopt::program::MpProgram;

fn main() {
    for f in std::env::args().skip(1) {
        let path = std::path::Path::new(&f);
        let without = MpProgram::from_json_file_opts(path, false).unwrap().num_constraints();
        let with = MpProgram::from_json_file_opts(path, true).unwrap().num_constraints();
        println!("{} {without} {with}", path.file_stem().unwrap().to_string_lossy());
    }
}
