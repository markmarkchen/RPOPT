"""Exact (rational) check of the regions of a basis list: is each region full dimensional?
usage: exact_regions.py bases.txt   (first line PROGRAM <json>, then one basis per line)"""
import json, sys
from fractions import Fraction as Fr
from math import sqrt

lines = open(sys.argv[1]).read().strip().split('\n')
prog = json.loads(lines[0][len('PROGRAM '):])
bases = [tuple(map(int, l.split())) for l in lines[1:]]
A = [[Fr(v) for v in r] for r in prog['A']]
b = [Fr(v) for v in prog['b']]
F = [[Fr(v) for v in r] for r in prog['F']]
c = [Fr(v) for v in prog['c']]
At = [[Fr(v) for v in r] for r in prog['A_t']]
bt = [Fr(v) for v in prog['b_t']]
m, nx, nt = len(A), len(A[0]), len(F[0])


def inv(M):
    n = len(M)
    a = [row[:] + [Fr(int(i == j)) for j in range(n)] for i, row in enumerate(M)]
    for i in range(n):
        p = next(r for r in range(i, n) if a[r][i] != 0)
        a[i], a[p] = a[p], a[i]
        d = a[i][i]
        a[i] = [v / d for v in a[i]]
        for r in range(n):
            if r != i and a[r][i] != 0:
                f = a[r][i]
                a[r] = [x - f * y for x, y in zip(a[r], a[i])]
    return [row[n:] for row in a]


def simplex_max_t(E, f, w, M):
    """max tau s.t. E th + w tau <= f + M w, th free, tau >= 0 (tau = t + M); returns t*."""
    rows = len(E)
    nv = 2 * nt + 1  # th+, th-, tau
    # tableau: rows constraints with slack, rhs
    T = []
    for i in range(rows):
        r = [E[i][k] for k in range(nt)] + [-E[i][k] for k in range(nt)] + [w[i]] + [Fr(int(i == j)) for j in range(rows)] + [f[i] + M * w[i]]
        T.append(r)
    obj = [Fr(0)] * (nv + rows + 1)
    obj[2 * nt] = Fr(-1)  # maximize tau: row of -c
    basis = [nv + i for i in range(rows)]
    it = 0
    while True:
        it += 1
        # Bland: lowest index with negative reduced cost
        ent = next((j for j in range(nv + rows) if obj[j] < 0), None)
        if ent is None:
            break
        best = None
        for i in range(rows):
            if T[i][ent] > 0:
                ratio = T[i][-1] / T[i][ent]
                if best is None or ratio < best[0] or (ratio == best[0] and basis[i] < basis[best[1]]):
                    best = (ratio, i)
        if best is None:
            return None  # unbounded
        i = best[1]
        d = T[i][ent]
        T[i] = [v / d for v in T[i]]
        for r in range(rows):
            if r != i and T[r][ent] != 0:
                fct = T[r][ent]
                T[r] = [x - fct * y for x, y in zip(T[r], T[i])]
        fct = obj[ent]
        obj = [x - fct * y for x, y in zip(obj, T[i])]
        basis[i] = ent
    return obj[-1] - M


for bi, B in enumerate(bases):
    Bi = inv([A[i] for i in B])
    # x = Bi (b_B + F_B th) = xb + Xt th
    xb = [sum(Bi[k][p] * b[B[p]] for p in range(nx)) for k in range(nx)]
    Xt = [[sum(Bi[k][p] * F[B[p]][t] for p in range(nx)) for t in range(nt)] for k in range(nx)]
    # lambda = -A_B^{-T} c
    lam = [-sum(Bi[k][p] * c[k] for k in range(nx)) for p in range(nx)]
    E, f = [], []
    for j in range(m):
        if j in B:
            continue
        e = [sum(A[j][k] * Xt[k][t] for k in range(nx)) - F[j][t] for t in range(nt)]
        rhs = b[j] - sum(A[j][k] * xb[k] for k in range(nx))
        E.append(e)
        f.append(rhs)
    for i in range(len(At)):
        E.append(At[i])
        f.append(bt[i])
    w = [Fr(sqrt(float(sum(v * v for v in e)))) for e in E]
    # rows with e = 0 and f < 0 make the region empty
    empty = any(all(v == 0 for v in e) and fv < 0 for e, fv in zip(E, f))
    tstar = None if empty else simplex_max_t(E, f, w, Fr(10**12))
    print(bi, B, 'lam_min', float(min(lam)), 'lam_zero', sum(1 for l in lam if l == 0), 'EMPTY' if empty else 'tstar %.6e (exact sign %s)' % (float(tstar), '>0' if tstar > 0 else ('=0' if tstar == 0 else '<0')), flush=True)
