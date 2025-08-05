mod castling;
mod countermove_heuristic;
mod history_heuristic;
mod move_order;

use chess::{Board, ChessMove, Piece};
use evaluation::piece_value;
pub use move_order::ordered_moves;

pub use castling::Castle;
pub use countermove_heuristic::CountermoveHeuristic;
pub use history_heuristic::HistoryHeuristic;
use uci::commands::Score;

#[inline(always)]
pub fn is_zugzwang(board: &Board) -> bool {
    let side_bits = *board.color_combined(board.side_to_move());
    let pawn_bits = *board.pieces(Piece::Pawn) & side_bits;
    let king_bits = *board.pieces(Piece::King) & side_bits;

    // King + pawn endgame is typical zugzwang
    (side_bits ^ pawn_bits ^ king_bits).popcnt() == 0
}

#[inline(always)]
pub fn game_phase(board: &Board) -> f32 {
    let knights = board.pieces(Piece::Knight);
    let bishops = board.pieces(Piece::Bishop);
    let rooks = board.pieces(Piece::Rook);
    let queens = board.pieces(Piece::Queen);

    let score = knights.popcnt() + bishops.popcnt() + 2 * rooks.popcnt() + 4 * queens.popcnt();

    (score.min(24) as f32) / 24.0
}

#[inline(always)]
pub fn convert_mate_score(score: i16, pv: &[ChessMove]) -> Score {
    let mate_in = (pv.len() as i16 + 1) / 2;

    if score > 0 {
        Score::Mate(mate_in)
    } else {
        Score::Mate(-mate_in)
    }
}

#[inline(always)]
pub fn convert_centipawn_score(score: i16) -> Score {
    Score::Centipawns(score)
}

#[inline(always)]
pub fn see_naive(board: &Board, capture_move: ChessMove, phase: f32) -> i16 {
    if let (Some(captured_piece), Some(capturing_piece)) = (
        board.piece_on(capture_move.get_dest()),
        board.piece_on(capture_move.get_source()),
    ) {
        piece_value(captured_piece, phase) - piece_value(capturing_piece, phase)
    } else {
        0
    }
}
