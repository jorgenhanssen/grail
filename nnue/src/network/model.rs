use candle_core::{Result, Tensor};
use candle_nn::{Linear, VarBuilder, linear};

use crate::encoding::NUM_FEATURES;

use super::{EMBEDDING_SIZE, HIDDEN_SIZE, OUTPUT_BUCKETS};

/// Full-precision network for training and weight loading.
///
/// A single embedding layer is run over both perspectives and the outputs are
/// concatenated [...stm, ...nstm] before being fed to the phase-specific hidden
/// stack.
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
                hidden1: linear(2 * EMBEDDING_SIZE, HIDDEN_SIZE, bvs.pp("hidden1")).unwrap(),
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
    ///
    /// Computes all bucket outputs, then gathers the correct one per sample.
    /// During backprop, `gather` scatters gradients only to each sample's
    /// selected bucket—unused buckets receive zero gradient for that sample.
    ///
    /// `stm` and `nstm` are feature tensors for the side-to-move and opponent
    /// perspectives of the same position. The output is in stm-perspective
    /// space; callers that need a white-perspective value must sign-flip.
    #[inline]
    pub fn forward(&self, stm: &Tensor, nstm: &Tensor, buckets: &[usize]) -> Result<Tensor> {
        let stm_embed = stm.apply(&self.embedding)?.relu()?;
        let nstm_embed = nstm.apply(&self.embedding)?.relu()?;
        let embedding_out = Tensor::cat(&[stm_embed, nstm_embed], 1)?;

        // Compute all bucket outputs: [batch, OUTPUT_BUCKETS]
        let all_outputs: Vec<_> = self
            .buckets
            .iter()
            .map(|b| b.forward(&embedding_out))
            .collect::<Result<_>>()?;
        let stacked = Tensor::cat(&all_outputs, 1)?;

        // Select each sample's bucket output: [batch, 1]
        let indices = Tensor::from_vec(
            buckets.iter().map(|&i| i as u32).collect::<Vec<_>>(),
            (buckets.len(), 1),
            stacked.device(),
        )?;
        stacked.gather(&indices, 1)
    }
}

/// Hidden layers and output head for a single game phase.
pub struct OutputStack {
    pub hidden1: Linear,
    pub hidden2: Linear,
    pub output: Linear,
}

impl OutputStack {
    fn forward(&self, input: &Tensor) -> Result<Tensor> {
        let h1 = input.apply(&self.hidden1)?.relu()?;
        let h2 = (h1.apply(&self.hidden2)? + &h1)?.relu()?;
        h2.apply(&self.output)
    }
}
