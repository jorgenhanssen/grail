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

    /// Training forward pass. Runs every bucket and then gathers the one each
    /// sample actually wants (unused buckets get zero gradient through gather).
    /// stm/nstm are the position encoded from each side, output is in stm
    /// space so the caller has to sign-flip if they want it as white.
    pub fn forward(&self, stm: &Tensor, nstm: &Tensor, buckets: &[usize]) -> Result<Tensor> {
        let stm_embed = stm.apply(&self.embedding)?.relu()?;
        let nstm_embed = nstm.apply(&self.embedding)?.relu()?;
        let embedding_out = Tensor::cat(&[stm_embed, nstm_embed], 1)?;

        let all_outputs: Vec<_> = self
            .buckets
            .iter()
            .map(|b| b.forward(&embedding_out))
            .collect::<Result<_>>()?;
        let stacked = Tensor::cat(&all_outputs, 1)?;

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
