use candle_core::{Result, Tensor};
use candle_nn::{Linear, VarBuilder, linear};

use crate::encoding::NUM_FEATURES;

use super::{EMBEDDING_SIZE, HIDDEN_SIZE, OUTPUT_BUCKETS};

/// Full-precision network for training and weight loading.
pub struct Network {
    pub embedding: Linear,
    pub eval_head: EvalHead,
}

impl Network {
    pub fn new(vs: &VarBuilder) -> Result<Self> {
        Ok(Self {
            embedding: linear(NUM_FEATURES, EMBEDDING_SIZE, vs.pp("embedding"))?,
            eval_head: EvalHead::new(vs)?,
        })
    }

    /// Forward pass returning all bucket outputs: [batch, OUTPUT_BUCKETS].
    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let embedding = x.apply(&self.embedding)?.relu()?;
        self.eval_head.forward(&embedding)
    }
}

/// Phase-specific evaluation head with one output stack per bucket.
pub struct EvalHead {
    pub buckets: [OutputStack; OUTPUT_BUCKETS],
}

impl EvalHead {
    fn new(vs: &VarBuilder) -> Result<Self> {
        let buckets = (0..OUTPUT_BUCKETS)
            .map(|i| OutputStack::new(&vs.pp(format!("bucket_{i}"))))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            buckets: buckets.try_into().ok().expect("correct bucket count"),
        })
    }

    fn forward(&self, embedding: &Tensor) -> Result<Tensor> {
        let outputs = self
            .buckets
            .iter()
            .map(|b| b.forward(embedding))
            .collect::<Result<Vec<_>>>()?;
        Tensor::cat(&outputs, 1)
    }

    /// Select each sample's bucket from the full output: [batch, 1].
    pub fn gather(all_buckets: &Tensor, buckets: &[usize]) -> Result<Tensor> {
        let indices = Tensor::from_vec(
            buckets.iter().map(|&i| i as u32).collect::<Vec<_>>(),
            (buckets.len(), 1),
            all_buckets.device(),
        )?;
        all_buckets.gather(&indices, 1)
    }
}

/// Hidden layers and output head for a single game phase.
pub struct OutputStack {
    pub hidden1: Linear,
    pub hidden2: Linear,
    pub output: Linear,
}

impl OutputStack {
    fn new(vs: &VarBuilder) -> Result<Self> {
        Ok(Self {
            hidden1: linear(EMBEDDING_SIZE, HIDDEN_SIZE, vs.pp("hidden1"))?,
            hidden2: linear(HIDDEN_SIZE, HIDDEN_SIZE, vs.pp("hidden2"))?,
            output: linear(HIDDEN_SIZE, 1, vs.pp("output"))?,
        })
    }

    fn forward(&self, input: &Tensor) -> Result<Tensor> {
        let h1 = input.apply(&self.hidden1)?.relu()?;
        let h2 = (h1.apply(&self.hidden2)? + &h1)?.relu()?;
        h2.apply(&self.output)
    }
}
