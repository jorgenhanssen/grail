use std::fmt::Write;

use cozy_chess::{Board, Color, File, Rank, Square};

const LINE: &str = "  +---+---+---+---+---+---+---+---+\n";

/// Renders a board as ASCII art.
pub fn board_to_ascii(board: &Board) -> String {
    let mut out = String::new();

    out.push_str(LINE);

    for rank in Rank::ALL.iter().rev() {
        write!(out, "{} |", *rank as u8 + 1).unwrap();

        for file in &File::ALL {
            write!(out, " {} |", square_char(board, Square::new(*file, *rank))).unwrap();
        }

        out.push('\n');
        out.push_str(LINE);
    }

    out.push_str("    a   b   c   d   e   f   g   h\n");

    out
}

fn square_char(board: &Board, sq: Square) -> char {
    match (board.piece_on(sq), board.color_on(sq)) {
        (Some(piece), Some(Color::White)) => char::from(piece).to_ascii_uppercase(),
        (Some(piece), Some(_)) => char::from(piece),
        _ => ' ',
    }
}
