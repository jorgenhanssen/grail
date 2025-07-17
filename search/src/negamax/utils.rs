use chess::{Board, ChessMove, Piece};
use evaluation::piece_value;
use uci::commands::Score;

use crate::utils::MAX_PIECE_PRIORITY;

#[inline(always)]
pub fn see_naive(board: &Board, capture_move: ChessMove) -> i16 {
    if let (Some(captured_piece), Some(capturing_piece)) = (
        board.piece_on(capture_move.get_dest()),
        board.piece_on(capture_move.get_source()),
    ) {
        piece_value(captured_piece) - piece_value(capturing_piece)
    } else {
        0
    }
}

#[inline(always)]
pub fn lmr(remaining_depth: u8, score: i16, check: bool) -> u8 {
    // Don't reduce if:
    // - At low depth
    // - In check
    // - Tactical moves (captures, promotions, PV/TT moves with score > 0)
    if remaining_depth < 3 || check || score > MAX_PIECE_PRIORITY {
        return 0;
    }

    // Reduce quiet moves by 1 ply.
    return 1;
}

#[inline(always)]
pub fn convert_mate_score(score: i16, pv: &Vec<ChessMove>) -> Score {
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
pub fn is_zugzwang(board: &Board) -> bool {
    let side_bits = *board.color_combined(board.side_to_move());
    let pawn_bits = *board.pieces(Piece::Pawn) & side_bits;
    let king_bits = *board.pieces(Piece::King) & side_bits;

    // King + pawn endgame is typical zugzwang
    (side_bits ^ pawn_bits ^ king_bits).popcnt() == 0
}
