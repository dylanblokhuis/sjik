use beuk::ash::vk::PresentModeKHR;
use beuk::ctx::RenderContextDescriptor;
use beuk::raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crossbeam_utils::atomic::AtomicCell;

use decoder::MediaDecoder;
use dioxus_beuk::{DioxusApp, Redraw};
use media_render_pass::MediaRenderPass;
use present_render_pass::PresentRenderPass;
use tao::event_loop::ControlFlow;
use tao::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

use std::sync::{Arc, RwLock};

mod decoder;
mod media_render_pass;
mod present_render_pass;
mod ui;

fn main() {
    std::env::set_var("RUST_LOG", "info");
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let event_loop = EventLoop::<Redraw>::with_user_event();

    let window = WindowBuilder::new()
        .with_title("Sjik")
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 720.0))
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
    let (tx, rx) = crossbeam_channel::unbounded::<Vec<u8>>();

    std::thread::spawn({
        let video_size = video_size.clone();

        move || {
            let mut media_decoder = MediaDecoder::new("http://192.168.178.49:32400/library/parts/1717/1689522231/file.mkv?download=1&X-Plex-Token=J3j74Py7w49SsXrq3ThS", move|frame| {
                tx.send(frame).unwrap();
            });
            let (width, height) = media_decoder.get_video_size();
            video_size.store(Some((width, height, 1)));
            media_decoder.start();
        }
    });

    let media_node =
        std::sync::Arc::new(RwLock::new(MediaRenderPass::new(&mut ctx.write().unwrap())));
    let mut present_node = PresentRenderPass::new(&mut ctx.write().unwrap());

    let mut application = DioxusApp::new(
        ui::app,
        &mut ctx.write().unwrap(),
        event_loop.create_proxy(),
    );

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

    event_loop.run(move |event, _, control_flow| {
        application.send_event(&event);

        match event {
            tao::event::Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            tao::event::Event::MainEventsCleared => {
                window.request_redraw();
            }
            tao::event::Event::RedrawRequested(_) => {
                let present_index = ctx.read().unwrap().acquire_present_index();
                media_node.read().unwrap().draw(&mut ctx.write().unwrap());
                if !application.clean().is_empty() {
                    println!("Redrawing ui");
                    application.render(&mut ctx.write().unwrap());
                }
                present_node.combine_and_draw(
                    &ctx.read().unwrap(),
                    application.get_attachment_handle(),
                    media_node.read().unwrap().attachment,
                    present_index,
                );
                ctx.write().unwrap().present_submit(present_index);
            }
            tao::event::Event::UserEvent(_redraw) => {
                window.request_redraw();
            }
            tao::event::Event::WindowEvent {
                event: WindowEvent::Resized(physical_size),
                window_id: _,
                ..
            } => {
                application.set_size(physical_size);
            }
            _ => (),
        }
    });
}
