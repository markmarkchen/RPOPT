#!/bin/bash
# Login node, sbatch only. Submits the build, then the arrays behind it, then the merge.
#   bash submit.sh <main-tag> <census-tag>
# Layout in ~/rpopt/retry-census: main-src, census-src, prob/, man/, bin/, out/, logs/, task.py, merge.py, array.sh, build.sh
set -euo pipefail
RC=$HOME/rpopt/retry-census
cd "$RC"
mkdir -p logs out
/usr/bin/python3 make_manifests.py man
b=$(sbatch --parsable build.sh "$1" "$2")
n() { wc -l < "man/$1.txt"; }
sub() { sbatch --parsable --dependency=afterok:$b "$@"; }
c23=$(sub --job-name=rc_c23 --array=0-$(( $(n c23) - 1 )) array.sh man/c23.txt)
big=$(sub --job-name=rc_c23big --cpus-per-task=2 --mem=4000M --time=06:00:00 --array=0-0 array.sh man/c23_big.txt)
c4=$(sub --job-name=rc_c4 --array=0-$(( $(n c4) - 1 )) array.sh man/c4.txt)
# the timing arrays wait for the censuses, so that the timed tasks share nodes with fewer other jobs
tw() { sbatch --parsable --dependency=afterany:$c23:$big:$c4 "$@"; }
base=$(tw --mem=1G --job-name=rc_base --array=0-$(( $(n base) - 1 )) array.sh man/base.txt)
cert=$(tw --mem=1G --job-name=rc_cert --array=0-$(( $(n cert) - 1 )) array.sh man/cert.txt)
m=$(sbatch --parsable --dependency=afterany:$c23:$big:$c4:$base:$cert --job-name=rc_merge --account=rkhir --partition=cpu --cpus-per-task=1 --mem=1900M --time=00:20:00 --output="$RC/logs/%x_%j.out" --wrap="export RC=$RC; /usr/bin/python3 $RC/merge.py > $RC/summary.md")
echo "build $b c23 $c23 big $big c4 $c4 base $base cert $cert merge $m"
