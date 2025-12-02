// Huber loss transition point: L2 for small errors, L1 for large errors.
const HUBER_DELTA: f64 = 1.0;

/// Huber loss: smooth L1 that's less sensitive to outliers than MSE.
///
/// For |error| < delta: uses L2 (quadratic), giving strong gradients for small errors.
/// For |error| >= delta: uses L1 (linear), preventing large outliers from dominating.
///
/// Good for chess eval because some positions have extreme scores (near-mate)
/// that shouldn't dominate the loss compared to typical positions.
pub fn huber(
    pred: &candle_core::Tensor,
    target: &candle_core::Tensor,
) -> candle_core::Result<candle_core::Tensor> {
    let diff = (pred - target)?.abs()?;

    let mask = diff.lt(HUBER_DELTA)?;
    let mask = mask.to_dtype(candle_core::DType::F32)?;

    // L2 region: 0.5 * x^2
    let l2 = (diff.sqr()? * 0.5)?;
    // L1 region: delta * |x| - 0.5 * delta^2
    let l1 = ((diff * HUBER_DELTA)? - (0.5 * HUBER_DELTA * HUBER_DELTA))?;

    let inverted_mask = (1.0 - &mask)?;
    let term1 = mask.broadcast_mul(&l2)?;
    let term2 = inverted_mask.broadcast_mul(&l1)?;

    let loss = (term1 + term2)?;
    loss.mean_all()
}
