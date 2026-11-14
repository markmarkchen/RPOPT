#!/usr/bin/env python3
"""Builds references.tex (a thebibliography block) from refs.bib for the keys cited in technical_report.tex.
Each entry with a DOI gets a Purdue EZproxy link; entries without a DOI get their arXiv link."""
import re, os

HERE = os.path.dirname(os.path.abspath(__file__))
tex = open(os.path.join(HERE, "technical_report.tex")).read()
bib = open(os.path.join(HERE, "refs.bib")).read()

# cited keys in order of first appearance
keys = []
for m in re.finditer(r"\\cite[a-z]*\{([^}]*)\}", tex):
    for k in m.group(1).split(","):
        k = k.strip()
        if k and k not in keys:
            keys.append(k)


def parse_entries(text):
    out = {}
    for m in re.finditer(r"@(\w+)\{([^,]+),", text):
        typ, key = m.group(1), m.group(2)
        i = m.end()
        depth = 1
        j = i
        while depth and j < len(text):
            if text[j] == "{":
                depth += 1
            elif text[j] == "}":
                depth -= 1
            j += 1
        body = text[i:j - 1]
        fields = {}
        for fm in re.finditer(r"(\w+)\s*=\s*", body):
            name = fm.group(1).lower()
            k0 = fm.end()
            if k0 >= len(body):
                continue
            if body[k0] == "{":
                d = 1
                k1 = k0 + 1
                while d and k1 < len(body):
                    if body[k1] == "{":
                        d += 1
                    elif body[k1] == "}":
                        d -= 1
                    k1 += 1
                fields.setdefault(name, body[k0 + 1:k1 - 1])
            elif body[k0] == '"':
                k1 = body.index('"', k0 + 1)
                fields.setdefault(name, body[k0 + 1:k1])
            else:
                k1 = re.search(r"[,\s]", body[k0:]).start() + k0
                fields.setdefault(name, body[k0:k1])
        fields["_type"] = typ
        out[key] = fields
    return out


E = parse_entries(bib)


def initials(first):
    parts = re.split(r"[\s]+", first.strip())
    res = []
    for p in parts:
        if not p:
            continue
        if p.startswith("{"):
            res.append(p.strip("{}")[:1] + ".")
        elif "-" in p:
            res.append("-".join(q[:1] + "." for q in p.split("-") if q))
        else:
            res.append(p[:1] + "." if not p.endswith(".") else p)
    return " ".join(res)


def authors(a):
    names = [x.strip() for x in re.split(r"\s+and\s+", a)]
    out = []
    for n in names:
        if "," in n:
            last, first = n.split(",", 1)
            out.append(f"{last.strip()}, {initials(first)}")
        else:
            out.append(n)
    if len(out) > 6:
        return ", ".join(out[:6]) + ", et al."
    if len(out) > 1:
        return ", ".join(out[:-1]) + ", and " + out[-1]
    return out[0]


def clean(s):
    return re.sub(r"\s+", " ", s.replace("\n", " ")).strip()


lines = [r"\begin{thebibliography}{99}", r"\small", r"\setlength{\itemsep}{1pt}"]
missing = []
for i, k in enumerate(keys, 1):
    f = E.get(k)
    if not f:
        missing.append(k)
        continue
    parts = [authors(clean(f.get("author", "")))] if "author" in f else []
    parts.append(f"{clean(f.get('year', ''))}.")
    parts.append(clean(f.get("title", "")) + ".")
    venue = f.get("journal") or f.get("booktitle") or f.get("howpublished") or ""
    venue = clean(venue)
    vol = f.get("volume")
    num = f.get("number")
    pg = f.get("pages")
    vs = r"\emph{" + venue + "}" if venue else ""
    if vol:
        vs += f", {vol}" + (f"({num})" if num else "")
    if pg:
        vs += f", {clean(pg)}"
    if vs:
        parts.append(vs + ".")
    doi = f.get("doi")
    if doi:
        url = "https://login.ezproxy.lib.purdue.edu/login?url=https://doi.org/" + doi
        parts.append(r"\href{" + url + r"}{doi:" + doi.replace("_", r"\_") + "}")
    else:
        hp = f.get("howpublished", "")
        m = re.search(r"arXiv:(\d+\.\d+)", hp)
        if m:
            parts.append(r"\href{https://arxiv.org/abs/" + m.group(1) + r"}{arXiv:" + m.group(1) + "}")
    lines.append(r"\bibitem{" + k + "} " + " ".join(parts))
lines.append(r"\end{thebibliography}")
open(os.path.join(HERE, "references.tex"), "w").write("\n".join(lines) + "\n")
print(len(keys), "cited keys; missing:", missing)
