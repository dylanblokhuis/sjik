use beuk::ash::vk::{
    self, BufferUsageFlags, PipelineVertexInputStateCreateInfo,
};
use beuk::memory::MemoryLocation;
use beuk::{ctx::RenderContext, memory::PipelineHandle};
use beuk::{
    memory::BufferHandle,
    pipeline::{GraphicsPipelineDescriptor, PrimitiveState},
    shaders::Shader,
};
use el::UiVertex;
use lyon::lyon_tessellation::VertexBuffers;

use self::el::{Props, UiContext, UiRenderContext};

pub mod el;
pub mod tailwind;

pub struct UiRenderNode {
    pipeline_handle: PipelineHandle,
    vertex_buffer: Option<BufferHandle>,
    index_buffer: Option<BufferHandle>,
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
                });

        Self {
            pipeline_handle,
            index_buffer: None,
            vertex_buffer: None,
        }
    }

    fn update_buffers(&mut self, ctx: &mut RenderContext, geometry: &VertexBuffers<UiVertex, u16>) {
        if let Some(vertex_buffer) = self.vertex_buffer {
            let buffer = ctx.buffer_manager.get_buffer_mut(vertex_buffer);
            buffer.copy_from_slice(&geometry.vertices, 0);
        }

        if let Some(index_buffer) = self.index_buffer {
            let buffer = ctx.buffer_manager.get_buffer_mut(index_buffer);
            buffer.copy_from_slice(&geometry.indices, 0);
        }

        let vertex_buffer = ctx.buffer_manager.create_buffer_with_data(
            "vertices",
            bytemuck::cast_slice(&geometry.vertices),
            BufferUsageFlags::VERTEX_BUFFER,
            MemoryLocation::CpuToGpu,
        );

        let index_buffer = ctx.buffer_manager.create_buffer_with_data(
            "indices",
            bytemuck::cast_slice(&geometry.indices),
            BufferUsageFlags::INDEX_BUFFER,
            MemoryLocation::CpuToGpu,
        );

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
    }

    pub fn draw(&mut self, ctx: &mut RenderContext, present_index: u32) {
        let mut ui = UiContext::default();
        ui.div("bg-red-200 w-200 h-200", Props, |ui| {
            ui.div("bg-green-500 h-20", Props, |_| {});
            ui.div("bg-blue-500 h-20", Props, |_| {});
        });

        let render_context = UiRenderContext::new((
            ctx.render_swapchain.surface_resolution.width,
            ctx.render_swapchain.surface_resolution.height,
        ));
        let geometry = ui.finish(render_context);
        self.update_buffers(ctx, &geometry);

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
                    geometry.indices.len() as u32,
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
