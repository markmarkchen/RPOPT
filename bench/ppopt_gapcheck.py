"""Sampled coverage check of PPOPT mpLP solutions: fraction of feasible theta in no region."""
import contextlib
import io
import json
import os
import sys

sys.path.insert(0, os.environ['PPOPT_SRC'])
import numpy  # noqa: E402
from ppopt.mplp_program import MPLP_Program  # noqa: E402
from ppopt.mp_solvers.solve_mpqp import solve_mpqp, mpqp_algorithm  # noqa: E402

path, alg = sys.argv[1], sys.argv[2]
n = int(sys.argv[3]) if len(sys.argv) > 3 else 2000
d = json.load(open(path))
m = {k: (numpy.array(v, dtype=float) if k != 'equality_indices' else v) for k, v in d.items()}
prog = MPLP_Program(m['A'], m['b'], m['c'], m['H'], m['A_t'], m['b_t'], m['F'], equality_indices=m['equality_indices'])
with contextlib.redirect_stdout(io.StringIO()):
    sol = solve_mpqp(prog, getattr(mpqp_algorithm, alg))
nt = m['A_t'].shape[1]
hi = m['b_t'][:nt].flatten()
rng = numpy.random.default_rng(7)
feas = gaps = wrong = 0
for _ in range(n):
    th = (rng.uniform(-hi, hi)).reshape(-1, 1)
    s = prog.solve_theta(th)
    if s is None:
        continue
    feas += 1
    r = sol.get_region(th)
    if r is None:
        gaps += 1
        continue
    x = r.evaluate(th).flatten()
    obj = float(prog.evaluate_objective(x.reshape(-1, 1), th))
    if abs(obj - float(s.obj)) > 1e-6 * (1 + abs(float(s.obj))):
        wrong += 1
print(f"{os.path.basename(path)} {alg}: regions {len(sol.critical_regions)}, feasible samples {feas}, gaps {gaps} ({100.0 * gaps / max(feas, 1):.1f}%), wrong objective {wrong}")
