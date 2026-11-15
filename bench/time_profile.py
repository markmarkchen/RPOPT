#!/usr/bin/env python3
"""Builds bench/results/time_profile.md and time_profile.csv from cached census profiles.

  time_profile.py <cache_dir> <old_md_dir> <out_dir>

<cache_dir> holds one JSON line per (solver, problem) as printed by `lp_census --pair <problem.json> <solver>`
(file name <solver>.<problem>.json, key "profile"). <old_md_dir> holds earlier census reports
(d1.parametric_daqp.<problem>.md, m1.graph.<problem>.md) that are reused for the pairs they cover; their
shares are rounded to 1%. A cache file wins over an old report.
"""
import json, os, re, sys

PROBLEMS = ['sys3x2_N5', 'rand_t8_x5_m20_s0', 'rand_t12_x5_m20_s0', 'pop_017', 'pop_005', 'masses3_N4', 'masses4_N3', 'masses5_N3', 'pop_002']
SOLVERS = ['mplcp', 'graph', 'combinatorial_graph', 'parametric_daqp', 'geometric', 'combinatorial']
PHASES = [('cand', 'neighbour search / candidate tests'), ('rowgen', 'row generation'), ('box', 'bounding-box LPs'), ('perrow', 'per-row redundancy tests'), ('cheb', 'Chebyshev LP'), ('build_other', 'other construction'), ('tab', 'tableau update'), ('rest', 'rest')]


def walk_of(prof):
    # the census counters R1Nanos0..5 time the walks (they are bumped after `pivot_facet`); older cached JSONs call the sum 'rank1'
    return prof.get('walk', prof.get('rank1', 0.0))


def from_md(path, solver):
    t = open(path).read()
    mo = re.search(r'\| \S+ \| ' + solver + r' \| (\d+) \| ([\d.]+) \|', t)
    ms = float(mo.group(2))
    regions = int(mo.group(1))
    total = ms * 1e6
    sec = t.split('### Share of total solve time by purpose')[1].split('###')[0]
    row = [l for l in sec.split('\n') if l.startswith('| ') and solver in l][0].split('|')
    cells = [c.strip() for c in row]

    def pct(c):
        m = re.match(r'(\d+)%', c)
        return float(m.group(1)) / 100 if m else 0.0
    # | problem | alg | LP+QP/solve | redundancy | feasibility | neighbour | chebyshev | bbox | init | other
    red, feas, nbr, cheb, bbox = [pct(cells[i]) for i in (4, 5, 6, 7, 8)]
    sec = t.split('### Region construction')[1].split('###')[0]
    row = [l for l in sec.split('\n') if l.startswith('| ') and solver in l][0].split('|')
    cells = [c.strip() for c in row]
    cons, lpshare = pct(cells[3]), pct(cells[4])
    cand = (feas + nbr) * total
    if solver == 'parametric_daqp':
        m = re.search(r'### High-dimension 3j[^\n]*\n+\|[^\n]*\n\|[^\n]*\n\| \S+ \|([^\n]*)\n', t)
        if m:
            f = [c.strip() for c in m.group(1).split('|')]
            # regions, solve ms, tests/region, licq, empty, nonempty, early, daqp, fallbacks, ms in tests, ...
            cand = float(f[9]) * 1e6
    build_total = cons * total
    build_lp = lpshare * total
    box, perrow, cheb = bbox * total, red * total, cheb * total
    return regions, {'total': total, 'cand': cand, 'case2': 0.0, 'walk': 0.0, 'tab': 0.0, 'build_total': build_total, 'rowgen': max(0.0, build_total - build_lp), 'box': box, 'perrow': perrow, 'cheb': cheb,
            'build_other': max(0.0, build_lp - box - perrow - cheb), 'rest': total - cand - build_total}


def load(cache, old, solver, problem):
    f = os.path.join(cache, '%s.%s.json' % (solver, problem))
    if os.path.exists(f) and os.path.getsize(f) > 0:
        try:
            j = json.loads(open(f).read().strip().splitlines()[-1])
            return j['profile'], 'census (this tree)', j['regions']
        except Exception:
            pass
    if os.path.exists(f + '.timeout'):
        rc = open(f + '.timeout').read().strip()
        if rc == 'rc=137':
            return None, 'killed at the memory limit (up to 8 GB)', None
        return None, 'no result within 1500 s (%s)' % rc, None
    for pre in ('d1', 'm1'):
        g = os.path.join(old, '%s.%s.%s.md' % (pre, solver, problem))
        if os.path.exists(g):
            try:
                rg, prof = from_md(g, solver)
                return prof, 'cached report %s (shares rounded to 1%%)' % os.path.basename(g), rg
            except Exception as e:
                sys.stderr.write('parse %s: %s\n' % (g, e))
    return None, 'missing', None


def main():
    cache, old, out = sys.argv[1:4]
    md = ['# Time profile of the exact mpQP solvers\n']
    csv = ['problem,solver,source,regions,total_ms,' + ','.join(k + '_ms' for k, _ in PHASES) + ',case2_ms,walk_ms']
    summary = []
    for p in PROBLEMS:
        md.append('## %s\n' % p)
        md.append('| solver | regions | solve ms | ' + ' | '.join(n for _, n in PHASES) + ' | source |')
        md.append('|---|---:|---:|' + '---:|' * len(PHASES) + '---|')
        for s in SOLVERS:
            prof, src, regions = load(cache, old, s, p)
            if prof is None:
                md.append('| %s | - | - | ' % s + ' | '.join('-' for _ in PHASES) + ' | %s |' % src)
                continue
            tot = prof['total']
            cells = []
            vals = []
            for k, _ in PHASES:
                v = prof.get(k, 0.0)
                vals.append(v)
                cells.append('-' if v <= 0 and k in ('tab',) else '%.1f (%.0f%%)' % (v / 1e6, 100 * v / tot))
            if s == 'mplcp' and prof.get('case2', 0) > 0:
                cells[0] += ', Case 2 %.1f (walks %.1f)' % (prof['case2'] / 1e6, walk_of(prof) / 1e6)
            md.append('| %s | %s | %.1f | ' % (s, '{:,}'.format(regions) if regions else '-', tot / 1e6) + ' | '.join(cells) + ' | %s |' % src)
            csv.append(','.join([p, s, src.split(' (')[0].replace(',', ';'), str(regions or ''), '%.3f' % (tot / 1e6)] + ['%.3f' % (v / 1e6) for v in vals] + ['%.3f' % (prof.get('case2', 0) / 1e6), '%.3f' % (walk_of(prof) / 1e6)]))
            top = max(range(len(PHASES)), key=lambda i: vals[i])
            summary.append((p, s, tot / 1e6, PHASES[top][1], 100 * vals[top] / tot, 100 * (prof['box'] + prof['perrow'] + prof['cheb']) / tot))
        md.append('')
    md.append('## Summary\n')
    md.append('| problem | solver | solve ms | largest phase | its share | LPs of the build (box + per-row + Chebyshev) |')
    md.append('|---|---|---:|---|---:|---:|')
    for p, s, t, n, sh, lp in summary:
        md.append('| %s | %s | %.1f | %s | %.0f%% | %.0f%% |' % (p, s, t, n, sh, lp))
    open(os.path.join(out, 'time_profile_tables.md'), 'w').write('\n'.join(md) + '\n')
    open(os.path.join(out, 'time_profile.csv'), 'w').write('\n'.join(csv) + '\n')


main()
