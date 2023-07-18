use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use taffy::prelude::*;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton},
};

use lyon::{
    geom::{point, Box2D},
    lyon_tessellation::{
        BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor,
        VertexBuffers,
    },
    path::builder::BorderRadii,
};

use super::tailwind::Tailwind;

#[repr(C, align(16))]
#[derive(Clone, Debug, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiVertex {
    pub point: [f32; 2],
    pub color: [f32; 4],
    pub _padding: [f32; 2],
}

pub struct Custom {
    pub color: [f32; 4],
}
impl FillVertexConstructor<UiVertex> for Custom {
    fn new_vertex(&mut self, vertex: FillVertex) -> UiVertex {
        UiVertex {
            point: vertex.position().to_array(),
            color: self.color,
            ..Default::default()
        }
    }
}

pub struct UiRenderContext {
    geometry: VertexBuffers<UiVertex, u16>,
    fill_tess: FillTessellator,
    viewport: (u32, u32),
    mouse_position: Option<PhysicalPosition<f64>>,
    mouse_button: Option<(MouseButton, ElementState)>,
}

impl UiRenderContext {
    pub fn new(
        viewport: (u32, u32),
        mouse_position: Option<PhysicalPosition<f64>>,
        mouse_button: Option<(MouseButton, ElementState)>,
    ) -> Self {
        let fill_tess = FillTessellator::new();
        let geometry: VertexBuffers<UiVertex, u16> = VertexBuffers::new();

        Self {
            geometry,
            fill_tess,
            viewport,
            mouse_button,
            mouse_position,
        }
    }
    pub fn normalize_width(&self, pixel_size: f32) -> f32 {
        pixel_size / (self.viewport.0 as f32 * 0.5)
    }

    pub fn normalize_height(&self, pixel_size: f32) -> f32 {
        pixel_size / (self.viewport.1 as f32 * 0.5)
    }

    pub fn rect(&mut self, item: Box2D<f32>, color: [f32; 4]) {
        self.fill_tess
            .tessellate_rectangle(
                &item,
                &FillOptions::DEFAULT,
                &mut BuffersBuilder::new(&mut self.geometry, Custom { color }),
            )
            .unwrap();
    }

    pub fn rect_radius(&mut self, item: Box2D<f32>, radius: f32, color: [f32; 4]) {
        let mut binding = BuffersBuilder::new(&mut self.geometry, Custom { color });
        let options = FillOptions::default()
            .with_sweep_orientation(lyon::lyon_tessellation::Orientation::Horizontal);
        let mut builder = self.fill_tess.builder(&options, &mut binding);
        builder.add_rounded_rectangle(
            &item,
            &BorderRadii {
                bottom_left: radius,
                bottom_right: radius,
                top_left: radius,
                top_right: radius,
            },
            lyon::path::Winding::Positive,
        );
        builder.build().unwrap();
    }

    pub fn finish(self) -> VertexBuffers<UiVertex, u16> {
        self.geometry
    }
}

trait Node {
    fn render(&self, render_context: &mut UiRenderContext) -> Tailwind;
    fn on_click(&mut self) {}
    fn on_hover(&mut self) {}
}

struct Div {
    classes: String,
    props: DivProps,
}

impl Node for Div {
    fn render(&self, _render_context: &mut UiRenderContext) -> Tailwind {
        Tailwind::new(&self.classes)
    }

    fn on_click(&mut self) {
        let Some(func) = self.props.on_click.as_mut() else {
            return;
        };
        func();
    }

    fn on_hover(&mut self) {
        let Some(func) = self.props.on_hover.as_mut() else {
            return;
        };
        func();
    }
}

#[derive(Default)]
pub struct DivProps {
    pub on_click: Option<Box<dyn FnMut()>>,
    pub on_hover: Option<Box<dyn FnMut()>>,
}

impl DivProps {
    pub fn on_click(mut self, func: impl FnMut() + 'static) -> Self {
        self.on_click = Some(Box::new(func));
        self
    }

    pub fn on_hover(mut self, func: impl FnMut() + 'static) -> Self {
        self.on_hover = Some(Box::new(func));
        self
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, PartialOrd, Ord)]
struct NodeId(usize);

#[derive(Clone)]
struct ContextNode {
    parent: Option<NodeId>,
    node: Rc<RefCell<dyn Node>>,
}

#[derive(Default)]
pub struct UiContext {
    nodes: BTreeMap<NodeId, ContextNode>,
    parent_node: Option<NodeId>,
}

impl UiContext {
    pub fn div(
        &mut self,
        classes: &str,
        props: DivProps,
        children: impl FnOnce(UiContext) -> UiContext,
    ) -> Self {
        let node_id = self.insert(Rc::new(RefCell::new(Div {
            classes: classes.to_string(),
            props,
        })));
        let ctx = children(Self {
            parent_node: Some(node_id),
            ..Default::default()
        });
        self.nodes.extend(ctx.nodes);
        Self {
            nodes: self.nodes.clone(),
            parent_node: self.parent_node,
        }
    }

    fn insert(&mut self, node: Rc<RefCell<dyn Node>>) -> NodeId {
        let mut id = NodeId(self.nodes.len() + 1);
        if let Some(parent_node) = self.parent_node {
            id.0 += parent_node.0;
        }

        self.nodes.insert(
            id,
            ContextNode {
                parent: self.parent_node,
                node,
            },
        );
        id
    }

    pub fn finish(mut self, mut render_context: UiRenderContext) -> VertexBuffers<UiVertex, u16> {
        let mut taffy = Taffy::new();
        let mut couples = Vec::<(taffy::node::Node, NodeId, Tailwind)>::new();
        for (node_id, ctx) in self.nodes.iter_mut() {
            let tw = ctx.node.borrow().render(&mut render_context);
            let taffy_id = taffy.new_leaf(tw.layout_style.clone()).unwrap();

            couples.push((taffy_id, *node_id, tw));
        }

        for (taffy_node, node_id, _) in couples.iter() {
            if let Some(parent_node) = self.nodes.get(node_id).unwrap().parent {
                let parent_taffy_node = couples
                    .iter()
                    .find(|(_, node_id, _)| node_id == &parent_node)
                    .unwrap()
                    .0;
                taffy.add_child(parent_taffy_node, *taffy_node).unwrap();
            }
        }

        couples.sort_by(|a, b| {
            a.1 .0
                .partial_cmp(&b.1 .0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let first_node = couples.first().unwrap().0;
        taffy
            .compute_layout(
                first_node,
                Size {
                    width: AvailableSpace::Definite(render_context.viewport.0 as f32 / 100.0),
                    height: AvailableSpace::Definite(render_context.viewport.1 as f32 / 100.0),
                },
            )
            .unwrap();

        for (taffy_node, node_id, tw) in couples {
            let layout = taffy.layout(taffy_node).unwrap();

            let min_x = layout.location.x;
            let max_x = min_x + layout.size.width;
            let min_y = layout.location.y;
            let max_y = min_y + layout.size.height;

            // check if click is inside this node
            if let Some(mouse_position) = render_context.mouse_position {
                if mouse_position.x >= min_x.into()
                    && mouse_position.x <= max_x.into()
                    && mouse_position.y >= min_y.into()
                    && mouse_position.y <= max_y.into()
                {
                    let node = self.nodes.get_mut(&node_id).unwrap();
                    let mut node = node.node.borrow_mut();
                    node.on_hover();
                    if let Some((mouse_button, state)) = render_context.mouse_button {
                        if mouse_button == MouseButton::Left && state == ElementState::Released {
                            node.on_click();
                        }
                    }
                }
            }

            // map everything to top left corner in vulkan coords (-1, -1)
            let min_x = 2.0 * (layout.location.x / render_context.viewport.0 as f32) - 1.0;
            let max_x = min_x + 2.0 * (layout.size.width / render_context.viewport.0 as f32);
            let min_y = 2.0 * (layout.location.y / render_context.viewport.1 as f32) - 1.0;
            let max_y = min_y + 2.0 * (layout.size.height / render_context.viewport.1 as f32);

            if tw.visual_style.border_radius != [0.0; 4] {
                render_context.rect_radius(
                    Box2D::new(point(min_x, min_y), point(max_x, max_y)),
                    tw.visual_style.border_radius[0],
                    tw.visual_style.background_color,
                );
            } else {
                render_context.rect(
                    Box2D::new(point(min_x, min_y), point(max_x, max_y)),
                    tw.visual_style.background_color,
                );
            }
        }

        render_context.finish()
    }
}
