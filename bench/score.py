"""Scoreboard: frozen baseline (778cdb9) vs merged, medians over rounds, ratio = base / new."""
import glob
import json
import statistics
import sys

out = sys.argv[1]


def times(pattern):
    runs = {}
    for f in sorted(glob.glob(pattern)):
        d = json.load(open(f))
        for k, v in d.items():
            if isinstance(v, dict) and 'solve_s' in v and not v.get('timed_out'):
                runs.setdefault(k, []).append(v['solve_s'])
    return {k: statistics.median(v) for k, v in runs.items()}


base = times(f'{out}/sb_base_*.json')
new = times(f'{out}/sb_new_*.json')
rows = []
for k in sorted(set(base) & set(new)):
    rows.append((k, base[k], new[k], base[k] / new[k]))
by_alg = {}
for k, b, n, r in rows:
    by_alg.setdefault(k.split('/')[1], []).append(r)
print('| problem | algorithm | baseline | merged | ratio |')
print('|---|---|---:|---:|---:|')
for k, b, n, r in rows:
    p, a = k.split('/')
    print(f'| {p} | {a} | {b * 1e3:.3f} ms | {n * 1e3:.3f} ms | {r:.2f}x |')
print()
for a, rs in sorted(by_alg.items()):
    print(f'{a}: median {statistics.median(rs):.2f}x, min {min(rs):.2f}x, max {max(rs):.2f}x ({len(rs)} problems)')
allr = [r for _, _, _, r in rows]
print(f'ALL: median {statistics.median(allr):.2f}x, min {min(allr):.2f}x, below 0.95x: {sum(r < 0.95 for r in allr)} of {len(allr)}')
