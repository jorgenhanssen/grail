mod capture_history;
mod countermove;
mod history_heuristic;
mod move_order;
mod see;

use chess::{Board, Piece};
pub use move_order::{MainMoveGenerator, QMoveGenerator};

pub use capture_history::CaptureHistory;
pub use countermove::CountermoveTable;
use evaluation::scores::MATE_VALUE;
pub use history_heuristic::HistoryHeuristic;
pub use see::see;
use uci::commands::Score;

#[inline(always)]
pub fn is_zugzwang(board: &Board) -> bool {
    let side_bits = *board.color_combined(board.side_to_move());
    let pawn_bits = *board.pieces(Piece::Pawn) & side_bits;
    let king_bits = *board.pieces(Piece::King) & side_bits;

    // Only king and pawns (common zugzwang scenario)
    if side_bits == (pawn_bits | king_bits) {
        return true;
    }

    // Positions with no pawns and no major pieces and at most
    // one minor piece are also prone to null-move failures.
    let knight_bits = *board.pieces(Piece::Knight) & side_bits;
    let bishop_bits = *board.pieces(Piece::Bishop) & side_bits;
    let rook_bits = *board.pieces(Piece::Rook) & side_bits;
    let queen_bits = *board.pieces(Piece::Queen) & side_bits;

    let has_pawns = pawn_bits.popcnt() > 0;
    let has_major = (rook_bits | queen_bits).popcnt() > 0;
    let minor_count = (knight_bits | bishop_bits).popcnt();

    !has_pawns && !has_major && minor_count <= 1
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
pub fn convert_mate_score(score: i16) -> Score {
    let mate_plies = (MATE_VALUE - score.abs()).max(0);
    let mate_in = (mate_plies + 1) / 2;
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
