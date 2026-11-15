//! `LpStatus`: how the last LP of a thread ended, on the small fast path (n <= 4), the revised path and the dense tableau (n = 20).
use rpopt::linalg::Mat;
use rpopt::solver::lp::{last_status, lp_feasible, lp_feasible_point, lp_point, LpStatus};
use rpopt::utils::chebyshev_ball_status;

/// The box |x_k| <= 1 in n dimensions as 2n rows.
fn boxn(n: usize, half: f64) -> (Mat, Vec<f64>) {
    let mut a = Mat::zeros(0, n);
    let mut b = vec![];
    for k in 0..n {
        let mut r = vec![0.0; n];
        r[k] = 1.0;
        a.push_row(&r);
        b.push(half);
        r[k] = -1.0;
        a.push_row(&r);
        b.push(half);
    }
    (a, b)
}

#[test]
fn feasible_and_infeasible_report_their_status() {
    for n in [2usize, 6, 20] {
        let (a, b) = boxn(n, 1.0);
        assert!(lp_feasible(&a, &b, &[]));
        assert_eq!(last_status(), LpStatus::Optimal, "n = {n}");
        // x_0 <= -2 and x_0 >= -1 -> empty, with a certificate
        let mut a2 = a.clone();
        let mut r = vec![0.0; n];
        r[0] = 1.0;
        a2.push_row(&r);
        let mut b2 = b.clone();
        b2.push(-2.0);
        assert!(!lp_feasible(&a2, &b2, &[]));
        assert_eq!(last_status(), LpStatus::Infeasible, "n = {n}");
        assert!(lp_feasible_point(&a2, &b2, &[], None).is_none());
        assert_eq!(last_status(), LpStatus::Infeasible, "n = {n}");
    }
}

#[test]
fn chebyshev_status() {
    let (a, b) = boxn(3, 1.0);
    let (sol, st) = chebyshev_ball_status(&a, &b, &[]);
    assert_eq!(st, LpStatus::Optimal);
    assert!((sol.unwrap().x[3] - 1.0).abs() < 1e-9);
    let (mut a2, mut b2) = (a.clone(), b.clone());
    a2.push_row(&[1.0, 0.0, 0.0]);
    b2.push(-3.0);
    let (sol, st) = chebyshev_ball_status(&a2, &b2, &[]);
    assert!(sol.is_none());
    assert_eq!(st, LpStatus::Infeasible);
    // a point LP leaves the status of its own call
    assert!(lp_point(Some(&[1.0, 0.0, 0.0]), &a, &b, &[]).is_some());
    assert_eq!(last_status(), LpStatus::Optimal);
}
