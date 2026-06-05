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
    pub buckets: OutputBuckets,
}

impl Network {
    pub fn new(vs: &VarBuilder) -> Result<Self> {
        Ok(Self {
            embedding: linear(NUM_FEATURES, EMBEDDING_SIZE, vs.pp("embedding"))?,
            buckets: OutputBuckets::new(vs)?,
        })
    }

    /// Training forward pass. stm/nstm are the position encoded from each side,
    /// output is in stm space so the caller has to sign-flip if they want it as
    /// white.
    pub fn forward(&self, stm: &Tensor, nstm: &Tensor, buckets: &[usize]) -> Result<Tensor> {
        let stm_embed = screlu(&stm.apply(&self.embedding)?)?;
        let nstm_embed = screlu(&nstm.apply(&self.embedding)?)?;
        let embedding_out = Tensor::cat(&[stm_embed, nstm_embed], 1)?;
        self.buckets.forward(&embedding_out, buckets)
    }
}

pub struct OutputBuckets {
    stacks: [OutputStack; OUTPUT_BUCKETS],
}

impl OutputBuckets {
    fn new(vs: &VarBuilder) -> Result<Self> {
        let stacks = std::array::from_fn(|i| {
            let bvs = vs.pp(format!("bucket_{}", i));
            OutputStack {
                hidden1: linear(2 * EMBEDDING_SIZE, HIDDEN_SIZE, bvs.pp("hidden1")).unwrap(),
                hidden2: linear(HIDDEN_SIZE, HIDDEN_SIZE, bvs.pp("hidden2")).unwrap(),
                output: linear(HIDDEN_SIZE, 1, bvs.pp("output")).unwrap(),
            }
        });
        Ok(Self { stacks })
    }

    /// Runs every bucket and then gathers the one each sample actually wants
    fn forward(&self, embedding_out: &Tensor, buckets: &[usize]) -> Result<Tensor> {
        // Since the buckets use the same embedding as their input we can
        // perform their multiplications in parallel.
        let h1 = crelu(&embedding_out.apply(&self.hidden1_for_all_buckets()?)?)?;

        // Buuut since the h1 => h2 have different inputs per bucket we kinda need
        // to split it up and compute each h2 separately.
        let scores: Vec<_> = self
            .stacks
            .iter()
            .enumerate()
            .map(|(i, stack)| {
                stack.finish(&h1.narrow(1, i * HIDDEN_SIZE, HIDDEN_SIZE)?.contiguous()?)
            })
            .collect::<Result<_>>()?;
        let scores = Tensor::cat(&scores, 1)?;

        // Finally we gather the scores for the buckets we want to compute gradients for.
        let indices = Tensor::from_vec(
            buckets.iter().map(|&i| i as u32).collect::<Vec<_>>(),
            (buckets.len(), 1),
            scores.device(),
        )?;
        scores.gather(&indices, 1)
    }

    fn hidden1_for_all_buckets(&self) -> Result<Linear> {
        let weights: Vec<_> = self.stacks.iter().map(|s| s.hidden1.weight()).collect();
        let biases: Vec<_> = self
            .stacks
            .iter()
            .map(|s| s.hidden1.bias().unwrap())
            .collect();

        Ok(Linear::new(
            Tensor::cat(&weights, 0)?,
            Some(Tensor::cat(&biases, 0)?),
        ))
    }

    pub fn get(&self, index: usize) -> &OutputStack {
        &self.stacks[index]
    }

    pub fn iter(&self) -> impl Iterator<Item = &OutputStack> {
        self.stacks.iter()
    }
}

/// Hidden layers and output head for a single game phase.
pub struct OutputStack {
    pub hidden1: Linear,
    pub hidden2: Linear,
    pub output: Linear,
}

impl OutputStack {
    /// Runs the individual bucket's layers that cannot be shared from
    /// the embedding.
    fn finish(&self, h1: &Tensor) -> Result<Tensor> {
        let h2 = crelu(&(h1.apply(&self.hidden2)? + h1)?)?;
        h2.apply(&self.output)
    }
}

fn crelu(x: &Tensor) -> Result<Tensor> {
    x.clamp(0f32, 1f32)
}

fn screlu(x: &Tensor) -> Result<Tensor> {
    x.clamp(0f32, 1f32)?.sqr()
}
