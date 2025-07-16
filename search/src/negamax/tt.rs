// For Transposition Table

use chess::ChessMove;

#[derive(Clone, Copy, PartialEq)]
pub enum Bound {
    Exact = 0,
    Lower = 1,
    Upper = 2,
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub plies: u8,
    pub value: i16,
    pub best_move: Option<ChessMove>,
    pub bound: Bound,
}

impl TTEntry {
    #[inline(always)]
    pub fn new(plies: u8, value: i16, bound: Bound, best_move: Option<ChessMove>) -> Self {
        Self {
            plies,
            value,
            best_move,
            bound,
        }
    }
}
