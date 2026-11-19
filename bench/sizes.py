"""Problem sizes for the benchmark report scripts: one shared lookup, one Markdown annotator.

Source: results/problem_sizes.csv, written by `cargo run --profile gate --example problem_sizes`
(examples/problem_sizes.rs). Stdlib only.

    tag("di_N4")  ->  "2/4/18"     theta / x / constraints (rows the solvers see after loading)

`annotate_md(text)` inserts a `size (theta/x/m)` column right after the `problem` column of every Markdown table whose
first header cell is `problem`, and a one-line legend at the top. It only adds cells (no number is changed or reformatted),
is idempotent, and leaves tables that already have a size column alone. The report generators call it before writing, and

    python sizes.py FILE.md [FILE.md ...]

applies it in place to a committed table that cannot be regenerated (its inputs are not in the repository).
"""
import csv
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CSV = os.path.join(HERE, "results", "problem_sizes.csv")
HEADER = "size (theta/x/m)"
LEGEND = ("size = theta/x/m: parameters, variables, constraints (rows of A x <= b + F theta that the solvers see after "
          "loading; `results/problem_sizes.md` lists every problem with the raw count and the regions).")

_cache = None


def load():
    """name -> csv row (dict). Problem names are unique across the five suites."""
    global _cache
    if _cache is None:
        with open(CSV, newline="") as f:
            _cache = {r["problem"]: r for r in csv.DictReader(f)}
    return _cache


def tag(name):
    """'theta/x/m' for a known problem, else '-'."""
    r = load().get(name.strip())
    return f"{r['theta']}/{r['x']}/{r['constraints']}" if r else "-"


def _cells(line):
    return [c.strip() for c in line.strip().strip("|").split("|")]


def _insert_after_first(line, text, pad=True):
    """Insert a cell after the first one, keeping the rest of the line byte for byte."""
    first_end = line.index("|", line.index("|") + 1)  # the '|' that closes the first cell
    return line[:first_end] + (f"| {text} " if pad else f"|{text}") + line[first_end:]


def annotate_md(text, legend=True):
    """Add the size column to every table whose header starts with `problem` and that has at least one known problem
    in its first column (a table of free-text rows is left alone)."""
    lines = text.split("\n")
    out = []
    changed = False
    i = 0
    while i < len(lines):
        line = lines[i]
        c = _cells(line) if line.startswith("|") else []
        if c and c[0] == "problem" and len(c) > 1 and not c[1].startswith("size") and i + 1 < len(lines):
            j = i + 2  # header, separator, then the body rows
            while j < len(lines) and lines[j].startswith("|"):
                j += 1
            body = lines[i + 2:j]
            if any(_cells(b)[0] in load() for b in body):
                out.append(_insert_after_first(line, HEADER))
                out.append(_insert_after_first(lines[i + 1], "---", pad=False))
                out.extend(_insert_after_first(b, tag(_cells(b)[0])) for b in body)
                changed = True
                i = j
                continue
        out.append(line)
        i += 1
    if changed and legend and LEGEND not in text:
        return LEGEND + "\n\n" + "\n".join(out)
    return "\n".join(out)


if __name__ == "__main__":
    legend = "--no-legend" not in sys.argv
    for path in [a for a in sys.argv[1:] if not a.startswith("--")]:
        with open(path, newline="", encoding="utf8") as f:
            raw = f.read()
        crlf = "\r\n" in raw
        new = annotate_md(raw.replace("\r\n", "\n"), legend)
        if crlf:
            new = new.replace("\n", "\r\n")
        if new != raw:
            with open(path, "w", newline="", encoding="utf8") as f:
                f.write(new)
            print("annotated", path)
        else:
            print("unchanged", path)
