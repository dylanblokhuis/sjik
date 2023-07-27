use dioxus_native_core::prelude::*;
use epaint::emath::Align;
use epaint::{ClippedShape,  FontId};

use taffy::prelude::Layout;
use taffy::prelude::Size;
use taffy::Taffy;
use tao::dpi::PhysicalSize;

use crate::focus::Focused;
use crate::renderer::Renderer;
use crate::style::Tailwind;

use crate::RealDom;

const FOCUS_BORDER_WIDTH: f32 = 6.0;

pub(crate) fn render(
    dom: &RealDom,
    taffy: &Taffy,
    renderer: &mut Renderer,
    window_size: PhysicalSize<u32>,
) {
    let root = &dom.get(dom.root_id()).unwrap();

    render_node(
        taffy,
        *root,
        renderer,
        epaint::Pos2::ZERO,
        &Size {
            width: window_size.width,
            height: window_size.height,
        },
    );
}

fn render_node(
    taffy: &Taffy,
    node: NodeRef,
    renderer: &mut Renderer,
    location: epaint::Pos2,
    viewport_size: &Size<u32>,
) {
    let taffy_node = node.get::<Tailwind>().unwrap().node.unwrap();
    let layout = taffy.layout(taffy_node).unwrap();
    let location = location + epaint::Vec2::new(layout.location.x, layout.location.y);
    
    match &*node.node_type() {
        NodeType::Text(TextNode { text, .. }) => {
            let parent = node.parent().unwrap();
            let tailwind: &Tailwind = &parent.get().unwrap();

            let shape = epaint::Shape::text(
                &renderer.state.fonts.read().unwrap(),
                epaint::Pos2 {
                    x: location.x as f32,
                    y: location.y as f32,
                },
                epaint::emath::Align2([Align::TOP, Align::LEFT]),
                text,
                FontId::default(),
                tailwind.color,
            );
            let clip = shape.visual_bounding_rect();
            renderer.shapes.push(ClippedShape(clip, shape));
        }
        NodeType::Element(_) => {
            let shape = get_shape(layout, node, viewport_size, location);
            let clip = shape.visual_bounding_rect();
            renderer.shapes.push(ClippedShape(clip, shape));
            for child in node.children() {
                render_node(taffy, child, renderer, location, viewport_size);
            }
        }
        _ => {}
    }
}

pub(crate) fn get_shape(
    layout: &Layout,
    node: NodeRef,
    viewport_size: &Size<u32>,
    location: epaint::Pos2,
) -> epaint::Shape {
    let rect = layout.size;
    let x = location.x;
    let y = location.y;
    let width: f32  = layout.size.width.into();
    let height: f32 = layout.size.height.into();
    let tailwind: &Tailwind = &node.get().unwrap();
    let focused = node.get::<Focused>().filter(|focused| focused.0).is_some();
    let left_border_width = if focused {
        FOCUS_BORDER_WIDTH
    } else {
        tailwind
            .border
            .width
    };
    let right_border_width = if focused {
        FOCUS_BORDER_WIDTH
    } else {
        tailwind
            .border
            .width
    };
    let top_border_width = if focused {
        FOCUS_BORDER_WIDTH
    } else {
        tailwind
            .border
            .width
    };
    let bottom_border_width = if focused {
        FOCUS_BORDER_WIDTH
    } else {
        tailwind
            .border
            .width
    };

    // The stroke is drawn on the outside of the border, so we need to offset the rect by the border width for each side.
    let x_start = x + left_border_width / 2.0;
    let y_start = y + top_border_width / 2.0;
    let x_end: f32 = x + width - right_border_width / 2.0;
    let y_end: f32 = y + height - bottom_border_width / 2.0;


    epaint::Shape::Rect(epaint::RectShape {
        rect: epaint::Rect {
            min: epaint::Pos2 {
                x: x_start as f32,
                y: y_start as f32,
            },
            max: epaint::Pos2 {
                x: x_end as f32,
                y: y_end as f32,
            },
        },
        rounding: epaint::Rounding {
            nw: tailwind
                .border
                .radius
                .nw,
            ne: tailwind
                .border
                .radius
                .ne,
            se: tailwind
                .border
                .radius
                .se,
            sw: tailwind
                .border
                .radius
                .sw,
        },
        fill: tailwind.background_color,
        stroke: epaint::Stroke {
            width: tailwind.border.width,
            color: tailwind.border.color,
        },
    })
}

pub(crate) fn get_abs_pos(layout: Layout, taffy: &Taffy, node: NodeRef) -> epaint::Pos2 {
    let mut node_layout = layout.location;
    let mut current = node.id();
    while let Some(parent) = node.real_dom().get(current).unwrap().parent() {
        let parent_id = parent.id();
        // the root element is positioned at (0, 0)
        if parent_id == node.real_dom().root_id() {
            break;
        }
        current = parent_id;
        let taffy_node = parent.get::<Tailwind>().unwrap().node.unwrap();
        let parent_layout = taffy.layout(taffy_node).unwrap();
        node_layout.x += parent_layout.location.x;
        node_layout.y += parent_layout.location.y;
    }
    epaint::Pos2::new(node_layout.x, node_layout.y)
}
