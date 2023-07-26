use std::collections::HashMap;
use std::mem::size_of;

use beuk::ash::vk::{
    self, ImageCreateInfo, PipelineVertexInputStateCreateInfo, PushConstantRange, ShaderStageFlags,
};
use beuk::ctx::SamplerDesc;
use beuk::memory::{MemoryLocation, TextureHandle};
use beuk::pipeline::{BlendState, MultisampleState};
use beuk::{ctx::RenderContext, memory::PipelineHandle};
use beuk::{
    pipeline::{GraphicsPipelineDescriptor, PrimitiveState},
    shaders::Shader,
};
use std::sync::{Arc, RwLock};

use epaint::textures::TextureOptions;
use epaint::{Fonts, Primitive, TessellationOptions, TextureId, TextureManager};

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
    pub attachment_handle: TextureHandle,
    pub multisampled_handle: Option<TextureHandle>,
    pub shapes: Vec<epaint::ClippedShape>,
    pub state: RendererState,
    pub textures: HashMap<epaint::TextureId, TextureHandle>,
}

impl Renderer {
    pub fn new(ctx: &mut RenderContext, state: RendererState) -> Self {
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

        let image_format = ctx.render_swapchain.surface_format.format;
        let msaa = 1;
        let multisampled_handle = if msaa != 1 {
            Some(ctx.texture_manager.create_texture(
                "ui_resolve",
                &ImageCreateInfo {
                    image_type: vk::ImageType::TYPE_2D,
                    format: image_format,
                    extent: vk::Extent3D {
                        width: ctx.render_swapchain.surface_resolution.width,
                        height: ctx.render_swapchain.surface_resolution.height,
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
        let attachment_handle = ctx.texture_manager.create_texture(
            "ui",
            &ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format: image_format,
                extent: vk::Extent3D {
                    width: ctx.render_swapchain.surface_resolution.width,
                    height: ctx.render_swapchain.surface_resolution.height,
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

        ctx.texture_manager
            .get_texture_mut(attachment_handle)
            .create_view(&ctx.device);

        if let Some(multisampled_handle) = multisampled_handle {
            ctx.texture_manager
                .get_texture_mut(multisampled_handle)
                .create_view(&ctx.device);
        }

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
                    viewport: ctx.render_swapchain.surface_resolution,
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

        let textures: HashMap<TextureId, TextureHandle> = HashMap::new();

        Self {
            pipeline_handle,
            attachment_handle,
            multisampled_handle,
            shapes: vec![],
            state,
            textures,
        }
    }

    pub fn render(&mut self, ctx: &mut RenderContext) {
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
            let handle = ctx.texture_manager.create_texture(
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
            let buffer_handle = ctx.buffer_manager.create_buffer_with_data(
                "fonts",
                bytemuck::cast_slice(&data),
                vk::BufferUsageFlags::TRANSFER_SRC,
                MemoryLocation::CpuToGpu,
            );
            let buffer = ctx.buffer_manager.get_buffer(buffer_handle);
            let texture = ctx.texture_manager.get_texture(handle);
            ctx.copy_buffer_to_texture(buffer, texture);
            ctx.buffer_manager.remove_buffer(buffer_handle);
            let texture = ctx.texture_manager.get_texture_mut(handle);
            let view = texture.create_view(&ctx.device);
            unsafe {
                let pipeline = ctx
                    .pipeline_manager
                    .get_graphics_pipeline(&self.pipeline_handle);
                ctx.device.update_descriptor_sets(
                    &[vk::WriteDescriptorSet::default()
                        .dst_set(pipeline.descriptor_sets[0])
                        .dst_binding(0)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(std::slice::from_ref(
                            &vk::DescriptorImageInfo::default()
                                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                                .image_view(view)
                                .sampler(ctx.pipeline_manager.immutable_shader_info.get_sampler(
                                    &SamplerDesc {
                                        address_modes: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                                        mipmap_mode: vk::SamplerMipmapMode::LINEAR,
                                        texel_filter: vk::Filter::LINEAR,
                                    },
                                )),
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
                    println!("mesh: {:?}", mesh.texture_id);
                    let vertex_buffer = ctx.buffer_manager.create_buffer_with_data(
                        &format!("vertices_{}", index),
                        bytemuck::cast_slice(&mesh.vertices),
                        vk::BufferUsageFlags::VERTEX_BUFFER,
                        MemoryLocation::CpuToGpu,
                    );

                    let index_buffer = ctx.buffer_manager.create_buffer_with_data(
                        &format!("indices-{}", index),
                        bytemuck::cast_slice(&mesh.indices),
                        vk::BufferUsageFlags::INDEX_BUFFER,
                        MemoryLocation::CpuToGpu,
                    );
                    println!("indices: {:?}", mesh.texture_id);
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

        ctx.record(
            ctx.draw_command_buffer,
            ctx.draw_commands_reuse_fence,
            |ctx, command_buffer| unsafe {
                let color_attachments = &[vk::RenderingAttachmentInfo::default()
                    .image_view(
                        ctx.texture_manager
                            .get_texture(if self.multisampled_handle.is_some() {
                                self.multisampled_handle.unwrap()
                            } else {
                                self.attachment_handle
                            })
                            .view
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

                let pipeline = ctx
                    .pipeline_manager
                    .get_graphics_pipeline(&self.pipeline_handle);
                pipeline.bind(&ctx.device, command_buffer);
                ctx.device.cmd_push_constants(
                    command_buffer,
                    pipeline.layout,
                    vk::ShaderStageFlags::ALL_GRAPHICS,
                    0,
                    bytemuck::bytes_of(&PushConstants {
                        screen_size: [
                            ctx.render_swapchain.surface_resolution.width as f32,
                            ctx.render_swapchain.surface_resolution.height as f32,
                        ],
                    }),
                );

                for (vertex_handle, index_handle, indices_len, texture_id) in draw_list.iter() {
                    ctx.device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        std::slice::from_ref(&ctx.buffer_manager.get_buffer(*vertex_handle).buffer),
                        &[0],
                    );
                    ctx.device.cmd_bind_index_buffer(
                        command_buffer,
                        ctx.buffer_manager.get_buffer(*index_handle).buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                    ctx.device
                        .cmd_draw_indexed(command_buffer, *indices_len, 1, 0, 0, 1);
                }

                ctx.end_rendering(command_buffer);
            },
        );
        ctx.submit(&ctx.draw_command_buffer, ctx.draw_commands_reuse_fence);

        for (vertex_handle, index_handle, _, _) in draw_list.iter() {
            ctx.buffer_manager.remove_buffer(*vertex_handle);
            ctx.buffer_manager.remove_buffer(*index_handle);
        }

        if let Some(multisampled_handle) = self.multisampled_handle {
            ctx.record(
                ctx.draw_command_buffer,
                ctx.draw_commands_reuse_fence,
                |ctx, command_buffer| unsafe {
                    let src_image = ctx.texture_manager.get_texture(multisampled_handle).image;
                    let dst_image = ctx
                        .texture_manager
                        .get_texture(self.attachment_handle)
                        .image;

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
                                width: ctx.render_swapchain.surface_resolution.width,
                                height: ctx.render_swapchain.surface_resolution.height,
                                depth: 1,
                            })],
                    );
                },
            );
            ctx.submit(&ctx.draw_command_buffer, ctx.draw_commands_reuse_fence);
        }
    }
}
