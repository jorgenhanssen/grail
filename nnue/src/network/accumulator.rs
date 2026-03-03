use std::simd::i8x32;
use std::simd::num::SimdInt;
use std::simd::prelude::SimdFloat;

use utils::bitset::Bitset;

use crate::encoding::NUM_FEATURES;

use super::simd::{SIMD_WIDTH_F32, SIMD_WIDTH_I16, SimdF32, SimdI16};
use super::{EMBEDDING_SIZE, QUANTIZATION_PERCENTILE};

/// The Accumulator manages the stateful first (embedding) layer of the NNUE.
///
/// Instead of recomputing the full embedding from scratch on each move,
/// we track which input features changed and incrementally add/subtract
/// the corresponding weight rows. This makes inference O(changed features)
/// rather than O(all features).
///
/// Weights are quantized to i8 and accumulated in i16 for wider SIMD.
/// Dequantization back to f32 happens when outputting to the next layer.
pub struct Accumulator {
    // [feature_idx][embedding_idx]
    weights: Vec<i8>,
    // [embedding_idx]
    biases: Vec<i16>,
    // Accumulated sum of active weights [embedding_idx]
    buffer: [i16; EMBEDDING_SIZE],

    // To know which inputs have changed since the last update
    previous_input: Bitset<NUM_FEATURES>,

    // Per-neuron weight quantization scale factors to convert it back to f32.
    // USes 1/scale to avoid slower division operations.
    inv_scales: Vec<f32>,
}

impl Accumulator {
    pub fn new(weights: &[f32], biases: &[f32]) -> Self {
        let (scales, inv_scales) = compute_quantization_scales(weights);

        let weights_i8 = quantize_embedding_weights(weights, &scales);
        let biases_i16 = quantize_embedding_biases(biases, &scales);

        let mut buffer = [0i16; EMBEDDING_SIZE];
        buffer.copy_from_slice(&biases_i16);

        Self {
            weights: weights_i8,
            biases: biases_i16,
            buffer,
            previous_input: Bitset::default(),
            inv_scales,
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

    /// Converts the accumulated i16 buffer into f32 activations with ReLU applied.
    pub fn dequantize_and_relu(&self, output: &mut [f32; EMBEDDING_SIZE]) {
        let zeros = SimdF32::splat(0.0);

        let mut i = 0;
        while i + SIMD_WIDTH_F32 <= EMBEDDING_SIZE {
            let vals_i16 = &self.buffer[i..i + SIMD_WIDTH_F32];
            let vals_f32 = SimdF32::from_array(std::array::from_fn(|j| vals_i16[j] as f32));
            let scale_vec = SimdF32::from_slice(&self.inv_scales[i..i + SIMD_WIDTH_F32]);

            let dequantized = vals_f32 * scale_vec;
            let activated = dequantized.simd_max(zeros); // ReLU

            activated.copy_to_slice(&mut output[i..i + SIMD_WIDTH_F32]);
            i += SIMD_WIDTH_F32;
        }

        // Cleanup remaining outside SIMD width
        while i < EMBEDDING_SIZE {
            output[i] = (self.buffer[i] as f32 * self.inv_scales[i]).max(0.0);
            i += 1;
        }
    }
}

/// Computes a per-output scale factor to quantize f32 weights to i8.
/// Uses a percentile-based approach to avoid extreme outliers stretching the range.
fn compute_quantization_scales(weights: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let mut scales = Vec::with_capacity(EMBEDDING_SIZE);
    let mut abs_weights = vec![0.0f32; NUM_FEATURES];

    // Index in the sorted abs-weight list used as the percentile cutoff.
    let percentile_index = ((NUM_FEATURES - 1) as f32 * QUANTIZATION_PERCENTILE) as usize;

    // Compute the quantization scale for each accumulator neuron
    for out_idx in 0..EMBEDDING_SIZE {
        let row = &weights[out_idx * NUM_FEATURES..(out_idx + 1) * NUM_FEATURES];
        for (i, &w) in row.iter().enumerate() {
            abs_weights[i] = w.abs();
        }
        abs_weights.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());

        let max = *abs_weights.last().unwrap_or(&0.0);
        let percentile_weight = abs_weights[percentile_index];

        scales.push(if percentile_weight > 0.0 {
            i8::MAX as f32 / percentile_weight
        } else if max > 0.0 {
            i8::MAX as f32 / max
        } else {
            64.0
        });
    }

    // so dequantization can multiply instead of divide
    let inv_scales = scales
        .iter()
        .map(|&s| if s != 0.0 { 1.0 / s } else { 0.0 })
        .collect();

    (scales, inv_scales)
}

/// Quantizes embedding weights from f32 to i8 and transposes for cache-friendly access.
/// Layout changes from [out_idx][feature_idx] to [feature_idx][out_idx].
fn quantize_embedding_weights(weights: &[f32], scales: &[f32]) -> Vec<i8> {
    let mut quantized = vec![0i8; NUM_FEATURES * EMBEDDING_SIZE];
    for out_idx in 0..EMBEDDING_SIZE {
        let row = &weights[out_idx * NUM_FEATURES..(out_idx + 1) * NUM_FEATURES];
        let scale = scales[out_idx];
        for (feature_idx, &w) in row.iter().enumerate() {
            quantized[feature_idx * EMBEDDING_SIZE + out_idx] = round_and_clip_to_i8(w * scale);
        }
    }
    quantized
}

/// Quantizes embedding biases from f32 to i16 using per-output scales.
fn quantize_embedding_biases(biases: &[f32], scales: &[f32]) -> Vec<i16> {
    let mut quantized = vec![0i16; EMBEDDING_SIZE];
    for i in 0..EMBEDDING_SIZE {
        let scale = scales[i];
        quantized[i] = round_and_clip_to_i16(biases[i] * scale);
    }
    quantized
}

fn round_and_clip_to_i8(val: f32) -> i8 {
    val.round().clamp(i8::MIN as f32, i8::MAX as f32) as i8
}

fn round_and_clip_to_i16(val: f32) -> i16 {
    val.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16
}
