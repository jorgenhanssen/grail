use candle_core::{DType, Device, Result, Tensor};
use nnue::encoding::NUM_FEATURES;

/// Creates dense features from active indices.
pub fn dense_features(indices: Vec<u32>, batch_len: usize, device: &Device) -> Result<Tensor> {
    // indices are padded so we can just get the length of each sample by dividing by num batches.
    // Gotta do this since this varies between batches (max active features differ between samples)
    let indices_per_sample = indices.len() / batch_len;

    let indices = Tensor::from_vec(indices, (batch_len, indices_per_sample), device)?;
    let features = Tensor::zeros((batch_len, NUM_FEATURES), DType::F32, device)?;
    let ones = Tensor::ones((batch_len, indices_per_sample), DType::F32, device)?;

    // Candle's scatter leaves `u32::MAX` indices untouched so padding doesnt write.
    features.scatter_set(&indices, &ones, 1)?;
    Ok(features)
}
