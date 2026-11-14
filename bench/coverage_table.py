"""
Time-to-coverage comparison table from the PPOPT event logs (coverage_ppopt.py) and the Rust
coverage tool (examples/coverage.rs).

Usage (from bench/, after `cargo build --release --example coverage`):
    python coverage_table.py results/coverage_ppopt results/coverage.json results/coverage.md [samples]
"""

import json
import subprocess
import sys
from pathlib import Path

EXE = Path(__file__).resolve().parent.parent / 'target' / 'release' / 'examples' / 'coverage'
LABELS = ['50%', '90%', '99%', '99.9%', '100%']


def fmt_s(x):
    if x >= 1:
        return f'{x:,.2f} s'
    if x >= 1e-3:
        return f'{x * 1e3:.1f} ms'
    return f'{x * 1e6:.0f} us'


def cell(p, r, limit):
    if p is None:
        return f'>{fmt_s(limit)} / {fmt_s(r)} (>{limit / r:,.0f}x)'
    return f'{fmt_s(p)} / {fmt_s(r)} ({p / r:,.0f}x)'


def main():
    ev_dir, out_json, out_md = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    samples = sys.argv[4] if len(sys.argv) > 4 else '100000'
    rows = []
    for ev in sorted(ev_dir.glob('*.jsonl')):
        prob, algo = ev.stem.split('__')
        out = subprocess.run([str(EXE), f'problems/{prob}.json', algo, str(ev), samples], capture_output=True, text=True, check=True)
        rows.append(json.loads(out.stdout))
        print(prob, algo, flush=True)
    out_json.write_text(json.dumps(rows, indent=1))
    lines = ['Cells: PPOPT time / Rust time (speedup). Coverage: share of the feasible parameter set (uniform samples',
             f'of Θ inside the final partition, {samples} samples) lying in regions built so far. "all regions": every region of',
             'the final partition built; "total": the whole solve, including the search for further regions.', '',
             '| problem | algorithm | regions | ' + ' | '.join(LABELS[i] for i in (0, 1, 2, 4)) + ' | all regions | total |',
             '|---|---|---|---|---|---|---|---|---|']
    ratios = {k: [] for k in ['50%', '90%', '99%', '100%', 'all regions', 'total']}
    bounds = []
    for r in rows:
        p, rs = r['ppopt'], r['rust']
        limit = p['observed_s']
        cells = []
        for i in (0, 1, 2, 4):
            cells.append(cell(p['levels'][i], rs['levels'][i], limit))
            if p['levels'][i] is not None:
                ratios[LABELS[i]].append((p['levels'][i] / rs['levels'][i], r['problem'], r['algorithm']))
        cells.append(cell(p['all_regions_built_s'], rs['all_regions_built_s'], limit))
        if p['all_regions_built_s'] is not None:
            ratios['all regions'].append((p['all_regions_built_s'] / rs['all_regions_built_s'], r['problem'], r['algorithm']))
        cells.append(cell(p['solve_s'] if p['finished'] else None, rs['solve_s'], limit))
        if p['finished']:
            ratios['total'].append((p['solve_s'] / rs['solve_s'], r['problem'], r['algorithm']))
        else:
            bounds.append(f"{r['problem']}/{r['algorithm']} (stopped at {fmt_s(limit)}, {p['final_regions_built']} of {r['regions']} regions built)")
        if p.get('final_regions_not_in_rust'):
            cells[-1] += f" [{p['final_regions_not_in_rust']} PPOPT regions not in Rust]"
        lines.append(f"| {r['problem']} | {r['algorithm']} | {r['regions']} | " + ' | '.join(cells) + ' |')
    lines.append('')
    for k, v in ratios.items():
        if v:
            v.sort()
            lines.append(f'{k}: speedup min {v[0][0]:,.0f}x ({v[0][1]}/{v[0][2]}), median {v[len(v) // 2][0]:,.0f}x, '
                         f'max {v[-1][0]:,.0f}x over {len(v)} runs; {sum(1 for x in v if x[0] >= 200)} at 200x or more.')
    if bounds:
        lines.append('PPOPT runs stopped at the time limit: ' + '; '.join(bounds) + '.')
    text = '\n'.join(lines)
    print(text)
    out_md.write_text(text + '\n')


if __name__ == '__main__':
    main()
