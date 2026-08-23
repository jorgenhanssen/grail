#!/bin/bash
set -e

CONCURRENCY=${1:-15}
ELO0=${ELO0:-0}
ELO1=${ELO1:-5}
BOOK=${BOOK:-books/UHO_Lichess_4852_v1.epd}

rm -rf sprt
mkdir -p sprt

COMMON=(
  -openings file=$BOOK format=epd order=random
  -draw movenumber=40 movecount=8 score=10
  -resign movecount=3 score=400
  -ratinginterval 10
  -autosaveinterval 0
  -repeat -recover
  -engine cmd=./target/release/grail name=grail
  -engine cmd=./target/release/grail-next name=grail-next
  -sprt elo0=$ELO0 elo1=$ELO1 alpha=0.05 beta=0.05
  -rounds 5000
  -concurrency $CONCURRENCY
)

trap 'echo; echo "^C — skipping current stage..."' INT

echo "SPRT: STC (10+0.1, Hash=16) [elo0=$ELO0 elo1=$ELO1]"
fastchess "${COMMON[@]}" -each tc=10+0.1 option.Hash=16 2>&1 | tee sprt/stc.log || true

echo "SPRT: LTC (60+0.6, Hash=64) [elo0=$ELO0 elo1=$ELO1]"
fastchess "${COMMON[@]}" -each tc=60+0.6 option.Hash=64 2>&1 | tee sprt/ltc.log || true

echo "SPRT: VLTC (180+1.8, Hash=192) [elo0=$ELO0 elo1=$ELO1]"
fastchess "${COMMON[@]}" -each tc=180+1.8 option.Hash=192 2>&1 | tee sprt/vltc.log || true
