use dioxus_native_core::prelude::*;
use tao::event::Event;

pub use crate::events::EventData;

mod application;
pub use application::DioxusApp;
mod events;
mod focus;
mod mouse;
mod prevent_default;
mod render;
mod renderer;
mod style;

#[derive(Debug)]
pub struct Redraw;

type TaoEvent<'a> = Event<'a, Redraw>;
