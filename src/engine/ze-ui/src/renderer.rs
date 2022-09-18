use crate::font::Font;
use crate::{LayoutContext, UiState};
use harfbuzz_rs::GlyphBuffer;
use std::mem::size_of;
use std::slice;
use std::sync::Arc;
use ze_core::color::Color4f32;
use ze_core::maths::{Matrix4f32, RectI32, Vec2f32};
use ze_gfx::backend::{
    BlendFactor, BlendOp, Buffer, BufferDesc, BufferSRV, BufferUsageFlags, CommandList, Device,
    IndexBufferFormat, MemoryLocation, PipelineBlendState, PipelineRenderTargetBlendDesc,
    ResourceState, ShaderResourceView, ShaderResourceViewDesc, ShaderResourceViewResource,
    ShaderResourceViewType, Viewport,
};
use ze_gfx::PixelFormat;
use ze_shader_system::{ShaderManager, ShaderModules};

pub type Index = u16;

#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct Vertex {
    position: Vec2f32,
    texcoord: Vec2f32,
    color: Color4f32,
}

impl Vertex {
    pub fn new(position: Vec2f32, texcoord: Vec2f32, color: Color4f32) -> Self {
        Self {
            position,
            texcoord,
            color,
        }
    }
}

pub struct DrawCommand {
    vertices_start_idx: usize,
    vertices_count: usize,
    indices_start_idx: usize,
    indices_count: usize,
    shader_modules: Arc<ShaderModules>,
    z_order: i32,
}

/// Hold context for drawing into a viewport
pub struct DrawContext<'a> {
    _glyph_cache: &'a mut crate::glyph_cache::Cache,
    commands: Vec<DrawCommand>,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

impl<'a> DrawContext<'a> {
    pub fn new(glyph_cache: &'a mut crate::glyph_cache::Cache) -> Self {
        Self {
            _glyph_cache: glyph_cache,
            commands: vec![],
            vertices: vec![],
            indices: vec![],
        }
    }

    fn sort(&mut self) -> Vec<Vec<usize>> {
        let mut batches = vec![];
        if !self.commands.is_empty() {
            self.commands.sort_by(|a, b| {
                (a.shader_modules.as_ref() as *const ShaderModules as usize)
                    .cmp(&(b.shader_modules.as_ref() as *const ShaderModules as usize))
            });

            let mut current_batch = vec![];
            for (i, _) in self.commands.iter().enumerate() {
                current_batch.push(i);
                /* if current_batch.is_empty() {
                } else {
                    current_batch.push(i);
                }
                } else if Arc::ptr_eq(
                    &self.commands[current_batch[0]].shader_modules,
                    &command.shader_modules,
                ) {
                    current_batch.push(i);
                } else {
                    batches.push(current_batch);
                    current_batch = vec![];
                }*/
            }

            if !current_batch.is_empty() {
                batches.push(current_batch);
            }

            // Sort batch commands by z-order
            for batch in &mut batches {
                batch.sort_by(|a, b| {
                    let a_command = &self.commands[*a];
                    let b_command = &self.commands[*b];
                    a_command.z_order.cmp(&b_command.z_order)
                });
            }
        }

        batches
    }

    pub fn rectangle(
        &mut self,
        position: Vec2f32,
        size: Vec2f32,
        color: Color4f32,
        z_order: Option<i32>,
    ) -> &mut DrawCommand {
        let vertex_count = 4;
        let index_count = 6;

        let ((vertex_start_index, index_start_index), (vertices, indices)) =
            self.reserve_vertex_and_index_slice(vertex_count, index_count);

        vertices[0] = Vertex::new(
            Vec2f32::new(0.0, 0.0) * size + position,
            Vec2f32::new(0.0, 0.0),
            color,
        );

        vertices[1] = Vertex::new(
            Vec2f32::new(1.0, 0.0) * size + position,
            Vec2f32::new(1.0, 0.0),
            color,
        );

        vertices[2] = Vertex::new(
            Vec2f32::new(1.0, 1.0) * size + position,
            Vec2f32::new(1.0, 1.0),
            color,
        );

        vertices[3] = Vertex::new(
            Vec2f32::new(0.0, 1.0) * size + position,
            Vec2f32::new(0.0, 1.0),
            color,
        );

        indices[0] = 0;
        indices[1] = 1;
        indices[2] = 2;
        indices[3] = 2;
        indices[4] = 3;
        indices[5] = 0;

        self.commands.push(DrawCommand {
            vertices_start_idx: vertex_start_index,
            vertices_count: vertex_count,
            indices_start_idx: index_start_index,
            indices_count: index_count,
            shader_modules: Arc::new(Default::default()),
            z_order: if let Some(z_order) = z_order {
                z_order
            } else {
                self.commands.len() as i32
            },
        });

        self.commands.last_mut().unwrap()
    }

    pub fn text(&mut self, _position: Vec2f32, _font: &Font, _shaped_buffer: &GlyphBuffer) {
        /*let glyph_positions = shaped_buffer.get_glyph_positions();
        let glyph_infos = shaped_buffer.get_glyph_infos();
        let mut current_advance_x = 0.0;
        let mut current_advance_y = 0.0;
        for (info, glyph_position) in glyph_infos.iter().zip(glyph_positions) {
            let cached_glyph = self
                .glyph_cache
                .glyph(font.hash(), font.glyph_index(info.codepoint));

            self.rectangle(
                position
                    + Vec2f32::new(
                        current_advance_x + glyph_position.x_offset as f32,
                        current_advance_y + glyph_position.y_offset as f32,
                    ),
                Vec2f32::new(4.0, 32.0),
                Color4f32::new(current_advance_x / 10.0, 0.0, 1.0, 1.0),
                None,
            );

            current_advance_x += glyph_position.x_advance as f32;
            current_advance_y += glyph_position.y_advance as f32;
        }
        */
    }

    fn reserve_vertex_and_index_slice(
        &mut self,
        vertex_count: usize,
        index_count: usize,
    ) -> ((usize, usize), (&mut [Vertex], &mut [u16])) {
        let vertex_start_idx = if self.vertices.is_empty() {
            0
        } else {
            self.vertices.len()
        };

        let index_start_idx = if self.indices.is_empty() {
            0
        } else {
            self.indices.len()
        };

        self.vertices
            .resize(self.vertices.len() + vertex_count, Vertex::default());
        self.indices.resize(self.indices.len() + index_count, 0);

        (
            (vertex_start_idx, index_start_idx),
            (
                &mut self.vertices[vertex_start_idx..vertex_start_idx + vertex_count],
                &mut self.indices[index_start_idx..index_start_idx + index_count],
            ),
        )
    }

    fn _glyph_cache(&mut self) -> &mut crate::glyph_cache::Cache {
        self._glyph_cache
    }
}

pub struct ViewportRenderer {
    device: Arc<dyn Device>,
    shader_manager: Arc<ShaderManager>,
    vertex_buffer: Option<Arc<Buffer>>,
    vertex_buffer_srv: Option<ShaderResourceView>,
    index_buffer: Option<Arc<Buffer>>,
}

/// A batch of draw commands with the same shader
#[derive(Default)]
struct DrawCommandBatch {
    vertex_count: usize,
    index_count: usize,
    base_vertex_location: usize,
    base_index_location: usize,
}

impl ViewportRenderer {
    pub fn new(device: Arc<dyn Device>, shader_manager: Arc<ShaderManager>) -> Self {
        Self {
            device,
            shader_manager,
            vertex_buffer: None,
            vertex_buffer_srv: None,
            index_buffer: None,
        }
    }

    fn generate_batches(
        &mut self,
        draw_context: &DrawContext,
        sorted_commands: Vec<Vec<usize>>,
    ) -> Vec<DrawCommandBatch> {
        let mut batches = vec![];

        let vertex_buffer_size =
            (draw_context.vertices.len() as u64) * (size_of::<Vertex>() as u64);
        let index_buffer_size = (draw_context.indices.len() as u64) * (size_of::<Index>() as u64);

        if Self::create_or_resize_buffer(&self.device, &mut self.vertex_buffer, vertex_buffer_size)
        {
            let srv = self
                .device
                .create_shader_resource_view(&ShaderResourceViewDesc {
                    resource: ShaderResourceViewResource::Buffer(
                        self.vertex_buffer.as_ref().unwrap().clone(),
                    ),
                    format: PixelFormat::Unknown,
                    ty: ShaderResourceViewType::Buffer(BufferSRV {
                        first_element_index: 0,
                        element_count: draw_context.vertices.len() as u32,
                        element_size_in_bytes: size_of::<Vertex>() as u32,
                    }),
                })
                .expect("Failed to create vertex buffer srv");
            self.vertex_buffer_srv = Some(srv);
        }

        Self::create_or_resize_buffer(&self.device, &mut self.index_buffer, index_buffer_size);

        if let (Some(vertex_buffer), Some(index_buffer)) = (&self.vertex_buffer, &self.index_buffer)
        {
            let mut vertex_ptr =
                self.device.buffer_mapped_ptr(vertex_buffer).unwrap() as *mut Vertex;
            let mut index_ptr = self.device.buffer_mapped_ptr(index_buffer).unwrap() as *mut Index;

            let mut vertex_idx = 0;
            let mut index_idx = 0;

            for sorted_command_batch in sorted_commands {
                let mut batch = DrawCommandBatch {
                    vertex_count: 0,
                    index_count: 0,
                    base_vertex_location: vertex_idx,
                    base_index_location: index_idx,
                };

                for command_index in sorted_command_batch {
                    let command = &draw_context.commands[command_index];

                    unsafe {
                        let dst_vertex_slice =
                            slice::from_raw_parts_mut(vertex_ptr, command.vertices_count);

                        let dst_index_slice =
                            slice::from_raw_parts_mut(index_ptr, command.indices_count);

                        dst_vertex_slice.copy_from_slice(
                            &draw_context.vertices[command.vertices_start_idx
                                ..command.vertices_start_idx + command.vertices_count],
                        );

                        let src_indices = draw_context.indices[command.indices_start_idx
                            ..command.indices_start_idx + command.indices_count]
                            .iter()
                            .map(|index| (batch.vertex_count as u16) + *index)
                            .collect::<Vec<_>>();

                        dst_index_slice.copy_from_slice(&src_indices);
                    }

                    vertex_ptr = unsafe { vertex_ptr.add(command.vertices_count) };
                    index_ptr = unsafe { index_ptr.add(command.indices_count) };

                    batch.vertex_count += command.vertices_count;
                    batch.index_count += command.indices_count;
                }

                vertex_idx += batch.vertex_count;
                index_idx += batch.index_count;

                batches.push(batch);
            }
        }

        batches
    }

    pub fn draw(
        &mut self,
        delta_time: f32,
        ui_state: &mut UiState,
        cmd_list: &mut CommandList,
        glyph_cache: &mut crate::glyph_cache::Cache,
        font_cache: &mut crate::font::FontCache,
        viewport_size: Vec2f32,
    ) {
        let mut layout_context = LayoutContext::new(font_cache);
        let mut draw_context = DrawContext::new(glyph_cache);
        ui_state.draw(
            delta_time,
            &mut layout_context,
            &mut draw_context,
            viewport_size,
        );

        let sorted_commands = draw_context.sort();
        let batches = self.generate_batches(&draw_context, sorted_commands);

        if let Ok(shader) = self
            .shader_manager
            .shader_modules(&"zeui_base".to_string(), None)
        {
            self.device.cmd_set_viewports(
                cmd_list,
                &[Viewport {
                    position: Default::default(),
                    size: viewport_size,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );

            self.device.cmd_set_scissors(
                cmd_list,
                &[RectI32 {
                    x: 0,
                    y: 0,
                    width: viewport_size.x as i32,
                    height: viewport_size.y as i32,
                }],
            );

            #[repr(C)]
            struct ShaderData {
                projection_matrix: Matrix4f32,
                base_vertex_location: u32,
                vertex_buffer: u32,
                texture: u32,
                texture_sampler: u32,
            }

            #[rustfmt::skip]
            let projection_matrix = {
                let left = 0.0;
                let right = 0.0 + viewport_size.x;
                let top = 0.0;
                let bottom = 0.0 + viewport_size.y;
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
                vertex_buffer: self.vertex_buffer_srv.as_ref().unwrap().descriptor_index(),
                texture: 0,         //font_texture.descriptor_index(),
                texture_sampler: 0, //sampler.descriptor_index(),
            };

            self.device
                .cmd_set_shader_stages(cmd_list, &shader.pipeline_stages());

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
            self.device.cmd_set_blend_state(cmd_list, &blend_state);
            self.device.cmd_bind_index_buffer(
                cmd_list,
                self.index_buffer.as_ref().unwrap(),
                IndexBufferFormat::Uint16,
            );

            for batch in batches {
                shader_data.base_vertex_location = batch.base_vertex_location as u32;

                self.device.cmd_push_constants(cmd_list, 0, unsafe {
                    slice::from_raw_parts(
                        (&shader_data as *const ShaderData) as *const u8,
                        size_of::<ShaderData>(),
                    )
                });

                self.device.cmd_draw_indexed(
                    cmd_list,
                    batch.index_count as u32,
                    1,
                    batch.base_index_location as u32,
                    0,
                );
            }
        }
    }

    fn create_or_resize_buffer(
        device: &Arc<dyn Device>,
        buffer: &mut Option<Arc<Buffer>>,
        size_bytes: u64,
    ) -> bool {
        if size_bytes == 0 {
            return false;
        }

        if let Some(buffer) = buffer {
            if buffer.info.size_bytes >= size_bytes {
                return false;
            }
        }

        let new_buffer = device
            .create_buffer(
                &BufferDesc {
                    size_bytes,
                    usage: BufferUsageFlags::default(),
                    memory_location: MemoryLocation::CpuToGpu,
                    default_resource_state: ResourceState::Common,
                },
                "Viewport Buffer",
            )
            .expect("Failed to create Viewport vertex buffer");

        *buffer = Some(Arc::new(new_buffer));
        true
    }
}
