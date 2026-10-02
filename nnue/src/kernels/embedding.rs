// Beware! The following code was assisted with an LLM due to skill issue.
// If you are allergic to such code, close this file immediately.

use candle_core::{CpuStorage, CustomOp3, Layout, Result, Shape, Tensor, WithDType};
use candle_nn::Linear;

use crate::network::EMBEDDING_SIZE;

#[cfg(feature = "cuda")]
use candle_core::CudaStorage;
#[cfg(feature = "cuda")]
use candle_core::cuda::cudarc::driver::{CudaView, LaunchConfig, PushKernelArg};
#[cfg(feature = "cuda")]
use candle_core::cuda_backend::WrapErr;

#[cfg(feature = "cuda")]
use super::cuda_kernels;

// Must match kBlockSize and kOutputsPerThread in embedding.cu.
// Each thread owns OUTPUTS_PER_THREAD neurons, spaced one block apart.
const BLOCK_SIZE: u32 = 256;
const OUTPUTS_PER_THREAD: usize = 3;
const _: () = assert!(EMBEDDING_SIZE == BLOCK_SIZE as usize * OUTPUTS_PER_THREAD);

/// The output contains stm first, followed by nstm.
pub fn embedding(stm_indices: &Tensor, nstm_indices: &Tensor, layer: &Linear) -> Result<Tensor> {
    // The kernel needs each feature's weights together, but Linear stores them by neuron.
    // Rebuild this copy each call to pick up optimizer updates. Candle carries the
    // gradient back through the transpose to the original weight.
    let weights = layer.weight().t()?.contiguous()?;
    let hidden = stm_indices.apply_op3(nstm_indices, &weights, Embedding)?;
    let batch_len = hidden.dim(0)?;

    // Candle handles the shared bias gradient and the ReLU mask.
    let hidden = match layer.bias() {
        Some(bias) => hidden.broadcast_add(bias)?,
        None => hidden,
    };
    hidden.relu()?.reshape((batch_len, 2 * EMBEDDING_SIZE))
}

struct Embedding;

impl CustomOp3 for Embedding {
    fn name(&self) -> &'static str {
        "embedding"
    }

    fn cpu_fwd(
        &self,
        stm: &CpuStorage,
        stm_layout: &Layout,
        nstm: &CpuStorage,
        nstm_layout: &Layout,
        weights: &CpuStorage,
        weights_layout: &Layout,
    ) -> Result<(CpuStorage, Shape)> {
        let dims = forward_dims(stm_layout, nstm_layout, weights_layout)?;
        let stm = cpu_values::<u32>(stm, stm_layout)?;
        let nstm = cpu_values::<u32>(nstm, nstm_layout)?;
        let weights = cpu_values::<f32>(weights, weights_layout)?;

        let mut output = vec![0.0f32; dims.batch_len * 2 * EMBEDDING_SIZE];
        for perspective in 0..2 {
            let indices = if perspective == 0 { stm } else { nstm };
            for sample in 0..dims.batch_len {
                let row_start = sample * dims.indices_per_sample;
                let row = &indices[row_start..row_start + dims.indices_per_sample];
                let out_start = (sample * 2 + perspective) * EMBEDDING_SIZE;
                let out = &mut output[out_start..out_start + EMBEDDING_SIZE];
                for &index in row {
                    if index as usize >= dims.num_features {
                        break;
                    }
                    let weight_start = index as usize * EMBEDDING_SIZE;
                    let weight_row = &weights[weight_start..weight_start + EMBEDDING_SIZE];
                    for (neuron, weight) in out.iter_mut().zip(weight_row) {
                        *neuron += *weight;
                    }
                }
            }
        }

        Ok((CpuStorage::F32(output), dims.output_shape))
    }

    #[cfg(feature = "cuda")]
    fn cuda_fwd(
        &self,
        stm: &CudaStorage,
        stm_layout: &Layout,
        nstm: &CudaStorage,
        nstm_layout: &Layout,
        weights: &CudaStorage,
        weights_layout: &Layout,
    ) -> Result<(CudaStorage, Shape)> {
        let dims = forward_dims(stm_layout, nstm_layout, weights_layout)?;
        let device = weights.device.clone();
        let stm = cuda_view::<u32>(stm, stm_layout)?;
        let nstm = cuda_view::<u32>(nstm, nstm_layout)?;
        let weights = cuda_view::<f32>(weights, weights_layout)?;
        let elems = dims.batch_len * 2 * EMBEDDING_SIZE;
        let output = unsafe { device.alloc::<f32>(elems) }?;

        if dims.batch_len > 0 {
            let batch_len = u32_len(dims.batch_len)?;
            let indices_per_sample = u32_len(dims.indices_per_sample)?;
            let num_features = u32_len(dims.num_features)?;
            let func = device.get_or_load_custom_func(
                "embedding_fwd_f32",
                "embedding",
                cuda_kernels::EMBEDDING,
            )?;
            let mut builder = func.builder();
            builder.arg(&stm);
            builder.arg(&nstm);
            builder.arg(&weights);
            builder.arg(&output);
            candle_core::builder_arg!(builder, batch_len, indices_per_sample, num_features);
            let cfg = LaunchConfig {
                grid_dim: (batch_len, 2, 1),
                block_dim: (BLOCK_SIZE, 1, 1),
                shared_mem_bytes: 0,
            };
            unsafe { builder.launch(cfg) }.w()?;
        }

        Ok((
            CudaStorage::wrap_cuda_slice(output, device),
            dims.output_shape,
        ))
    }

    fn bwd(
        &self,
        stm_indices: &Tensor,
        nstm_indices: &Tensor,
        weights: &Tensor,
        _output: &Tensor,
        grad_output: &Tensor,
    ) -> Result<(Option<Tensor>, Option<Tensor>, Option<Tensor>)> {
        let (num_features, embedding_size) = weights.dims2()?;
        if embedding_size != EMBEDDING_SIZE {
            candle_core::bail!(
                "embedding weight width is {embedding_size}, expected {EMBEDDING_SIZE}"
            );
        }

        // The CUDA kernel reads packed rows, so the incoming gradient must be contiguous.
        let weight_grad = stm_indices.contiguous()?.apply_op3_no_bwd(
            &nstm_indices.contiguous()?,
            &grad_output.contiguous()?,
            &EmbeddingBwd { num_features },
        )?;
        Ok((None, None, Some(weight_grad)))
    }
}

// A second op gives the backward pass access to CPU or CUDA storage.
struct EmbeddingBwd {
    num_features: usize,
}

impl CustomOp3 for EmbeddingBwd {
    fn name(&self) -> &'static str {
        "embedding-bwd"
    }

    fn cpu_fwd(
        &self,
        stm: &CpuStorage,
        stm_layout: &Layout,
        nstm: &CpuStorage,
        nstm_layout: &Layout,
        grad_output: &CpuStorage,
        grad_layout: &Layout,
    ) -> Result<(CpuStorage, Shape)> {
        let dims = backward_dims(stm_layout, nstm_layout, grad_layout, self.num_features)?;
        let stm = cpu_values::<u32>(stm, stm_layout)?;
        let nstm = cpu_values::<u32>(nstm, nstm_layout)?;
        let grad_output = cpu_values::<f32>(grad_output, grad_layout)?;

        let mut weight_grad = vec![0.0f32; dims.num_features * EMBEDDING_SIZE];
        for perspective in 0..2 {
            let indices = if perspective == 0 { stm } else { nstm };
            for sample in 0..dims.batch_len {
                let row_start = sample * dims.indices_per_sample;
                let row = &indices[row_start..row_start + dims.indices_per_sample];
                let grad_start = (sample * 2 + perspective) * EMBEDDING_SIZE;
                let grad = &grad_output[grad_start..grad_start + EMBEDDING_SIZE];
                for &index in row {
                    if index as usize >= dims.num_features {
                        break;
                    }
                    let weight_start = index as usize * EMBEDDING_SIZE;
                    let weight_row = &mut weight_grad[weight_start..weight_start + EMBEDDING_SIZE];
                    for (neuron, upstream) in weight_row.iter_mut().zip(grad) {
                        *neuron += *upstream;
                    }
                }
            }
        }

        Ok((CpuStorage::F32(weight_grad), dims.weight_shape))
    }

    #[cfg(feature = "cuda")]
    fn cuda_fwd(
        &self,
        stm: &CudaStorage,
        stm_layout: &Layout,
        nstm: &CudaStorage,
        nstm_layout: &Layout,
        grad_output: &CudaStorage,
        grad_layout: &Layout,
    ) -> Result<(CudaStorage, Shape)> {
        let dims = backward_dims(stm_layout, nstm_layout, grad_layout, self.num_features)?;
        let device = grad_output.device.clone();
        let stm = cuda_view::<u32>(stm, stm_layout)?;
        let nstm = cuda_view::<u32>(nstm, nstm_layout)?;
        let grad_output = cuda_view::<f32>(grad_output, grad_layout)?;
        let weight_grad = device.alloc_zeros::<f32>(dims.num_features * EMBEDDING_SIZE)?;

        if dims.batch_len > 0 {
            let batch_len = u32_len(dims.batch_len)?;
            let indices_per_sample = u32_len(dims.indices_per_sample)?;
            let num_features = u32_len(dims.num_features)?;
            let func = device.get_or_load_custom_func(
                "embedding_bwd_f32",
                "embedding",
                cuda_kernels::EMBEDDING,
            )?;
            let mut builder = func.builder();
            builder.arg(&stm);
            builder.arg(&nstm);
            builder.arg(&grad_output);
            builder.arg(&weight_grad);
            candle_core::builder_arg!(builder, batch_len, indices_per_sample, num_features);
            let cfg = LaunchConfig {
                grid_dim: (batch_len, 2, 1),
                block_dim: (BLOCK_SIZE, 1, 1),
                shared_mem_bytes: 0,
            };
            unsafe { builder.launch(cfg) }.w()?;
        }

        Ok((
            CudaStorage::wrap_cuda_slice(weight_grad, device),
            dims.weight_shape,
        ))
    }
}

struct ForwardDims {
    batch_len: usize,
    indices_per_sample: usize,
    num_features: usize,
    output_shape: Shape,
}

struct BackwardDims {
    batch_len: usize,
    indices_per_sample: usize,
    num_features: usize,
    weight_shape: Shape,
}

fn forward_dims(stm: &Layout, nstm: &Layout, weights: &Layout) -> Result<ForwardDims> {
    let (batch_len, indices_per_sample) = stm.shape().dims2()?;
    let (nstm_batch, nstm_width) = nstm.shape().dims2()?;
    if nstm_batch != batch_len || nstm_width != indices_per_sample {
        candle_core::bail!("stm and nstm index tensors must have the same shape");
    }
    let (num_features, embedding_size) = weights.shape().dims2()?;
    if embedding_size != EMBEDDING_SIZE {
        candle_core::bail!("embedding weight width is {embedding_size}, expected {EMBEDDING_SIZE}");
    }
    Ok(ForwardDims {
        batch_len,
        indices_per_sample,
        num_features,
        output_shape: Shape::from((batch_len, 2, EMBEDDING_SIZE)),
    })
}

fn backward_dims(
    stm: &Layout,
    nstm: &Layout,
    grad: &Layout,
    num_features: usize,
) -> Result<BackwardDims> {
    let (batch_len, indices_per_sample) = stm.shape().dims2()?;
    let (nstm_batch, nstm_width) = nstm.shape().dims2()?;
    if nstm_batch != batch_len || nstm_width != indices_per_sample {
        candle_core::bail!("stm and nstm index tensors must have the same shape");
    }
    let &[grad_batch, perspectives, embedding_size] = grad.dims() else {
        candle_core::bail!("embedding gradient must have shape [batch, 2, {EMBEDDING_SIZE}]");
    };
    if grad_batch != batch_len || perspectives != 2 || embedding_size != EMBEDDING_SIZE {
        candle_core::bail!("embedding gradient must have shape [{batch_len}, 2, {EMBEDDING_SIZE}]");
    }
    Ok(BackwardDims {
        batch_len,
        indices_per_sample,
        num_features,
        weight_shape: Shape::from((num_features, EMBEDDING_SIZE)),
    })
}

// A contiguous view can still start partway through the underlying storage.
fn contiguous_range(layout: &Layout) -> Result<(usize, usize)> {
    match layout.contiguous_offsets() {
        Some(range) => Ok(range),
        None => candle_core::bail!("embedding tensor must be contiguous"),
    }
}

fn cpu_values<'a, T: WithDType>(storage: &'a CpuStorage, layout: &Layout) -> Result<&'a [T]> {
    let values = storage.as_slice::<T>()?;
    let (start, end) = contiguous_range(layout)?;
    Ok(&values[start..end])
}

#[cfg(feature = "cuda")]
fn u32_len(len: usize) -> Result<u32> {
    u32::try_from(len)
        .map_err(|_| candle_core::Error::Msg("embedding dimension does not fit in u32".into()).bt())
}

#[cfg(feature = "cuda")]
fn cuda_view<'a, T: candle_core::cuda_backend::CudaDType>(
    storage: &'a CudaStorage,
    layout: &Layout,
) -> Result<CudaView<'a, T>> {
    let slice = storage.as_cuda_slice::<T>()?;
    let (start, end) = contiguous_range(layout)?;
    Ok(slice.slice(start..end))
}
