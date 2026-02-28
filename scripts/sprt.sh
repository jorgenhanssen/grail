#!/bin/bash
set -e

CONCURRENCY=${1:-15}

rm -rf sprt
mkdir -p sprt

COMMON=(
  -openings file=books/UHO_Lichess_4852_v1.epd format=epd order=random
  -draw movenumber=40 movecount=8 score=10
  -resign movecount=3 score=400
  -ratinginterval 10
  -autosaveinterval 0
  -repeat -recover
  -engine cmd=./target/release/grail name=grail
  -engine cmd=./target/release/grail-next name=grail-next
  -sprt elo0=0 elo1=10 alpha=0.05 beta=0.05
  -rounds 10000
  -concurrency $CONCURRENCY
)

echo "SPRT: STC (10+0.1)"
fastchess "${COMMON[@]}" -each tc=10+0.1 2>&1 | tee sprt/stc.log

echo "SPRT: LTC (60+0.6)"
fastchess "${COMMON[@]}" -each tc=60+0.6 2>&1 | tee sprt/ltc.log

echo "SPRT: VLTC (180+1.8)"
fastchess "${COMMON[@]}" -each tc=180+1.8 2>&1 | tee sprt/vltc.log