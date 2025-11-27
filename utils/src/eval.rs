use cozy_chess::{Board, Color};

/// Flip eval perspective between White's view and side-to-move's view.
#[inline(always)]
pub fn flip_eval_perspective(board: &Board, score: i16) -> i16 {
    if board.side_to_move() == Color::White {
        score
    } else {
        -score
    }
}
