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
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, WindowEvent};

use self::scratch::{EntityId, ENTITIES};
use taffy::prelude::Size;
use taffy::style::AvailableSpace;
pub mod scratch;
pub mod tailwind;

use lyon::lyon_tessellation::{FillVertex, FillVertexConstructor};

#[repr(C, align(16))]
#[derive(Clone, Debug, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiVertex {
    pub point: [f32; 2],
    pub color: [f32; 4],
    pub _padding: [f32; 2],
}

pub struct Custom {
    pub color: [f32; 4],
}
impl FillVertexConstructor<UiVertex> for Custom {
    fn new_vertex(&mut self, vertex: FillVertex) -> UiVertex {
        UiVertex {
            point: vertex.position().to_array(),
            color: self.color,
            ..Default::default()
        }
    }
}

pub struct UiRenderNode {
    pipeline_handle: PipelineHandle,
    vertex_buffer: Option<BufferHandle>,
    index_buffer: Option<BufferHandle>,
    pub geometry: VertexBuffers<UiVertex, u16>,
    fill_tess: FillTessellator,
    last_mouse_position: PhysicalPosition<f64>,
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
            fill_tess: FillTessellator::new(),
            geometry: VertexBuffers::new(),
            last_mouse_position: PhysicalPosition::new(0.0, 0.0),
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

    pub fn handle_events(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.bubble_on_hover_events(position);
                self.last_mouse_position = position;
            }
            WindowEvent::Touch(touch) => {
                self.bubble_on_click_events(touch.location);
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if button != MouseButton::Left || state != ElementState::Pressed {
                    return;
                }
                self.bubble_on_click_events(self.last_mouse_position);
            }
            _ => {}
        }
    }

    pub fn bubble_on_hover_events(&self, position: PhysicalPosition<f64>) {
        let Some(maybe_element) = self.find_element_on_point(position.x as f32, position.y as f32)
        else {
            return;
        };
        let entities = ENTITIES.read().unwrap();
        let element = entities.views.get(&EntityId(maybe_element)).unwrap();
        let Some(on_hover) = element.on_hover.as_ref() else {
            return;
        };
        on_hover();
    }

    pub fn bubble_on_click_events(&self, position: PhysicalPosition<f64>) {
        let Some(maybe_element) = self.find_element_on_point(position.x as f32, position.y as f32)
        else {
            return;
        };
        let entities = ENTITIES.read().unwrap();
        let element = entities.views.get(&EntityId(maybe_element)).unwrap();
        let Some(on_click) = element.on_click.as_ref() else {
            return;
        };
        on_click();
    }

    fn find_element_on_point(&self, x: f32, y: f32) -> Option<taffy::node::Node> {
        let entities = ENTITIES.read().unwrap();
        self.find_element_on_point_recursive(x, y, entities.views.first_key_value().unwrap().0 .0)
    }

    fn find_element_on_point_recursive(
        &self,
        x: f32,
        y: f32,
        node: taffy::node::Node,
    ) -> Option<taffy::node::Node> {
        let entities = ENTITIES.read().unwrap();
        let layout = entities.taffy.layout(node).unwrap();

        // check if point is inside the current node
        if x >= layout.location.x
            && x <= layout.location.x + layout.size.width
            && y >= layout.location.y
            && y <= layout.location.y + layout.size.height
        {
            // if point is inside, check children
            let children = entities.taffy.children(node).unwrap();
            for child in children.iter() {
                if let Some(found) = self.find_element_on_point_recursive(x, y, *child) {
                    return Some(found); // if a child contains the point, return it
                }
            }

            // if no child contains the point, the current node does
            return Some(node);
        }

        // point is not inside the current node
        None
    }

    pub fn write_geometry(&mut self, ctx: &mut RenderContext) {
        self.geometry = VertexBuffers::new();
        self.fill_tess = FillTessellator::new();

        let root_node = {
            let entities = ENTITIES.read().unwrap();
            entities.views.first_key_value().unwrap().0 .0
        };

        {
            let mut entities = ENTITIES.write().unwrap();

            entities
                .taffy
                .compute_layout(
                    root_node,
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
        }

        fn recursive_tess(
            s: &mut UiRenderNode,
            ctx: &RenderContext,
            node: taffy::node::Node,
            parent_location: &taffy::geometry::Point<f32>,
        ) {
            let entities = ENTITIES.read().unwrap();
            let layout = entities.taffy.layout(node).unwrap();
            let tw: tailwind::Tailwind = entities.views.get(&EntityId(node)).unwrap().tw.clone();
            let location = taffy::geometry::Point {
                x: parent_location.x + layout.location.x,
                y: parent_location.y + layout.location.y,
            };

            let min_x =
                2.0 * (location.x / ctx.render_swapchain.surface_resolution.width as f32) - 1.0;
            let max_x = min_x
                + 2.0 * (layout.size.width / ctx.render_swapchain.surface_resolution.width as f32);
            let min_y =
                2.0 * (location.y / ctx.render_swapchain.surface_resolution.height as f32) - 1.0;
            let max_y = min_y
                + 2.0
                    * (layout.size.height / ctx.render_swapchain.surface_resolution.height as f32);

            s.fill_tess
                .tessellate_rectangle(
                    &Box2D::new(point(min_x, min_y), point(max_x, max_y)),
                    &FillOptions::default(),
                    &mut BuffersBuilder::new(
                        &mut s.geometry,
                        Custom {
                            color: tw.visual_style.background_color,
                        },
                    ),
                )
                .unwrap();

            let children = entities.taffy.children(node).unwrap();
            for item in children.iter() {
                recursive_tess(s, ctx, *item, &location);
            }
        }

        recursive_tess(self, ctx, root_node, &taffy::geometry::Point::ZERO);

        self.update_buffers(ctx);
    }

    pub fn draw(&mut self, ctx: &RenderContext, present_index: u32) {
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
    }
}
