use candle_core::Result;
use cozy_chess::Color;
use utils::bitset::Bitset;

use crate::encoding::NUM_FEATURES;

use super::accumulator::Accumulator;
use super::linear::LinearLayer;
use super::model::Network;
use super::simd::{simd_add, simd_relu};
use super::{CP_BOUND, EMBEDDING_SIZE, FV_SCALE, HIDDEN_SIZE, OUTPUT_BUCKETS};

/// NNUE inference engine with quantized weights and dual-perspective accumulators.
pub struct NNUENetwork {
    accumulator: Accumulator,
    buckets: [OutputStack; OUTPUT_BUCKETS],
    embedding_buffer: [f32; 2 * EMBEDDING_SIZE],
}

impl NNUENetwork {
    pub fn from_network(network: &Network) -> Result<Self> {
        let accumulator = Accumulator::new(
            &network.embedding.weight().flatten_all()?.to_vec1()?,
            &network.embedding.bias().unwrap().to_vec1()?,
        );

        let buckets: [OutputStack; OUTPUT_BUCKETS] = std::array::from_fn(|i| {
            let bucket = network.buckets.get(i);
            OutputStack {
                hidden1: LinearLayer::from_candle_linear(&bucket.hidden1).unwrap(),
                hidden2: LinearLayer::from_candle_linear(&bucket.hidden2).unwrap(),
                output: LinearLayer::from_candle_linear(&bucket.output).unwrap(),
                h1_buffer: [0.0; HIDDEN_SIZE],
                h2_buffer: [0.0; HIDDEN_SIZE],
                out_buffer: [0.0; 1],
            }
        });

        Ok(Self {
            accumulator,
            buckets,
            embedding_buffer: [0.0; 2 * EMBEDDING_SIZE],
        })
    }

    pub fn reset(&mut self) {
        self.accumulator.reset();
    }

    pub fn forward(
        &mut self,
        white_bits: &Bitset<NUM_FEATURES>,
        black_bits: &Bitset<NUM_FEATURES>,
        stm: Color,
        bucket: usize,
    ) -> f32 {
        self.accumulator.update(Color::White, white_bits);
        self.accumulator.update(Color::Black, black_bits);

        let nstm = !stm;
        let (stm_half, nstm_half) = self.embedding_buffer.split_at_mut(EMBEDDING_SIZE);
        self.accumulator.dequantize_and_relu(stm, stm_half);
        self.accumulator.dequantize_and_relu(nstm, nstm_half);

        let output = self.buckets[bucket].forward(&self.embedding_buffer);

        (output * FV_SCALE).clamp(-CP_BOUND as f32, CP_BOUND as f32)
    }
}

/// Hidden layers and output head for a single game phase.
struct OutputStack {
    hidden1: LinearLayer,
    hidden2: LinearLayer,
    output: LinearLayer,
    h1_buffer: [f32; HIDDEN_SIZE],
    h2_buffer: [f32; HIDDEN_SIZE],
    out_buffer: [f32; 1],
}

impl OutputStack {
    fn forward(&mut self, input: &[f32]) -> f32 {
        self.hidden1.forward(input, &mut self.h1_buffer);
        simd_relu(&mut self.h1_buffer);

        self.hidden2.forward(&self.h1_buffer, &mut self.h2_buffer);
        simd_add(&mut self.h2_buffer, &self.h1_buffer); // residual connection
        simd_relu(&mut self.h2_buffer);

        self.output.forward(&self.h2_buffer, &mut self.out_buffer);

        self.out_buffer[0]
    }
}
