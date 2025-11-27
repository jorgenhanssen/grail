use cozy_chess::{Board, Color};

/// Flip evaluation score between White's perspective and side-to-move's perspective.
///
/// Evaluators return scores from White's perspective (positive = White better).
/// Search algorithms need scores from the side-to-move's perspective (positive = good for STM).
pub fn flip_eval_perspective(board: &Board, score: i16) -> i16 {
    if board.side_to_move() == Color::White {
        score
    } else {
        -score
    }
}
