use super::context::EvalContext;
use cozy_chess::{BitBoard, Color, Piece};

#[derive(Debug, Clone, Copy)]
pub struct CachedPawnEvaluation {
    pub white: i16,
    pub black: i16,
}

// Caches a single pawn evaluation for a given board.
// Rational is similar as for NNUE - pawn structures change very little between moves, so we can reuse the evaluation.
pub struct PawnCache {
    white_pawns: BitBoard,
    black_pawns: BitBoard,
    evaluation: CachedPawnEvaluation,
}

impl PawnCache {
    pub fn new() -> Self {
        Self {
            white_pawns: BitBoard::EMPTY,
            black_pawns: BitBoard::EMPTY,
            evaluation: CachedPawnEvaluation { white: 0, black: 0 },
        }
    }

    pub fn get(&self, ctx: &EvalContext) -> Option<CachedPawnEvaluation> {
        let board = ctx.position.board;
        let white_pawns = board.colored_pieces(Color::White, Piece::Pawn);
        let black_pawns = board.colored_pieces(Color::Black, Piece::Pawn);

        if white_pawns == self.white_pawns && black_pawns == self.black_pawns {
            Some(self.evaluation)
        } else {
            None
        }
    }

    pub fn set(&mut self, ctx: &EvalContext, cache_entry: CachedPawnEvaluation) {
        let board = ctx.position.board;
        self.white_pawns = board.colored_pieces(Color::White, Piece::Pawn);
        self.black_pawns = board.colored_pieces(Color::Black, Piece::Pawn);
        self.evaluation = cache_entry;
    }
}
