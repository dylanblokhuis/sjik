use beuk::{
    ash::vk::PresentModeKHR,
    ctx::{RenderContext, RenderContextDescriptor},
};
use dioxus::prelude::*;
use dioxus_beuk::DioxusApp;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use tao::{
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::with_user_event();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut render_context = RenderContext::new(RenderContextDescriptor {
        display_handle: window.raw_display_handle(),
        window_handle: window.raw_window_handle(),
        present_mode: PresentModeKHR::default(),
    });

    let mut application = DioxusApp::new(app, &mut render_context, event_loop.create_proxy());

    event_loop.run(move |event, _, control_flow| {
        // ControlFlow::Wait pauses the event loop if no events are available to process.
        // This is ideal for non-game applications that only update in response to user
        // input, and uses significantly less power/CPU time than ControlFlow::Poll.
        *control_flow = ControlFlow::Wait;

        application.send_event(&event);

        match event {
            tao::event::Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            tao::event::Event::MainEventsCleared => {
                // Application update code.

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw, in
                // applications which do not always need to. Applications that redraw continuously
                // can just render here instead.
                window.request_redraw();
            }
            tao::event::Event::RedrawRequested(_) => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in MainEventsCleared, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.
                let present_index = render_context.acquire_present_index();

                // if !application.clean().is_empty() {
                //     application.render(&mut render_context, present_index);
                // }

                render_context.present_submit(present_index);
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

fn app(cx: Scope) -> Element {
    let count = use_state(cx, || 100);

    cx.render(rsx! {
      div {
        width: "{count}",
        height: "100px",
        background: "red",
        onclick: move |_| {
          count.set(count.get() + 10);
        }
      }
      div {
        width: "100px",
        height: "100px",
        background: "blue",
      }
      div {
        width: "100px",
        height: "100px",
        background: "blue",
      }
      div {
        width: "100px",
        height: "100px",
        background: "green",
      }
    })
}
