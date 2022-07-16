use crate::device::D3D12Device;
use crate::utils::SendableIUnknown;
use parking_lot::Mutex;
use std::sync::Arc;
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_12_0;
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::*;
use ze_core::ze_info;
use ze_gfx::backend::*;

#[cfg(debug_assertions)]
const ENABLE_DEBUG_LAYERS: bool = true;

#[cfg(not(debug_assertions))]
const ENABLE_DEBUG_LAYERS: bool = false;

pub struct D3D12Backend {
    factory: Arc<Mutex<SendableIUnknown<IDXGIFactory4>>>,
}

impl D3D12Backend {
    pub fn new() -> Result<Arc<D3D12Backend>, BackendError> {
        // Create a debug controller if debug is enabled
        let debug_controller: Option<ID3D12Debug1> = unsafe {
            let mut debug: Option<ID3D12Debug> = None;
            if let Ok(_) = D3D12GetDebugInterface(&mut debug) {
                let debug = debug.unwrap();
                let controller: windows::core::Result<ID3D12Debug1> = debug.cast();
                match controller {
                    Ok(controller) => Some(controller),
                    Err(_) => None,
                }
            } else {
                None
            }
        };

        // Enable debug layers
        if let Some(debug) = debug_controller {
            unsafe {
                debug.EnableDebugLayer();
                debug.SetEnableGPUBasedValidation(true);
            }

            ze_info!("Using D3D12 debug layer");
        }

        // Create a DXGI factory to search for compatible adapters
        let factory: IDXGIFactory4 = unsafe {
            let mut flags = 0;
            if ENABLE_DEBUG_LAYERS {
                flags |= DXGI_CREATE_FACTORY_DEBUG;
            }
            match CreateDXGIFactory2::<IDXGIFactory4>(flags) {
                Ok(factory) => factory,
                Err(_) => return Err(BackendError::Unsupported),
            }
        };

        Ok(Arc::new(D3D12Backend {
            factory: Arc::new(Mutex::new(factory.into())),
        }))
    }
}

impl Drop for D3D12Backend {
    fn drop(&mut self) {
        if ENABLE_DEBUG_LAYERS {
            unsafe {
                if let Ok(debug) = DXGIGetDebugInterface1::<IDXGIDebug1>(0) {
                    debug
                        .ReportLiveObjects(
                            DXGI_DEBUG_ALL,
                            DXGI_DEBUG_RLO_FLAGS(
                                DXGI_DEBUG_RLO_DETAIL.0 | DXGI_DEBUG_RLO_IGNORE_INTERNAL.0,
                            ),
                        )
                        .unwrap();
                }
            }
        }
    }
}

impl Backend for D3D12Backend {
    fn create_device(&self) -> Result<Arc<dyn Device>, BackendError> {
        let factory = self.factory.lock();

        unsafe {
            // Search for a compatible adapter

            let mut adapter_index = 0;
            let mut adapter_to_use = None;

            loop {
                let adapter: IDXGIAdapter1 = match factory.EnumAdapters1(adapter_index) {
                    Ok(adapter) => adapter,
                    Err(_) => break,
                };

                let desc: DXGI_ADAPTER_DESC1 = adapter.GetDesc1().unwrap();
                if DXGI_ADAPTER_FLAG(desc.Flags) & DXGI_ADAPTER_FLAG_SOFTWARE
                    == DXGI_ADAPTER_FLAG_SOFTWARE
                {
                    break;
                }

                let adapter_name = String::from_utf16_lossy(&desc.Description);
                let adapter_name = adapter_name.trim_matches(char::from(0));
                ze_info!("Found compatible adapter: {}", adapter_name);

                adapter_to_use = Some(adapter);
                adapter_index += 1;
            }

            // Try create a device with this adapter
            if let Some(adapter) = adapter_to_use {
                let mut device: Option<ID3D12Device> = None;
                if let Ok(_) = D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_12_0, &mut device) {
                    let device = device.unwrap();
                    // Try also obtaining the debug device
                    let debug_device = {
                        if ENABLE_DEBUG_LAYERS {
                            let debug_device: windows::core::Result<ID3D12DebugDevice> =
                                device.cast();

                            match debug_device {
                                Ok(debug_device) => Some(debug_device.into()),
                                Err(_) => None,
                            }
                        } else {
                            None
                        }
                    };

                    Ok(Arc::new(D3D12Device::new(
                        self.factory.clone(),
                        device.into(),
                        debug_device,
                    )))
                } else {
                    Err(BackendError::Unsupported)
                }
            } else {
                Err(BackendError::Unsupported)
            }
        }
    }
}
