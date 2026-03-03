use candle_nn::{VarBuilder, VarMap};
use cozy_chess::Color;
use utils::Node;

use crate::{
    encoding::encode_board_bitset,
    network::{NNUENetwork, Network, output_bucket},
};
use candle_core::{DType, Device};

/// NNUE evaluator for inference.
///
/// The `network` field exists because candle's VarMap requires creating the network structure
/// first (which registers tensors), then loading weights. After loading, `enable_nnue()` creates
/// the quantized network from the loaded weights.
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

    /// Evaluates the position using the neural network.
    pub fn evaluate(&mut self, node: &Node) -> i16 {
        let board = node.board();
        let white_attacks = node.attacks_for(Color::White);
        let black_attacks = node.attacks_for(Color::Black);
        let white_support = node.support_for(Color::White);
        let black_support = node.support_for(Color::Black);
        let white_threats = node.threats_for(Color::White);
        let black_threats = node.threats_for(Color::Black);

        let bitset = encode_board_bitset(
            board,
            white_attacks,
            black_attacks,
            white_support,
            black_support,
            white_threats,
            black_threats,
        );

        let bucket = output_bucket(board);

        self.nnue
            .as_mut()
            .expect("NNUE network not initialized - call enable_nnue() first")
            .forward(&bitset, bucket)
            .clamp(i16::MIN as f32, i16::MAX as f32) as i16
    }
}
