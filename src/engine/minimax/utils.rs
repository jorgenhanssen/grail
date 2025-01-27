use crate::uci::commands::Score;
use crate::utils::{piece_value, CHECK_SCORE};
use chess::{Board, ChessMove};

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
pub fn convert_centipawn_score(board: &Board, score: f32) -> Score {
    let cp_score = if board.side_to_move() == chess::Color::White {
        100.0 * score
    } else {
        -100.0 * score
    };
    Score::Centipawns(cp_score as i32)
}
