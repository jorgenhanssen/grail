use candle_core::Device;
use std::error::Error;

pub fn get_device() -> Result<Device, Box<dyn Error>> {
    #[cfg(feature = "cuda")]
    if let Ok(device) = Device::cuda_if_available(0) {
        if device.is_cuda() {
            // I found that reducing precision squeezes a bit of training speed.
            // Since inference quantization is imprecise anyways, we might as well.
            // https://developer.nvidia.com/blog/accelerating-ai-training-with-tf32-tensor-cores/
            candle_core::cuda::set_gemm_reduced_precision_f32(true);
            return Ok(device);
        }
    }
    #[cfg(feature = "metal")]
    if let Ok(device) = Device::new_metal(0) {
        if device.is_metal() {
            return Ok(device);
        }
    }
    Ok(Device::Cpu)
}
