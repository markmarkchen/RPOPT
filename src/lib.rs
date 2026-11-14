//! # rpopt (RPOPT)
//!
//! A Rust rewrite of [PPOPT](https://github.com/TAMUparametric/PPOPT), the Python package for
//! multiparametric programming (mpLP, mpQP, mpMILP, mpMIQP), plus a family of point location
//! methods for evaluating explicit solutions online:
//!
//! | method | reference |
//! |---|---|
//! | sequential search | PPOPT `PointLocation` |
//! | binary search tree | Tøndel, Johansen, Bemporad (2003), Automatica 39(5) |
//! | linear machine | Airan, Bhushan, Bhartiya (2017), IEEE TAC 62(3); numerical check of Sharma & Bhushan (2020) |
//! | multiway tree | Mönnigmann & Kastsian (2011), IFAC World Congress |
//! | descriptor function | Baotić, Borrelli, Bemporad, Morari (2008), SIAM J. Control Optim. 47(5) |
//! | hash table | Bayat, Johansen, Jalali (2011), Automatica 47(3) |
//! | YANN | Braniff & Tian (2026), Computers & Chemical Engineering |

pub mod algorithms;
pub mod critical_region;
pub mod fxhash;
pub mod linalg;
pub mod mpqp_utils;
pub mod plot;
pub mod point_location;
pub mod problem_generator;
pub mod program;
pub mod solution;
pub mod solver;
pub mod stats;
pub mod utils;

pub use algorithms::{solve_mpqp, solve_mpqp_with, MpqpAlgorithm, SolveOptions};
pub use critical_region::CriticalRegion;
pub use linalg::Mat;
pub use program::MpProgram;
pub use solution::Solution;
