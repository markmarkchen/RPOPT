"""
Times PPOPT (Python) on the JSON benchmark problems written by mpc_problems.py.

One problem/algorithm pair runs per subprocess so a slow run can be cut off with a timeout.

Usage:
    PPOPT_SRC=/path/to/PPOPT/src python bench_ppopt.py problems results/ppopt.json [timeout_s] [only,these] [algo,list] [--force]
    (without PPOPT_SRC an installed `ppopt` package is used, e.g. after `pip install ppopt`)

Results already in the output file are reused: a finished run, or a run that hit a time limit at least
as long as the requested one, is not repeated (--force reruns everything selected).
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ALGORITHMS = ['combinatorial', 'combinatorial_graph', 'graph', 'geometric', 'combinatorial_parallel']


def worker(problem_path, algorithm):
    if os.environ.get('PPOPT_SRC'):
        sys.path.insert(0, os.environ['PPOPT_SRC'])  # otherwise use an installed `ppopt`
    import io
    import contextlib
    import numpy
    from ppopt.mpqp_program import MPQP_Program
    from ppopt.mp_solvers.solve_mpqp import solve_mpqp, mpqp_algorithm

    with open(problem_path) as fh:
        d = json.load(fh)
    m = {k: (numpy.array(v, dtype=float) if k != 'equality_indices' else v) for k, v in d.items()}

    t0 = time.perf_counter()
    if 'Q' in m:
        prog = MPQP_Program(m['A'], m['b'], m['c'], m['H'], m['Q'], m['A_t'], m['b_t'], m['F'],
                            equality_indices=m['equality_indices'])
    else:  # mpLP (bench/mplp_problems.py)
        from ppopt.mplp_program import MPLP_Program
        prog = MPLP_Program(m['A'], m['b'], m['c'], m['H'], m['A_t'], m['b_t'], m['F'],
                            equality_indices=m['equality_indices'])
    t1 = time.perf_counter()
    with contextlib.redirect_stdout(io.StringIO()):
        sol = solve_mpqp(prog, getattr(mpqp_algorithm, algorithm))
    t2 = time.perf_counter()

    out = {'build_s': t1 - t0, 'solve_s': t2 - t1, 'regions': len(sol.critical_regions),
           'lp_solver': prog.solver.solvers['lp'], 'qp_solver': prog.solver.solvers['qp']}

    # point location timing (PPOPT's numba-compiled sequential search) on the combinatorial solution,
    # and on the geometric one (for problems where combinatorial does not finish)
    if algorithm in ('combinatorial', 'geometric') and len(sol.critical_regions) > 0:
        from ppopt.upop.point_location import PointLocation
        pl = PointLocation(sol)
        rng = numpy.random.default_rng(0)
        lo = -m['b_t'][m['F'].shape[1]:].flatten()
        hi = m['b_t'][:m['F'].shape[1]].flatten()
        pts = [rng.uniform(lo, hi).reshape(-1, 1) for _ in range(20000)]
        pl.locate(pts[0])  # JIT warm-up
        t3 = time.perf_counter()
        hits = 0
        for p in pts:
            hits += pl.locate(p) >= 0
        t4 = time.perf_counter()
        out['pl_ns_per_query'] = 1e9 * (t4 - t3) / len(pts)
        out['pl_hit_fraction'] = hits / len(pts)
    print('RESULT ' + json.dumps(out))


def cached(entry, timeout):
    """True if a stored result makes a new run pointless: the run finished, or it already hit a
    time limit at least as long as the requested one."""
    if entry is None:
        return False
    if 'solve_s' in entry:
        return True
    return 'timeout_s' in entry and entry['timeout_s'] >= timeout


def main():
    # PPOPT runs are slow, so results already in out_file are kept and not rerun (--force reruns)
    force = '--force' in sys.argv
    argv = [a for a in sys.argv if a != '--force']
    prob_dir = Path(argv[1])
    out_file = Path(argv[2])
    timeout = float(argv[3]) if len(argv) > 3 else 600.0
    only = argv[4].split(',') if len(argv) > 4 and argv[4] else None
    algos = argv[5].split(',') if len(argv) > 5 and argv[5] else ALGORITHMS
    results = {}
    if out_file.exists():
        results = json.loads(out_file.read_text())
    for p in sorted(prob_dir.glob('*.json')):
        if only and p.stem not in only:
            continue
        for algo in algos:
            key = f'{p.stem}/{algo}'
            if not force and cached(results.get(key), timeout):
                print(key, 'cached', flush=True)
                continue
            try:
                proc = subprocess.run([sys.executable, '-I', __file__, '--worker', str(p), algo],
                                      capture_output=True, text=True, timeout=timeout, env=os.environ.copy())
                line = [ln for ln in proc.stdout.splitlines() if ln.startswith('RESULT ')]
                if line:
                    results[key] = json.loads(line[-1][7:])
                else:
                    results[key] = {'error': proc.stderr[-500:]}
            except subprocess.TimeoutExpired:
                results[key] = {'timeout_s': timeout}
            print(key, results[key], flush=True)
            out_file.write_text(json.dumps(results, indent=1))


if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] == '--worker':
        worker(sys.argv[2], sys.argv[3])
    else:
        main()
