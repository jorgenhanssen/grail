mod config;
mod eval_bishops;
mod eval_king;
mod eval_knights;
mod eval_material;
mod eval_pawns;
mod eval_queens;
mod eval_rooks;
mod pst;

pub use config::HCEConfig;

use crate::def::HCE;
use crate::piece_values::PieceValues;
use chess::{Board, Color};

pub struct Evaluator {
    piece_values: PieceValues,
    config: HCEConfig,
}

impl Evaluator {
    pub fn new(piece_values: PieceValues, config: HCEConfig) -> Self {
        Self {
            piece_values,
            config,
        }
    }
}

impl HCE for Evaluator {
    fn name(&self) -> String {
        "HCE".to_string()
    }

    fn evaluate(&mut self, board: &Board, phase: f32) -> i16 {
        let white_mask = board.color_combined(Color::White);
        let black_mask = board.color_combined(Color::Black);

        let mut cp: i16 = 0;

        cp += eval_material::evaluate(board, Color::White, white_mask, phase, &self.piece_values);
        cp -= eval_material::evaluate(board, Color::Black, black_mask, phase, &self.piece_values);

        cp += eval_pawns::evaluate(board, Color::White, &self.config);
        cp -= eval_pawns::evaluate(board, Color::Black, &self.config);

        cp += eval_rooks::evaluate(board, Color::White, phase, &self.config);
        cp -= eval_rooks::evaluate(board, Color::Black, phase, &self.config);

        cp += eval_bishops::evaluate(board, Color::White, phase, &self.config);
        cp -= eval_bishops::evaluate(board, Color::Black, phase, &self.config);

        cp += eval_knights::evaluate(board, Color::White, phase, &self.config);
        cp -= eval_knights::evaluate(board, Color::Black, phase, &self.config);

        cp += eval_queens::evaluate(board, Color::White, phase, &self.config);
        cp -= eval_queens::evaluate(board, Color::Black, phase, &self.config);

        cp += eval_king::evaluate(board, Color::White, phase, &self.config);
        cp -= eval_king::evaluate(board, Color::Black, phase, &self.config);

        // Tempo bonus
        if board.side_to_move() == Color::White {
            cp += self.config.tempo_bonus;
        } else {
            cp -= self.config.tempo_bonus;
        }

        cp
    }
}
