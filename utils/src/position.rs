use cozy_chess::{BitBoard, Board, Color};
use std::cell::OnceCell;

use crate::board_metrics::BoardMetrics;

/// A position wrapper that lazily computes and caches board metrics.
///
/// This avoids recomputing expensive attack/threat bitboards when the same
/// metrics are accessed multiple times during evaluation.
pub struct Position<'a> {
    /// The underlying board state.
    pub board: &'a Board,
    /// Cached board metrics, computed on first access.
    metrics: OnceCell<BoardMetrics>,
}

impl<'a> Position<'a> {
    /// Create a new position wrapper around a board reference.
    pub fn new(board: &'a Board) -> Self {
        Self {
            board,
            metrics: OnceCell::new(),
        }
    }

    /// Get or compute the board metrics (computed once, cached for reuse).
    fn metrics(&self) -> &BoardMetrics {
        self.metrics.get_or_init(|| BoardMetrics::new(self.board))
    }

    /// Get the space score for a color (number of squares attacked).
    pub fn space_for(&self, color: Color) -> i16 {
        self.metrics().space[color as usize]
    }

    /// Get the attack bitboard for a color (all squares attacked).
    pub fn attacks_for(&self, color: Color) -> BitBoard {
        self.metrics().attacks[color as usize]
    }

    /// Get the threats bitboard for a color (opponent's valuable pieces under attack).
    pub fn threats_for(&self, color: Color) -> BitBoard {
        self.metrics().threats[color as usize]
    }

    /// Get the support bitboard for a color (own pieces defended by own pieces).
    pub fn support_for(&self, color: Color) -> BitBoard {
        self.metrics().support[color as usize]
    }
}
