"""
Lower bound on PPOPT's solve time, and the resulting speedup, without running PPOPT.

The Rust port counts the deterministic-solver calls PPOPT makes for the same algorithm (ppopt_lps,
ppopt_qps: incremented wherever PPOPT calls solve_lp / solve_qp, whether or not Rust itself needs
that LP) and PPOPT's rank tests (ppopt_ranks, numpy.linalg.matrix_rank). Each is priced at the
measured cost of one such Python call (gurobi_lp_cost.py, quadprog_cost.py, rank_cost.py):

    PPOPT time >= ppopt_lps * c_lp + ppopt_qps * c_qp + ppopt_ranks * c_rank

Everything else PPOPT does (numpy work in region construction, Python loops) is left out, so the
real time is higher.

Usage:
    python lower_bound.py results/rust.json results/ [out.md]
"""

import json
import sys
from pathlib import Path


def load_cost(path, key):
    out = {}
    for line in open(path):
        d = json.loads(line)
        out[Path(d['problem']).stem] = d[key]
    return out


def main():
    rs = json.load(open(sys.argv[1]))
    rd = Path(sys.argv[2])
    c_lp = load_cost(rd / 'gurobi_lp_cost.jsonl', 'us_per_lp')
    c_qp = load_cost(rd / 'quadprog_cost.jsonl', 'us_per_qp')
    c_rk = load_cost(rd / 'rank_cost.jsonl', 'us_per_rank')
    lines = ['| problem | algorithm | regions | PPOPT LPs | QPs | rank tests | Rust time | PPOPT lower bound | speedup lower bound |',
             '|---|---|---|---|---|---|---|---|---|']
    single = []
    best = {}
    for key in sorted(rs):
        prob, _, algo = key.partition('/')
        v = rs[key]
        if 'ppopt_lps' not in v or prob not in c_lp:
            continue
        lb = 1e-6 * (v['ppopt_lps'] * c_lp[prob] + v['ppopt_qps'] * c_qp[prob] + v.get('ppopt_ranks', 0) * c_rk[prob])
        t = v['solve_s']
        sp = lb / t
        if not algo.endswith('parallel'):
            single.append((sp, prob, algo))
            b = best.setdefault(prob, [None, None])
            if b[0] is None or lb < b[0][0]:
                b[0] = (lb, algo)
            if b[1] is None or t < b[1][0]:
                b[1] = (t, algo)
        ts = f'{t * 1e3:.2f} ms' if t < 1 else f'{t:.2f} s'
        lbs = f'{lb * 1e3:.1f} ms' if lb < 1 else (f'{lb:.2f} s' if lb < 3600 else f'{lb / 3600:.2f} h')
        lines.append(f'| {prob} | {algo} | {v["regions"]} | {v["ppopt_lps"]:,} | {v["ppopt_qps"]:,} | {v.get("ppopt_ranks", 0):,} | {ts} | {lbs} | {sp:,.0f}x |')
    single.sort()
    lines.append('')
    lines.append(f'Single-thread, same algorithm: speedup lower bound min {single[0][0]:,.0f}x ({single[0][1]}/{single[0][2]}), '
                 f'median {single[len(single) // 2][0]:,.0f}x, max {single[-1][0]:,.0f}x over {len(single)} runs; '
                 f'{sum(1 for s in single if s[0] >= 200)} of {len(single)} runs at 200x or more.')
    lines.append('')
    lines.append('| problem | fastest PPOPT algorithm (lower bound) | fastest Rust algorithm (1 thread) | speedup lower bound |')
    lines.append('|---|---|---|---|')
    for prob, (bp, br) in sorted(best.items()):
        lines.append(f'| {prob} | {bp[1]} {bp[0] * 1e3:.1f} ms | {br[1]} {br[0] * 1e3:.2f} ms | {bp[0] / br[0]:,.0f}x |')
    text = '\n'.join(lines)
    print(text)
    if len(sys.argv) > 3:
        Path(sys.argv[3]).write_text(text + '\n')


if __name__ == '__main__':
    main()
