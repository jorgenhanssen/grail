use candle_core::{CpuStorage, CustomOp1, CustomOp2, Layout, Result, Shape, Tensor};

#[cfg(feature = "cuda")]
use candle_core::CudaStorage;
#[cfg(feature = "cuda")]
use candle_core::cuda::cudarc::driver::{LaunchConfig, PushKernelArg};
#[cfg(feature = "cuda")]
use candle_core::cuda_backend::WrapErr;

#[cfg(feature = "cuda")]
use super::cuda_kernels;

pub fn crelu(x: &Tensor) -> Result<Tensor> {
    x.contiguous()?.apply_op1(CRelu)
}

struct CRelu;
impl CustomOp1 for CRelu {
    fn name(&self) -> &'static str {
        "crelu"
    }

    fn cpu_fwd(&self, storage: &CpuStorage, layout: &Layout) -> Result<(CpuStorage, Shape)> {
        let slice = storage.as_slice::<f32>()?;
        let x = match layout.contiguous_offsets() {
            Some((o1, o2)) => &slice[o1..o2],
            None => candle_core::bail!("input has to be contiguous"),
        };
        let y = x.iter().map(|&x| x.clamp(0.0, 1.0)).collect::<Vec<f32>>();
        Ok((CpuStorage::F32(y), layout.shape().clone()))
    }

    #[cfg(feature = "cuda")]
    fn cuda_fwd(&self, storage: &CudaStorage, layout: &Layout) -> Result<(CudaStorage, Shape)> {
        let dev = storage.device.clone();
        let slice = storage.as_cuda_slice::<f32>()?;
        let x = match layout.contiguous_offsets() {
            Some((o1, o2)) => slice.slice(o1..o2),
            None => candle_core::bail!("input has to be contiguous"),
        };
        let n = layout.shape().elem_count();
        let y = unsafe { dev.alloc::<f32>(n) }?;

        let func = dev.get_or_load_custom_func("crelu_fwd_f32", "crelu", cuda_kernels::CRELU)?;
        let mut builder = func.builder();
        builder.arg(&x);
        builder.arg(&y);
        candle_core::builder_arg!(builder, n as i32);
        let cfg = LaunchConfig {
            grid_dim: ((n as u32).div_ceil(256).max(1), 1, 1),
            block_dim: (256, 1, 1),
            shared_mem_bytes: 0,
        };
        unsafe { builder.launch(cfg) }.w()?;

        Ok((CudaStorage::wrap_cuda_slice(y, dev), layout.shape().clone()))
    }

    fn bwd(&self, x: &Tensor, _y: &Tensor, grad: &Tensor) -> Result<Option<Tensor>> {
        Ok(Some(
            x.contiguous()?
                .apply_op2_no_bwd(&grad.contiguous()?, &CReluBwd)?,
        ))
    }
}

struct CReluBwd;
impl CustomOp2 for CReluBwd {
    fn name(&self) -> &'static str {
        "crelu_bwd"
    }

    fn cpu_fwd(
        &self,
        x: &CpuStorage,
        xl: &Layout,
        gy: &CpuStorage,
        gyl: &Layout,
    ) -> Result<(CpuStorage, Shape)> {
        let xs = x.as_slice::<f32>()?;
        let x = match xl.contiguous_offsets() {
            Some((o1, o2)) => &xs[o1..o2],
            None => candle_core::bail!("input has to be contiguous"),
        };
        let gs = gy.as_slice::<f32>()?;
        let gy = match gyl.contiguous_offsets() {
            Some((o1, o2)) => &gs[o1..o2],
            None => candle_core::bail!("input has to be contiguous"),
        };

        let gx = x
            .iter()
            .zip(gy)
            .map(|(&x, &gy)| if x > 0.0 && x < 1.0 { gy } else { 0.0 })
            .collect::<Vec<f32>>();
        Ok((CpuStorage::F32(gx), xl.shape().clone()))
    }

    #[cfg(feature = "cuda")]
    fn cuda_fwd(
        &self,
        x: &CudaStorage,
        xl: &Layout,
        gy: &CudaStorage,
        gyl: &Layout,
    ) -> Result<(CudaStorage, Shape)> {
        let dev = x.device.clone();
        let xs = x.as_cuda_slice::<f32>()?;
        let x = match xl.contiguous_offsets() {
            Some((o1, o2)) => xs.slice(o1..o2),
            None => candle_core::bail!("input has to be contiguous"),
        };
        let gs = gy.as_cuda_slice::<f32>()?;
        let gy = match gyl.contiguous_offsets() {
            Some((o1, o2)) => gs.slice(o1..o2),
            None => candle_core::bail!("input has to be contiguous"),
        };
        let n = xl.shape().elem_count();
        let gx = unsafe { dev.alloc::<f32>(n) }?;

        let func = dev.get_or_load_custom_func("crelu_bwd_f32", "crelu", cuda_kernels::CRELU)?;
        let mut builder = func.builder();
        builder.arg(&x);
        builder.arg(&gy);
        builder.arg(&gx);
        candle_core::builder_arg!(builder, n as i32);
        let cfg = LaunchConfig {
            grid_dim: ((n as u32).div_ceil(256).max(1), 1, 1),
            block_dim: (256, 1, 1),
            shared_mem_bytes: 0,
        };
        unsafe { builder.launch(cfg) }.w()?;

        Ok((CudaStorage::wrap_cuda_slice(gx, dev), xl.shape().clone()))
    }
}
