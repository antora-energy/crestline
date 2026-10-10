#!/bin/bash
# Profile-guided build of crest (same results as the plain build: PGO only
# moves code and changes inlining and branch layout).
#   HIGHS_DIR=<HiGHS checkout> bench/pgo_build.sh [MODEL ...]
# Trains on LPs and MIPs of $HIGHS_DIR/check/instances plus the given models
# (e.g. a few of the production MILPs; MIPs stop at 500 nodes or 60 s), then
# builds target/release/crest with the profile. RUSTFLAGS and cargo
# arguments (CARGO_ARGS, e.g. '--features "libstdcxx no_fma"') pass through;
# for x86-64-v3: RUSTFLAGS="-C target-cpu=x86-64-v3" bench/pgo_build.sh ...
# Needs llvm-profdata: rustup component add llvm-tools
set -eu
ROOT=$(cd "$(dirname "$0")/.." && pwd)
I=${HIGHS_DIR:?set HIGHS_DIR to a HiGHS checkout}/check/instances
PROFDATA=$(ls "$(rustc --print sysroot)"/lib/rustlib/*/bin/llvm-profdata | head -1)
BASE="-C llvm-args=-unroll-runtime=false ${RUSTFLAGS:-}"
tmp=$(mktemp -d)
cd "$ROOT"
RUSTFLAGS="$BASE -Cprofile-generate=$tmp/raw" eval cargo build --release --bin crest \
  --target-dir target/pgo-gen ${CARGO_ARGS:-}
crest=target/pgo-gen/release/crest
printf "threads = 1\nmip_rel_gap = 0.01\nmip_max_nodes = 500\ntime_limit = 60\n" > $tmp/mip.opts
for f in scrs8 e226 shell israel standata gas11 sctest dcmulti etamacro galenet; do
  $crest --presolve off $I/$f.mps > /dev/null 2>&1 || true
  $crest $I/$f.mps > /dev/null 2>&1 || true
  printf "simplex_strategy = 4\n" > $tmp/p.opts; $crest --options_file $tmp/p.opts $I/$f.mps > /dev/null 2>&1 || true
done
for f in scrs8 dcmulti; do $crest --solver ipm $I/$f.mps > /dev/null 2>&1 || true; done
printf "pdlp_iteration_limit = 5000\n" > $tmp/pdlp.opts
for f in scrs8 shell; do $crest --solver pdlp --options_file $tmp/pdlp.opts $I/$f.mps > /dev/null 2>&1 || true; done
for f in bell5 egout rgn p0548 lseu gesa2 flugpl "$@"; do
  case $f in /*) m=$f;; *) m=$I/$f.mps;; esac
  $crest --options_file $tmp/mip.opts "$m" > /dev/null 2>&1 || true
done
"$PROFDATA" merge -o $tmp/crest.profdata $tmp/raw
RUSTFLAGS="$BASE -Cprofile-use=$tmp/crest.profdata" eval cargo build --release --bin crest ${CARGO_ARGS:-}
rm -rf $tmp
