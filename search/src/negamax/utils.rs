use chess::{Board, ChessMove};
use evaluation::piece_value;
use uci::commands::Score;

#[inline(always)]
pub fn see_naive(board: &Board, capture_move: ChessMove) -> f32 {
    if let (Some(captured_piece), Some(capturing_piece)) = (
        board.piece_on(capture_move.get_dest()),
        board.piece_on(capture_move.get_source()),
    ) {
        piece_value(captured_piece) - piece_value(capturing_piece)
    } else {
        0.0
    }
}

#[inline(always)]
pub fn lmr(remaining_depth: u64, score: i32, in_check: bool) -> u64 {
    // Don't reduce if:
    // - At low depth
    // - In check
    // - Tactical moves (captures, promotions, PV/TT moves with score > 0)
    if remaining_depth < 3 || in_check || score > 0 {
        return 0;
    }

    // Reduce quiet moves by 1 ply.
    return 1;
}

#[inline(always)]
pub fn convert_mate_score(board: &Board, score: f32, pv: &Vec<ChessMove>) -> Score {
    let is_winning = (score > 0.0) == (board.side_to_move() == chess::Color::White);
    let mate_in = if is_winning {
        pv.len() as i32 - 1
    } else {
        -((pv.len() as i32) - 1)
    };
    Score::Mate(mate_in)
}

#[inline(always)]
pub fn convert_centipawn_score(score: f32) -> Score {
    Score::Centipawns((100.0 * score) as i32)
}
