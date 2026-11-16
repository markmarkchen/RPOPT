#!/bin/bash
# One array task = one instance. Usage (from the sbatch wrapper): run_task.sh <plan_class.csv>
# plan rows: dir,problem,solver,reason,mplcp_table_regions. Task id i takes row i+1 (+ OFFSET).
# Needs COMMIT (12 chars) and PROBDIR (the bench directory) in the environment.
set -euo pipefail
SCR=/scratch/negishi/hsuhsing
PLAN=$1
ROW=$(sed -n "$((SLURM_ARRAY_TASK_ID + ${OFFSET:-0} + 1))p" "$PLAN")
IFS=, read -r DIR PROB SOLVER REASON TREG <<< "$ROW"
OUT=$SCR/rpopt-solutions/$COMMIT/$DIR
mkdir -p "$OUT"
if [ -s "$OUT/$PROB.json" ] && [ -s "$OUT/$PROB.rpsol" ] && [ -s "$OUT/$PROB.sets" ]; then echo "skip $DIR/$PROB"; exit 0; fi
echo "$DIR/$PROB solver $SOLVER on $(hostname -s)"
export RPOPT_SAVE=$OUT/$PROB.rpsol RPOPT_SETS_OUT=$OUT/$PROB.sets RPOPT_JSON_OUT=$OUT/$PROB.json
rm -f "$RPOPT_JSON_OUT"
$SCR/rpopt-archive-bin/$COMMIT/region_hash "$SOLVER" "$PROBDIR/$DIR/$PROB.json"
test -s "$OUT/$PROB.json"
