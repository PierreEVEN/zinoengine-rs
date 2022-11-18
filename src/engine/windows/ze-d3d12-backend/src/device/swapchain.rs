use crate::utils::SendableIUnknown;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use windows::Win32::Graphics::Dxgi::IDXGISwapChain3;
use ze_gfx::backend::Texture;

pub(crate) struct D3D12SwapChain {
    pub swapchain: SendableIUnknown<IDXGISwapChain3>,
    pub textures: Vec<Arc<Texture>>,
    pub need_restart: AtomicBool,
}

impl D3D12SwapChain {
    pub fn new(swapchain: SendableIUnknown<IDXGISwapChain3>, textures: Vec<Arc<Texture>>) -> Self {
        Self {
            swapchain,
            textures,
            need_restart: AtomicBool::new(true),
        }
    }
}
