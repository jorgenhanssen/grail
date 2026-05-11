use candle_core::Result;
use candle_nn::Linear;

use super::simd::dot_product;

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
        let weights: Box<[f32]> = linear.weight().flatten_all()?.to_vec1()?.into_boxed_slice();
        let biases: Box<[f32]> = linear.bias().unwrap().to_vec1()?.into_boxed_slice();
        let input_size = linear.weight().dim(1)?;
        let output_size = linear.weight().dim(0)?;

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
        output.copy_from_slice(&self.biases);

        for &i in self.active_outputs.iter() {
            let offset = i * self.input_size;
            let weights_row = &self.weights[offset..offset + self.input_size];
            output[i] += dot_product(input, weights_row, self.input_size);
        }
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
