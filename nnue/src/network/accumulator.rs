use std::simd::i8x32;
use std::simd::num::SimdInt;
use std::simd::prelude::SimdFloat;

use utils::bitset::Bitset;

use crate::encoding::NUM_FEATURES;

use super::simd::{SimdF32, SimdI16, SIMD_WIDTH_F32, SIMD_WIDTH_I16};
use super::{EMBEDDING_SIZE, QUANTIZATION_PERCENTILE};

/// The Accumulator manages the stateful first (embedding) layer of the NNUE.
///
/// Instead of recomputing the full embedding from scratch on each move,
/// we track which input features changed and incrementally add/subtract
/// the corresponding weight rows. This makes inference O(changed features)
/// rather than O(all features).
///
/// Weights are quantized to i8 and accumulated in i16 for speed (SIMD-friendly).
/// Dequantization back to f32 happens only when outputting to the next layer.
pub struct Accumulator {
    // [feature_idx][embedding_idx]
    weights: Box<[i8]>,
    // [embedding_idx]
    biases: Box<[i16]>,
    // Accumulated sum of active weights [embedding_idx]
    buffer: [i16; EMBEDDING_SIZE],

    // To know which inputs have changed since the last update
    previous_input: Bitset<NUM_FEATURES>,

    // Scale factor to dequantize back to f32
    scale: f32,
}

impl Accumulator {
    pub fn new(weights: &[f32], biases: &[f32]) -> Self {
        let scale = compute_quantization_scale(weights);
        let weights_i8 = quantize_embedding_weights(weights, scale);
        let biases_i16 = quantize_embedding_biases(biases, scale);

        let mut buffer = [0i16; EMBEDDING_SIZE];
        buffer.copy_from_slice(&biases_i16);

        Self {
            weights: weights_i8,
            biases: biases_i16,
            buffer,
            previous_input: Bitset::default(),
            scale,
        }
    }

    pub fn reset(&mut self) {
        self.buffer.copy_from_slice(&self.biases);
        self.previous_input = Bitset::default();
    }

    /// Updates the accumulator based on the difference between previous and current inputs.
    pub fn update(&mut self, new_input: &Bitset<NUM_FEATURES>) {
        // TODO: Look into if we can avoid cloning this.
        self.previous_input.clone().for_each_diff(new_input, |idx| {
            let is_active = new_input.get(idx);
            self.apply_feature_change(idx, is_active);
        });

        self.previous_input = *new_input;
    }

    fn apply_feature_change(&mut self, feature_idx: usize, add: bool) {
        let offset: usize = feature_idx * EMBEDDING_SIZE;
        let weights_row = &self.weights[offset..offset + EMBEDDING_SIZE];

        let mut i = 0;

        while i + SIMD_WIDTH_I16 <= EMBEDDING_SIZE {
            // Load current buffer values
            let mut buffer_vec = SimdI16::from_slice(&self.buffer[i..i + SIMD_WIDTH_I16]);

            // Load and widen weights (i8 -> i16)
            let weights_i8 = i8x32::from_slice(&weights_row[i..i + SIMD_WIDTH_I16]);
            let weights_i16: SimdI16 = weights_i8.cast();

            if add {
                buffer_vec += weights_i16;
            } else {
                buffer_vec -= weights_i16;
            }

            buffer_vec.copy_to_slice(&mut self.buffer[i..i + SIMD_WIDTH_I16]);
            i += SIMD_WIDTH_I16;
        }

        // Cleanup remaining outside SIMD width
        while i < EMBEDDING_SIZE {
            let w = weights_row[i] as i16;
            if add {
                self.buffer[i] += w;
            } else {
                self.buffer[i] -= w;
            }
            i += 1;
        }
    }

    // Converts the accumulated i16 buffer into f32 activations with ReLU applied.
    pub fn dequantize_and_relu(&self, output: &mut [f32; EMBEDDING_SIZE]) {
        let scale = 1.0 / self.scale;
        let scale_vec = SimdF32::splat(scale);
        let zeros = SimdF32::splat(0.0);

        let mut i = 0;
        while i + SIMD_WIDTH_F32 <= EMBEDDING_SIZE {
            let vals_i16 = &self.buffer[i..i + SIMD_WIDTH_F32];
            let vals_f32 = SimdF32::from_array(std::array::from_fn(|j| vals_i16[j] as f32));

            let dequantized = vals_f32 * scale_vec;
            let activated = dequantized.simd_max(zeros); // ReLU

            activated.copy_to_slice(&mut output[i..i + SIMD_WIDTH_F32]);
            i += SIMD_WIDTH_F32;
        }

        // Cleanup remaining outside SIMD width
        while i < EMBEDDING_SIZE {
            output[i] = (self.buffer[i] as f32 * scale).max(0.0);
            i += 1;
        }
    }
}

/// Computes a scale factor to quantize f32 weights to i8.
/// Uses a percentile-based approach to avoid extreme outliers stretching the range.
fn compute_quantization_scale(weights: &[f32]) -> f32 {
    let max_abs_weight = weights.iter().map(|&w| w.abs()).fold(0.0f32, f32::max);

    let mut abs_weights: Vec<f32> = weights.iter().map(|&w| w.abs()).collect();
    abs_weights.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let percentile_idx = ((abs_weights.len() - 1) as f32 * QUANTIZATION_PERCENTILE) as usize;
    let percentile_weight = abs_weights[percentile_idx];

    if percentile_weight > 0.0 {
        (i8::MAX as f32) / percentile_weight
    } else if max_abs_weight > 0.0 {
        (i8::MAX as f32) / max_abs_weight
    } else {
        64.0
    }
}

/// Quantizes embedding weights from f32 to i8 and transposes for cache-friendly access.
/// Layout changes from [out_idx][feature_idx] to [feature_idx][out_idx].
fn quantize_embedding_weights(weights: &[f32], scale: f32) -> Box<[i8]> {
    let mut quantized = vec![0i8; NUM_FEATURES * EMBEDDING_SIZE].into_boxed_slice();
    for out_idx in 0..EMBEDDING_SIZE {
        let src_row_offset = out_idx * NUM_FEATURES;
        for feature_idx in 0..NUM_FEATURES {
            let val = (weights[src_row_offset + feature_idx] * scale).round();
            quantized[feature_idx * EMBEDDING_SIZE + out_idx] =
                val.clamp(i8::MIN as f32, i8::MAX as f32) as i8;
        }
    }
    quantized
}

fn quantize_embedding_biases(biases: &[f32], scale: f32) -> Box<[i16]> {
    biases
        .iter()
        .map(|&b| (b * scale).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16)
        .collect()
}
