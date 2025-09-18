#!/usr/bin/env bash
set -e

ENGINE="./target/release/grail"
COMMAND="negamax"
DEPTH=16

# --- Baselines
baseline_time=(4425 3701 423 5413 479 525 1073 1472 3654 1534)
baseline_nodes=(11161242 8244919 1231091 13542916 1319706 1630207 2856694 4783309 9253696 4237350)
baseline_nps=(2522155 2227253 2903867 2501652 2751616 3104262 2661102 3248754 2532465 2760726)


FENS=(
  "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
  "r2q1rk1/pp2bppp/2n1pn2/2bp4/2P5/2N1PN2/PP1BBPPP/R2Q1RK1 w - - 0 9"
  "r3k2r/ppp2ppp/2n2n2/3pp3/1b1PP3/2N2N2/PPP2PPP/R3K2R b KQkq d3 0 8"
  "2r2rk1/1b2qppp/p1n1pn2/1p2N3/2pP4/2N1B3/PP3PPP/2RQ1RK1 w - - 2 15"
  "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1"
  "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8"
  "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10"
  "3r1k2/4npp1/1ppr3p/p6P/P2PPPP1/1NR5/5K2/2R5 w - - 0 1"
  "2q1rr1k/3bbnnp/p2p1pp1/2pPp3/PpP1P1P1/1P2BNNP/2BQ1PRK/7R b - - 0 1"
  "4rrk1/pp1n3p/3p2pq/2pP4/2P1PP2/2N3PQ/PP1B3P/3RRK2 b - - 0 1"
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
