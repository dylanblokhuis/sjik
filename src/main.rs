use beuk::ash::vk::PresentModeKHR;
use beuk::ctx::RenderContextDescriptor;
use beuk::raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crossbeam_utils::atomic::AtomicCell;

use media_render_pass::MediaRenderPass;
use ui_render_pass::scratch::ENTITIES;
use ui_render_pass::UiRenderNode;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

use std::sync::{Arc, RwLock};

mod decoder;
mod media_render_pass;
mod ui;
mod ui_render_pass;

fn main() {
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Sjik")
        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
        .build(&event_loop)
        .unwrap();

    let ctx = std::sync::Arc::new(std::sync::RwLock::new(beuk::ctx::RenderContext::new(
        RenderContextDescriptor {
            display_handle: window.raw_display_handle(),
            window_handle: window.raw_window_handle(),
            present_mode: PresentModeKHR::FIFO,
        },
    )));

    let video_size: Arc<AtomicCell<Option<(u32, u32, u32)>>> = Arc::new(AtomicCell::new(None));
    let (_tx, rx) = crossbeam_channel::unbounded::<Vec<u8>>();

    std::thread::spawn({
        let _video_size = video_size.clone();

        move || {
            // let mut media_decoder = MediaDecoder::new("http://192.168.178.49:32400/library/parts/1717/1689522231/file.mkv?download=1&X-Plex-Token=J3j74Py7w49SsXrq3ThS", move|frame| {
            //     tx.send(frame).unwrap();
            // });
            // let (width, height) = media_decoder.get_video_size();
            // video_size.store(Some((width, height, 1)));
            // media_decoder.start();
        }
    });

    let media_node =
        std::sync::Arc::new(RwLock::new(MediaRenderPass::new(&mut ctx.write().unwrap())));

    let mut ui_node = UiRenderNode::new(&mut ctx.write().unwrap());

    std::thread::spawn({
        let video_size = video_size.clone();
        let ctx = ctx.clone();
        let media_node = media_node.clone();

        move || loop {
            let frame = rx.recv().unwrap();

            media_node
                .write()
                .unwrap()
                .setup_buffers(&mut ctx.write().unwrap(), video_size.load().unwrap());

            log::debug!("Copying frame to buffer: {}", frame.len());
            let media_node = media_node.read().unwrap();

            let mut ctx = ctx.write().unwrap();
            let buffer = ctx
                .buffer_manager
                .get_buffer_mut(media_node.frame_buffer.unwrap());
            buffer.copy_from_slice(&frame, 0);
            log::debug!("Copying frame to gpu");
            media_node.copy_yuv420_frame_to_gpu(&ctx);
            log::debug!("Done copying frame to gpu");
        }
    });

    leptos_reactive::create_scope(leptos_reactive::create_runtime(), move |cx| {
        leptos_reactive::SpecialNonReactiveZone::enter();
        let (button_width, set_button_width) = leptos_reactive::create_signal(cx, 40);

        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => control_flow.set_exit(),

            Event::WindowEvent { event, .. } => {
                *ENTITIES.write().unwrap() = Default::default();
                ui::app(cx, button_width, set_button_width);
                ui_node.write_geometry(&mut ctx.write().unwrap());

                ui_node.handle_events(event);
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let present_index = ctx.read().unwrap().acquire_present_index();
                media_node
                    .read()
                    .unwrap()
                    .draw(&mut ctx.write().unwrap(), present_index);
                ui_node.draw(&ctx.read().unwrap(), present_index);
                ctx.write().unwrap().present_submit(present_index);
            }
            _ => (),
        });
    })
    .dispose();
}
