use candle_core::Result;
use utils::bits::set_bit;

use crate::encoding::{NUM_FEATURES, NUM_U64S};

use super::accumulator::Accumulator;
use super::linear::LinearLayer;
use super::model::Network;
use super::simd::{simd_add, simd_relu};
use super::{CP_MAX, CP_MIN, EMBEDDING_SIZE, FV_SCALE, HIDDEN_SIZE};

/// Main NNUE inference engine.
pub struct NNUENetwork {
    accumulator: Accumulator,
    hidden1: LinearLayer,
    hidden2: LinearLayer,
    output: LinearLayer,

    // Scratch buffers to avoid allocation during forward pass
    hidden1_buffer: [f32; HIDDEN_SIZE],
    hidden2_buffer: [f32; HIDDEN_SIZE],
    output_buffer: [f32; 1],
}

impl NNUENetwork {
    pub fn from_network(network: &Network) -> Result<Self> {
        let accumulator = Accumulator::new(
            &network.embedding.weight().flatten_all()?.to_vec1()?,
            &network.embedding.bias().unwrap().to_vec1()?,
        );

        Ok(Self {
            accumulator,
            hidden1: LinearLayer::from_candle_linear(&network.hidden1)?,
            hidden2: LinearLayer::from_candle_linear(&network.hidden2)?,
            output: LinearLayer::from_candle_linear(&network.output)?,
            hidden1_buffer: [0.0; HIDDEN_SIZE],
            hidden2_buffer: [0.0; HIDDEN_SIZE],
            output_buffer: [0.0; 1],
        })
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.accumulator.reset();
    }

    #[inline(always)]
    pub fn forward(&mut self, input: &[f32]) -> f32 {
        let mut current_input = [0u64; NUM_U64S];

        for (i, &val) in input.iter().enumerate().take(NUM_FEATURES) {
            if val > 0.0 {
                set_bit(&mut current_input, i);
            }
        }

        self.forward_bitset(&current_input)
    }

    // Forward pass with incremental updates from a bitset.
    #[inline(always)]
    pub fn forward_bitset(&mut self, bitset: &[u64; NUM_U64S]) -> f32 {
        self.accumulator.update(bitset);

        let mut embedding_output = [0.0; EMBEDDING_SIZE];
        self.accumulator.dequantize_and_relu(&mut embedding_output);

        self.hidden1
            .forward(&embedding_output, &mut self.hidden1_buffer);
        simd_relu(&mut self.hidden1_buffer);

        self.hidden2
            .forward(&self.hidden1_buffer, &mut self.hidden2_buffer);
        simd_add(&mut self.hidden2_buffer, &self.hidden1_buffer); // residual connection
        simd_relu(&mut self.hidden2_buffer);

        self.output
            .forward(&self.hidden2_buffer, &mut self.output_buffer);

        // Scale to CP range
        (self.output_buffer[0] * FV_SCALE).clamp(CP_MIN as f32, CP_MAX as f32)
    }
}
