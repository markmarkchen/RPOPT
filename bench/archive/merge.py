#!/usr/bin/env python3
"""manifest.csv for one archive: one row per planned instance. Usage: merge.py <plan.csv> <archive_dir> <manifest.csv>"""
import csv, hashlib, json, os, sys

plan, arch, out = sys.argv[1:4]
cols = ["dir", "problem", "solver", "reason", "status", "regions", "mplcp_table_regions", "region_flag", "solve_s", "bytes", "hash",
        "sets_hash", "roundtrip_ok", "rss_peak_mb", "host", "sha256"]
rows = []
for line in open(plan):
    d, p, solver, reason, treg = line.strip().split(",")
    r = dict(dir=d, problem=p, solver=solver, reason=reason, mplcp_table_regions=treg, status="missing")
    jf, rf = "%s/%s/%s.json" % (arch, d, p), "%s/%s/%s.rpsol" % (arch, d, p)
    if os.path.exists(jf) and os.path.exists(rf):
        j = json.load(open(jf))
        h = hashlib.sha256()
        with open(rf, "rb") as f:
            for chunk in iter(lambda: f.read(1 << 24), b""):
                h.update(chunk)
        ok = j["roundtrip_ok"] and os.path.getsize(rf) == j["bytes"]
        r.update(status="ok" if ok else "bad", regions=j["regions"], solve_s=j["solve_s"], bytes=j["bytes"], hash=j["hash"],
                 sets_hash=j["sets_hash"], roundtrip_ok=j["roundtrip_ok"], rss_peak_mb=j["rss_peak_mb"], host=j["host"], sha256=h.hexdigest())
        if treg:
            same = int(treg) == j["regions"]
            # a solver other than mplcp whose count differs from mplcp's stored count may hold an incomplete partition;
            # an mplcp count that differs is drift since the table (later merges changed the region sets)
            r["region_flag"] = "ok" if same else ("MISMATCH" if solver != "mplcp" else "drift")
    rows.append(r)
with open(out, "w") as f:
    w = csv.DictWriter(f, cols)
    w.writeheader()
    w.writerows(rows)
print("%d rows, ok %d, bad %d, missing %d, MISMATCH %d, drift %d, bytes %d" % (
    len(rows), sum(r["status"] == "ok" for r in rows), sum(r["status"] == "bad" for r in rows),
    sum(r["status"] == "missing" for r in rows), sum(r.get("region_flag") == "MISMATCH" for r in rows),
    sum(r.get("region_flag") == "drift" for r in rows), sum(r.get("bytes", 0) for r in rows)))
