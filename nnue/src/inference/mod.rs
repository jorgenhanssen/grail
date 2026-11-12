mod accumulator;
mod linear;
mod simd;

use candle_core::{DType, Device, Result};
use candle_nn::VarBuilder;
use cozy_chess::Board;
use utils::board_metrics::BoardMetrics;

use crate::network::{EVAL_OUTPUT_BOUND, FV_SCALE, HIDDEN_SIZE, model, output_bucket};

pub use accumulator::Accumulator;
use accumulator::AccumulatorWeights;
use linear::LinearLayer;
use simd::{simd_add, simd_relu};

pub struct Network {
    accumulator_weights: AccumulatorWeights,
    buckets: Box<[OutputStack]>,
}

impl Network {
    pub fn from_safetensors(bytes: &[u8]) -> Result<Self> {
        let vb = VarBuilder::from_slice_safetensors(bytes, DType::F32, &Device::Cpu)?;
        let model = model::Network::new(&vb)?;
        let buckets = model
            .buckets
            .iter()
            .map(OutputStack::from_model)
            .collect::<Result<Vec<_>>>()?
            .into_boxed_slice();

        Ok(Self {
            accumulator_weights: AccumulatorWeights::from_model(&model.embedding)?,
            buckets,
        })
    }

    pub fn new_accumulator(&self) -> Accumulator {
        Accumulator::new(&self.accumulator_weights)
    }

    /// Returns the score from the side to move's perspective.
    pub fn forward(
        &self,
        board: &Board,
        metrics: &BoardMetrics,
        accumulator: &mut Accumulator,
    ) -> i16 {
        let input = accumulator.update(&self.accumulator_weights, board, metrics);
        let output = self.buckets[output_bucket(board)].forward(input);
        (output * FV_SCALE).clamp(-EVAL_OUTPUT_BOUND as f32, EVAL_OUTPUT_BOUND as f32) as i16
    }
}

/// Hidden layers and output head for a single game phase.
struct OutputStack {
    hidden1: LinearLayer,
    hidden2: LinearLayer,
    output: LinearLayer,
}

impl OutputStack {
    fn from_model(stack: &model::OutputStack) -> Result<Self> {
        Ok(Self {
            hidden1: LinearLayer::from_candle_linear(&stack.hidden1)?,
            hidden2: LinearLayer::from_candle_linear(&stack.hidden2)?,
            output: LinearLayer::from_candle_linear(&stack.output)?,
        })
    }

    fn forward(&self, input: &[f32]) -> f32 {
        let mut hidden1 = [0.0; HIDDEN_SIZE];
        let mut hidden2 = [0.0; HIDDEN_SIZE];
        let mut output = [0.0];

        self.hidden1.forward(input, &mut hidden1);
        simd_relu(&mut hidden1);

        self.hidden2.forward(&hidden1, &mut hidden2);
        simd_add(&mut hidden2, &hidden1); // residual connection
        simd_relu(&mut hidden2);

        self.output.forward(&hidden2, &mut output);
        output[0]
    }
}
