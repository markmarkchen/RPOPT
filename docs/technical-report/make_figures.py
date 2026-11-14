#!/usr/bin/env python3
"""Figures for the RPOPT technical report. Reads only committed files in bench/results."""
import re, os
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D

HERE = os.path.dirname(os.path.abspath(__file__))
RES = os.path.join(HERE, "..", "..", "bench", "results")
OUT = os.path.join(HERE, "figures")
os.makedirs(OUT, exist_ok=True)

# Okabe-Ito palette
C = dict(blue="#0072B2", orange="#E69F00", green="#009E73", verm="#D55E00",
         sky="#56B4E9", purple="#CC79A7", black="#222222", grey="#888888")
plt.rcParams.update({
    "font.family": "serif", "font.size": 8.5, "axes.titlesize": 9, "axes.labelsize": 8.5,
    "xtick.labelsize": 8, "ytick.labelsize": 8, "legend.fontsize": 8,
    "axes.spines.top": False, "axes.spines.right": False,
    "axes.grid": True, "grid.color": "#dddddd", "grid.linewidth": 0.5, "axes.axisbelow": True,
    "pdf.fonttype": 42,
})


def num(s):
    return float(s.replace(",", ""))


def read(name):
    with open(os.path.join(RES, name)) as f:
        return f.read()


def cells(line):
    return [x.strip() for x in line.strip().strip("|").split("|")]


# ---------------------------------------------------------------- Figure 1
def fig_mpqp_same():
    txt = read("speedup.md")
    sec = txt.split("### Same algorithm, single thread")[1].split("###")[0]
    rows = []
    for line in sec.splitlines():
        p = cells(line)
        if len(p) == 6 and p[0] != "problem" and not p[0].startswith("-"):
            sp = p[4]
            rows.append((p[0], p[1], num(sp.lstrip(">").rstrip("x")), sp.startswith(">")))
    probs = ["di_doc_N3", "di_N4", "di_N6", "di_N8", "di_N10", "masses2_N4", "masses2_N6", "sys3x2_N3", "sys3x2_N5"]
    algs = [("graph", C["blue"], "o"), ("geometric", C["green"], "s"),
            ("combinatorial_graph", C["orange"], "D"), ("combinatorial", C["verm"], "^")]
    off = {"graph": -0.24, "geometric": -0.08, "combinatorial_graph": 0.08, "combinatorial": 0.24}
    fig, ax = plt.subplots(figsize=(6.4, 4.0))
    ax.axvline(200, color=C["grey"], lw=0.9, ls="--")
    n = 0
    for pi, pr in enumerate(probs):
        for (a, col, mk) in algs:
            for (p, al, sp, lb) in rows:
                if p == pr and al == a:
                    y = pi + off[a]
                    if lb:
                        ax.plot(sp, y, marker=mk, ms=5, mfc="white", mec=col, mew=1.2, ls="none")
                        ax.annotate("", xy=(sp * 1.9, y), xytext=(sp * 1.07, y),
                                    arrowprops=dict(arrowstyle="->", color=col, lw=0.9))
                    else:
                        ax.plot(sp, y, marker=mk, ms=5, color=col, ls="none")
                        n += 1
    ax.set_xscale("log")
    ax.set_xlim(150, 8000)
    ax.set_xticks([200, 500, 1000, 2000, 5000])
    ax.set_xticklabels(["200", "500", "1,000", "2,000", "5,000"])
    ax.minorticks_off()
    ax.set_yticks(range(len(probs)))
    ax.set_yticklabels(probs)
    ax.invert_yaxis()
    ax.grid(axis="y", visible=False)
    ax.set_xlabel("speedup over PPOPT, same algorithm, 1 thread (log scale; dashed line: 200x)")
    h = [Line2D([], [], marker=mk, color=col, ls="none", ms=5, label=a.replace("_", " ")) for a, col, mk in algs]
    h.append(Line2D([], [], marker="o", mfc="white", mec=C["black"], color=C["black"], ls="none", ms=5,
                    label="hollow: lower bound, PPOPT unfinished"))
    fig.legend(handles=h, loc="lower center", ncol=3, frameon=False, bbox_to_anchor=(0.5, -0.01))
    fig.tight_layout(rect=(0, 0.1, 1, 1))
    fig.savefig(os.path.join(OUT, "fig_mpqp_same.pdf"))
    plt.close(fig)
    print("fig1 measured points:", n)


# ---------------------------------------------------------------- Figure 2
def fig_mplp():
    rows = []
    for line in read("mplp_speedup.md").splitlines():
        p = cells(line)
        if len(p) == 5 and p[0] != "problem" and not p[0].startswith("-"):
            sp = p[4]
            rows.append((p[0], num(sp.lstrip("> ").rstrip("x")), sp.startswith(">")))
    fig, ax = plt.subplots(figsize=(6.4, 4.2))
    ys = np.arange(len(rows))
    for y, (nm, sp, lb) in zip(ys, rows):
        if lb:
            ax.barh(y, sp, color="white", edgecolor=C["verm"], hatch="////", lw=0.8, height=0.65)
            ax.text(sp * 1.2, y, f"> {sp:,.0f}x", va="center", fontsize=7.5)
        else:
            ax.barh(y, sp, color=C["blue"], height=0.65)
            ax.text(sp * 1.2, y, f"{sp:,.0f}x", va="center", fontsize=7.5)
    ax.set_xscale("log")
    ax.set_xlim(100, 3e6)
    ax.set_yticks(ys)
    ax.set_yticklabels([r[0] for r in rows])
    ax.invert_yaxis()
    ax.grid(axis="y", visible=False)
    ax.set_xlabel("speedup of mplp_lex over PPOPT's fastest gap-free solver (log scale)")
    h = [plt.Rectangle((0, 0), 1, 1, color=C["blue"], label="measured (PPOPT finished, no sampled gap)"),
         plt.Rectangle((0, 0), 1, 1, fc="white", ec=C["verm"], hatch="////",
                       label="lower bound (no gap-free PPOPT answer in 120 s)")]
    fig.legend(handles=h, loc="lower center", ncol=1, frameon=False, bbox_to_anchor=(0.5, -0.01))
    fig.tight_layout(rect=(0, 0.1, 1, 1))
    fig.savefig(os.path.join(OUT, "fig_mplp.pdf"))
    plt.close(fig)


# ---------------------------------------------------------------- Figure 3
def fig_scaling():
    data = {}
    for line in read("scaling.txt").splitlines():
        m = re.match(r"^(di|masses2|sys3x2)_N(\d+)\s+\d+\s+\d+\s+\d+\s+(.+?)\s{2,}([0-9.]+|skipped|failed)\s", line + " ")
        if not m:
            continue
        fam, N, alg, t = m.group(1), int(m.group(2)), m.group(3).strip(), m.group(4)
        if t in ("skipped", "failed"):
            continue
        data.setdefault((fam, alg), []).append((N, float(t)))
    algs = [("mplcp", C["blue"], "o"), ("geometric", C["green"], "s"), ("graph", C["sky"], "v"),
            ("combinatorial_graph", C["orange"], "D"), ("combinatorial", C["verm"], "^"),
            ("parametric_daqp", C["purple"], "P"), ("ahmadi", C["grey"], "X"), ("dp (2023 sym)", C["black"], "*")]
    fams = [("di", "double integrator\n2 parameters, 11 regions"),
            ("masses2", "two masses\n4 parameters, 51 to 2,837 regions"),
            ("sys3x2", "3 states, 2 inputs\n3 parameters, 61 to 2,095 regions")]
    fig, axs = plt.subplots(1, 3, figsize=(6.9, 3.2), sharey=True)
    for ax, (fam, title) in zip(axs, fams):
        for a, col, mk in algs:
            pts = sorted(data.get((fam, a), []))
            if pts:
                ax.plot([p[0] for p in pts], [p[1] for p in pts], marker=mk, ms=3.4, lw=1.0, color=col)
        ax.set_yscale("log")
        ax.set_title(title, fontsize=7.6)
        ax.set_xlabel("horizon N")
    axs[0].set_ylabel("solve time [ms] († busy machine)")
    axs[0].set_xscale("log")
    axs[0].set_xticks([4, 8, 15, 30])
    axs[0].set_xticklabels(["4", "8", "15", "30"])
    axs[0].minorticks_off()
    h = [Line2D([], [], marker=mk, color=col, lw=1.0, ms=4,
                label=a.replace("dp (2023 sym)", "dp (2023, symmetric)").replace("_", " ")) for a, col, mk in algs]
    fig.legend(handles=h, loc="lower center", ncol=4, frameon=False, bbox_to_anchor=(0.5, -0.01))
    fig.tight_layout(rect=(0, 0.14, 1, 1))
    fig.savefig(os.path.join(OUT, "fig_scaling.pdf"))
    plt.close(fig)


# ---------------------------------------------------------------- Figure 4
def fig_scoreboard():
    def parse(fn):
        out = {}
        for line in read(fn).splitlines():
            p = cells(line)
            if len(p) == 5 and p[4].endswith("x") and p[4][0].isdigit():
                out.setdefault(p[1], []).append(float(p[4][:-1]))
        return out
    qp, lp = parse("scoreboard_vs_778cdb9_mpqp.md"), parse("scoreboard_vs_778cdb9_mplp.md")
    print("scoreboard pairs:", sum(len(v) for v in qp.values()), sum(len(v) for v in lp.values()))
    fig, axs = plt.subplots(1, 2, figsize=(6.9, 3.0), gridspec_kw=dict(width_ratios=[1, 1.15]))
    rng = np.random.default_rng(1)
    specs = [
        (axs[0], qp, "mpQP: 60 pairs, median 1.10x",
         ["combinatorial", "combinatorial_graph", "graph", "geometric", "mplcp"], (0.9, 1.95)),
        (axs[1], lp, "mpLP: 54 pairs, median 3.48x",
         ["combinatorial", "geometric", "combinatorial_graph", "mplp_lex", "graph"], (0.8, 2500))]
    for ax, d, title, order, xl in specs:
        for i, a in enumerate(order):
            v = np.array(d[a])
            y = i + rng.uniform(-0.13, 0.13, len(v))
            ax.plot(v, y, "o", ms=3.4, color=C["blue"], alpha=0.75, mec="none")
            ax.plot([np.median(v)] * 2, [i - 0.3, i + 0.3], color=C["verm"], lw=2)
        ax.axvline(1.0, color=C["grey"], lw=0.9, ls="--")
        ax.set_yticks(range(len(order)))
        ax.set_yticklabels([o.replace("_", " ") for o in order])
        ax.invert_yaxis()
        ax.set_title(title)
        ax.grid(axis="y", visible=False)
        ax.set_xscale("log")
        ax.set_xlim(*xl)
        ax.set_xlabel("speedup over 778cdb9 (log scale)")
    axs[0].set_xticks([1.0, 1.25, 1.5, 1.75])
    axs[0].set_xticklabels(["1.0", "1.25", "1.5", "1.75"])
    axs[0].minorticks_off()
    axs[1].set_xticks([1, 10, 100, 1000])
    axs[1].set_xticklabels(["1", "10", "100", "1,000"])
    h = [Line2D([], [], marker="o", color=C["blue"], ls="none", ms=4, label="one problem"),
         Line2D([], [], color=C["verm"], lw=2, label="median of the algorithm")]
    fig.legend(handles=h, loc="lower center", ncol=2, frameon=False, bbox_to_anchor=(0.5, -0.01))
    fig.tight_layout(rect=(0, 0.08, 1, 1))
    fig.savefig(os.path.join(OUT, "fig_scoreboard.pdf"))
    plt.close(fig)


# ---------------------------------------------------------------- Figure 5
def fig_pl():
    txt = read("speedup.md").split("### Point location")[1]
    rows = []

    def ns(s):
        m = re.search(r"([0-9,]+) ns$", s)
        return num(m.group(1)) if m else np.nan
    for line in txt.splitlines():
        p = cells(line)
        if len(p) == 4 and p[0] != "problem" and not p[0].startswith("-"):
            rows.append((p[0], ns(p[1]), ns(p[2]), ns(p[3])))
    fig, ax = plt.subplots(figsize=(6.6, 3.1))
    x = np.arange(len(rows))
    w = 0.27
    labs = [("PPOPT numba PointLocation", C["verm"]), ("RPOPT sequential (PPOPT's algorithm)", C["sky"]),
            ("RPOPT hash table", C["blue"])]
    for k, (lab, col) in enumerate(labs):
        ax.bar(x + (k - 1) * w, [r[1 + k] for r in rows], w, color=col, label=lab)
    ax.set_yscale("log")
    ax.set_ylim(5, 4e5)
    ax.set_xticks(x)
    ax.set_xticklabels([r[0] for r in rows], rotation=35, ha="right")
    ax.grid(axis="x", visible=False)
    ax.set_ylabel("time per query [ns], log scale")
    ax.legend(loc="upper left", frameon=True, framealpha=0.95, edgecolor="#cccccc")
    fig.tight_layout()
    fig.savefig(os.path.join(OUT, "fig_pl.pdf"))
    plt.close(fig)


if __name__ == "__main__":
    fig_mpqp_same()
    fig_mplp()
    fig_scaling()
    fig_scoreboard()
    fig_pl()
    print("done")
