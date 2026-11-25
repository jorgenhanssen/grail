use std::sync::Arc;

use utils::{Node, flip_eval_perspective, refine_static_eval};

use crate::inference::{Accumulator, Network};

#[derive(Clone)]
pub struct Evaluator {
    network: Arc<Network>,
    accumulator: Accumulator,
}

impl Evaluator {
    pub fn from_safetensors(bytes: &[u8]) -> candle_core::Result<Self> {
        let network = Arc::new(Network::from_safetensors(bytes)?);
        let accumulator = network.new_accumulator();

        Ok(Self {
            network,
            accumulator,
        })
    }

    /// Runs the NNUE forward pass and returns the score from white's perspective.
    pub fn evaluate(&mut self, node: &Node) -> i16 {
        let board = node.board();
        let stm = board.side_to_move();
        let metrics = node.metrics();

        let mut score = self.network.forward(board, metrics, &mut self.accumulator);

        score = flip_eval_perspective(stm, score);
        score = refine_static_eval(board, score);

        score
    }
}
