use cozy_chess::{Board, Move};
use utils::{captured_piece, piece_value};

use crate::history::CaptureHistory;

pub(super) struct ScoredMove {
    pub mov: Move,
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

pub(super) fn capture_score(board: &Board, mv: Move, capture_history: &CaptureHistory) -> i16 {
    let victim = captured_piece(board, mv).unwrap();
    let hist = capture_history.get(board, mv);

    piece_value(victim) + hist
}
