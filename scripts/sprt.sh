#!/bin/bash
set -e

CONCURRENCY=${CONCURRENCY:-15}
MODE=${MODE:-standard}
ELO0=${ELO0:-0}
ELO1=${ELO1:-5}

case "$MODE" in
  standard)
    VARIANT=standard
    BOOK=${BOOK:-books/UHO_Lichess_4852_v1.epd}
    ;;
  frc)
    VARIANT=fischerandom
    BOOK=${BOOK:-books/chess960.epd}
    ;;
  *)
    echo "Invalid MODE: $MODE (expected standard or frc)" >&2
    exit 1
    ;;
esac

LOG_DIR="sprt/$MODE"
rm -rf "$LOG_DIR"
mkdir -p "$LOG_DIR"

COMMON=(
  -variant "$VARIANT"
  -openings "file=$BOOK" format=epd order=random
  -draw movenumber=40 movecount=8 score=10
  -resign movecount=3 score=400
  -ratinginterval 10
  -autosaveinterval 0
  -repeat -recover
  -engine cmd=./target/release/grail name=grail
  -engine cmd=./target/release/grail-next name=grail-next
  -sprt "elo0=$ELO0" "elo1=$ELO1" alpha=0.05 beta=0.05
  -rounds 5000
  -concurrency "$CONCURRENCY"
)

trap 'echo; echo "^C - skipping current stage..."' INT

run() {
  local name=$1
  local tc=$2
  local hash=$3

  echo "SPRT ($MODE): $name ($tc, Hash=$hash) [elo0=$ELO0 elo1=$ELO1]"
  fastchess "${COMMON[@]}" -each "tc=$tc" "option.Hash=$hash" \
    2>&1 | tee "$LOG_DIR/$name.log" || true
}

run stc 10+0.1 16
run ltc 60+0.6 64
run vltc 180+1.8 192
