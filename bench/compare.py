"""
Joins results/ppopt.json (bench_ppopt.py) and results/rust.json (ppopt_bench) into speedup tables.

Usage:
    python compare.py results/ppopt.json results/rust.json [results/speedup.md]

Serial PPOPT algorithms are compared with the same Rust algorithm on one thread; PPOPT's
combinatorial_parallel (pathos processes) with Rust's combinatorial_parallel (rayon, all cores).
A PPOPT run that hit its time limit gives a lower bound (">").
"""

import json
import sys

SERIAL = ['combinatorial', 'graph', 'geometric', 'combinatorial_graph']
PARALLEL = {'combinatorial_parallel': 'combinatorial_parallel'}


def fmt_s(x):
    if x is None:
        return '-'
    if x >= 1:
        return f'{x:,.2f} s'
    if x >= 1e-3:
        return f'{x * 1e3:.1f} ms'
    return f'{x * 1e6:.0f} us'


def ppopt_time(p):
    """(seconds, regions, timed_out) or None."""
    if p is None:
        return None
    if 'solve_s' in p:
        return p['solve_s'], p['regions'], False
    if 'timeout_s' in p:
        return p['timeout_s'], '-', True
    return None


def summary(label, rows):
    done = sorted(sp for _, _, sp, lb in rows if not lb)
    bounds = sorted(sp for _, _, sp, lb in rows if lb)
    if not done and not bounds:
        return []
    out = []
    if done:
        worst = min((r for r in rows if not r[3]), key=lambda r: r[2])
        n200 = sum(1 for s in done if s >= 200)
        out.append(f'{label}: measured on {len(done)} runs, min {done[0]:,.0f}x ({worst[0]}/{worst[1]}), '
                   f'median {done[len(done) // 2]:,.0f}x, max {done[-1]:,.0f}x; {n200} of {len(done)} at 200x or more.')
    if bounds:
        out.append(f'{label}, PPOPT runs stopped at the time limit (lower bounds): '
                   + ', '.join(f'{p}/{a} >{s:,.0f}x' for p, a, s, lb in rows if lb) + '.')
    return out


def main():
    py = json.load(open(sys.argv[1]))
    rs = json.load(open(sys.argv[2]))
    out_path = sys.argv[3] if len(sys.argv) > 3 else None
    problems = sorted({k.split('/')[0] for k in py} | {k.split('/')[0] for k in rs if '/' in k})
    lines = ['### Same algorithm, single thread', '',
             '| problem | algorithm | PPOPT | Rust (1 thread) | speedup | regions (PPOPT / Rust) |',
             '|---|---|---|---|---|---|']
    serial_rows, par_rows, best_rows = [], [], []
    for prob in problems:
        best_py = None
        for algo in SERIAL:
            pt = ppopt_time(py.get(f'{prob}/{algo}'))
            r = rs.get(f'{prob}/{algo}')
            if pt is None or r is None or 'solve_s' not in r:
                continue
            t, reg, lb = pt
            sp = t / r['solve_s']
            serial_rows.append((prob, algo, sp, lb))
            if not lb and (best_py is None or t < best_py[1]):
                best_py = (algo, t)
            lines.append(f"| {prob} | {algo} | {'>' if lb else ''}{fmt_s(t)} | {fmt_s(r['solve_s'])} | "
                         f"{'>' if lb else ''}{sp:,.0f}x | {reg} / {r['regions']} |")
        serial_rust = [(k.split('/')[1], v['solve_s']) for k, v in rs.items()
                       if k.startswith(prob + '/') and 'solve_s' in v and not k.endswith('_parallel')]
        if best_py and serial_rust:
            br = min(serial_rust, key=lambda x: x[1])
            best_rows.append((prob, best_py, br, best_py[1] / br[1]))
    lines += ['', '### Parallel algorithms', '',
              '| problem | algorithm | PPOPT (processes) | Rust (threads) | speedup | regions (PPOPT / Rust) |',
              '|---|---|---|---|---|---|']
    for prob in problems:
        for pa, ra in PARALLEL.items():
            pt = ppopt_time(py.get(f'{prob}/{pa}'))
            r = rs.get(f'{prob}/{ra}')
            if pt is None or r is None or 'solve_s' not in r:
                continue
            t, reg, lb = pt
            sp = t / r['solve_s']
            par_rows.append((prob, pa, sp, lb))
            lines.append(f"| {prob} | {pa} | {'>' if lb else ''}{fmt_s(t)} | {fmt_s(r['solve_s'])} | "
                         f"{'>' if lb else ''}{sp:,.0f}x | {reg} / {r['regions']} |")
    lines += ['', '### Fastest serial algorithm per problem', '',
              '| problem | fastest PPOPT | fastest Rust (1 thread) | speedup |', '|---|---|---|---|']
    for prob, bp, br, sp in best_rows:
        lines.append(f'| {prob} | {bp[0]} {fmt_s(bp[1])} | {br[0]} {fmt_s(br[1])} | {sp:,.0f}x |')
    lines += ['', '### Point location', '',
              '| problem | PPOPT PointLocation (numba) | Rust sequential (PPOPT algorithm) | Rust fastest method |',
              '|---|---|---|---|']
    for prob in problems:
        r = rs.get(f'{prob}/point_location')
        if r is None:
            continue
        seq = [m for m in r['methods'] if m['method'] == 'sequential' or m['method'].startswith('sequential (PPOPT')]
        fastest = min(r['methods'], key=lambda m: m['ns_per_query'])
        pl = next((py[f'{prob}/{a}']['pl_ns_per_query'] for a in ('combinatorial', 'geometric')
                   if 'pl_ns_per_query' in py.get(f'{prob}/{a}', {})), None)
        lines.append((f'| {prob} | {pl:,.0f} ns |' if pl else f'| {prob} | - |')
                     + f" {seq[0]['ns_per_query']:.0f} ns | {fastest['method']} {fastest['ns_per_query']:.0f} ns |")
    lines.append('')
    lines += summary('Same algorithm, single thread', serial_rows)
    lines += summary('Parallel vs parallel', par_rows)
    if best_rows:
        sp = sorted(s for *_, s in best_rows)
        lines.append(f'Fastest serial PPOPT algorithm vs fastest serial Rust algorithm: {sp[0]:,.0f}x to {sp[-1]:,.0f}x.')
    text = '\n'.join(lines)
    print(text)
    if out_path:
        open(out_path, 'w').write(text + '\n')


if __name__ == '__main__':
    main()
