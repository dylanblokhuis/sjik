use beuk::{
    ash::vk::{self, BufferUsageFlags, PipelineVertexInputStateCreateInfo},
    ctx::{RenderContext, SamplerDesc},
    memory::{BufferHandle, MemoryLocation, PipelineHandle, TextureHandle},
    pipeline::{GraphicsPipelineDescriptor, PrimitiveState},
};

pub struct PresentRenderPass {
    pipeline_handle: PipelineHandle,
    vertex_buffer: BufferHandle,
    index_buffer: BufferHandle,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

impl PresentRenderPass {
    pub fn new(ctx: &mut RenderContext) -> Self {
        let vertex_shader = beuk::shaders::Shader::from_source_text(
            &ctx.device,
            include_str!("./shader.vert"),
            "shader.vert",
            beuk::shaders::ShaderKind::Vertex,
            "main",
        );

        let fragment_shader = beuk::shaders::Shader::from_source_text(
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
                                offset: bytemuck::offset_of!(Vertex, pos) as u32,
                            },
                            vk::VertexInputAttributeDescription {
                                location: 1,
                                binding: 0,
                                format: vk::Format::R32G32_SFLOAT,
                                offset: bytemuck::offset_of!(Vertex, uv) as u32,
                            },
                        ])
                        .vertex_binding_descriptions(&[vk::VertexInputBindingDescription {
                            binding: 0,
                            stride: std::mem::size_of::<Vertex>() as u32,
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
                    blend: Default::default(),
                    multisample: Default::default(),
                });

        let vertex_buffer = ctx.buffer_manager.create_buffer_with_data(
            "vertices",
            bytemuck::cast_slice(&[
                Vertex {
                    pos: [-1.0, -1.0],
                    uv: [0.0, 0.0],
                },
                Vertex {
                    pos: [1.0, -1.0],
                    uv: [1.0, 0.0],
                },
                Vertex {
                    pos: [1.0, 1.0],
                    uv: [1.0, 1.0],
                },
                Vertex {
                    pos: [-1.0, -1.0],
                    uv: [0.0, 0.0],
                },
                Vertex {
                    pos: [1.0, 1.0],
                    uv: [1.0, 1.0],
                },
                Vertex {
                    pos: [-1.0, 1.0],
                    uv: [0.0, 1.0],
                },
            ]),
            BufferUsageFlags::VERTEX_BUFFER,
            MemoryLocation::CpuToGpu,
        );

        let index_buffer = ctx.buffer_manager.create_buffer_with_data(
            "indices",
            bytemuck::cast_slice(&[0u32, 1, 2, 3, 4, 5]),
            BufferUsageFlags::INDEX_BUFFER,
            MemoryLocation::CpuToGpu,
        );

        Self {
            pipeline_handle,
            index_buffer,
            vertex_buffer,
        }
    }

    pub fn combine_and_draw(
        &mut self,
        ctx: &RenderContext,
        ui_attachment: TextureHandle,
        media_attachment: TextureHandle,
        present_index: u32,
    ) {
        let pipeline = ctx
            .pipeline_manager
            .get_graphics_pipeline(&self.pipeline_handle);
        unsafe {
            ctx.device.update_descriptor_sets(
                &[
                    vk::WriteDescriptorSet::default()
                        .dst_set(pipeline.descriptor_sets[0])
                        .dst_binding(0)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(std::slice::from_ref(
                            &vk::DescriptorImageInfo::default()
                                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                                .image_view(
                                    ctx.texture_manager.get_buffer(ui_attachment).view.unwrap(),
                                )
                                .sampler(ctx.pipeline_manager.immutable_shader_info.get_sampler(
                                    &SamplerDesc {
                                        address_modes: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                                        mipmap_mode: vk::SamplerMipmapMode::LINEAR,
                                        texel_filter: vk::Filter::LINEAR,
                                    },
                                )),
                        )),
                    vk::WriteDescriptorSet::default()
                        .dst_set(pipeline.descriptor_sets[0])
                        .dst_binding(1)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(std::slice::from_ref(
                            &vk::DescriptorImageInfo::default()
                                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                                .image_view(
                                    ctx.texture_manager
                                        .get_buffer(media_attachment)
                                        .view
                                        .unwrap(),
                                )
                                .sampler(ctx.pipeline_manager.immutable_shader_info.get_sampler(
                                    &SamplerDesc {
                                        address_modes: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                                        mipmap_mode: vk::SamplerMipmapMode::LINEAR,
                                        texel_filter: vk::Filter::LINEAR,
                                    },
                                )),
                        )),
                ],
                &[],
            );
        };

        ctx.present_record(present_index, |ctx, command_buffer, present_index| unsafe {
            let color_attachments = &[vk::RenderingAttachmentInfo::default()
                .image_view(ctx.render_swapchain.present_image_views[present_index as usize])
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                })];

            ctx.begin_rendering(command_buffer, color_attachments, None);

            pipeline.bind(&ctx.device, command_buffer);
            ctx.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                std::slice::from_ref(&ctx.buffer_manager.get_buffer(self.vertex_buffer).buffer),
                &[0],
            );
            ctx.device.cmd_bind_index_buffer(
                command_buffer,
                ctx.buffer_manager.get_buffer(self.index_buffer).buffer,
                0,
                vk::IndexType::UINT32,
            );
            ctx.device.cmd_draw_indexed(command_buffer, 6, 1, 0, 0, 1);
            ctx.end_rendering(command_buffer);
        });
    }
}
