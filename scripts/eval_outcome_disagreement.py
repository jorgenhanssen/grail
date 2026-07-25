#!/usr/bin/env python3
"""Quick look at eval vs outcome disagreements in datagen CSVs.

Usage:
  python3 scripts/eval_outcome_disagreement.py
  python3 scripts/eval_outcome_disagreement.py -t 600
  python3 scripts/eval_outcome_disagreement.py -t 800 nnue/data/*.csv
"""

import argparse
from pathlib import Path

import polars as pl

EXAMPLES = 5

COL_FEN = "fen"
COL_GAME_ID = "game_id"
COL_SCORE = "score"
COL_OUTCOME = "outcome"

WHITE = "W"
BLACK = "B"
DRAW = "D"

THRESHOLD = 600


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("files", nargs="*", type=Path)
    parser.add_argument("-t", "--threshold", type=int, default=THRESHOLD)
    args = parser.parse_args()

    files = [str(path) for path in (args.files or sorted(Path("nnue/data").glob("*.csv")))]
    threshold = args.threshold

    total, draw_disagreements, decisive_disagreements = count_disagreements(
        files, threshold
    )

    print(f"Files:                 {len(files)}")
    print(f"Threshold:             {threshold} cp")
    print(f"Total samples:         {total:,}")

    draw_examples = find_examples(files, draw_disagreement(threshold))
    decisive_examples = find_examples(files, decisive_disagreement(threshold))
    print_examples("Draw disagreements", draw_disagreements, total, draw_examples)
    print_examples("Decisive opposites", decisive_disagreements, total, decisive_examples)


def scan_csv(files, columns):
    return pl.scan_csv(
        files,
        schema_overrides={COL_SCORE: pl.Int32},
    ).select(columns)


def draw_disagreement(threshold):
    return (pl.col(COL_OUTCOME) == DRAW) & (pl.col(COL_SCORE).abs() > threshold)


def decisive_disagreement(threshold):
    white_won_but_eval_black = (pl.col(COL_OUTCOME) == WHITE) & (
        pl.col(COL_SCORE) < -threshold
    )
    black_won_but_eval_white = (pl.col(COL_OUTCOME) == BLACK) & (
        pl.col(COL_SCORE) > threshold
    )
    return white_won_but_eval_black | black_won_but_eval_white


def count_disagreements(files, threshold):
    data = scan_csv(files, [COL_SCORE, COL_OUTCOME])
    stats = data.select(
        pl.len().alias("total"),
        draw_disagreement(threshold).sum().alias("draw_disagreements"),
        decisive_disagreement(threshold).sum().alias("decisive_disagreements"),
    ).collect(engine="streaming")
    return stats.row(0)

# Find examples from different games without keeping all disagreements in memory.
def find_examples(files, condition):
    # game_id starts over in each file so the source file is part of the id.
    source_file = "_source_file"
    data = pl.scan_csv(
        files,
        schema_overrides={COL_SCORE: pl.Int32},
        include_file_paths=source_file,
    ).select([COL_SCORE, COL_OUTCOME, COL_FEN, COL_GAME_ID, source_file])

    # Without this the five examples can just be five moves from the same game (which is often take case).
    return (
        data.filter(condition)
        .unique(
            subset=[source_file, COL_GAME_ID],
            keep="first",
            maintain_order=True,
        )
        .head(EXAMPLES)
        .select([COL_SCORE, COL_OUTCOME, COL_FEN])
        .collect(engine="streaming")
    )


def print_examples(title, count, total, examples):
    print(f"\n{title}: {count:,} ({100 * count / total:.3f}%)")
    for score, outcome, fen in examples.iter_rows():
        print(f"  {int(score):+5d}  {outcome}  {fen}")


if __name__ == "__main__":
    main()
