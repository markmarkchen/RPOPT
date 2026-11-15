#!/usr/bin/env python3
# One 1-CPU task of the retry census (Python 3.6 compatible; run with the nodes' /usr/bin/python3).
#   task.py cert  <problem> <alt>   item 1: new (RPOPT_CERT=0), base, base again in shuffled order, one process each
#   task.py base  <problem> <alt>   release solve time of the default build (denominator of item 4)
#   task.py c23   <problem>         census of items 2 and 3 (RPOPT_RETRY=2,3), one JSON line
#   task.py c4    <problem>         census of item 4 (RPOPT_RETRY=4), one JSON line
# Outputs under $RC/out/<mode>/; a task whose output exists is skipped.
import json, os, random, socket, subprocess, sys, time

RC = os.environ.get('RC', os.path.expanduser('~/rpopt/retry-census'))
BIN = os.path.join(RC, 'bin')
PROB = os.path.join(RC, 'prob')


def run(cmd, env_extra=None):
    env = dict(os.environ)
    env.update(env_extra or {})
    p = subprocess.run(cmd, stdout=subprocess.PIPE, universal_newlines=True, env=env, check=True)
    return p.stdout


def bench_ms(binary, path, reps, env_extra):
    env = {'NOGEO': '1'}
    env.update(env_extra)
    out = run([binary, '', str(reps), path], env)
    for line in out.splitlines():
        f = line.split()
        if 'ser' in f:
            return int(f[1]), float(f[f.index('ser') + 1])
    raise RuntimeError('no result: ' + out)


def load():
    try:
        return open('/proc/loadavg').read().split()[:3]
    except Exception:
        return []


def main():
    mode, name = sys.argv[1], sys.argv[2]
    alt = sys.argv[3] if len(sys.argv) > 3 else '0'
    path = os.path.join(PROB, name + '.json')
    od = os.path.join(RC, 'out', mode)
    os.makedirs(od, exist_ok=True)
    out = os.path.join(od, '%s_%s.json' % (name, alt))
    if os.path.exists(out) and os.path.getsize(out) > 0:
        print('skip', out)
        return
    rec = {'problem': name, 'alt': int(alt), 'node': socket.gethostname(), 'load_before': load()}
    if mode in ('cert', 'base'):
        binary = os.path.join(BIN, 'mplcp_bench_main')
        _, pilot = bench_ms(binary, path, 1, {})
        reps = max(3, min(101, int(600.0 / max(pilot, 0.05))))
        if pilot > 3000:
            reps = 1
        rec['pilot_ms'] = pilot
        rec['nt'] = len(json.load(open(path))['A_t'][0])
        rec['reps'] = reps
        if mode == 'base':
            n, ms = bench_ms(binary, path, reps, {})
            rec['regions'] = n
            rec['ms'] = ms
        else:
            variants = [('new', {'RPOPT_CERT': '0'}), ('base', {}), ('base2', {})]
            random.Random('%s/%s' % (name, alt)).shuffle(variants)
            rec['order'] = [v[0] for v in variants]
            rec['ms'] = {}
            rec['regions'] = {}
            for tag, env in variants:
                n, ms = bench_ms(binary, path, reps, env)
                rec['ms'][tag] = ms
                rec['regions'][tag] = n
            rec['ratio_new_over_base'] = rec['ms']['new'] / rec['ms']['base']
            rec['ratio_aa'] = rec['ms']['base2'] / rec['ms']['base']
    else:
        binary = os.path.join(BIN, 'retry_census')
        items = '2,3' if mode == 'c23' else '4'
        t = time.time()
        line = run([binary, path], {'RPOPT_RETRY': items}).strip().splitlines()[-1]
        rec.update(json.loads(line))
        rec['wall_s'] = time.time() - t
    rec['load_after'] = load()
    tmp = out + '.tmp'
    with open(tmp, 'w') as fh:
        json.dump(rec, fh)
    os.rename(tmp, out)


main()
