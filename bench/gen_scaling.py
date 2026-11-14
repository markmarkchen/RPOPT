import json
import sys
from pathlib import Path

bench = Path(sys.argv[1])
out = Path(sys.argv[2])
sys.path.insert(0, str(bench))
import mpc_problems as mp  # noqa: E402

out.mkdir(parents=True, exist_ok=True)
jobs = [(f'di_N{n}', lambda n=n: mp.double_integrator(n)) for n in (4, 6, 8, 10, 12, 15, 20, 25, 30)]
jobs += [(f'masses2_N{n}', lambda n=n: mp.oscillating_masses(n)) for n in (2, 3, 4, 5, 6, 7, 8)]
jobs += [(f'sys3x2_N{n}', lambda n=n: mp.two_input_system(n)) for n in (2, 3, 4, 5, 6, 7)]
for name, f in jobs:
    p = f()
    (out / f'{name}.json').write_text(json.dumps(mp.to_json(p)))
    print(name, p['A'].shape, flush=True)
