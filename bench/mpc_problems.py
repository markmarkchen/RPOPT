"""
Builds the explicit-MPC benchmark problems used to compare PPOPT (Python) with RPOPT (Rust).

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


def box_polytope(bound):
    """{z : |z_i| <= bound_i} as {H z <= k} with rows [+e_1..+e_n, -e_1..-e_n]."""
    bound = numpy.asarray(bound, dtype=float).reshape(-1)
    n = bound.size
    return dict(H=numpy.vstack([numpy.eye(n), -numpy.eye(n)]).tolist(), k=numpy.concatenate([bound, bound]).tolist())


def mpc_data(A, B, Qx, Ru, P, N, x_bound, u_bound, row_map, t_poly=None):
    """
    MPC structure of a benchmark problem, read by the Rust DP algorithms (rpopt::mpc::MpcProblem).

    x(k+1) = A x(k) + B u(k); cost sum_{k<N} x'Qx x + u'Ru u + x(N)'P x(N) (the QP carries a factor 1/2);
    u(k) in {Hu u <= ku}, x(k) in {Hx x <= kx} for k = 0..N-1 (x(0) is the parameter), x(N) in T = X.
    `row_map[i]` is the row of the problem's A/b/F (before RPOPT's constraint processing) that
    corresponds to row i of the DP constraint order: for k = 0..N-1 the rows of u(k) then of x(k),
    then the rows of x(N); -1 for the x(0) rows, which have no counterpart (they are the box A_t).
    """
    return dict(A=numpy.asarray(A).tolist(), B=numpy.asarray(B).tolist(), Q=numpy.asarray(Qx).tolist(),
                R=numpy.asarray(Ru).tolist(), P=numpy.asarray(P).tolist(), N=N,
                u_poly=box_polytope(u_bound), x_poly=box_polytope(x_bound), t_poly=box_polytope(x_bound) if t_poly is None else t_poly,
                row_map=[int(i) for i in row_map])


def lqr_admissible_set(A, B, Ru, P, x_bound, u_bound, max_iter=200):
    """
    Maximal positively invariant set of x+ = (A + B K) x under |x| <= x_bound, |K x| <= u_bound
    (K the LQR gain of terminal cost P), as {H x <= k} without redundant rows (Gilbert and Tan).
    """
    from scipy.optimize import linprog
    nx, nu = B.shape
    K = -numpy.linalg.solve(Ru + B.T @ P @ B, B.T @ P @ A)
    Acl = A + B @ K
    C = numpy.vstack([numpy.eye(nx), -numpy.eye(nx), K, -K])
    c = numpy.concatenate([x_bound, x_bound, numpy.tile(u_bound, 2)])
    Hs, ks = [C], [c]
    M = numpy.eye(nx)
    for _ in range(max_iter):
        M = Acl @ M
        Hn, kn = C @ M, c
        Hc, kc = numpy.vstack(Hs), numpy.concatenate(ks)
        grew = False
        for i in range(Hn.shape[0]):
            res = linprog(-Hn[i], A_ub=Hc, b_ub=kc, bounds=[(None, None)] * nx)
            if res.status == 3 or -res.fun > kn[i] + 1e-9:
                grew = True
                break
        if not grew:
            break
        Hs.append(Hn)
        ks.append(kn)
    Hc, kc = numpy.vstack(Hs), numpy.concatenate(ks)
    # drop redundant and duplicate rows
    keep = []
    for i in range(Hc.shape[0]):
        nrm = numpy.linalg.norm(Hc[i])
        Hc[i], kc[i] = Hc[i] / nrm, kc[i] / nrm
    for i in range(Hc.shape[0]):
        if any(numpy.allclose(Hc[i], Hc[j]) and abs(kc[i] - kc[j]) < 1e-9 for j in keep):
            continue
        mask = [j for j in range(Hc.shape[0]) if j != i]
        res = linprog(-Hc[i], A_ub=Hc[mask], b_ub=kc[mask], bounds=[(None, None)] * nx)
        if res.status == 3 or -res.fun > kc[i] + 1e-9:
            keep.append(i)
    return Hc[keep], kc[keep]


def condensed_mpc(A, B, Qx, Ru, N, x_bound, u_bound, terminal_lqr=True, invariant_terminal=False, model_out=None):
    """
    Condensed (state-eliminated) linear MPC with box constraints on states and inputs.

    x_{k+1} = A x_k + B u_k, theta = x_0, decision U = [u_0, ..., u_{N-1}].
    Cost: sum_{k=1}^{N-1} x_k'Qx x_k + x_N' P x_N + sum_{k=0}^{N-1} u_k'Ru u_k (times 1/2).
    With `invariant_terminal` the box on x_N is replaced by the maximal admissible set T of the
    unconstrained LQR (the terminal set of Mitze and Moennigmann 2020).
    If `model_out` is a dict it is filled with the model data (for closed-loop simulation).
    """
    nx, nu = B.shape
    P = solve_discrete_are(A, B, Qx, Ru) if terminal_lqr else Qx
    if model_out is not None:
        model_out.update(A=A, B=B, Q=Qx, R=Ru, P=P, N=N, x_bound=numpy.asarray(x_bound), u_bound=numpy.asarray(u_bound),
                         nx=nx, nu=nu, u_index=0)

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
    # state bounds for k = 1..N (k = 1..N-1 and the set T at N with an invariant terminal set)
    t_poly = None
    for k in range(N - 1 if invariant_terminal else N):
        for sign in (1.0, -1.0):
            rows_A.append(sign * Gamma[k])
            rows_b.append(x_bound.reshape(-1, 1))
            rows_F.append(-sign * Phi[k])
    if invariant_terminal:
        Ht, kt = lqr_admissible_set(A, B, Ru, P, x_bound, u_bound)
        rows_A.append(Ht @ Gamma[N - 1])
        rows_b.append(kt.reshape(-1, 1))
        rows_F.append(-Ht @ Phi[N - 1])
        t_poly = dict(H=Ht.tolist(), k=kt.tolist())

    A_c = numpy.vstack(rows_A)
    b_c = numpy.vstack(rows_b)
    F_c = numpy.vstack(rows_F)

    A_t = numpy.vstack([numpy.eye(nx), -numpy.eye(nx)])
    b_t = numpy.vstack([x_bound.reshape(-1, 1), x_bound.reshape(-1, 1)])

    # DP constraint order -> row of the condensed problem (input rows: [+, -] blocks of nu*N; state rows
    # of x(k), k = 1..N: [+nx, -nx] per k)
    def u_row(k, sgn, j):
        return sgn * nu * N + k * nu + j

    def x_row(k, sgn, i):  # state x(k), k >= 1
        return 2 * nu * N + (k - 1) * 2 * nx + sgn * nx + i

    row_map = []
    for k in range(N):
        row_map += [u_row(k, sgn, j) for sgn in (0, 1) for j in range(nu)]
        row_map += [-1 if k == 0 else x_row(k, sgn, i) for sgn in (0, 1) for i in range(nx)]
    if invariant_terminal:
        row_map += [2 * nu * N + (N - 1) * 2 * nx + i for i in range(len(kt))]
    else:
        row_map += [x_row(N, sgn, i) for sgn in (0, 1) for i in range(nx)]

    return dict(Q=Hq, c=numpy.zeros((nu * N, 1)), H=Fq, A=A_c, b=b_c, F=F_c, A_t=A_t, b_t=b_t,
                equality_indices=[], mpc=mpc_data(A, B, Qx, Ru, P, N, x_bound, u_bound, row_map, t_poly))


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
    # DP constraint order -> row of A (equalities first, then +I x <= ub, then -I x <= ub); variables are
    # [x(1), ..., x(N), u(0), ..., u(N-1)]
    neq = N * nx

    def var_row(v, sgn):
        return neq + sgn * nv + v

    row_map = []
    for k in range(N):
        row_map += [var_row(N * nx + k * nu + j, sgn) for sgn in (0, 1) for j in range(nu)]
        row_map += [-1 if k == 0 else var_row((k - 1) * nx + i, sgn) for sgn in (0, 1) for i in range(nx)]
    row_map += [var_row((N - 1) * nx + i, sgn) for sgn in (0, 1) for i in range(nx)]
    return dict(Q=numpy.eye(nv), c=numpy.zeros((nv, 1)), H=numpy.zeros((nv, nx)),
                A=numpy.vstack([A_eq, A_ineq]), b=numpy.vstack([b_eq, b_ineq]), F=numpy.vstack([F_eq, F_ineq]),
                A_t=numpy.vstack([numpy.eye(nx), -numpy.eye(nx)]), b_t=numpy.vstack([x_upper, x_upper]),
                equality_indices=list(range(N * nx)),
                mpc=mpc_data(A_ss, B_ss, numpy.eye(nx), numpy.eye(nu), numpy.eye(nx), N, x_upper, u_upper[0:1], row_map))


def double_integrator(N, invariant_terminal=False):
    A = numpy.array([[1.0, 1.0], [0.0, 1.0]])
    B = numpy.array([[0.5], [1.0]])
    return condensed_mpc(A, B, numpy.eye(2), numpy.array([[1.0]]), N,
                         x_bound=numpy.array([4.0, 4.0]), u_bound=numpy.array([1.0]),
                         invariant_terminal=invariant_terminal)


def oscillating_masses(N, n_masses=2, Ts=0.5, model_out=None):
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
                         x_bound=numpy.full(2 * n, 4.0), u_bound=numpy.array([0.5]), model_out=model_out)


def two_input_system(N, model_out=None):
    """A 3-state, 2-input open-loop unstable plant (common textbook-style benchmark)."""
    A = numpy.array([[1.1, 0.2, 0.0], [0.0, 0.9, 0.3], [0.1, 0.0, 0.95]])
    B = numpy.array([[0.0, 0.1], [0.1, 0.0], [0.05, 0.05]])
    return condensed_mpc(A, B, numpy.eye(3), 0.1 * numpy.eye(2), N,
                         x_bound=numpy.array([5.0, 5.0, 5.0]), u_bound=numpy.array([1.0, 1.0]), model_out=model_out)


BENCHMARKS = {
    'di_doc_N3': lambda: double_integrator_noncondensed(3),
    'di_N4': lambda: double_integrator(4),
    'di_N6': lambda: double_integrator(6),
    'di_N8': lambda: double_integrator(8),
    'di_N10': lambda: double_integrator(10),
    'masses2_N4': lambda: oscillating_masses(4),
    'masses2_N6': lambda: oscillating_masses(6),
    'dinv_N4': lambda: double_integrator(4, invariant_terminal=True),
    'dinv_N6': lambda: double_integrator(6, invariant_terminal=True),
    'dinv_N8': lambda: double_integrator(8, invariant_terminal=True),
    'sys3x2_N3': lambda: two_input_system(3),
    'sys3x2_N5': lambda: two_input_system(5),
}


def to_json(prob):
    out = {}
    for k, v in prob.items():
        out[k] = v.tolist() if isinstance(v, numpy.ndarray) else v
    return out


# Larger problems (`python mpc_problems.py <dir> --large`): longer horizons, bigger systems. Each entry also gets a
# sidecar `<name>.model.json` (A, B, Q, R, P, N, bounds) used by ppopt_bench for closed-loop query sequences.
LARGE = {
    'masses2_N8': lambda m: oscillating_masses(8, model_out=m),
    'masses2_N10': lambda m: oscillating_masses(10, model_out=m),
    'masses2_N12': lambda m: oscillating_masses(12, model_out=m),
    'sys3x2_N7': lambda m: two_input_system(7, model_out=m),
    'sys3x2_N9': lambda m: two_input_system(9, model_out=m),
    'masses3_N2': lambda m: oscillating_masses(2, n_masses=3, model_out=m),
    'masses3_N3': lambda m: oscillating_masses(3, n_masses=3, model_out=m),
    'masses3_N4': lambda m: oscillating_masses(4, n_masses=3, model_out=m),
    'masses3_N5': lambda m: oscillating_masses(5, n_masses=3, model_out=m),
    'masses4_N2': lambda m: oscillating_masses(2, n_masses=4, model_out=m),
    'masses3_N6': lambda m: oscillating_masses(6, n_masses=3, model_out=m),
    'masses3_N7': lambda m: oscillating_masses(7, n_masses=3, model_out=m),
    'masses4_N3': lambda m: oscillating_masses(3, n_masses=4, model_out=m),
    'masses4_N4': lambda m: oscillating_masses(4, n_masses=4, model_out=m),
    'masses4_N5': lambda m: oscillating_masses(5, n_masses=4, model_out=m),
    'masses5_N2': lambda m: oscillating_masses(2, n_masses=5, model_out=m),
    'masses5_N3': lambda m: oscillating_masses(3, n_masses=5, model_out=m),
    'masses5_N4': lambda m: oscillating_masses(4, n_masses=5, model_out=m),
}


def model_to_json(model):
    out = {}
    for k, v in model.items():
        out[k] = v.tolist() if isinstance(v, numpy.ndarray) else v
    return out


if __name__ == '__main__':
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    large = '--large' in sys.argv
    out_dir = Path(args[0] if args else ('problems_large' if large else 'problems'))
    out_dir.mkdir(parents=True, exist_ok=True)
    if large:
        for name, builder in LARGE.items():
            model = {}
            prob = builder(model)
            with open(out_dir / f'{name}.json', 'w') as fh:
                json.dump(to_json(prob), fh)
            with open(out_dir / f'{name}.model.json', 'w') as fh:
                json.dump(model_to_json(model), fh)
            print(name, 'vars', prob['A'].shape[1], 'cons', prob['A'].shape[0], 'params', prob['F'].shape[1])
        sys.exit(0)
    for name, builder in BENCHMARKS.items():
        prob = builder()
        with open(out_dir / f'{name}.json', 'w') as fh:
            json.dump(to_json(prob), fh)
        print(name, 'vars', prob['A'].shape[1], 'cons', prob['A'].shape[0], 'params', prob['F'].shape[1])
