use std::collections::HashMap;
use std::mem::size_of;

use beuk::ash::vk::{
    self, ImageCreateInfo, PipelineVertexInputStateCreateInfo, PushConstantRange, ShaderStageFlags,
};
use beuk::ctx::SamplerDesc;
use beuk::memory::{MemoryLocation, TextureHandle};
use beuk::pipeline::BlendState;
use beuk::{ctx::RenderContext, memory::PipelineHandle};
use beuk::{
    pipeline::{GraphicsPipelineDescriptor, PrimitiveState},
    shaders::Shader,
};

use epaint::text::FontDefinitions;
use epaint::textures::{TextureOptions, TexturesDelta};
use epaint::{Primitive, TessellationOptions, TextureId, TextureManager};

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
    pub shapes: Vec<epaint::ClippedShape>,
    pub fonts: epaint::Fonts,
    pub textures: HashMap<epaint::TextureId, TextureHandle>,
    pub tex_manager: TextureManager,
}

impl Renderer {
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

        let image_format = ctx.render_swapchain.surface_format.format;
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
                    multisample: Default::default(),
                });

        let fonts = epaint::Fonts::new(1.0, 8 * 1024, FontDefinitions::default());

        // println!("{:?}", fonts.families().first().unwrap().);

        let mut textures = HashMap::new();
        // textures.insert(TextureId::default(), font_texture);
        let mut tex_manager = TextureManager::default();

        // tex_manager.set(TextureId::default(), )

        Self {
            pipeline_handle,
            attachment_handle,
            shapes: vec![],
            fonts,
            textures,
            tex_manager,
        }
    }

    pub fn update_descriptor_for_texture(&self, ctx: &RenderContext, texture_id: &TextureId) {
        let handle = self.textures.get(texture_id).unwrap();
        let texture = ctx.texture_manager.get_texture(*handle);
        let view = texture.view.unwrap();
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

    pub fn render(&mut self, ctx: &mut RenderContext) {
        println!("!!!!!!!!!!!!!!!!!!!!!!!!!! rendering");
        self.fonts
            .begin_frame(self.fonts.pixels_per_point(), self.fonts.max_texture_side());

        let texture_delta = {
            let font_image_delta = self.fonts.font_image_delta();
            println!("font_image_delta: {:?}", font_image_delta.is_some());
            if let Some(font_image_delta) = font_image_delta {
                self.tex_manager.alloc(
                    "font".into(),
                    font_image_delta.image,
                    TextureOptions::LINEAR,
                );
            }

            self.tex_manager.take_delta()
        };

        println!(
            "texture_delta: set {:?} free {:?}",
            texture_delta.set.len(),
            texture_delta.free.len()
        );
        let font_texture = {
            let (id, delta) = texture_delta.set.first().unwrap();
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

            handle
        };

        println!(
            "texture_delta: {:?} {:?}",
            texture_delta.set.len(),
            texture_delta.free.len()
        );

        let texture_atlas = self.fonts.texture_atlas();
        let (font_tex_size, prepared_discs) = {
            let atlas = texture_atlas.lock();
            (atlas.size(), atlas.prepared_discs())
        };

        let primitives = epaint::tessellator::tessellate_shapes(
            self.fonts.pixels_per_point(),
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
                            .get_texture(self.attachment_handle)
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
    }
}
