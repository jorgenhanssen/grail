use candle_core::Result;
use candle_nn::Linear;

use super::simd::dot_product;

// Linear layer optimized for CPU inference
pub struct LinearLayer {
    weights: Box<[f32]>,
    biases: Box<[f32]>,
    input_size: usize,
    output_size: usize,
}

impl LinearLayer {
    pub fn from_candle_linear(linear: &Linear) -> Result<Self> {
        Ok(Self {
            weights: linear.weight().flatten_all()?.to_vec1()?.into_boxed_slice(),
            biases: linear.bias().unwrap().to_vec1()?.into_boxed_slice(),
            input_size: linear.weight().dim(1)?,
            output_size: linear.weight().dim(0)?,
        })
    }

    pub fn forward(&self, input: &[f32], output: &mut [f32]) {
        output.copy_from_slice(&self.biases);

        for (i, val) in output.iter_mut().enumerate().take(self.output_size) {
            let offset = i * self.input_size;
            let weights_row = &self.weights[offset..offset + self.input_size];
            *val += dot_product(input, weights_row, self.input_size);
        }
    }
}
