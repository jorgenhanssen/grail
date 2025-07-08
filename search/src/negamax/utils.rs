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
pub fn calculate_dynamic_lmr_reduction(depth: u64, move_index: usize, score: i32, in_check: bool) -> u64 {
    if depth <= 3 {
        // do not reduce  immediate moves
        return 0;
    }
    if in_check {
        // do not reduce checks
        return 0;
    }
    if score > 0 {
        // do not reduce important moves
        return 0;
    }

    // quiet moves, reduce by 1 ply
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
