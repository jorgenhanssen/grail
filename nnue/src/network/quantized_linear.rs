use candle_core::Result;
use candle_nn::Linear;

use super::simd::dot_i8_i8;
use super::{QA, WEIGHT_SCALE_BITS};

/// Quantized linear layer for fast integer inference.
///
/// Uses i8 weights and i16 biases. Takes i8 input (from CReLU'd accumulator),
/// computes i8 × i8 → i32 dot products with SIMD.
pub struct QuantizedLinearLayer {
    weights: Box<[i8]>,
    biases: Box<[i32]>,
    input_size: usize,
    output_size: usize,
}

impl QuantizedLinearLayer {
    pub fn from_candle_linear(linear: &Linear, input_scale: f32) -> Result<Self> {
        let weights_f32: Vec<f32> = linear.weight().flatten_all()?.to_vec1()?;
        let biases_f32: Vec<f32> = linear.bias().unwrap().to_vec1()?;
        let input_size = linear.weight().dim(1)?;
        let output_size = linear.weight().dim(0)?;

        // Weight scale: 2^WEIGHT_SCALE_BITS
        let weight_scale = (1 << WEIGHT_SCALE_BITS) as f32;

        // Quantize weights to i8
        let weights: Box<[i8]> = weights_f32
            .iter()
            .map(|&w| {
                let q = (w * weight_scale).round();
                q.clamp(i8::MIN as f32, i8::MAX as f32) as i8
            })
            .collect();

        // Biases: scale by input_scale * weight_scale to match accumulator scale
        let combined_scale = input_scale * weight_scale;
        let biases: Box<[i32]> = biases_f32
            .iter()
            .map(|&b| (b * combined_scale).round() as i32)
            .collect();

        Ok(Self {
            weights,
            biases,
            input_size,
            output_size,
        })
    }

    /// Forward pass: i8 input → i32 output (before activation).
    /// Uses i8×i8 dot product for maximum SIMD throughput.
    #[inline]
    pub fn forward_i8(&self, input: &[i8], output: &mut [i32]) {
        for (i, out) in output.iter_mut().enumerate().take(self.output_size) {
            let offset = i * self.input_size;
            let weights_row = &self.weights[offset..offset + self.input_size];
            *out = self.biases[i] + dot_i8_i8(input, weights_row);
        }
    }

    /// Forward pass for small layers (16 inputs): i8 input → i32 output.
    /// Unrolled scalar for small sizes where SIMD overhead isn't worth it.
    #[inline]
    pub fn forward_small(&self, input: &[i8], output: &mut [i32]) {
        for (i, out) in output.iter_mut().enumerate().take(self.output_size) {
            let offset = i * self.input_size;
            let weights_row = &self.weights[offset..offset + self.input_size];

            let mut sum = self.biases[i];
            for (a, w) in input.iter().zip(weights_row.iter()) {
                sum += (*a as i32) * (*w as i32);
            }
            *out = sum;
        }
    }

    #[inline]
    pub fn input_size(&self) -> usize {
        self.input_size
    }
}

/// Applies CReLU to i32 values and scales down to i8 for the next layer.
/// Clamps to [0, QA] range after scaling.
#[inline]
pub fn crelu_and_scale_i8(input: &[i32], output: &mut [i8], scale: f32) {
    let inv_scale = 1.0 / scale;
    let qa = QA;

    for (out, &val) in output.iter_mut().zip(input.iter()) {
        let scaled = ((val as f32) * inv_scale).round() as i32;
        *out = scaled.clamp(0, qa) as i8;
    }
}
