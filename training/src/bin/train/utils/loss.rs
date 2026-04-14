use candle_core::{Device, Result, Tensor};
use candle_nn::loss::mse;
use candle_nn::ops::sigmoid;

const WDL_DECAY: f64 = 0.05;

/// Per-sample WDL weight via exponential decay on distance-to-end.
pub fn wdl_weights(
    distance_to_end: &[u16],
    start: f64,
    end: f64,
    device: &Device,
) -> Result<Tensor> {
    let range = end - start;
    let weights: Vec<f32> = distance_to_end
        .iter()
        .map(|&d| (start + range * (-WDL_DECAY * d as f64).exp()) as f32)
        .collect();
    Tensor::from_vec(weights, (distance_to_end.len(), 1), device)
}

/// Sigmoid MSE loss blending eval and game outcome in win-probability space.
///
/// target = w * outcome + (1 - w) * sigmoid(eval)
/// loss   = mean((sigmoid(output) - target)²)
///
/// `wdl_weights` is a per-sample tensor of shape (batch, 1).
pub fn wdl_eval_loss(
    net_output: &Tensor,
    target_eval: &Tensor,
    target_outcome: &Tensor,
    wdl_weights: &Tensor,
) -> Result<Tensor> {
    let sigmoid_output = sigmoid(net_output)?;
    let sigmoid_eval = sigmoid(target_eval)?;

    let eval_weight = (1.0 - wdl_weights)?;
    let target = ((target_outcome * wdl_weights)? + (sigmoid_eval * eval_weight)?)?;

    mse(&sigmoid_output, &target)
}
