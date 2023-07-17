use std::collections::BTreeMap;
use taffy::prelude::*;

use lyon::{
    geom::{point, Box2D},
    lyon_tessellation::{
        FillTessellator, FillVertex, FillVertexConstructor,
        VertexBuffers, FillOptions, BuffersBuilder,
    },
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

    pub fn finish(self) -> VertexBuffers<UiVertex, u16> {
        self.geometry
    }
}


trait Node {
    fn render(&self, render_context: &mut UiRenderContext) -> Tailwind;
}

// div
struct Div {
    classes: String,
}

impl Node for Div {
    fn render(&self, _render_context: &mut UiRenderContext) -> Tailwind {
        
        // println!("div {}", self.classes);
        // let min_x = render_context.normalize_width(tw.width as f32 * -0.5);
        // let max_x = render_context.normalize_width(tw.width as f32 * 0.5);
        // let min_y = render_context.normalize_height(tw.height as f32 * -0.5);
        // let max_y = render_context.normalize_height(tw.height as f32 * 0.5);


        Tailwind::new(&self.classes)
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, PartialOrd, Ord)]
struct NodeId(usize);
impl NodeId {
    fn new() -> Self {
        static mut COUNTER: usize = 0;
        unsafe {
            COUNTER += 1;
            Self(COUNTER)
        }
    }
}

struct ContextNode {
    parent: Option<NodeId>,
    node: Box<dyn Node>,
}

#[derive(Default)]
pub struct UiContext {
    nodes: BTreeMap<NodeId, ContextNode>,
    parent_node: Option<NodeId>,
}

impl UiContext {
    pub fn div(&mut self, classes: &str, _props: Props, children: impl FnOnce(&mut UiContext)) {
        let node_id = self.insert(Box::new(Div {
            classes: classes.to_string(),
        }));
        self.parent_node = Some(node_id);
        children(self);
        self.parent_node = None;
    }

    fn insert(&mut self, node: Box<dyn Node>) -> NodeId {
        let id = NodeId::new();
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
        let mut couples = Vec::<(taffy::node::Node, (NodeId, Tailwind))>::new();
        for (node_id, ctx) in self.nodes {
            let tw = ctx.node.render(&mut render_context);
            let taffy_id = taffy.new_leaf(tw.layout_style.clone()).unwrap();

            let maybe_parent = ctx.parent.and_then(|parent_id| {
                couples
                    .iter()
                    .find(|(_, (id, _))| *id == parent_id)
                    .map(|(node, _)| node)
            });
            if let Some(parent) = maybe_parent {
                taffy.add_child(*parent, taffy_id).unwrap();
            }

            couples.push((taffy_id, (node_id, tw)));
        }

        let first_node = couples.first().unwrap().0;
        taffy.compute_layout(first_node, Size {
            width: AvailableSpace::Definite(render_context.viewport.0 as f32),
            height: AvailableSpace::Definite(render_context.viewport.1 as f32),
        }).unwrap();

        for (taffy_node, (_, tw)) in couples {
            let layout = taffy.layout(taffy_node).unwrap();            
            // the whole screen maps from 0.0..1.0
            println!("{} {} {} {}", layout.location.x, layout.location.y, layout.size.width, layout.size.height);

            let min_x = layout.location.x - layout.size.width * 0.5;
            let max_x = layout.location.x + layout.size.width * 0.5;
            let min_y = layout.location.y - layout.size.height * 0.5;
            let max_y = layout.location.y + layout.size.height * 0.5;

            // println!("{} {} {} {}", min_x, max_x, min_y, max_y);

            render_context.rect(Box2D::new(
                point(render_context.normalize_width(min_x), render_context.normalize_height(min_y)),
                point(render_context.normalize_width(max_x), render_context.normalize_height(max_y))
            ), tw.visual_style.background_color);            
        }

        render_context.finish()
    }
}

pub struct Props;
