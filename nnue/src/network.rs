use candle_core::{Result, Tensor};
use candle_nn::{linear, Linear, Module, VarBuilder};
use std::simd::prelude::SimdFloat;

use crate::encoding::{NUM_FEATURES, NUM_U64S};
use crate::samples::{CP_MAX, CP_MIN, TRAINING_SCALE};

use std::simd::f32x8;
const SIMD_WIDTH: usize = 8;

const EMBEDDING_SIZE: usize = 1024;
const HIDDEN_SIZE: usize = 16;

pub struct Network {
    embedding: Linear,
    hidden: Linear,
    output: Linear,
}

impl Network {
    pub fn new(vs: &VarBuilder) -> Result<Self> {
        let network = Self {
            embedding: linear(NUM_FEATURES, EMBEDDING_SIZE, vs.pp("embedding"))?,
            hidden: linear(EMBEDDING_SIZE, HIDDEN_SIZE, vs.pp("hidden"))?,
            output: linear(HIDDEN_SIZE, 1, vs.pp("output"))?,
        };

        Ok(network)
    }
}

impl Module for Network {
    #[inline]
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let x = x.apply(&self.embedding)?.relu()?;
        let x = x.apply(&self.hidden)?.relu()?;
        let x = x.apply(&self.output)?;

        Ok(x)
    }
}

pub struct LinearLayer {
    weights: Box<[f32]>,
    biases: Box<[f32]>,
    input_size: usize,
    output_size: usize,
}

impl LinearLayer {
    // Create from Candle Linear layer
    fn from_candle_linear(linear: &Linear) -> Result<Self> {
        let weights = linear.weight().flatten_all()?.to_vec1()?.into_boxed_slice();
        let biases = linear.bias().unwrap().to_vec1()?.into_boxed_slice();

        let input_size = linear.weight().dim(1)?;
        let output_size = linear.weight().dim(0)?;

        Ok(Self {
            weights,
            biases,
            input_size,
            output_size,
        })
    }

    // Forward pass
    #[inline(always)]
    fn forward(&self, input: &[f32], output: &mut [f32]) {
        // Initialize with biases
        output.copy_from_slice(&self.biases);

        for i in 0..self.output_size {
            let weights_row_offset = i * self.input_size;
            let weights_row =
                &self.weights[weights_row_offset..weights_row_offset + self.input_size];

            output[i] += simd_dot(input, weights_row, self.input_size);
        }
    }
}

pub struct NNUENetwork {
    embedding: LinearLayer,
    hidden: LinearLayer,
    output: LinearLayer,

    // [feature_idx][embedding_idx]
    embedding_weights_by_feature: Box<[f32]>,

    // Accumulated state
    embedding_buffer: [f32; EMBEDDING_SIZE],
    hidden_buffer: [f32; HIDDEN_SIZE],
    output_buffer: [f32; 1],

    // Input state for change detection
    current_input: [u64; NUM_U64S],
    previous_input: [u64; NUM_U64S],
}

impl NNUENetwork {
    pub fn from_network(network: &Network) -> Result<Self> {
        let embedding = LinearLayer::from_candle_linear(&network.embedding)?;
        let hidden = LinearLayer::from_candle_linear(&network.hidden)?;
        let output = LinearLayer::from_candle_linear(&network.output)?;

        // Transposed embedding weights for cache-friendly updates.
        // [in=NUM_FEATURES][out=EMBEDDING_SIZE].
        let mut embedding_weights_by_feature =
            vec![0.0f32; NUM_FEATURES * EMBEDDING_SIZE].into_boxed_slice();
        for out_idx in 0..EMBEDDING_SIZE {
            let src_row_offset = out_idx * NUM_FEATURES;
            for feature_idx in 0..NUM_FEATURES {
                let src = embedding.weights[src_row_offset + feature_idx];
                embedding_weights_by_feature[feature_idx * EMBEDDING_SIZE + out_idx] = src;
            }
        }

        let mut embedding_buffer = [0.0; EMBEDDING_SIZE];
        embedding_buffer.copy_from_slice(&embedding.biases);

        let hidden_buffer = [0.0; HIDDEN_SIZE];
        let output_buffer = [0.0; 1];

        let current_input = [0u64; NUM_U64S];
        let previous_input = [0u64; NUM_U64S];

        Ok(Self {
            embedding,
            hidden,
            output,
            embedding_weights_by_feature,
            embedding_buffer,
            hidden_buffer,
            output_buffer,
            current_input,
            previous_input,
        })
    }

    // Reset the NNUE state (useful when starting a new position evaluation)
    #[inline(always)]
    pub fn reset(&mut self) {
        self.embedding_buffer
            .copy_from_slice(&self.embedding.biases);
        self.previous_input.fill(0);
        self.current_input.fill(0);
    }

    // Update embedding for a single feature change
    #[inline(always)]
    fn update_embedding_for_feature(&mut self, feature_idx: usize, is_active: bool) {
        let sign = if is_active { 1.0 } else { -1.0 };
        let sign_vec = f32x8::splat(sign);

        let mut i = 0;
        let weights_row = &self.embedding_weights_by_feature
            [feature_idx * EMBEDDING_SIZE..feature_idx * EMBEDDING_SIZE + EMBEDDING_SIZE];

        while i + SIMD_WIDTH <= EMBEDDING_SIZE {
            // Load current embedding values
            let mut embedding_chunk = f32x8::from_slice(&self.embedding_buffer[i..i + SIMD_WIDTH]);

            let weights_chunk = f32x8::from_slice(&weights_row[i..i + SIMD_WIDTH]);

            // Multiply weights by sign and add to embedding
            embedding_chunk += sign_vec * weights_chunk;

            embedding_chunk.copy_to_slice(&mut self.embedding_buffer[i..i + SIMD_WIDTH]);

            i += SIMD_WIDTH;
        }

        // Handle remaining elements
        while i < EMBEDDING_SIZE {
            self.embedding_buffer[i] += sign * weights_row[i];
            i += 1;
        }
    }

    // Main forward function that handles incremental updates
    #[inline(always)]
    pub fn forward(&mut self, input: &[f32]) -> f32 {
        // Convert float input to bitset, using the internal buffer
        self.current_input.fill(0); // Clear the buffer

        for (i, &val) in input.iter().enumerate().take(NUM_FEATURES) {
            if val > 0.0 {
                let word_idx = i / 64;
                let bit_idx = i % 64;
                self.current_input[word_idx] |= 1u64 << bit_idx;
            }
        }

        // Always do incremental updates by comparing with previous_input
        for word_idx in 0..NUM_U64S {
            // XOR to find bits that differ
            let mut changes = self.previous_input[word_idx] ^ self.current_input[word_idx];
            while changes != 0 {
                let bit_idx = changes.trailing_zeros() as usize;
                changes &= changes - 1;

                let feature_idx = word_idx * 64 + bit_idx;
                // Guard against stray bits beyond NUM_FEATURES (last partial u64).
                if feature_idx >= NUM_FEATURES {
                    continue;
                }
                // Check if it's now active or inactive
                let mask = 1u64 << bit_idx;
                let is_active = (self.current_input[word_idx] & mask) != 0;
                self.update_embedding_for_feature(feature_idx, is_active);
            }
        }

        // Store current input for next time
        self.previous_input.copy_from_slice(&self.current_input);

        // Create an activated view of the embedding (ReLU), leaving the
        // pre-activation buffer intact for incremental updates.
        let mut activated_embedding = [0.0; EMBEDDING_SIZE];
        activated_embedding.copy_from_slice(&self.embedding_buffer);
        simd_relu(&mut activated_embedding);

        self.output
            .forward(&activated_embedding, &mut self.output_buffer);

        let cp = self.output_buffer[0] * TRAINING_SCALE;
        cp.clamp(CP_MIN as f32, CP_MAX as f32)
    }

    // Forward pass with a pre-encoded bitset input for incremental updates.
    #[inline(always)]
    pub fn forward_bitset(&mut self, bitset: &[u64; NUM_U64S]) -> f32 {
        // Copy bitset into current_input buffer
        self.current_input.copy_from_slice(bitset);

        // Apply incremental updates by comparing with previous_input
        for word_idx in 0..NUM_U64S {
            let mut changes = self.previous_input[word_idx] ^ self.current_input[word_idx];
            while changes != 0 {
                let bit_idx = changes.trailing_zeros() as usize;
                changes &= changes - 1;

                let feature_idx = word_idx * 64 + bit_idx;
                if feature_idx >= NUM_FEATURES {
                    continue;
                }
                let mask = 1u64 << bit_idx;
                let is_active = (self.current_input[word_idx] & mask) != 0;
                self.update_embedding_for_feature(feature_idx, is_active);
            }
        }

        // Store current input for next time
        self.previous_input.copy_from_slice(&self.current_input);

        // Apply ReLU to embedding
        let mut activated_embedding = [0.0; EMBEDDING_SIZE];
        activated_embedding.copy_from_slice(&self.embedding_buffer);
        simd_relu(&mut activated_embedding);

        // Pass through hidden layer
        self.hidden
            .forward(&activated_embedding, &mut self.hidden_buffer);

        // Apply ReLU to hidden
        simd_relu(&mut self.hidden_buffer);

        // Pass through output layer
        self.output
            .forward(&self.hidden_buffer, &mut self.output_buffer);

        let cp = self.output_buffer[0] * TRAINING_SCALE;
        cp.clamp(CP_MIN as f32, CP_MAX as f32)
    }
}

#[inline(always)]
fn simd_relu(values: &mut [f32]) {
    let len = values.len();
    let mut i = 0;

    const UNROLL: usize = 4;

    let zeros = f32x8::splat(0.0);
    let limit = len - (len % (SIMD_WIDTH * UNROLL));

    // 4 SIMD chunks
    while i < limit {
        let chunk0 = f32x8::from_slice(&values[i..i + SIMD_WIDTH]);
        let chunk1 = f32x8::from_slice(&values[i + SIMD_WIDTH..i + SIMD_WIDTH * 2]);
        let chunk2 = f32x8::from_slice(&values[i + SIMD_WIDTH * 2..i + SIMD_WIDTH * 3]);
        let chunk3 = f32x8::from_slice(&values[i + SIMD_WIDTH * 3..i + SIMD_WIDTH * 4]);

        let result0 = chunk0.simd_max(zeros);
        let result1 = chunk1.simd_max(zeros);
        let result2 = chunk2.simd_max(zeros);
        let result3 = chunk3.simd_max(zeros);

        result0.copy_to_slice(&mut values[i..i + SIMD_WIDTH]);
        result1.copy_to_slice(&mut values[i + SIMD_WIDTH..i + SIMD_WIDTH * 2]);
        result2.copy_to_slice(&mut values[i + SIMD_WIDTH * 2..i + SIMD_WIDTH * 3]);
        result3.copy_to_slice(&mut values[i + SIMD_WIDTH * 3..i + SIMD_WIDTH * 4]);

        i += SIMD_WIDTH * UNROLL;
    }

    // Handle remaining elements
    for j in i..len {
        values[j] = values[j].max(0.0);
    }
}

// SIMD dot product helper function
#[inline(always)]
fn simd_dot(a: &[f32], b: &[f32], len: usize) -> f32 {
    let mut sum_vec0 = f32x8::splat(0.0);
    let mut sum_vec1 = f32x8::splat(0.0);
    let mut sum_vec2 = f32x8::splat(0.0);
    let mut sum_vec3 = f32x8::splat(0.0);

    const UNROLL: usize = 4;

    let limit = len - (len % (SIMD_WIDTH * UNROLL));
    let mut i = 0;

    // Process 4 SIMD vectors at once (unrolled loop)
    while i < limit {
        // Correct slice ranges for from_slice
        let a0 = f32x8::from_slice(&a[i..i + SIMD_WIDTH]);
        let b0 = f32x8::from_slice(&b[i..i + SIMD_WIDTH]);
        let a1 = f32x8::from_slice(&a[i + SIMD_WIDTH..i + SIMD_WIDTH * 2]);
        let b1 = f32x8::from_slice(&b[i + SIMD_WIDTH..i + SIMD_WIDTH * 2]);
        let a2 = f32x8::from_slice(&a[i + SIMD_WIDTH * 2..i + SIMD_WIDTH * 3]);
        let b2 = f32x8::from_slice(&b[i + SIMD_WIDTH * 2..i + SIMD_WIDTH * 3]);
        let a3 = f32x8::from_slice(&a[i + SIMD_WIDTH * 3..i + SIMD_WIDTH * 4]);
        let b3 = f32x8::from_slice(&b[i + SIMD_WIDTH * 3..i + SIMD_WIDTH * 4]);

        // Multiply and accumulate
        sum_vec0 += a0 * b0;
        sum_vec1 += a1 * b1;
        sum_vec2 += a2 * b2;
        sum_vec3 += a3 * b3;

        i += SIMD_WIDTH * UNROLL;
    }

    let sum_vec = sum_vec0 + sum_vec1 + sum_vec2 + sum_vec3;
    let sum = sum_vec.as_array().iter().sum::<f32>();

    // Handle remaining
    let mut scalar_sum = 0.0;
    while i < len {
        scalar_sum += a[i] * b[i];
        i += 1;
    }

    sum + scalar_sum
}
