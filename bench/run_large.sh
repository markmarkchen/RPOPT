#!/bin/sh
# Runs the large benchmark (problem families in bench/problems_large) and writes results/large.json and results/large.md.
# Run from ppopt_rs/. The machine may be shared: all timings are "busy machine" timings.
#
#   cargo build --release --bin ppopt_bench
#   bench/run_large.sh [max_seconds_per_solve=600]
set -e
CAP=${1:-600}
BIN=target/release/ppopt_bench
DIR=bench/problems_large
OUT=bench/results/large.json
PY=${PY:-python3}

rm -f "$OUT"
# main pass: every algorithm except the exponential combinatorial and mplcp_enum; point location in both query modes
$BIN "$DIR" "$OUT" --algos geometric,graph,combinatorial_graph,ahmadi,mplcp --queries 100000 --traj 1000,100 --max-seconds "$CAP"

# combinatorial only where it can finish: problems whose graph solve took < 0.5 s, at most 60 s each
ONLY=$($PY - "$OUT" <<'EOF'
import json, sys
d = json.load(open(sys.argv[1]))
names = sorted({k.split('/')[0] for k in d})
ok = [n for n in names if f'{n}/graph' in d and not d[f'{n}/graph'].get('timed_out') and d[f'{n}/graph']['solve_s'] < 0.5
      and d.get(f'{n}/info', {}).get('constraints', 99) <= 40]
print(','.join(ok))
EOF
)
if [ -n "$ONLY" ]; then
  $BIN "$DIR" "$OUT" --algos combinatorial --only "$ONLY" --no-pl --resume --max-seconds 60
fi
$PY bench/large_report.py "$OUT" bench/results/large.md
