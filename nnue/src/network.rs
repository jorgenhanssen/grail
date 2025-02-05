use candle_core::{Result, Tensor};
use candle_nn::{linear, Linear, Module, VarBuilder};

use crate::encoding::NUM_FEATURES;

const EMBEDDING_SIZE: usize = 256;
const HIDDEN_SIZE: usize = 32;

pub struct Network {
    embedding: Linear,
    hidden1: Linear,
    hidden2: Linear,
    output: Linear,
}

impl Network {
    pub fn new(vs: &VarBuilder) -> Result<Self> {
        let network = Self {
            embedding: linear(NUM_FEATURES, EMBEDDING_SIZE, vs.pp("embedding"))?,

            hidden1: linear(EMBEDDING_SIZE, HIDDEN_SIZE, vs.pp("hidden1"))?,
            hidden2: linear(HIDDEN_SIZE, HIDDEN_SIZE, vs.pp("hidden2"))?,

            output: linear(HIDDEN_SIZE, 1, vs.pp("output"))?,
        };

        Ok(network)
    }
}

impl Module for Network {
    #[inline]
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        // Embedding
        let mut x = x.apply(&self.embedding)?.relu()?;

        // Hidden layers
        x = x.apply(&self.hidden1)?.relu()?;
        x = x.apply(&self.hidden2)?.relu()?;

        // Output layer
        x = x.apply(&self.output)?;

        Ok(x)
    }
}
