#!/bin/bash
#SBATCH --account=rkhir
#SBATCH --partition=cpu
#SBATCH --cpus-per-task=1
#SBATCH --mem=1900M
#SBATCH --time=03:00:00
#SBATCH --output=/home/hsuhsing/rpopt/retry-census/logs/%x_%A_%a.out
#SBATCH --error=/home/hsuhsing/rpopt/retry-census/logs/%x_%A_%a.err
# One 1-CPU task per line of the manifest: "<mode> <problem> <alt>". sbatch --array=0-N array.sh <manifest> [offset]
set -euo pipefail
export RC=$HOME/rpopt/retry-census
line=$(sed -n "$((SLURM_ARRAY_TASK_ID + ${2:-0} + 1))p" "$1")
exec /usr/bin/python3 "$RC/task.py" $line
