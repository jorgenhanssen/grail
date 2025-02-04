use candle_core::{Result, Tensor};
use candle_nn::{linear, Linear, Module, VarBuilder};

use crate::encoding::NUM_FEATURES;

pub struct Network {
    embedding: Linear,
    hidden1: Linear,
    hidden2: Linear,
    output: Linear,
}

impl Network {
    pub fn new(vs: &VarBuilder) -> Result<Self> {
        let network = Self {
            embedding: linear(NUM_FEATURES, 128, vs.pp("embedding"))?,
            hidden1: linear(128, 32, vs.pp("hidden1"))?,
            hidden2: linear(32, 32, vs.pp("hidden2"))?,
            output: linear(32, 1, vs.pp("output"))?,
        };

        Ok(network)
    }
}

impl Module for Network {
    #[inline]
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        // Embedding
        let mut x = x.apply(&self.embedding)?.clamp(0.0, 1.0)?; // Clipped ReLU

        // Hidden layers
        x = x.apply(&self.hidden1)?.relu()?;
        x = x.apply(&self.hidden2)?.relu()?;

        // Output layer
        x = x.apply(&self.output)?.tanh()?;

        Ok(x)
    }
}
