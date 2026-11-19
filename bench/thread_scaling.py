"""
Thread scaling of the parallel mpQP algorithms: runs ppopt_bench with --threads T and tabulates
solve time, speedup over T = 1, LP count and region count (checked against the serial results).

Usage (from bench/):
    python thread_scaling.py ../target/release/ppopt_bench results/rust.json results/thread_scaling.md
"""

import json
import subprocess
import sys
from pathlib import Path

import sizes

PROBLEMS = ['di_N8', 'masses2_N4', 'masses2_N6', 'sys3x2_N3', 'sys3x2_N5']
ALGOS = ['combinatorial_parallel', 'graph_parallel', 'combinatorial_graph_parallel', 'geometric_parallel']
THREADS = [1, 2, 4, 8, 12, 16]
SERIAL = {'combinatorial_parallel': 'combinatorial', 'graph_parallel': 'graph',
          'combinatorial_graph_parallel': 'combinatorial_graph', 'geometric_parallel': 'geometric'}


def main():
    exe, serial_json, out_md = sys.argv[1], sys.argv[2], sys.argv[3]
    serial = json.load(open(serial_json))
    tmp = Path('results/threads')
    tmp.mkdir(parents=True, exist_ok=True)
    runs = {}
    for t in THREADS:
        out = tmp / f'threads_{t}.json'
        if out.exists():
            out.unlink()
        subprocess.run([exe, 'problems', str(out), '--only', ','.join(PROBLEMS), '--algos', ','.join(ALGOS),
                        '--no-pl', '--threads', str(t)], check=True, capture_output=True)
        runs[t] = json.load(open(out))
        print('threads', t, 'done', flush=True)
    lines = ['| problem | algorithm | serial (1 thread, serial code) | ' + ' | '.join(f'{t} thr' for t in THREADS) + ' | speedup 16 vs 1 | speedup 16 vs serial | regions ok |',
             '|---|---|---|' + '---|' * len(THREADS) + '---|---|---|']
    for p in PROBLEMS:
        for a in ALGOS:
            ser = serial.get(f'{p}/{SERIAL[a]}')
            cells = [p, a, f"{ser['solve_s'] * 1e3:.1f} ms" if ser else '-']
            times = []
            ok = True
            for t in THREADS:
                r = runs[t].get(f'{p}/{a}')
                times.append(r['solve_s'])
                cells.append(f"{r['solve_s'] * 1e3:.1f}")
                if ser and r['regions'] != ser['regions']:
                    ok = False
                if r.get('sample_gaps', 0) or r.get('sample_overlaps', 0) or r.get('sample_wrong', 0):
                    ok = False
            cells.append(f'{times[0] / times[-1]:.1f}x')
            cells.append(f"{ser['solve_s'] / times[-1]:.1f}x" if ser else '-')
            cells.append('yes' if ok else 'NO')
            lines.append('| ' + ' | '.join(cells) + ' |')
    text = sizes.annotate_md('\n'.join(lines)) + '\n\nTimes in ms. "regions ok": same region count as the serial algorithm and no sampled gaps, overlaps or wrong laws at every thread count.\n'
    print(text)
    Path(out_md).write_text(text)


if __name__ == '__main__':
    main()
