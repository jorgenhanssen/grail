use super::context::EvalContext;
use chess::{BitBoard, Color, Piece};

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
            white_pawns: BitBoard(0),
            black_pawns: BitBoard(0),
            evaluation: CachedPawnEvaluation { white: 0, black: 0 },
        }
    }

    #[inline(always)]
    pub fn get(&self, ctx: &EvalContext) -> Option<CachedPawnEvaluation> {
        let board = ctx.position.board;
        let pawns = board.pieces(Piece::Pawn);
        let white_pawns = pawns & board.color_combined(Color::White);
        let black_pawns = pawns & board.color_combined(Color::Black);

        if white_pawns == self.white_pawns && black_pawns == self.black_pawns {
            Some(self.evaluation)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn set(&mut self, ctx: &EvalContext, cache_entry: CachedPawnEvaluation) {
        let board = ctx.position.board;
        let pawns = board.pieces(Piece::Pawn);
        self.white_pawns = pawns & board.color_combined(Color::White);
        self.black_pawns = pawns & board.color_combined(Color::Black);
        self.evaluation = cache_entry;
    }
}
