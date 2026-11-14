"""
Turns the output of `ppopt_bench` on bench/problems_large into a markdown report.

Usage:
    python large_report.py results/large.json results/large.md
"""
import json
import math
import re
import sys
from collections import defaultdict

ALGOS = ['combinatorial', 'geometric', 'graph', 'combinatorial_graph', 'ahmadi', 'mplcp']


def family(name):
    if name.startswith('pop_'):
        return 'POP'
    if name.startswith('rand_'):
        return 'random'
    return 'MPC'


def natural(name):
    return [int(t) if t.isdigit() else t for t in re.split(r'(\d+)', name)]


def fmt_time(rec):
    if rec is None:
        return '-'
    if rec.get('timed_out'):
        return 'T/O'
    s = rec['solve_s']
    return f'{s * 1e3:.1f} ms' if s < 1 else f'{s:.2f} s'


def gmean(xs):
    xs = [x for x in xs if x and x > 0 and not math.isnan(x)]
    return math.exp(sum(math.log(x) for x in xs) / len(xs)) if xs else float('nan')


def main():
    data = json.load(open(sys.argv[1]))
    names = sorted({k.split('/')[0] for k in data}, key=natural)
    out = []
    w = out.append
    w('# Large benchmark (busy machine)\n')
    w('All timings were taken on a shared machine with other jobs running ("busy machine"): treat them as noisy, '
      'ratios within a row as more reliable than absolute values. `T/O` = per-solve cap reached.\n')

    info = {}
    for n in names:
        i = data.get(f'{n}/info', {})
        regs = [data[f'{n}/{a}']['regions'] for a in ALGOS if f'{n}/{a}' in data and not data[f'{n}/{a}'].get('timed_out')]
        info[n] = dict(i, regions=max(regs) if regs else None, regs=sorted(set(regs)))

    def correctness(n):
        bits = []
        diff = [a for a in ALGOS if f'{n}/{a}' in data and not data[f'{n}/{a}'].get('timed_out') and not data[f'{n}/{a}']['same_regions_as_first']]
        if diff:
            bits.append('active sets differ: ' + ','.join(diff))
        bad = []
        for a in ALGOS:
            r = data.get(f'{n}/{a}')
            if r and not r.get('timed_out') and (r['sample_gaps'] or r['sample_overlaps'] or r['sample_wrong']):
                bad.append(f"{a} g{r['sample_gaps']}/o{r['sample_overlaps']}/w{r['sample_wrong']}")
        if bad:
            bits.append('samples: ' + '; '.join(bad))
        v = data.get(f'{n}/verify')
        if v and v['mismatch']:
            bits.append(f"verify {v['mismatch']}/{v['checked']}")
        pl = data.get(f'{n}/point_location')
        if pl:
            m = sum(x['mismatches'] for x in pl['methods'])
            mt = sum(x['mismatches_traj'] for x in pl['methods'])
            if m or mt:
                bits.append(f'PL mismatches uniform {m}, traj {mt}')
        return '; '.join(bits) if bits else 'ok'

    for fam in ['MPC', 'random', 'POP']:
        ns = [n for n in names if family(n) == fam]
        if not ns:
            continue
        w(f'\n## {fam}: problems, solve times, correctness\n')
        w('| problem | theta | x | constraints | regions | ' + ' | '.join(ALGOS) + ' | checks (g=gaps, o=overlaps, w=wrong law, 2000 samples) |')
        w('|' + '---|' * (5 + len(ALGOS) + 1))
        for n in ns:
            i = info[n]
            regs = i['regions'] if i['regions'] is not None else '-'
            if len(i['regs']) > 1:
                regs = '/'.join(map(str, i['regs']))
            w(f"| {n} | {i.get('t', '')} | {i.get('x', '')} | {i.get('constraints', '')} | {regs} | "
              + ' | '.join(fmt_time(data.get(f'{n}/{a}')) for a in ALGOS) + f' | {correctness(n)} |')

    methods = []
    for n in names:
        for m in data.get(f'{n}/point_location', {}).get('methods', []):
            if m['method'] not in methods:
                methods.append(m['method'])
    w('\n## Point location, ns per query (uniform / closed-loop trajectory), busy machine\n')
    w('Trajectory mode: closed-loop simulation x+ = A x + B u with the explicit law (MPC), a random walk in theta '
      '(POP, random). 1000 trajectories of up to 100 steps. Slow methods are timed on a prefix of the queries. `-` = method not built.\n')
    for fam in ['MPC', 'random', 'POP']:
        ns = [n for n in names if family(n) == fam and f'{n}/point_location' in data]
        if not ns:
            continue
        w(f'\n### {fam}\n')
        w('| problem | regions | ' + ' | '.join(methods) + ' |')
        w('|' + '---|' * (2 + len(methods)))
        for n in ns:
            ms = {m['method']: m for m in data[f'{n}/point_location']['methods']}
            cells = []
            for m in methods:
                r = ms.get(m)
                cells.append('-' if r is None else (f"{r['ns_per_query']:.0f} / {r['ns_per_query_traj']:.0f}" if not math.isnan(r['ns_per_query_traj'] if r['ns_per_query_traj'] is not None else float('nan')) else f"{r['ns_per_query']:.0f} / -"))
            w(f"| {n} | {info[n]['regions']} | " + ' | '.join(cells) + ' |')
        if fam == 'POP':
            w('\nGeometric mean over the POP problems (ns per query, uniform / trajectory):\n')
            w('| ' + ' | '.join(methods) + ' |')
            w('|' + '---|' * len(methods))
            cells = []
            for m in methods:
                u = [mm['ns_per_query'] for n in ns for mm in data[f'{n}/point_location']['methods'] if mm['method'] == m]
                t = [mm['ns_per_query_traj'] for n in ns for mm in data[f'{n}/point_location']['methods'] if mm['method'] == m and mm['ns_per_query_traj'] == mm['ns_per_query_traj']]
                cells.append(f'{gmean(u):.0f} / {gmean(t):.0f}')
            w('| ' + ' | '.join(cells) + ' |')

    w('\n## Point location storage (floats) and build time (ms), problems with the most regions\n')
    top = sorted([n for n in names if f'{n}/point_location' in data], key=lambda n: -(info[n]['regions'] or 0))[:12]
    w('| problem | regions | ' + ' | '.join(methods) + ' |')
    w('|' + '---|' * (2 + len(methods)))
    for n in top:
        ms = {m['method']: m for m in data[f'{n}/point_location']['methods']}
        cells = ['-' if ms.get(m) is None else f"{ms[m]['storage_floats']} / {ms[m]['build_s'] * 1e3:.0f}" for m in methods]
        w(f"| {n} | {info[n]['regions']} | " + ' | '.join(cells) + ' |')

    # method failures
    w('\n## Not built / skipped\n')
    for n in names:
        lm = data.get(f'{n}/linear_machine')
        if lm:
            w(f"- {n}: linear machine infeasible ({lm.get('multi_neighbour_facets')} multi-neighbour facets)")
    to = [(n, a) for n in names for a in ALGOS if data.get(f'{n}/{a}', {}).get('timed_out')]
    if to:
        w('\nTimed-out solves: ' + ', '.join(f'{n}/{a}' for n, a in to))
    # summary
    w('\n## Summary by algorithm\n')
    w('| algorithm | solved | timed out | geometric-mean time vs graph (common problems) |')
    w('|---|---|---|---|')
    for a in ALGOS:
        done = [n for n in names if f'{n}/{a}' in data and not data[f'{n}/{a}'].get('timed_out')]
        tos = [n for n in names if data.get(f'{n}/{a}', {}).get('timed_out')]
        common = [n for n in done if f'{n}/graph' in data and not data[f'{n}/graph'].get('timed_out')]
        ratio = gmean([data[f'{n}/{a}']['solve_s'] / data[f'{n}/graph']['solve_s'] for n in common])
        w(f'| {a} | {len(done)} | {len(tos)} | {ratio:.2f} |')
    open(sys.argv[2], 'w').write('\n'.join(out) + '\n')


if __name__ == '__main__':
    main()
