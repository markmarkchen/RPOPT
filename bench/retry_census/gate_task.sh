#!/bin/bash
#SBATCH --account=rkhir
#SBATCH --partition=cpu
#SBATCH --cpus-per-task=1
#SBATCH --mem=1900M
#SBATCH --time=04:00:00
#SBATCH --output=/home/hsuhsing/rpopt/retry-census/logs/%x_%A_%a.out
#SBATCH --error=/home/hsuhsing/rpopt/retry-census/logs/%x_%A_%a.err
# One solve per task: a line "<problem> <path> <cert>" of the manifest; cert 1 is main's default, 0 the new default.
# Writes gate/<problem>_<cert>.sets (sorted active sets) and .txt (region_hash line; rows rounded at RPOPT_HASH_SCALE=1e6).
#   sbatch --array=0-N gate_task.sh <tag> <manifest>
set -euo pipefail
RC=$HOME/rpopt/retry-census
TAG=$1
read -r name path cert < <(sed -n "$((SLURM_ARRAY_TASK_ID + 1))p" "$2")
O=$RC/gate/$TAG
mkdir -p "$O"
[ -s "$O/${name}_$cert.txt" ] && exit 0
RAYON_NUM_THREADS=1 RPOPT_CERT=$cert RPOPT_HASH_SCALE=1e6 RPOPT_SETS_OUT="$O/${name}_$cert.sets" \
  "$RC/bin/region_hash_$TAG" mplcp "$path" > "$O/${name}_$cert.tmp"
mv "$O/${name}_$cert.tmp" "$O/${name}_$cert.txt"
