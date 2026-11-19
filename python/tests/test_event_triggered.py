"""EventTriggered: on-demand explicit solutions for mpLP and mpQP, checked against direct solves."""

import json
from pathlib import Path

import numpy as np
import pytest

import rpopt

ROOT = Path(__file__).resolve().parent.parent.parent / "bench"
CASES = [("problems", "di_N4"), ("problems", "masses2_N4"), ("problems_lp", "diinf_N3"), ("problems_lp", "sys3x2inf_N3")]


def load(folder, name):
    d = json.loads((ROOT / folder / f"{name}.json").read_text())
    arr = lambda k: np.array(d[k], dtype=float)
    eq = d.get("equality_indices", [])
    if d.get("Q") is not None:
        return rpopt.MPQP_Program(arr("A"), arr("b"), arr("c"), arr("H"), arr("Q"), arr("A_t"), arr("b_t"), arr("F"), eq)
    return rpopt.MPLP_Program(arr("A"), arr("b"), arr("c"), arr("H"), arr("A_t"), arr("b_t"), arr("F"), eq)


def box(program):
    """Bounding box of {theta : A_t theta <= b_t} for the box-shaped benchmark parameter sets."""
    a_t, b_t = np.asarray(program.A_t), np.asarray(program.b_t).ravel()
    n = a_t.shape[1]
    lo, hi = np.full(n, -np.inf), np.full(n, np.inf)
    for row, b in zip(a_t, b_t):
        nz = np.flatnonzero(np.abs(row) > 1e-12)
        if len(nz) == 1:
            k = nz[0]
            if row[k] > 0:
                hi[k] = min(hi[k], b / row[k])
            else:
                lo[k] = max(lo[k], b / row[k])
    return lo, hi


def walk(program, n, step, seed):
    lo, hi = box(program)
    rng = np.random.default_rng(seed)
    th = lo + (hi - lo) * rng.random(len(lo))
    out = []
    for _ in range(n):
        th = np.clip(th + step * (hi - lo) * (2 * rng.random(len(lo)) - 1), lo, hi)
        out.append(th.copy())
    return np.array(out)


def check(program, theta, objective, status):
    ref = program.solve_theta(theta)
    if ref is None:
        assert status == "infeasible"
    else:
        assert status in ("hit", "built", "solved"), status
        assert abs(objective - ref.obj) <= 1e-6 * (1 + abs(ref.obj))


@pytest.fixture(params=CASES, ids=[c[1] for c in CASES])
def program(request):
    return load(*request.param)


def test_query_matches_solver(program):
    et = rpopt.EventTriggered(program)
    for th in walk(program, 200, 0.05, 1):
        r = et.query(th, stream=0)
        check(program, th, r.objective, r.status)
        assert r.x.shape == (program.num_x(),)
        assert r.duals.shape == (program.num_constraints(),)
    s = et.stats()
    assert s["hits"] > 0 and s["regions"] == len(et) > 0


def test_batch_parallel_and_serial(program):
    thetas = walk(program, 300, 0.08, 2)
    streams = np.arange(300) % 5
    names = rpopt.EventTriggered.STATUS_NAMES
    for parallel in (True, False):
        et = rpopt.EventTriggered(program, parallel=parallel)
        out = et.query_batch(thetas, streams)
        assert out["x"].shape == (300, program.num_x())
        for i, th in enumerate(thetas):
            check(program, th, out["objective"][i], names[out["status"][i]])
        again = et.query_batch(thetas, streams)
        assert np.all(again["hit"] | (again["status"] == names.index("infeasible")))
        np.testing.assert_allclose(again["objective"], out["objective"], rtol=1e-6, atol=1e-6)


def test_cache_limit(program):
    et = rpopt.EventTriggered(program, max_regions=2, eviction="lfu", parallel=False)
    for th in walk(program, 150, 0.3, 3):
        r = et.query(th)
        check(program, th, r.objective, r.status)
        assert len(et) <= 2
    et.max_regions = 1
    assert len(et) == 1


def test_policies():
    p = load("problems", "di_N4")
    thetas = [t for t in walk(p, 60, 0.3, 4) if p.solve_theta(t) is not None]

    et = rpopt.EventTriggered(p, miss="report")
    r = et.query(thetas[0])
    assert r.status == "miss" and np.isnan(r.objective) and len(et) == 0
    rid = et.insert_theta(thetas[0])
    assert et.query(thetas[0]).region == rid

    et = rpopt.EventTriggered(p, miss="solve_only")
    for t in thetas:
        r = et.query(t)
        assert r.status == "solved"
    assert len(et) == 0

    et = rpopt.EventTriggered(p, miss="deferred")
    assert et.query(thetas[0]).status == "miss"
    assert et.num_pending == 1 and et.flush() == 1
    statuses = {et.query(t).status for t in thetas[1:]}
    assert statuses <= {"hit", "approx"}
    et.flush()
    for t in thetas:
        r = et.query(t)
        assert r.status == "hit"
        check(p, t, r.objective, r.status)

    et.miss = "solve_only"
    assert et.miss == "solve_only"
    with pytest.raises(ValueError):
        rpopt.EventTriggered(p, miss="sometimes")


def test_external_active_sets_and_export():
    p = load("problems", "di_N4")
    full = rpopt.solve_mpqp(p)
    et = rpopt.EventTriggered(p, miss="report")
    for cr in full.critical_regions:
        et.insert_active_set(list(cr.active_set))
    assert len(et) == len(full.critical_regions)
    for t in walk(p, 200, 0.2, 5):
        r = et.query(t)
        if p.solve_theta(t) is not None:
            assert r.hit
    sol = et.to_solution()
    assert len(sol) == len(et)
    assert len(et.regions()) == len(et.region_ids()) == len(et)


def test_basis_rules():
    lp, qp = load("problems_lp", "sys3x2inf_N3"), load("problems", "di_N4")
    assert rpopt.EventTriggered(lp).basis == "lex"
    assert rpopt.EventTriggered(qp).basis == "solver"
    with pytest.raises(ValueError):
        rpopt.EventTriggered(qp, basis="lex")
    with pytest.raises(ValueError):
        rpopt.EventTriggered(lp, basis="pareto")  # needs cut_coords and core
    lo, hi = box(lp)
    coords = list(range(lp.num_t()))
    for basis, extra in (("lex", {}), ("solver", {}), ("pareto", {"cut_coords": coords, "core": 0.5 * (lo + hi)})):
        et = rpopt.EventTriggered(lp, basis=basis, **extra)
        assert et.basis == basis
        for th in walk(lp, 150, 0.05, 6):
            r = et.query(th, stream=0)
            check(lp, th, r.objective, r.status)


def test_rejects_non_finite_theta():
    p = load("problems", "di_N4")
    et = rpopt.EventTriggered(p)
    th = np.full(p.num_t(), np.nan)
    with pytest.raises(ValueError):
        et.query(th)
    with pytest.raises(ValueError):
        et.query_batch(np.vstack([np.zeros(p.num_t()), th]))
    with pytest.raises(ValueError):
        et.insert_theta(th)
    with pytest.raises(ValueError):
        et.insert_active_set(list(range(p.num_constraints())))
