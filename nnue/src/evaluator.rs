use std::path::PathBuf;

use candle_nn::{VarBuilder, VarMap};
use chess::Board;
use evaluation::Evaluator;

use crate::{
    encoding::{encode_board, NUM_FEATURES},
    network::Network,
};
use candle_core::{DType, Device, Module, Tensor};

pub struct NNUE {
    network: Network,
    device: Device,
}

impl NNUE {
    pub fn new(path: PathBuf) -> Self {
        let device = Device::Cpu;

        let mut varmap = VarMap::new();
        let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);

        log::info!("Loading NNUE model from {}", path.display());

        let network = Network::new(&vs).unwrap();
        varmap.load(path).expect("Failed to load NNUE model");

        Self { network, device }
    }
}

impl Evaluator for NNUE {
    #[inline]
    fn evaluate(&self, board: &Board) -> f32 {
        let encoded_board =
            Tensor::from_slice(&encode_board(board), (1, NUM_FEATURES), &self.device)
                .expect("Failed to create tensor from encoded board");

        self.network
            .forward(&encoded_board)
            .and_then(|t| t.get(0))
            .and_then(|t| t.get(0))
            .and_then(|t| t.to_scalar::<f32>())
            .expect("Failed to evaluate position")
    }
}
