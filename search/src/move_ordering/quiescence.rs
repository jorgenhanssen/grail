// Move ordering for quiescence search inspired by Black Marlin

use arrayvec::ArrayVec;
use cozy_chess::Move;
use utils::piece_value;

use crate::history::CaptureHistory;
use utils::Node;

use super::utils::{ScoredMove, capture_score, select_highest};

pub const MAX_FORCING_MOVES: usize = 32;

pub struct QMoveGenerator {
    forcing_moves: ArrayVec<ScoredMove, MAX_FORCING_MOVES>,
}

impl QMoveGenerator {
    pub fn new(node: &Node, capture_history: &CaptureHistory, best_move: Option<Move>) -> Self {
        if node.in_check() {
            Self::gen_evasions(node, best_move)
        } else {
            Self::gen_captures(node, capture_history, best_move)
        }
    }

    fn gen_captures(
        node: &Node,
        capture_history: &CaptureHistory,
        best_move: Option<Move>,
    ) -> Self {
        let board = node.board();
        let mut forcing_moves = ArrayVec::new();
        let enemy_pieces = board.colors(!board.side_to_move());

        board.generate_moves(|moves| {
            let mut captures = moves;
            captures.to &= enemy_pieces;

            for mov in captures {
                if forcing_moves.len() >= MAX_FORCING_MOVES {
                    return true;
                }

                let score = if Some(mov) == best_move {
                    i16::MAX
                } else {
                    // MVV-LVA + capture history: prefer capturing valuable pieces with cheap ones
                    capture_score(board, mov, capture_history)
                };

                forcing_moves.push(ScoredMove { mov, score });
            }
            false
        });

        Self { forcing_moves }
    }

    fn gen_evasions(node: &Node, best_move: Option<Move>) -> Self {
        let board = node.board();
        let mut forcing_moves = ArrayVec::new();

        board.generate_moves(|moves| {
            for mov in moves {
                if forcing_moves.len() >= MAX_FORCING_MOVES {
                    return true;
                }

                let score = if Some(mov) == best_move {
                    i16::MAX
                } else {
                    // Evasion ordering by negated piece value: king (0) first, then
                    // cheapest pieces. This prioritizes safe king escapes and risks
                    // the least valuable material when blocking or capturing.
                    -piece_value(board.piece_on(mov.from).unwrap())
                };

                forcing_moves.push(ScoredMove { mov, score });
            }
            false
        });

        Self { forcing_moves }
    }

    pub fn next(&mut self) -> Option<Move> {
        let index = select_highest(&self.forcing_moves)?;
        Some(self.forcing_moves.swap_remove(index).mov)
    }
}
