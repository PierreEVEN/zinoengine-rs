use std::env;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Weak};
use std::time::Instant;
use url::Url;
use ze_d3d12_backend::backend::D3D12Backend;
use ze_d3d12_shader_compiler::D3D12ShaderCompiler;
use ze_filesystem::mount_points::StdMountPoint;
use ze_filesystem::FileSystem;
use ze_gfx::backend::*;
use ze_gfx::PixelFormat;
use ze_imgui::Context;
use ze_jobsystem::JobSystem;
use ze_platform::{Message, Platform, Window, WindowFlagBits, WindowFlags};
use ze_render_graph::registry::PhysicalResourceTextureView;
use ze_render_graph::{RenderGraph, TextureInfo};
use ze_shader_compiler::ShaderCompiler;
use ze_shader_system::ShaderManager;
use ze_windows_platform::WindowsPlatform;

pub struct EditorApplication {
    platform: Arc<dyn Platform>,
    backend: Arc<dyn Backend>,
    device: Arc<dyn Device>,
    jobsystem: Arc<JobSystem>,
    filesystem: Arc<FileSystem>,
    shader_compiler: Arc<dyn ShaderCompiler>,
    shader_manager: Arc<ShaderManager>,
    main_window: Arc<dyn Window>,
    main_window_swapchain: Option<Arc<SwapChain>>,
    main_window_swapchain_rtvs: Vec<Arc<RenderTargetView>>,
    imgui: Box<Context>,
}

impl EditorApplication {
    pub fn new() -> Self {
        let platform = WindowsPlatform::new();
        let jobsystem = JobSystem::new(JobSystem::get_cpu_thread_count());
        let filesystem = FileSystem::new();
        filesystem.mount(StdMountPoint::new(
            "main",
            Path::new(&env::current_dir().unwrap()),
        ));

        let backend = D3D12Backend::new().expect("Failed to create graphics backend");
        let device = backend
            .create_device()
            .expect("Failed to create graphics device");

        let shader_compiler = D3D12ShaderCompiler::new(filesystem.clone());

        let shader_manager =
            ShaderManager::new(device.clone(), jobsystem.clone(), shader_compiler.clone());
        shader_manager.search_shaders(
            &filesystem,
            &Url::from_str("vfs:///assets/shaders").unwrap(),
        );

        let screen_0_bounds = platform.get_monitor(0).bounds;
        let main_window = platform
            .create_window(
                "ZinoEngine Editor",
                1280,
                720,
                (screen_0_bounds.width / 2) - (1280 / 2),
                (screen_0_bounds.height / 2) - (720 / 2),
                WindowFlags::from_flag(WindowFlagBits::Resizable),
            )
            .unwrap();

        let imgui = Context::new(
            device.clone(),
            shader_manager.clone(),
            platform.clone(),
            main_window.clone(),
        );

        Self {
            platform,
            backend,
            device,
            jobsystem,
            filesystem,
            shader_compiler,
            shader_manager,
            main_window,
            main_window_swapchain: None,
            main_window_swapchain_rtvs: vec![],
            imgui,
        }
    }

    pub fn run(&mut self) {
        self.update_main_window_swapchain();

        let mut running = true;
        let mut previous = Instant::now();

        while running {
            let delta_time = previous.elapsed().as_secs_f32();
            previous = Instant::now();

            while let Some(message) = self.platform.poll_event() {
                self.imgui.send_platform_message(&message);
                match message {
                    Message::WindowClosed(event_window) => {
                        if Weak::ptr_eq(&event_window, &Arc::downgrade(&self.main_window)) {
                            running = false;
                        }
                    }
                    Message::WindowResized(event_window, _, _) => {
                        if Weak::ptr_eq(&event_window, &Arc::downgrade(&self.main_window)) {
                            self.update_main_window_swapchain();
                        }
                    }
                    _ => {}
                }
            }

            self.device.begin_frame();

            self.imgui.begin_frame(
                delta_time,
                self.platform.get_mouse_position(),
                &*self.main_window,
            );

            self.imgui.end_frame();

            // Render

            let swapchain = self.main_window_swapchain.as_ref().unwrap();
            let backbuffer_index = self.device.get_swapchain_backbuffer_index(swapchain);

            let backbuffer = self
                .device
                .get_swapchain_backbuffer(swapchain, backbuffer_index)
                .unwrap();

            let mut main_cmd_list = self
                .device
                .create_command_list(QueueType::Graphics)
                .unwrap();

            let mut test = ze_render_graph::registry::PhysicalResourceRegistry::new();
            test.insert_or_update_existing_texture(
                "backbuffer",
                backbuffer.clone(),
                PhysicalResourceTextureView::RTV(
                    self.main_window_swapchain_rtvs[backbuffer_index as usize].clone(),
                ),
            );

            let mut render_graph = RenderGraph::new(self.device.clone(), &mut test);
            render_graph.add_graphics_pass(
                "imgui",
                |render_graph, render_pass| {
                    render_graph.add_pass_color_output(
                        render_pass,
                        "backbuffer",
                        TextureInfo::default(),
                    );
                },
                |_, cmd_list| {
                    self.imgui
                        .draw_viewport(cmd_list, self.imgui.get_main_viewport());
                },
            );

            render_graph.compile("backbuffer");
            render_graph.execute(&mut main_cmd_list);

            self.imgui.draw_non_main_viewports(&mut main_cmd_list);
            self.device
                .submit(QueueType::Graphics, &[&main_cmd_list], &[], &[]);
            self.device.present(swapchain);
            self.imgui.present();

            self.device.end_frame();
        }
    }

    fn update_main_window_swapchain(&mut self) {
        self.device.wait_idle();
        self.main_window_swapchain_rtvs.clear();

        let old_swapchain = self.main_window_swapchain.take().map(|old_swapchain| {
            Arc::try_unwrap(old_swapchain)
                .expect("Editor main window swapchain was still referenced when resized!")
        });

        let swapchain = Arc::new(
            self.device
                .create_swapchain(
                    &SwapChainDesc {
                        width: self.main_window.get_width(),
                        height: self.main_window.get_height(),
                        format: PixelFormat::R8G8B8A8Unorm,
                        sample_desc: Default::default(),
                        usage_flags: TextureUsageFlags::from_flag(
                            TextureUsageFlagBits::RenderTarget,
                        ),
                        window_handle: self.main_window.get_handle(),
                    },
                    old_swapchain,
                )
                .expect("Failed to create editor main window swapchain"),
        );

        for i in 0..self.device.get_swapchain_backbuffer_count(&swapchain) {
            self.main_window_swapchain_rtvs.push(Arc::new(
                self.device
                    .create_render_target_view(&RenderTargetViewDesc {
                        resource: self
                            .device
                            .get_swapchain_backbuffer(&swapchain, i as u32)
                            .unwrap(),
                        format: PixelFormat::R8G8B8A8Unorm,
                        ty: RenderTargetViewType::Texture2D(Texture2DRTV { mip_level: 0 }),
                    })
                    .expect("Failed to create editor main window RTV"),
            ));
        }

        self.main_window_swapchain = Some(swapchain);
    }
}
