use std::collections::HashMap;
use std::mem::size_of;

use beuk::ash::vk::{
    self, ImageCreateInfo, PipelineVertexInputStateCreateInfo, PushConstantRange, ShaderStageFlags,
};
use beuk::buffer::BufferDescriptor;
use beuk::ctx::SamplerDesc;
use beuk::memory::MemoryLocation;
use beuk::memory2::ResourceHandle;
use beuk::pipeline::{BlendState, MultisampleState};
use beuk::texture::Texture;
use beuk::{ctx::RenderContext, memory::PipelineHandle};
use beuk::{
    pipeline::{GraphicsPipelineDescriptor, PrimitiveState},
    shaders::Shader,
};

use epaint::textures::TextureOptions;
use epaint::{Primitive, TessellationOptions, TextureId};

use crate::application::RendererState;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PushConstants {
    pub screen_size: [f32; 2],
}

pub struct Renderer {
    pub pipeline_handle: PipelineHandle,
    // pub vertex_buffer: Option<BufferHandle>,
    // pub index_buffer: Option<BufferHandle>,
    pub attachment_handle: ResourceHandle<Texture>,
    pub multisampled_handle: Option<ResourceHandle<Texture>>,
    pub shapes: Vec<epaint::ClippedShape>,
    pub state: RendererState,
    pub textures: HashMap<epaint::TextureId, ResourceHandle<Texture>>,
}

impl Renderer {
    pub fn new(ctx: &RenderContext, state: RendererState) -> Self {
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

        let swapchain = ctx.get_swapchain();
        let image_format = swapchain.surface_format.format;
        let msaa = 1;
        let multisampled_handle = if msaa != 1 {
            Some(ctx.create_texture(
                "ui_resolve",
                &ImageCreateInfo {
                    image_type: vk::ImageType::TYPE_2D,
                    format: image_format,
                    extent: vk::Extent3D {
                        width: swapchain.surface_resolution.width,
                        height: swapchain.surface_resolution.height,
                        depth: 1,
                    },
                    samples: vk::SampleCountFlags::from_raw(msaa),
                    usage: vk::ImageUsageFlags::COLOR_ATTACHMENT
                        | vk::ImageUsageFlags::TRANSFER_SRC
                        | vk::ImageUsageFlags::SAMPLED,
                    mip_levels: 1,
                    array_layers: 1,
                    sharing_mode: vk::SharingMode::EXCLUSIVE,
                    ..Default::default()
                },
            ))
        } else {
            None
        };
        let attachment_handle = ctx.create_texture(
            "ui",
            &ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format: image_format,
                extent: vk::Extent3D {
                    width: swapchain.surface_resolution.width,
                    height: swapchain.surface_resolution.height,
                    depth: 1,
                },
                samples: vk::SampleCountFlags::TYPE_1,
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::TRANSFER_SRC
                    | vk::ImageUsageFlags::SAMPLED,
                mip_levels: 1,
                array_layers: 1,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            },
        );

        let pipeline_handle = ctx.create_graphics_pipeline(&GraphicsPipelineDescriptor {
            vertex_shader,
            fragment_shader,
            vertex_input: PipelineVertexInputStateCreateInfo::default()
                .vertex_attribute_descriptions(&[
                    vk::VertexInputAttributeDescription {
                        location: 0,
                        binding: 0,
                        format: vk::Format::R32G32_SFLOAT,
                        offset: bytemuck::offset_of!(epaint::Vertex, pos) as u32,
                    },
                    vk::VertexInputAttributeDescription {
                        location: 1,
                        binding: 0,
                        format: vk::Format::R32G32_SFLOAT,
                        offset: bytemuck::offset_of!(epaint::Vertex, uv) as u32,
                    },
                    vk::VertexInputAttributeDescription {
                        location: 2,
                        binding: 0,
                        format: vk::Format::R8G8B8A8_UNORM,
                        offset: bytemuck::offset_of!(epaint::Vertex, color) as u32,
                    },
                ])
                .vertex_binding_descriptions(&[vk::VertexInputBindingDescription {
                    binding: 0,
                    stride: std::mem::size_of::<epaint::Vertex>() as u32,
                    input_rate: vk::VertexInputRate::VERTEX,
                }]),
            color_attachment_formats: &[image_format],
            depth_attachment_format: vk::Format::UNDEFINED,
            viewport: None,
            primitive: PrimitiveState {
                cull_mode: vk::CullModeFlags::NONE,
                topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                front_face: vk::FrontFace::COUNTER_CLOCKWISE,
                ..Default::default()
            },
            depth_stencil: Default::default(),
            push_constant_range: Some(
                PushConstantRange::default()
                    .stage_flags(ShaderStageFlags::ALL_GRAPHICS)
                    .offset(0)
                    .size(size_of::<PushConstants>() as u32),
            ),
            blend: vec![BlendState::ALPHA_BLENDING],
            multisample: MultisampleState {
                count: msaa,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        let textures: HashMap<TextureId, ResourceHandle<Texture>> = HashMap::new();

        Self {
            pipeline_handle,
            attachment_handle,
            multisampled_handle,
            shapes: vec![],
            state,
            textures,
        }
    }

    #[tracing::instrument(name = "Renderer::render", skip_all)]
    pub fn render(&mut self, ctx: &RenderContext) {
        let texture_delta = {
            let font_image_delta = self.state.fonts.read().unwrap().font_image_delta();
            if let Some(font_image_delta) = font_image_delta {
                self.state.tex_manager.write().unwrap().alloc(
                    "font".into(),
                    font_image_delta.image,
                    TextureOptions::LINEAR,
                );
            }

            self.state.tex_manager.write().unwrap().take_delta()
        };

        println!(
            "texture_delta: set {:?} free {:?}",
            texture_delta.set.len(),
            texture_delta.free.len()
        );

        for (id, delta) in texture_delta.set {
            let delta = delta.clone();
            let texture_handle = ctx.create_texture(
                "fonts",
                &ImageCreateInfo {
                    image_type: vk::ImageType::TYPE_2D,
                    format: vk::Format::R8G8B8A8_UNORM,
                    extent: vk::Extent3D {
                        width: delta.image.width() as u32,
                        height: delta.image.height() as u32,
                        depth: 1,
                    },
                    usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
                    mip_levels: 1,
                    array_layers: 1,
                    samples: vk::SampleCountFlags::TYPE_1,
                    sharing_mode: vk::SharingMode::EXCLUSIVE,
                    tiling: vk::ImageTiling::OPTIMAL,
                    ..Default::default()
                },
            );
            let data = match delta.image {
                epaint::ImageData::Color(image) => image.pixels,
                epaint::ImageData::Font(font) => font.srgba_pixels(None).collect(),
            };
            let buffer_handle = ctx.create_buffer_with_data(
                &BufferDescriptor {
                    debug_name: "fonts",
                    location: MemoryLocation::CpuToGpu,
                    size: data.len() as u64 * 4,
                    usage: vk::BufferUsageFlags::TRANSFER_SRC,
                },
                bytemuck::cast_slice(&data),
                0,
            );

            ctx.copy_buffer_to_texture(&buffer_handle, &texture_handle);
            let view = ctx.get_texture_view(&texture_handle).unwrap();
            unsafe {
                let manager = ctx.get_pipeline_manager();
                let pipeline = manager.get_graphics_pipeline(&self.pipeline_handle.id());
                ctx.device.update_descriptor_sets(
                    &[vk::WriteDescriptorSet::default()
                        .dst_set(pipeline.descriptor_sets[0])
                        .dst_binding(0)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(std::slice::from_ref(
                            &vk::DescriptorImageInfo::default()
                                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                                .image_view(*view)
                                .sampler(
                                    ctx.get_pipeline_manager()
                                        .immutable_shader_info
                                        .get_sampler(&SamplerDesc {
                                            address_modes: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                                            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
                                            texel_filter: vk::Filter::LINEAR,
                                        }),
                                ),
                        ))],
                    &[],
                );
            }
        }

        let (font_tex_size, prepared_discs) = {
            let fonts = self.state.fonts.read().unwrap();
            let atlas = fonts.texture_atlas();
            let atlas = atlas.lock();
            (atlas.size(), atlas.prepared_discs())
        };

        let primitives = epaint::tessellator::tessellate_shapes(
            self.state.fonts.read().unwrap().pixels_per_point(),
            TessellationOptions::default(),
            font_tex_size,
            prepared_discs,
            self.shapes.clone(),
        );

        let mut draw_list = Vec::with_capacity(primitives.len());
        for (index, primitive) in primitives.iter().enumerate() {
            match &primitive.primitive {
                Primitive::Mesh(mesh) => {
                    let vertex_buffer = ctx.create_buffer_with_data(
                        &BufferDescriptor {
                            debug_name: "vertices",
                            size: (mesh.vertices.len() * std::mem::size_of::<epaint::Vertex>())
                                as u64,
                            location: MemoryLocation::GpuOnly,
                            usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                        },
                        bytemuck::cast_slice(&mesh.vertices),
                        0,
                    );

                    let index_buffer = ctx.create_buffer_with_data(
                        &BufferDescriptor {
                            debug_name: "indices",
                            size: (mesh.indices.len() * std::mem::size_of::<u32>()) as u64,
                            location: MemoryLocation::GpuOnly,
                            usage: vk::BufferUsageFlags::INDEX_BUFFER,
                        },
                        bytemuck::cast_slice(&mesh.indices),
                        0,
                    );
                    draw_list.push((
                        vertex_buffer,
                        index_buffer,
                        mesh.indices.len() as u32,
                        mesh.texture_id,
                    ));
                }
                Primitive::Callback(_) => unreachable!(),
            }
        }

        ctx.record_submit(
            ctx.draw_command_buffer,
            ctx.draw_commands_reuse_fence,
            |ctx, command_buffer| unsafe {
                let color_attachments = &[vk::RenderingAttachmentInfo::default()
                    .image_view(
                        *ctx.get_texture_view(if self.multisampled_handle.is_some() {
                            self.multisampled_handle.as_ref().unwrap()
                        } else {
                            &self.attachment_handle
                        })
                        .unwrap(),
                    )
                    .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .clear_value(vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 0.0],
                        },
                    })];

                ctx.begin_rendering(command_buffer, color_attachments, None);

                let manager = ctx.get_pipeline_manager();
                let pipeline = manager.get_graphics_pipeline(&self.pipeline_handle.id());
                pipeline.bind(&ctx.device, command_buffer);
                let swapchain = ctx.get_swapchain();
                ctx.device.cmd_push_constants(
                    command_buffer,
                    pipeline.layout,
                    vk::ShaderStageFlags::ALL_GRAPHICS,
                    0,
                    bytemuck::bytes_of(&PushConstants {
                        screen_size: [
                            swapchain.surface_resolution.width as f32,
                            swapchain.surface_resolution.height as f32,
                        ],
                    }),
                );
                drop(swapchain);

                for (vertex_handle, index_handle, indices_len, texture_id) in draw_list.iter() {
                    ctx.device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        std::slice::from_ref(
                            &ctx.buffer_manager.get(vertex_handle.id()).unwrap().buffer,
                        ),
                        &[0],
                    );
                    ctx.device.cmd_bind_index_buffer(
                        command_buffer,
                        ctx.buffer_manager.get(index_handle.id()).unwrap().buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                    ctx.device
                        .cmd_draw_indexed(command_buffer, *indices_len, 1, 0, 0, 1);
                }

                ctx.end_rendering(command_buffer);
            },
        );

        if let Some(multisampled_handle) = self.multisampled_handle.as_ref() {
            ctx.record_submit(
                ctx.draw_command_buffer,
                ctx.draw_commands_reuse_fence,
                |ctx, command_buffer| unsafe {
                    let src_image = ctx
                        .texture_manager
                        .get(multisampled_handle.id())
                        .unwrap()
                        .image;
                    let dst_image = ctx
                        .texture_manager
                        .get(self.attachment_handle.id())
                        .unwrap()
                        .image;

                    let swapchain = ctx.get_swapchain();
                    ctx.device.cmd_resolve_image(
                        command_buffer,
                        src_image,
                        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                        dst_image,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &[vk::ImageResolve::default()
                            .src_subresource(vk::ImageSubresourceLayers {
                                aspect_mask: vk::ImageAspectFlags::COLOR,
                                mip_level: 0,
                                base_array_layer: 0,
                                layer_count: 1,
                            })
                            .dst_subresource(vk::ImageSubresourceLayers {
                                aspect_mask: vk::ImageAspectFlags::COLOR,
                                mip_level: 0,
                                base_array_layer: 0,
                                layer_count: 1,
                            })
                            .extent(vk::Extent3D {
                                width: swapchain.surface_resolution.width,
                                height: swapchain.surface_resolution.height,
                                depth: 1,
                            })],
                    );
                },
            );
        }
    }
}
