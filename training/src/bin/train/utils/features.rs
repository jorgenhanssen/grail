use candle_core::{DType, Device, Result, Tensor};
use nnue::encoding::{MAX_ACTIVE_FEATURES, NUM_FEATURES};

/// Creates dense features from active indices.
pub fn dense_features(indices: Vec<u32>, batch_len: usize, device: &Device) -> Result<Tensor> {
    let indices = Tensor::from_vec(indices, (batch_len, MAX_ACTIVE_FEATURES), device)?;
    let features = Tensor::zeros((batch_len, NUM_FEATURES), DType::F32, device)?;
    let ones = Tensor::ones((batch_len, MAX_ACTIVE_FEATURES), DType::F32, device)?;

    // Candle's scatter leaves `u32::MAX` indices untouched so padding doesnt write.
    features.scatter_set(&indices, &ones, 1)?;
    Ok(features)
}
