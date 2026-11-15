#!/bin/bash
#SBATCH --job-name=rpopt_build
#SBATCH --dependency=singleton
#SBATCH --account=rkhir
#SBATCH --partition=cpu
#SBATCH --cpus-per-task=8
#SBATCH --mem=6G
#SBATCH --time=00:40:00
#SBATCH --output=/home/hsuhsing/rpopt/retry-census/logs/%x_%j.out
#SBATCH --error=/home/hsuhsing/rpopt/retry-census/logs/%x_%j.err
# Gate for the certificate-off default: builds region_hash (gate profile) of ~/rpopt/retry-census/off-src/<tag>.
#   sbatch gate_build.sh <tag>
set -euo pipefail
RC=$HOME/rpopt/retry-census
TAG=$1
export CARGO_BUILD_JOBS=${SLURM_CPUS_PER_TASK}
export CARGO_TARGET_DIR=/scratch/negishi/hsuhsing/retry-census/target
mkdir -p "$RC/bin" "$CARGO_TARGET_DIR"
if [ ! -x "$RC/bin/region_hash_$TAG" ]; then
  cd "$RC/off-src/$TAG"
  find . -type f \( -name "*.rs" -o -name "Cargo.toml" -o -name "Cargo.lock" \) -exec touch {} +
  cargo build --profile gate --example region_hash
  cp "$CARGO_TARGET_DIR/gate/examples/region_hash" "$RC/bin/region_hash_$TAG"
fi
