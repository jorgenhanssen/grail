#!/usr/bin/env python3
"""Quick look at eval vs outcome disagreements in datagen CSVs.

Usage:
  python3 scripts/eval_outcome_disagreement.py
  python3 scripts/eval_outcome_disagreement.py -t 600
  python3 scripts/eval_outcome_disagreement.py -t 800 nnue/data/*.csv
"""

import argparse
from pathlib import Path

import pandas as pd

EXAMPLES = 5

FEN = "fen"
SCORE = "score"
OUTCOME = "outcome"

WHITE = "W"
BLACK = "B"
DRAW = "D"

THRESHOLD = 600


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("files", nargs="*", type=Path)
    parser.add_argument("-t", "--threshold", type=int, default=THRESHOLD)
    args = parser.parse_args()

    files = args.files or sorted(Path("nnue/data").glob("*.csv"))
    threshold = args.threshold

    total = 0
    draws = 0
    draw_disagreements = 0
    decisive_disagreements = 0
    draw_hits = []
    decisive_hits = []

    for path in files:
        df = pd.read_csv(path, usecols=[FEN, SCORE, OUTCOME])
        total += len(df)

        is_draw = df[OUTCOME] == DRAW
        draws += int(is_draw.sum())

        # draw, but eval is way off
        high_eval_draw = is_draw & (df[SCORE].abs() > threshold)

        # game was decisive, but eval strongly favored the other side
        white_won_but_eval_black = (df[OUTCOME] == WHITE) & (df[SCORE] < -threshold)
        black_won_but_eval_white = (df[OUTCOME] == BLACK) & (df[SCORE] > threshold)
        opposite_eval = white_won_but_eval_black | black_won_but_eval_white

        draw_disagreements += int(high_eval_draw.sum())
        decisive_disagreements += int(opposite_eval.sum())
        draw_hits.append(df.loc[high_eval_draw, [SCORE, OUTCOME, FEN]])
        decisive_hits.append(df.loc[opposite_eval, [SCORE, OUTCOME, FEN]])

    disagreements = draw_disagreements + decisive_disagreements

    print(f"files:                 {len(files)}")
    print(f"threshold:             {threshold} cp")
    print(f"total samples:         {total:,}")
    print(f"draws:                 {draws:,} ({100 * draws / total:.3f}%)")
    print(f"draw + |score| > T:    {draw_disagreements:,} ({100 * draw_disagreements / total:.3f}%)")
    print(f"decisive + opposite:   {decisive_disagreements:,} ({100 * decisive_disagreements / total:.3f}%)")
    print(f"total disagreements:   {disagreements:,} ({100 * disagreements / total:.3f}%)")

    print_examples("Draw disagreements", draw_hits, draw_disagreements)
    print_examples("Decisive opposites", decisive_hits, decisive_disagreements)


def print_examples(title, hits, count):
    if not count:
        return

    examples = pd.concat(hits).sample(n=min(EXAMPLES, count))
    print(f"\n{title}:")
    for row in examples.itertuples(index=False):
        print(f"  {int(row.score):+5d}  {row.outcome}  {row.fen}")


if __name__ == "__main__":
    main()
