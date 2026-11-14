"""
Converts the POP mpQP test set (Oberdieck et al. 2016; test_set.mat of github.com/darnstrom/cdc24-mpqp,
the set used by Arnstrom and Axehill, CDC 2024) to RPOPT JSON and verifies the mapping.

Usage:
    python pop_convert.py convert <test_set.mat> <out_dir>
    python pop_verify.py ...   (see pop_verify.py)

POP convention (per problem, theta in R^nt, x in R^nx):
    min_x   1/2 x'Q x + (c + Ht theta)'x     (+ 1/2 theta'Qt theta + ct'theta + cc, constant in x, all zero here)
    s.t.    A x <= b + F theta
            Xmin <= x <= Xmax                 (all finite in this set; appended as constraint rows)
            CRA theta <= CRb,  Tmin <= theta <= Tmax   (the Tmin/Tmax box coincides with CRA/CRb in all 100 problems)
RPOPT/PPOPT convention: min 1/2 x'Qx + theta'H'x + c'x, A x <= b + F theta, A_t theta <= b_t, so
    H = Ht (Ht is nx x nt, objective term theta'Ht'x), A_t = CRA (+ Tmin/Tmax rows when not already implied), b_t = CRb.
Rows of the stacked constraint matrix [A b F] are scaled to unit norm of the A row (the .mat data has entries
up to 1e7); this does not change the feasible set or the solution.
"""
import json
import sys
from pathlib import Path

import numpy
import scipy.io


def load(path):
    d = scipy.io.loadmat(path)
    return d['problem'][0]


def pop_to_arrays(p):
    """Returns dict of arrays in RPOPT convention for one struct entry."""
    Q = numpy.array(p['Q'], dtype=float)
    Q = 0.5 * (Q + Q.T)
    c = numpy.array(p['c'], dtype=float).reshape(-1, 1)
    Ht = numpy.array(p['Ht'], dtype=float)
    nx = Q.shape[0]
    nt = Ht.shape[1]
    A = numpy.array(p['A'], dtype=float).reshape(-1, nx)
    b = numpy.array(p['b'], dtype=float).reshape(-1, 1)
    F = numpy.array(p['F'], dtype=float).reshape(-1, nt)
    Xmin = numpy.array(p['Xmin'], dtype=float).reshape(-1)
    Xmax = numpy.array(p['Xmax'], dtype=float).reshape(-1)
    rows_A, rows_b, rows_F = [A], [b], [F]
    for k in range(nx):
        if numpy.isfinite(Xmax[k]):
            r = numpy.zeros((1, nx)); r[0, k] = 1.0
            rows_A.append(r); rows_b.append([[Xmax[k]]]); rows_F.append(numpy.zeros((1, nt)))
        if numpy.isfinite(Xmin[k]):
            r = numpy.zeros((1, nx)); r[0, k] = -1.0
            rows_A.append(r); rows_b.append([[-Xmin[k]]]); rows_F.append(numpy.zeros((1, nt)))
    A = numpy.vstack(rows_A); b = numpy.vstack(rows_b); F = numpy.vstack(rows_F)
    nrm = numpy.linalg.norm(A, axis=1)
    nrm[nrm == 0] = 1.0
    A = A / nrm[:, None]; b = b / nrm[:, None]; F = F / nrm[:, None]

    CRA = numpy.array(p['CRA'], dtype=float).reshape(-1, nt)
    CRb = numpy.array(p['CRb'], dtype=float).reshape(-1, 1)
    Tmin = numpy.array(p['Tmin'], dtype=float).reshape(-1)
    Tmax = numpy.array(p['Tmax'], dtype=float).reshape(-1)
    At_rows, bt_rows = [CRA], [CRb]
    # add the Tmin/Tmax box rows unless an identical +-e_k row with the same bound is already in CRA
    for k in range(nt):
        for sgn, bound in ((1.0, Tmax[k]), (-1.0, -Tmin[k])):
            e = numpy.zeros(nt); e[k] = sgn
            dup = any(numpy.allclose(CRA[i], e) and abs(CRb[i, 0] - bound) < 1e-12 for i in range(CRA.shape[0]))
            if not dup:
                At_rows.append(e.reshape(1, -1)); bt_rows.append([[bound]])
    return dict(Q=Q, c=c, H=Ht.copy(), A=A, b=b, F=F, A_t=numpy.vstack(At_rows), b_t=numpy.vstack(bt_rows),
                equality_indices=[])


def convert(mat_path, out_dir):
    probs = load(mat_path)
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    for i, p in enumerate(probs):
        d = pop_to_arrays(p)
        out = {k: (v.tolist() if isinstance(v, numpy.ndarray) else v) for k, v in d.items()}
        with open(out_dir / f'pop_{i + 1:03d}.json', 'w') as fh:
            json.dump(out, fh)
    print('wrote', len(probs), 'problems to', out_dir)


if __name__ == '__main__':
    if len(sys.argv) == 4 and sys.argv[1] == 'convert':
        convert(sys.argv[2], sys.argv[3])
    else:
        print(__doc__)
