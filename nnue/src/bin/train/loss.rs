use candle_core::{Result as CandleResult, Tensor};
use nnue::samples::FV_SCALE;

const HUBER_THRESHOLD_CP: f64 = 400.0;
const HUBER_DELTA: f64 = HUBER_THRESHOLD_CP / FV_SCALE as f64;

pub fn huber(pred: &Tensor, eval_target: &Tensor) -> CandleResult<Tensor> {
    let diff = (pred - eval_target)?;
    let abs_diff = diff.abs()?;

    let is_small = abs_diff.lt(HUBER_DELTA)?;

    let quadratic = (diff.sqr()? * 0.5)?;
    let linear = ((abs_diff - 0.5 * HUBER_DELTA)? * HUBER_DELTA)?;

    let loss = is_small.where_cond(&quadratic, &linear)?;
    loss.mean_all()
}
