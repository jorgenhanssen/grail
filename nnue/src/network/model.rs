use candle_core::{Result, Tensor};
use candle_nn::{Linear, VarBuilder, linear};

use crate::encoding::NUM_FEATURES;
use crate::kernels::embedding;

use super::{EMBEDDING_SIZE, HIDDEN_SIZE, OUTPUT_BUCKETS, PAIRWISE_OUT_SIZE};

/// Full-precision network for training and weight loading.
///
/// A single embedding layer is run over both perspectives. Each side is
/// pairwise-multiplied, then the two halves are concatenated [...stm, ...nstm]
/// before being fed to the phase-specific hidden stack.
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

    pub fn forward(
        &self,
        stm_indices: &Tensor,
        nstm_indices: &Tensor,
        buckets: &[usize],
    ) -> Result<Tensor> {
        let embedding_out = embedding(stm_indices, nstm_indices, &self.embedding)?;
        self.buckets
            .forward(&pairwise_perspectives(&embedding_out)?, buckets)
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
                hidden1: linear(2 * PAIRWISE_OUT_SIZE, HIDDEN_SIZE, bvs.pp("hidden1")).unwrap(),
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
        let h1 = embedding_out
            .apply(&self.hidden1_for_all_buckets()?)?
            .relu()?;

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

/// Pairwise-multiply each perspective of a concatenated [...stm, ...nstm]
/// embedding. The embedding kernel already ReLUs, so clamp here adds the
/// CReLU upper bound of 1 before the split-and-multiply.
fn pairwise_perspectives(embedding_out: &Tensor) -> Result<Tensor> {
    let stm = embedding_out.narrow(1, 0, EMBEDDING_SIZE)?;
    let nstm = embedding_out.narrow(1, EMBEDDING_SIZE, EMBEDDING_SIZE)?;
    Tensor::cat(&[pairwise_mul(&stm)?, pairwise_mul(&nstm)?], 1)
}

/// Pairwise multiplication: clamp each lane to [0, 1], split into two equal
/// parts, multiply corresponding elements.
/// <https://www.chessprogramming.org/NNUE#Pairwise_Multiplication>
fn pairwise_mul(embedding: &Tensor) -> Result<Tensor> {
    let activated = embedding.clamp(0.0f32, 1.0f32)?;
    let first = activated.narrow(1, 0, PAIRWISE_OUT_SIZE)?;
    let second = activated.narrow(1, PAIRWISE_OUT_SIZE, PAIRWISE_OUT_SIZE)?;
    &first * &second
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
        let h2 = (h1.apply(&self.hidden2)? + h1)?.relu()?;
        h2.apply(&self.output)
    }
}
