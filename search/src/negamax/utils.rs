use chess::{Board, ChessMove};
use evaluation::piece_value;
use uci::commands::Score;

#[inline(always)]
pub fn see_naive(board: &Board, capture_move: ChessMove) -> i32 {
    if let (Some(captured_piece), Some(capturing_piece)) = (
        board.piece_on(capture_move.get_dest()),
        board.piece_on(capture_move.get_source()),
    ) {
        (piece_value(captured_piece) - piece_value(capturing_piece)) as i32
    } else {
        0
    }
}

// Same as Weiss
#[inline(always)]
pub fn calculate_dynamic_lmr_reduction(depth: u64, move_index: usize, score: i32) -> u64 {
    // TODO: Fix this later
    return 0;

    // if score < CHECK_SCORE {
    //     (1.35 + (depth as f64).ln() * (move_index as f64).ln() / 2.75).ceil() as u32
    // } else {
    //     (0.20 + (depth as f64).ln() * (move_index as f64).ln() / 3.35).ceil() as u32
    // }
}

#[inline(always)]
pub fn convert_mate_score(score: i32, pv: &Vec<ChessMove>) -> Score {
    let mate_in = (pv.len() as i32 + 1) / 2;

    if score > 0 {
        Score::Mate(mate_in)
    } else {
        Score::Mate(-mate_in)
    }
}

#[inline(always)]
pub fn convert_centipawn_score(score: i32) -> Score {
    Score::Centipawns(score)
}
