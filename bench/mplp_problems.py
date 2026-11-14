"""
Explicit-MPC mpLP benchmarks: linear MPC with an infinity-norm or 1-norm cost
(Bemporad, Borrelli and Morari, IEEE TAC 47(12), 2002), written to JSON in PPOPT's format
without "Q" (so RPOPT reads them as mpLPs):

    min_z  c'z   s.t.  A z <= b + F theta,   A_t theta <= b_t,   theta = x(0)

z = [U, eps]: inputs U = [u_0, .., u_{N-1}] (states condensed out) and epigraph variables eps.
  inf-norm: J = sum_{k=1}^{N} ||Qx x_k||_inf + sum_{k=0}^{N-1} ||Ru u_k||_inf, one eps per term,
  1-norm:   J = sum_{k=1}^{N} ||Qx x_k||_1   + sum_{k=0}^{N-1} ||Ru u_k||_1,   one eps per row.
Box constraints on u_k and on x_k for k = 1..N; theta is boxed by the state bounds.
The inf-norm problems are primal and dual degenerate, the usual stress test for mpLP solvers.

Usage:
    python mplp_problems.py <output_dir>
"""

import json
import sys
from pathlib import Path

import numpy
from scipy.linalg import expm


def norm_mpc(A, B, Qx, Ru, N, x_bound, u_bound, norm):
    nx, nu = B.shape
    Phi, Gamma = [], []
    for k in range(1, N + 1):
        Phi.append(numpy.linalg.matrix_power(A, k))
        g = numpy.zeros((nx, nu * N))
        for j in range(k):
            g[:, j * nu:(j + 1) * nu] = numpy.linalg.matrix_power(A, k - 1 - j) @ B
        Gamma.append(g)

    nU = nu * N
    if norm == 'inf':
        ex = [[nU + k] for k in range(N)]                       # eps of x_{k+1}
        eu = [[nU + N + k] for k in range(N)]                   # eps of u_k
        nz = nU + 2 * N
    else:
        qx, ru = Qx.shape[0], Ru.shape[0]
        ex = [[nU + k * qx + i for i in range(qx)] for k in range(N)]
        eu = [[nU + N * qx + k * ru + j for j in range(ru)] for k in range(N)]
        nz = nU + N * (qx + ru)

    rows_A, rows_b, rows_F = [], [], []

    def add(a, b, f):
        rows_A.append(a)
        rows_b.append(b)
        rows_F.append(f)

    zero_t = numpy.zeros(nx)
    # input bounds
    for sign in (1.0, -1.0):
        for k in range(N):
            for j in range(nu):
                a = numpy.zeros(nz)
                a[k * nu + j] = sign
                add(a, u_bound[j], zero_t)
    # state bounds, k = 1..N
    for sign in (1.0, -1.0):
        for k in range(N):
            for i in range(nx):
                a = numpy.zeros(nz)
                a[:nU] = sign * Gamma[k][i]
                add(a, x_bound[i], -sign * Phi[k][i])
    # epigraph of the state terms:  +-Qx_i x_k <= eps
    for k in range(N):
        for i in range(Qx.shape[0]):
            for sign in (1.0, -1.0):
                a = numpy.zeros(nz)
                a[:nU] = sign * (Qx[i] @ Gamma[k])
                a[ex[k][0] if norm == 'inf' else ex[k][i]] = -1.0
                add(a, 0.0, -sign * (Qx[i] @ Phi[k]))
    # epigraph of the input terms:  +-Ru_j u_k <= eps
    for k in range(N):
        for j in range(Ru.shape[0]):
            for sign in (1.0, -1.0):
                a = numpy.zeros(nz)
                a[k * nu:(k + 1) * nu] = sign * Ru[j]
                a[eu[k][0] if norm == 'inf' else eu[k][j]] = -1.0
                add(a, 0.0, zero_t)

    c = numpy.zeros(nz)
    c[nU:] = 1.0
    A_t = numpy.vstack([numpy.eye(nx), -numpy.eye(nx)])
    b_t = numpy.concatenate([x_bound, x_bound])
    return dict(A=numpy.array(rows_A).tolist(), b=[[float(v)] for v in rows_b], F=numpy.array(rows_F).tolist(),
                c=[[float(v)] for v in c], H=numpy.zeros((nz, nx)).tolist(), A_t=A_t.tolist(),
                b_t=[[float(v)] for v in b_t], equality_indices=[])


def double_integrator(N, norm):
    A = numpy.array([[1.0, 1.0], [0.0, 1.0]])
    B = numpy.array([[0.5], [1.0]])
    return norm_mpc(A, B, numpy.eye(2), numpy.array([[1.0]]), N, numpy.array([4.0, 4.0]), numpy.array([1.0]), norm)


def two_input_system(N, norm):
    A = numpy.array([[1.1, 0.2, 0.0], [0.0, 0.9, 0.3], [0.1, 0.0, 0.95]])
    B = numpy.array([[0.0, 0.1], [0.1, 0.0], [0.05, 0.05]])
    return norm_mpc(A, B, numpy.eye(3), 0.1 * numpy.eye(2), N, numpy.array([5.0, 5.0, 5.0]), numpy.array([1.0, 1.0]), norm)


def oscillating_masses(N, norm, n_masses=2, Ts=0.5):
    n = n_masses
    K = 2.0 * numpy.eye(n) - numpy.eye(n, k=1) - numpy.eye(n, k=-1)
    Ac = numpy.block([[numpy.zeros((n, n)), numpy.eye(n)], [-K, numpy.zeros((n, n))]])
    Bc = numpy.zeros((2 * n, 1))
    Bc[n, 0] = 1.0
    M = expm(numpy.block([[Ac, Bc], [numpy.zeros((1, 2 * n + 1))]]) * Ts)
    return norm_mpc(M[:2 * n, :2 * n], M[:2 * n, 2 * n:], numpy.eye(2 * n), numpy.array([[1.0]]), N,
                    numpy.full(2 * n, 4.0), numpy.array([0.5]), norm)


BENCHMARKS = {}
for norm in ('inf', 'one'):
    for N in (2, 3, 4, 5, 6, 8, 10):
        BENCHMARKS[f'di{norm}_N{N}'] = (lambda N=N, norm=norm: double_integrator(N, norm))
    for N in (2, 3, 4, 5):
        BENCHMARKS[f'sys3x2{norm}_N{N}'] = (lambda N=N, norm=norm: two_input_system(N, norm))
        BENCHMARKS[f'masses2{norm}_N{N}'] = (lambda N=N, norm=norm: oscillating_masses(N, norm))


if __name__ == '__main__':
    out_dir = Path(sys.argv[1] if len(sys.argv) > 1 else 'problems_lp')
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, builder in BENCHMARKS.items():
        prob = builder()
        (out_dir / f'{name}.json').write_text(json.dumps(prob))
        print(name, 'vars', len(prob['c']), 'cons', len(prob['b']), 'params', len(prob['A_t'][0]))
