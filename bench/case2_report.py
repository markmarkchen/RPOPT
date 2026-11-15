#!/usr/bin/env python3
"""Case-2 degeneracy and connectivity census: CSV and summary tables.

  case2_report.py <sweep_out_dir> <manifest> [<manifest2>] <out_dir>

sweep_out_dir holds <problem>.json (the `lp_census --pair <problem.json> mplcp` output with RPOPT_CONN=1 RPOPT_CERT=0, key `conn`),
the manifests list `<problem> <parameters> <path>`. Writes case2_degeneracy.csv and case2_degeneracy_tables.md.
"""
import csv, json, os, statistics, sys, collections


def family(n):
    for p in ('rand', 'masses', 'pop', 'sys'):
        if n.startswith(p):
            return p
    return 'other'


def main():
    sw = sys.argv[1]
    out = sys.argv[-1]
    mans = sys.argv[2:-1]
    probs = []
    for m in mans:
        for l in open(m):
            if l.strip():
                n, nt, f = l.split()
                probs.append((n, int(nt)))
    cols = ['problem', 'family', 'parameters', 'regions', 'facets_boundary', 'facets_case1', 'facets_single_swap', 'facets_several', 'isolated', 'flag_any', 'flag_substantive', 'share_any', 'share_substantive',
            'components_v1', 'start_size_v1', 'outside_v1', 'median_size_v1', 'max_size_v1', 'singletons_v1', 'case2_cross_v1', 'case2_pairs_v1',
            'components_v2', 'start_size_v2', 'outside_v2', 'median_size_v2', 'max_size_v2', 'singletons_v2', 'case2_cross_v2', 'case2_pairs_v2']
    rows, failed = [], []
    for n, nt in probs:
        f = os.path.join(sw, n + '.json')
        if not os.path.exists(f):
            failed.append((n, nt, open(f + '.fail').read().strip() if os.path.exists(f + '.fail') else 'not run'))
            continue
        j = json.loads(open(f).read().strip().splitlines()[-1])
        c = j['conn']
        r = j['regions']
        row = {'problem': n, 'family': family(n), 'parameters': nt, 'regions': r}
        for k in cols[4:11] + cols[13:]:
            row[k] = int(c[k])
        row['flag_any'] = int(c['flag_any'])
        row['share_any'] = c['flag_any'] / max(r, 1)
        row['share_substantive'] = c['flag_substantive'] / max(r, 1)
        rows.append(row)
    with open(os.path.join(out, 'case2_degeneracy.csv'), 'w', newline='') as fh:
        w = csv.DictWriter(fh, fieldnames=cols)
        w.writeheader()
        for r in rows:
            w.writerow({k: ('%.5f' % r[k] if isinstance(r[k], float) else r[k]) for k in cols})
    md = []

    def summary(title, key):
        g = collections.defaultdict(list)
        for r in rows:
            g[key(r)].append(r)
        md.append('\n### %s\n' % title)
        md.append('| group | problems | regions | boundary facets | Case-1 facets | single-swap facets | several-candidate facets | regions with every interior facet Case 2 | of which no Case 1 and no single swap | problems with regions outside the start component (Case 1 only) | regions outside it | components (Case 1 only) | problems still disconnected with single swaps | regions outside it |')
        md.append('|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|')
        for k in sorted(g, key=lambda x: (str(type(x)), x)):
            rs = g[k]
            R = sum(r['regions'] for r in rs)
            s = lambda f: sum(r[f] for r in rs)
            fa = sum(r['flag_any'] for r in rs)
            fs = sum(r['share_substantive'] * r['regions'] for r in rs)
            fct = max(s('facets_boundary') + s('facets_case1') + s('facets_single_swap') + s('facets_several'), 1)
            md.append('| %s | %d | %s | %.1f%% | %.1f%% | %.1f%% | %.1f%% | %.2f%% | %.2f%% | %d | %s | %s | %d | %s |' % (
                k, len(rs), '{:,}'.format(R), 100 * s('facets_boundary') / fct, 100 * s('facets_case1') / fct, 100 * s('facets_single_swap') / fct, 100 * s('facets_several') / fct,
                100 * fa / max(R, 1), 100 * fs / max(R, 1), sum(1 for r in rs if r['outside_v1'] > 0), '{:,}'.format(s('outside_v1')), '{:,}'.format(s('components_v1')),
                sum(1 for r in rs if r['outside_v2'] > 0), '{:,}'.format(s('outside_v2'))))
    summary('By family', lambda r: r['family'])
    summary('By number of parameters', lambda r: r['parameters'])
    md.append('\n### Problems where regions are reachable only across degenerate facets\n')
    md.append('| problem | parameters | regions | Case-1 components | start component | outside | median size | max | singletons | Case-2 crossings joining two components | distinct pairs | with single swaps: components | outside | crossings (several-candidate only) |')
    md.append('|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|')
    for r in sorted(rows, key=lambda r: -r['outside_v1']):
        if r['outside_v1'] > 0:
            md.append('| %s | %d | %d | %d | %d | %d | %d | %d | %d | %d | %d | %d | %d | %d |' % (r['problem'], r['parameters'], r['regions'], r['components_v1'], r['start_size_v1'], r['outside_v1'], r['median_size_v1'], r['max_size_v1'], r['singletons_v1'], r['case2_cross_v1'], r['case2_pairs_v1'], r['components_v2'], r['outside_v2'], r['case2_cross_v2']))
    md.append('\nProblems that did not finish: ' + (', '.join('%s (%s)' % (n, why) for n, nt, why in failed) if failed else 'none') + '.')
    md.append('\nProblems with every region connected by Case 1 alone: %d of %d.' % (sum(1 for r in rows if r['outside_v1'] == 0), len(rows)))
    open(os.path.join(out, 'case2_degeneracy_tables.md'), 'w').write('\n'.join(md) + '\n')


main()
