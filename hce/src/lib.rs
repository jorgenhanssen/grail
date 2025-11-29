mod config;
mod context;
mod eval_bishops;
mod eval_king;
mod eval_material;
mod eval_pawns;
mod eval_rooks;
mod eval_space;
mod eval_threats;
mod pawn_cache;
mod pst;

pub use config::HCEConfig;
use context::EvalContext;
use pawn_cache::PawnCache;

use crate::pawn_cache::CachedPawnEvaluation;
use cozy_chess::Color;
use evaluation::{PieceValues, HCE};
use utils::{side_has_insufficient_material, Position};

/// Hand-Crafted Evaluation: tunable metrics based on human knowledge and chess concepts.
///
/// <https://www.chessprogramming.org/Evaluation>
pub struct Evaluator {
    piece_values: PieceValues,
    config: HCEConfig,
    pawn_cache: PawnCache,
}

impl Evaluator {
    pub fn new(piece_values: PieceValues, config: HCEConfig) -> Self {
        Self {
            piece_values,
            config,
            pawn_cache: PawnCache::new(),
        }
    }
}

impl HCE for Evaluator {
    fn name(&self) -> String {
        "HCE".to_string()
    }

    /// Evaluates from White's perspective. Positive = White advantage.
    fn evaluate(&mut self, position: &Position, phase: f32) -> i16 {
        let ctx = EvalContext::new(position, phase);
        let board = position.board;

        let mut cp: i16 = 0;

        cp += eval_material::evaluate(&ctx, Color::White, &self.piece_values);
        cp -= eval_material::evaluate(&ctx, Color::Black, &self.piece_values);

        if let Some(scores) = self.pawn_cache.get(&ctx) {
            cp += scores.white;
            cp -= scores.black;
        } else {
            let white_score = eval_pawns::evaluate(&ctx, Color::White, &self.config);
            let black_score = eval_pawns::evaluate(&ctx, Color::Black, &self.config);

            cp += white_score;
            cp -= black_score;

            let cache_entry = CachedPawnEvaluation {
                white: white_score,
                black: black_score,
            };
            self.pawn_cache.set(&ctx, cache_entry);
        };

        cp += eval_rooks::evaluate(&ctx, Color::White, &self.config);
        cp -= eval_rooks::evaluate(&ctx, Color::Black, &self.config);

        cp += eval_bishops::evaluate(&ctx, Color::White, &self.config);
        cp -= eval_bishops::evaluate(&ctx, Color::Black, &self.config);

        cp += eval_king::evaluate(&ctx, Color::White, &self.config);
        cp -= eval_king::evaluate(&ctx, Color::Black, &self.config);

        cp += eval_space::evaluate(&ctx, Color::White, &self.config);
        cp -= eval_space::evaluate(&ctx, Color::Black, &self.config);

        cp += eval_space::evaluate_support(&ctx, Color::White, &self.config);
        cp -= eval_space::evaluate_support(&ctx, Color::Black, &self.config);

        cp += eval_threats::evaluate(&ctx, Color::White, &self.config);
        cp -= eval_threats::evaluate(&ctx, Color::Black, &self.config);

        // Tempo bonus
        if board.side_to_move() == Color::White {
            cp += self.config.tempo_bonus;
        } else {
            cp -= self.config.tempo_bonus;
        }

        // Cap evaluation based on insufficient material
        // If a side cannot possibly win, cap eval at draw (0) from their perspective
        // Because a bishop + king gets higher cp than vs a king, even if it is a draw.
        if side_has_insufficient_material(board, Color::White) {
            cp = cp.min(0);
        }
        if side_has_insufficient_material(board, Color::Black) {
            cp = cp.max(0);
        }

        cp
    }
}
