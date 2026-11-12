use candle_core::Result;
use candle_nn::Linear;
use cozy_chess::{Board, Color};
use utils::board_metrics::BoardMetrics;
use wide::{i8x32, i16x16, i16x32, i32x16};

use crate::features::{Features, NUM_ACCUMULATOR_BUCKETS, NUM_FEATURES};
use crate::network::{EMBEDDING_SIZE, QUANTIZATION_PERCENTILE};

use super::simd::{SIMD_WIDTH_F32, SIMD_WIDTH_I16, SimdF32, SimdI16};

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

/// First-layer sums for each color, king bucket and mirror state.
///
/// Moving to another king bucket changes the piece weights. We keep the
/// buckets around so returning to one only requires updating its changed
/// features. Each bucket holds both piece and tactical contributions.
#[derive(Clone)]
pub struct Accumulator {
    buckets: [Box<[Bucket]>; Color::NUM],
    output: [[f32; EMBEDDING_SIZE]; 2],
}

impl Accumulator {
    pub fn new(weights: &AccumulatorWeights) -> Self {
        Self {
            buckets: std::array::from_fn(|_| {
                vec![Bucket::new(weights); NUM_ACCUMULATOR_BUCKETS].into_boxed_slice()
            }),
            output: [[0.0; EMBEDDING_SIZE]; 2],
        }
    }

    pub fn update(
        &mut self,
        weights: &AccumulatorWeights,
        board: &Board,
        metrics: &BoardMetrics,
    ) -> &[f32] {
        let stm = board.side_to_move();

        // The output layers expect the side to move's features first.
        for (color, output) in [stm, !stm].into_iter().zip(&mut self.output) {
            let features = Features::from_board(board, metrics, color);
            let bucket = &mut self.buckets[color as usize][features.accumulator_index()];
            bucket
                .previous_features
                .for_each_diff(&features, |feature_idx, added| {
                    apply_feature_change(&mut bucket.values, weights, feature_idx, added);
                });
            bucket.previous_features = features;

            dequantize_and_relu(&bucket.values, weights, output);
        }

        self.output.as_flattened()
    }
}

#[derive(Clone)]
struct Bucket {
    values: [i16; EMBEDDING_SIZE],
    previous_features: Features,
}

impl Bucket {
    fn new(weights: &AccumulatorWeights) -> Self {
        Self {
            values: weights.biases,
            previous_features: Features::default(),
        }
    }
}

/// Shared first-layer weights, stored by feature with a scale per neuron.
pub struct AccumulatorWeights {
    // [feature_idx][embedding_idx]
    weights: Box<[i8]>,
    // [embedding_idx]
    biases: [i16; EMBEDDING_SIZE],

    // Per-neuron weight quantization scale factors to convert it back to f32.
    // USes 1/scale to avoid slower division operations.
    inv_scales: [f32; EMBEDDING_SIZE],
}

impl AccumulatorWeights {
    pub fn from_model(layer: &Linear) -> Result<Self> {
        let weights = layer.weight().flatten_all()?.to_vec1()?;
        let biases = layer.bias().unwrap().to_vec1()?;
        let (scales, inv_scales) = compute_quantization_scales(&weights);
        Ok(Self {
            weights: quantize_embedding_weights(&weights, &scales).into_boxed_slice(),
            biases: quantize_embedding_biases(&biases, &scales),
            inv_scales,
        })
    }
}

fn apply_feature_change(
    buffer: &mut [i16; EMBEDDING_SIZE],
    weights: &AccumulatorWeights,
    feature_idx: usize,
    add: bool,
) {
    let offset = feature_idx * EMBEDDING_SIZE;
    let weights_row = &weights.weights[offset..offset + EMBEDDING_SIZE];
    let (buffer, _) = buffer.as_chunks_mut::<SIMD_WIDTH_I16>();
    let (weights, _) = weights_row.as_chunks::<SIMD_WIDTH_I16>();

    for (values, weights) in buffer.iter_mut().zip(weights) {
        let mut buffer_vec = SimdI16::new(*values);
        let weights_i8 = i8x32::new(*weights);
        let weights_i16 = i16x32::from_i8x32(weights_i8);

        if add {
            buffer_vec += weights_i16;
        } else {
            buffer_vec -= weights_i16;
        }

        *values = *buffer_vec.as_array();
    }
}

/// Dequantizes one color's i16 buffer into f32 and applies ReLU into output.
fn dequantize_and_relu(
    buffer: &[i16; EMBEDDING_SIZE],
    weights: &AccumulatorWeights,
    output: &mut [f32; EMBEDDING_SIZE],
) {
    let zeros = SimdF32::splat(0.0);
    let (buffer, _) = buffer.as_chunks::<SIMD_WIDTH_F32>();
    let (scales, _) = weights.inv_scales.as_chunks::<SIMD_WIDTH_F32>();
    let (output, _) = output.as_chunks_mut::<SIMD_WIDTH_F32>();

    for ((values, scales), output) in buffer.iter().zip(scales).zip(output) {
        let vals_i16 = i16x16::new(*values);
        let vals_f32 = i32x16::from_i16x16(vals_i16).round_float();
        let scale_vec = SimdF32::new(*scales);

        let activated = (vals_f32 * scale_vec).max(zeros);
        *output = *activated.as_array();
    }
}

/// Computes a per-output scale factor to quantize f32 weights to i8.
/// Uses a percentile-based approach to avoid extreme outliers stretching the range.
fn compute_quantization_scales(weights: &[f32]) -> ([f32; EMBEDDING_SIZE], [f32; EMBEDDING_SIZE]) {
    let mut abs_weights = vec![0.0f32; NUM_FEATURES];

    let percentile_index = ((NUM_FEATURES - 1) as f32 * QUANTIZATION_PERCENTILE) as usize;

    let scales = std::array::from_fn(|out_idx| {
        let row = &weights[out_idx * NUM_FEATURES..(out_idx + 1) * NUM_FEATURES];
        for (i, &w) in row.iter().enumerate() {
            abs_weights[i] = w.abs();
        }
        abs_weights.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());

        let max = *abs_weights.last().unwrap_or(&0.0);
        let percentile_weight = abs_weights[percentile_index];

        if percentile_weight > 0.0 {
            i8::MAX as f32 / percentile_weight
        } else if max > 0.0 {
            i8::MAX as f32 / max
        } else {
            64.0
        }
    });

    // so dequantization can multiply instead of divide
    let inv_scales = scales.map(|s| if s != 0.0 { 1.0 / s } else { 0.0 });

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
fn quantize_embedding_biases(biases: &[f32], scales: &[f32]) -> [i16; EMBEDDING_SIZE] {
    std::array::from_fn(|i| round_and_clip_to_i16(biases[i] * scales[i]))
}

fn round_and_clip_to_i8(val: f32) -> i8 {
    val.round().clamp(i8::MIN as f32, i8::MAX as f32) as i8
}

fn round_and_clip_to_i16(val: f32) -> i16 {
    val.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16
}
