#!/bin/bash
#SBATCH --job-name=rpopt_build
#SBATCH --dependency=singleton
#SBATCH --account=rkhir
#SBATCH --partition=cpu
#SBATCH --cpus-per-task=8
#SBATCH --mem=6G
#SBATCH --time=00:50:00
#SBATCH --output=/home/hsuhsing/rpopt/retry-census/logs/%x_%j.out
#SBATCH --error=/home/hsuhsing/rpopt/retry-census/logs/%x_%j.err
# Builds, once per source version, (a) the release mplcp_bench of the main tree (item 1 and the base times) and (b) the gate-profile
# retry_census of the census tree (feature lp_census). Both share one target directory; a binary already in bin/ is not rebuilt.
#   sbatch build.sh <main-tag> <census-tag>      sources in $RC/main-src and $RC/census-src
set -euo pipefail
RC=$HOME/rpopt/retry-census
MAINTAG=$1; CENTAG=$2
export CARGO_BUILD_JOBS=${SLURM_CPUS_PER_TASK}
export CARGO_TARGET_DIR=/scratch/negishi/hsuhsing/retry-census/target
mkdir -p "$RC/bin" "$CARGO_TARGET_DIR"
if [ ! -x "$RC/bin/mplcp_bench_main_$MAINTAG" ]; then
  cd "$RC/main-src"
  find . -type f \( -name "*.rs" -o -name "Cargo.toml" -o -name "Cargo.lock" \) -exec touch {} +
  cargo build --release --example mplcp_bench
  cp "$CARGO_TARGET_DIR/release/examples/mplcp_bench" "$RC/bin/mplcp_bench_main_$MAINTAG"
fi
if [ ! -x "$RC/bin/retry_census_$CENTAG" ]; then
  cd "$RC/census-src"
  find . -type f \( -name "*.rs" -o -name "Cargo.toml" -o -name "Cargo.lock" \) -exec touch {} +
  cargo build --profile gate --features lp_census --example retry_census
  cp "$CARGO_TARGET_DIR/gate/examples/retry_census" "$RC/bin/retry_census_$CENTAG"
fi
ln -sfn "$RC/bin/mplcp_bench_main_$MAINTAG" "$RC/bin/mplcp_bench_main"
ln -sfn "$RC/bin/retry_census_$CENTAG" "$RC/bin/retry_census"
