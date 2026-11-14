"""
Builds the explicit-MPC benchmark problems used to compare PPOPT (Python) with ppopt_rs (Rust).

Every problem is written to JSON in PPOPT's mpQP format:

    min_x  1/2 x'Qx + theta'H'x + c'x
    s.t.   A x <= b + F theta      (rows in `equality_indices` hold with equality)
           A_t theta <= b_t

so both implementations solve exactly the same matrices.

Usage:
    python mpc_problems.py <output_dir>
"""

import json
import sys
from pathlib import Path

import numpy
from scipy.linalg import expm, solve_discrete_are


def condensed_mpc(A, B, Qx, Ru, N, x_bound, u_bound, terminal_lqr=True):
    """
    Condensed (state-eliminated) linear MPC with box constraints on states and inputs.

    x_{k+1} = A x_k + B u_k, theta = x_0, decision U = [u_0, ..., u_{N-1}].
    Cost: sum_{k=1}^{N-1} x_k'Qx x_k + x_N' P x_N + sum_{k=0}^{N-1} u_k'Ru u_k (times 1/2).
    """
    nx, nu = B.shape
    P = solve_discrete_are(A, B, Qx, Ru) if terminal_lqr else Qx

    # prediction matrices x_k = Phi_k x0 + Gamma_k U
    Phi = []
    Gamma = []
    for k in range(1, N + 1):
        Phi.append(numpy.linalg.matrix_power(A, k))
        row = numpy.zeros((nx, nu * N))
        for j in range(k):
            row[:, j * nu:(j + 1) * nu] = numpy.linalg.matrix_power(A, k - 1 - j) @ B
        Gamma.append(row)

    Hq = numpy.kron(numpy.eye(N), Ru)
    Fq = numpy.zeros((nu * N, nx))
    for k in range(N):
        W = P if k == N - 1 else Qx
        Hq += Gamma[k].T @ W @ Gamma[k]
        Fq += Gamma[k].T @ W @ Phi[k]
    Hq = 0.5 * (Hq + Hq.T)

    rows_A, rows_b, rows_F = [], [], []
    # input bounds
    for sign in (1.0, -1.0):
        rows_A.append(sign * numpy.eye(nu * N))
        rows_b.append(numpy.tile(u_bound.reshape(-1, 1), (N, 1)))
        rows_F.append(numpy.zeros((nu * N, nx)))
    # state bounds for k = 1..N
    for k in range(N):
        for sign in (1.0, -1.0):
            rows_A.append(sign * Gamma[k])
            rows_b.append(x_bound.reshape(-1, 1))
            rows_F.append(-sign * Phi[k])

    A_c = numpy.vstack(rows_A)
    b_c = numpy.vstack(rows_b)
    F_c = numpy.vstack(rows_F)

    A_t = numpy.vstack([numpy.eye(nx), -numpy.eye(nx)])
    b_t = numpy.vstack([x_bound.reshape(-1, 1), x_bound.reshape(-1, 1)])

    return dict(Q=Hq, c=numpy.zeros((nu * N, 1)), H=Fq, A=A_c, b=b_c, F=F_c, A_t=A_t, b_t=b_t,
                equality_indices=[])


def double_integrator_noncondensed(N):
    """The PPOPT documentation example (doc/mpc.rst), generalised to horizon N, states kept as variables."""
    A_ss = numpy.array([[1.0, 1.0], [0.0, 1.0]])
    B_ss = numpy.array([[0.5], [1.0]])
    x_upper = numpy.array([[4.0], [4.0]])
    u_upper = numpy.array([[1.0]])
    nx, nu = 2, 1
    nv = N * nx + N * nu
    A_eq = numpy.zeros((N * nx, nv))
    for k in range(N):
        A_eq[k * nx:(k + 1) * nx, k * nx:(k + 1) * nx] = numpy.eye(nx)
        A_eq[k * nx:(k + 1) * nx, N * nx + k * nu: N * nx + (k + 1) * nu] = -B_ss
        if k > 0:
            A_eq[k * nx:(k + 1) * nx, (k - 1) * nx:k * nx] = -A_ss
    b_eq = numpy.zeros((N * nx, 1))
    F_eq = numpy.zeros((N * nx, nx))
    F_eq[0:nx, :] = A_ss
    A_ineq = numpy.vstack([numpy.eye(nv), -numpy.eye(nv)])
    ub = numpy.vstack([numpy.tile(x_upper, (N, 1)), numpy.tile(u_upper, (N, 1))])
    b_ineq = numpy.vstack([ub, ub])
    F_ineq = numpy.zeros((2 * nv, nx))
    return dict(Q=numpy.eye(nv), c=numpy.zeros((nv, 1)), H=numpy.zeros((nv, nx)),
                A=numpy.vstack([A_eq, A_ineq]), b=numpy.vstack([b_eq, b_ineq]), F=numpy.vstack([F_eq, F_ineq]),
                A_t=numpy.vstack([numpy.eye(nx), -numpy.eye(nx)]), b_t=numpy.vstack([x_upper, x_upper]),
                equality_indices=list(range(N * nx)))


def double_integrator(N):
    A = numpy.array([[1.0, 1.0], [0.0, 1.0]])
    B = numpy.array([[0.5], [1.0]])
    return condensed_mpc(A, B, numpy.eye(2), numpy.array([[1.0]]), N,
                         x_bound=numpy.array([4.0, 4.0]), u_bound=numpy.array([1.0]))


def oscillating_masses(N, n_masses=2, Ts=0.5):
    """Chain of unit masses joined by unit springs, one force input on the first mass (nx = 2*n_masses)."""
    n = n_masses
    K = 2.0 * numpy.eye(n) - numpy.eye(n, k=1) - numpy.eye(n, k=-1)
    Ac = numpy.block([[numpy.zeros((n, n)), numpy.eye(n)], [-K, numpy.zeros((n, n))]])
    Bc = numpy.zeros((2 * n, 1))
    Bc[n, 0] = 1.0
    M = expm(numpy.block([[Ac, Bc], [numpy.zeros((1, 2 * n + 1))]]) * Ts)
    A = M[:2 * n, :2 * n]
    B = M[:2 * n, 2 * n:]
    return condensed_mpc(A, B, numpy.eye(2 * n), numpy.array([[1.0]]), N,
                         x_bound=numpy.full(2 * n, 4.0), u_bound=numpy.array([0.5]))


def two_input_system(N):
    """A 3-state, 2-input open-loop unstable plant (common textbook-style benchmark)."""
    A = numpy.array([[1.1, 0.2, 0.0], [0.0, 0.9, 0.3], [0.1, 0.0, 0.95]])
    B = numpy.array([[0.0, 0.1], [0.1, 0.0], [0.05, 0.05]])
    return condensed_mpc(A, B, numpy.eye(3), 0.1 * numpy.eye(2), N,
                         x_bound=numpy.array([5.0, 5.0, 5.0]), u_bound=numpy.array([1.0, 1.0]))


BENCHMARKS = {
    'di_doc_N3': lambda: double_integrator_noncondensed(3),
    'di_N4': lambda: double_integrator(4),
    'di_N6': lambda: double_integrator(6),
    'di_N8': lambda: double_integrator(8),
    'di_N10': lambda: double_integrator(10),
    'masses2_N4': lambda: oscillating_masses(4),
    'masses2_N6': lambda: oscillating_masses(6),
    'sys3x2_N3': lambda: two_input_system(3),
    'sys3x2_N5': lambda: two_input_system(5),
}


def to_json(prob):
    out = {}
    for k, v in prob.items():
        out[k] = v.tolist() if isinstance(v, numpy.ndarray) else v
    return out


if __name__ == '__main__':
    out_dir = Path(sys.argv[1] if len(sys.argv) > 1 else 'problems')
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, builder in BENCHMARKS.items():
        prob = builder()
        with open(out_dir / f'{name}.json', 'w') as fh:
            json.dump(to_json(prob), fh)
        print(name, 'vars', prob['A'].shape[1], 'cons', prob['A'].shape[0], 'params', prob['F'].shape[1])
