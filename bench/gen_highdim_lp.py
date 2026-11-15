"""
Writes the 5 to 12 parameter mpLP set to <output_dir> (default problems_lp_highdim): oscillating-mass MPC with
3 to 6 masses (6, 8, 10, 12 states = parameters), inf-norm and 1-norm cost, short horizons.
Random mpLPs (5 to 16 parameters) come from `gen_random --lp`. masses8 with the 1-norm (16 parameters) has 523,189 regions
and needs 18 minutes and 7.6 GB in mplp_lex (masses7 with the 1-norm: 131,012 regions, 3 minutes).

Usage:
    python gen_highdim_lp.py <output_dir> [n_masses:N:norm ...]
"""

import json
import sys
from pathlib import Path

from mplp_problems import oscillating_masses

DEFAULT = [(3, 2, 'inf'), (3, 3, 'inf'), (3, 2, 'one'), (4, 2, 'inf'), (4, 1, 'one'),
           (5, 1, 'inf'), (5, 2, 'inf'), (5, 1, 'one'), (6, 1, 'inf'), (6, 1, 'one'),
           (7, 1, 'inf'), (7, 1, 'one'), (8, 1, 'inf'), (8, 1, 'one')]

if __name__ == '__main__':
    out = Path(sys.argv[1] if len(sys.argv) > 1 else 'problems_lp_highdim')
    out.mkdir(parents=True, exist_ok=True)
    specs = [(int(a), int(b), c) for a, b, c in (s.split(':') for s in sys.argv[2:])] or DEFAULT
    for n, N, norm in specs:
        prob = oscillating_masses(N, norm, n_masses=n)
        name = f'masses{n}{norm}_N{N}'
        (out / f'{name}.json').write_text(json.dumps(prob))
        print(name, 'vars', len(prob['c']), 'cons', len(prob['b']), 'params', len(prob['A_t'][0]))
