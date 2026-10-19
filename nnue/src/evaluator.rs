use candle_core::{DType, Device};
use candle_nn::VarBuilder;
use cozy_chess::Color;
use utils::{Node, flip_eval_perspective};

use crate::{
    encoding::encode_board_bitset,
    network::{NNUENetwork, Network, output_bucket},
};

/// NNUE evaluator for inference.
#[derive(Clone)]
pub struct Evaluator {
    net: NNUENetwork,
}

impl Evaluator {
    pub fn from_safetensors(bytes: &[u8]) -> candle_core::Result<Self> {
        let vb = VarBuilder::from_slice_safetensors(bytes, DType::F32, &Device::Cpu)?;
        let network = Network::new(&vb)?;

        Ok(Self {
            net: NNUENetwork::from_network(&network)?,
        })
    }

    /// Runs the NNUE forward pass and returns the score from white's perspective.
    pub fn evaluate(&mut self, node: &Node) -> i16 {
        let board = node.board();
        let stm = board.side_to_move();

        let metrics = node.metrics();
        let white_bits = encode_board_bitset(board, metrics, Color::White);
        let black_bits = encode_board_bitset(board, metrics, Color::Black);

        let bucket = output_bucket(board);

        let stm_score = self.net.forward(&white_bits, &black_bits, stm, bucket);

        let stm_score = stm_score.clamp(i16::MIN as f32, i16::MAX as f32) as i16;

        // Return the score as white (as needed by the search)
        flip_eval_perspective(stm, stm_score)
    }
}
