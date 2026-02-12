use candle_core::{Result, Tensor};
use candle_nn::{Linear, VarBuilder, linear};

use crate::encoding::NUM_FEATURES;

use super::{
    EMBEDDING_SIZE, EVAL_HIDDEN_SIZE, OUTPUT_BUCKETS, POLICY_HIDDEN_SIZE, POLICY_OUTPUT_SIZE,
};

/// Full-precision network for training and weight loading.
/// Shared embedding feeds into phase-specific eval heads and a policy/piece head.
pub struct Network {
    pub embedding: Linear,
    pub eval_heads: [EvalHead; OUTPUT_BUCKETS],
    pub policy_head: PolicyHead,
}

impl Network {
    pub fn new(vs: &VarBuilder) -> Result<Self> {
        let eval_heads: [EvalHead; OUTPUT_BUCKETS] = std::array::from_fn(|i| {
            let bvs = vs.pp(format!("bucket_{}", i));
            EvalHead {
                hidden1: linear(EMBEDDING_SIZE, EVAL_HIDDEN_SIZE, bvs.pp("hidden1")).unwrap(),
                hidden2: linear(EVAL_HIDDEN_SIZE, EVAL_HIDDEN_SIZE, bvs.pp("hidden2")).unwrap(),
                output: linear(EVAL_HIDDEN_SIZE, 1, bvs.pp("output")).unwrap(),
            }
        });

        let pvs = vs.pp("policy");
        let policy_head = PolicyHead {
            hidden1: linear(EMBEDDING_SIZE, POLICY_HIDDEN_SIZE, pvs.pp("hidden1")).unwrap(),
            hidden2: linear(POLICY_HIDDEN_SIZE, POLICY_HIDDEN_SIZE, pvs.pp("hidden2")).unwrap(),
            output: linear(POLICY_HIDDEN_SIZE, POLICY_OUTPUT_SIZE, pvs.pp("output")).unwrap(),
        };

        Ok(Self {
            embedding: linear(NUM_FEATURES, EMBEDDING_SIZE, vs.pp("embedding"))?,
            eval_heads,
            policy_head,
        })
    }

    /// Forward pass for training. Returns (eval [batch, 1], policy_logits [batch, 6]).
    ///
    /// Computes all bucket outputs then gathers the correct one per sample.
    /// `gather` scatters gradients only to each sample's selected bucket.
    #[inline]
    pub fn forward(&self, x: &Tensor, buckets: &[usize]) -> Result<(Tensor, Tensor)> {
        let embedding_out = x.apply(&self.embedding)?.relu()?;

        // Eval head (bucketed)
        let all_outputs: Vec<_> = self
            .eval_heads
            .iter()
            .map(|h| h.forward(&embedding_out))
            .collect::<Result<_>>()?;
        let stacked = Tensor::cat(&all_outputs, 1)?;

        // Select each sample's bucket output: [batch, 1]
        let indices = Tensor::from_vec(
            buckets.iter().map(|&i| i as u32).collect::<Vec<_>>(),
            (buckets.len(), 1),
            stacked.device(),
        )?;
        let eval = stacked.gather(&indices, 1)?;

        let policy = self.policy_head.forward(&embedding_out)?;

        Ok((eval, policy))
    }
}

/// Eval head for a single game phase (output bucket).
pub struct EvalHead {
    pub hidden1: Linear,
    pub hidden2: Linear,
    pub output: Linear,
}

impl EvalHead {
    fn forward(&self, input: &Tensor) -> Result<Tensor> {
        let h1 = input.apply(&self.hidden1)?.relu()?;
        let h2 = (h1.apply(&self.hidden2)? + &h1)?.relu()?;
        h2.apply(&self.output)
    }
}

/// Policy head: predicts the best move's piece type from the shared embedding.
pub struct PolicyHead {
    pub hidden1: Linear,
    pub hidden2: Linear,
    pub output: Linear,
}

impl PolicyHead {
    fn forward(&self, embedding: &Tensor) -> Result<Tensor> {
        let h1 = embedding.apply(&self.hidden1)?.relu()?;
        let h2 = (h1.apply(&self.hidden2)? + &h1)?.relu()?;
        h2.apply(&self.output)
    }
}
