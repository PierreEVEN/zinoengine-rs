extern crate core;

pub mod backend;
mod command_manager;
mod descriptor_manager;
pub mod device;
mod frame_manager;
mod pipeline_manager;
mod pix;
mod resource_manager;
pub mod utils;

#[macro_export]
macro_rules! ze_d3d12_agility_sdk_statics {
    () => {
        #[no_mangle]
        #[used]
        pub static D3D12SDKVersion: u32 = 602;

        #[no_mangle]
        #[used]
        pub static D3D12SDKPath: &[u8; 9] = b".\\D3D12\\\0";
    };
}
