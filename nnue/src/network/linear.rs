use candle_core::Result;
use candle_nn::Linear;

use super::simd::{SIMD_BATCH_SIZE, SIMD_WIDTH_F32, simd_dot_product, simd_dot_product_batched};

/// Linear layer optimized for CPU inference.
///
/// Uses SIMD-accelerated dot products and pre-flattened weight storage.
/// Unlike the embedding layer, this is not quantized since the hidden layers
/// are small enough that f32 performance is acceptable.
pub struct LinearLayer {
    weights: Box<[f32]>,
    biases: Box<[f32]>,
    input_size: usize,
    active_outputs: Box<[usize]>,
}

impl LinearLayer {
    pub fn from_candle_linear(linear: &Linear) -> Result<Self> {
        let weights = linear.weight();
        let (output_size, input_size) = weights.dims2()?;
        assert!(
            input_size.is_multiple_of(SIMD_WIDTH_F32),
            "NNUE linear input width must be a multiple of {SIMD_WIDTH_F32}"
        );

        let weights: Box<[f32]> = weights.flatten_all()?.to_vec1()?.into_boxed_slice();
        let biases: Box<[f32]> = linear.bias().unwrap().to_vec1()?.into_boxed_slice();

        // Earlier NNUE analysis have found some cases with dead neurons.
        // They can be skipped to save computation.
        let active_outputs = prune_dead_neurons(&weights, input_size, output_size);

        Ok(Self {
            weights,
            biases,
            input_size,
            active_outputs,
        })
    }

    pub fn forward(&self, input: &[f32], output: &mut [f32]) {
        // Let's batch the neurons so we don't have to reload the input for each and every one of them.
        output.copy_from_slice(&self.biases);

        let (batches, rest) = self.active_outputs.as_chunks::<SIMD_BATCH_SIZE>();
        for &indices in batches {
            let rows = indices.map(|index| self.weight_row(index));
            let dots = simd_dot_product_batched(input, rows);
            for i in 0..indices.len() {
                output[indices[i]] += dots[i];
            }
        }
        for &i in rest {
            output[i] += simd_dot_product(input, self.weight_row(i));
        }
    }

    fn weight_row(&self, index: usize) -> &[f32] {
        let offset = index * self.input_size;
        &self.weights[offset..offset + self.input_size]
    }
}

/// Output indices with at least one weight above the threshold.
fn prune_dead_neurons(weights: &[f32], input_size: usize, output_size: usize) -> Box<[usize]> {
    const DEAD_ROW_THRESHOLD: f32 = 0.004;

    (0..output_size)
        .filter(|&i| {
            let row = &weights[i * input_size..(i + 1) * input_size];
            row.iter().any(|&w| w.abs() >= DEAD_ROW_THRESHOLD)
        })
        .collect()
}
