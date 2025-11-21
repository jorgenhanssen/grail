use candle_core::{Result, Tensor};
use candle_nn::{linear, Linear, Module, VarBuilder};
use std::simd::prelude::SimdFloat;

use crate::encoding::{NUM_FEATURES, NUM_U64S};

use std::simd::{f32x16, i16x32};
type SimdF32 = f32x16;
type SimdI16 = i16x32;
const SIMD_WIDTH_F32: usize = 16;
const SIMD_WIDTH_I16: usize = 32;

const EMBEDDING_SIZE: usize = 1024;
const HIDDEN_SIZE: usize = 16;

// Neural network evaluation scaling constants
pub const CP_MAX: i16 = 5000;
pub const CP_MIN: i16 = -5000;
pub const FV_SCALE: f32 = 400.0;

// Quantization scale is computed from this percentile of absolute weights.
// Lower values clip more outliers but give better precision for typical weights.
const QUANTIZATION_PERCENTILE: f32 = 0.999;

// Quantization range limits
const I8_MIN: f32 = -128.0;
const I8_MAX: f32 = 127.0;
const I16_MIN: f32 = -32768.0;
const I16_MAX: f32 = 32767.0;

// Bitset encoding
const BITS_PER_U64: usize = 64;

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
        let x = x.apply(&self.embedding)?.relu()?;

        let h1 = x.apply(&self.hidden1)?.relu()?;
        let h2 = (h1.apply(&self.hidden2)? + &h1)?.relu()?;

        let x = h2.apply(&self.output)?;

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

    #[inline(always)]
    fn forward(&self, input: &[f32], output: &mut [f32]) {
        output.copy_from_slice(&self.biases);

        for i in 0..self.output_size {
            let weights_row_offset = i * self.input_size;
            let weights_row =
                &self.weights[weights_row_offset..weights_row_offset + self.input_size];

            output[i] += dot(input, weights_row, self.input_size);
        }
    }
}

pub struct NNUENetwork {
    hidden1: LinearLayer,
    hidden2: LinearLayer,
    output: LinearLayer,

    // Transposed layout: [feature_idx][embedding_idx]
    embedding_weights_i8: Box<[i8]>,
    embedding_biases_i16: Box<[i16]>,
    embedding_buffer_i16: [i16; EMBEDDING_SIZE],
    quantization_scale: f32,

    hidden1_buffer: [f32; HIDDEN_SIZE],
    hidden2_buffer: [f32; HIDDEN_SIZE],
    output_buffer: [f32; 1],

    current_input: [u64; NUM_U64S],
    previous_input: [u64; NUM_U64S],
}

impl NNUENetwork {
    pub fn from_network(network: &Network) -> Result<Self> {
        let embedding = LinearLayer::from_candle_linear(&network.embedding)?;
        let hidden1 = LinearLayer::from_candle_linear(&network.hidden1)?;
        let hidden2 = LinearLayer::from_candle_linear(&network.hidden2)?;
        let output = LinearLayer::from_candle_linear(&network.output)?;

        let quantization_scale = compute_quantization_scale(&embedding.weights);
        let embedding_weights_i8 =
            quantize_embedding_weights(&embedding.weights, quantization_scale);
        let embedding_biases_i16 = quantize_embedding_biases(&embedding.biases, quantization_scale);

        let mut embedding_buffer_i16 = [0i16; EMBEDDING_SIZE];
        embedding_buffer_i16.copy_from_slice(&embedding_biases_i16);

        Ok(Self {
            hidden1,
            hidden2,
            output,
            embedding_weights_i8,
            embedding_biases_i16,
            embedding_buffer_i16,
            quantization_scale,
            hidden1_buffer: [0.0; HIDDEN_SIZE],
            hidden2_buffer: [0.0; HIDDEN_SIZE],
            output_buffer: [0.0; 1],
            current_input: [0u64; NUM_U64S],
            previous_input: [0u64; NUM_U64S],
        })
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.embedding_buffer_i16
            .copy_from_slice(&self.embedding_biases_i16);
        self.previous_input.fill(0);
        self.current_input.fill(0);
    }

    #[inline(always)]
    fn update_embedding_for_feature(&mut self, feature_idx: usize, is_active: bool) {
        let weights_row = &self.embedding_weights_i8
            [feature_idx * EMBEDDING_SIZE..feature_idx * EMBEDDING_SIZE + EMBEDDING_SIZE];

        let mut i = 0;
        let mut weights_i16_buf = [0i16; SIMD_WIDTH_I16];

        if is_active {
            while i + SIMD_WIDTH_I16 <= EMBEDDING_SIZE {
                let mut acc =
                    SimdI16::from_slice(&self.embedding_buffer_i16[i..i + SIMD_WIDTH_I16]);

                for j in 0..SIMD_WIDTH_I16 {
                    weights_i16_buf[j] = weights_row[i + j] as i16;
                }
                let weights_i16 = SimdI16::from_slice(&weights_i16_buf);

                acc += weights_i16;
                acc.copy_to_slice(&mut self.embedding_buffer_i16[i..i + SIMD_WIDTH_I16]);
                i += SIMD_WIDTH_I16;
            }

            while i < EMBEDDING_SIZE {
                self.embedding_buffer_i16[i] += weights_row[i] as i16;
                i += 1;
            }
        } else {
            while i + SIMD_WIDTH_I16 <= EMBEDDING_SIZE {
                let mut acc =
                    SimdI16::from_slice(&self.embedding_buffer_i16[i..i + SIMD_WIDTH_I16]);

                for j in 0..SIMD_WIDTH_I16 {
                    weights_i16_buf[j] = weights_row[i + j] as i16;
                }
                let weights_i16 = SimdI16::from_slice(&weights_i16_buf);

                acc -= weights_i16;
                acc.copy_to_slice(&mut self.embedding_buffer_i16[i..i + SIMD_WIDTH_I16]);
                i += SIMD_WIDTH_I16;
            }

            while i < EMBEDDING_SIZE {
                self.embedding_buffer_i16[i] -= weights_row[i] as i16;
                i += 1;
            }
        }
    }

    #[inline(always)]
    pub fn forward(&mut self, input: &[f32]) -> f32 {
        self.current_input.fill(0);

        for (i, &val) in input.iter().enumerate().take(NUM_FEATURES) {
            if val > 0.0 {
                let word_idx = i / BITS_PER_U64;
                let bit_idx = i % BITS_PER_U64;
                self.current_input[word_idx] |= 1u64 << bit_idx;
            }
        }

        // Incremental update: only process changed features
        for word_idx in 0..NUM_U64S {
            let mut changes = self.previous_input[word_idx] ^ self.current_input[word_idx];
            while changes != 0 {
                let bit_idx = changes.trailing_zeros() as usize;
                changes &= changes - 1;

                let feature_idx = word_idx * BITS_PER_U64 + bit_idx;
                if feature_idx >= NUM_FEATURES {
                    continue;
                }
                let mask = 1u64 << bit_idx;
                let is_active = (self.current_input[word_idx] & mask) != 0;
                self.update_embedding_for_feature(feature_idx, is_active);
            }
        }

        self.previous_input.copy_from_slice(&self.current_input);

        let mut activated_embedding = [0.0; EMBEDDING_SIZE];
        dequantize_and_relu(
            &self.embedding_buffer_i16,
            &mut activated_embedding,
            self.quantization_scale,
        );

        self.hidden1
            .forward(&activated_embedding, &mut self.hidden1_buffer);
        relu(&mut self.hidden1_buffer);

        self.hidden2
            .forward(&self.hidden1_buffer, &mut self.hidden2_buffer);
        add(&mut self.hidden2_buffer, &self.hidden1_buffer);
        relu(&mut self.hidden2_buffer);

        self.output
            .forward(&self.hidden2_buffer, &mut self.output_buffer);

        let cp = self.output_buffer[0] * FV_SCALE;
        cp.clamp(CP_MIN as f32, CP_MAX as f32)
    }

    #[inline(always)]
    pub fn forward_bitset(&mut self, bitset: &[u64; NUM_U64S]) -> f32 {
        self.current_input.copy_from_slice(bitset);

        for word_idx in 0..NUM_U64S {
            let mut changes = self.previous_input[word_idx] ^ self.current_input[word_idx];
            while changes != 0 {
                let bit_idx = changes.trailing_zeros() as usize;
                changes &= changes - 1;

                let feature_idx = word_idx * BITS_PER_U64 + bit_idx;
                if feature_idx >= NUM_FEATURES {
                    continue;
                }
                let mask = 1u64 << bit_idx;
                let is_active = (self.current_input[word_idx] & mask) != 0;
                self.update_embedding_for_feature(feature_idx, is_active);
            }
        }

        self.previous_input.copy_from_slice(&self.current_input);

        let mut activated_embedding = [0.0; EMBEDDING_SIZE];
        dequantize_and_relu(
            &self.embedding_buffer_i16,
            &mut activated_embedding,
            self.quantization_scale,
        );

        self.hidden1
            .forward(&activated_embedding, &mut self.hidden1_buffer);
        relu(&mut self.hidden1_buffer);

        self.hidden2
            .forward(&self.hidden1_buffer, &mut self.hidden2_buffer);
        add(&mut self.hidden2_buffer, &self.hidden1_buffer);
        relu(&mut self.hidden2_buffer);

        self.output
            .forward(&self.hidden2_buffer, &mut self.output_buffer);

        let cp = self.output_buffer[0] * FV_SCALE;
        cp.clamp(CP_MIN as f32, CP_MAX as f32)
    }
}

fn compute_quantization_scale(weights: &[f32]) -> f32 {
    let max_abs_weight = weights.iter().map(|&w| w.abs()).fold(0.0f32, f32::max);

    // Use percentile-based scaling instead of max to optimize precision
    //
    // Rationale: Max weights are often outliers. Using percentile-based scaling
    // gives better precision for the majority of weights at the cost of clipping
    // rare outliers.
    let mut abs_weights: Vec<f32> = weights.iter().map(|&w| w.abs()).collect();
    abs_weights.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Compute percentile index (len-1 to get valid array index for 100th percentile)
    let percentile_idx = ((abs_weights.len() - 1) as f32 * QUANTIZATION_PERCENTILE) as usize;
    let percentile_weight = abs_weights[percentile_idx];

    if percentile_weight > 0.0 {
        I8_MAX / percentile_weight
    } else if max_abs_weight > 0.0 {
        I8_MAX / max_abs_weight
    } else {
        64.0 // Fallback if all weights are zero (should never happen)
    }
}

fn quantize_embedding_weights(weights: &[f32], scale: f32) -> Box<[i8]> {
    let mut quantized = vec![0i8; NUM_FEATURES * EMBEDDING_SIZE].into_boxed_slice();

    // Transpose: [out][in] -> [in][out] layout for efficient incremental updates
    for out_idx in 0..EMBEDDING_SIZE {
        let src_row_offset = out_idx * NUM_FEATURES;
        for feature_idx in 0..NUM_FEATURES {
            let weight = weights[src_row_offset + feature_idx];
            let quantized_value = (weight * scale).round().clamp(I8_MIN, I8_MAX) as i8;
            quantized[feature_idx * EMBEDDING_SIZE + out_idx] = quantized_value;
        }
    }

    quantized
}

fn quantize_embedding_biases(biases: &[f32], scale: f32) -> Box<[i16]> {
    biases
        .iter()
        .map(|&b| (b * scale).round().clamp(I16_MIN, I16_MAX) as i16)
        .collect()
}

#[inline(always)]
fn dequantize_and_relu(
    input_i16: &[i16; EMBEDDING_SIZE],
    output_f32: &mut [f32; EMBEDDING_SIZE],
    quantization_scale: f32,
) {
    let dequant_scale = 1.0 / quantization_scale;
    let zeros = SimdF32::splat(0.0);
    let scale_vec = SimdF32::splat(dequant_scale);
    let mut i = 0;

    while i + SIMD_WIDTH_F32 <= EMBEDDING_SIZE {
        let vals_i16 = &input_i16[i..i + SIMD_WIDTH_F32];
        let vals_f32_array: [f32; SIMD_WIDTH_F32] = std::array::from_fn(|j| vals_i16[j] as f32);
        let vals_f32 = SimdF32::from_array(vals_f32_array);

        let dequantized = vals_f32 * scale_vec;
        let activated = dequantized.simd_max(zeros);

        activated.copy_to_slice(&mut output_f32[i..i + SIMD_WIDTH_F32]);
        i += SIMD_WIDTH_F32;
    }

    while i < EMBEDDING_SIZE {
        output_f32[i] = (input_i16[i] as f32 * dequant_scale).max(0.0);
        i += 1;
    }
}

#[inline(always)]
fn relu(values: &mut [f32]) {
    let len = values.len();
    let mut i = 0;

    const UNROLL: usize = 4;

    let zeros = SimdF32::splat(0.0);
    let limit = len - (len % (SIMD_WIDTH_F32 * UNROLL));

    while i < limit {
        let chunk0 = SimdF32::from_slice(&values[i..i + SIMD_WIDTH_F32]);
        let chunk1 = SimdF32::from_slice(&values[i + SIMD_WIDTH_F32..i + SIMD_WIDTH_F32 * 2]);
        let chunk2 = SimdF32::from_slice(&values[i + SIMD_WIDTH_F32 * 2..i + SIMD_WIDTH_F32 * 3]);
        let chunk3 = SimdF32::from_slice(&values[i + SIMD_WIDTH_F32 * 3..i + SIMD_WIDTH_F32 * 4]);

        let result0 = chunk0.simd_max(zeros);
        let result1 = chunk1.simd_max(zeros);
        let result2 = chunk2.simd_max(zeros);
        let result3 = chunk3.simd_max(zeros);

        result0.copy_to_slice(&mut values[i..i + SIMD_WIDTH_F32]);
        result1.copy_to_slice(&mut values[i + SIMD_WIDTH_F32..i + SIMD_WIDTH_F32 * 2]);
        result2.copy_to_slice(&mut values[i + SIMD_WIDTH_F32 * 2..i + SIMD_WIDTH_F32 * 3]);
        result3.copy_to_slice(&mut values[i + SIMD_WIDTH_F32 * 3..i + SIMD_WIDTH_F32 * 4]);

        i += SIMD_WIDTH_F32 * UNROLL;
    }

    for j in i..len {
        values[j] = values[j].max(0.0);
    }
}

#[inline(always)]
fn add(dest: &mut [f32], src: &[f32]) {
    let len = dest.len();
    let mut i = 0;

    const UNROLL: usize = 4;
    let limit = len - (len % (SIMD_WIDTH_F32 * UNROLL));

    while i < limit {
        let dest0 = SimdF32::from_slice(&dest[i..i + SIMD_WIDTH_F32]);
        let dest1 = SimdF32::from_slice(&dest[i + SIMD_WIDTH_F32..i + SIMD_WIDTH_F32 * 2]);
        let dest2 = SimdF32::from_slice(&dest[i + SIMD_WIDTH_F32 * 2..i + SIMD_WIDTH_F32 * 3]);
        let dest3 = SimdF32::from_slice(&dest[i + SIMD_WIDTH_F32 * 3..i + SIMD_WIDTH_F32 * 4]);

        let src0 = SimdF32::from_slice(&src[i..i + SIMD_WIDTH_F32]);
        let src1 = SimdF32::from_slice(&src[i + SIMD_WIDTH_F32..i + SIMD_WIDTH_F32 * 2]);
        let src2 = SimdF32::from_slice(&src[i + SIMD_WIDTH_F32 * 2..i + SIMD_WIDTH_F32 * 3]);
        let src3 = SimdF32::from_slice(&src[i + SIMD_WIDTH_F32 * 3..i + SIMD_WIDTH_F32 * 4]);

        let result0 = dest0 + src0;
        let result1 = dest1 + src1;
        let result2 = dest2 + src2;
        let result3 = dest3 + src3;

        result0.copy_to_slice(&mut dest[i..i + SIMD_WIDTH_F32]);
        result1.copy_to_slice(&mut dest[i + SIMD_WIDTH_F32..i + SIMD_WIDTH_F32 * 2]);
        result2.copy_to_slice(&mut dest[i + SIMD_WIDTH_F32 * 2..i + SIMD_WIDTH_F32 * 3]);
        result3.copy_to_slice(&mut dest[i + SIMD_WIDTH_F32 * 3..i + SIMD_WIDTH_F32 * 4]);

        i += SIMD_WIDTH_F32 * UNROLL;
    }

    for j in i..len {
        dest[j] += src[j];
    }
}

#[inline(always)]
fn dot(a: &[f32], b: &[f32], len: usize) -> f32 {
    let mut sum_vec0 = SimdF32::splat(0.0);
    let mut sum_vec1 = SimdF32::splat(0.0);
    let mut sum_vec2 = SimdF32::splat(0.0);
    let mut sum_vec3 = SimdF32::splat(0.0);

    const UNROLL: usize = 4;

    let limit = len - (len % (SIMD_WIDTH_F32 * UNROLL));
    let mut i = 0;

    while i < limit {
        let a0 = SimdF32::from_slice(&a[i..i + SIMD_WIDTH_F32]);
        let b0 = SimdF32::from_slice(&b[i..i + SIMD_WIDTH_F32]);
        let a1 = SimdF32::from_slice(&a[i + SIMD_WIDTH_F32..i + SIMD_WIDTH_F32 * 2]);
        let b1 = SimdF32::from_slice(&b[i + SIMD_WIDTH_F32..i + SIMD_WIDTH_F32 * 2]);
        let a2 = SimdF32::from_slice(&a[i + SIMD_WIDTH_F32 * 2..i + SIMD_WIDTH_F32 * 3]);
        let b2 = SimdF32::from_slice(&b[i + SIMD_WIDTH_F32 * 2..i + SIMD_WIDTH_F32 * 3]);
        let a3 = SimdF32::from_slice(&a[i + SIMD_WIDTH_F32 * 3..i + SIMD_WIDTH_F32 * 4]);
        let b3 = SimdF32::from_slice(&b[i + SIMD_WIDTH_F32 * 3..i + SIMD_WIDTH_F32 * 4]);

        sum_vec0 += a0 * b0;
        sum_vec1 += a1 * b1;
        sum_vec2 += a2 * b2;
        sum_vec3 += a3 * b3;

        i += SIMD_WIDTH_F32 * UNROLL;
    }

    let sum_vec = sum_vec0 + sum_vec1 + sum_vec2 + sum_vec3;
    let sum = sum_vec.as_array().iter().sum::<f32>();

    let mut scalar_sum = 0.0;
    while i < len {
        scalar_sum += a[i] * b[i];
        i += 1;
    }

    sum + scalar_sum
}
