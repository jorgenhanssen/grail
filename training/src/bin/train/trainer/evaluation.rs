use candle_core::{Device, Tensor};
use nnue::network::Network;
use std::error::Error;

use crate::dataset::DataLoader;
use crate::utils::features::dense_features;
use crate::utils::loss::wdl_eval_loss;

pub fn evaluate(
    network: &Network,
    loader: DataLoader,
    device: &Device,
    wdl: f64,
) -> Result<f32, Box<dyn Error>> {
    let mut total_loss = 0.0;
    let mut batches = 0;

    for batch in loader {
        let batch_len = batch.scores.len();
        if batch_len == 0 {
            continue;
        }

        let stm = dense_features(batch.stm_features, batch_len, device)?;
        let nstm = dense_features(batch.nstm_features, batch_len, device)?;
        let y_eval = Tensor::from_vec(batch.scores, (batch_len, 1), device)?;
        let y_outcome = Tensor::from_vec(batch.outcomes, (batch_len, 1), device)?;

        let preds = network.forward(&stm, &nstm, &batch.buckets)?;
        let loss = wdl_eval_loss(&preds, &y_eval, &y_outcome, wdl)?;

        total_loss += loss.to_vec0::<f32>()?;
        batches += 1;
    }

    Ok(total_loss / batches.max(1) as f32)
}
