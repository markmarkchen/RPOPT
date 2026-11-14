"""Figure for the mplcp report: single-thread solve time of each exact RPOPT solver on five MPC problems.
Source: bench/results/rust.json (quiet machine, faster median of two full runs)."""
import json
import os

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
d = json.load(open(os.path.join(HERE, "..", "..", "bench", "results", "rust.json")))
probs = ["di_N10", "sys3x2_N3", "masses2_N4", "masses2_N6", "sys3x2_N5"]
algs = [
    ("mplcp", "mplcp (pivoting)", "#1b6ca8", "o"),
    ("geometric", "geometric", "#d1495b", "s"),
    ("graph", "graph", "#66a182", "^"),
    ("combinatorial_graph", "combinatorial_graph", "#edae49", "D"),
    ("parametric_daqp", "parametric_daqp (Arnström-Axehill)", "#8d6a9f", "v"),
    ("mplcp_enum", "mplcp_enum (pLCP enumeration)", "#555555", "x"),
]
fig, ax = plt.subplots(figsize=(6.6, 3.1))
for i, (a, label, color, marker) in enumerate(algs):
    xs, ys = [], []
    for j, p in enumerate(probs):
        v = d.get(f"{p}/{a}", {})
        if "solve_s" in v:
            xs.append(v["solve_s"] * 1e3)
            ys.append(j + (i - 2.5) * 0.11)
    ax.scatter(xs, ys, color=color, marker=marker, s=26, label=label, zorder=3)
ax.set_xscale("log")
ax.set_yticks(range(len(probs)))
regions = [d[f"{p}/geometric"]["regions"] for p in probs]
ax.set_yticklabels([f"{p} ({r})" for p, r in zip(probs, regions)], fontsize=8)
ax.invert_yaxis()
ax.set_xlabel("solve time, one thread [ms] (log scale)", fontsize=8)
ax.tick_params(axis="x", labelsize=8)
ax.grid(axis="x", which="both", color="#dddddd", lw=0.6, zorder=0)
for s in ("top", "right"):
    ax.spines[s].set_visible(False)
ax.legend(fontsize=7, frameon=False, loc="upper left", bbox_to_anchor=(1.0, 1.0))
fig.tight_layout()
fig.savefig(os.path.join(HERE, "fig_times.pdf"))
print("ok")
