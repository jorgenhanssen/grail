// Move ordering for quiescence search inspired by Black Marlin

use arrayvec::ArrayVec;
use chess::{Board, ChessMove, MoveGen};
use evaluation::piece_values::PieceValues;

use crate::history::CaptureHistory;

use super::utils::{capture_score, select_highest, ScoredMove};

pub const MAX_FORCING_MOVES: usize = 32;

pub struct QMoveGenerator {
    forcing_moves: ArrayVec<ScoredMove, MAX_FORCING_MOVES>,
}

impl QMoveGenerator {
    pub fn new(
        in_check: bool,
        board: &Board,
        capture_history: &CaptureHistory,
        phase: f32,
        piece_values: PieceValues,
    ) -> Self {
        let mut gen = MoveGen::new_legal(board);

        if !in_check {
            gen.set_iterator_mask(*board.color_combined(!board.side_to_move()));

            let mut forcing_moves = ArrayVec::new();

            for mov in gen.take(MAX_FORCING_MOVES) {
                forcing_moves.push(ScoredMove {
                    mov,
                    score: capture_score(board, mov, capture_history, phase, &piece_values),
                });
            }

            Self { forcing_moves }
        } else {
            let mut forcing_moves = ArrayVec::new();
            for mov in gen.take(MAX_FORCING_MOVES) {
                forcing_moves.push(ScoredMove { mov, score: 0 });
            }
            Self { forcing_moves }
        }
    }

    pub fn next(&mut self) -> Option<ChessMove> {
        if let Some(index) = select_highest(&self.forcing_moves) {
            let scored_move = self.forcing_moves.swap_remove(index);
            return Some(scored_move.mov);
        }
        None
    }
}

