"""
Per-call cost of PPOPT's rank test `numpy.linalg.matrix_rank(A[indices]) == len(indices)`
(utils.constraint_utilities.is_full_rank) on random active sets of a benchmark problem.
Does not run PPOPT.

Usage:
    python rank_cost.py problems/masses2_N6.json [n]
"""

import json
import sys
import time

import numpy


def is_full_rank(A, indices):
    if len(indices) == 0:
        return True
    return numpy.linalg.matrix_rank(A[indices]) == len(indices)


def main():
    d = json.load(open(sys.argv[1]))
    n = int(sys.argv[2]) if len(sys.argv) > 2 else 5000
    A = numpy.array(d['A'], float)
    nx = A.shape[1]
    rng = numpy.random.default_rng(0)
    sets = [sorted(rng.choice(A.shape[0], size=rng.integers(1, nx + 1), replace=False).tolist()) for _ in range(n)]
    t0 = time.perf_counter()
    for s in sets:
        is_full_rank(A, s)
    t1 = time.perf_counter()
    print(json.dumps({'problem': sys.argv[1], 'ranks': n, 'us_per_rank': 1e6 * (t1 - t0) / n}))


if __name__ == '__main__':
    main()
