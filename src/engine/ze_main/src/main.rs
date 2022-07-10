use image::ImageBuffer;
use lazy_static::lazy_static;
use std::borrow::Borrow;
use std::mem::size_of;
use std::ops::Deref;
use std::path::Path;
use std::slice;
use std::sync::Arc;
use std::time::{Duration, Instant};
use ze_core::logger::StdoutSink;
use ze_core::maths::{RectI32, Vec2, Vec2f32, Vec4f32};
use ze_core::{logger, thread, ze_info};
use ze_d3d12_backend::backend::D3D12Backend;
use ze_d3d12_backend::utils;
use ze_d3d12_shader_compiler::D3D12ShaderCompiler;
use ze_filesystem::mount_points::StdMountPoint;
use ze_filesystem::FileSystem;
use ze_gfx::backend::{
    Backend, BufferDesc, BufferSRV, BufferUsageFlagBits, BufferUsageFlags, ClearValue,
    IndexBufferFormat, MemoryLocation, QueueType, RenderPassDesc, RenderPassTexture,
    RenderPassTextureLoadMode, RenderPassTextureStoreMode, RenderTargetViewDesc,
    RenderTargetViewType, ResourceBarrier, ResourceState, ResourceTransitionBarrier,
    ResourceTransitionBarrierResource, SamplerDesc, ShaderResourceViewDesc,
    ShaderResourceViewResource, ShaderResourceViewType, SwapChainDesc, Texture2DRTV, Texture2DSRV,
    TextureDesc, TextureUsageFlagBits, TextureUsageFlags, Viewport,
};
use ze_gfx::utils::{copy_data_to_buffer, copy_data_to_texture};
use ze_gfx::{PixelFormat, SampleDesc};
use ze_jobsystem::JobSystem;
use ze_platform::{Message, Platform, Window, WindowFlagBits, WindowFlags};
use ze_shader_system::{Shader, ShaderManager};
use ze_windows_platform::WindowsPlatform;

#[repr(C)]
struct Vertex {
    pub position: Vec2f32,
    pub color: Vec4f32,
    pub uv: Vec2f32,
}

lazy_static! {
    static ref VERTICES: [Vertex; 4] = [
        Vertex {
            position: Vec2f32::new(0.5, 0.5),
            color: Vec4f32::new(1.0, 0.0, 0.0, 1.0),
            uv: Vec2f32::new(0.0, 0.0),
        },
        Vertex {
            position: Vec2f32::new(-0.5, 0.5),
            color: Vec4f32::new(0.0, 1.0, 0.0, 1.0),
            uv: Vec2f32::new(1.0, 0.0),
        },
        Vertex {
            position: Vec2f32::new(-0.5, -0.5),
            color: Vec4f32::new(0.0, 0.0, 1.0, 1.0),
            uv: Vec2f32::new(1.0, 1.0),
        },
        Vertex {
            position: Vec2f32::new(0.5, -0.5),
            color: Vec4f32::new(1.0, 1.0, 0.0, 1.0),
            uv: Vec2f32::new(0.0, 1.0),
        },
    ];
    static ref INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];
}

fn main() {
    thread::set_thread_name(std::thread::current().id(), "Main Thread".to_string());
    logger::register_sink(Box::new(StdoutSink::new()));

    let platform = WindowsPlatform::new();

    let screen_0_bounds = platform.get_monitor(0).bounds;
    let window = platform
        .create_window(
            "Coucou",
            1280,
            720,
            (screen_0_bounds.width / 2) - (1280 / 2),
            (screen_0_bounds.height / 2) - (720 / 2),
            WindowFlags::from_flag(WindowFlagBits::Resizable),
        )
        .unwrap();

    let backend = D3D12Backend::new().unwrap();
    let device = backend.create_device().unwrap();
    let jobsystem = JobSystem::new(JobSystem::get_cpu_thread_count());
    let filesystem = FileSystem::new();
    let shader_compiler = D3D12ShaderCompiler::new(filesystem.clone());
    filesystem.mount(StdMountPoint::new(Path::new("./")));

    let shader_system =
        ShaderManager::new(device.clone(), jobsystem.clone(), shader_compiler.clone());
    shader_system.search_shaders(&filesystem, "assets/shaders".as_ref());

    let mut swapchain = Arc::new(
        device
            .create_swapchain(
                &SwapChainDesc {
                    width: window.get_width(),
                    height: window.get_height(),
                    format: PixelFormat::R8G8B8A8Unorm,
                    sample_desc: SampleDesc::default(),
                    usage_flags: TextureUsageFlags::from_flag(TextureUsageFlagBits::RenderTarget),
                    window_handle: window.get_handle(),
                },
                None,
            )
            .unwrap(),
    );

    let vertex_buffer = Arc::new(
        device
            .create_buffer(
                &BufferDesc {
                    size_bytes: (VERTICES.len() * size_of::<Vertex>()) as u64,
                    usage: BufferUsageFlags::from_flag(BufferUsageFlagBits::UnorderedAccess),
                    memory_location: MemoryLocation::GpuOnly,
                    default_resource_state: ResourceState::Common,
                },
                "",
            )
            .unwrap(),
    );

    let vertex_buffer_srv = device
        .create_shader_resource_view(&ShaderResourceViewDesc {
            resource: ShaderResourceViewResource::Buffer(vertex_buffer.clone()),
            format: PixelFormat::Unknown,
            ty: ShaderResourceViewType::Buffer(BufferSRV {
                first_element_index: 0,
                element_count: VERTICES.len() as u32,
                element_size_in_bytes: size_of::<Vertex>() as u32,
            }),
        })
        .unwrap();

    let index_buffer = device
        .create_buffer(
            &BufferDesc {
                size_bytes: (INDICES.len() * size_of::<u32>()) as u64,
                usage: BufferUsageFlags::from_flag(BufferUsageFlagBits::IndexBuffer),
                memory_location: MemoryLocation::GpuOnly,
                default_resource_state: ResourceState::Common,
            },
            "",
        )
        .unwrap();

    unsafe {
        copy_data_to_buffer(
            &device,
            &vertex_buffer,
            slice::from_raw_parts(
                VERTICES.as_ptr() as *const u8,
                VERTICES.len() * size_of::<Vertex>(),
            ),
            ResourceState::Common,
        )
        .unwrap();
    }

    unsafe {
        copy_data_to_buffer(
            &device,
            &index_buffer,
            slice::from_raw_parts(
                INDICES.as_ptr() as *const u8,
                INDICES.len() * size_of::<u32>(),
            ),
            ResourceState::Common,
        )
        .unwrap();
    }

    let mut running = true;

    let mut swapchain_render_target_views = vec![];
    for i in 0..device.get_swapchain_backbuffer_count(&swapchain) {
        swapchain_render_target_views.push(
            device
                .create_render_target_view(&RenderTargetViewDesc {
                    resource: device
                        .get_swapchain_backbuffer(&swapchain, i as u32)
                        .unwrap(),
                    format: PixelFormat::R8G8B8A8Unorm,
                    ty: RenderTargetViewType::Texture2D(Texture2DRTV { mip_level: 0 }),
                })
                .unwrap(),
        );
    }

    let image_data = image::open("assets/vald.png").unwrap();

    let texture = Arc::new(
        device
            .create_texture(
                &TextureDesc {
                    width: image_data.width(),
                    height: image_data.height(),
                    depth: 1,
                    mip_levels: 1,
                    format: PixelFormat::R8G8B8A8Unorm,
                    sample_desc: Default::default(),
                    usage_flags: TextureUsageFlags::default(),
                    memory_location: MemoryLocation::GpuOnly,
                },
                "",
            )
            .unwrap(),
    );

    let mut texture_srv = device
        .create_shader_resource_view(&ShaderResourceViewDesc {
            resource: ShaderResourceViewResource::Texture(texture.clone()),
            format: PixelFormat::R8G8B8A8Unorm,
            ty: ShaderResourceViewType::Texture2D(Texture2DSRV {
                min_mip_level: 0,
                mip_levels: 1,
            }),
        })
        .unwrap();

    copy_data_to_texture(
        &device,
        image_data.as_bytes(),
        &texture,
        ResourceState::Common,
    )
    .unwrap();

    let sampler = device.create_sampler(&SamplerDesc::default()).unwrap();

    #[repr(C)]
    struct PushConstant {
        vertex_buffer_idx: u32,
        vald_texture_idx: u32,
        sampler_texture_idx: u32,
        time: f32,
    }
    let mut push_constant = PushConstant {
        vertex_buffer_idx: vertex_buffer_srv.get_descriptor_index(),
        vald_texture_idx: texture_srv.get_descriptor_index(),
        sampler_texture_idx: sampler.get_descriptor_index(),
        time: 0.0,
    };

    let mut imgui = ze_imgui::Context::new(
        device.clone(),
        shader_system.clone(),
        platform.clone(),
        window.clone(),
    );
    imgui.update_monitors();

    let mut previous = Instant::now();
    while running {
        let delta_time = previous.elapsed().as_secs_f32();
        previous = Instant::now();

        let mut need_call_new_frame = true;
        while let Some(message) = platform.poll_event() {
            imgui.send_platform_message(&message);
            match message {
                Message::WindowClosed(_) => {
                    running = false;
                }
                Message::WindowResized(event_window, width, height) => {
                    device.wait_idle();
                    if event_window.as_ptr() != window.as_ref() as *const dyn Window {
                        break;
                    }
                    //device.begin_frame();
                    //need_call_new_frame = false;
                    swapchain_render_target_views.clear();
                    swapchain = Arc::new(
                        device
                            .create_swapchain(
                                &SwapChainDesc {
                                    width,
                                    height,
                                    format: PixelFormat::R8G8B8A8Unorm,
                                    sample_desc: SampleDesc::default(),
                                    usage_flags: TextureUsageFlags::from_flag(
                                        TextureUsageFlagBits::RenderTarget,
                                    ),
                                    window_handle: window.get_handle(),
                                },
                                Some(unsafe { Arc::try_unwrap(swapchain).unwrap_unchecked() }),
                            )
                            .unwrap(),
                    );
                    for i in 0..device.get_swapchain_backbuffer_count(&swapchain) {
                        swapchain_render_target_views.push(
                            device
                                .create_render_target_view(&RenderTargetViewDesc {
                                    resource: device
                                        .get_swapchain_backbuffer(&swapchain, i as u32)
                                        .unwrap(),
                                    format: PixelFormat::R8G8B8A8Unorm,
                                    ty: RenderTargetViewType::Texture2D(Texture2DRTV {
                                        mip_level: 0,
                                    }),
                                })
                                .unwrap(),
                        );
                    }
                }
                _ => {}
            }
        }

        if need_call_new_frame {
            device.begin_frame();
        }

        let mut cmd_list = device.create_command_list(QueueType::Graphics).unwrap();
        let mut backbuffer_index = device.get_swapchain_backbuffer_index(&swapchain);
        imgui.begin(delta_time, platform.get_mouse_position(), &*window);
        imgui.window("My first window").begin();
        imgui.text("ZIEJFZIOEFJEZIOFJEZIOFJEZOIFJEZOIFJZIOJFEZIOJFZEOIFJ");
        imgui.end();

        imgui.window("bb je te baise").begin();
        imgui.text("ZIEJFZIOEFJEZIOFJEZIOFJEZOIFJEZOIFJZIOJFEZIOJFZEOIFJ");
        imgui.end();
        imgui.finish();

        push_constant.time += 0.001;

        let backbuffer = device
            .get_swapchain_backbuffer(&swapchain, backbuffer_index)
            .unwrap();

        device.cmd_resource_barrier(
            &mut cmd_list,
            &[ResourceBarrier::Transition(ResourceTransitionBarrier {
                resource: ResourceTransitionBarrierResource::Texture(&*backbuffer),
                source_state: ResourceState::Present,
                dest_state: ResourceState::RenderTargetWrite,
            })],
        );

        device.cmd_begin_render_pass(
            &mut cmd_list,
            &RenderPassDesc {
                render_targets: &[RenderPassTexture {
                    render_target_view: &swapchain_render_target_views[backbuffer_index as usize],
                    load_mode: RenderPassTextureLoadMode::Clear,
                    store_mode: RenderPassTextureStoreMode::Preserve,
                    clear_value: ClearValue::Color([0.0, 0.0, 0.0, 1.0]),
                }],
                depth_stencil: None,
            },
        );

        device.cmd_set_viewports(
            &mut cmd_list,
            &[Viewport {
                position: Vec2f32::default(),
                size: Vec2f32::new(window.get_width() as f32, window.get_height() as f32),
                min_depth: 0.0,
                max_depth: 1.0,
            }],
        );

        device.cmd_set_scissors(
            &mut cmd_list,
            &[RectI32::new(
                0,
                0,
                window.get_width() as i32,
                window.get_height() as i32,
            )],
        );

        let shader = shader_system.get_shader_modules(&"Test".to_string(), None);
        if let Ok(shader) = shader {
            let stages = shader.get_pipeline_stages();
            device.cmd_set_shader_stages(&mut cmd_list, &stages);
            device.cmd_bind_index_buffer(&mut cmd_list, &index_buffer, IndexBufferFormat::Uint32);
            unsafe {
                device.cmd_push_constants(
                    &mut cmd_list,
                    0,
                    slice::from_raw_parts(
                        (&push_constant as *const PushConstant) as *const u8,
                        4 * 4,
                    ),
                );
            }
            device.cmd_draw_indexed(&mut cmd_list, INDICES.len() as u32, 1, 0, 0);
        }

        imgui.draw_viewport(&mut cmd_list, imgui.get_main_viewport());
        device.cmd_end_render_pass(&mut cmd_list);

        device.cmd_resource_barrier(
            &mut cmd_list,
            &[ResourceBarrier::Transition(ResourceTransitionBarrier {
                resource: ResourceTransitionBarrierResource::Texture(&*backbuffer),
                source_state: ResourceState::RenderTargetWrite,
                dest_state: ResourceState::Present,
            })],
        );

        imgui.draw_non_main_viewports(&mut cmd_list);
        device.submit(QueueType::Graphics, &[&cmd_list], &[], &[]);
        device.present(&swapchain);
        imgui.present();

        device.end_frame();
    }
}
