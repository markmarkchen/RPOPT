#!/bin/bash
#SBATCH --account=rkhir
#SBATCH --partition=cpu
#SBATCH --job-name=rpopt_arch_merge
#SBATCH --cpus-per-task=1
#SBATCH --mem=1900M
#SBATCH --time=00:40:00
#SBATCH --output=%x_%j.out
# Usage: sbatch --dependency=afterany:<ids> --export=ALL,COMMIT=...,PLAN=...,SRC=... merge.sbatch
set -euo pipefail
SCR=/scratch/negishi/hsuhsing
ARCH=$SCR/rpopt-solutions/$COMMIT
python3 "$SRC/bench/archive/merge.py" "$PLAN" "$ARCH" "$ARCH/manifest.csv"
mkdir -p "$HOME/rpopt/rpopt-solutions/$COMMIT"
cp "$ARCH/manifest.csv" "$HOME/rpopt/rpopt-solutions/$COMMIT/manifest.csv"
