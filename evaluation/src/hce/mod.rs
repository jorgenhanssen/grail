mod eval_bishops;
mod eval_king;
mod eval_knights;
mod eval_material;
mod eval_pawns;
mod eval_queens;
mod eval_rooks;
mod pst;

use crate::def::HCE;
use crate::scores::MATE_VALUE;
use chess::{Board, BoardStatus, Color};

pub struct Evaluator;

impl HCE for Evaluator {
    fn name(&self) -> String {
        "HCE".to_string()
    }

    fn evaluate(&mut self, board: &Board, phase: f32) -> i16 {
        let is_white = board.side_to_move() == Color::White;

        match board.status() {
            BoardStatus::Checkmate => {
                // If it's White to move and board is checkmated => White lost
                if is_white {
                    return -MATE_VALUE;
                } else {
                    return MATE_VALUE;
                }
            }
            BoardStatus::Stalemate => return 0,
            BoardStatus::Ongoing => {}
        }

        let white_mask = board.color_combined(Color::White);
        let black_mask = board.color_combined(Color::Black);

        let mut cp: i16 = 0;

        cp += eval_material::evaluate(board, Color::White, white_mask, phase);
        cp -= eval_material::evaluate(board, Color::Black, black_mask, phase);

        cp += eval_pawns::evaluate(board, Color::White);
        cp -= eval_pawns::evaluate(board, Color::Black);

        cp += eval_rooks::evaluate(board, Color::White, phase);
        cp -= eval_rooks::evaluate(board, Color::Black, phase);

        cp += eval_bishops::evaluate(board, Color::White, phase);
        cp -= eval_bishops::evaluate(board, Color::Black, phase);

        cp += eval_knights::evaluate(board, Color::White, phase);
        cp -= eval_knights::evaluate(board, Color::Black, phase);

        cp += eval_queens::evaluate(board, Color::White, phase);
        cp -= eval_queens::evaluate(board, Color::Black, phase);

        cp += eval_king::evaluate(board, Color::White, phase);
        cp -= eval_king::evaluate(board, Color::Black, phase);

        // Tempo bonus
        if board.side_to_move() == Color::White {
            cp += 10;
        } else {
            cp -= 10;
        }

        cp
    }
}
