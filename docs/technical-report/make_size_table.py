#!/usr/bin/env python3
"""LaTeX tables of the benchmark problem sizes, from bench/results/problem_sizes.csv.

The CSV is written by `cargo run --profile gate --example problem_sizes` (examples/problem_sizes.rs).
Writes tables/tab_sizes.tex (one row per suite, for the main text) and tables/tab_sizes_full.tex (one row per
problem, the complete appendix longtable).

Separate from make_tables.py on purpose: it touches only these two files. Stdlib only.
"""
import csv
import os

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, "..", "..", "bench", "results", "problem_sizes.csv")
OUT = os.path.join(HERE, "tables")
SUITES = ["problems", "problems_highdim", "problems_large", "problems_lp", "problems_lp_highdim"]
TITLES = {
    "problems": "mpQP, explicit MPC",
    "problems_highdim": "mpQP, 12 to 16 parameters",
    "problems_large": "mpQP, large suite (MPC, POP, random)",
    "problems_lp": "mpLP, MPC",
    "problems_lp_highdim": "mpLP, 5 to 16 parameters",
}


def esc(s):
    for a, b in (("\\", r"\textbackslash{}"), ("_", r"\_"), ("%", r"\%"), ("&", r"\&"), ("#", r"\#")):
        s = s.replace(a, b)
    return s


def tt(s):
    return r"\code{" + esc(s) + "}"


def n(x):
    return f"{int(x):,}"


def rng(vals):
    lo, hi = min(vals), max(vals)
    return n(lo) if lo == hi else f"{n(lo)} to {n(hi)}"


def raw_to(raw, proc):
    """'a' when preprocessing kept every row, else 'a $\\to$ b' (rows in the JSON file, rows the solvers see)."""
    return n(raw) if raw == proc else f"{n(raw)} $\\to$ {n(proc)}"


rows = list(csv.DictReader(open(SRC)))
by = {s: [r for r in rows if r["suite"] == s] for s in SUITES}
missing = [s for s in SUITES if not by[s]]
assert not missing, f"no rows for {missing} in {SRC}"
os.makedirs(OUT, exist_ok=True)

# ---- summary: one row per suite
t = [r"\begin{tabular}{@{}L{4.6cm} l r r r r r@{}}", r"\toprule",
     r"suite & type & problems & $n_\theta$ & $n_x$ & constraints & regions \\", r"\midrule"]
for s in SUITES:
    rs = by[s]
    regs = [int(r["regions"]) for r in rs if r["regions"]]
    kinds = "/".join(sorted({r["type"] for r in rs}))
    t.append(" & ".join([tt(s), kinds, n(len(rs)), rng([int(r["theta"]) for r in rs]), rng([int(r["x"]) for r in rs]),
                         rng([int(r["constraints"]) for r in rs]), rng(regs) if regs else "-"]) + r" \\")
t += [r"\bottomrule", r"\end{tabular}"]
with open(os.path.join(OUT, "tab_sizes.tex"), "w") as f:
    f.write("\n".join(t) + "\n")

# ---- full list: a complete longtable (not a row fragment: \input of rows inside a longtable breaks \midrule);
# suite sub-headers carry the problem type, so there is no type column
NCOL = 6
HEAD = r"problem & $n_\theta$ & $n_x$ & constraints & $\theta$ rows & regions \\"
body = [r"\begin{longtable}{@{}L{4.4cm} r r r r r@{}}",
        r"\caption{Size of every benchmark problem.}\label{tab:sizes_full}\\",
        r"\toprule", HEAD, r"\endfirsthead", r"\toprule", HEAD, r"\endhead"]
for s in SUITES:
    rs = by[s]
    body.append(r"\midrule")
    body.append(r"\multicolumn{%d}{@{}l@{}}{\textbf{%s} (%s, %d problems)} \\" % (NCOL, esc(TITLES[s]), tt(s), len(rs)))
    body.append(r"\addlinespace[2pt]")
    for r in rs:
        reg = n(r["regions"]) if r["regions"] else "-"
        body.append(" & ".join([tt(r["problem"]), r["theta"], r["x"], raw_to(int(r["constraints_raw"]), int(r["constraints"])),
                                raw_to(int(r["theta_constraints_raw"]), int(r["theta_constraints"])), reg]) + r" \\")
body += [r"\bottomrule", r"\end{longtable}"]
with open(os.path.join(OUT, "tab_sizes_full.tex"), "w") as f:
    f.write("\n".join(body) + "\n")

print("tables written: tab_sizes.tex, tab_sizes_full.tex;", len(rows), "problems")
