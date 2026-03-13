use candle_core::{Result, Tensor};
use candle_nn::loss::mse;
use candle_nn::ops::sigmoid;
use nnue::network::OUTPUT_BUCKETS;

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

/// Mean squared difference between adjacent bucket outputs.
/// Regularizes bucket transitions to be smooth across game phases.
pub fn bucket_smoothness_loss(all_buckets: &Tensor) -> Result<Tensor> {
    let left = all_buckets.narrow(1, 0, OUTPUT_BUCKETS - 1)?;
    let right = all_buckets.narrow(1, 1, OUTPUT_BUCKETS - 1)?;
    (left - right)?.sqr()?.mean_all()
}
