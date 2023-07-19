use beuk::ash::vk::{self, BufferUsageFlags, PipelineVertexInputStateCreateInfo};
use beuk::memory::MemoryLocation;
use beuk::pipeline::BlendState;
use beuk::{ctx::RenderContext, memory::PipelineHandle};
use beuk::{
    memory::BufferHandle,
    pipeline::{GraphicsPipelineDescriptor, PrimitiveState},
    shaders::Shader,
};

use lyon::geom::{point, Box2D};
use lyon::lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};
use ui_render::UiVertex;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton};

use self::scratch::{generate_layout};
use self::ui_render::Custom;
use taffy::prelude::Size;
use taffy::{style::AvailableSpace};
pub mod scratch;
pub mod tailwind;
pub mod ui_render;

pub struct UiRenderNode {
    pipeline_handle: PipelineHandle,
    vertex_buffer: Option<BufferHandle>,
    index_buffer: Option<BufferHandle>,
    last_mouse_position: Option<PhysicalPosition<f64>>,
    last_mouse_button: Option<(MouseButton, ElementState)>,
    geometry: VertexBuffers<UiVertex, u16>,
    fill_tess: FillTessellator,
}

impl UiRenderNode {
    pub fn new(ctx: &mut RenderContext) -> Self {
        let vertex_shader = Shader::from_source_text(
            &ctx.device,
            include_str!("./shader.vert"),
            "shader.vert",
            beuk::shaders::ShaderKind::Vertex,
            "main",
        );

        let fragment_shader = Shader::from_source_text(
            &ctx.device,
            include_str!("./shader.frag"),
            "shader.frag",
            beuk::shaders::ShaderKind::Fragment,
            "main",
        );

        let pipeline_handle =
            ctx.pipeline_manager
                .create_graphics_pipeline(GraphicsPipelineDescriptor {
                    vertex_shader,
                    fragment_shader,
                    vertex_input: PipelineVertexInputStateCreateInfo::default()
                        .vertex_attribute_descriptions(&[
                            vk::VertexInputAttributeDescription {
                                location: 0,
                                binding: 0,
                                format: vk::Format::R32G32_SFLOAT,
                                offset: bytemuck::offset_of!(UiVertex, point) as u32,
                            },
                            vk::VertexInputAttributeDescription {
                                location: 1,
                                binding: 0,
                                format: vk::Format::R32G32B32A32_SFLOAT,
                                offset: bytemuck::offset_of!(UiVertex, color) as u32,
                            },
                        ])
                        .vertex_binding_descriptions(&[vk::VertexInputBindingDescription {
                            binding: 0,
                            stride: std::mem::size_of::<UiVertex>() as u32,
                            input_rate: vk::VertexInputRate::VERTEX,
                        }]),
                    color_attachment_formats: &[ctx.render_swapchain.surface_format.format],
                    depth_attachment_format: vk::Format::UNDEFINED,
                    viewport: ctx.render_swapchain.surface_resolution,
                    primitive: PrimitiveState {
                        topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                        ..Default::default()
                    },
                    depth_stencil: Default::default(),
                    push_constant_range: None,
                    blend: vec![BlendState::ALPHA_BLENDING],
                });

        Self {
            pipeline_handle,
            index_buffer: None,
            vertex_buffer: None,
            last_mouse_position: None,
            last_mouse_button: None,
            fill_tess: FillTessellator::new(),
            geometry: VertexBuffers::new(),
        }
    }

    fn update_buffers(&mut self, ctx: &mut RenderContext) {
        if let Some(vertex_buffer) = self.vertex_buffer {
            let buffer = ctx.buffer_manager.get_buffer_mut(vertex_buffer);
            buffer.copy_from_slice(&self.geometry.vertices, 0);
        }

        if let Some(index_buffer) = self.index_buffer {
            let buffer = ctx.buffer_manager.get_buffer_mut(index_buffer);
            buffer.copy_from_slice(&self.geometry.indices, 0);
        }

        let vertex_buffer = ctx.buffer_manager.create_buffer_with_data(
            "vertices",
            bytemuck::cast_slice(&self.geometry.vertices),
            BufferUsageFlags::VERTEX_BUFFER,
            MemoryLocation::CpuToGpu,
        );

        let index_buffer = ctx.buffer_manager.create_buffer_with_data(
            "indices",
            bytemuck::cast_slice(&self.geometry.indices),
            BufferUsageFlags::INDEX_BUFFER,
            MemoryLocation::CpuToGpu,
        );

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
    }

    pub fn on_mouse_move(&mut self, position: PhysicalPosition<f64>) {
        self.last_mouse_position = Some(position);
    }

    pub fn on_mouse_input(&mut self, button: (MouseButton, ElementState)) {
        self.last_mouse_button = Some(button);
    }

    pub fn write_geometry(&mut self, ctx: &mut RenderContext) {
        let mut layout = generate_layout();
        let first_node = layout.root_node;

        layout
            .taffy
            .compute_layout(
                first_node,
                Size {
                    width: AvailableSpace::Definite(
                        ctx.render_swapchain.surface_resolution.width as f32 / 100.0,
                    ),
                    height: AvailableSpace::Definite(
                        ctx.render_swapchain.surface_resolution.height as f32 / 100.0,
                    ),
                },
            )
            .unwrap();

        layout.couples.sort_by(|(node_a, _, _), (node_b, _, _)| {
            let layout_a = layout.taffy.layout(*node_a).unwrap();
            let layout_b = layout.taffy.layout(*node_b).unwrap();

            // sort by x and y

            layout_a
                .location
                .y
                .partial_cmp(&layout_b.location.y)
                .unwrap()
                .then(
                    layout_a
                        .location
                        .x
                        .partial_cmp(&layout_b.location.x)
                        .unwrap(),
                )

            // layout_a
            //     .location
            //     .x
            //     .partial_cmp(&layout_b.location.x)
            //     .unwrap()
        });

        for (taffy_node, node, tw) in layout.couples.iter() {
            let layout = layout.taffy.layout(*taffy_node).unwrap();

            println!(
                "layout order {:?} {:?} {:?}",
                layout.order, node, tw.visual_style
            );

            // let min_x = layout.location.x;
            // let max_x = min_x + layout.size.width;
            // let min_y = layout.location.y;
            // let max_y = min_y + layout.size.height;

            // check if click is inside this node
            // if let Some(mouse_position) = render_context.mouse_position {
            //     if mouse_position.x >= min_x.into()
            //         && mouse_position.x <= max_x.into()
            //         && mouse_position.y >= min_y.into()
            //         && mouse_position.y <= max_y.into()
            //     {
            //         let node = self.nodes.get_mut(&node_id).unwrap();
            //         let mut node = node.node.borrow_mut();
            //         node.on_hover();
            //         if let Some((mouse_button, state)) = render_context.mouse_button {
            //             if mouse_button == MouseButton::Left && state == ElementState::Released {
            //                 node.on_click();
            //             }
            //         }
            //     }
            // }

            // map everything to top left corner in vulkan coords (-1, -1)
            let min_x = 2.0
                * (layout.location.x / ctx.render_swapchain.surface_resolution.width as f32)
                - 1.0;
            let max_x = min_x
                + 2.0 * (layout.size.width / ctx.render_swapchain.surface_resolution.width as f32);
            let min_y = 2.0
                * (layout.location.y / ctx.render_swapchain.surface_resolution.height as f32)
                - 1.0;
            let max_y = min_y
                + 2.0
                    * (layout.size.height / ctx.render_swapchain.surface_resolution.height as f32);

            self.fill_tess
                .tessellate_rectangle(
                    &Box2D::new(point(min_x, min_y), point(max_x, max_y)),
                    &FillOptions::default(),
                    &mut BuffersBuilder::new(
                        &mut self.geometry,
                        Custom {
                            color: tw.visual_style.background_color,
                        },
                    ),
                )
                .unwrap();
        }
        self.update_buffers(ctx);
    }

    pub fn draw(&mut self, ctx: &mut RenderContext, present_index: u32) {
        ctx.present_record(
            present_index,
            |ctx, command_buffer, present_index: u32| unsafe {
                let color_attachments = &[vk::RenderingAttachmentInfo::default()
                    .image_view(ctx.render_swapchain.present_image_views[present_index as usize])
                    .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .load_op(vk::AttachmentLoadOp::LOAD)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .clear_value(vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 1.0],
                        },
                    })];

                ctx.begin_rendering(command_buffer, color_attachments, None);

                let pipeline = ctx
                    .pipeline_manager
                    .get_graphics_pipeline(&self.pipeline_handle);
                pipeline.bind(&ctx.device, command_buffer);
                ctx.device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    std::slice::from_ref(
                        &ctx.buffer_manager
                            .get_buffer(self.vertex_buffer.unwrap())
                            .buffer,
                    ),
                    &[0],
                );
                ctx.device.cmd_bind_index_buffer(
                    command_buffer,
                    ctx.buffer_manager
                        .get_buffer(self.index_buffer.unwrap())
                        .buffer,
                    0,
                    vk::IndexType::UINT16,
                );
                ctx.device.cmd_draw_indexed(
                    command_buffer,
                    self.geometry.indices.len() as u32,
                    1,
                    0,
                    0,
                    1,
                );
                ctx.end_rendering(command_buffer);
            },
        );

        self.last_mouse_button = None;
    }
}
