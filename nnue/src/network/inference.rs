use candle_core::Result;
use utils::bitset::Bitset;

use crate::encoding::NUM_FEATURES;

use super::accumulator::Accumulator;
use super::linear::LinearLayer;
use super::model::Network;
use super::simd::{simd_add, simd_relu};
use super::{
    CP_BOUND, EMBEDDING_SIZE, EVAL_HIDDEN_SIZE, FV_SCALE, OUTPUT_BUCKETS, PIECE_HIDDEN_SIZE,
    PIECE_OUTPUT_SIZE,
};

/// NNUE inference engine with quantized weights.
/// Uses an incremental accumulator for the embedding layer and
/// phase-specific output stacks selected by piece count.
pub struct NNUENetwork {
    accumulator: Accumulator,
    eval_heads: [EvalHead; OUTPUT_BUCKETS],
    piece_head: PieceHead,
    embedding_buffer: [f32; EMBEDDING_SIZE],
}

impl NNUENetwork {
    pub fn from_network(network: &Network) -> Result<Self> {
        let accumulator = Accumulator::new(
            &network.embedding.weight().flatten_all()?.to_vec1()?,
            &network.embedding.bias().unwrap().to_vec1()?,
        );

        let eval_heads: [EvalHead; OUTPUT_BUCKETS] = std::array::from_fn(|i| {
            let head = &network.eval_heads[i];
            EvalHead {
                hidden1: LinearLayer::from_candle_linear(&head.hidden1).unwrap(),
                hidden2: LinearLayer::from_candle_linear(&head.hidden2).unwrap(),
                output: LinearLayer::from_candle_linear(&head.output).unwrap(),
                h1_buffer: [0.0; EVAL_HIDDEN_SIZE],
                h2_buffer: [0.0; EVAL_HIDDEN_SIZE],
                out_buffer: [0.0; 1],
            }
        });

        let piece_head = PieceHead {
            hidden1: LinearLayer::from_candle_linear(&network.piece_head.hidden1).unwrap(),
            hidden2: LinearLayer::from_candle_linear(&network.piece_head.hidden2).unwrap(),
            output: LinearLayer::from_candle_linear(&network.piece_head.output).unwrap(),
            h1_buffer: [0.0; PIECE_HIDDEN_SIZE],
            h2_buffer: [0.0; PIECE_HIDDEN_SIZE],
            out_buffer: [0.0; PIECE_OUTPUT_SIZE],
        };

        Ok(Self {
            accumulator,
            eval_heads,
            piece_head,
            embedding_buffer: [0.0; EMBEDDING_SIZE],
        })
    }

    pub fn reset(&mut self) {
        self.accumulator.reset();
    }

    /// Updates the accumulator from the bitset and runs the eval head.
    /// Use `output_bucket(&board)` to compute the bucket index.
    #[inline]
    pub fn forward(&mut self, bitset: &Bitset<NUM_FEATURES>, bucket: usize) -> f32 {
        self.accumulator.update(bitset);
        self.accumulator
            .dequantize_and_relu(&mut self.embedding_buffer);

        let output = self.eval_heads[bucket].forward(&self.embedding_buffer);

        (output * FV_SCALE).clamp(-CP_BOUND as f32, CP_BOUND as f32)
    }

    /// Runs the piece head on the current embedding, returning piece-type logits.
    /// Call after `forward()` — reuses the embedding buffer populated there.
    #[inline]
    pub fn piece_logits(&mut self) -> &[f32; PIECE_OUTPUT_SIZE] {
        self.piece_head.forward(&self.embedding_buffer);
        &self.piece_head.out_buffer
    }
}

/// Eval head for a single game phase (output bucket).
struct EvalHead {
    hidden1: LinearLayer,
    hidden2: LinearLayer,
    output: LinearLayer,
    h1_buffer: [f32; EVAL_HIDDEN_SIZE],
    h2_buffer: [f32; EVAL_HIDDEN_SIZE],
    out_buffer: [f32; 1],
}

impl EvalHead {
    #[inline]
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

/// Piece head: predicts the best piece type to move from the shared embedding.
struct PieceHead {
    hidden1: LinearLayer,
    hidden2: LinearLayer,
    output: LinearLayer,
    h1_buffer: [f32; PIECE_HIDDEN_SIZE],
    h2_buffer: [f32; PIECE_HIDDEN_SIZE],
    out_buffer: [f32; PIECE_OUTPUT_SIZE],
}

impl PieceHead {
    #[inline]
    fn forward(&mut self, input: &[f32]) {
        self.hidden1.forward(input, &mut self.h1_buffer);
        simd_relu(&mut self.h1_buffer);

        self.hidden2.forward(&self.h1_buffer, &mut self.h2_buffer);
        simd_add(&mut self.h2_buffer, &self.h1_buffer);
        simd_relu(&mut self.h2_buffer);

        self.output.forward(&self.h2_buffer, &mut self.out_buffer);
    }
}
