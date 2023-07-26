use beuk::ash::vk::PresentModeKHR;
use beuk::ctx::RenderContextDescriptor;
use beuk::raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use decoder::MediaDecoder;
use dioxus_beuk::{DioxusApp, Redraw};
use media_render_pass::MediaRenderPass;
use present_render_pass::PresentRenderPass;
use tao::event_loop::ControlFlow;
use tao::{event::WindowEvent, event_loop::EventLoop, window::WindowBuilder};

use std::sync::{Arc, RwLock};

mod decoder;
mod media_render_pass;
mod present_render_pass;
mod ui;

#[derive(Clone)]
pub struct CurrentVideo {
    pub width: u32,
    pub height: u32,
}

fn main() {
    #[cfg(feature = "hot-reload")]
    dioxus_hot_reload::hot_reload_init!();

    std::env::set_var("RUST_LOG", "info");
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let event_loop = EventLoop::<Redraw>::with_user_event();

    let window = WindowBuilder::new()
        .with_title("Sjik")
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 720.0))
        .build(&event_loop)
        .unwrap();

    let mut ctx = beuk::ctx::RenderContext::new(RenderContextDescriptor {
        display_handle: window.raw_display_handle(),
        window_handle: window.raw_window_handle(),
        present_mode: PresentModeKHR::FIFO,
    });

    let current_video: Arc<RwLock<Option<CurrentVideo>>> = Arc::new(RwLock::new(None));
    let (tx, rx) = crossbeam_channel::bounded::<Vec<u8>>(1);

    std::thread::spawn({
        let current_video = current_video.clone();

        move || {
            // let mut media_decoder = MediaDecoder::new("http://192.168.178.49:32400/library/parts/1739/1690127603/file.mkv?download=1&X-Plex-Token=J3j74Py7w49SsXrq3ThS", move|frame| {
            //     tx.send(frame).unwrap();
            // });
            // let (width, height) = media_decoder.get_video_size();
            // *current_video.write().unwrap() = Some(CurrentVideo { width, height });
            // media_decoder.start();
        }
    });

    let mut media_node = MediaRenderPass::new(&mut ctx);
    let mut present_node = PresentRenderPass::new(&mut ctx);

    let mut application = DioxusApp::new(ui::app, &mut ctx, event_loop.create_proxy());

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
                if rx.is_full() {
                    if let Some(current_video) = current_video.read().unwrap().as_ref() {
                        let frame = rx.recv().unwrap();
                        media_node.setup_buffers(&mut ctx, current_video);
                        media_node.draw(&mut ctx, current_video, &frame);
                    }
                }

                if !application.clean().is_empty() {
                    application.render(&mut ctx);
                }

                let present_index = ctx.acquire_present_index();
                present_node.combine_and_draw(
                    &ctx,
                    application.get_attachment_handle(),
                    media_node.attachment,
                    present_index,
                );
                ctx.present_submit(present_index);
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
