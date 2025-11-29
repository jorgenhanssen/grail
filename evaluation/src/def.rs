use cozy_chess::Board;
use utils::Position;

/// Hand-Crafted Evaluation interface.
pub trait HCE: Send {
    fn name(&self) -> String;
    /// Evaluate position from White's perspective. Positive = White advantage.
    fn evaluate(&mut self, position: &Position, phase: f32) -> i16;
}

/// Neural Network Evaluation interface.
pub trait NNUE: Send {
    fn name(&self) -> String;
    /// Evaluate position from White's perspective. Positive = White advantage.
    fn evaluate(&mut self, board: &Board) -> i16;
}
