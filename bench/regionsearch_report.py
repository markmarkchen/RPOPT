#!/usr/bin/env python3
"""Per-region price of mplcp's neighbour search against the graph family's candidate tests.

  regionsearch_report.py <regions_dir> <cache_dir> <old_md_dir> > table.md

<regions_dir> holds mplcp.<problem>.regions (little-endian f64, 12 per popped region, see
`census_diag::REGREC`) and mplcp.<problem>.json (the census JSON). <cache_dir> holds the
parametric_daqp.<problem>.json census files (mean us per candidate test = cand / pd_tests), <old_md_dir> the older
reports d1.parametric_daqp.<problem>.md (column 'us per test' of table 3j) for the pairs not in the cache.
"""
import json, os, re, struct, sys

PROBLEMS = ['sys3x2_N5', 'rand_t8_x5_m20_s0', 'rand_t12_x5_m20_s0', 'pop_017', 'pop_005', 'masses3_N4', 'masses4_N3', 'masses5_N3', 'pop_002']
N = 12


def daqp_us(cache, old, p):
    f = os.path.join(cache, 'parametric_daqp.%s.json' % p)
    if os.path.exists(f) and os.path.getsize(f) > 0:
        j = json.loads(open(f).read().strip().splitlines()[-1])
        pr = j['profile']
        if pr.get('pd_tests', 0) > 0:
            return pr['cand'] / pr['pd_tests'] / 1e3, 'census'
    g = os.path.join(old, 'd1.parametric_daqp.%s.md' % p)
    if os.path.exists(g):
        t = open(g).read()
        m = re.search(r'### High-dimension 3j[^\n]*\n+\|[^\n]*\n\|[^\n]*\n\| \S+ \|([^\n]*)\n', t)
        if m:
            f = [c.strip() for c in m.group(1).split('|')]
            return float(f[13]), 'cached report'  # us per test
    return None, 'missing'


def main():
    rd, cache, old = sys.argv[1:4]
    out = ['| problem | regions popped | daqp us/test | source | mplcp search ms | priced as DAQP tests ms | regions where mplcp is cheaper | per-region ideal choice ms | saving, share of solve | per-facet-type ideal ms | saving, share of solve |', '|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|']
    ft = ['', '| problem | Case-1 facets: mplcp ms | DAQP ms (same candidates) | cheaper | rank-1: mplcp ms | DAQP ms | cheaper | walks: mplcp ms | DAQP ms | cheaper | search time not in these (none, overhead) ms |', '|---|---:|---:|---|---:|---:|---|---:|---:|---|---:|']
    for p in PROBLEMS:
        f = os.path.join(rd, 'mplcp.%s.regions' % p)
        if not os.path.exists(f):
            out.append('| %s | - | | | | | | | | | |' % p)
            continue
        data = open(f, 'rb').read()
        n = len(data) // (8 * N)
        v = struct.unpack('<%dd' % (n * N), data)
        us, src = daqp_us(cache, old, p)
        j = json.loads(open(os.path.join(rd, 'mplcp.%s.json' % p)).read().strip().splitlines()[-1])
        solve = j['profile']['total']
        if us is None:
            out.append('| %s | %d | - | missing | | | | | | | |' % (p, n))
            continue
        price = us * 1e3  # ns per candidate test
        s_nbr = s_daqp = s_ideal = s_ideal_ft = 0.0
        cheaper = 0
        c1 = [0.0, 0.0]; r1 = [0.0, 0.0]; wk = [0.0, 0.0]; rest = 0.0
        for i in range(n):
            nb, fast, none, case2, rank1, fc, nc, c2c, r1e, fr1, fr2, unseen = v[i * N:(i + 1) * N]
            dq = unseen * price
            s_nbr += nb; s_daqp += dq; s_ideal += min(nb, dq)
            if nb < dq:
                cheaper += 1
            walk = max(case2 - rank1, 0.0)
            c1c, r1c, wkc = fr1, min(r1e, fr2), max(fr2 - min(r1e, fr2), 0.0)
            a, b = fast + none, c1c * price
            c, d = rank1, r1c * price
            e, g = walk, wkc * price
            c1[0] += a; c1[1] += b; r1[0] += c; r1[1] += d; wk[0] += e; wk[1] += g
            other = max(nb - fast - none - case2, 0.0)
            rest += other
            s_ideal_ft += min(a, b) + min(c, d) + min(e, g) + other
        def ms(x):
            return '%.1f' % (x / 1e6)
        out.append('| %s | %d | %.2f | %s | %s | %s | %.0f%% | %s | %.1f%% | %s | %.1f%% |' % (p, n, us, src, ms(s_nbr), ms(s_daqp), 100.0 * cheaper / max(n, 1), ms(s_ideal), 100.0 * (s_nbr - s_ideal) / solve, ms(s_ideal_ft), 100.0 * (s_nbr - s_ideal_ft) / solve))
        ft.append('| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |' % (p, ms(c1[0]), ms(c1[1]), 'mplcp' if c1[0] < c1[1] else 'DAQP', ms(r1[0]), ms(r1[1]), 'mplcp' if r1[0] < r1[1] else 'DAQP', ms(wk[0]), ms(wk[1]), 'mplcp' if wk[0] < wk[1] else 'DAQP', ms(rest)))
    print('\n'.join(out))
    print('')
    print('\n'.join(ft))


main()
