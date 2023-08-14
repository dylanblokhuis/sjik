use beuk::ash::vk::PresentModeKHR;
use beuk::ctx::RenderContextDescriptor;
use beuk::raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use decoder::{DecodedFrame, MediaCommands, MediaDecoder, MediaDecoderOptions};
use dioxus_beuk::{DioxusApp, Redraw};
use media_render_pass::MediaRenderPass;
use present_render_pass::PresentRenderPass;
use tao::dpi::PhysicalSize;
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::{event::WindowEvent, window::WindowBuilder};

use std::sync::{Arc, RwLock};

mod decoder;
mod media_render_pass;
mod present_render_pass;
mod ui;

#[derive(Clone)]
pub struct CurrentVideo {
    pub width: u32,
    pub height: u32,
    // pub format: vk::Format,
}

#[derive(Clone)]
pub struct AppContext {
    window_size: PhysicalSize<u32>,
    command_sender: Option<crossbeam_channel::Sender<MediaCommands>>,
}

pub type AppContextRef = Arc<RwLock<AppContext>>;

fn main() {
    #[cfg(feature = "tracing")]
    {
        use tracing_subscriber::layer::SubscriberExt;
        tracing::subscriber::set_global_default(
            tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
        )
        .expect("set up the subscriber");
    }

    #[cfg(feature = "hot-reload")]
    dioxus_beuk::hot_reload::init(
        dioxus_beuk::hot_reload::Config::new().root(env!("CARGO_MANIFEST_DIR")),
    );

    std::env::set_var("RUST_LOG", "info");
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let args: Vec<String> = std::env::args().collect();
    let event_loop = EventLoopBuilder::<Redraw>::with_user_event().build();

    let window = WindowBuilder::new()
        .with_title("Sjik")
        .with_inner_size(tao::dpi::LogicalSize::new(1028.0, 768.0))
        .build(&event_loop)
        .unwrap();

    let ctx = Arc::new(beuk::ctx::RenderContext::new(RenderContextDescriptor {
        display_handle: window.raw_display_handle(),
        window_handle: window.raw_window_handle(),
        present_mode: PresentModeKHR::default(),
    }));

    let app_context = Arc::new(RwLock::new(AppContext {
        command_sender: None,
        window_size: window.inner_size(),
    }));

    let current_video: Arc<RwLock<Option<CurrentVideo>>> = Arc::new(RwLock::new(None));
    let (decoder_tx, decoder_rx) = crossbeam_channel::bounded::<DecodedFrame>(1);

    std::thread::spawn({
        let current_video = current_video.clone();
        let app_context = app_context.clone();
        move || {
            let Some(arg) = args.get(1) else {
                log::info!("Please provide an url");
                return;
            };
            let mut media_decoder = MediaDecoder::new(
                arg,
                MediaDecoderOptions { use_hw_accel: true },
                move |frame| {
                    decoder_tx.send(frame).unwrap();
                },
            );

            let (width, height) = media_decoder.get_video_size();
            *current_video.write().unwrap() = Some(CurrentVideo { width, height });
            app_context.write().unwrap().command_sender =
                Some(media_decoder.command_sender.clone());
            media_decoder.start();
        }
    });

    let mut present_node = PresentRenderPass::new(&ctx);
    let mut media_node = MediaRenderPass::new(&ctx);
    let media_attachment_handle = media_node.attachment.clone();

    let mut application = DioxusApp::new(
        ui::app,
        &ctx,
        event_loop.create_proxy(),
        app_context.clone(),
    );
    let ui_attachment_handle = application.get_attachment_handle().clone();

    ctx.command_thread_pool.spawn({
        let ctx = ctx.clone();
        let event_loop_proxy = event_loop.create_proxy();
        let current_video = current_video.clone();
        move || {
            while let Ok(frame) = decoder_rx.recv() {
                if let Some(current_video) = current_video.read().unwrap().as_ref() {
                    media_node.setup_buffers(&ctx, current_video, &frame);
                    media_node.draw(&ctx, &frame);
                    event_loop_proxy.send_event(Redraw(false)).unwrap();
                }
            }
        }
    });

    // ui thread
    let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event<'static, Redraw>>();
    ctx.command_thread_pool.spawn({
        let ctx = ctx.clone();
        let event_loop_proxy = event_loop.create_proxy();
        let app_context = app_context.clone();

        move || {
            application.render(&ctx);
            while let Ok(event) = event_rx.recv() {
                application.send_event(&event);
                match event {
                    tao::event::Event::WindowEvent {
                        event: ref w_event, ..
                    } => {
                        if let tao::event::WindowEvent::Resized(physical_size) = &w_event {
                            app_context.write().unwrap().window_size = *physical_size;
                            application.set_size(*physical_size);
                        } else if let tao::event::WindowEvent::ScaleFactorChanged {
                            new_inner_size,
                            ..
                        } = &w_event
                        {
                            app_context.write().unwrap().window_size = **new_inner_size;
                            application.set_size(**new_inner_size);
                        }

                        event_loop_proxy.send_event(Redraw(true)).unwrap();
                    }

                    Event::UserEvent(redraw) => {
                        if redraw.0 {
                            application.render(&ctx);
                        }
                    }

                    _ => (),
                }
            }
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = tao::event_loop::ControlFlow::Wait;

        let Some(st_event) = event.to_static() else {
            return;
        };

        let mut redraw = || {
            let present_index = ctx.acquire_present_index();
            present_node.combine_and_draw(
                &ctx,
                &ui_attachment_handle,
                &media_attachment_handle,
                present_index,
            );
            ctx.present_submit(present_index);
        };

        match st_event.clone() {
            tao::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            tao::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            tao::event::Event::WindowEvent {
                event: WindowEvent::CloseRequested | WindowEvent::Destroyed,
                ..
            } => *control_flow = ControlFlow::Exit,

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    ctx.recreate_swapchain(new_inner_size.width, new_inner_size.height);
                    window.request_redraw();
                }
                WindowEvent::Resized(size) => {
                    ctx.recreate_swapchain(size.width, size.height);
                    window.request_redraw();
                }
                _ => (),
            },

            tao::event::Event::NewEvents(tao::event::StartCause::ResumeTimeReached { .. }) => {
                window.request_redraw();
            }

            tao::event::Event::UserEvent(redraw_ev) => {
                if redraw_ev.0 {
                    window.request_redraw();
                } else {
                    redraw();
                }
            }
            _ => (),
        }

        event_tx.try_send(st_event).unwrap();
    });
}
