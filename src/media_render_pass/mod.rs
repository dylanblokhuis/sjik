use beuk::ash::vk::{
    self, BufferUsageFlags, DeviceSize, ImageCreateInfo, PipelineVertexInputStateCreateInfo,
    SamplerYcbcrConversionInfo,
};
use beuk::buffer::{Buffer, BufferDescriptor, MemoryLocation};
use beuk::ctx::SamplerDesc;
use beuk::memory::ResourceHandle;
use beuk::pipeline::GraphicsPipeline;
use beuk::texture::{Texture, TransitionDesc};
use beuk::{
    ctx::RenderContext,
    pipeline::{GraphicsPipelineDescriptor, PrimitiveState},
    shaders::Shader,
};

use crate::decoder::DecodedFrame;
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
    pipeline_handle: Option<ResourceHandle<GraphicsPipeline>>,
    vertex_buffer: Option<ResourceHandle<Buffer>>,
    index_buffer: ResourceHandle<Buffer>,
    pub yuv: Option<ResourceHandle<Texture>>,
    pub frame_buffer: Option<ResourceHandle<Buffer>>,
    pub attachment: ResourceHandle<Texture>,
}

impl MediaRenderPass {
    pub fn new(ctx: &RenderContext) -> Self {
        let index_buffer = ctx.create_buffer_with_data(
            &BufferDescriptor {
                debug_name: "indices",
                size: std::mem::size_of::<[u32; 6]>() as DeviceSize,
                location: MemoryLocation::GpuOnly,
                usage: BufferUsageFlags::INDEX_BUFFER,
            },
            bytemuck::cast_slice(&[0u32, 1, 2, 3, 4, 5]),
            0,
        );

        let swapchain = ctx.get_swapchain();
        let attachment_format = swapchain.surface_format.format;
        let attachment_handle = ctx.create_texture(
            "media",
            &ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format: attachment_format,
                extent: vk::Extent3D {
                    width: swapchain.surface_resolution.width,
                    height: swapchain.surface_resolution.height,
                    depth: 1,
                },
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT
                    | vk::ImageUsageFlags::TRANSFER_SRC
                    | vk::ImageUsageFlags::SAMPLED
                    | vk::ImageUsageFlags::TRANSFER_DST,
                samples: vk::SampleCountFlags::TYPE_1,
                mip_levels: 1,
                array_layers: 1,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                tiling: vk::ImageTiling::OPTIMAL,
                ..Default::default()
            },
        );       

        // make the attachment black, on macOS undefined textures are red for some reason
        {
            let attachment = ctx.texture_manager.get(&attachment_handle).unwrap();
            println!("attachment: {:?}", attachment.format);
            let texture_bytes = (swapchain.surface_resolution.width * swapchain.surface_resolution.height) * attachment.bytes_per_texel();
            let black_screen = vec![0u8; texture_bytes as usize];
    
            let buffer_handle = ctx.create_buffer_with_data(
                &BufferDescriptor {
                    debug_name: "texture_black_screen",
                    usage: vk::BufferUsageFlags::TRANSFER_SRC,
                    location: MemoryLocation::CpuToGpu,
                    ..Default::default()
                },
                bytemuck::cast_slice(&black_screen),
                0,
            );
            ctx.copy_buffer_to_texture(&buffer_handle, &attachment_handle);
        }
        
        Self {
            pipeline_handle: None,
            vertex_buffer: None,
            index_buffer,
            yuv: None,
            frame_buffer: None,
            attachment: attachment_handle,
        }
    }

    pub fn setup_buffers(
        &mut self,
        ctx: &RenderContext,
        current_video: &CurrentVideo,
        frame: &DecodedFrame,
    ) {
        // means that we already have the buffers setup
        if self.vertex_buffer.is_some() {
            return;
        }

        // calculate uvs based on video size
        let screen_size = ctx.get_swapchain().surface_resolution;
        let screen_aspect = screen_size.width as f32 / screen_size.height as f32;
        let video_aspect = current_video.width as f32 / current_video.height as f32;

        let positions = if screen_aspect > video_aspect {
            // Pillarbox
            let scale_x = video_aspect / screen_aspect;
            [
                [-scale_x, -1.0],
                [scale_x, -1.0],
                [scale_x, 1.0],
                [-scale_x, -1.0],
                [scale_x, 1.0],
                [-scale_x, 1.0],
            ]
        } else {
            // Letterbox
            let scale_y = screen_aspect / video_aspect;
            [
                [-1.0, -scale_y],
                [1.0, -scale_y],
                [1.0, scale_y],
                [-1.0, -scale_y],
                [1.0, scale_y],
                [-1.0, scale_y],
            ]
        };

        let vertex_buffer = ctx.create_buffer_with_data(
            &BufferDescriptor {
                debug_name: "vertices",
                size: std::mem::size_of::<[Vertex; 6]>() as DeviceSize,
                usage: BufferUsageFlags::VERTEX_BUFFER,
                location: MemoryLocation::GpuOnly,
            },
            bytemuck::cast_slice(&[
                Vertex {
                    pos: positions[0],
                    uv: [0.0, 0.0],
                },
                Vertex {
                    pos: positions[1],
                    uv: [1.0, 0.0],
                },
                Vertex {
                    pos: positions[2],
                    uv: [1.0, 1.0],
                },
                Vertex {
                    pos: positions[3],
                    uv: [0.0, 0.0],
                },
                Vertex {
                    pos: positions[4],
                    uv: [1.0, 1.0],
                },
                Vertex {
                    pos: positions[5],
                    uv: [0.0, 1.0],
                },
            ]),
            0,
        );

        let is_10_bit = frame.linesizes.iter().sum::<i32>() > 15000;
        log::info!(
            "is_10_bit: {} ({})",
            is_10_bit,
            frame.linesizes.iter().sum::<i32>()
        );
        let video_format = match frame.linesizes.len() {
            // YUV420SP
            2 => {
                if is_10_bit {
                    vk::Format::G10X6_B10X6R10X6_2PLANE_420_UNORM_3PACK16
                } else {
                    vk::Format::G8_B8R8_2PLANE_420_UNORM
                }
            }
            // YUV420P
            3 => {
                if is_10_bit {
                    vk::Format::G10X6_B10X6_R10X6_3PLANE_420_UNORM_3PACK16
                } else {
                    vk::Format::G8_B8_R8_3PLANE_420_UNORM
                }
            }
            x => panic!("Unsupported format with {x} linesizes"),
        };
        log::info!("Using format {:?}", video_format);

        let pipeline_handle = ctx.create_graphics_pipeline(&GraphicsPipelineDescriptor {
            vertex_shader: Shader::from_source_text(
                &ctx.device,
                include_str!("./shader.vert"),
                "shader.vert",
                beuk::shaders::ShaderKind::Vertex,
                "main",
            ),
            fragment_shader: Shader::from_source_text(
                &ctx.device,
                &r#"
                #version 450
                    #extension GL_ARB_separate_shader_objects : enable
                    #extension GL_ARB_shading_language_420pack : enable

                    layout (set = 0, binding = 0) uniform sampler2D textureLinearYUV420P;

                    layout (location = 0) in vec2 o_uv;
                    layout (location = 0) out vec4 a_frag_color;

                    void main() {
                        a_frag_color = texture(textureLinearYUV420P, o_uv);
                    }"#
                .replace(
                    "textureLinearYUV420P",
                    match video_format {
                        vk::Format::G8_B8R8_2PLANE_420_UNORM => "textureLinearYUV420SP",
                        vk::Format::G8_B8_R8_3PLANE_420_UNORM => "textureLinearYUV420P",
                        vk::Format::G10X6_B10X6R10X6_2PLANE_420_UNORM_3PACK16 => {
                            "textureLinearYUV420SP10"
                        }
                        vk::Format::G10X6_B10X6_R10X6_3PLANE_420_UNORM_3PACK16 => {
                            "textureLinearYUV420P10"
                        }
                        _ => panic!("Unsupported format"),
                    },
                ),
                "shader.frag",
                beuk::shaders::ShaderKind::Fragment,
                "main",
            ),
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
            color_attachment_formats: &[ctx.texture_manager.get(&self.attachment).unwrap().format],
            depth_attachment_format: vk::Format::UNDEFINED,
            viewport: None,
            primitive: PrimitiveState {
                topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                ..Default::default()
            },
            depth_stencil: Default::default(),
            push_constant_range: None,
            blend: Default::default(),
            multisample: Default::default(),
        });

        let yuv = ctx.create_texture(
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

        {
            let texture = ctx.texture_manager.get(&yuv).unwrap();

            let (sampler_conversion, _) = ctx
                .yuv_immutable_samplers
                .get(&(
                    video_format,
                    SamplerDesc {
                        texel_filter: vk::Filter::LINEAR,
                        mipmap_mode: vk::SamplerMipmapMode::LINEAR,
                        address_modes: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                    },
                ))
                .unwrap();

            let view = unsafe {
                let mut conversion_info =
                    SamplerYcbcrConversionInfo::default().conversion(*sampler_conversion);

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
            let pipeline = ctx.graphics_pipelines.get(&pipeline_handle).unwrap();
            unsafe {
                ctx.device.update_descriptor_sets(
                    &[vk::WriteDescriptorSet::default()
                        .dst_set(pipeline.descriptor_sets[0])
                        .dst_binding(0)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(std::slice::from_ref(
                            &vk::DescriptorImageInfo::default()
                                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                                .image_view(view),
                        ))],
                    &[],
                );
            };
        }

        log::debug!("Creating frame buffer of size {}", frame.data.len());

        let frame_buffer = ctx.create_buffer(&BufferDescriptor {
            debug_name: "frame",
            size: frame.data.len() as DeviceSize,
            location: MemoryLocation::CpuToGpu,
            usage: BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::TRANSFER_DST,
        });

        self.vertex_buffer = Some(vertex_buffer);
        self.yuv = Some(yuv);
        self.frame_buffer = Some(frame_buffer);
        self.pipeline_handle = Some(pipeline_handle);
    }

    pub unsafe fn copy_yuv420_frame_to_gpu(
        &self,
        command_buffer: vk::CommandBuffer,
        ctx: &RenderContext,
        frame: &DecodedFrame,
    ) {
        let Some(yuv) = self.yuv.as_ref() else {
            return;
        };

        let Some(frame_buffer) = self.frame_buffer.as_ref() else {
            return;
        };

        let buffer = ctx
            .buffer_manager
            .get_mut(self.frame_buffer.as_ref().unwrap())
            .unwrap();
        buffer.copy_from_slice(&frame.data, 0);

        let texture = ctx.texture_manager.get_mut(yuv).unwrap();
        let frame_buffer = ctx.buffer_manager.get(frame_buffer).unwrap();

        texture.transition(
            &ctx.device,
            command_buffer,
            &TransitionDesc {
                new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                new_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                new_stage_mask: vk::PipelineStageFlags::TRANSFER,
            },
        );

        let y_plane_size = frame.linesizes[0] as usize * texture.extent.height as usize;
        let u_plane_size = frame.linesizes[1] as usize * texture.extent.height as usize / 2;

        match texture.format {
            vk::Format::G8_B8R8_2PLANE_420_UNORM
            | vk::Format::G10X6_B10X6R10X6_2PLANE_420_UNORM_3PACK16 => {
                log::debug!("copying PLANE_0 with offset {:?}", 0);
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

                log::debug!("copying PLANE_1 with offset {:?}", y_plane_size);
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
            }
            vk::Format::G8_B8_R8_3PLANE_420_UNORM
            | vk::Format::G10X6_B10X6_R10X6_3PLANE_420_UNORM_3PACK16 => {
                log::debug!("copying PLANE_0 with offset {:?}", 0);
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

                log::debug!("copying PLANE_1 with offset {:?}", y_plane_size);
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

                log::debug!(
                    "copying PLANE_2 with offset {:?}",
                    y_plane_size + u_plane_size
                );
                ctx.device.cmd_copy_buffer_to_image(
                    command_buffer,
                    frame_buffer.buffer,
                    texture.image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[vk::BufferImageCopy::default()
                        .buffer_offset((y_plane_size + u_plane_size) as DeviceSize)
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
            }
            _ => panic!("Unsupported format"),
        }

        texture.transition(
            &ctx.device,
            command_buffer,
            &TransitionDesc {
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                new_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                new_stage_mask: vk::PipelineStageFlags::TRANSFER,
            },
        );
    }

    #[tracing::instrument(name = "MediaRenderPass::draw", skip_all)]
    pub fn draw(&self, ctx: &RenderContext, frame: &DecodedFrame) {
        log::debug!("Copying frame to gpu");

        ctx.record_submit(|ctx, command_buffer| unsafe {
            self.copy_yuv420_frame_to_gpu(command_buffer, ctx, frame);
            let color_attachments = &[vk::RenderingAttachmentInfo::default()
                .image_view(*ctx.get_texture_view(&self.attachment).unwrap())
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                })];

            ctx.begin_rendering(command_buffer, color_attachments, None);

            ctx.graphics_pipelines
                .get(self.pipeline_handle.as_ref().unwrap())
                .unwrap()
                .bind(&ctx.device, command_buffer);
            ctx.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                std::slice::from_ref(
                    &ctx.buffer_manager
                        .get(self.vertex_buffer.as_ref().unwrap())
                        .unwrap()
                        .buffer,
                ),
                &[0],
            );
            ctx.device.cmd_bind_index_buffer(
                command_buffer,
                ctx.buffer_manager.get(&self.index_buffer).unwrap().buffer,
                0,
                vk::IndexType::UINT32,
            );
            ctx.device.cmd_draw_indexed(command_buffer, 6, 1, 0, 0, 1);
            ctx.end_rendering(command_buffer);
        });
        log::debug!("Frame copied to gpu");
    }
}
