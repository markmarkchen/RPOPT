"""PPOPT's documentation example (doc/mpc.rst), written the way a PPOPT user writes it.

The imports are PPOPT's; ``install_as_ppopt`` points them at RPOPT. Set PPOPT_PYTHON and PPOPT_SRC
(the ``src`` directory of a PPOPT checkout) to also compare against the real PPOPT on this problem.
"""

import json
import os
import subprocess
import sys
import textwrap
from pathlib import Path

import numpy
import pytest

import rpopt

DOC_EXAMPLE = '''
import numpy
from ppopt.mpqp_program import MPQP_Program

# define the matrices for the double integrator
A_ss = numpy.array([[1, 1], [0, 1]])
B_ss = numpy.array([[0.5], [1]])

# define the upper and lower bounds of the operating space
x_upper = numpy.array([[4], [4]])
x_lower = -x_upper

u_upper = numpy.array([[1]])
u_lower = -u_upper

# make some auxiliary variables to simply the formulating the problem
I_2 = numpy.eye(2)
Z_2 = 0.0 * I_2
Z_u = 0 * B_ss

# write the equality constraints
A_eq = numpy.block([[I_2, Z_2, Z_2, -B_ss, Z_u, Z_u],
                    [-A_ss, I_2, Z_2, Z_u, -B_ss, Z_u],
                    [Z_2, -A_ss, I_2, Z_u, Z_u, -B_ss]])

b_eq = numpy.zeros((6, 1)).reshape(-1, 1)
F_eq = numpy.block([[A_ss], [Z_2], [Z_2]])

# write the inequality constraints
A_ineq = numpy.block([[numpy.eye(9)], [-numpy.eye(9)]])

b_ineq = numpy.block(
    [[x_upper], [x_upper], [x_upper], [u_upper], [u_upper], [u_upper], [-x_lower], [-x_lower], [-x_lower], [-u_lower],
     [-u_lower], [-u_lower]])

F_ineq = numpy.zeros((18, 2))

# write the constraints on the parametric space
A_t = numpy.block([[I_2], [-I_2]])
b_t = numpy.block([[x_upper], [-x_lower]])

# write the objective terms
Q = numpy.eye(9)
c = numpy.zeros((9, 1))
H = numpy.zeros((9, 2))

# combine the equality and inequality constraints
A = numpy.block([[A_eq], [A_ineq]])
b = numpy.block([[b_eq], [b_ineq]])
F = numpy.block([[F_eq], [F_ineq]])

# use ppopt to build the mpqp problem object
mpmpc = MPQP_Program(A, b, c, H, Q, A_t, b_t, F, equality_indices=list(range(6)))

# process constraints
mpmpc.process_constraints()

from ppopt.mp_solvers.solve_mpqp import solve_mpqp, mpqp_algorithm

sol = solve_mpqp(mpmpc, mpqp_algorithm.combinatorial)

# generate random samples inside of the region of interest
num_samples = 100
perts = (numpy.random.default_rng(0).random((num_samples, 2)) - 0.5)
center = numpy.array([[0.0], [0.0]]).T
samples = perts + center

# get the optimal solution from the explicit map
outputs = list(map(lambda i: sol.evaluate(samples[i].reshape(-1, 1)), range(num_samples)))

# the vertices of the sampled space in x_0
vertices = [numpy.array([[.5], [0.5]]), numpy.array([[-.5], [0.5]]), numpy.array([[-.5], [-0.5]]), numpy.array(
    [[.5], [-0.5]]), numpy.array([[.5], [0.5]])]

# project the vertices into the final state x_3 to build the bounds of where the final state can be
cr = sol.get_region(numpy.array([[0], [0]]))
find_extend = lambda x: cr.A[4:6, :] @ x + cr.b[4:6]
transformed_verts = numpy.block(list(map(find_extend, vertices)))
'''


def run_doc_example():
    ns = {}
    exec(DOC_EXAMPLE, ns)
    return ns


@pytest.fixture(scope="module")
def doc_ns():
    saved = {k: v for k, v in sys.modules.items() if k == "ppopt" or k.startswith("ppopt.")}
    rpopt.install_as_ppopt()
    try:
        yield run_doc_example()
    finally:
        for k in [k for k in sys.modules if k == "ppopt" or k.startswith("ppopt.")]:
            del sys.modules[k]
        sys.modules.update(saved)


def test_doc_example_runs(doc_ns):
    sol, mp = doc_ns["sol"], doc_ns["mpmpc"]
    assert len(sol.critical_regions) == 7
    assert mp.num_x() == 9 and mp.num_t() == 2
    assert all(o is not None and o.shape == (9, 1) for o in doc_ns["outputs"])
    assert doc_ns["transformed_verts"].shape == (2, 5)
    assert sol.verify_solution()
    # each sampled output equals a direct solve
    for s, o in zip(doc_ns["samples"], doc_ns["outputs"]):
        d = mp.solve_theta(s.reshape(-1, 1))
        numpy.testing.assert_allclose(o.ravel(), d.sol.ravel(), atol=1e-6)
    cr = doc_ns["cr"]
    # at the origin nothing is active except the equalities (plus nothing else)
    assert sorted(cr.active_set) == list(range(6))


PPOPT_PY = os.environ.get("PPOPT_PYTHON")
PPOPT_SRC = os.environ.get("PPOPT_SRC")


@pytest.mark.skipif(not (PPOPT_PY and PPOPT_SRC), reason="set PPOPT_PYTHON and PPOPT_SRC to compare with PPOPT")
def test_matches_real_ppopt(doc_ns):
    script = textwrap.dedent(
        """
        import json
        exec(open(%r).read())
        """
    )
    code = DOC_EXAMPLE + textwrap.dedent(
        """
        import json
        out = {
            "n": len(sol.critical_regions),
            "active_sets": sorted(sorted(int(i) for i in r.active_set) for r in sol.critical_regions),
            "outputs": [o.ravel().tolist() for o in outputs],
            "verts": transformed_verts.tolist(),
            "x_at_origin": sol.evaluate(numpy.array([[0.0], [0.0]])).ravel().tolist(),
        }
        print("JSON" + json.dumps(out))
        """
    )
    env = dict(os.environ, PYTHONPATH=PPOPT_SRC)
    res = subprocess.run([PPOPT_PY, "-c", code], capture_output=True, text=True, env=env, timeout=300)
    assert res.returncode == 0, res.stderr[-2000:]
    ref = json.loads(res.stdout.split("JSON")[-1])
    sol = doc_ns["sol"]
    assert ref["n"] == len(sol.critical_regions)
    # PPOPT's active sets index its own (processed) constraint order, which can differ from RPOPT's
    # row removal, so compare the solution itself.
    for s, o, r in zip(doc_ns["samples"], doc_ns["outputs"], ref["outputs"]):
        numpy.testing.assert_allclose(o.ravel(), r, atol=1e-5)
    numpy.testing.assert_allclose(doc_ns["transformed_verts"], numpy.array(ref["verts"]), atol=1e-5)
    numpy.testing.assert_allclose(sol.evaluate(numpy.zeros((2, 1))).ravel(), ref["x_at_origin"], atol=1e-6)
    print("PPOPT active sets:", ref["active_sets"])
    print("RPOPT active sets:", sorted(r.active_set for r in sol.critical_regions))
