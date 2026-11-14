"""
Verifies the POP -> RPOPT conversion (pop_convert.py): for 20 random theta per problem, solve the QP
directly from the raw .mat data (quadprog, independent of the converter) and compare with
`MpProgram::solve_theta` on the converted JSON (examples/solve_thetas.rs).

Usage:
    python pop_verify.py <test_set.mat> <converted_dir> <solve_thetas_binary> <out.json> [n_theta=20]
"""
import json
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy
import quadprog
import scipy.io


def raw_solve(p, theta):
    """Direct QP from raw POP data. min 1/2 x'Qx + (c + Ht theta)'x s.t. A x <= b + F theta, Xmin <= x <= Xmax."""
    Q = numpy.array(p['Q'], float); Q = 0.5 * (Q + Q.T)
    c = numpy.array(p['c'], float).reshape(-1)
    Ht = numpy.array(p['Ht'], float)
    nx = Q.shape[0]
    A = numpy.array(p['A'], float).reshape(-1, nx)
    b = numpy.array(p['b'], float).reshape(-1)
    F = numpy.array(p['F'], float).reshape(A.shape[0], -1)
    lin = c + Ht @ theta
    rhs = b + F @ theta
    C = numpy.vstack([A, numpy.eye(nx), -numpy.eye(nx)])
    d = numpy.concatenate([rhs, numpy.array(p['Xmax'], float).reshape(-1), -numpy.array(p['Xmin'], float).reshape(-1)])
    # quadprog: min 1/2 x'Gx - a'x s.t. C'x >= b
    try:
        sol = quadprog.solve_qp(Q, -lin, -C.T, -d)
    except ValueError:
        return None
    x = sol[0]
    return x, 0.5 * x @ Q @ x + lin @ x


def main():
    mat, conv_dir, binary, out_path = sys.argv[1:5]
    n_theta = int(sys.argv[5]) if len(sys.argv) > 5 else 20
    probs = scipy.io.loadmat(mat)['problem'][0]
    rng = numpy.random.default_rng(0)
    report = {}
    tot_bad = 0
    for i, p in enumerate(probs):
        name = f'pop_{i + 1:03d}'
        nt = numpy.array(p['Ht']).shape[1]
        lo = numpy.array(p['Tmin'], float).reshape(-1)
        hi = numpy.array(p['Tmax'], float).reshape(-1)
        CRA = numpy.array(p['CRA'], float).reshape(-1, lo.size)
        CRb = numpy.array(p['CRb'], float).reshape(-1)
        thetas = []
        while len(thetas) < n_theta:
            t = rng.uniform(lo, hi)
            if numpy.all(CRA @ t <= CRb):
                thetas.append(t)
        with tempfile.TemporaryDirectory() as td:
            tf = Path(td) / 't.json'; of = Path(td) / 'o.json'
            tf.write_text(json.dumps([t.tolist() for t in thetas]))
            subprocess.run([binary, str(Path(conv_dir) / f'{name}.json'), str(tf), str(of)], check=True)
            rust = json.loads(of.read_text())
        bad = 0; n_feas = 0; max_dx = 0.0; max_dobj = 0.0
        for t, r in zip(thetas, rust):
            ref = raw_solve(p, t)
            if (ref is None) != (r is None):
                bad += 1
                continue
            if ref is None:
                continue
            n_feas += 1
            x, obj = ref
            rx = numpy.array(r['x'])
            dx = numpy.abs(rx - x).max() / (1.0 + numpy.abs(x).max())
            dobj = abs(r['obj'] - obj) / (1.0 + abs(obj))
            max_dx = max(max_dx, dx); max_dobj = max(max_dobj, dobj)
            if dobj > 1e-6 or dx > 1e-4:
                bad += 1
        report[name] = dict(n_theta=n_theta, feasible=n_feas, bad=bad, max_rel_dx=max_dx, max_rel_dobj=max_dobj)
        tot_bad += bad
    Path(out_path).write_text(json.dumps(report, indent=1))
    print('problems', len(report), 'total mismatches', tot_bad,
          'feasible samples', sum(r['feasible'] for r in report.values()),
          'max rel dx', max(r['max_rel_dx'] for r in report.values()),
          'max rel dobj', max(r['max_rel_dobj'] for r in report.values()))


if __name__ == '__main__':
    main()
