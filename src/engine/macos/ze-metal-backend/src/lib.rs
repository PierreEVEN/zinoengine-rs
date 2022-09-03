use device::MetalDevice;
use std::sync::Arc;
use ze_gfx::backend::{Backend, BackendError};

pub struct MetalBackend {}

impl MetalBackend {
    pub fn new() -> Result<Arc<MetalBackend>, BackendError> {
        Ok(Arc::new(MetalBackend {}))
    }
}

impl Backend for MetalBackend {
    fn create_device(
        &self,
    ) -> Result<std::sync::Arc<dyn ze_gfx::backend::Device>, ze_gfx::backend::BackendError> {
        let device = if let Some(device) = metal::Device::system_default() {
            device
        } else {
            return Err(BackendError::Unsupported);
        };

        Ok(Arc::new(MetalDevice::new(device)))
    }

    fn name(&self) -> &str {
        "Metal"
    }
}

pub mod device;
