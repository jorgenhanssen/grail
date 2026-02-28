use std::fmt::Write;

use cozy_chess::{Board, Color, File, Move, Rank, Square};

use crate::search_metadata::SearchResultMeta;

const LINE: &str = "  +---+---+---+---+---+---+---+---+\n";
const ANSI_RESET: &str = "\x1b[0m";

struct PvColor {
    from_bg: u8,
    to_bg: u8,
}

const PV_COLORS: &[PvColor] = &[
    PvColor {
        from_bg: 28,
        to_bg: 40,
    },
    PvColor {
        from_bg: 136,
        to_bg: 178,
    },
    PvColor {
        from_bg: 25,
        to_bg: 39,
    },
    PvColor {
        from_bg: 90,
        to_bg: 163,
    },
    PvColor {
        from_bg: 30,
        to_bg: 44,
    },
];

pub fn display_position(board: &Board, last_search: Option<&SearchResultMeta>) {
    match last_search {
        Some(search) => {
            let top_moves = search.top_moves();
            println!("\n{}", board_to_ascii(board, &top_moves));
            for (i, score) in search.scores_white().enumerate() {
                println!("{}", eval_bar_ascii(score, i));
            }
        }
        None => println!("\n{}", board_to_ascii(board, &[])),
    }
    println!();
}

fn board_to_ascii(board: &Board, top_moves: &[Move]) -> String {
    let mut out = String::new();

    out.push_str(LINE);

    for rank in Rank::ALL.iter().rev() {
        write!(out, "{} |", *rank as u8 + 1).unwrap();

        for file in &File::ALL {
            let sq = Square::new(*file, *rank);
            let ch = square_char(board, sq);

            if let Some(color_idx) = highlight_color(top_moves, sq) {
                write!(out, "\x1b[48;5;{color_idx}m\x1b[97m {} {ANSI_RESET}|", ch).unwrap();
            } else {
                write!(out, " {} |", ch).unwrap();
            }
        }

        out.push('\n');
        out.push_str(LINE);
    }

    out.push_str("    a   b   c   d   e   f   g   h\n");

    out
}

fn eval_bar_ascii(score_cp: i16, pv_index: usize) -> String {
    const BAR_WIDTH: usize = 33;

    let pct = score_to_pct(score_cp);
    let filled = ((pct * BAR_WIDTH as f64) / 100.0).round() as usize;
    let empty = BAR_WIDTH.saturating_sub(filled);

    let score_f = score_cp as f64 / 100.0;
    let sign = if score_f >= 0.0 { "+" } else { "" };
    let color = &PV_COLORS[pv_index % PV_COLORS.len()];
    format!(
        "  \x1b[38;5;{}m{}{ANSI_RESET}{} \x1b[38;5;{}m{}{:.2}{ANSI_RESET}",
        color.to_bg,
        "█".repeat(filled),
        "░".repeat(empty),
        color.to_bg,
        sign,
        score_f,
    )
}

fn highlight_color(top_moves: &[Move], sq: Square) -> Option<u8> {
    for (i, mv) in top_moves.iter().enumerate() {
        let color = &PV_COLORS[i % PV_COLORS.len()];
        if sq == mv.to {
            return Some(color.to_bg);
        } else if sq == mv.from {
            return Some(color.from_bg);
        }
    }
    None
}

// logistic sigmoid (to win prob).
// https://www.chessprogramming.org/Pawn_Advantage,_Win_Percentage,_and_Elo
fn score_to_pct(score_cp: i16) -> f64 {
    // K=4 is used in the wiki above, but 2.0 looks better as an eval bar
    const K: f64 = 2.0;

    let p = score_cp as f64 / 100.0; // from centipawns to pawns
    100.0 / (1.0 + (-p * K).exp())
}

fn square_char(board: &Board, sq: Square) -> char {
    match (board.piece_on(sq), board.color_on(sq)) {
        (Some(piece), Some(Color::White)) => char::from(piece).to_ascii_uppercase(),
        (Some(piece), Some(_)) => char::from(piece),
        _ => ' ',
    }
}
