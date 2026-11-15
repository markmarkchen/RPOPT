#!/bin/bash
# Submits the archive of commit $1 (12 chars) from the sources in $SRC: one array per memory class, then the merge job.
# Run on the login node (only sbatch). Prerequisite: the binary exists (sbatch build.sbatch).
set -euo pipefail
COMMIT=$1
SRC=${SRC:-$HOME/rpopt/archive-src/$COMMIT}
SCR=/scratch/negishi/hsuhsing
PLAN=${PLAN:-$SRC/bench/archive/plan.csv}
W=$SCR/rpopt-archive-work/$COMMIT
mkdir -p "$W"
cd "$W"
# class: name|cpus|mem|time|regex over problem names (explicit CPUs: above 2010 MB per CPU SLURM adds CPUs)
CLASSES="c40|20|40000M|02:00:00|^pop_036$
c32|16|32000M|01:00:00|^pop_095$
lphd|8|16000M|01:30:00|^(masses7one_N1|masses8one_N1)$
c51|6|11400M|01:10:00|^pop_051$
c3800|3|5900M|00:30:00|^(pop_024|pop_033)$
c3000|2|3000M|00:30:00|^pop_002$"
cp "$PLAN" rest.csv
IDS=""
while IFS='|' read -r NAME CPUS MEM TIME RE; do
  awk -F, -v re="$RE" '$2 ~ re' "$PLAN" > "$NAME.csv"
  awk -F, -v re="$RE" '$2 !~ re' rest.csv > rest.tmp
  mv rest.tmp rest.csv
  N=$(wc -l < "$NAME.csv")
  [ "$N" -gt 0 ] || continue
  J=$(sbatch --parsable -A rkhir -p cpu -J "arch_$NAME" --array=0-$((N-1)) --cpus-per-task="$CPUS" --mem="$MEM" --time="$TIME" -o "%x_%A_%a.out" -e "%x_%A_%a.err" \
      --export=ALL,COMMIT="$COMMIT",PROBDIR="$SRC/bench",OFFSET=0 --wrap "bash $SRC/bench/archive/run_task.sh $W/$NAME.csv" < /dev/null)
  IDS="$IDS:$J"
  echo "$NAME $N tasks job $J"
done <<< "$CLASSES"
N=$(wc -l < rest.csv)
# at most 1001 array elements per job
for ((off = 0; off < N; off += 1000)); do
  M=$((N - off < 1000 ? N - off : 1000))
  J=$(sbatch --parsable -A rkhir -p cpu -J arch_small --array=0-$((M-1)) --cpus-per-task=1 --mem=1900M --time=00:30:00 -o "%x_%A_%a.out" -e "%x_%A_%a.err" \
      --export=ALL,COMMIT="$COMMIT",PROBDIR="$SRC/bench",OFFSET=$off --wrap "bash $SRC/bench/archive/run_task.sh $W/rest.csv" < /dev/null)
  IDS="$IDS:$J"
  echo "small $M tasks offset $off job $J"
done
sbatch --parsable -A rkhir -p cpu --dependency=afterany$IDS --export=ALL,COMMIT="$COMMIT",PLAN="$PLAN",SRC="$SRC" "$SRC/bench/archive/merge.sbatch"
