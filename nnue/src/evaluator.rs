use candle_nn::{VarBuilder, VarMap};
use cozy_chess::{Board, Color};
use evaluation::NNUE;
use utils::board_metrics::BoardMetrics;

use crate::{
    encoding::encode_board_bitset,
    network::{NNUENetwork, Network},
};
use candle_core::{DType, Device};

pub struct Evaluator {
    nnue_network: Option<NNUENetwork>,
    network: Network,
}

impl Evaluator {
    pub fn new(varmap: &VarMap, device: &Device) -> Self {
        let vs = VarBuilder::from_varmap(varmap, DType::F32, device);
        let network = Network::new(&vs).unwrap();

        Self {
            nnue_network: None,
            network,
        }
    }

    pub fn enable_nnue(&mut self) {
        self.nnue_network = Some(NNUENetwork::from_network(&self.network).unwrap());
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
        let white_attacks = metrics.attacks[Color::White as usize];
        let black_attacks = metrics.attacks[Color::Black as usize];
        let white_support = metrics.support[Color::White as usize];
        let black_support = metrics.support[Color::Black as usize];
        let white_threats = metrics.threats[Color::White as usize];
        let black_threats = metrics.threats[Color::Black as usize];

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
            .as_mut()
            .expect("NNUE network not initialized - call enable_nnue() first")
            .forward_bitset(&bitset)
            .clamp(i16::MIN as f32, i16::MAX as f32) as i16
    }
}
