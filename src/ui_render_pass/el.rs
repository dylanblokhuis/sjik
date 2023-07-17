use dyn_clone::DynClone;
use std::collections::BTreeMap;
use taffy::prelude::*;

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
}

impl UiRenderContext {
    pub fn new(viewport: (u32, u32)) -> Self {
        let fill_tess = FillTessellator::new();
        let geometry: VertexBuffers<UiVertex, u16> = VertexBuffers::new();

        Self {
            geometry,
            fill_tess,
            viewport,
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
        builder.close();
        builder.build().unwrap();
    }

    pub fn finish(self) -> VertexBuffers<UiVertex, u16> {
        self.geometry
    }
}

trait Node: DynClone {
    fn render(&self, render_context: &mut UiRenderContext) -> Tailwind;
}
dyn_clone::clone_trait_object!(Node);

// div

#[derive(Clone)]
struct Div {
    classes: String,
}

impl Node for Div {
    fn render(&self, _render_context: &mut UiRenderContext) -> Tailwind {
        Tailwind::new(&self.classes)
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, PartialOrd, Ord)]
struct NodeId(usize);

#[derive(Clone)]
struct ContextNode {
    parent: Option<NodeId>,
    node: Box<dyn Node>,
}

#[derive(Default, Clone)]
pub struct UiContext {
    nodes: BTreeMap<NodeId, ContextNode>,
    parent_node: Option<NodeId>,
}

impl UiContext {
    pub fn div(
        &mut self,
        classes: &str,
        _props: Props,
        children: impl FnOnce(UiContext) -> UiContext,
    ) -> UiContext {
        let node_id = self.insert(Box::new(Div {
            classes: classes.to_string(),
        }));
        let ctx = children(UiContext {
            parent_node: Some(node_id),
            ..Default::default()
        });
        self.nodes.extend(ctx.nodes);
        self.clone()
    }

    fn insert(&mut self, node: Box<dyn Node>) -> NodeId {
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

    pub fn finish(self, mut render_context: UiRenderContext) -> VertexBuffers<UiVertex, u16> {
        let mut taffy = Taffy::new();
        let mut couples = Vec::<(taffy::node::Node, NodeId, Tailwind)>::new();
        for (node_id, ctx) in self.nodes.iter() {
            let tw = ctx.node.render(&mut render_context);
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
            b.1 .0
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

        for (taffy_node, _, tw) in couples {
            let layout = taffy.layout(taffy_node).unwrap();

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

pub struct Props;
