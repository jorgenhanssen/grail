use candle_core::{Device, Tensor};
use candle_nn::loss::mse;
use candle_nn::Module;
use nnue::encoding::NUM_FEATURES;
use nnue::network::Network;
use std::error::Error;

use crate::dataset::DataLoader;

pub fn evaluate(
    network: &Network,
    loader: DataLoader,
    device: &Device,
) -> Result<f32, Box<dyn Error>> {
    let mut total_loss = 0.0;
    let mut batches = 0;

    for (features, scores) in loader {
        let batch_len = scores.len();
        if batch_len == 0 {
            continue;
        }

        let x = Tensor::from_vec(features, (batch_len, NUM_FEATURES), device)?;
        let y = Tensor::from_vec(scores, (batch_len, 1), device)?;

        let preds = network.forward(&x)?;
        let loss = mse(&preds, &y)?;

        total_loss += loss.to_vec0::<f32>()?;
        batches += 1;
    }

    Ok(total_loss / batches.max(1) as f32)
}
