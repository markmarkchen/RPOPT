"""
Point location summary (ns per query) from results/rust.json.

Usage:
    python pl_table.py results/rust.json [extra.json ...]
"""

import json
import sys

import sizes

rs = json.load(open(sys.argv[1]))
for extra in sys.argv[2:]:
    for k, v in json.load(open(extra)).items():
        if k.endswith('/linear_machine'):
            rs[k] = v
if len(sys.argv) > 2:
    json.dump(rs, open(sys.argv[1], 'w'), indent=1)


def pick(ms, name, cond=lambda m: True):
    c = [m for m in ms if m['method'] == name and cond(m)]
    return f"{c[0]['ns_per_query']:.0f}" if c else 'n/a'


out = ['| problem | regions | sequential (PPOPT algorithm) | sequential, early exit | BST | multiway (order 7) | linear machine | descriptor | hash table (16R boxes) | YANN |',
       '|---|---|---|---|---|---|---|---|---|---|']
for p in sorted({k.split('/')[0] for k in rs}):
    pl = rs.get(p + '/point_location')
    if not pl:
        continue
    ms = pl['methods']
    hts = [m for m in ms if m['method'] == 'hash table']
    lm = pick(ms, 'linear machine')
    if lm == 'n/a' and p + '/linear_machine' in rs:
        lm = f"none exists ({rs[p + '/linear_machine']['multi_neighbour_facets']} non-f2f facets)"
    out.append(f"| {p} | {rs[p + '/combinatorial']['regions']} | {pick(ms, 'sequential (PPOPT)')} | {pick(ms, 'sequential (early exit)')} | "
               f"{pick(ms, 'binary search tree')} | {pick(ms, 'multiway tree', lambda m: m['info'].get('levels') == 3)} | {lm} | "
               f"{pick(ms, 'descriptor function')} | {hts[-1]['ns_per_query']:.0f} | {pick(ms, 'YANN')} |")
print(sizes.annotate_md('\n'.join(out)))   # adds the size (theta/x/m) column after `problem`
