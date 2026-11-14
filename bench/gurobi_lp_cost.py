"""
Per-LP cost of the Python + gurobipy path that PPOPT uses for its LP sub-problems (Gurobi is the
default 'lp' solver when gurobipy is installed). This does not run PPOPT; it times the same kind
of call (build a model, add the constraint block, optimize, read primal/dual/slack) on the
active-set feasibility LPs of a benchmark problem, to bound PPOPT's time from below:
PPOPT time >= (number of LPs) x (cost per LP).

Usage:
    python gurobi_lp_cost.py problems/masses2_N4.json [n_lps]
"""

import json
import sys
import time

import gurobipy as gp
import numpy
from gurobipy import GRB


def lp_gurobi(env, c, A, b, eq):
    m = gp.Model(env=env)
    x = m.addMVar(A.shape[1], lb=-GRB.INFINITY, ub=GRB.INFINITY)
    ineq = [i for i in range(A.shape[0]) if i not in eq]
    if ineq:
        m.addMConstr(A[ineq], x, GRB.LESS_EQUAL, b[ineq])
    if eq:
        m.addMConstr(A[eq], x, GRB.EQUAL, b[eq])
    m.setMObjective(None, c, 0.0, sense=GRB.MINIMIZE)
    m.optimize()
    if m.status != GRB.OPTIMAL:
        m.dispose()
        return None
    out = (m.objVal, x.X, numpy.array(m.getAttr('Pi')), numpy.array(m.getAttr('Slack')))
    m.dispose()
    return out


def main():
    d = json.load(open(sys.argv[1]))
    n_lps = int(sys.argv[2]) if len(sys.argv) > 2 else 2000
    A = numpy.array(d['A'], float)
    b = numpy.array(d['b'], float).flatten()
    F = numpy.array(d['F'], float)
    A_t = numpy.array(d['A_t'], float)
    b_t = numpy.array(d['b_t'], float).flatten()
    nx, nt = A.shape[1], F.shape[1]
    big = numpy.block([[A, -F], [numpy.zeros((A_t.shape[0], nx)), A_t]])
    bb = numpy.concatenate([b, b_t])
    c = numpy.zeros(nx + nt)
    env = gp.Env(empty=True)
    env.setParam('OutputFlag', 0)
    env.start()
    rng = numpy.random.default_rng(0)
    sets = [sorted(rng.choice(A.shape[0], size=rng.integers(1, min(nx, 4) + 1), replace=False).tolist()) for _ in range(n_lps)]
    lp_gurobi(env, c, big, bb, sets[0])
    t0 = time.perf_counter()
    feasible = 0
    for s in sets:
        feasible += lp_gurobi(env, c, big, bb, s) is not None
    t1 = time.perf_counter()
    print(json.dumps({'problem': sys.argv[1], 'lps': n_lps, 'us_per_lp': 1e6 * (t1 - t0) / n_lps, 'feasible': feasible,
                      'gurobi': '.'.join(map(str, gp.gurobi.version()))}))


if __name__ == '__main__':
    main()
