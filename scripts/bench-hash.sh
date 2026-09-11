#!/bin/bash
set -euo pipefail

FILE=bench-hash.txt

hash=$(./target/release/grail bench | grep 'Bench hash' | awk '{print $NF}')

if [ "${1:-}" = update ]; then
  echo "$hash" > "$FILE"
  echo "updated $FILE"
  exit
fi

expected=$(cat "$FILE")
if [ "$hash" != "$expected" ]; then
  echo "bench hash mismatch: expected $expected got $hash"
  exit 1
fi
