#!/bin/bash
#SBATCH --account=rkhir
#SBATCH --partition=cpu
#SBATCH --job-name=rpopt_build
#SBATCH --dependency=singleton
#SBATCH --cpus-per-task=8
#SBATCH --mem=6G
#SBATCH --time=00:40:00
#SBATCH --output=%x_%j.out
# Builds examples/region_hash (release) from $SRC into the persistent target $TGT and copies the binary to
# $BIN/<commit>/region_hash. Usage: sbatch --export=ALL,SRC=...,COMMIT=... build.sbatch
set -euo pipefail
SCR=/scratch/negishi/hsuhsing
TGT=${TGT:-$SCR/rpopt-archive-target}
BIN=${BIN:-$SCR/rpopt-archive-bin}
mkdir -p "$BIN/$COMMIT"
if [ -x "$BIN/$COMMIT/region_hash" ]; then echo "cached binary for $COMMIT"; exit 0; fi
cd "$SRC"
# the login environment sets CC=mpicc, which the compute nodes lack (mimalloc's build script fails on it)
export CC=gcc CXX=g++
CARGO_TARGET_DIR=$TGT cargo build --release --example region_hash -j "$SLURM_CPUS_PER_TASK"
cp "$TGT/release/examples/region_hash" "$BIN/$COMMIT/region_hash"
ls -l "$BIN/$COMMIT/region_hash"
