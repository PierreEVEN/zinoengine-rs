use crate::renderer::{SwapChainType, ViewportRendererData};
use crate::str_buffer::StrBuffer;
use enumflags2::*;
use std::alloc::Layout;
use std::ffi::{CStr, CString};
use std::mem::{size_of, MaybeUninit};
use std::os::raw::*;
use std::ptr::null_mut;
use std::sync::Arc;
use std::{mem, slice};
use ze_core::maths::{Matrix4f32, RectI32, Vec2f32, Vec2i32};
use ze_core::{ze_verbose};
use ze_gfx::backend::*;
use ze_gfx::{utils, PixelFormat, SampleDesc};
use ze_imgui_sys::*;
use ze_platform::{Cursor, KeyCode, Message, MouseButton, Platform, SystemCursor, Window};
use ze_shader_system::ShaderManager;

#[repr(transparent)]
pub struct Viewport {
    viewport: ImGuiViewport,
}

impl Viewport {
    pub fn center(&self) -> ImVec2 {
        let mut center = ImVec2::default();
        unsafe {
            let viewport = &self.viewport as *const _ as *mut _;
            ImGuiViewport_GetCenter(&mut center, viewport);
        }
        center
    }
    
    fn draw_data(&mut self) -> *mut ImDrawData {
        self.viewport.DrawData
    }

    fn renderer_user_data(&mut self) -> *mut c_void {
        self.viewport.RendererUserData
    }
}

impl PartialEq<Self> for Viewport {
    fn eq(&self, other: &Self) -> bool {
        self.viewport.ID == other.viewport.ID
    }
}

impl Eq for Viewport {}

pub struct Context {
    device: Arc<dyn Device>,
    shader_manager: Arc<ShaderManager>,
    platform: Arc<dyn Platform>,
    str_buffer: StrBuffer,
    context: *mut ImGuiContext,
    _font_texture: Arc<Texture>,
    font_texture_view: ShaderResourceView,
    sampler: Sampler,
    cursors: [Box<dyn Cursor>; ImGuiMouseCursor__ImGuiMouseCursor_COUNT as usize],
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
        io.ConfigFlags |= ImGuiConfigFlags__ImGuiConfigFlags_ViewportsEnable as i32;
        io.ConfigFlags |= ImGuiConfigFlags__ImGuiConfigFlags_DockingEnable as i32;
        io.BackendFlags |= ImGuiBackendFlags__ImGuiBackendFlags_HasMouseCursors as i32;
        io.BackendFlags |= ImGuiBackendFlags__ImGuiBackendFlags_PlatformHasViewports as i32;
        io.BackendFlags |= ImGuiBackendFlags__ImGuiBackendFlags_RendererHasViewports as i32;
        io.BackendFlags |= ImGuiBackendFlags__ImGuiBackendFlags_RendererHasVtxOffset as i32;

        unsafe {
            let file = CString::new("assets/Inter-SemiBold.ttf").unwrap();
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
            let io = igGetIO().as_mut().unwrap_unchecked();
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
                width as u32,
                height as u32,
                4,
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

        let cursors = [
            platform.create_system_cursor(SystemCursor::Arrow),
            platform.create_system_cursor(SystemCursor::Ibeam),
            platform.create_system_cursor(SystemCursor::SizeAll),
            platform.create_system_cursor(SystemCursor::SizeNorthSouth),
            platform.create_system_cursor(SystemCursor::SizeWestEast),
            platform.create_system_cursor(SystemCursor::SizeNorthEastSouthWest),
            platform.create_system_cursor(SystemCursor::SizeNorthWestSouthEast),
            platform.create_system_cursor(SystemCursor::Hand),
            platform.create_system_cursor(SystemCursor::No),
        ];

        let mut context = Box::new(Self {
            device,
            shader_manager,
            platform,
            str_buffer: StrBuffer::default(),
            context,
            _font_texture: font_texture,
            sampler,
            font_texture_view,
            cursors,
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

        // Default ZE style
        {
            let style = unsafe { igGetStyle().as_mut().unwrap_unchecked() };
            style.WindowRounding = 0.0;
            style.FrameRounding = 3.0;
            style.TabRounding = 2.0;
            style.ScrollbarRounding = 0.0;
            style.WindowMenuButtonPosition = ImGuiDir__ImGuiDir_Right;
            style.TabMinWidthForCloseButton = 0.0;
            style.CellPadding = ImVec2::new(1.0, 0.0);
            style.WindowPadding = ImVec2::new(3.0, 1.0);
            style.ItemSpacing = ImVec2::new(8.0, 4.0);
            style.IndentSpacing = 9.0;
            style.WindowBorderSize = 0.0;
            style.FrameBorderSize = 0.0;
            style.PopupBorderSize = 1.0;
            style.TabBorderSize = 1.0;

            let colors = &mut style.Colors;

            colors[ImGuiCol__ImGuiCol_Text as usize] = ImVec4::new(0.79, 0.79, 0.79, 1.0);
            colors[ImGuiCol__ImGuiCol_TextDisabled as usize] = ImVec4::new(0.50, 0.50, 0.50, 1.0);
            colors[ImGuiCol__ImGuiCol_WindowBg as usize] = ImVec4::new(0.07, 0.07, 0.07, 1.00);
            colors[ImGuiCol__ImGuiCol_ChildBg as usize] = ImVec4::new(0.14, 0.14, 0.14, 1.00);
            colors[ImGuiCol__ImGuiCol_PopupBg as usize] = ImVec4::new(0.20, 0.20, 0.20, 0.94);
            colors[ImGuiCol__ImGuiCol_Border as usize] = ImVec4::new(0.09, 0.09, 0.09, 1.0);
            colors[ImGuiCol__ImGuiCol_BorderShadow as usize] = ImVec4::new(0.00, 0.00, 0.00, 0.00);
            colors[ImGuiCol__ImGuiCol_FrameBg as usize] = ImVec4::new(0.09, 0.09, 0.09, 1.0);
            colors[ImGuiCol__ImGuiCol_FrameBgHovered as usize] = ImVec4::new(0.05, 0.05, 0.05, 1.0);
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
            colors[ImGuiCol__ImGuiCol_Header as usize] = ImVec4::from(0.115);
            colors[ImGuiCol__ImGuiCol_HeaderHovered as usize] = ImVec4::new(0.27, 0.33, 0.43, 0.45);
            colors[ImGuiCol__ImGuiCol_HeaderActive as usize] = ImVec4::new(0.27, 0.33, 0.63, 1.00);
            colors[ImGuiCol__ImGuiCol_Separator as usize] = ImVec4::new(0.25, 0.25, 0.25, 1.0);
            colors[ImGuiCol__ImGuiCol_SeparatorHovered as usize] =
                ImVec4::new(0.15, 0.14, 0.16, 1.00);
            colors[ImGuiCol__ImGuiCol_SeparatorActive as usize] =
                ImVec4::new(0.14, 0.13, 0.16, 1.00);
            colors[ImGuiCol__ImGuiCol_ResizeGrip as usize] = ImVec4::new(0.00, 0.00, 0.00, 0.25);
            colors[ImGuiCol__ImGuiCol_ResizeGripHovered as usize] =
                ImVec4::new(0.11, 0.11, 0.11, 0.67);
            colors[ImGuiCol__ImGuiCol_ResizeGripActive as usize] =
                ImVec4::new(0.00, 0.00, 0.00, 0.95);
            colors[ImGuiCol__ImGuiCol_Tab as usize] = ImVec4::new(0.078, 0.078, 0.078, 1.0);
            colors[ImGuiCol__ImGuiCol_TabHovered as usize] = ImVec4::new(0.29, 0.29, 0.29, 0.80);
            colors[ImGuiCol__ImGuiCol_TabActive as usize] = ImVec4::new(0.14, 0.14, 0.14, 1.00);
            colors[ImGuiCol__ImGuiCol_TabUnfocused as usize] = ImVec4::new(0.24, 0.24, 0.24, 0.97);
            colors[ImGuiCol__ImGuiCol_TabUnfocusedActive as usize] =
                ImVec4::new(0.24, 0.24, 0.24, 1.00);
            colors[ImGuiCol__ImGuiCol_DockingPreview as usize] =
                ImVec4::new(0.26, 0.59, 0.98, 0.70);
            colors[ImGuiCol__ImGuiCol_DockingEmptyBg as usize] =
                ImVec4::new(0.12, 0.12, 0.12, 1.00);
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
                ImVec4::new(0.10, 0.10, 0.10, 1.00);
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
                ImVec4::new(0.80, 0.80, 0.80, 0.0);
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
            x: main_viewport_window.width() as f32,
            y: main_viewport_window.height() as f32,
        };
        io.MousePos = ImVec2 {
            x: mouse_position.x as f32,
            y: mouse_position.y as f32,
        };

        // Update cursor
        let cursor = unsafe { igGetMouseCursor() };

        if io.MouseDrawCursor || cursor == ImGuiMouseCursor__ImGuiMouseCursor_None {
            self.platform.set_cursor(None);
        } else {
            self.platform
                .set_cursor(Some(&*self.cursors[cursor as usize]));
        }

        unsafe {
            igNewFrame();
            igShowStyleEditor(null_mut());
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
            },
            Message::KeyDown(_, key, _, _) => {
                unsafe { ImGuiIO_AddKeyEvent(igGetIO(), Key::from(*key) as ImGuiKey, true) };
            },
            Message::KeyUp(_, key, _, _) => {
                unsafe { ImGuiIO_AddKeyEvent(igGetIO(), Key::from(*key) as ImGuiKey, false) };
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
        let io = unsafe { igGetPlatformIO().as_mut().unwrap_unchecked() };
        let viewports =
            unsafe { slice::from_raw_parts(io.Viewports.Data, io.Viewports.Size as usize) };

        for viewport in viewports {
            let viewport = unsafe { (*viewport as *mut Viewport).as_mut().unwrap_unchecked() };
            let renderer_data = viewport.renderer_user_data() as *mut ViewportRendererData;

            if viewport != self.main_viewport() {
                if let SwapChainType::Owned((swapchain, views)) =
                    unsafe { &(*renderer_data).swapchain }
                {
                    let swapchain = unsafe { swapchain.assume_init_ref() };

                    let backbuffer_index = self.device.swapchain_backbuffer_index(swapchain);
                    let backbuffer = self
                        .device
                        .swapchain_backbuffer(swapchain, backbuffer_index)
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
                        viewport,
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

    pub fn draw_viewport(&self, cmd_list: &mut CommandList, viewport: &mut Viewport) {
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
        let io = unsafe { igGetPlatformIO().as_mut().unwrap_unchecked() };
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
        let monitor_count = self.platform.monitor_count();
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

        let monitors = unsafe { slice::from_raw_parts_mut(io.Monitors.Data, monitor_count) };
        for (index, monitor) in monitors.iter_mut().enumerate() {
            let platform_monitor = self.platform.monitor(index);
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

    pub fn str_buffer(&mut self) -> &mut StrBuffer {
        &mut self.str_buffer
    }

    pub fn main_viewport(&self) -> &Viewport {
        unsafe { (igGetMainViewport() as *mut Viewport).as_ref().unwrap_unchecked() }
    }
    
    #[allow(clippy::mut_from_ref)]
    pub fn main_viewport_mut(&self) -> &mut Viewport {
        unsafe { (igGetMainViewport() as *mut Viewport).as_mut().unwrap_unchecked() }
    }
}

#[bitflags]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum TableFlagBits {
    Resizable = 1,
    Reorderable = 2,
    Hideable = 4,
    Sortable = 8,
    NoSavedSettings = 16,
    ContextMenuInBody = 32,
    RowBg = 64,
    BordersInnerH = 128,
    BordersOuterH = 256,
    BordersInnerV = 512,
    BordersOuterV = 1024,
    NoBordersInBody = 2048,
    NoBordersInBodyUntilResize = 4096,
    SizingFixedFit = 8192,
    SizingFixedSame = 16384,
    NoHostExtendX = 65536,
    NoHostExtendY = 131072,
    NoKeepColumnsVisible = 262144,
    PreciseWidths = 524288,
    NoClip = 1048576,
    PadOuterX = 2097152,
    NoPadOuterX = 4194304,
    NoPadInnerX = 8388608,
    ScrollX = 16777216,
    ScrollY = 33554432,
    SortMulti = 67108864,
    SortTristate = 134217728,
}

pub type TableFlags = BitFlags<TableFlagBits>;

pub const TABLE_FLAG_SIZING_STRETCH_PROP: TableFlags =
    TableFlags::from_bits_truncate_c(3 << 13, BitFlags::CONST_TOKEN);

pub const TABLE_FLAG_SIZING_STRETCH_SAME: TableFlags =
    TableFlags::from_bits_truncate_c(4 << 13, BitFlags::CONST_TOKEN);

#[bitflags]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum TableColumnFlagBits {
    Disabled = 1,
    DefaultHide = 2,
    DefaultSort = 4,
    WidthStretch = 8,
    WidthFixed = 16,
    NoResize = 32,
    NoReorder = 64,
    NoHide = 128,
    NoClip = 256,
    NoSort = 512,
    NoSortAscending = 1024,
    NoSortDescending = 2048,
    NoHeaderLabel = 4096,
    NoHeaderWidth = 8192,
    PreferSortAscending = 16384,
    PreferSortDescending = 32768,
    IndentEnable = 65536,
    IndentDisable = 131072,
    IsEnabled = 16777216,
    IsVisible = 33554432,
    IsSorted = 67108864,
    IsHovered = 134217728,
}

pub type TableColumnFlags = BitFlags<TableColumnFlagBits>;

#[bitflags]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum WindowFlagBits {
    NoTitleBar = 1,
    NoResize = 2,
    NoMove = 4,
    NoScrollbar = 8,
    NoScrollWithMouse = 16,
    NoCollapse = 32,
    AlwaysAutoResize = 64,
    NoBackground = 128,
    NoSavedSettings = 256,
    NoMouseInputs = 512,
    MenuBar = 1024,
    HorizontalScrollbar = 2048,
    NoFocusOnAppearing = 4096,
    NoBringToFrontOnFocus = 8192,
    AlwaysVerticalScrollbar = 16384,
    AlwaysHorizontalScrollbar = 32768,
    AlwaysUseWindowPadding = 65536,
    NoNavInputs = 262144,
    NoNavFocus = 524288,
    UnsavedDocument = 1048576,
    NoDocking = 2097152,
    NavFlattened = 8388608,
    ChildWindow = 16777216,
    Tooltip = 33554432,
    Popup = 67108864,
    Modal = 134217728,
    ChildMenu = 268435456,
    DockNodeHost = 536870912,
}

pub type WindowFlags = BitFlags<WindowFlagBits>;

pub enum StyleVar {
    Alpha,
    DisabledAlpha,
    WindowPadding,
    WindowRounding,
    WindowBorderSize,
    WindowMinSize,
    WindowTitleAlign,
    ChildRounding,
    ChildBorderSize,
    FramePadding,
    FrameRounding,
    FrameBorderSize,
    FrameItemSpacing,
    FrameItemInnerSpacing,
    CellPadding,
    ScrollbarSize,
    ScrollbarRounding,
    GrabMinSize,
    GrabRounding,
    TabRounding,
    ButtonTextAlign,
    SelectableTextAlign,
}

pub enum Cond {
    None,
    Always,
    Once,
    FirstUseEver,
    Appearing
}

impl From<Cond> for ImGuiCond {
    fn from(other: Cond) -> Self {
        match other {
            Cond::None => ImGuiCond__ImGuiCond_None,
            Cond::Always => ImGuiCond__ImGuiCond_Always,
            Cond::Once => ImGuiCond__ImGuiCond_Once,
            Cond::FirstUseEver => ImGuiCond__ImGuiCond_FirstUseEver,
            Cond::Appearing => ImGuiCond__ImGuiCond_Appearing,
        }
    }
}


#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum Key {
    None = 0,
    Tab = 512,
    LeftArrow = 513,
    RightArrow = 514,
    UpArrow = 515,
    DownArrow = 516,
    PageUp = 517,
    PageDown = 518,
    Home = 519,
    End = 520,
    Insert = 521,
    Delete = 522,
    Backspace = 523,
    Space = 524,
    Enter = 525,
    Escape = 526,
    LeftCtrl = 527,
    LeftShift = 528,
    LeftAlt = 529,
    LeftSuper = 530,
    RightCtrl = 531,
    RightShift = 532,
    RightAlt = 533,
    RightSuper = 534,
    Menu = 535,
    Zero = 536,
    One = 537,
    Two = 538,
    Three = 539,
    Four = 540,
    Five = 541,
    Six = 542,
    Seven = 543,
    Eight = 544,
    Nine = 545,
    A = 546,
    B = 547,
    C = 548,
    D = 549,
    E = 550,
    F = 551,
    G = 552,
    H = 553,
    I = 554,
    J = 555,
    K = 556,
    L = 557,
    M = 558,
    N = 559,
    O = 560,
    P = 561,
    Q = 562,
    R = 563,
    S = 564,
    T = 565,
    U = 566,
    V = 567,
    W = 568,
    X = 569,
    Y = 570,
    Z = 571,
    F1 = 572,
    F2 = 573,
    F3 = 574,
    F4 = 575,
    F5 = 576,
    F6 = 577,
    F7 = 578,
    F8 = 579,
    F9 = 580,
    F10 = 581,
    F11 = 582,
    F12 = 583,
    Apostrophe = 584,
    Comma = 585,
    Minus = 586,
    Period = 587,
    Slash = 588,
    Semicolon = 589,
    Equal = 590,
    LeftBracket = 591,
    Backslash = 592,
    RightBracket = 593,
    GraveAccent = 594,
    CapsLock = 595,
    ScrollLock = 596,
    NumLock = 597,
    PrintScreen = 598,
    Pause = 599,
    Keypad0 = 600,
    Keypad1 = 601,
    Keypad2 = 602,
    Keypad3 = 603,
    Keypad4 = 604,
    Keypad5 = 605,
    Keypad6 = 606,
    Keypad7 = 607,
    Keypad8 = 608,
    Keypad9 = 609,
    KeypadDecimal = 610,
    KeypadDivide = 611,
    KeypadMultiply = 612,
    KeypadSubtract = 613,
    KeypadAdd = 614,
    KeypadEnter = 615,
    KeypadEqual = 616,
    GamepadStart = 617,
    GamepadBack = 618,
    GamepadFaceUp = 619,
    GamepadFaceDown = 620,
    GamepadFaceLeft = 621,
    GamepadFaceRight = 622,
    GamepadDpadUp = 623,
    GamepadDpadDown = 624,
    GamepadDpadLeft = 625,
    GamepadDpadRight = 626,
    GamepadL1 = 627,
    GamepadR1 = 628,
    GamepadL2 = 629,
    GamepadR2 = 630,
    GamepadL3 = 631,
    GamepadR3 = 632,
    GamepadLStickUp = 633,
    GamepadLStickDown = 634,
    GamepadLStickLeft = 635,
    GamepadLStickRight = 636,
    GamepadRStickUp = 637,
    GamepadRStickDown = 638,
    GamepadRStickLeft = 639,
    GamepadRStickRight = 640,
    ModCtrl = 641,
    ModShift = 642,
    ModAlt = 643,
    ModSuper = 644,
}

impl From<KeyCode> for Key {
    fn from(key: KeyCode) -> Self {
        match key {
            KeyCode::None => Key::None,
            KeyCode::Num0 => Key::Zero,
            KeyCode::Num1 => Key::One,
            KeyCode::Num2 => Key::Two,
            KeyCode::Num3 => Key::Three,
            KeyCode::Num4 => Key::Four,
            KeyCode::Num5 => Key::Five,
            KeyCode::Num6 => Key::Six,
            KeyCode::Num7 => Key::Seven,
            KeyCode::Num8 => Key::Eight,
            KeyCode::Num9 => Key::Nine,
            KeyCode::Numpad0 => Key::Keypad0,
            KeyCode::Numpad1 => Key::Keypad1,
            KeyCode::Numpad2 => Key::Keypad2,
            KeyCode::Numpad3 => Key::Keypad3,
            KeyCode::Numpad4 => Key::Keypad4,
            KeyCode::Numpad5 => Key::Keypad5,
            KeyCode::Numpad6 => Key::Keypad6,
            KeyCode::Numpad7 => Key::Keypad7,
            KeyCode::Numpad8 => Key::Keypad8,
            KeyCode::Numpad9 => Key::Keypad9,
            KeyCode::A => Key::A,
            KeyCode::B => Key::B,
            KeyCode::C => Key::C,
            KeyCode::D => Key::D,
            KeyCode::E => Key::E,
            KeyCode::F => Key::F,
            KeyCode::G => Key::G,
            KeyCode::H => Key::H,
            KeyCode::I => Key::I,
            KeyCode::J => Key::J,
            KeyCode::K => Key::K,
            KeyCode::L => Key::L,
            KeyCode::M => Key::M,
            KeyCode::N => Key::N,
            KeyCode::O => Key::O,
            KeyCode::P => Key::P,
            KeyCode::Q => Key::Q,
            KeyCode::R => Key::R,
            KeyCode::S => Key::S,
            KeyCode::T => Key::T,
            KeyCode::U => Key::U,
            KeyCode::V => Key::V,
            KeyCode::W => Key::W,
            KeyCode::X => Key::X,
            KeyCode::Y => Key::Y,
            KeyCode::Z => Key::Z,
            KeyCode::Escape => Key::Escape,
            KeyCode::LeftControl => Key::LeftCtrl,
            KeyCode::RightControl => Key::RightCtrl,
            KeyCode::LeftAlt => Key::LeftAlt,
            KeyCode::RightAlt => Key::RightAlt,
            KeyCode::LeftShift => Key::LeftShift,
            KeyCode::RightShift => Key::RightShift,
            KeyCode::Space => Key::Space,
            KeyCode::Backspace => Key::Backspace,
            KeyCode::F1 => Key::F1,
            KeyCode::F2 => Key::F2,
            KeyCode::F3 => Key::F3,
            KeyCode::F4 => Key::F4,
            KeyCode::F5 => Key::F5,
            KeyCode::F6 => Key::F6,
            KeyCode::F7 => Key::F7,
            KeyCode::F8 => Key::F8,
            KeyCode::F9 => Key::F9,
            KeyCode::F10 => Key::F10,
            KeyCode::F11 => Key::F11,
            KeyCode::F12 => Key::F12,
            _ => { ze_verbose!("Key {:?} not handled", key); Key::None }
        }
    }
}

// UI elements
impl Context {
    pub fn separator(&self) {
        unsafe {
            igSeparator();
        }
    }

    pub fn collapsing_header(&mut self, label: &str, flags: TreeNodeFlags) -> bool {
        let label = self.str_buffer.convert(label);
        unsafe { igCollapsingHeader_TreeNodeFlags(label, flags.bits() as i32) }
    }

    pub fn checkbox(&mut self, label: &str, checked: &mut bool) -> bool {
        let label = self.str_buffer.convert(label);
        unsafe { igCheckbox(label, checked) }
    }
    
    pub fn button(&mut self, label: &str, size: ImVec2) -> bool {
        let label = self.str_buffer.convert(label);
        unsafe { igButton(label, size) }
    }
    
    pub fn set_scroll_x(&mut self, scroll: f32) {
        unsafe { igSetScrollX_Float(scroll) }
    }

    pub fn set_scroll_y(&mut self, scroll: f32) {
        unsafe { igSetScrollY_Float(scroll) }
    }
    
    pub fn text(&mut self, text: &str) {
        let c_text = self.str_buffer.convert(text);
        unsafe { igTextUnformatted(c_text, c_text.add(text.len())) };
    }

    pub fn text_wrapped(&mut self, text: &str) {
        let text = self.str_buffer.convert(text);
        unsafe { igTextWrappedV(text, null_mut()) };
    }

    pub fn text_centered_wrapped(&mut self, text: &str, wrap_character_pos: usize) {
        let window_width = unsafe { igGetWindowWidth() };

        let mut words = vec![];
        let mut split_idx = wrap_character_pos.min(text.len());
        while text.is_char_boundary(split_idx) {
            let (a, b) = text.split_at(split_idx);
            words.push(a.to_string());
            words.push(b.to_string());
            split_idx += b.len() + wrap_character_pos;
        }

        for word in words {
            let c_word = self.str_buffer.convert(&word);
            let mut word_size = ImVec2::default();
            unsafe {
                igCalcTextSize(&mut word_size, c_word, c_word.add(word.len()), false, 0.0);
                igSetCursorPosX((window_width - word_size.x) * 0.5);
                igTextUnformatted(c_word, c_word.add(word.len()))
            }
        }
    }

    pub fn image(&mut self, srv: &ShaderResourceView, size: ImVec2) {
        let srv = srv as *const _ as *mut ShaderResourceView as *mut c_void;

        unsafe {
            igImage(
                srv,
                size,
                ImVec2::new(0.0, 0.0),
                ImVec2::new(1.0, 1.0),
                ImVec4::new(1.0, 1.0, 1.0, 1.0),
                ImVec4::new(0.0, 0.0, 0.0, 0.0),
            )
        }
    }

    pub fn image_centered(&mut self, srv: &ShaderResourceView, size: ImVec2) {
        unsafe {
            let window_width = igGetWindowWidth();
            let window_height = igGetWindowHeight();
            igSetCursorPosX((window_width - size.x) * 0.5);
            igSetCursorPosY((window_height - size.y) * 0.5);
        }
        self.image(srv, size);
    }

    pub fn image_centered_x(&mut self, srv: &ShaderResourceView, size: ImVec2) {
        unsafe {
            let window_width = igGetWindowWidth();
            igSetCursorPosX((window_width - size.x) * 0.5);
        }
        self.image(srv, size);
    }

    pub fn selectable(&mut self, label: &str, size: ImVec2) -> bool {
        let label = self.str_buffer.convert(label);
        unsafe {
            igSelectable_Bool(
                label,
                true,
                ImGuiSelectableFlags__ImGuiSelectableFlags_None as i32,
                size,
            )
        }
    }

    pub fn set_cursor_pos(&mut self, cursor_pos: ImVec2) {
        unsafe {
            igSetCursorPos(cursor_pos);
        }
    }

    pub fn set_next_window_pos(&mut self, window_pos: ImVec2, cond: Cond, pivot: ImVec2) {
        unsafe {
            igSetNextWindowPos(window_pos, cond.into(), pivot);
        }
    }

    pub fn push_style_var_f32(&mut self, var: StyleVar, val: f32) {
        unsafe { igPushStyleVar_Float(var as i32, val) }
    }

    pub fn push_style_var_vec2f32(&mut self, var: StyleVar, val: ImVec2) {
        unsafe { igPushStyleVar_Vec2(var as i32, val) }
    }

    pub fn pop_style_var(&mut self, count: i32) {
        unsafe { igPopStyleVar(count) }
    }

    pub fn begin_window(&mut self, name: &str, flags: WindowFlags) -> bool {
        let name = self.str_buffer.convert(name);
        unsafe { igBegin(name, null_mut(), flags.bits() as i32) }
    }

    pub fn begin_window_closable(
        &mut self,
        name: &str,
        open: &mut bool,
        flags: WindowFlags,
    ) -> bool {
        let name = self.str_buffer.convert(name);
        unsafe { igBegin(name, open, flags.bits() as i32) }
    }

    pub fn end_window(&mut self) {
        unsafe { igEnd() };
    }
    
    pub fn begin_popup(&mut self, id: &str, flags: WindowFlags) -> bool {
        unsafe {
            let id = self.str_buffer.convert(id);
            igBeginPopup(id, flags.bits() as i32)
        }
    }

    pub fn begin_popup_modal(&mut self, id: &str, open: &mut bool, flags: WindowFlags) -> bool {
        unsafe {
            let id = self.str_buffer.convert(id);
            igBeginPopupModal(id, open, flags.bits() as i32)
        }
    }
    
    pub fn end_popup(&mut self) {
        unsafe { igEndPopup() }
    }

    pub fn open_popup(&mut self, id: &str) {
        unsafe {
            let id = self.str_buffer.convert(id);
            igOpenPopup_Str(id, ImGuiPopupFlags__ImGuiPopupFlags_None as i32)
        }
    }

    pub fn close_current_popup(&mut self) {
        unsafe {
            igCloseCurrentPopup()
        }
    }
    
    pub fn dock_space_over_viewport(&self, viewport: &Viewport) -> ImGuiID {
        unsafe {
            igDockSpaceOverViewport(
                viewport as *const _ as *mut ImGuiViewport,
                ImGuiDockNodeFlags__ImGuiDockNodeFlags_None as i32,
                std::ptr::null(),
            )
        }
    }

    pub fn available_content_region(&self) -> ImVec2 {
        let mut vec = ImVec2::default();
        unsafe {
            igGetContentRegionAvail(&mut vec);
        }
        vec
    }

    pub fn begin_table(
        &mut self,
        name: &str,
        column_count: u32,
        flags: TableFlags,
        outer_size: ImVec2,
    ) -> bool {
        let name = self.str_buffer.convert(name);
        unsafe {
            igBeginTable(
                name,
                column_count as c_int,
                flags.bits() as i32,
                outer_size,
                0.0,
            )
        }
    }

    pub fn table_next_row(&mut self) {
        unsafe {
            igTableNextRow(ImGuiTableRowFlags__ImGuiTableRowFlags_None as i32, 0.0);
        }
    }

    pub fn table_next_column(&mut self) {
        unsafe {
            igTableNextColumn();
        }
    }

    pub fn table_setup_column(
        &mut self,
        label: &str,
        init_width_or_weight: f32,
        flags: TableColumnFlags,
    ) {
        let label = self.str_buffer.convert(label);
        unsafe {
            igTableSetupColumn(
                label,
                flags.bits() as i32,
                init_width_or_weight,
                igGetID_Str(label),
            );
        }
    }

    pub fn end_table(&mut self) {
        unsafe {
            igEndTable();
        }
    }

    pub fn is_window_hovered(&self) -> bool {
        unsafe { igIsWindowHovered(ImGuiHoveredFlags__ImGuiHoveredFlags_None as i32) }
    }

    pub fn is_item_hovered(&self) -> bool {
        unsafe { igIsItemHovered(ImGuiHoveredFlags__ImGuiHoveredFlags_None as i32) }
    }

    pub fn is_item_clicked(&self, button: MouseButton) -> bool {
        unsafe {
            igIsItemClicked(match button {
                MouseButton::Left => ImGuiMouseButton__ImGuiMouseButton_Left as i32,
                MouseButton::Middle => ImGuiMouseButton__ImGuiMouseButton_Middle as i32,
                MouseButton::Right => ImGuiMouseButton__ImGuiMouseButton_Right as i32,
            })
        }
    }

    pub fn is_mouse_double_clicked(&self, button: MouseButton) -> bool {
        unsafe {
            igIsMouseDoubleClicked(match button {
                MouseButton::Left => ImGuiMouseButton__ImGuiMouseButton_Left as i32,
                MouseButton::Middle => ImGuiMouseButton__ImGuiMouseButton_Middle as i32,
                MouseButton::Right => ImGuiMouseButton__ImGuiMouseButton_Right as i32,
            })
        }
    }
    
    pub fn is_key_pressed(&self, key: Key, repeat: bool) -> bool {
        unsafe {
            igIsKeyPressed(key as ImGuiKey, repeat)
        }
    }

    pub fn is_key_released(&self, key: Key) -> bool {
        unsafe {
            igIsKeyReleased(key as ImGuiKey)
        }
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        unsafe {
            igIsKeyDown(key as ImGuiKey)
        }
    }
    
    pub fn begin_main_menu_bar(&self) -> bool {
        unsafe { igBeginMainMenuBar() }
    }

    pub fn end_main_menu_bar(&self) {
        unsafe { igEndMainMenuBar() }
    }

    pub fn begin_tooltip(&self) {
        unsafe { igBeginTooltip() }
    }

    pub fn end_tooltip(&self) {
        unsafe { igEndTooltip() }
    }

    pub fn dummy(&self, size: ImVec2) {
        unsafe { igDummy(size) }
    }

    pub fn same_line(&self, offset_from_x: f32, spacing: f32) {
        unsafe { igSameLine(offset_from_x, spacing) }
    }

    pub fn id_from_str(&mut self, str: &str) -> ImGuiID {
        let str = self.str_buffer.convert(str);
        unsafe { igGetID_Str(str) }
    }

    pub fn next_window_dock_id(&self, id: ImGuiID) {
        unsafe {
            igSetNextWindowDockID(id, ImGuiCond__ImGuiCond_Once as i32);
        }
    }

    pub fn push_id_str(&mut self, id: &str) {
        let c_id = self.str_buffer.convert(id);
        unsafe { igPushID_Str(c_id) }
    }

    pub fn push_id_i32(&mut self, id: i32) {
        unsafe { igPushID_Int(id) }
    }

    pub fn push_id_ptr<T>(&mut self, ptr: *const T) {
        unsafe { igPushID_Ptr(ptr as *const c_void) }
    }

    pub fn pop_id(&self) {
        unsafe { igPopID() }
    }

    pub fn begin_combo(&mut self, label: &str, preview_value: &str) -> bool {
        // TODO: Rework strbuffer, i'm lazy for now
        let c_label = self.str_buffer.convert(label);
        let mut preview_value_buffer = StrBuffer::default();
        let preview_value = preview_value_buffer.convert(preview_value);
        unsafe {
            igBeginCombo(
                c_label,
                preview_value,
                ImGuiComboFlags__ImGuiComboFlags_None as i32,
            )
        }
    }

    pub fn end_combo(&self) {
        unsafe {
            igEndCombo();
        }
    }
}

impl Context {
    pub fn cursor_screen_pos(&mut self) -> ImVec2 {
        let mut pos = ImVec2::default();
        unsafe {
            igGetCursorScreenPos(&mut pos);
        }
        pos
    }

    pub fn window_add_rect_filled(&mut self, min: ImVec2, max: ImVec2, color: ImVec4) {
        unsafe {
            ImDrawList_AddRectFilled(
                igGetWindowDrawList(),
                min,
                max,
                igColorConvertFloat4ToU32(color),
                2.0,
                ImDrawFlags__ImDrawFlags_None as i32,
            )
        }
    }
}

#[bitflags]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum TreeNodeFlagBits {
    Selected = 1 << 0,
    Framed = 1 << 1,
    AllowItemOverlap = 1 << 2,
    NoTreePushOnOpen = 1 << 3,
    NoAutoOpenOnLog = 1 << 4,
    DefaultOpen = 1 << 5,
    OpenOnDoubleClick = 1 << 6,
    OpenOnArrow = 1 << 7,
    Leaf = 1 << 8,
    Bullet = 1 << 9,
    FramePadding = 1 << 10,
    SpanAvailWidth = 1 << 11,
    SpanFullWidth = 1 << 12,
    NavLeftJumpsBackHere = 1 << 13,
}

pub type TreeNodeFlags = BitFlags<TreeNodeFlagBits>;

// Tree
impl Context {
    pub fn tree_node_ex(&mut self, id: &str, flags: TreeNodeFlags) -> bool {
        let id = self.str_buffer.convert(id);
        unsafe { igTreeNodeEx_Str(id, flags.bits() as i32) }
    }

    pub fn tree_pop(&mut self) {
        unsafe {
            igTreePop();
        }
    }
}

impl Context {
    pub fn begin_child(
        &mut self,
        id: &str,
        size: ImVec2,
        border: bool,
        flags: WindowFlags,
    ) -> bool {
        let id = self.str_buffer.convert(id);
        unsafe { igBeginChild_Str(id, size, border, flags.bits() as i32) }
    }

    pub fn end_child(&self) {
        unsafe { igEndChild() }
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
    viewport: &mut Viewport,
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
    }

    let renderer_data =
        unsafe { (viewport.renderer_user_data() as *mut ViewportRendererData).as_mut() }.unwrap();

    let draw_data = unsafe { viewport.draw_data().as_ref().unwrap_unchecked() };
    renderer_data.update_buffers(device, draw_data);

    if let Ok(shader) = shader_manager.shader_modules(&"ImGui".to_string(), None) {
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
                    .descriptor_index(),
                texture: font_texture.descriptor_index(),
                texture_sampler: sampler.descriptor_index(),
            };

            device.cmd_set_shader_stages(cmd_list, &shader.pipeline_stages());

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
                renderer_data.index_buffer.as_ref().unwrap(),
                IndexBufferFormat::Uint16,
            );

            device.cmd_set_viewports(
                cmd_list,
                &[ze_gfx::backend::Viewport {
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
                    if cmd.TextureId.is_null() {
                        shader_data.texture = font_texture.descriptor_index();
                    } else {
                        let srv = cmd.TextureId as *mut ShaderResourceView;
                        shader_data.texture = unsafe { srv.as_ref() }.unwrap().descriptor_index();
                    }

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
                ze_platform::WindowFlags::from_flag(ze_platform::WindowFlagBits::Borderless),
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

unsafe extern "C" fn platform_get_window_size(_: *mut ImGuiViewport, _: *mut ImVec2) {
    todo!()
}

unsafe extern "C" fn platform_get_window_pos(vp: *mut ImGuiViewport, pos: *mut ImVec2) {
    let platform_user_data = ((*vp).PlatformUserData as *mut ViewportPlatformData)
        .as_ref()
        .unwrap_unchecked();

    (*pos).x = platform_user_data.window.position().x as f32;
    (*pos).y = platform_user_data.window.position().y as f32;
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
                    window_handle: (*platform_data).window.handle(),
                },
                None,
            )
            .unwrap();

        let mut swapchain_render_target_views = vec![];
        for i in 0..context.device.swapchain_backbuffer_count(&swapchain) {
            swapchain_render_target_views.push(
                context
                    .device
                    .create_render_target_view(&RenderTargetViewDesc {
                        resource: context
                            .device
                            .swapchain_backbuffer(&swapchain, i as u32)
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

        let old_swapchain = mem::replace(old_swapchain, MaybeUninit::uninit());

        let swapchain = context
            .device
            .create_swapchain(
                &SwapChainDesc {
                    width: size.x as u32,
                    height: size.y as u32,
                    format: PixelFormat::R8G8B8A8Unorm,
                    sample_desc: SampleDesc::default(),
                    usage_flags: TextureUsageFlags::from_flag(TextureUsageFlagBits::RenderTarget),
                    window_handle: (*platform_user_data).window.handle(),
                },
                Some(Arc::try_unwrap(old_swapchain.assume_init()).expect("Failed to unwrap arc!")),
            )
            .unwrap();

        let mut swapchain_render_target_views = vec![];
        for i in 0..context.device.swapchain_backbuffer_count(&swapchain) {
            swapchain_render_target_views.push(
                context
                    .device
                    .create_render_target_view(&RenderTargetViewDesc {
                        resource: context
                            .device
                            .swapchain_backbuffer(&swapchain, i as u32)
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

unsafe extern "C" fn renderer_swap_buffers(_: *mut ImGuiViewport, _: *mut c_void) {
    unimplemented!();
}

unsafe extern "C" fn renderer_render_window(_: *mut ImGuiViewport, _: *mut c_void) {
    unimplemented!();
}

mod renderer;
mod str_buffer;

pub extern crate ze_imgui_sys;
