#!/usr/bin/env bash
set -e

ENGINE="./target/release/grail"
COMMAND="negamax"
DEPTH=8

# --- Baselines
baseline_time=(642 7795 1987 7240)
baseline_nodes=(1886272 19566965 4749037 17012514)
baseline_nps=(2936310 2510063 2389740 2349539)

FENS=(
  "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
  "r2q1rk1/pp2bppp/2n1pn2/2bp4/2P5/2N1PN2/PP1BBPPP/R2Q1RK1 w - - 0 9"
  "r3k2r/ppp2ppp/2n2n2/3pp3/1b1PP3/2N2N2/PPP2PPP/R3K2R b KQkq d3 0 8"
  "2r2rk1/1b2qppp/p1n1pn2/1p2N3/2pP4/2N1B3/PP3PPP/2RQ1RK1 w - - 2 15"
)

# --- Totals for averages
sum_time=0
sum_nodes=0
sum_nps=0
sum_dtime=0
sum_dnodes=0
sum_dnps=0
ok=0

# Build the engine
make build-grail

# --- Header
echo "| FEN # | Time (ms) | Δ        | Nodes     | Δ        | NPS     | Δ        |"
echo "|-------|-----------|----------|-----------|----------|---------|----------|"

# --- Benchmark loop
for i in "${!FENS[@]}"; do
  FEN="${FENS[$i]}"
  b_time=${baseline_time[$i]}
  b_nodes=${baseline_nodes[$i]}
  b_nps=${baseline_nps[$i]}

  LINE=$(printf "position fen %s\ngo depth %s\nquit\n" "$FEN" "$DEPTH" \
    | "$ENGINE" "$COMMAND" 2>&1 | grep "depth $DEPTH" | tail -1)

  if [[ -z $LINE ]]; then
    echo "| $((i+1)) | -         | -        | -         | -        | -       | -        |"
    continue
  fi

  c_time=$(awk '{for(i=1;i<=NF;i++) if($i=="time") print $(i+1)}' <<< "$LINE")
  c_nodes=$(awk '{for(i=1;i<=NF;i++) if($i=="nodes") print $(i+1)}' <<< "$LINE")
  c_nps=$(awk '{for(i=1;i<=NF;i++) if($i=="nps") print $(i+1)}' <<< "$LINE")

  d_time=$(awk  "BEGIN{printf \"%+.2f\", (($c_time - $b_time) / $b_time) * 100}")
  d_nodes=$(awk "BEGIN{printf \"%+.2f\", (($c_nodes - $b_nodes) / $b_nodes) * 100}")
  d_nps=$(awk   "BEGIN{printf \"%+.2f\", (($c_nps - $b_nps) / $b_nps) * 100}")

  sum_time=$((sum_time + c_time))
  sum_nodes=$((sum_nodes + c_nodes))
  sum_nps=$((sum_nps + c_nps))
  sum_dtime=$(awk "BEGIN{print $sum_dtime + $d_time}")
  sum_dnodes=$(awk "BEGIN{print $sum_dnodes + $d_nodes}")
  sum_dnps=$(awk "BEGIN{print $sum_dnps + $d_nps}")
  ((ok++))

  printf "| %5d | %9d | **%+0.2f%%** | %9d | **%+0.2f%%** | %7d | **%+0.2f%%** |\n" \
  "$((i+1))" "$c_time" "$d_time" "$c_nodes" "$d_nodes" "$c_nps" "$d_nps"

done

# --- Final row with averages
if (( ok > 0 )); then
  avg_time=$((sum_time / ok))
  avg_nodes=$((sum_nodes / ok))
  avg_nps=$((sum_nps / ok))
  avg_dtime=$(awk  "BEGIN{printf \"%+.2f\", $sum_dtime / $ok}")
  avg_dnodes=$(awk "BEGIN{printf \"%+.2f\", $sum_dnodes / $ok}")
  avg_dnps=$(awk   "BEGIN{printf \"%+.2f\", $sum_dnps / $ok}")

  printf "| **Avg** | %9d | **%+0.2f%%** | %9d | **%+0.2f%%** | %7d | **%+0.2f%%** |\n" \
  "$avg_time" "$avg_dtime" "$avg_nodes" "$avg_dnodes" "$avg_nps" "$avg_dnps"

fi
