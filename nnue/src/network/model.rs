use candle_core::{Result, Tensor};
use candle_nn::{linear, Linear, VarBuilder};

use crate::encoding::NUM_FEATURES;

use super::{EMBEDDING_SIZE, HIDDEN_SIZE, OUTPUT_BUCKETS};

/// Per-bucket network layers (hidden1, hidden2, output).
/// Each bucket has its own weights to learn phase-specific evaluation.
pub struct BucketLayers {
    pub(crate) hidden1: Linear,
    pub(crate) hidden2: Linear,
    pub(crate) output: Linear,
}

/// Full-precision network for training and weight loading (via Candle).
/// Architecture: shared embedding, per-bucket hidden layers and output.
/// This allows learning phase-specific evaluation (opening vs endgame).
pub struct Network {
    pub(crate) embedding: Linear,
    pub(crate) buckets: [BucketLayers; OUTPUT_BUCKETS],
}

impl Network {
    pub fn new(vs: &VarBuilder) -> Result<Self> {
        // Note: unwrap() is used here because layer creation only fails on programmer error
        // (wrong dimensions). This keeps the array initialization clean.
        let buckets: [BucketLayers; OUTPUT_BUCKETS] = std::array::from_fn(|i| {
            let bvs = vs.pp(format!("bucket_{}", i));
            BucketLayers {
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

    /// Forward pass with bucket selection for training.
    /// Computes all buckets and gathers the correct output per sample.
    /// Gradients only flow to each sample's selected bucket via scatter.
    #[inline]
    pub fn forward_with_buckets(&self, x: &Tensor, bucket_indices: &[usize]) -> Result<Tensor> {
        let embedding_out = x.apply(&self.embedding)?.relu()?;

        // Compute all bucket outputs (backward pass will only update relevant buckets)
        let mut all_outputs = Vec::with_capacity(OUTPUT_BUCKETS);
        for bucket in &self.buckets {
            let h1 = embedding_out.apply(&bucket.hidden1)?.relu()?;
            let h2 = (h1.apply(&bucket.hidden2)? + &h1)?.relu()?;
            all_outputs.push(h2.apply(&bucket.output)?);
        }
        let stacked = Tensor::cat(&all_outputs, 1)?;

        // Gather selects correct bucket per sample; scatter routes gradients back
        let batch_size = bucket_indices.len();
        let indices = Tensor::from_vec(
            bucket_indices.iter().map(|&i| i as u32).collect::<Vec<_>>(),
            (batch_size, 1),
            stacked.device(),
        )?;

        stacked.gather(&indices, 1)
    }
}
