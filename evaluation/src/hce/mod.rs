mod config;
mod context;
mod eval_bishops;
mod eval_king;
mod eval_knights;
mod eval_material;
mod eval_pawns;
mod eval_queens;
mod eval_rooks;
mod eval_space;
mod pawn_cache;
mod pst;

pub use config::HCEConfig;
use context::EvalContext;
use pawn_cache::PawnCache;

use crate::def::HCE;
use crate::hce::pawn_cache::CachedPawnEvaluation;
use crate::piece_values::PieceValues;
use chess::Color;
use utils::Position;

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

        cp += eval_knights::evaluate(&ctx, Color::White, &self.config);
        cp -= eval_knights::evaluate(&ctx, Color::Black, &self.config);

        cp += eval_queens::evaluate(&ctx, Color::White, &self.config);
        cp -= eval_queens::evaluate(&ctx, Color::Black, &self.config);

        cp += eval_king::evaluate(&ctx, Color::White, &self.config);
        cp -= eval_king::evaluate(&ctx, Color::Black, &self.config);

        // Space advantage (general mobility)
        cp += eval_space::evaluate(&ctx, Color::White, &self.config);
        cp -= eval_space::evaluate(&ctx, Color::Black, &self.config);

        // Tempo bonus
        if board.side_to_move() == Color::White {
            cp += self.config.tempo_bonus;
        } else {
            cp -= self.config.tempo_bonus;
        }

        cp
    }
}
