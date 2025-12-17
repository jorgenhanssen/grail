use candle_core::{Result, Tensor};
use candle_nn::{linear, Linear, VarBuilder};

use crate::encoding::NUM_FEATURES;

use super::{EMBEDDING_SIZE, HIDDEN_SIZE, OUTPUT_BUCKETS};

/// Hidden layers and output head for a single game phase.
pub struct OutputStack {
    pub hidden1: Linear,
    pub hidden2: Linear,
    pub output: Linear,
}

/// Full-precision network for training and weight loading.
/// Shared embedding layer with phase-specific output stacks.
pub struct Network {
    pub embedding: Linear,
    pub buckets: [OutputStack; OUTPUT_BUCKETS],
}

impl Network {
    pub fn new(vs: &VarBuilder) -> Result<Self> {
        // Note: unwrap() is used here because layer creation only fails on programmer error
        // (wrong dimensions). This keeps the array initialization clean.
        let buckets: [OutputStack; OUTPUT_BUCKETS] = std::array::from_fn(|i| {
            let bvs = vs.pp(format!("bucket_{}", i));
            OutputStack {
                hidden1: linear(EMBEDDING_SIZE, HIDDEN_SIZE, bvs.pp("hidden1")).unwrap(),
                hidden2: linear(HIDDEN_SIZE, HIDDEN_SIZE, bvs.pp("hidden2")).unwrap(),
                output: linear(HIDDEN_SIZE, 1, bvs.pp("output")).unwrap(),
            }
        });

        Ok(Self {
            embedding: linear(NUM_FEATURES, EMBEDDING_SIZE, vs.pp("embedding"))?,
            buckets,
        })
    }

    /// Forward pass for training.
    #[inline]
    pub fn forward(&self, x: &Tensor, buckets: &[usize]) -> Result<Tensor> {
        let embedding_out = x.apply(&self.embedding)?.relu()?;

        let mut all_outputs = Vec::with_capacity(OUTPUT_BUCKETS);
        for bucket in &self.buckets {
            let h1 = embedding_out.apply(&bucket.hidden1)?.relu()?;
            let h2 = (h1.apply(&bucket.hidden2)? + &h1)?.relu()?;
            all_outputs.push(h2.apply(&bucket.output)?);
        }
        let stacked = Tensor::cat(&all_outputs, 1)?;

        let batch_size = buckets.len();
        let indices = Tensor::from_vec(
            buckets.iter().map(|&i| i as u32).collect::<Vec<_>>(),
            (batch_size, 1),
            stacked.device(),
        )?;

        stacked.gather(&indices, 1)
    }
}
