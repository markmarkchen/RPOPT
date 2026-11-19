#!/usr/bin/env python3
"""Generates the LaTeX tables of the RPOPT technical report from bench/results (no numbers typed by hand)."""
import os, re, statistics

HERE = os.path.dirname(os.path.abspath(__file__))
RES = os.path.join(HERE, "..", "..", "bench", "results")
OUT = os.path.join(HERE, "tables")
os.makedirs(OUT, exist_ok=True)


def read(n):
    with open(os.path.join(RES, n)) as f:
        return f.read()


def cells(line):
    return [x.strip() for x in line.strip().strip("|").split("|")]


def num(s):
    return float(s.replace(",", ""))


def esc(s):
    s = s.replace("\\", r"\textbackslash{}")
    for a, b in (("_", r"\_"), ("%", r"\%"), ("&", r"\&"), ("#", r"\#")):
        s = s.replace(a, b)
    return s


def tt(s):
    return r"\code{" + esc(s) + "}"


def val(s):
    """A number-with-unit string from the result files, as LaTeX."""
    s = s.strip()
    s = s.replace(">", "$>$").replace(" us", r" \textmu s").replace("us ", r"\textmu s ")
    s = re.sub(r"(\d) us$", r"\1 \\textmu s", s)
    s = s.replace("%", r"\%").replace("_", r"\_")
    return s


def regcount(x):
    """Region count column: 'a / b' with a = PPOPT (or '-' when unfinished) and b = RPOPT; a equals b when both exist."""
    a, b = [v.strip() for v in x.split("/")]
    assert a in ("-", b), x
    return b


def write(name, text):
    with open(os.path.join(OUT, name), "w") as f:
        f.write(text)


def rows_of(section_text, ncols):
    out = []
    for line in section_text.splitlines():
        if not line.startswith("|"):
            continue
        p = cells(line)
        if len(p) == ncols and p[0] != "problem" and not set(p[0]) <= set("-: "):
            out.append(p)
    return out


# ------------------------------------------------------------------ mpQP vs PPOPT
sp = read("speedup.md")
same = rows_of(sp.split("### Same algorithm, single thread")[1].split("###")[0], 6)
t = [r"\begin{tabular}{@{}llrrrr@{}}", r"\toprule",
     r"problem & algorithm & PPOPT & RPOPT (1 thread) & speedup & regions \\", r"\midrule"]
prev = None
for p in same:
    if prev and p[0] != prev:
        t.append(r"\addlinespace[2pt]")
    prev = p[0]
    t.append(" & ".join([tt(p[0]) if p[0] != "" else "", tt(p[1]), val(p[2]), val(p[3]), val(p[4]), regcount(p[5])]) + r" \\")
t += [r"\bottomrule", r"\end{tabular}"]
write("tab_same.tex", "\n".join(t))

par = rows_of(sp.split("### Parallel algorithms")[1].split("###")[0], 6)
t = [r"\begin{tabular}{@{}lrrrr@{}}", r"\toprule",
     r"problem & PPOPT (processes) & RPOPT (16 threads) & speedup & regions \\", r"\midrule"]
for p in par:
    t.append(" & ".join([tt(p[0]), val(p[2]), val(p[3]), val(p[4]), regcount(p[5])]) + r" \\")
t += [r"\bottomrule", r"\end{tabular}"]
write("tab_par.tex", "\n".join(t))

fast = rows_of(sp.split("### Fastest serial algorithm per problem")[1].split("###")[0], 4)
t = [r"\begin{tabular}{@{}lllr@{}}", r"\toprule",
     r"problem & PPOPT's fastest & RPOPT's fastest (1 thread) & speedup \\", r"\midrule"]
for p in fast:
    def alg(x):
        m = re.match(r"(\S+) (.*)", x)
        return tt(m.group(1)) + " " + val(m.group(2))
    t.append(" & ".join([tt(p[0]), alg(p[1]), alg(p[2]), val(p[3])]) + r" \\")
t += [r"\bottomrule", r"\end{tabular}"]
write("tab_fast.tex", "\n".join(t))

# ------------------------------------------------------------------ mpLP vs PPOPT
full = read("mplp_full.txt")
regs = {}
for line in full.splitlines():
    m = re.match(r"^(\w+)\s+\d+\s+\d+\s+\d+\s+mplp_lex\s+(\d+)\s", line)
    if m:
        regs.setdefault(m.group(1), int(m.group(2)))
ms = read("mplp_speedup.md")
rows = rows_of(ms, 5)
t = [r"\begin{tabular}{@{}l r L{3.9cm} L{3.1cm} r r@{}}", r"\toprule",
     r"problem & regions & PPOPT, fastest gap-free & PPOPT \code{geometric} (sampled gaps) & RPOPT \code{mplp\_lex} & speedup \\", r"\midrule"]
for p in rows:
    a = p[1]
    if a.startswith("none"):
        a = "none in 120 s"
    else:
        m = re.match(r"(\S+) (.*)", a)
        a = tt(m.group(1)) + "~" + val(m.group(2)).replace(" ", "~")
    g = p[2].replace("(", "(").replace(" gaps", "")
    g = val(g).replace("crashed", "crashes")
    r_ = p[3].replace("mplp_lex ", "")
    t.append(" & ".join([tt(p[0]), f"{regs[p[0]]:,}", a, g, val(r_), val(p[4]).replace("$>$ ", "$>$")]) + r" \\")
t += [r"\bottomrule", r"\end{tabular}"]
write("tab_mplp.tex", "\n".join(t))

# same-algorithm mpLP summary by algorithm, computed from mplp_same_algorithm.md
sa = read("mplp_same_algorithm.md")
by = {}
for line in sa.splitlines():
    p = cells(line)
    if len(p) == 5 and p[0] not in ("problem",) and p[4].endswith("x") and p[4][0].isdigit():
        by.setdefault(p[1], []).append(num(p[4][:-1]))
t = [r"\begin{tabular}{@{}lrrrr@{}}", r"\toprule", r"algorithm & pairs & min & median & max \\", r"\midrule"]
allv = []
for a in ["combinatorial", "combinatorial_graph", "geometric", "graph"]:
    v = by[a]
    allv += v
    t.append(f"{tt(a)} & {len(v)} & {min(v):,.0f}x & {statistics.median(v):,.0f}x & {max(v):,.0f}x \\\\")
t.append(r"\midrule")
t.append(f"all & {len(allv)} & {min(allv):,.0f}x & {statistics.median(allv):,.0f}x & {max(allv):,.0f}x \\\\")
t += [r"\bottomrule", r"\end{tabular}"]
write("tab_mplp_same.tex", "\n".join(t))
print("mplp same-algorithm: ", len(allv), min(allv), statistics.median(allv), max(allv))

# mpLP gap coverage of PPOPT (parsed from ppopt_lp_coverage.txt)
cov = read("ppopt_lp_coverage.txt")
t = [r"\begin{tabular}{@{}llrrrr@{}}", r"\toprule",
     r"problem & PPOPT algorithm & regions & uncovered samples & share & wrong objective \\", r"\midrule"]
for line in cov.splitlines():
    m = re.match(r"^(\S+)\.json (\w+): regions (\d+), feasible samples (\d+), gaps (\d+) \(([\d.]+)%\), wrong objective (\d+)", line)
    if m:
        t.append(f"{tt(m.group(1))} & {tt(m.group(2))} & {int(m.group(3)):,} & {m.group(5)} of {m.group(4)} & {m.group(6)}\\% & {m.group(7)} \\\\")
    elif "TypeError" in line:
        t.append(r"\texttt{sys3x2inf\_N3} & \texttt{geometric} & \multicolumn{4}{l}{crashes (\texttt{TypeError}, no solution)} \\")
t += [r"\bottomrule", r"\end{tabular}"]
# keep order of the file but reposition the crash row after sys3x2inf_N2
write("tab_gaps.tex", "\n".join(t))

# ------------------------------------------------------------------ scoreboard summary
def score_summary(fn, order):
    out = {}
    for line in read(fn).splitlines():
        m = re.match(r"^(\w+): median ([\d.]+)x, min ([\d.]+)x, max ([\d.]+)x \((\d+)", line)
        if m:
            out[m.group(1)] = m.groups()[1:]
    return out


qs = score_summary("scoreboard_vs_778cdb9_mpqp.md", None)
ls = score_summary("scoreboard_vs_778cdb9_mplp.md", None)
t = [r"\begin{tabular}{@{}lrrrrrrr@{}}", r"\toprule",
     r" & \multicolumn{3}{c}{mpQP (12 problems)} & & \multicolumn{3}{c}{mpLP (timed pairs)} \\",
     r"\cmidrule(r){2-4}\cmidrule(l){6-8}",
     r"algorithm & min & median & max & & min & median & max \\", r"\midrule"]
algs = ["combinatorial", "combinatorial_graph", "geometric", "graph", "mplcp", "mplp_lex"]
for a in algs:
    q = qs.get(a)
    l = ls.get(a)
    def cm(v):
        x = float(v)
        return f"{x:,.2f}"
    qq = f"{cm(q[1])}x & {cm(q[0])}x & {cm(q[2])}x" if q else "-- & -- & --"
    ll = f"{cm(l[1])}x & {cm(l[0])}x & {cm(l[2])}x" if l else "-- & -- & --"
    t.append(f"{tt(a)} & {qq} & & {ll} \\\\")
t += [r"\bottomrule", r"\end{tabular}"]
write("tab_score.tex", "\n".join(t))

# ------------------------------------------------------------------ thread scaling
th = read("thread_scaling.md")
rows = rows_of(th, 12)
t = [r"\begin{tabular}{@{}llrrrrrrr@{}}", r"\toprule",
     r"problem & algorithm & serial & 1 & 2 & 4 & 8 & 16 & 16 vs 1 \\", r"\midrule"]
prev = None
for p in rows:
    if prev and p[0] != prev:
        t.append(r"\addlinespace[2pt]")
    prev = p[0]
    tf = lambda x: f"{float(x.replace(' ms', '').replace(',', '')):,.1f}"
    t.append(" & ".join([tt(p[0]), tt(p[1].replace("_parallel", "")), tf(p[2]), tf(p[3]), tf(p[4]), tf(p[5]), tf(p[6]), tf(p[8]), p[9].replace("x", r"$\times$")]) + r" \\")
t += [r"\bottomrule", r"\end{tabular}"]
write("tab_threads.tex", "\n".join(t))

# ------------------------------------------------------------------ point location (all methods)
pl = read("point_location.md")
rows = rows_of(pl, 10)
t = [r"\begin{tabular}{@{}lrrrrrrrr@{}}", r"\toprule",
     r"problem & regions & seq. & seq. early & BST & multiway & descr. & hash & YANN \\", r"\midrule"]
have, none_ = [], []
for p in rows:
    if p[6].startswith("none"):
        none_.append(p[0])
    else:
        have.append((p[0], p[6]))
    c = lambda x: f"{int(x.replace(',', '')):,}"
    t.append(" & ".join([tt(p[0]), c(p[1]), c(p[2]), c(p[3]), c(p[4]), c(p[5]), c(p[7]), c(p[8]), c(p[9])]) + r" \\")
write("note_linmach.tex", "\\newcommand{\\linmachnote}{linear machine, ns per query: " + ", ".join(f"{tt(a)} {b}" for a, b in have)
      + "; none exists on " + ", ".join(tt(a) for a in none_) + "}")
t += [r"\bottomrule", r"\end{tabular}"]
write("tab_pl_methods.tex", "\n".join(t))

# ------------------------------------------------------------------ point location dimension sweep (busy)
data = {}
order = []
for l in read("pl_dim_sweep_busy_machine.txt").splitlines():
    p = l.rstrip("\n").split("\t")
    if len(p) >= 13 and p[0] != "spec":
        k = (p[0], p[1], p[2])
        if k not in data:
            order.append(k)
        data.setdefault(k, {})[p[3]] = p
pick = ["file:di_N10", "qp:8:3:100:2", "file:masses2_N6", "qp:8:4:60:1", "qp:6:5:40:1", "mass:3:3", "qp:4:6:14:1", "qp:4:8:14:1"]
t = [r"\begin{tabular}{@{}lrrrrrr@{}}", r"\toprule",
     r"instance & $n_\theta$ & regions & sequential & BST & optimized tree & hash table, 8 MB \\", r"\midrule"]
mism = set()


def g(v, *names):
    for n in names:
        if n in v:
            return v[n], ("*" if "truncated" in n else "")
    return None, ""


for k in sorted(order, key=lambda k: (int(k[1]), int(k[2]))):
    if k[0] in pick:
        v = data[k]
        s, _ = g(v, "sequential")
        b, bs = g(v, "bst", "bst (truncated)")
        o, os_ = g(v, "opt tree", "opt tree (truncated)")
        h, _ = g(v, "hash 8MB")
        def f(x, star=""):
            return (f"{float(x[6]):,.0f}" + star) if x else "--"
        name = k[0].replace("file:", "")
        t.append(" & ".join([tt(name), k[1], f"{int(k[2]):,}", f(s), f(b, bs), f(o, os_), f(h)]) + r" \\")
        mism.add(h[12] if h else "--")
t += [r"\bottomrule", r"\end{tabular}"]
assert mism == {"0"}, mism
write("tab_pl_sweep.tex", "\n".join(t))

# ------------------------------------------------------------------ horizon scaling selected points
sc = {}
for line in read("scaling.txt").splitlines():
    m = re.match(r"^(\w+)_N(\d+)\s+\d+\s+\d+\s+(\d+)\s+(.+?)\s{2,}([0-9.]+|skipped|failed)\s", line + " ")
    if m:
        sc[(m.group(1), int(m.group(2)), m.group(4).strip())] = (m.group(5), int(m.group(3)))
cols = ["mplcp", "geometric", "graph", "combinatorial_graph", "parametric_daqp", "ahmadi", "dp (2023 sym)", "combinatorial"]
heads = ["mplcp", "geom.", "graph", "comb. graph", "pdaqp", "ahmadi", "dp", "comb."]
sel = [("di", 10), ("di", 12), ("di", 20), ("di", 30), ("masses2", 6), ("masses2", 8), ("sys3x2", 5), ("sys3x2", 7)]


def fmt(x):
    if x in ("skipped", "failed"):
        return "--" if x == "skipped" else "fail"
    v = float(x)
    if v >= 100:
        return f"{v:,.0f}"
    if v >= 10:
        return f"{v:.1f}"
    if v >= 1:
        return f"{v:.2f}"
    return f"{v:.3f}"


t = [r"\begin{tabular}{@{}lr" + "r" * len(cols) + "@{}}", r"\toprule",
     "instance & regions & " + " & ".join(heads) + r" \\", r"\midrule"]
for fam, N in sel:
    reg = sc[(fam, N, "mplcp")][1]
    t.append(" & ".join([tt(f"{fam}_N{N}".replace("di_", "di_")), f"{reg:,}"] + [fmt(sc[(fam, N, c)][0]) for c in cols]) + r" \\")
t += [r"\bottomrule", r"\end{tabular}"]
write("tab_scaling.tex", "\n".join(t))

# ------------------------------------------------------------------ large suite summary
lg = read("large.md")
rows = rows_of(lg.split("## Summary by algorithm")[1], 5)  # algorithm | solved | timed out | returned no regions | time vs graph
t = [r"\begin{tabular}{@{}lrrrr@{}}", r"\toprule",
     r"algorithm & solved & timed out & no regions & time vs \texttt{graph} \\", r"\midrule"]
for p in rows:
    if p[0] == "algorithm":
        continue
    t.append(f"{tt(p[0])} & {p[1]} & {p[2]} & {p[3]} & {p[4]} \\\\")
t += [r"\bottomrule", r"\end{tabular}"]
write("tab_large.tex", "\n".join(t))

# ------------------------------------------------------------------ LP census overview
ce = read("lp_census.md").split("### Overview")[1].split("###")[0]
rows = rows_of(ce, 11)
t = [r"\begin{tabular}{@{}llrrrrr@{}}", r"\toprule",
     r"problem & algorithm & regions & LPs & LPs per region & QPs & LP time / solve$^\dagger$ \\", r"\midrule"]
prev = None
for p in rows:
    if p[0] not in ("masses2_N6", "sys3x2_N5", "diinf_N4"):
        continue
    if p[1] == "combinatorial" and p[0] == "diinf_N4":
        continue
    if prev and p[0] != prev:
        t.append(r"\addlinespace[2pt]")
    prev = p[0]
    t.append(" & ".join([tt(p[0]), tt(p[1]), p[2].replace(",", ""), f"{int(p[4]):,}", f"{float(p[5]):,.1f}", f"{int(p[6]):,}", p[7].replace("%", r"\%")]) + r" \\")
t += [r"\bottomrule", r"\end{tabular}"]
write("tab_census.tex", "\n".join(t))

# ------------------------------------------------------------------ coverage summary
cv = read("coverage.md")
lines = [l for l in cv.splitlines() if re.match(r"^(50%|90%|99%|100%|all regions|total):", l)]
t = [r"\begin{tabular}{@{}lrrrr@{}}", r"\toprule", r"coverage level & minimum & median & maximum & runs at 200x or more \\", r"\midrule"]
for l in lines:
    m = re.match(r"^([\w% ]+): speedup min (\d+)x \(([^)]*)\), median ([\d,]+)x, max ([\d,]+)x over (\d+) runs; (\d+) at 200x", l)
    if m:
        level = m.group(1).replace("%", r"\%")
        t.append(f"{level} & {m.group(2)}x & {m.group(4)}x & {m.group(5)}x & {m.group(7)} of {m.group(6)} \\\\")
t += [r"\bottomrule", r"\end{tabular}"]
write("tab_coverage.tex", "\n".join(t))
print("tables written:", sorted(os.listdir(OUT)))
