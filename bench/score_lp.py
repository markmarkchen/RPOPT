"""mpLP scoreboard from mplp_bench text output: frozen baseline vs merged, median over rounds."""
import glob
import statistics
import sys

out = sys.argv[1]
ALGS = ['geometric', 'graph', 'combinatorial', 'combinatorial_graph', 'mplp_lex']


def parse(pattern):
    runs = {}
    for f in sorted(glob.glob(pattern)):
        for line in open(f):
            t = line.split()
            if len(t) < 7 or t[0] == 'problem' or t[4] not in ALGS:
                continue
            key = (t[0], t[4])
            if t[5] == '-':
                runs.setdefault(key, []).append(('timeout' if t[6].startswith('>') else t[6], None, None, None))
                continue
            runs.setdefault(key, []).append(('ok', float(t[6]), int(t[7]), int(t[8]), int(t[5])))
    return runs


base, new = parse(f'{out}/lp_base_*.txt'), parse(f'{out}/lp_new_*.txt')


def summ(v):
    ok = [x for x in v if x[0] == 'ok']
    if not ok:
        return None, v[0][0], None, None, None
    return statistics.median(x[1] for x in ok), 'ok', max(x[2] for x in ok), max(x[3] for x in ok), ok[0][4]


print('| problem | algorithm | baseline ms (regions, gaps) | merged ms (regions, gaps) | ratio |')
print('|---|---|---:|---:|---:|')
ratios = {}
for key in sorted(set(base) | set(new)):
    bt, bs, bg, be, br = summ(base.get(key, [('missing', None, None, None)]))
    nt, ns, ng, ne, nr = summ(new.get(key, [('missing', None, None, None)]))
    bcell = f'{bt:.3f} ({br}, {bg})' if bt is not None else bs
    ncell = f'{nt:.3f} ({nr}, {ng})' if nt is not None else ns
    if bt is not None and nt is not None:
        r = f'{bt / nt:.2f}x'
        ratios.setdefault(key[1], []).append(bt / nt)
    elif bt is None and nt is not None:
        r = 'baseline did not finish'
    else:
        r = '-'
    print(f'| {key[0]} | {key[1]} | {bcell} | {ncell} | {r} |')
print()
allr = []
for a, rs in sorted(ratios.items()):
    allr += rs
    print(f'{a}: median {statistics.median(rs):.2f}x, min {min(rs):.2f}x, max {max(rs):.2f}x ({len(rs)} timed pairs)')
if allr:
    print(f'ALL timed pairs: median {statistics.median(allr):.2f}x, min {min(allr):.2f}x, below 0.95x: {sum(r < 0.95 for r in allr)} of {len(allr)}')
