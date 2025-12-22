// Move ordering for quiescence search inspired by Black Marlin

use arrayvec::ArrayVec;
use cozy_chess::{Board, Move};
use utils::piece_value;

use crate::history::CaptureHistory;

use super::utils::{capture_score, select_highest, ScoredMove};

pub const MAX_FORCING_MOVES: usize = 32;

pub struct QMoveGenerator {
    forcing_moves: ArrayVec<ScoredMove, MAX_FORCING_MOVES>,
}

impl QMoveGenerator {
    pub fn new(in_check: bool, board: &Board, capture_history: &CaptureHistory) -> Self {
        if in_check {
            Self::gen_evasions(board)
        } else {
            Self::gen_captures(board, capture_history)
        }
    }

    fn gen_captures(board: &Board, capture_history: &CaptureHistory) -> Self {
        let mut forcing_moves = ArrayVec::new();
        let enemy_pieces = board.colors(!board.side_to_move());

        board.generate_moves(|moves| {
            let mut captures = moves;
            captures.to &= enemy_pieces;

            for mov in captures {
                if forcing_moves.len() >= MAX_FORCING_MOVES {
                    return true;
                }

                // MVV-LVA + capture history: prefer capturing valuable pieces with cheap ones
                let score = capture_score(board, mov, capture_history);

                forcing_moves.push(ScoredMove { mov, score });
            }
            false
        });

        Self { forcing_moves }
    }

    fn gen_evasions(board: &Board) -> Self {
        let mut forcing_moves = ArrayVec::new();

        board.generate_moves(|moves| {
            for mov in moves {
                if forcing_moves.len() >= MAX_FORCING_MOVES {
                    return true;
                }

                // Evasion ordering by negated piece value: king (0) first, then
                // cheapest pieces. This prioritizes safe king escapes and risks
                // the least valuable material when blocking or capturing.
                let moved_piece = board.piece_on(mov.from).unwrap();
                let score: i16 = -piece_value(moved_piece);

                forcing_moves.push(ScoredMove { mov, score });
            }
            false
        });

        Self { forcing_moves }
    }

    pub fn next(&mut self) -> Option<Move> {
        if let Some(index) = select_highest(&self.forcing_moves) {
            let scored_move = self.forcing_moves.swap_remove(index);
            return Some(scored_move.mov);
        }
        None
    }
}
