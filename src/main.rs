use std::mem::size_of_val;

use beuk::ash::vk::{
    self, BufferUsageFlags, DeviceSize, ImageCreateInfo,
    PhysicalDeviceSamplerYcbcrConversionFeatures, PipelineVertexInputStateCreateInfo,
    SamplerYcbcrConversion, SamplerYcbcrConversionCreateInfo, SamplerYcbcrConversionInfo,
    SamplerYcbcrModelConversion, SamplerYcbcrRange,
};
use beuk::ctx::SamplerDesc;
use beuk::memory::{MemoryLocation, TextureHandle};
use beuk::raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use beuk::{
    ctx::RenderContext,
    memory::{BufferHandle, PipelineHandle},
    pipeline::{GraphicsPipelineDescriptor, PrimitiveState},
    shaders::Shader,
};
use decoder::MediaDecoder;
use winit::event::{KeyboardInput, VirtualKeyCode};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

mod decoder;

fn main() {
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
        .build(&event_loop)
        .unwrap();

    let ctx = std::sync::Arc::new(std::sync::RwLock::new(beuk::ctx::RenderContext::new(
        window.raw_display_handle(),
        window.raw_window_handle(),
        |dc| dc,
    )));

    let (tx, rx) = crossbeam_channel::unbounded::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut media_decoder = MediaDecoder::new("http://192.168.178.49:32400/library/parts/1694/1689270378/file.mkv?download=1&X-Plex-Token=J3j74Py7w49SsXrq3ThS", move|frame| {
            // canvas.copy_frame_to_gpu(&ctx, &frame);
            tx.send(frame).unwrap();
        });
        media_decoder.start();
    });

    let canvas = std::sync::Arc::new(Canvas::new(&mut ctx.write().unwrap()));
    let ctx_clone = ctx.clone();
    let canvas_clone = canvas.clone();

    std::thread::spawn(move || loop {
        let val = rx.recv().unwrap();
        {
            let mut binding = ctx_clone.write().unwrap();
            let buffer = binding
                .buffer_manager
                .get_buffer_mut(canvas_clone.frame_buffer);

            buffer.copy_from_slice(&val, 0);
        }
        canvas_clone.copy_frame_to_gpu(&ctx_clone.read().unwrap(), &val);
    });

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            window_id,
        } if window_id == window.id() => control_flow.set_exit(),
        Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    device_id,
                    input,
                    is_synthetic,
                },
            ..
        } => {
            let Some(keycode) = input.virtual_keycode else {
                return;
            };
            match keycode {
                VirtualKeyCode::Escape => control_flow.set_exit(),
                VirtualKeyCode::Space => {
                    log::info!("Space pressed");
                }
                _ => (),
            }
        }
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        Event::RedrawRequested(_) => {
            canvas.draw(&mut ctx.write().unwrap());
        }
        _ => (),
    });
}

#[repr(C, align(16))]
#[derive(Clone, Debug, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
}
struct Canvas {
    pipeline_handle: PipelineHandle,
    vertex_buffer: BufferHandle,
    index_buffer: BufferHandle,
    yuv: TextureHandle,
    frame_buffer: BufferHandle,
}

impl Canvas {
    fn new(ctx: &mut RenderContext) -> Self {
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
                    depth_attachment_format: ctx.render_swapchain.depth_image_format,
                    viewport: ctx.render_swapchain.surface_resolution,
                    primitive: PrimitiveState {
                        topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                        ..Default::default()
                    },
                    depth_stencil: Default::default(),
                    push_constant_range: None,
                });

        let (yuv, _) = ctx.texture_manager.create_texture(
            "yuv420",
            &ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format: vk::Format::G8_B8R8_2PLANE_420_UNORM,
                extent: vk::Extent3D {
                    width: 1920,
                    height: 1080,
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
            let texture = ctx.texture_manager.get_buffer_mut(yuv);
            let (sampler_conversion, sampler) = ctx
                .pipeline_manager
                .immutable_shader_info
                .get_yuv_conversion_sampler(
                    &ctx.device,
                    SamplerDesc {
                        texel_filter: vk::Filter::LINEAR,
                        mipmap_mode: vk::SamplerMipmapMode::LINEAR,
                        address_modes: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                    },
                    vk::Format::G8_B8R8_2PLANE_420_UNORM,
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
                    std::slice::from_ref(
                        &vk::WriteDescriptorSet::default()
                            .dst_set(pipeline.descriptor_sets[0])
                            .dst_binding(0)
                            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                            .image_info(std::slice::from_ref(
                                &vk::DescriptorImageInfo::default()
                                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                                    .image_view(view),
                            )),
                    ),
                    &[],
                );
            };
        }

        let fullscreen_of_yuv = 1920 * 1080 * 3 / 2;

        println!("fullscreen_yuv_size {:?}", fullscreen_of_yuv);

        let frame_buffer = ctx.buffer_manager.create_buffer(
            "frame",
            fullscreen_of_yuv as DeviceSize,
            vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::CpuToGpu,
        );

        Self {
            pipeline_handle,
            vertex_buffer,
            index_buffer,
            yuv,
            frame_buffer,
        }
    }

    fn copy_frame_to_gpu(&self, ctx: &RenderContext, frame: &[u8]) {
        println!("frame size: {:?}", size_of_val(frame));

        ctx.record(
            ctx.setup_command_buffer,
            Some(ctx.setup_commands_reuse_fence),
            |ctx, command_buffer| unsafe {
                let texture = ctx.texture_manager.get_buffer(self.yuv);
                let frame_buffer = ctx.buffer_manager.get_buffer(self.frame_buffer);

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

                let y_plane_size = texture.extent.width * texture.extent.height;
                let uv_plane_size = (texture.extent.width / 2) * (texture.extent.height / 2) * 2;

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
                            mip_level: 0,
                            base_array_layer: 0,
                            layer_count: 1,
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
                            mip_level: 0,
                            base_array_layer: 0,
                            layer_count: 1,
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

    pub fn draw(&self, ctx: &mut RenderContext) {
        let present_index = ctx.present_record(|ctx, command_buffer, present_index: u32| unsafe {
            let color_attachments = &[vk::RenderingAttachmentInfo::default()
                .image_view(ctx.render_swapchain.present_image_views[present_index as usize])
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.1, 0.1, 0.1, 1.0],
                    },
                })];

            let depth_attachment = &vk::RenderingAttachmentInfo::default()
                .image_view(ctx.render_swapchain.depth_image_view)
                .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                });

            ctx.begin_rendering(command_buffer, color_attachments, Some(depth_attachment));

            let pipeline = ctx
                .pipeline_manager
                .get_graphics_pipeline(&self.pipeline_handle);
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
        ctx.present_submit(present_index);
    }
}
