use candle_nn::{VarBuilder, VarMap};
use chess::Board;
use evaluation::Evaluator;

use crate::{
    encoding::{encode_board, NUM_FEATURES},
    network::{NNUENetwork, Network},
};
use candle_core::{DType, Device, Module, Tensor};

pub struct NNUE {
    nnue_network: NNUENetwork,
    network: Network,
    device: Device,
    version: u32,
}

impl NNUE {
    pub fn new(varmap: &VarMap, device: &Device, version: u32) -> Self {
        let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        let network = Network::new(&vs).unwrap();
        let nnue_network = NNUENetwork::from_network(&network).unwrap();

        Self {
            nnue_network,
            network,
            device: device.clone(),
            version,
        }
    }

    pub fn enable_nnue(&mut self) {
        self.nnue_network = NNUENetwork::from_network(&self.network).unwrap();
    }
}

impl Evaluator for NNUE {
    fn name(&self) -> String {
        format!("NNUE-{}", self.version)
    }

    #[inline(always)]
    fn evaluate(&mut self, board: &Board) -> f32 {
        let encoded_board = encode_board(board);
        self.nnue_network.forward(&encoded_board)
    }
}
