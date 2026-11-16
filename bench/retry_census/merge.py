#!/usr/bin/env python3
# Merges the task outputs of the retry census into the tables of mplcp_retry_census.md (stdout). Python 3.6 compatible.
import glob, json, os, statistics as st

RC = os.environ.get('RC', os.path.expanduser('~/rpopt/retry-census'))


def load(mode):
    return [json.load(open(f)) for f in sorted(glob.glob(os.path.join(RC, 'out', mode, '*.json')))]


def pct(a, b):
    return '%.1f%%' % (100.0 * a / b) if b else 'n/a'


def main():
    # item 1
    by = {}
    for r in load('cert'):
        by.setdefault(r['problem'], []).append(r)
    print('## Item 1: RPOPT_CERT=0 against default (ratio new/base, new = certificate off; below 1 means the certificate costs more than it saves)\n')
    print('| problem | parameters | alternations | base ms | new/base median | min | max | A/A median | A/A min | A/A max | regions equal |')
    print('|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|')
    rows = []
    for p, rs in sorted(by.items(), key=lambda kv: (kv[1][0]['nt'], kv[1][0]['pilot_ms'])):
        ra = [r['ratio_new_over_base'] for r in rs]
        aa = [r['ratio_aa'] for r in rs]
        eq = all(len(set(r['regions'].values())) == 1 for r in rs)
        rows.append((p, rs[0]['nt'], st.median(ra), st.median(aa)))
        print('| %s | %d | %d | %.3f | %.3f | %.3f | %.3f | %.3f | %.3f | %.3f | %s |' % (p, rs[0]['nt'], len(rs), st.median([r['ms']['base'] for r in rs]), st.median(ra), min(ra), max(ra), st.median(aa), min(aa), max(aa), 'yes' if eq else 'NO'))
    print('\n| parameters | problems | median new/base | median A/A |')
    print('|---:|---:|---:|---:|')
    for nt in sorted(set(r[1] for r in rows) | {-1}):
        sel = [r for r in rows if nt < 0 or r[1] == nt]
        if sel:
            print('| %s | %d | %.3f | %.3f |' % ('all' if nt < 0 else nt, len(sel), st.median([r[2] for r in sel]), st.median([r[3] for r in sel])))
    # items 2 and 3
    c23 = load('c23')
    print('\n## Item 2: interior point from the hint (accepted regions; margin m of the chord midpoint)\n')
    print('| problem | parameters | regions | stage calls | refused | accepted | hint infeasible | non-unit rows | A m>1e-8 (of entry children) | B m>1e-8 | C available | C infeasible start | C m>1e-8 | best of A,B,C m>1e-6 | m>1e-8 | after 3 recentring steps m>1e-6 | m>1e-8 | m>1e-8 weighted by stage ns | feasible hint: pass | infeasible hint: pass | steps 1 / 2 / 3 | probe ns/region | stage ns/region | refused with m>1e-8 |')
    print('|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|---:|')
    for r in c23:
        a = r['B_ACC']
        print('| %s | %d | %d | %d | %d | %d | %s | %d | %s | %s | %d | %d | %s | %s | %s | %s | %s | %s | %s | %s | %s / %s / %s | %.0f | %.0f | %d |' % (
            r['problem'], r['nt'], r['regions'], r['B_CALLS'], r['B_REJ'], a, pct(r['B_HINT_INFEAS'], a), r['B_NONUNIT'], pct(r['B_A_M8'], r['B_A_ACC']), pct(r['B_B_M8'], a),
            r['B_C_AVAIL'], r['B_C_INFEAS'], pct(r['B_C_M8'], r['B_C_AVAIL']), pct(r['B_X0_M6'], a), pct(r['B_X0_M8'], a), pct(r['B_R3_M6'], a), pct(r['B_R3_M8'], a),
            pct(r['B_R3_NS8'], r['B_ACC_NS']), pct(r['B_FEAS_R3_M8'], r['B_FEAS_ACC']), pct(r['B_INF_R3_M8'], r['B_INF_ACC']),
            pct(r['B_R1_M8'], a), pct(r['B_R2_M8'], a), pct(r['B_R3_M8'], a), r['B_PROBE_NS'] / max(r['B_CALLS'], 1), r['B_ACC_NS'] / max(a, 1), r['B_REJ_M8']))
    print('\n## Item 3: parent dual support (rows that reach the per-row LP)\n')
    print('| problem | parameters | regions | LP rows/region | LP us/row | tries (counterpart dropped by parent LP) | valid (support system) | valid share | missing row | of which toggled constraint | no span | negative | bound | singular | us/try | LP ns saved / LP ns of tries | (saved - try ns) / LP ns of all rows | valid share (tight-row LP) | us/try (tight-row LP) | support size mean | valid but LP keeps |')
    print('|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|')
    for r in c23:
        t = r['R_CP_ROWS']
        print('| %s | %d | %d | %.1f | %.2f | %d | %d | %s | %s | %s | %s | %s | %s | %s | %.2f | %s | %s | %s | %.2f | %.2f | %d |' % (
            r['problem'], r['nt'], r['R_REGIONS'], r['R_LP_ROWS'] / max(r['R_REGIONS'], 1), r['R_LP_NS'] / max(r['R_LP_ROWS'], 1) / 1e3, t, r['R_TRY_VALID'], pct(r['R_TRY_VALID'], t),
            pct(r['R_TRY_INCOMPLETE'], t), pct(r['R_TRY_INC_TOG'], t), pct(r['R_TRY_NOSPAN'], t), pct(r['R_TRY_NEG'], t), pct(r['R_TRY_BOUND'], t), pct(r['R_TRY_SINGULAR'], t),
            r['R_TRY_NS'] / max(t, 1) / 1e3, pct(r['R_SAVED_NS'], r['R_CP_LP_NS']), pct(r['R_SAVED_NS'] - r['R_TRY_NS'], r['R_LP_NS']),
            pct(r['R_T_VALID'], r['R_T_TRY']), r['R_T_NS'] / max(r['R_T_TRY'], 1) / 1e3, r['R_SUP_SUM'] / max(r['R_SUP_N'], 1), r['R_TRY_VALID_KEPT']))
    # item 4
    c4 = load('c4')
    base = {}
    for r in load('base'):
        base.setdefault(r['problem'], []).append(r['ms'])
    print('\n## Item 4: sub-facet tests of rank-1 calls (found = the call emitted at least one candidate; its time is the gross deferral saving)\n')
    print('| problem | base ms | census-build ms | calls | found | found ms | % of base | % of census build | no candidate | ms | % of base | back to walk | ms |')
    print('|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|')
    n4 = big = 0
    for r in c4:
        b = st.median(base[r['problem']]) if r['problem'] in base else float('nan')
        f = r['S_FOUND_NS'] / 1e6
        if b >= 20.0:
            big += 1
            if 100.0 * f / b >= 4.0:
                n4 += 1
        print('| %s | %.1f | %.1f | %d | %d | %.2f | %.2f%% | %.2f%% | %d | %.2f | %.2f%% | %d | %.2f |' % (
            r['problem'], b, r['census_solve_ms'], r['S_CALLS'], r['S_FOUND'], f, 100.0 * f / b, 100.0 * f / r['census_solve_ms'], r['S_NOTFOUND'], r['S_NOTFOUND_NS'] / 1e6, 100.0 * r['S_NOTFOUND_NS'] / 1e6 / b, r['S_ABORT'], r['S_ABORT_NS'] / 1e6))
    print('\nProblems of 20 ms or more with found-call time of 4%% or more of the base solve: %d of %d (rule: 10 or more)' % (n4, big))


main()
