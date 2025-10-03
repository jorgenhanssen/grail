use super::context::EvalContext;
use chess::{BitBoard, Color};

#[derive(Debug, Clone, Copy)]
pub struct CachedPawnEvaluation {
    pub white: i16,
    pub black: i16,
}

/// Incremental pawn evaluation cache.
///
/// Caches pawn evaluation scores based on pawn structure.
/// Since pawn eval only depends on pawn positions, we can reuse
/// results when the pawn structure hasn't changed.
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
        let white_pawns = ctx.pawns_for(Color::White);
        let black_pawns = ctx.pawns_for(Color::Black);

        if white_pawns == self.white_pawns && black_pawns == self.black_pawns {
            Some(self.evaluation)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn set(&mut self, ctx: &EvalContext, cache_entry: CachedPawnEvaluation) {
        self.white_pawns = ctx.pawns_for(Color::White);
        self.black_pawns = ctx.pawns_for(Color::Black);
        self.evaluation = cache_entry;
    }
}
