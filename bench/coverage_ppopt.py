"""
PPOPT region-build event logs for the time-to-coverage comparison (see examples/coverage.rs).

Each run solves one problem with one serial PPOPT algorithm in a fresh process. PPOPT itself is not
modified: the worker wraps `gen_cr_from_active_set` in the solver modules and writes one JSON line
per region it returns, {"t": seconds since solve_mpqp started, "as": active set}, flushed at once so
that a run stopped at the time limit keeps its log. A finished run ends with
{"done": true, "solve_s": ..., "final": [active sets of the solution]}; a stopped one with
{"elapsed": <time limit>}.

Usage (from bench/, in an environment with PPOPT's dependencies):
    PPOPT_SRC=/path/to/PPOPT/src python coverage_ppopt.py problems results/coverage_ppopt <timeout_s> [problems] [algorithms] [jobs] [--force]

Logs already in the output directory are reused (a finished run, or one stopped at a limit at least as
long as the requested one); --force reruns them.
"""

import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ALGORITHMS = ['combinatorial', 'graph', 'geometric', 'combinatorial_graph']
MODULES = ['ppopt.mp_solvers.mpqp_combinatorial', 'ppopt.mp_solvers.mpqp_graph', 'ppopt.mp_solvers.mpqp_geometric',
           'ppopt.mp_solvers.mpqp_combi_graph', 'ppopt.mp_solvers.solver_utils']


def worker(problem_path, algorithm, events_path):
    if os.environ.get('PPOPT_SRC'):
        sys.path.insert(0, os.environ['PPOPT_SRC'])
    import importlib
    import io
    import contextlib
    import numpy
    from ppopt.mpqp_program import MPQP_Program
    from ppopt.mp_solvers.solve_mpqp import solve_mpqp, mpqp_algorithm
    import ppopt.utils.mpqp_utils as mu

    fh = open(events_path, 'w')
    start = [None]
    original = mu.gen_cr_from_active_set

    def logged(*args, **kwargs):
        region = original(*args, **kwargs)
        if region is not None and start[0] is not None:
            fh.write(json.dumps({'t': time.perf_counter() - start[0], 'as': sorted(int(i) for i in region.active_set)}) + '\n')
            fh.flush()
        return region

    for name in MODULES:
        mod = importlib.import_module(name)
        if hasattr(mod, 'gen_cr_from_active_set'):
            mod.gen_cr_from_active_set = logged

    with open(problem_path) as f:
        d = json.load(f)
    m = {k: (numpy.array(v, dtype=float) if k != 'equality_indices' else v) for k, v in d.items()
         if not isinstance(v, dict)}  # skip metadata such as the 'mpc' plant description
    prog = MPQP_Program(m['A'], m['b'], m['c'], m['H'], m['Q'], m['A_t'], m['b_t'], m['F'],
                        equality_indices=m['equality_indices'])
    start[0] = time.perf_counter()
    with contextlib.redirect_stdout(io.StringIO()):
        sol = solve_mpqp(prog, getattr(mpqp_algorithm, algorithm))
    solve_s = time.perf_counter() - start[0]
    final = [sorted(int(i) for i in r.active_set) for r in sol.critical_regions]
    fh.write(json.dumps({'done': True, 'solve_s': solve_s, 'final': final}) + '\n')
    fh.close()


def cached(events, timeout):
    """True if the log is from a finished run, or from a run stopped at a limit >= timeout."""
    if not events.exists():
        return False
    lines = events.read_text().splitlines()
    if not lines:
        return False
    try:
        last = json.loads(lines[-1])
    except ValueError:
        return False
    return bool(last.get('done')) or last.get('elapsed', 0) >= timeout


def run_one(args):
    prob, algo, out_dir, timeout, force = args
    events = out_dir / f'{prob.stem}__{algo}.jsonl'
    if not force and cached(events, timeout):
        print(f'{prob.stem}/{algo}: cached', flush=True)
        return
    t0 = time.time()
    try:
        subprocess.run([sys.executable, '-I', __file__, '--worker', str(prob), algo, str(events)],
                       capture_output=True, text=True, timeout=timeout, env=os.environ.copy(), check=True)
        status = 'done'
    except subprocess.TimeoutExpired:
        with open(events, 'a') as fh:
            fh.write(json.dumps({'elapsed': timeout}) + '\n')
        status = f'stopped at {timeout:.0f} s'
    except subprocess.CalledProcessError as e:
        status = 'error: ' + e.stderr.strip().splitlines()[-1] if e.stderr else 'error'
    print(f'{prob.stem}/{algo}: {status} ({time.time() - t0:.1f} s wall)', flush=True)


def main():
    if sys.argv[1] == '--worker':
        worker(sys.argv[2], sys.argv[3], sys.argv[4])
        return
    # PPOPT runs are slow: finished logs (or logs stopped at a limit >= timeout) are reused unless --force
    force = '--force' in sys.argv
    argv = [a for a in sys.argv if a != '--force']
    prob_dir, out_dir = Path(argv[1]), Path(argv[2])
    timeout = float(argv[3])
    only = argv[4].split(',') if len(argv) > 4 and argv[4] else None
    algos = argv[5].split(',') if len(argv) > 5 and argv[5] else ALGORITHMS
    jobs = int(argv[6]) if len(argv) > 6 else 1
    out_dir.mkdir(parents=True, exist_ok=True)
    tasks = [(p, a, out_dir, timeout, force) for p in sorted(prob_dir.glob('*.json')) if not only or p.stem in only for a in algos]
    # long runs first so the pool stays busy
    tasks.sort(key=lambda t: t[1] != 'combinatorial')
    with ThreadPoolExecutor(jobs) as ex:
        list(ex.map(run_one, tasks))


if __name__ == '__main__':
    main()
