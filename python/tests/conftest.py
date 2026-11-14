import json
from pathlib import Path

import numpy as np
import pytest

import rpopt

HERE = Path(__file__).resolve().parent
PROBLEMS = HERE.parent.parent / "bench" / "problems"
NAMES = sorted(p.stem for p in PROBLEMS.glob("*.json"))


def load_program(name):
    d = json.loads((PROBLEMS / f"{name}.json").read_text())
    arr = lambda k: np.array(d[k], dtype=float)
    eq = d.get("equality_indices", [])
    if "Q" in d and d["Q"] is not None:
        return rpopt.MPQP_Program(arr("A"), arr("b"), arr("c"), arr("H"), arr("Q"), arr("A_t"), arr("b_t"), arr("F"), eq)
    return rpopt.MPLP_Program(arr("A"), arr("b"), arr("c"), arr("H"), arr("A_t"), arr("b_t"), arr("F"), eq)


def theta_box(program):
    """Bounding box of the benchmark problem's original (box-shaped) parameter set. `program` is a
    problem name, or a program that carries its name in `_name`."""
    name = program if isinstance(program, str) else program._name
    d = json.loads((PROBLEMS / f"{name}.json").read_text())
    a_t, b_t = np.array(d["A_t"], dtype=float), np.array(d["b_t"], dtype=float).ravel()
    n = a_t.shape[1]
    lo, hi = np.full(n, -np.inf), np.full(n, np.inf)
    for row, b in zip(a_t, b_t):
        nz = np.flatnonzero(row)
        if len(nz) == 1:
            k = nz[0]
            if row[k] > 0:
                hi[k] = min(hi[k], b / row[k])
            else:
                lo[k] = max(lo[k], b / row[k])
    return lo, hi


@pytest.fixture(scope="session")
def reference():
    return {
        alg: json.loads((HERE / "data" / f"ref_{alg}.json").read_text()) for alg in ("combinatorial", "geometric")
    }


_cache = {}


def solved(name, algorithm="combinatorial"):
    key = (name, algorithm)
    if key not in _cache:
        prog = load_program(name)
        _cache[key] = (prog, rpopt.solve_mpqp(prog, algorithm))
    return _cache[key]
