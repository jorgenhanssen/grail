use candle_core::Device;
use std::error::Error;

pub fn get_device() -> Result<Device, Box<dyn Error>> {
    #[cfg(feature = "cuda")]
    if let Ok(device) = Device::cuda_if_available(0) {
        if device.is_cuda() {
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
