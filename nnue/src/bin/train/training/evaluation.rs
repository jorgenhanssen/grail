use candle_core::{Device, Tensor};
use nnue::encoding::NUM_FEATURES;
use nnue::network::Network;
use std::error::Error;

use crate::dataset::DataLoader;
use crate::utils::loss::{cross_entropy, huber};

pub fn evaluate(
    network: &Network,
    loader: DataLoader,
    device: &Device,
    policy_weight: f64,
) -> Result<f32, Box<dyn Error>> {
    let mut total_loss = 0.0;
    let mut batches = 0;

    for batch in loader {
        let batch_len = batch.scores.len();
        if batch_len == 0 {
            continue;
        }

        let x = Tensor::from_vec(batch.features, (batch_len, NUM_FEATURES), device)?;
        let y = Tensor::from_vec(batch.scores, (batch_len, 1), device)?;
        let policy_y = Tensor::from_vec(
            batch.policy_targets.iter().map(|&t| t as u32).collect::<Vec<_>>(),
            batch_len,
            device,
        )?;

        let (eval_preds, policy_logits) = network.forward(&x, &batch.buckets)?;
        let eval_loss = huber(&eval_preds, &y)?;
        let policy_loss = cross_entropy(&policy_logits, &policy_y)?;
        let loss = (eval_loss + (policy_loss * policy_weight)?)?;

        total_loss += loss.to_vec0::<f32>()?;
        batches += 1;
    }

    Ok(total_loss / batches.max(1) as f32)
}
