use chess::{BitBoard, Board, Color};
use std::cell::OnceCell;

use crate::board_metrics::BoardMetrics;

pub struct Position<'a> {
    pub board: &'a Board,
    metrics: OnceCell<BoardMetrics>,
}

impl<'a> Position<'a> {
    #[inline(always)]
    pub fn new(board: &'a Board) -> Self {
        Self {
            board,
            metrics: OnceCell::new(),
        }
    }

    // Get or compute the board metrics (computed once, cached for reuse)
    #[inline(always)]
    fn metrics(&self) -> &BoardMetrics {
        self.metrics.get_or_init(|| BoardMetrics::new(self.board))
    }

    #[inline(always)]
    pub fn space_for(&self, color: Color) -> i16 {
        self.metrics().space[color.to_index()]
    }

    #[inline(always)]
    pub fn attacks_for(&self, color: Color) -> BitBoard {
        self.metrics().attacks[color.to_index()]
    }

    #[inline(always)]
    pub fn threats_for(&self, color: Color) -> BitBoard {
        self.metrics().threats[color.to_index()]
    }

    #[inline(always)]
    pub fn support_for(&self, color: Color) -> BitBoard {
        self.metrics().support[color.to_index()]
    }
}
