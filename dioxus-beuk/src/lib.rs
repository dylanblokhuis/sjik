use dioxus_native_core::prelude::*;
use tao::event::Event;

pub use crate::events::EventData;

mod application;
pub use application::DioxusApp;
mod events;
mod focus;
mod image;
mod mouse;
mod paint;
mod prevent_default;
mod renderer;
mod style;

#[cfg(feature = "hot-reload")]
pub mod hot_reload {
    pub use dioxus_hot_reload::*;
}

#[derive(Debug, Clone)]
pub struct Redraw(pub bool);

type TaoEvent<'a> = Event<'a, Redraw>;
