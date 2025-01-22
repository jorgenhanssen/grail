// For Transposition Table

use chess::ChessMove;

#[derive(Clone, Copy)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub plies: u32,
    pub value: f32,
    pub bound: Bound,
    pub best_move: Option<ChessMove>,
}
