//! UCI move notation helpers.
//!
//! cozy-chess uses "king captures rook" notation for castling internally (e.g., e1h1),
//! but UCI expects standard notation (e.g., e1g1). These helpers handle the conversion.

use cozy_chess::{util::display_uci_move, Board, Move};

/// Convert a single move to UCI notation, handling castling correctly.
#[inline]
pub fn move_to_uci(board: &Board, mv: Move) -> String {
    display_uci_move(board, mv).to_string()
}

/// Convert a PV (principal variation) to UCI notation.
/// Each move is converted in sequence, with the board updated after each move.
pub fn pv_to_uci(starting_board: &Board, pv: &[Move]) -> Vec<String> {
    let mut result = Vec::with_capacity(pv.len());
    let mut board = starting_board.clone();

    for &mv in pv {
        result.push(move_to_uci(&board, mv));
        board.play_unchecked(mv);
    }

    result
}
