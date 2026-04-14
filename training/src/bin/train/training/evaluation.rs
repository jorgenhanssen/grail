use candle_core::{Device, Tensor};
use nnue::encoding::NUM_FEATURES;
use nnue::network::Network;
use std::error::Error;

use crate::dataset::DataLoader;
use crate::utils::loss::{wdl_eval_loss, wdl_weights};

pub fn evaluate(
    network: &Network,
    loader: DataLoader,
    device: &Device,
    wdl_start: f64,
    wdl_end: f64,
) -> Result<f32, Box<dyn Error>> {
    let mut total_loss = 0.0;
    let mut batches = 0;

    for batch in loader {
        let batch_len = batch.scores.len();
        if batch_len == 0 {
            continue;
        }

        let x = Tensor::from_vec(batch.features, (batch_len, NUM_FEATURES), device)?;
        let y_eval = Tensor::from_vec(batch.scores, (batch_len, 1), device)?;
        let y_outcome = Tensor::from_vec(batch.outcomes, (batch_len, 1), device)?;
        let w = wdl_weights(&batch.distance_to_end, wdl_start, wdl_end, device)?;

        let preds = network.forward(&x, &batch.buckets)?;
        let loss = wdl_eval_loss(&preds, &y_eval, &y_outcome, &w)?;

        total_loss += loss.to_vec0::<f32>()?;
        batches += 1;
    }

    Ok(total_loss / batches.max(1) as f32)
}
