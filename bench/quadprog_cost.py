"""
Per-call cost of a quadprog QP solve done the way PPOPT's quad_prog_interface does it (reorder rows,
symmetrise Q, call quadprog.solve_qp, rebuild multipliers, slack and active set). Does not run PPOPT.

Usage:
    python quadprog_cost.py problems/masses2_N6.json [n]
"""

import json
import sys
import time

import numpy
import quadprog


def solve_qp_like_ppopt(Q, c, A, b, eq):
    num_constraints = A.shape[0]
    ineq = [i for i in range(A.shape[0]) if i not in eq]
    if eq:
        new_A = A[[*eq, *ineq]]
        new_b = b[[*eq, *ineq]]
    else:
        new_A, new_b = A, b
    Q_ = .5 * (Q + Q.T)
    try:
        sol = quadprog.solve_qp(G=Q_, a=-c.flatten(), C=-new_A.T, b=-new_b.flatten(), meq=len(eq))
    except ValueError:
        return None
    lagrange = numpy.zeros(num_constraints)
    x_star = sol[0]
    lagrange[[*eq, *ineq]] = sol[4]
    lagrange[ineq] = -lagrange[ineq]
    slack = b - A @ x_star.reshape(-1, 1)
    active = [i for i in range(num_constraints) if abs(slack[i]) <= 1e-10 or lagrange[i] != 0]
    return x_star, numpy.array(active)


def main():
    d = json.load(open(sys.argv[1]))
    n = int(sys.argv[2]) if len(sys.argv) > 2 else 2000
    A = numpy.array(d['A'], float)
    b = numpy.array(d['b'], float)
    F = numpy.array(d['F'], float)
    Q = numpy.array(d['Q'], float)
    H = numpy.array(d['H'], float)
    c = numpy.array(d['c'], float)
    lo = -numpy.array(d['b_t'], float).flatten()[F.shape[1]:]
    hi = numpy.array(d['b_t'], float).flatten()[:F.shape[1]]
    eq = d.get('equality_indices', [])
    rng = numpy.random.default_rng(0)
    thetas = [rng.uniform(lo, hi).reshape(-1, 1) for _ in range(n)]
    t0 = time.perf_counter()
    for th in thetas:
        solve_qp_like_ppopt(Q, H @ th + c, A, b + F @ th, eq)
    t1 = time.perf_counter()
    print(json.dumps({'problem': sys.argv[1], 'qps': n, 'us_per_qp': 1e6 * (t1 - t0) / n}))


if __name__ == '__main__':
    main()
