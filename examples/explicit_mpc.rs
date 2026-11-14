//! Explicit MPC for the double integrator (the PPOPT documentation example, doc/mpc.rst):
//! solve the mpQP, plot the partition, and evaluate the control law with several point location
//! methods.
//!
//! cargo run --release --example explicit_mpc

use ppopt_rs::algorithms::{solve_mpqp, MpqpAlgorithm};
use ppopt_rs::linalg::Mat;
use ppopt_rs::point_location::*;
use ppopt_rs::program::MpProgram;
use std::sync::Arc;

fn main() {
    // x_{t+1} = [1 1; 0 1] x_t + [0.5; 1] u_t, N = 3, |u| <= 1, |x| <= 4, states kept as variables
    let n = 3;
    let a_ss = [[1.0, 1.0], [0.0, 1.0]];
    let b_ss = [0.5, 1.0];
    let nv = n * 2 + n;
    let mut a = Mat::zeros(0, nv);
    let mut b = vec![];
    let mut f = Mat::zeros(0, 2);
    for k in 0..n {
        for r in 0..2 {
            let mut row = vec![0.0; nv];
            row[k * 2 + r] = 1.0;
            row[2 * n + k] = -b_ss[r];
            if k > 0 {
                row[(k - 1) * 2] = -a_ss[r][0];
                row[(k - 1) * 2 + 1] = -a_ss[r][1];
            }
            a.push_row(&row);
            b.push(0.0);
            f.push_row(&if k == 0 { a_ss[r].to_vec() } else { vec![0.0, 0.0] });
        }
    }
    let ub: Vec<f64> = (0..nv).map(|i| if i < 2 * n { 4.0 } else { 1.0 }).collect();
    for sign in [1.0, -1.0] {
        for i in 0..nv {
            let mut row = vec![0.0; nv];
            row[i] = sign;
            a.push_row(&row);
            b.push(ub[i]);
            f.push_row(&[0.0, 0.0]);
        }
    }
    let a_t = Mat::from_rows(&[vec![1.0, 0.0], vec![0.0, 1.0], vec![-1.0, 0.0], vec![0.0, -1.0]]);
    let prog = Arc::new(MpProgram::new_mpqp(a, b, vec![0.0; nv], Mat::zeros(nv, 2), Mat::eye(nv), a_t, vec![4.0; 4], f, (0..2 * n).collect()));

    let sol = solve_mpqp(&prog, MpqpAlgorithm::Combinatorial);
    println!("{} critical regions", sol.len());
    std::fs::write("explicit_mpc.svg", ppopt_rs::plot::parametric_plot_svg(&sol, 600.0, 600.0)).unwrap();
    println!("partition written to explicit_mpc.svg");

    let geom = PartitionGeometry::new(&sol);
    let lm = LinearMachine::new(&geom).expect("linear machine");
    let bst = BinarySearchTree::new(&geom);
    let x0 = [1.5, -0.5];
    let region = lm.locate(&x0).expect("x0 is feasible");
    assert_eq!(bst.locate(&x0), Some(region));
    let u = sol.critical_regions[region].evaluate(&x0);
    println!("x0 = {:?}: region {region}, active set {:?}, u0 = {:.4}", x0, sol.critical_regions[region].active_set, u[2 * n]);
    println!("direct QP: u0 = {:.4}", prog.solve_theta(&x0).unwrap().sol[2 * n]);
}
