// Move ordering for quiescence search inspired by Black Marlin

use arrayvec::ArrayVec;
use cozy_chess::{Board, Move};
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
        let mut forcing_moves = ArrayVec::new();

        if !in_check {
            let enemy_pieces = board.colors(!board.side_to_move());

            board.generate_moves(|moves| {
                let mut captures = moves;
                captures.to &= enemy_pieces;

                for mov in captures {
                    if forcing_moves.len() >= MAX_FORCING_MOVES {
                        return true;
                    }
                    forcing_moves.push(ScoredMove {
                        mov,
                        score: capture_score(board, mov, capture_history, phase, &piece_values),
                    });
                }
                false
            });

            Self { forcing_moves }
        } else {
            board.generate_moves(|moves| {
                for mov in moves {
                    if forcing_moves.len() >= MAX_FORCING_MOVES {
                        return true;
                    }
                    forcing_moves.push(ScoredMove { mov, score: 0 });
                }
                false
            });
            Self { forcing_moves }
        }
    }

    pub fn next(&mut self) -> Option<Move> {
        if let Some(index) = select_highest(&self.forcing_moves) {
            let scored_move = self.forcing_moves.swap_remove(index);
            return Some(scored_move.mov);
        }
        None
    }
}
