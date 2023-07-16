use std::collections::BTreeMap;

use lyon::{
    geom::{point, Box2D},
    lyon_tessellation::{
        BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor,
        VertexBuffers,
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
    fn render(&self, render_context: &mut UiRenderContext);
}

// div
struct Div {
    classes: String,
}

impl Node for Div {
    fn render(&self, render_context: &mut UiRenderContext) {
        let tw = Tailwind::new(&self.classes);
        // println!("div {}", self.classes);
        let min_x = render_context.normalize_width(tw.width as f32 * -0.5);
        let max_x = render_context.normalize_width(tw.width as f32 * 0.5);
        let min_y = render_context.normalize_height(tw.height as f32 * -0.5);
        let max_y = render_context.normalize_height(tw.height as f32 * 0.5);

        render_context.rect(
            Box2D::new(point(min_x, min_y), point(max_x, max_y)),
            tw.background_color,
        );
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
    last_node: Option<NodeId>,
}

impl UiContext {
    pub fn div(&mut self, classes: &str, props: Props, children: impl FnOnce(&mut UiContext)) {
        self.insert(Box::new(Div {
            classes: classes.to_string(),
        }));
        children(self);
    }

    fn insert(&mut self, node: Box<dyn Node>) {
        let id = NodeId::new();
        self.nodes.insert(
            id,
            ContextNode {
                parent: self.last_node,
                node,
            },
        );
        self.last_node = Some(id);
    }

    pub fn finish(self, render_context: &mut UiRenderContext) {
        for (id, ctx) in self.nodes {
            // println!("{:?}", id);
            ctx.node.render(render_context);
        }
    }
}

pub struct Props;
