use candle_core::{Result, Tensor};
use candle_nn::{linear, Linear, Module, VarBuilder};
use std::simd::prelude::SimdFloat;

use crate::encoding::NUM_FEATURES;

use std::simd::f32x8;
const SIMD_WIDTH: usize = 8;

const EMBEDDING_SIZE: usize = 256;
const HIDDEN_SIZE: usize = 32;

pub struct Network {
    embedding: Linear,
    hidden1: Linear,
    hidden2: Linear,
    output: Linear,
}

impl Network {
    pub fn new(vs: &VarBuilder) -> Result<Self> {
        let network = Self {
            embedding: linear(NUM_FEATURES, EMBEDDING_SIZE, vs.pp("embedding"))?,

            hidden1: linear(EMBEDDING_SIZE, HIDDEN_SIZE, vs.pp("hidden1"))?,
            hidden2: linear(HIDDEN_SIZE, HIDDEN_SIZE, vs.pp("hidden2"))?,

            output: linear(HIDDEN_SIZE, 1, vs.pp("output"))?,
        };

        Ok(network)
    }
}

impl Module for Network {
    #[inline]
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        // Embedding
        let mut x = x.apply(&self.embedding)?.relu()?;

        // Hidden layers
        x = x.apply(&self.hidden1)?.relu()?;
        x = x.apply(&self.hidden2)?.relu()?;

        // Output layer
        x = x.apply(&self.output)?;

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
    hidden1: LinearLayer,
    hidden2: LinearLayer,
    output: LinearLayer,

    // Accumulated state
    embedding_buffer: [f32; EMBEDDING_SIZE],
    hidden1_buffer: [f32; HIDDEN_SIZE],
    hidden2_buffer: [f32; HIDDEN_SIZE],
    output_buffer: [f32; 1],

    // Previous input state for change detection
    previous_input: Box<[u64]>,

    // Track if this is the first call
    is_first_eval: bool,
}

impl NNUENetwork {
    pub fn from_network(network: &Network) -> Result<Self> {
        let embedding = LinearLayer::from_candle_linear(&network.embedding)?;
        let hidden1 = LinearLayer::from_candle_linear(&network.hidden1)?;
        let hidden2 = LinearLayer::from_candle_linear(&network.hidden2)?;
        let output = LinearLayer::from_candle_linear(&network.output)?;

        let embedding_buffer = [0.0; EMBEDDING_SIZE];
        let hidden1_buffer = [0.0; HIDDEN_SIZE];
        let hidden2_buffer = [0.0; HIDDEN_SIZE];
        let output_buffer = [0.0; 1];

        // Calculate how many u64s needed to represent all features
        let num_u64s = (NUM_FEATURES + 63) / 64;
        let previous_input = vec![0u64; num_u64s].into_boxed_slice();

        Ok(Self {
            embedding,
            hidden1,
            hidden2,
            output,
            embedding_buffer,
            hidden1_buffer,
            hidden2_buffer,
            output_buffer,
            previous_input,
            is_first_eval: true,
        })
    }

    // Reset the NNUE state (useful when starting a new position evaluation)
    pub fn reset(&mut self) {
        self.embedding_buffer.fill(0.0);
        for bits in self.previous_input.iter_mut() {
            *bits = 0;
        }
        self.is_first_eval = true;
    }

    // Update embedding for a single feature change
    #[inline(always)]
    fn update_embedding_for_feature(&mut self, feature_idx: usize, is_active: bool) {
        if feature_idx >= NUM_FEATURES {
            return;
        }

        let sign = if is_active { 1.0 } else { -1.0 };

        // The embedding weights for this feature affect all embedding neurons
        let weight_offset = feature_idx; // Base index for this feature's weights
        let weight_stride = NUM_FEATURES; // Stride between consecutive output neurons

        for i in 0..EMBEDDING_SIZE {
            self.embedding_buffer[i] +=
                sign * self.embedding.weights[i * weight_stride + weight_offset];
        }
    }

    // Main forward function that handles incremental updates
    pub fn forward(&mut self, input: &[f32]) -> f32 {
        // Convert float input to bitset
        let num_u64s = self.previous_input.len();
        let mut current_input = vec![0u64; num_u64s];

        for (i, &val) in input.iter().enumerate().take(NUM_FEATURES) {
            if val > 0.0 {
                let word_idx = i / 64;
                let bit_idx = i % 64;
                current_input[word_idx] |= 1u64 << bit_idx;
            }
        }

        if self.is_first_eval {
            // First evaluation - initialize the embedding from scratch
            self.embedding_buffer.fill(0.0);

            // Add contributions from all active features
            for word_idx in 0..num_u64s {
                let bits = current_input[word_idx];
                if bits != 0 {
                    for bit_idx in 0..64 {
                        let mask = 1u64 << bit_idx;
                        if bits & mask != 0 {
                            let feature_idx = word_idx * 64 + bit_idx;
                            self.update_embedding_for_feature(feature_idx, true);
                        }
                    }
                }
            }

            self.is_first_eval = false;
        } else {
            // Incremental update - only process features that changed
            for word_idx in 0..num_u64s {
                let changes = self.previous_input[word_idx] ^ current_input[word_idx];

                if changes != 0 {
                    // Process each bit that changed
                    for bit_idx in 0..64 {
                        let mask = 1u64 << bit_idx;
                        if changes & mask != 0 {
                            let feature_idx = word_idx * 64 + bit_idx;
                            // Current state determines if feature is now active
                            let is_active = (current_input[word_idx] & mask) != 0;
                            self.update_embedding_for_feature(feature_idx, is_active);
                        }
                    }
                }
            }
        }

        // Store current input for next time
        self.previous_input.copy_from_slice(&current_input);

        // Create a temporary copy of embedding with biases added
        let mut temp_embedding = [0.0; EMBEDDING_SIZE];
        for i in 0..EMBEDDING_SIZE {
            temp_embedding[i] = self.embedding_buffer[i] + self.embedding.biases[i];
        }

        // Continue with the rest of the network as before
        simd_relu(&mut temp_embedding);

        self.hidden1
            .forward(&temp_embedding, &mut self.hidden1_buffer);
        simd_relu(&mut self.hidden1_buffer);

        self.hidden2
            .forward(&self.hidden1_buffer, &mut self.hidden2_buffer);
        simd_relu(&mut self.hidden2_buffer);

        self.output
            .forward(&self.hidden2_buffer, &mut self.output_buffer);

        self.output_buffer[0]
    }
}

// Helper to apply ReLU activation in-place
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
        values[j] = f32::max(values[j], 0.0);
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
