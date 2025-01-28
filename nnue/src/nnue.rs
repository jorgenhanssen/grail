use candle_core::{Result, Tensor};
use candle_nn::{linear, Linear, VarBuilder};

// A simple MLP struct with 4 linear layers
pub struct NNUE {
    l1: Linear,
    l2: Linear,
    l3: Linear,
    l4: Linear,
}

impl NNUE {
    // Constructor: create each linear layer
    pub fn new(vs: &VarBuilder, in_features: usize) -> Result<Self> {
        // Candle automatically initializes parameters (weights/biases) inside `VarBuilder`.
        let l1: Linear = linear(in_features, 256, vs.pp("l1"))?;
        let l2: Linear = linear(256, 128, vs.pp("l2"))?;
        let l3: Linear = linear(128, 128, vs.pp("l3"))?;
        let l4: Linear = linear(128, 1, vs.pp("l4"))?;
        Ok(Self { l1, l2, l3, l4 })
    }

    // Forward pass: x -> relu -> relu -> relu -> tanh
    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let x = x.apply(&self.l1)?.relu()?;
        let x = x.apply(&self.l2)?.relu()?;
        let x = x.apply(&self.l3)?.relu()?;
        // Final layer, then Tanh
        let x = x.apply(&self.l4)?;
        x.tanh()
    }
}
