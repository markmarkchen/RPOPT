//! The integration tests, compiled as one binary: one link instead of one per file.
//!
//! `mplcp_rows` and `mplcp_tableau_law` stay separate binaries (`tests/*.rs`). They flip the
//! process-wide switch `mplcp::set_tableau_law`, which would race with mplcp tests running in other
//! threads of this binary. A new test that changes process-wide state belongs there too.

mod ahmadi;
mod approx_methods;
mod combinatorial_kp;
mod dependent_start;
mod dp_mpc;
mod geometric_degenerate;
mod incremental_kkt;
mod mplcp;
mod mplp_graph_gaps;
mod mplp_lex;
mod mpmilp_bb;
mod mpmiqp_exact;
mod mpr;
mod opt_tree;
mod overlap_strict;
mod pdaqp;
mod pl_agreement;
mod pl_sequence;
mod pl_wnn;
mod ppopt_parity;
mod reduction;
mod solution_io;
