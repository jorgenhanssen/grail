use candle_core::{Result, Tensor};
use candle_nn::{linear, Linear, Module, VarBuilder};

use crate::encoding::NUM_FEATURES;

pub struct Network {
    layers: Vec<Linear>,
    last_size: usize, // Track the last layer's output size
}

impl Network {
    pub fn new(vs: &VarBuilder) -> Result<Self> {
        let mut network = Self {
            layers: Vec::new(),
            last_size: NUM_FEATURES,
        };

        // Build the network architecture
        network.add_layer(128, vs)?;
        network.add_layer(64, vs)?;
        network.add_layer(1, vs)?;

        Ok(network)
    }

    // Helper method to add layers during construction
    fn add_layer(&mut self, out_size: usize, vs: &VarBuilder) -> Result<()> {
        let layer_num = self.layers.len() + 1;
        let layer = linear(self.last_size, out_size, vs.pp(&format!("l{}", layer_num)))?;
        self.layers.push(layer);
        self.last_size = out_size;
        Ok(())
    }
}

impl Module for Network {
    #[inline]
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let mut x = x.clone();

        // relu for all layers except the last one
        let last_idx = self.layers.len() - 1;
        for i in 0..last_idx {
            x = x.apply(&self.layers[i])?.relu()?;
        }

        x = x.apply(&self.layers[last_idx])?;
        x.tanh()
    }
}
