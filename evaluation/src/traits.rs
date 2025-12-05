// Evaluator traits for position evaluation.
//
// The `evaluation` crate provides shared interfaces and types for evaluation.
// Concrete implementations live in their own crates:
// - `hce` crate: Hand-Crafted Evaluation (implements HCE trait)
// - `nnue` crate: Neural Network Evaluation (implements NNUE trait)
//
// This separation allows `search` to depend only on the interface, while
// the `nnue` crate can depend on `search` for its training tools without
// creating a circular dependency.

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
