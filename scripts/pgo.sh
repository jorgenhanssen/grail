#!/bin/bash
set -euo pipefail

# Usage: scripts/pgo.sh <cargo-selector> <workload>
#   grail:    scripts/pgo.sh "--bin grail" "./target/release/grail bench"
#   generate: scripts/pgo.sh "-p training --bin generate" "./target/release/generate ..."

CARGO_SELECTOR="$1"
WORKLOAD="$2"

PGO_DIR="$(pwd)/target/pgo-data"
rm -rf "$PGO_DIR"
mkdir -p "$PGO_DIR"

PROFDATA="$(dirname "$(rustc --print target-libdir)")/bin/llvm-profdata"
RUSTFLAGS="${RUSTFLAGS:-} -C profile-generate=$PGO_DIR" cargo build --release $CARGO_SELECTOR

$WORKLOAD

"$PROFDATA" merge -o "$PGO_DIR/merged.profdata" "$PGO_DIR"

RUSTFLAGS="${RUSTFLAGS:-} -C profile-use=$PGO_DIR/merged.profdata -C llvm-args=-pgo-warn-missing-function=false" cargo build --release $CARGO_SELECTOR
