use std::simd::num::SimdInt;
use std::simd::prelude::SimdFloat;
use std::simd::{i8x32, i16x16};

use cozy_chess::Color;

use crate::bitset;
use crate::encoding::NUM_FEATURES;

use super::simd::{SIMD_WIDTH_F32, SIMD_WIDTH_I16, SimdF32, SimdI16};
use super::{EMBEDDING_SIZE, QUANTIZATION_PERCENTILE};

// Just enforcing the sizes as multiples of the SIMD widths.
// That way I don't have to do cleanup loops after the SIMD,
const _: () = assert!(
    EMBEDDING_SIZE.is_multiple_of(SIMD_WIDTH_I16),
    "EMBEDDING_SIZE must be a multiple of SIMD_WIDTH_I16"
);
const _: () = assert!(
    EMBEDDING_SIZE.is_multiple_of(SIMD_WIDTH_F32),
    "EMBEDDING_SIZE must be a multiple of SIMD_WIDTH_F32"
);

/// The Accumulator manages the stateful first (embedding) layer of the NNUE.
///
/// Instead of recomputing the full embedding from scratch on each move,
/// we track which input features changed and incrementally add/subtract
/// the corresponding weight rows. This makes inference O(changed features)
/// rather than O(all features).
///
/// The embedding is shared across both perspectives, so we run it twice
/// against the same weights and keep one buffer per absolute color.
///
/// Weights are quantized to i8 and accumulated in i16 for wider SIMD.
/// Dequantization back to f32 happens when outputting to the next layer.
pub struct Accumulator {
    // [feature_idx][embedding_idx]
    weights: Vec<i8>,
    // [embedding_idx]
    biases: Vec<i16>,
    // Accumulated sum of active weights [embedding_idx]
    buffer_white: [i16; EMBEDDING_SIZE],
    buffer_black: [i16; EMBEDDING_SIZE],

    // To know which inputs have changed since the last update
    previous_white: bitset!(NUM_FEATURES),
    previous_black: bitset!(NUM_FEATURES),

    // Per-neuron weight quantization scale factors to convert it back to f32.
    // USes 1/scale to avoid slower division operations.
    inv_scales: Vec<f32>,
}

impl Accumulator {
    pub fn new(weights: &[f32], biases: &[f32]) -> Self {
        let (scales, inv_scales) = compute_quantization_scales(weights);

        let weights_i8 = quantize_embedding_weights(weights, &scales);
        let biases_i16 = quantize_embedding_biases(biases, &scales);

        let mut buffer_white = [0i16; EMBEDDING_SIZE];
        let mut buffer_black = [0i16; EMBEDDING_SIZE];
        buffer_white.copy_from_slice(&biases_i16);
        buffer_black.copy_from_slice(&biases_i16);

        Self {
            weights: weights_i8,
            biases: biases_i16,
            buffer_white,
            buffer_black,
            previous_white: Default::default(),
            previous_black: Default::default(),
            inv_scales,
        }
    }

    pub fn reset(&mut self) {
        self.buffer_white.copy_from_slice(&self.biases);
        self.buffer_black.copy_from_slice(&self.biases);
        self.previous_white = Default::default();
        self.previous_black = Default::default();
    }

    /// Updates the accumulators based on the difference between previous and current inputs.
    pub fn update(&mut self, color: Color, new_input: &bitset!(NUM_FEATURES)) {
        let previous = match color {
            Color::White => self.previous_white,
            Color::Black => self.previous_black,
        };

        previous.for_each_diff(new_input, |idx| {
            let is_active = new_input.get(idx);
            self.apply_feature_change(color, idx, is_active);
        });

        match color {
            Color::White => self.previous_white = *new_input,
            Color::Black => self.previous_black = *new_input,
        }
    }

    fn apply_feature_change(&mut self, color: Color, feature_idx: usize, add: bool) {
        let offset = feature_idx * EMBEDDING_SIZE;
        let weights_row = &self.weights[offset..offset + EMBEDDING_SIZE];
        let buffer = match color {
            Color::White => &mut self.buffer_white,
            Color::Black => &mut self.buffer_black,
        };

        for i in (0..EMBEDDING_SIZE).step_by(SIMD_WIDTH_I16) {
            let mut buffer_vec = SimdI16::from_slice(&buffer[i..i + SIMD_WIDTH_I16]);
            let weights_i8 = i8x32::from_slice(&weights_row[i..i + SIMD_WIDTH_I16]);
            let weights_i16: SimdI16 = weights_i8.cast();

            if add {
                buffer_vec += weights_i16;
            } else {
                buffer_vec -= weights_i16;
            }

            buffer_vec.copy_to_slice(&mut buffer[i..i + SIMD_WIDTH_I16]);
        }
    }

    /// Dequantizes one color's i16 buffer into f32 and applies ReLU into output.
    pub fn dequantize_and_relu(&self, color: Color, output: &mut [f32]) {
        debug_assert_eq!(output.len(), EMBEDDING_SIZE);
        let buffer = match color {
            Color::White => &self.buffer_white,
            Color::Black => &self.buffer_black,
        };

        let zeros = SimdF32::splat(0.0);

        for i in (0..EMBEDDING_SIZE).step_by(SIMD_WIDTH_F32) {
            let vals_f32 = i16x16::from_slice(&buffer[i..i + SIMD_WIDTH_F32]).cast();
            let scale_vec = SimdF32::from_slice(&self.inv_scales[i..i + SIMD_WIDTH_F32]);

            let dequantized = vals_f32 * scale_vec;
            let activated = dequantized.simd_max(zeros); // ReLU

            activated.copy_to_slice(&mut output[i..i + SIMD_WIDTH_F32]);
        }
    }
}

/// Computes a per-output scale factor to quantize f32 weights to i8.
/// Uses a percentile-based approach to avoid extreme outliers stretching the range.
fn compute_quantization_scales(weights: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let mut scales = Vec::with_capacity(EMBEDDING_SIZE);
    let mut abs_weights = vec![0.0f32; NUM_FEATURES];

    let percentile_index = ((NUM_FEATURES - 1) as f32 * QUANTIZATION_PERCENTILE) as usize;

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
