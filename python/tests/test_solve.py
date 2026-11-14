import numpy as np
import pytest

import rpopt
from conftest import NAMES, load_program, solved, theta_box


@pytest.mark.parametrize("algorithm", ["combinatorial", "geometric"])
@pytest.mark.parametrize("name", NAMES)
def test_regions_match_rust(name, algorithm, reference):
    prog, sol = solved(name, algorithm)
    ref = reference[algorithm][name]
    assert len(sol) == len(sol.critical_regions) == ref["n"]
    assert sorted(cr.active_set for cr in sol.critical_regions) == ref["active_sets"]
    cr = sol.critical_regions[0]
    nx, nt = prog.num_x(), prog.num_t()
    assert cr.A.shape == (nx, nt) and cr.b.shape == (nx, 1)
    assert cr.E.shape[1] == nt and cr.f.shape == (cr.E.shape[0], 1)
    assert cr.C.shape[0] == cr.d.shape[0] == len(cr.active_set)


@pytest.mark.parametrize("name", NAMES)
def test_evaluate_matches_solve_theta(name):
    prog, sol = solved(name)
    lo, hi = theta_box(name)
    rng = np.random.default_rng(1)
    thetas = rng.uniform(lo, hi, size=(200, prog.num_t()))
    X, found = sol.evaluate_many(thetas)
    n_in = 0
    for k, th in enumerate(thetas):
        direct = prog.solve_theta(th)
        x = sol.evaluate(th)
        assert (x is None) == (direct is None)
        assert found[k] == (x is not None)
        if x is None:
            continue
        n_in += 1
        np.testing.assert_allclose(x, direct.sol, atol=1e-5, rtol=1e-5)
        np.testing.assert_allclose(X[k], x)
        # column in, column out
        xc = sol.evaluate(th.reshape(-1, 1))
        assert xc.shape == (prog.num_x(), 1)
        np.testing.assert_allclose(xc.ravel(), x)
        assert sol.evaluate_objective(th) == pytest.approx(direct.obj, rel=1e-5, abs=1e-6)
    assert n_in > 0


@pytest.mark.parametrize("name", ["di_doc_N3", "di_N4", "masses2_N4"])
def test_verify_solution(name):
    _, sol = solved(name)
    assert sol.verify_solution()


def test_region_methods():
    prog, sol = solved("di_doc_N3")
    th = np.array([[0.3], [-0.4]])
    cr = sol.get_region(th)
    assert cr is sol.critical_regions[sol.get_region_index(th)]
    assert cr.is_inside(th)
    np.testing.assert_allclose(cr.evaluate(th), cr.A @ th + cr.b)
    np.testing.assert_allclose(cr.lagrange_multipliers(th), cr.C @ th + cr.d)
    E, f = cr.get_constraints()
    assert E.shape == cr.E.shape and f.shape == cr.f.shape
    assert cr.is_full_dimension()
    assert sol.get_region(np.array([100.0, 100.0])) is None
    assert sol.evaluate(np.array([100.0, 100.0])) is None


def test_program_accessors():
    prog = load_program("di_doc_N3")
    assert (prog.num_x(), prog.num_t()) == (9, 2)
    assert prog.num_equality_constraints() == 6
    assert prog.A.shape == (prog.num_constraints(), 9)
    assert prog.Q.shape == (9, 9)
    ball = prog.feasible_space_chebychev_ball()
    assert ball is not None and ball.sol[-1] > 0
    assert prog.solve_theta(np.array([50.0, 50.0])) is None
    prog.process_constraints()  # no-op for PPOPT script compatibility


def test_mplp():
    # min -x1 - 2 x2 + theta x1  s.t.  x1 + x2 <= 4 + theta, x >= 0, x1 <= 3
    A = np.array([[1, 1], [-1, 0], [0, -1], [1, 0]], dtype=float)
    b = np.array([[4], [0], [0], [3]], dtype=float)
    F = np.array([[1], [0], [0], [0]], dtype=float)
    c = np.array([[-1], [-2]], dtype=float)
    H = np.array([[1], [0]], dtype=float)
    A_t = np.array([[1], [-1]], dtype=float)
    b_t = np.array([[2], [2]], dtype=float)
    prog = rpopt.MPLP_Program(A, b, c, H, A_t, b_t, F)
    for alg in ("combinatorial", "graph", "geometric", "mplp_lex"):
        sol = rpopt.solve_mpqp(prog, alg)
        assert len(sol) >= 1
        for th in (-1.5, 0.0, 1.0, 1.9):
            x = sol.evaluate(np.array([th]))
            d = prog.solve_theta(np.array([th]))
            np.testing.assert_allclose(x, d.sol, atol=1e-6)


@pytest.mark.parametrize(
    "alg",
    ["combinatorial", "combinatorial_parallel", "graph", "graph_exp", "graph_parallel", "combinatorial_graph",
     "geometric", "geometric_parallel", "ahmadi", "mplcp", "mplcp_enum"],
)
def test_all_algorithms_di_doc(alg, reference):
    prog = load_program("di_doc_N3")
    sol = rpopt.solve_mpqp(prog, getattr(rpopt.mpqp_algorithm, alg))
    assert len(sol) == 7
    assert sorted(cr.active_set for cr in sol.critical_regions) == reference["combinatorial"]["di_doc_N3"]["active_sets"]


def test_bad_algorithm():
    prog = load_program("di_doc_N3")
    with pytest.raises(NotImplementedError):
        rpopt.solve_mpqp(prog, rpopt.mpqp_algorithm.graph_parallel_exp)
    with pytest.raises(ValueError):
        rpopt.solve_mpqp(prog, "nope")
    with pytest.raises(ValueError):
        prog.solve_theta(np.zeros(5))
