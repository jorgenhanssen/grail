use candle_core::{Result, Tensor};
use candle_nn::loss::mse;
use candle_nn::ops::sigmoid;

/// Sigmoid MSE loss blending eval and game outcome in win-probability space.
///
/// target = wdl * outcome + (1 - wdl) * sigmoid(eval)
/// loss   = mean((sigmoid(output) - target)²)
pub fn wdl_eval_loss(
    net_output: &Tensor,
    target_eval: &Tensor,
    target_outcome: &Tensor,
    wdl: f64,
) -> Result<Tensor> {
    let sigmoid_output = sigmoid(net_output)?;
    let sigmoid_eval = sigmoid(target_eval)?;

    let target = ((target_outcome * wdl)? + (sigmoid_eval * (1.0 - wdl))?)?;

    mse(&sigmoid_output, &target)
}
