use crate::traditional::bonus::TEMPO_BONUS;
use chess::{Board, Color};

/// Evaluates tempo advantage for the side to move
#[inline(always)]
pub(super) fn evaluate(board: &Board, phase: f32) -> i16 {
    let is_white = board.side_to_move() == Color::White;

    // Tempo bonus for side to move
    if is_white {
        ((TEMPO_BONUS as f32) * phase).round() as i16
    } else {
        -((TEMPO_BONUS as f32) * phase).round() as i16
    }
}
