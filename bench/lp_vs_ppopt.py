"""mpLP speedup over PPOPT: RPOPT's fastest gap-free algorithm vs PPOPT's fastest gap-free algorithm.

Usage (from bench/):
    python lp_vs_ppopt.py results/ppopt_lp.json results/ppopt_lp_coverage.txt 'runs/lp_new_*.txt'

ppopt_lp.json comes from bench_ppopt.py on problems_lp, the coverage file from ppopt_gapcheck.py, and the
run files from examples/mplp_bench (medians over the files are used).
"""
import glob
import json
import re
import statistics
import sys

import sizes

ppopt = json.load(open(sys.argv[1]))
gaps = {}
for line in open(sys.argv[2]):
    m = re.match(r'(\S+)\.json (\S+): regions (\d+), feasible samples (\d+), gaps (\d+) \(([\d.]+)%\), wrong objective (\d+)', line)
    if m:
        gaps[(m[1], m[2])] = (int(m[5]), float(m[6]), int(m[7]))

# RPOPT merged: median over rounds per (problem, algorithm), gap-free runs only
runs = {}
for f in glob.glob(sys.argv[3]):
    for line in open(f):
        t = line.split()
        if len(t) >= 9 and t[0] != 'problem' and t[5] != '-':
            if int(t[7]) == 0:
                runs.setdefault((t[0], t[4]), []).append(float(t[6]))
rp = {k: statistics.median(v) for k, v in runs.items()}

probs = ['diinf_N2', 'diinf_N3', 'diinf_N4', 'diinf_N5', 'dione_N2', 'dione_N3', 'dione_N4', 'sys3x2inf_N2', 'sys3x2inf_N3',
         'masses2inf_N2', 'masses2inf_N3', 'sys3x2one_N2', 'masses2one_N2']
SERIAL = ['geometric', 'graph', 'combinatorial', 'combinatorial_graph', 'mplp_lex', 'graph_exp']
out = ['| problem | PPOPT fastest gap-free | PPOPT geometric (gaps) | RPOPT fastest (1 thread) | speedup |',
       '|---|---|---|---|---:|']
for p in probs:
    best = None
    for a in ['geometric', 'graph', 'combinatorial', 'combinatorial_graph']:
        v = ppopt.get(f'{p}/{a}', {})
        g = gaps.get((p, a))
        complete = (g is not None and g[0] == 0) or (g is None and a in ('combinatorial', 'combinatorial_graph'))
        if 'solve_s' in v and complete and (best is None or v['solve_s'] < best[1]):
            best = (a, v['solve_s'])
    geo = ppopt.get(f'{p}/geometric', {})
    g = gaps.get((p, 'geometric'))
    geo_cell = f"{geo['solve_s'] * 1e3:,.0f} ms ({g[1]:.1f}% gaps)" if 'solve_s' in geo and g else ('crashed' if 'error' in geo else '-')
    cands = [(a, rp[(p, a)]) for a in SERIAL if (p, a) in rp]
    ra, rt = min(cands, key=lambda x: x[1])
    if best:
        sp = f'{best[1] * 1e3 / rt:,.0f}x'
        bcell = f'{best[0]} {best[1] * 1e3:,.0f} ms'
    else:
        sp = f'> {120e3 / rt:,.0f}x'
        bcell = 'none within 120 s'
    out.append(f'| {p} | {bcell} | {geo_cell} | {ra} {rt:.2f} ms | {sp} |')
print(sizes.annotate_md('\n'.join(out)))   # adds the size (theta/x/m) column after `problem`
