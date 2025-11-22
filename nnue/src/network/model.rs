use candle_core::{Result, Tensor};
use candle_nn::{linear, Linear, Module, VarBuilder};

use crate::encoding::NUM_FEATURES;

use super::{EMBEDDING_SIZE, HIDDEN_SIZE};

// Candle-compatible network definition (used for loading/training)
pub struct Network {
    pub(crate) embedding: Linear,
    pub(crate) hidden1: Linear,
    pub(crate) hidden2: Linear,
    pub(crate) output: Linear,
}

impl Network {
    pub fn new(vs: &VarBuilder) -> Result<Self> {
        Ok(Self {
            embedding: linear(NUM_FEATURES, EMBEDDING_SIZE, vs.pp("embedding"))?,
            hidden1: linear(EMBEDDING_SIZE, HIDDEN_SIZE, vs.pp("hidden1"))?,
            hidden2: linear(HIDDEN_SIZE, HIDDEN_SIZE, vs.pp("hidden2"))?,
            output: linear(HIDDEN_SIZE, 1, vs.pp("output"))?,
        })
    }
}

impl Module for Network {
    #[inline]
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let x = x.apply(&self.embedding)?.relu()?;
        let h1 = x.apply(&self.hidden1)?.relu()?;
        let h2 = (h1.apply(&self.hidden2)? + &h1)?.relu()?;
        let x = h2.apply(&self.output)?;
        Ok(x)
    }
}
