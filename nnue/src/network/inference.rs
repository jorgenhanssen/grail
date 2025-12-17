use candle_core::Result;
use utils::bitset::Bitset;

use crate::encoding::NUM_FEATURES;

use super::accumulator::Accumulator;
use super::linear::LinearLayer;
use super::model::Network;
use super::simd::{simd_add, simd_relu};
use super::{CP_BOUND, EMBEDDING_SIZE, FV_SCALE, HIDDEN_SIZE, OUTPUT_BUCKETS};

/// Hidden layers and output head for a single game phase.
struct OutputStack {
    hidden1: LinearLayer,
    hidden2: LinearLayer,
    output: LinearLayer,
}

/// NNUE inference engine with quantized weights.
/// Uses an incremental accumulator for the embedding layer and
/// phase-specific output stacks selected by piece count.
pub struct NNUENetwork {
    accumulator: Accumulator,
    buckets: [OutputStack; OUTPUT_BUCKETS],

    // Scratch buffers to avoid allocation during forward pass.
    // TODO: Move these into LinearLayer for consistency with Accumulator.
    embedding_buffer: [f32; EMBEDDING_SIZE],
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

        let buckets: [OutputStack; OUTPUT_BUCKETS] = std::array::from_fn(|i| {
            let bucket = &network.buckets[i];
            OutputStack {
                hidden1: LinearLayer::from_candle_linear(&bucket.hidden1).unwrap(),
                hidden2: LinearLayer::from_candle_linear(&bucket.hidden2).unwrap(),
                output: LinearLayer::from_candle_linear(&bucket.output).unwrap(),
            }
        });

        Ok(Self {
            accumulator,
            buckets,
            embedding_buffer: [0.0; EMBEDDING_SIZE],
            hidden1_buffer: [0.0; HIDDEN_SIZE],
            hidden2_buffer: [0.0; HIDDEN_SIZE],
            output_buffer: [0.0; 1],
        })
    }

    pub fn reset(&mut self) {
        self.accumulator.reset();
    }

    /// Forward pass with incremental updates from a bitset.
    /// Use `output_bucket(&board)` to compute the bucket index.
    #[inline]
    pub fn forward(&mut self, bitset: &Bitset<NUM_FEATURES>, bucket: usize) -> f32 {
        self.accumulator.update(bitset);
        self.accumulator
            .dequantize_and_relu(&mut self.embedding_buffer);

        let layers = &self.buckets[bucket];

        layers
            .hidden1
            .forward(&self.embedding_buffer, &mut self.hidden1_buffer);
        simd_relu(&mut self.hidden1_buffer);

        layers
            .hidden2
            .forward(&self.hidden1_buffer, &mut self.hidden2_buffer);
        simd_add(&mut self.hidden2_buffer, &self.hidden1_buffer); // residual connection
        simd_relu(&mut self.hidden2_buffer);

        layers
            .output
            .forward(&self.hidden2_buffer, &mut self.output_buffer);

        // Scale to CP range
        (self.output_buffer[0] * FV_SCALE).clamp(-CP_BOUND as f32, CP_BOUND as f32)
    }
}
