const HUBER_DELTA: f64 = 1.0;

pub fn huber(
    pred: &candle_core::Tensor,
    target: &candle_core::Tensor,
) -> candle_core::Result<candle_core::Tensor> {
    let diff = (pred - target)?.abs()?;

    let mask = diff.lt(HUBER_DELTA)?;
    let mask = mask.to_dtype(candle_core::DType::F32)?;

    let l2 = (diff.sqr()? * 0.5)?;
    let l1 = ((diff * HUBER_DELTA)? - (0.5 * HUBER_DELTA * HUBER_DELTA))?;

    let inverted_mask = (1.0 - &mask)?;
    let term1 = mask.broadcast_mul(&l2)?;
    let term2 = inverted_mask.broadcast_mul(&l1)?;

    let loss = (term1 + term2)?;
    loss.mean_all()
}
