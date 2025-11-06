use candle_nn::{VarBuilder, VarMap};
use chess::{Board, Color};
use evaluation::NNUE;
use utils::board_metrics::BoardMetrics;

use crate::{
    encoding::encode_board_bitset,
    network::{NNUENetwork, Network},
};
use candle_core::{DType, Device};

pub struct Evaluator {
    nnue_network: NNUENetwork,
    network: Network,
}

impl Evaluator {
    pub fn new(varmap: &VarMap, device: &Device) -> Self {
        let vs = VarBuilder::from_varmap(varmap, DType::F32, device);
        let network = Network::new(&vs).unwrap();
        let nnue_network = NNUENetwork::from_network(&network).unwrap();

        Self {
            nnue_network,
            network,
        }
    }

    pub fn enable_nnue(&mut self) {
        self.nnue_network = NNUENetwork::from_network(&self.network).unwrap();
    }
}

impl NNUE for Evaluator {
    fn name(&self) -> String {
        "NNUE".to_string()
    }

    #[inline(always)]
    fn evaluate(&mut self, board: &Board) -> i16 {
        // Compute tactical features
        let metrics = BoardMetrics::new(board);
        let white_attacks = metrics.attacks[Color::White.to_index()];
        let black_attacks = metrics.attacks[Color::Black.to_index()];
        let white_support = metrics.support[Color::White.to_index()];
        let black_support = metrics.support[Color::Black.to_index()];
        let white_threats = metrics.threats[Color::White.to_index()];
        let black_threats = metrics.threats[Color::Black.to_index()];

        let bitset = encode_board_bitset(
            board,
            white_attacks,
            black_attacks,
            white_support,
            black_support,
            white_threats,
            black_threats,
        );
        self.nnue_network
            .forward_bitset(&bitset)
            .clamp(i16::MIN as f32, i16::MAX as f32) as i16
    }
}
