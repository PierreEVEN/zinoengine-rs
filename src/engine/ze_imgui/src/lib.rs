use crate::renderer::{SwapChainType, ViewportRendererData};
use crate::str_buffer::StrBuffer;
use std::alloc::Layout;
use std::cmp::max;
use std::ffi::{CStr, CString};
use std::mem::{size_of, MaybeUninit};
use std::os::raw::*;
use std::sync::Arc;
use std::{cmp, mem, slice};
use ze_core::maths::{Matrix4f32, RectI32, Vec2f32, Vec2i32};
use ze_gfx::backend::{
    BlendFactor, BlendOp, ClearValue, CommandList, Device, IndexBufferFormat, MemoryLocation,
    PipelineBlendState, PipelineRenderTargetBlendDesc, RenderPassDesc, RenderPassTexture,
    RenderPassTextureLoadMode, RenderPassTextureStoreMode, RenderTargetViewDesc,
    RenderTargetViewType, ResourceBarrier, ResourceState, ResourceTransitionBarrier,
    ResourceTransitionBarrierResource, Sampler, SamplerDesc, ShaderResourceView,
    ShaderResourceViewDesc, ShaderResourceViewResource, ShaderResourceViewType, SwapChain,
    SwapChainDesc, Texture, Texture2DRTV, Texture2DSRV, TextureDesc, TextureUsageFlagBits,
    TextureUsageFlags, Viewport,
};
use ze_gfx::{utils, PixelFormat, SampleDesc};
use ze_imgui_sys::*;
use ze_platform::{Message, Platform, Window, WindowFlagBits, WindowFlags};
use ze_shader_system::{Shader, ShaderManager, ShaderModules};

const BACKEND_PLATFORM_NAME: &str = "ze_platform";
const BACKEND_RENDERER_NAME: &str = "ze_gfx";

pub struct Context {
    device: Arc<dyn Device>,
    shader_manager: Arc<ShaderManager>,
    platform: Arc<dyn Platform>,
    str_buffer: StrBuffer,
    context: *mut ImGuiContext,
    font_texture: Arc<Texture>,
    font_texture_view: ShaderResourceView,
    sampler: Sampler,
}

impl Context {
    pub fn new(
        device: Arc<dyn Device>,
        shader_manager: Arc<ShaderManager>,
        platform: Arc<dyn Platform>,
        main_window: Arc<dyn Window>,
    ) -> Box<Self> {
        let context = unsafe { igCreateContext(std::ptr::null_mut()) };

        let io = unsafe { igGetIO().as_mut().unwrap_unchecked() };
        io.ConfigFlags |= ImGuiConfigFlags__ImGuiConfigFlags_ViewportsEnable;
        io.ConfigFlags |= ImGuiConfigFlags__ImGuiConfigFlags_DockingEnable;
        io.BackendFlags |= ImGuiBackendFlags__ImGuiBackendFlags_HasMouseCursors;
        io.BackendFlags |= ImGuiBackendFlags__ImGuiBackendFlags_PlatformHasViewports;
        io.BackendFlags |= ImGuiBackendFlags__ImGuiBackendFlags_RendererHasViewports;
        io.BackendFlags |= ImGuiBackendFlags__ImGuiBackendFlags_RendererHasVtxOffset;

        unsafe {
            let file = CString::new("assets/Roboto-Regular.ttf").unwrap();
            ImFontAtlas_AddFontFromFileTTF(
                io.Fonts,
                file.as_ptr(),
                16.0,
                std::ptr::null(),
                std::ptr::null(),
            );
        }

        let mut platform_io = unsafe { igGetPlatformIO().as_mut().unwrap_unchecked() };
        platform_io.Platform_CreateWindow = Some(platform_create_window);
        platform_io.Platform_DestroyWindow = Some(platform_destroy_window);
        //platform_io.Platform_GetWindowPos = Some(platform_get_window_pos);
        //platform_io.Platform_GetWindowSize = Some(platform_get_window_size);
        platform_io.Platform_SetWindowPos = Some(platform_set_window_pos);
        platform_io.Platform_SetWindowSize = Some(platform_set_window_size);
        platform_io.Platform_SetWindowTitle = Some(platform_set_window_title);
        platform_io.Platform_ShowWindow = Some(platform_show_window);

        platform_io.Renderer_CreateWindow = Some(renderer_create_window);
        platform_io.Renderer_DestroyWindow = Some(renderer_destroy_window);
        platform_io.Renderer_SetWindowSize = Some(renderer_set_window_size);
        platform_io.Renderer_SwapBuffers = Some(renderer_swap_buffers);
        platform_io.Renderer_RenderWindow = Some(renderer_render_window);

        unsafe {
            ImGuiPlatformIO_Set_Platform_GetWindowPos(
                igGetPlatformIO(),
                Some(platform_get_window_pos),
            );
            ImGuiPlatformIO_Set_Platform_GetWindowSize(
                igGetPlatformIO(),
                Some(platform_get_window_size),
            );
        }

        let sampler = device
            .create_sampler(&SamplerDesc::default())
            .expect("Cannot create ImGui sampler");

        // Build font texture
        let font_texture = unsafe {
            let mut io = unsafe { igGetIO().as_mut().unwrap_unchecked() };
            let mut pixels = std::ptr::null_mut();
            let mut width = 0;
            let mut height = 0;
            ImFontAtlas_GetTexDataAsRGBA32(
                io.Fonts,
                &mut pixels,
                &mut width,
                &mut height,
                std::ptr::null_mut(),
            );

            let texture = device
                .create_texture(
                    &TextureDesc {
                        width: width as u32,
                        height: height as u32,
                        depth: 1,
                        mip_levels: 1,
                        format: PixelFormat::R8G8B8A8Unorm,
                        sample_desc: Default::default(),
                        usage_flags: TextureUsageFlags::default(),
                        memory_location: MemoryLocation::GpuOnly,
                    },
                    "ImGui Font texture",
                )
                .expect("Failed to create ImGui font texture");

            utils::copy_data_to_texture(
                &device,
                slice::from_raw_parts(pixels, (width * height * 4) as usize),
                &texture,
                ResourceState::Common,
            )
            .expect("Failed to copy font texture data");

            Arc::new(texture)
        };

        let font_texture_view = device
            .create_shader_resource_view(&ShaderResourceViewDesc {
                resource: ShaderResourceViewResource::Texture(font_texture.clone()),
                format: PixelFormat::R8G8B8A8Unorm,
                ty: ShaderResourceViewType::Texture2D(Texture2DSRV {
                    min_mip_level: 0,
                    mip_levels: 1,
                }),
            })
            .expect("Failed to create ImGui font texture view");

        let mut context = Box::new(Self {
            device,
            shader_manager,
            platform,
            str_buffer: StrBuffer::default(),
            context,
            font_texture,
            sampler,
            font_texture_view,
        });

        io.UserData = (context.as_mut() as *mut Context) as *mut c_void;

        // Create main resources for main viewport
        unsafe {
            platform_create_window(igGetMainViewport());
            let platform_data =
                (*igGetMainViewport()).PlatformUserData as *mut ViewportPlatformData;
            platform_data.write(ViewportPlatformData::new(main_window));

            renderer_create_window(igGetMainViewport());
            let renderer_data =
                (*igGetMainViewport()).RendererUserData as *mut ViewportRendererData;
            renderer_data.write(ViewportRendererData::default());
        }

        /** Default ZE style */
        {
            let style = unsafe { igGetStyle().as_mut().unwrap_unchecked() };
            style.WindowRounding = 0.0;
            style.FrameRounding = 3.0;
            style.TabRounding = 0.0;
            style.ScrollbarRounding = 0.0;
            style.WindowMenuButtonPosition = ImGuiDir__ImGuiDir_Right;
            style.TabMinWidthForCloseButton = 0.0;
            style.ItemSpacing = ImVec2::new(8.0, 4.0);
            style.WindowBorderSize = 0.0;
            style.FrameBorderSize = 0.0;
            style.PopupBorderSize = 1.0;
            style.TabBorderSize = 1.0;

            let mut colors = &mut style.Colors;

            colors[ImGuiCol__ImGuiCol_Text as usize] = ImVec4::new(0.79, 0.79, 0.79, 1.0);
            colors[ImGuiCol__ImGuiCol_TextDisabled as usize] = ImVec4::new(0.50, 0.50, 0.50, 1.0);
            colors[ImGuiCol__ImGuiCol_WindowBg as usize] = ImVec4::new(0.22, 0.22, 0.22, 0.94);
            colors[ImGuiCol__ImGuiCol_ChildBg as usize] = ImVec4::new(0.00, 0.00, 0.00, 0.00);
            colors[ImGuiCol__ImGuiCol_PopupBg as usize] = ImVec4::new(0.20, 0.20, 0.20, 0.94);
            colors[ImGuiCol__ImGuiCol_Border as usize] = ImVec4::new(0.09, 0.09, 0.09, 0.50);
            colors[ImGuiCol__ImGuiCol_BorderShadow as usize] = ImVec4::new(0.00, 0.00, 0.00, 0.00);
            colors[ImGuiCol__ImGuiCol_FrameBg as usize] = ImVec4::new(0.16, 0.16, 0.16, 0.54);
            colors[ImGuiCol__ImGuiCol_FrameBgHovered as usize] =
                ImVec4::new(0.30, 0.30, 0.30, 0.40);
            colors[ImGuiCol__ImGuiCol_FrameBgActive as usize] = ImVec4::new(0.33, 0.33, 0.33, 0.67);
            colors[ImGuiCol__ImGuiCol_TitleBg as usize] = ImVec4::new(0.16, 0.16, 0.16, 1.00);
            colors[ImGuiCol__ImGuiCol_TitleBgActive as usize] = ImVec4::new(0.16, 0.16, 0.16, 1.00);
            colors[ImGuiCol__ImGuiCol_TitleBgCollapsed as usize] =
                ImVec4::new(0.00, 0.00, 0.00, 0.51);
            colors[ImGuiCol__ImGuiCol_MenuBarBg as usize] = ImVec4::new(0.14, 0.14, 0.14, 1.00);
            colors[ImGuiCol__ImGuiCol_ScrollbarBg as usize] = ImVec4::new(0.02, 0.02, 0.02, 0.53);
            colors[ImGuiCol__ImGuiCol_ScrollbarGrab as usize] = ImVec4::new(0.31, 0.31, 0.31, 1.00);
            colors[ImGuiCol__ImGuiCol_ScrollbarGrabHovered as usize] =
                ImVec4::new(0.41, 0.41, 0.41, 1.00);
            colors[ImGuiCol__ImGuiCol_ScrollbarGrabActive as usize] =
                ImVec4::new(0.51, 0.51, 0.51, 1.00);
            colors[ImGuiCol__ImGuiCol_CheckMark as usize] = ImVec4::new(0.71, 0.71, 0.71, 1.00);
            colors[ImGuiCol__ImGuiCol_SliderGrab as usize] = ImVec4::new(0.29, 0.29, 0.29, 1.00);
            colors[ImGuiCol__ImGuiCol_SliderGrabActive as usize] =
                ImVec4::new(0.26, 0.26, 0.26, 1.00);
            colors[ImGuiCol__ImGuiCol_Button as usize] = ImVec4::new(0.29, 0.29, 0.29, 0.40);
            colors[ImGuiCol__ImGuiCol_ButtonHovered as usize] = ImVec4::new(0.26, 0.26, 0.26, 1.00);
            colors[ImGuiCol__ImGuiCol_ButtonActive as usize] = ImVec4::new(0.23, 0.23, 0.23, 1.00);
            colors[ImGuiCol__ImGuiCol_Header as usize] = ImVec4::new(0.11, 0.11, 0.11, 0.31);
            colors[ImGuiCol__ImGuiCol_HeaderHovered as usize] = ImVec4::new(0.13, 0.13, 0.13, 0.80);
            colors[ImGuiCol__ImGuiCol_HeaderActive as usize] = ImVec4::new(0.12, 0.12, 0.12, 1.00);
            colors[ImGuiCol__ImGuiCol_Separator as usize] = ImVec4::new(0.15, 0.14, 0.16, 0.50);
            colors[ImGuiCol__ImGuiCol_SeparatorHovered as usize] =
                ImVec4::new(0.15, 0.14, 0.16, 1.00);
            colors[ImGuiCol__ImGuiCol_SeparatorActive as usize] =
                ImVec4::new(0.14, 0.13, 0.16, 1.00);
            colors[ImGuiCol__ImGuiCol_ResizeGrip as usize] = ImVec4::new(0.00, 0.00, 0.00, 0.25);
            colors[ImGuiCol__ImGuiCol_ResizeGripHovered as usize] =
                ImVec4::new(0.11, 0.11, 0.11, 0.67);
            colors[ImGuiCol__ImGuiCol_ResizeGripActive as usize] =
                ImVec4::new(0.00, 0.00, 0.00, 0.95);
            colors[ImGuiCol__ImGuiCol_Tab as usize] = ImVec4::new(0.16, 0.16, 0.16, 0.86);
            colors[ImGuiCol__ImGuiCol_TabHovered as usize] = ImVec4::new(0.29, 0.29, 0.29, 0.80);
            colors[ImGuiCol__ImGuiCol_TabActive as usize] = ImVec4::new(0.24, 0.24, 0.24, 1.00);
            colors[ImGuiCol__ImGuiCol_TabUnfocused as usize] = ImVec4::new(0.24, 0.24, 0.24, 0.97);
            colors[ImGuiCol__ImGuiCol_TabUnfocusedActive as usize] =
                ImVec4::new(0.24, 0.24, 0.24, 1.00);
            colors[ImGuiCol__ImGuiCol_DockingPreview as usize] =
                ImVec4::new(0.26, 0.59, 0.98, 0.70);
            colors[ImGuiCol__ImGuiCol_DockingEmptyBg as usize] =
                ImVec4::new(0.20, 0.20, 0.20, 1.00);
            colors[ImGuiCol__ImGuiCol_PlotLines as usize] = ImVec4::new(0.61, 0.61, 0.61, 1.00);
            colors[ImGuiCol__ImGuiCol_PlotLinesHovered as usize] =
                ImVec4::new(1.00, 0.43, 0.35, 1.00);
            colors[ImGuiCol__ImGuiCol_PlotHistogram as usize] = ImVec4::new(0.90, 0.70, 0.00, 1.00);
            colors[ImGuiCol__ImGuiCol_PlotHistogramHovered as usize] =
                ImVec4::new(1.00, 0.60, 0.00, 1.00);
            colors[ImGuiCol__ImGuiCol_TableHeaderBg as usize] = ImVec4::new(0.19, 0.19, 0.20, 1.00);
            colors[ImGuiCol__ImGuiCol_TableBorderStrong as usize] =
                ImVec4::new(0.31, 0.31, 0.35, 1.00);
            colors[ImGuiCol__ImGuiCol_TableBorderLight as usize] =
                ImVec4::new(0.23, 0.23, 0.25, 1.00);
            colors[ImGuiCol__ImGuiCol_TableRowBg as usize] = ImVec4::new(0.00, 0.00, 0.00, 0.00);
            colors[ImGuiCol__ImGuiCol_TableRowBgAlt as usize] = ImVec4::new(1.00, 1.00, 1.00, 0.06);
            colors[ImGuiCol__ImGuiCol_TextSelectedBg as usize] =
                ImVec4::new(0.26, 0.59, 0.98, 0.35);
            colors[ImGuiCol__ImGuiCol_DragDropTarget as usize] =
                ImVec4::new(1.00, 1.00, 0.00, 0.90);
            colors[ImGuiCol__ImGuiCol_NavHighlight as usize] = ImVec4::new(0.26, 0.59, 0.98, 1.00);
            colors[ImGuiCol__ImGuiCol_NavWindowingHighlight as usize] =
                ImVec4::new(1.00, 1.00, 1.00, 0.70);
            colors[ImGuiCol__ImGuiCol_NavWindowingDimBg as usize] =
                ImVec4::new(0.80, 0.80, 0.80, 0.20);
            colors[ImGuiCol__ImGuiCol_ModalWindowDimBg as usize] =
                ImVec4::new(0.80, 0.80, 0.80, 0.35);
        }

        context.update_monitors();
        context
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { igDestroyContext(self.context) };
    }
}

impl Context {
    pub fn begin_frame(
        &mut self,
        delta_time: f32,
        mouse_position: Vec2i32,
        main_viewport_window: &dyn Window,
    ) {
        unsafe {
            igSetCurrentContext(self.context);
        }

        let mut io = unsafe { igGetIO().as_mut().unwrap_unchecked() };

        io.DeltaTime = delta_time;
        io.DisplaySize = ImVec2 {
            x: main_viewport_window.get_width() as f32,
            y: main_viewport_window.get_height() as f32,
        };
        io.MousePos = ImVec2 {
            x: mouse_position.x as f32,
            y: mouse_position.y as f32,
        };

        unsafe {
            igNewFrame();
            igShowDemoWindow(std::ptr::null_mut());
        }
    }

    pub fn send_platform_message(&mut self, message: &Message) {
        let mut io = unsafe { igGetIO().as_mut().unwrap_unchecked() };

        match message {
            Message::MouseButtonDown(_, button, _) => {
                io.MouseDown[*button as usize] = true;
            }
            Message::MouseButtonUp(_, button, _) => {
                io.MouseDown[*button as usize] = false;
            }
            Message::MouseButtonDoubleClick(_, button, _) => {
                io.MouseDown[*button as usize] = true;
            }
            Message::MouseWheel(_, delta, _) => {
                io.MouseWheel += delta;
            }
            _ => {}
        }
    }

    pub fn end_frame(&mut self) {
        unsafe {
            igRender();
            igUpdatePlatformWindows();
        }
    }

    pub fn draw_non_main_viewports(&mut self, cmd_list: &mut CommandList) {
        let mut io = unsafe { igGetPlatformIO().as_mut().unwrap_unchecked() };
        let viewports =
            unsafe { slice::from_raw_parts(io.Viewports.Data, io.Viewports.Size as usize) };

        let main_viewport = unsafe { igGetMainViewport() };

        for viewport in viewports {
            let renderer_data =
                unsafe { (*(*viewport)).RendererUserData as *mut ViewportRendererData };

            if *viewport != main_viewport {
                if let SwapChainType::Owned((swapchain, views)) =
                    unsafe { &(*renderer_data).swapchain }
                {
                    let swapchain = unsafe { swapchain.assume_init_ref() };

                    let backbuffer_index = self.device.get_swapchain_backbuffer_index(&swapchain);
                    let backbuffer = self
                        .device
                        .get_swapchain_backbuffer(&swapchain, backbuffer_index)
                        .unwrap();

                    self.device.cmd_resource_barrier(
                        cmd_list,
                        &[ResourceBarrier::Transition(ResourceTransitionBarrier {
                            resource: ResourceTransitionBarrierResource::Texture(&*backbuffer),
                            source_state: ResourceState::Present,
                            dest_state: ResourceState::RenderTargetWrite,
                        })],
                    );

                    self.device.cmd_begin_render_pass(
                        cmd_list,
                        &RenderPassDesc {
                            render_targets: &[RenderPassTexture {
                                render_target_view: &views[backbuffer_index as usize],
                                load_mode: RenderPassTextureLoadMode::Clear,
                                store_mode: RenderPassTextureStoreMode::Preserve,
                                clear_value: ClearValue::Color([0.0, 0.0, 0.0, 1.0]),
                            }],
                            depth_stencil: None,
                        },
                    );

                    draw_viewport_internal(
                        unsafe { (*viewport).as_mut() }.unwrap(),
                        &self.device,
                        &self.shader_manager,
                        &self.font_texture_view,
                        &self.sampler,
                        cmd_list,
                    );

                    self.device.cmd_end_render_pass(cmd_list);
                    self.device.cmd_resource_barrier(
                        cmd_list,
                        &[ResourceBarrier::Transition(ResourceTransitionBarrier {
                            resource: ResourceTransitionBarrierResource::Texture(&*backbuffer),
                            source_state: ResourceState::RenderTargetWrite,
                            dest_state: ResourceState::Present,
                        })],
                    );
                }
            }
        }
    }

    pub fn draw_viewport(&self, cmd_list: &mut CommandList, viewport: &mut ImGuiViewport) {
        draw_viewport_internal(
            viewport,
            &self.device,
            &self.shader_manager,
            &self.font_texture_view,
            &self.sampler,
            cmd_list,
        );
    }

    pub fn present(&mut self) {
        let mut io = unsafe { igGetPlatformIO().as_mut().unwrap_unchecked() };
        let viewports =
            unsafe { slice::from_raw_parts(io.Viewports.Data, io.Viewports.Size as usize) };
        for viewport in viewports {
            if *viewport != unsafe { igGetMainViewport() } {
                unsafe {
                    let renderer_data =
                        (*(*viewport)).RendererUserData as *mut ViewportRendererData;
                    if let SwapChainType::Owned((swapchain, _)) = &(*renderer_data).swapchain {
                        self.device.present(swapchain.assume_init_ref());
                    }
                }
            }
        }
    }

    pub fn update_monitors(&mut self) {
        let mut io = unsafe { igGetPlatformIO().as_mut().unwrap_unchecked() };
        let monitor_count = self.platform.get_monitor_count();
        if io.Monitors.Capacity > 0 {
            unsafe {
                igMemFree(io.Monitors.Data as *mut c_void);
            }
        }

        io.Monitors.Capacity = monitor_count as c_int;
        io.Monitors.Size = monitor_count as c_int;
        io.Monitors.Data =
            unsafe { igMemAlloc((monitor_count * size_of::<ImGuiPlatformMonitor>()) as u64) }
                as *mut ImGuiPlatformMonitor;

        let mut monitors = unsafe { slice::from_raw_parts_mut(io.Monitors.Data, monitor_count) };
        for (index, monitor) in monitors.iter_mut().enumerate() {
            let platform_monitor = self.platform.get_monitor(index);
            monitor.MainPos = ImVec2 {
                x: platform_monitor.bounds.x as f32,
                y: platform_monitor.bounds.y as f32,
            };
            monitor.MainSize = ImVec2 {
                x: platform_monitor.bounds.width as f32,
                y: platform_monitor.bounds.height as f32,
            };
            monitor.WorkPos = ImVec2 {
                x: platform_monitor.work_bounds.x as f32,
                y: platform_monitor.work_bounds.y as f32,
            };
            monitor.WorkSize = ImVec2 {
                x: platform_monitor.work_bounds.width as f32,
                y: platform_monitor.work_bounds.height as f32,
            };
            monitor.DpiScale = platform_monitor.dpi / 96.0;
        }
    }

    pub fn get_str_buffer(&mut self) -> &mut StrBuffer {
        &mut self.str_buffer
    }

    #[allow(clippy::mut_from_ref)]
    pub fn get_main_viewport(&self) -> &mut ImGuiViewport {
        unsafe { igGetMainViewport().as_mut().unwrap_unchecked() }
    }
}

// UI elements
impl Context {
    pub fn window<'a>(&'a mut self, name: &'a str) -> window::WindowBuilder<'a> {
        window::WindowBuilder::new(self, name)
    }

    pub fn text(&mut self, text: &str) {
        let c_text = self.str_buffer.convert(text);
        unsafe { igTextUnformatted(c_text, c_text.add(text.len())) };
    }

    pub fn end(&mut self) {
        unsafe { igEnd() };
    }
}

struct ViewportPlatformData {
    window: Arc<dyn Window>,
}

impl ViewportPlatformData {
    fn new(window: Arc<dyn Window>) -> Self {
        Self { window }
    }
}

fn draw_viewport_internal(
    viewport: &mut ImGuiViewport,
    device: &Arc<dyn Device>,
    shader_manager: &Arc<ShaderManager>,
    font_texture: &ShaderResourceView,
    sampler: &Sampler,
    cmd_list: &mut CommandList,
) {
    #[repr(C)]
    struct ShaderData {
        projection_matrix: Matrix4f32,
        base_vertex_location: u32,
        vertex_buffer: u32,
        texture: u32,
        texture_sampler: u32,
    };

    let mut renderer_data =
        unsafe { (viewport.RendererUserData as *mut ViewportRendererData).as_mut() }.unwrap();

    let draw_data = unsafe { viewport.DrawData.as_ref().unwrap_unchecked() };
    renderer_data.update_buffers(device, draw_data);

    if let Ok(shader) = shader_manager.get_shader_modules(&"ImGui".to_string(), None) {
        if draw_data.CmdListsCount > 0 {
            #[rustfmt::skip]
                let projection_matrix = {
                let left = draw_data.DisplayPos.x;
                let right = draw_data.DisplayPos.x + draw_data.DisplaySize.x;
                let top = draw_data.DisplayPos.y;
                let bottom = draw_data.DisplayPos.y + draw_data.DisplaySize.y;
                Matrix4f32::new([
                    [2.0 / (right - left), 0.0, 0.0, 0.0],
                    [0.0, 2.0 / (top - bottom), 0.0, 0.0],
                    [0.0, 0.0, 0.5, 0.0],
                    [(right + left) / (left - right), (top + bottom) / (bottom - top), 0.5, 1.0],
                ])
            };

            let mut shader_data = ShaderData {
                projection_matrix,
                base_vertex_location: 0,
                vertex_buffer: renderer_data
                    .vertex_buffer_srv
                    .as_ref()
                    .unwrap()
                    .get_descriptor_index(),
                texture: font_texture.get_descriptor_index(),
                texture_sampler: sampler.get_descriptor_index(),
            };

            device.cmd_set_shader_stages(cmd_list, &shader.get_pipeline_stages());

            let mut blend_state = PipelineBlendState::default();
            blend_state.render_targets[0] = PipelineRenderTargetBlendDesc {
                enable_blend: true,
                src_color_blend_factor: BlendFactor::SrcAlpha,
                dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
                color_blend_op: BlendOp::Add,
                src_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
                dst_alpha_blend_factor: BlendFactor::Zero,
                alpha_blend_op: BlendOp::Add,
            };
            device.cmd_set_blend_state(cmd_list, &blend_state);
            device.cmd_bind_index_buffer(
                cmd_list,
                &renderer_data.index_buffer.as_ref().unwrap(),
                IndexBufferFormat::Uint16,
            );

            device.cmd_set_viewports(
                cmd_list,
                &[Viewport {
                    position: Vec2f32::default(),
                    size: Vec2f32::new(draw_data.DisplaySize.x, draw_data.DisplaySize.y),
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );

            let mut vertex_offset = 0;
            let mut index_offset = 0;

            let draw_lists = unsafe {
                slice::from_raw_parts(draw_data.CmdLists, draw_data.CmdListsCount as usize)
            };

            for draw_list in draw_lists {
                let draw_list = unsafe { draw_list.as_ref().unwrap_unchecked() };
                let cmd_buffers = unsafe {
                    slice::from_raw_parts(
                        draw_list.CmdBuffer.Data,
                        draw_list.CmdBuffer.Size as usize,
                    )
                };
                for cmd in cmd_buffers {
                    let clip_offset = draw_data.DisplayPos;

                    let clip_min = Vec2i32::new(
                        (cmd.ClipRect.x - clip_offset.x) as i32,
                        (cmd.ClipRect.y - clip_offset.y) as i32,
                    );

                    let clip_max = Vec2i32::new(
                        (cmd.ClipRect.z - clip_offset.x) as i32,
                        (cmd.ClipRect.w - clip_offset.y) as i32,
                    );

                    let clip_rect = RectI32::new(clip_min.x, clip_min.y, clip_max.x, clip_max.y);

                    device.cmd_set_scissors(cmd_list, &[clip_rect]);

                    shader_data.base_vertex_location = cmd.VtxOffset + vertex_offset;
                    device.cmd_push_constants(cmd_list, 0, unsafe {
                        slice::from_raw_parts(
                            (&shader_data as *const ShaderData) as *const u8,
                            size_of::<ShaderData>(),
                        )
                    });

                    device.cmd_draw_indexed(
                        cmd_list,
                        cmd.ElemCount,
                        1,
                        cmd.IdxOffset + index_offset,
                        0,
                    );
                }

                vertex_offset += draw_list.VtxBuffer.Size as u32;
                index_offset += draw_list.IdxBuffer.Size as u32;
            }
        }
    }
}

// ImGui Platform IO callbacks
unsafe extern "C" fn platform_create_window(vp: *mut ImGuiViewport) {
    let context = ((*igGetIO()).UserData as *const Context)
        .as_ref()
        .unwrap_unchecked();

    let viewport = vp.as_mut().unwrap_unchecked();
    let platform_data =
        std::alloc::alloc(Layout::new::<ViewportPlatformData>()) as *mut ViewportPlatformData;

    if vp != igGetMainViewport() {
        let window = context
            .platform
            .create_window(
                "ImGui Viewport Window",
                viewport.Size.x as u32,
                viewport.Size.y as u32,
                viewport.Pos.x as i32,
                viewport.Pos.y as i32,
                WindowFlags::from_flag(WindowFlagBits::Borderless),
            )
            .unwrap();
        platform_data.write(ViewportPlatformData::new(window));
    }

    viewport.PlatformUserData = platform_data as *mut c_void;
}

unsafe extern "C" fn platform_destroy_window(vp: *mut ImGuiViewport) {
    let platform_data = (*vp).PlatformUserData as *mut ViewportPlatformData;
    platform_data.drop_in_place();
    std::alloc::dealloc(
        platform_data as *mut u8,
        Layout::new::<ViewportPlatformData>(),
    );
    (*vp).PlatformUserData = std::ptr::null_mut();
}

unsafe extern "C" fn platform_get_window_size(vp: *mut ImGuiViewport, size: *mut ImVec2) {
    todo!()
}

unsafe extern "C" fn platform_get_window_pos(vp: *mut ImGuiViewport, pos: *mut ImVec2) {
    let platform_user_data = ((*vp).PlatformUserData as *mut ViewportPlatformData)
        .as_ref()
        .unwrap_unchecked();

    (*pos).x = platform_user_data.window.get_position().x as f32;
    (*pos).y = platform_user_data.window.get_position().y as f32;
}

unsafe extern "C" fn platform_set_window_pos(vp: *mut ImGuiViewport, pos: ImVec2) {
    let platform_user_data = ((*vp).PlatformUserData as *mut ViewportPlatformData)
        .as_ref()
        .unwrap_unchecked();

    platform_user_data
        .window
        .set_position(Vec2i32::new(pos.x as i32, pos.y as i32));
}

unsafe extern "C" fn platform_set_window_size(vp: *mut ImGuiViewport, size: ImVec2) {
    let platform_user_data = ((*vp).PlatformUserData as *mut ViewportPlatformData)
        .as_ref()
        .unwrap_unchecked();

    platform_user_data
        .window
        .set_size(size.x as u32, size.y as u32);
}

unsafe extern "C" fn platform_set_window_title(vp: *mut ImGuiViewport, title: *const c_char) {
    let platform_user_data = ((*vp).PlatformUserData as *mut ViewportPlatformData)
        .as_ref()
        .unwrap_unchecked();

    let title = CStr::from_ptr(title);
    platform_user_data
        .window
        .set_title(title.to_string_lossy().as_ref());
}

unsafe extern "C" fn platform_show_window(vp: *mut ImGuiViewport) {
    let platform_user_data = ((*vp).PlatformUserData as *mut ViewportPlatformData)
        .as_ref()
        .unwrap_unchecked();

    platform_user_data.window.show();
}

// Renderer
unsafe extern "C" fn renderer_create_window(vp: *mut ImGuiViewport) {
    let context = ((*igGetIO()).UserData as *const Context)
        .as_ref()
        .unwrap_unchecked();

    let viewport = vp.as_mut().unwrap_unchecked();
    let renderer_data =
        std::alloc::alloc(Layout::new::<ViewportRendererData>()) as *mut ViewportRendererData;
    renderer_data.write(ViewportRendererData::default());

    if vp != igGetMainViewport() {
        let platform_data = (*vp).PlatformUserData as *mut ViewportPlatformData;

        let swapchain = context
            .device
            .create_swapchain(
                &SwapChainDesc {
                    width: (*vp).Size.x as u32,
                    height: (*vp).Size.y as u32,
                    format: PixelFormat::R8G8B8A8Unorm,
                    sample_desc: SampleDesc::default(),
                    usage_flags: TextureUsageFlags::from_flag(TextureUsageFlagBits::RenderTarget),
                    window_handle: (*platform_data).window.get_handle(),
                },
                None,
            )
            .unwrap();

        let mut swapchain_render_target_views = vec![];
        for i in 0..context.device.get_swapchain_backbuffer_count(&swapchain) {
            swapchain_render_target_views.push(
                context
                    .device
                    .create_render_target_view(&RenderTargetViewDesc {
                        resource: context
                            .device
                            .get_swapchain_backbuffer(&swapchain, i as u32)
                            .unwrap(),
                        format: PixelFormat::R8G8B8A8Unorm,
                        ty: RenderTargetViewType::Texture2D(Texture2DRTV { mip_level: 0 }),
                    })
                    .unwrap(),
            );
        }

        (*renderer_data).swapchain = SwapChainType::Owned((
            MaybeUninit::new(Arc::new(swapchain)),
            swapchain_render_target_views,
        ));
    }

    viewport.RendererUserData = renderer_data as *mut c_void;
}

unsafe extern "C" fn renderer_destroy_window(vp: *mut ImGuiViewport) {
    let renderer_data = (*vp).RendererUserData as *mut ViewportRendererData;
    renderer_data.drop_in_place();
    std::alloc::dealloc(
        renderer_data as *mut u8,
        Layout::new::<ViewportRendererData>(),
    );
    (*vp).RendererUserData = std::ptr::null_mut();
}

unsafe extern "C" fn renderer_set_window_size(vp: *mut ImGuiViewport, size: ImVec2) {
    let context = ((*igGetIO()).UserData as *const Context)
        .as_ref()
        .unwrap_unchecked();

    let platform_user_data = ((*vp).PlatformUserData as *mut ViewportPlatformData)
        .as_ref()
        .unwrap_unchecked();

    let mut renderer_user_data = ((*vp).RendererUserData as *mut ViewportRendererData)
        .as_mut()
        .unwrap_unchecked();

    if let SwapChainType::Owned((old_swapchain, old_rtvs)) = &mut renderer_user_data.swapchain {
        context.device.wait_idle();
        old_rtvs.clear();

        let old_swapchain = unsafe { mem::replace(old_swapchain, MaybeUninit::uninit()) };

        let swapchain = context
            .device
            .create_swapchain(
                &SwapChainDesc {
                    width: size.x as u32,
                    height: size.y as u32,
                    format: PixelFormat::R8G8B8A8Unorm,
                    sample_desc: SampleDesc::default(),
                    usage_flags: TextureUsageFlags::from_flag(TextureUsageFlagBits::RenderTarget),
                    window_handle: (*platform_user_data).window.get_handle(),
                },
                Some(Arc::try_unwrap(old_swapchain.assume_init()).expect("Failed to unwrap arc!")),
            )
            .unwrap();

        let mut swapchain_render_target_views = vec![];
        for i in 0..context.device.get_swapchain_backbuffer_count(&swapchain) {
            swapchain_render_target_views.push(
                context
                    .device
                    .create_render_target_view(&RenderTargetViewDesc {
                        resource: context
                            .device
                            .get_swapchain_backbuffer(&swapchain, i as u32)
                            .unwrap(),
                        format: PixelFormat::R8G8B8A8Unorm,
                        ty: RenderTargetViewType::Texture2D(Texture2DRTV { mip_level: 0 }),
                    })
                    .unwrap(),
            );
        }

        renderer_user_data.swapchain = SwapChainType::Owned((
            MaybeUninit::new(Arc::new(swapchain)),
            swapchain_render_target_views,
        ));
    }
}

unsafe extern "C" fn renderer_swap_buffers(vp: *mut ImGuiViewport, userdata: *mut c_void) {
    unimplemented!();
}

unsafe extern "C" fn renderer_render_window(vp: *mut ImGuiViewport, userdata: *mut c_void) {
    unimplemented!();
}

mod renderer;
mod str_buffer;
pub mod window;
