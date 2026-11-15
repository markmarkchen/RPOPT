#!/usr/bin/env python3
"""Writes the archive plan: dir,problem,algorithm,reason,mplcp_table_regions (one line per instance).
mpQPs in the 111-problem table of mplcp_goal_status_2.md: mplcp where mplcp/best <= 1, else the named best other solver.
mpLPs: mplp_lex. Everything else: mplcp, reason "default"."""
import os, re, sys
root = sys.argv[1] if len(sys.argv) > 1 else "."
tab = {}
sect = False
for line in open(f"{root}/bench/results/mplcp_goal_status_2.md"):
    if line.startswith("### All problems"):
        sect = True
        continue
    if sect and line.startswith("###"):
        break
    c = [x.strip() for x in line.strip().strip("|").split("|")]
    if sect and len(c) >= 7 and c[0].isdigit():
        tab[c[1]] = (int(c[2]), c[4], float(c[6]))
# the large-problems table of the same file (one cold solve each): graph 2.98x and 3.04x faster; mplcp region counts there
large = {"masses5_N4": (44065, "graph"), "masses4_N5": (67963, "graph")}
rows = []
for d in ["problems", "problems_highdim", "problems_large", "problems_lp", "problems_lp_highdim"]:
    for f in sorted(os.listdir(f"{root}/bench/{d}")):
        if not f.endswith(".json") or f.endswith(".model.json"):
            continue
        n = f[:-5]
        if d.startswith("problems_lp"):
            rows.append((d, n, "mplp_lex", "README", ""))
        elif n in large:
            rows.append((d, n, large[n][1], "table", large[n][0]))
        elif n in tab:
            reg, other, ratio = tab[n]
            rows.append((d, n, "mplcp" if ratio <= 1 else other, "table", reg))
        else:
            rows.append((d, n, "mplcp", "default", ""))
for r in rows:
    print(",".join(map(str, r)))
