use candle_nn::{VarBuilder, VarMap};
use cozy_chess::Color;
use utils::{Node, flip_eval_perspective};

use crate::{
    encoding::encode_board_bitset,
    network::{NNUENetwork, Network, output_bucket},
};
use candle_core::{DType, Device};

/// NNUE evaluator for inference. The full-precision network field is kept
/// around because candle's VarMap wants to register tensors before weights are
/// loaded; enable_nnue then quantizes it into the nnue field for actual eval.
pub struct Evaluator {
    /// Quantized network for fast inference
    nnue: Option<NNUENetwork>,

    /// Full-precision network used to load weights before quantization
    network: Network,
}

impl Evaluator {
    pub fn new(varmap: &VarMap, device: &Device) -> Self {
        let vs = VarBuilder::from_varmap(varmap, DType::F32, device);
        let network = Network::new(&vs).unwrap();

        Self {
            nnue: None,
            network,
        }
    }

    pub fn enable_nnue(&mut self) {
        self.nnue = Some(NNUENetwork::from_network(&self.network).unwrap());
    }

    /// Runs the NNUE forward pass and returns the score from white's perspective.
    pub fn evaluate(&mut self, node: &Node) -> i16 {
        let board = node.board();
        let stm = board.side_to_move();

        let white_support = node.support_for(Color::White);
        let black_support = node.support_for(Color::Black);
        let white_threats = node.threats_for(Color::White);
        let black_threats = node.threats_for(Color::Black);

        let white_bits = encode_board_bitset(
            board,
            white_support,
            black_support,
            white_threats,
            black_threats,
            Color::White,
        );
        let black_bits = encode_board_bitset(
            board,
            white_support,
            black_support,
            white_threats,
            black_threats,
            Color::Black,
        );

        let bucket = output_bucket(board);

        let stm_score = self
            .nnue
            .as_mut()
            .expect("NNUE network not initialized - call enable_nnue() first")
            .forward(&white_bits, &black_bits, stm, bucket);

        let stm_score = stm_score.clamp(i16::MIN as f32, i16::MAX as f32) as i16;

        // Return the score as white (as needed by the search)
        flip_eval_perspective(stm, stm_score)
    }
}
