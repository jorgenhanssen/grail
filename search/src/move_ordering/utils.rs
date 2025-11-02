use chess::{Board, ChessMove};
use evaluation::piece_values::PieceValues;

use crate::history::CaptureHistory;

pub(super) struct ScoredMove {
    pub mov: ChessMove,
    pub score: i16,
}

pub(super) fn select_highest(array: &[ScoredMove]) -> Option<usize> {
    if array.is_empty() {
        return None;
    }
    let mut best_score = array[0].score;
    let mut best_index = 0;
    for (index, mv) in array.iter().enumerate().skip(1) {
        if mv.score > best_score {
            best_score = mv.score;
            best_index = index;
        }
    }
    Some(best_index)
}

#[inline(always)]
pub(super) fn capture_score(
    board: &Board,
    mv: ChessMove,
    capture_history: &CaptureHistory,
    phase: f32,
    piece_values: &PieceValues,
) -> i16 {
    let victim = board.piece_on(mv.get_dest()).unwrap();
    let attacker = board.piece_on(mv.get_source()).unwrap();
    let hist = capture_history.get(attacker, mv.get_dest(), victim);

    piece_values.get(victim, phase) + hist
}

