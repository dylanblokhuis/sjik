use beuk::ash::vk::{
    self, BufferUsageFlags, DeviceSize, ImageCreateInfo, PipelineVertexInputStateCreateInfo,
    SamplerYcbcrConversionInfo,
};
use beuk::ctx::SamplerDesc;
use beuk::memory::{MemoryLocation, TextureHandle};
use beuk::{
    ctx::RenderContext,
    memory::{BufferHandle, PipelineHandle},
    pipeline::{GraphicsPipelineDescriptor, PrimitiveState},
    shaders::Shader,
};

use crate::CurrentVideo;

#[repr(C, align(16))]
#[derive(Clone, Debug, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Debug, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    index: i32,
}

pub struct MediaRenderPass {
    pipeline_handle: Option<PipelineHandle>,
    vertex_buffer: BufferHandle,
    index_buffer: BufferHandle,
    pub yuv: Option<TextureHandle>,
    pub frame_buffer: Option<BufferHandle>,
    pub uniform_buffer: Option<BufferHandle>,
    pub attachment: TextureHandle,
}

impl MediaRenderPass {
    pub fn new(ctx: &mut RenderContext) -> Self {
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

        let attachment_format = ctx.render_swapchain.surface_format.format;
        let (attachment_handle, _) = ctx.texture_manager.create_texture(
            "media",
            &ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format: attachment_format,
                extent: vk::Extent3D {
                    width: ctx.render_swapchain.surface_resolution.width,
                    height: ctx.render_swapchain.surface_resolution.height,
                    depth: 1,
                },
                samples: vk::SampleCountFlags::TYPE_1,
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT
                    | vk::ImageUsageFlags::TRANSFER_SRC
                    | vk::ImageUsageFlags::SAMPLED,
                mip_levels: 1,
                array_layers: 1,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            },
        );
        ctx.texture_manager
            .get_buffer_mut(attachment_handle)
            .create_view(&ctx.device);

        Self {
            pipeline_handle: None,
            vertex_buffer,
            index_buffer,
            yuv: None,
            frame_buffer: None,
            uniform_buffer: None,
            attachment: attachment_handle,
        }
    }

    pub fn setup_buffers(&mut self, ctx: &mut RenderContext, current_video: &CurrentVideo) {
        if self.yuv.is_some() && self.frame_buffer.is_some() && self.uniform_buffer.is_some() {
            return;
        }

        let video_format = vk::Format::G8_B8R8_2PLANE_420_UNORM;
        let index = match video_format {
            // YUV420SP
            vk::Format::G8_B8R8_2PLANE_420_UNORM => 0,
            // YUV420P
            vk::Format::G8_B8_R8_3PLANE_420_UNORM => 1,
            // YUV420SP HDR
            vk::Format::G10X6_B10X6R10X6_2PLANE_420_UNORM_3PACK16 => 2,
            // YUV420P HDR
            vk::Format::G10X6_B10X6_R10X6_3PLANE_420_UNORM_3PACK16 => 3,
            _ => panic!("Unsupported format"),
        };

        let vertex_shader = Shader::from_source_text(
            &ctx.device,
            include_str!("./shader.vert"),
            "shader.vert",
            beuk::shaders::ShaderKind::Vertex,
            "main",
        );

        // planning on compiling this based on the required format
        let fragment_shader = Shader::from_source_text(
            &ctx.device,
            r#"#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (set = 0, binding = 0) uniform sampler2D textureLinearYUV420P;
layout (set = 0, binding = 1) uniform Uniform {
    int index;
} ubo;

layout (location = 0) in vec2 o_uv;
layout (location = 0) out vec4 a_frag_color;

void main() {
    a_frag_color = texture(textureLinearYUV420P, o_uv);
}
            "#,
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
                    color_attachment_formats: &[ctx
                        .texture_manager
                        .get_buffer(self.attachment)
                        .format],
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

        let (yuv, _) = ctx.texture_manager.create_texture(
            "yuv420",
            &ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format: video_format,
                extent: vk::Extent3D {
                    width: current_video.width,
                    height: current_video.height,
                    depth: 1,
                },
                samples: vk::SampleCountFlags::TYPE_1,
                usage: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                mip_levels: 1,
                array_layers: 1,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            },
        );

        let uniform_handle = ctx.buffer_manager.create_buffer_with_data(
            "uniform",
            bytemuck::cast_slice(&[Uniform { index }]),
            BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
        );

        {
            let uniform = ctx.buffer_manager.get_buffer(uniform_handle);
            let texture = ctx.texture_manager.get_buffer_mut(yuv);

            let (sampler_conversion, _) = ctx
                .pipeline_manager
                .immutable_shader_info
                .get_yuv_conversion_sampler(
                    &ctx.device,
                    SamplerDesc {
                        texel_filter: vk::Filter::LINEAR,
                        mipmap_mode: vk::SamplerMipmapMode::LINEAR,
                        address_modes: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                    },
                    video_format,
                );

            let view = unsafe {
                let mut conversion_info =
                    SamplerYcbcrConversionInfo::default().conversion(sampler_conversion);

                ctx.device.create_image_view(
                    &vk::ImageViewCreateInfo {
                        view_type: vk::ImageViewType::TYPE_2D,
                        format: texture.format,
                        components: vk::ComponentMapping {
                            r: vk::ComponentSwizzle::R,
                            g: vk::ComponentSwizzle::G,
                            b: vk::ComponentSwizzle::B,
                            a: vk::ComponentSwizzle::A,
                        },
                        subresource_range: vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            level_count: 1,
                            layer_count: 1,
                            ..Default::default()
                        },
                        image: texture.image,
                        ..Default::default()
                    }
                    .push_next(&mut conversion_info),
                    None,
                )
            }
            .unwrap();
            let pipeline = ctx.pipeline_manager.get_graphics_pipeline(&pipeline_handle);
            unsafe {
                ctx.device.update_descriptor_sets(
                    &[
                        // 2 plane sampler
                        vk::WriteDescriptorSet::default()
                            .dst_set(pipeline.descriptor_sets[0])
                            .dst_binding(0)
                            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                            .image_info(std::slice::from_ref(
                                &vk::DescriptorImageInfo::default()
                                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                                    .image_view(view),
                            )),
                        // // 3 plane sampler
                        // vk::WriteDescriptorSet::default()
                        //     .dst_set(pipeline.descriptor_sets[0])
                        //     .dst_binding(0)
                        //     .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        //     .dst_array_element(1)
                        //     .image_info(std::slice::from_ref(
                        //         &vk::DescriptorImageInfo::default()
                        //             .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        //             .image_view(view),
                        //     )),
                        // // 2 plane sampler hdr
                        // vk::WriteDescriptorSet::default()
                        //     .dst_set(pipeline.descriptor_sets[0])
                        //     .dst_binding(0)
                        //     .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        //     .dst_array_element(2)
                        //     .image_info(std::slice::from_ref(
                        //         &vk::DescriptorImageInfo::default()
                        //             .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        //             .image_view(view),
                        //     )),
                        // // 3 plane sampler hdr
                        // vk::WriteDescriptorSet::default()
                        //     .dst_set(pipeline.descriptor_sets[0])
                        //     .dst_binding(0)
                        //     .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        //     .dst_array_element(3)
                        //     .image_info(std::slice::from_ref(
                        //         &vk::DescriptorImageInfo::default()
                        //             .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        //             .image_view(view),
                        //     )),
                        // uniform
                        vk::WriteDescriptorSet::default()
                            .dst_set(pipeline.descriptor_sets[0])
                            .dst_binding(1)
                            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                            .buffer_info(std::slice::from_ref(
                                &vk::DescriptorBufferInfo::default()
                                    .buffer(uniform.buffer)
                                    .range(uniform.size)
                                    .offset(0),
                            )),
                    ],
                    &[],
                );
            };
        }

        let size = current_video.width * current_video.height;
        let fullscreen_of_yuv = size + (size / 2);

        log::debug!("Creating frame buffer of size {}", fullscreen_of_yuv);

        let frame_buffer = ctx.buffer_manager.create_buffer(
            "frame",
            fullscreen_of_yuv as DeviceSize,
            vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::CpuToGpu,
        );

        log::debug!("Created frame buffer of size {}", fullscreen_of_yuv);

        self.yuv = Some(yuv);
        self.frame_buffer = Some(frame_buffer);
        self.uniform_buffer = Some(uniform_handle);
        self.pipeline_handle = Some(pipeline_handle);
    }

    pub fn copy_yuv420_frame_to_gpu(&self, ctx: &RenderContext) {
        let Some(yuv) = self.yuv else {
            return;
        };

        let Some(frame_buffer) = self.frame_buffer else {
            return;
        };

        ctx.record(
            ctx.setup_command_buffer,
            Some(ctx.setup_commands_reuse_fence),
            |ctx, command_buffer| unsafe {
                let texture = ctx.texture_manager.get_buffer(yuv);
                let frame_buffer = ctx.buffer_manager.get_buffer(frame_buffer);

                let layout_transition_barriers = vk::ImageMemoryBarrier::default()
                    .image(texture.image)
                    .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ)
                    .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(
                                vk::ImageAspectFlags::PLANE_0 | vk::ImageAspectFlags::PLANE_1,
                            )
                            .layer_count(1)
                            .level_count(1),
                    );

                ctx.device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[layout_transition_barriers],
                );

                let y_plane_size = (texture.extent.width * texture.extent.height) as usize;

                // println!("copy y_plane {:?}", y_plane_size);
                // println!("copy uv_plane {:?}", uv_plane_size);

                ctx.device.cmd_copy_buffer_to_image(
                    command_buffer,
                    frame_buffer.buffer,
                    texture.image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[vk::BufferImageCopy::default()
                        .buffer_offset(0)
                        .buffer_row_length(0)
                        .buffer_image_height(0)
                        .image_subresource(vk::ImageSubresourceLayers {
                            aspect_mask: vk::ImageAspectFlags::PLANE_0,
                            layer_count: 1,
                            ..Default::default()
                        })
                        .image_extent(texture.extent)],
                );

                ctx.device.cmd_copy_buffer_to_image(
                    command_buffer,
                    frame_buffer.buffer,
                    texture.image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[vk::BufferImageCopy::default()
                        .buffer_offset(y_plane_size as DeviceSize)
                        .buffer_row_length(0)
                        .buffer_image_height(0)
                        .image_subresource(vk::ImageSubresourceLayers {
                            aspect_mask: vk::ImageAspectFlags::PLANE_1,
                            layer_count: 1,
                            ..Default::default()
                        })
                        .image_extent(vk::Extent3D {
                            width: texture.extent.width / 2,
                            height: texture.extent.height / 2,
                            depth: 1,
                        })],
                );

                // ctx.device.cmd_copy_buffer_to_image(
                //     command_buffer,
                //     frame_buffer.buffer,
                //     texture.image,
                //     vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                //     &[vk::BufferImageCopy::default()
                //         .buffer_offset((y_plane_size + uv_plane_size) as DeviceSize)
                //         .buffer_row_length(0)
                //         .buffer_image_height(0)
                //         .image_subresource(vk::ImageSubresourceLayers {
                //             aspect_mask: vk::ImageAspectFlags::PLANE_2,
                //             layer_count: 1,
                //             ..Default::default()
                //         })
                //         .image_extent(vk::Extent3D {
                //             width: texture.extent.width / 2,
                //             height: texture.extent.height / 2,
                //             depth: 1,
                //         })],
                // );
            },
        );
        ctx.submit(&ctx.setup_command_buffer, ctx.setup_commands_reuse_fence);
    }

    pub fn copy_yuv420_10_frame_to_gpu(&self, ctx: &RenderContext) {
        let Some(yuv) = self.yuv else {
            return;
        };

        let Some(frame_buffer) = self.frame_buffer else {
            return;
        };

        ctx.record(
            ctx.setup_command_buffer,
            Some(ctx.setup_commands_reuse_fence),
            |ctx, command_buffer| unsafe {
                let texture = ctx.texture_manager.get_buffer(yuv);
                let frame_buffer = ctx.buffer_manager.get_buffer(frame_buffer);

                let layout_transition_barriers = vk::ImageMemoryBarrier::default()
                    .image(texture.image)
                    .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ)
                    .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(
                                vk::ImageAspectFlags::PLANE_0
                                    | vk::ImageAspectFlags::PLANE_1
                                    | vk::ImageAspectFlags::PLANE_2,
                            )
                            .layer_count(1)
                            .level_count(1),
                    );

                ctx.device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[layout_transition_barriers],
                );

                ctx.device.cmd_copy_buffer_to_image(
                    command_buffer,
                    frame_buffer.buffer,
                    texture.image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[vk::BufferImageCopy::default()
                        .buffer_offset(0)
                        .buffer_row_length(0)
                        .buffer_image_height(0)
                        .image_subresource(vk::ImageSubresourceLayers {
                            aspect_mask: vk::ImageAspectFlags::PLANE_0,
                            layer_count: 1,
                            ..Default::default()
                        })
                        .image_extent(texture.extent)],
                );

                let y_plane_size = texture.extent.width * texture.extent.height;

                ctx.device.cmd_copy_buffer_to_image(
                    command_buffer,
                    frame_buffer.buffer,
                    texture.image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[vk::BufferImageCopy::default()
                        .buffer_offset(y_plane_size as DeviceSize)
                        .buffer_row_length(0)
                        .buffer_image_height(0)
                        .image_subresource(vk::ImageSubresourceLayers {
                            aspect_mask: vk::ImageAspectFlags::PLANE_1,
                            layer_count: 1,
                            ..Default::default()
                        })
                        .image_extent(vk::Extent3D {
                            width: texture.extent.width / 2,
                            height: texture.extent.height / 2,
                            depth: 1,
                        })],
                );

                ctx.device.cmd_copy_buffer_to_image(
                    command_buffer,
                    frame_buffer.buffer,
                    texture.image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[vk::BufferImageCopy::default()
                        .buffer_offset(y_plane_size as DeviceSize * 2)
                        .buffer_row_length(0)
                        .buffer_image_height(0)
                        .image_subresource(vk::ImageSubresourceLayers {
                            aspect_mask: vk::ImageAspectFlags::PLANE_2,
                            layer_count: 1,
                            ..Default::default()
                        })
                        .image_extent(vk::Extent3D {
                            width: texture.extent.width / 2,
                            height: texture.extent.height / 2,
                            depth: 1,
                        })],
                );
            },
        );
        ctx.submit(&ctx.setup_command_buffer, ctx.setup_commands_reuse_fence);
    }

    pub fn draw(&self, ctx: &mut RenderContext, current_video: &CurrentVideo, frame: &[u8]) {
        let buffer = ctx
            .buffer_manager
            .get_buffer_mut(self.frame_buffer.unwrap());
        buffer.copy_from_slice(frame, 0);
        log::debug!("Copying frame to gpu");
        self.copy_yuv420_frame_to_gpu(ctx);

        ctx.record(
            ctx.draw_command_buffer,
            Some(ctx.draw_commands_reuse_fence),
            |ctx, command_buffer| unsafe {
                let color_attachments = &[vk::RenderingAttachmentInfo::default()
                    .image_view(
                        ctx.texture_manager
                            .get_buffer(self.attachment)
                            .view
                            .unwrap(),
                    )
                    .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .clear_value(vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 1.0],
                        },
                    })];

                ctx.begin_rendering(command_buffer, color_attachments, None);

                let pipeline = ctx
                    .pipeline_manager
                    .get_graphics_pipeline(self.pipeline_handle.as_ref().unwrap());
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
            },
        );

        ctx.submit(&ctx.draw_command_buffer, ctx.draw_commands_reuse_fence);
    }
}
