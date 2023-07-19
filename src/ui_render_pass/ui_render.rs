

use lyon::{
    lyon_tessellation::{
        FillVertex, FillVertexConstructor,
    },
};

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

// pub struct UiRenderContext {
//     geometry: VertexBuffers<UiVertex, u16>,
//     fill_tess: FillTessellator,
//     pub viewport: (u32, u32),
//     pub mouse_position: Option<PhysicalPosition<f64>>,
//     pub mouse_button: Option<(MouseButton, ElementState)>,
// }

// impl UiRenderContext {
//     pub fn new(
//         viewport: (u32, u32),
//         mouse_position: Option<PhysicalPosition<f64>>,
//         mouse_button: Option<(MouseButton, ElementState)>,
//     ) -> Self {
//         let fill_tess = FillTessellator::new();
//         let geometry: VertexBuffers<UiVertex, u16> = VertexBuffers::new();

//         Self {
//             geometry,
//             fill_tess,
//             viewport,
//             mouse_button,
//             mouse_position,
//         }
//     }
//     pub fn normalize_width(&self, pixel_size: f32) -> f32 {
//         pixel_size / (self.viewport.0 as f32 * 0.5)
//     }

//     pub fn normalize_height(&self, pixel_size: f32) -> f32 {
//         pixel_size / (self.viewport.1 as f32 * 0.5)
//     }

//     pub fn rect(&mut self, item: Box2D<f32>, color: [f32; 4]) {
//         self.fill_tess
//             .tessellate_rectangle(
//                 &item,
//                 &FillOptions::DEFAULT,
//                 &mut BuffersBuilder::new(&mut self.geometry, Custom { color }),
//             )
//             .unwrap();
//     }

//     pub fn rect_radius(&mut self, item: Box2D<f32>, radius: f32, color: [f32; 4]) {
//         let mut binding = BuffersBuilder::new(&mut self.geometry, Custom { color });
//         let options = FillOptions::default()
//             .with_sweep_orientation(lyon::lyon_tessellation::Orientation::Horizontal);
//         let mut builder = self.fill_tess.builder(&options, &mut binding);
//         builder.add_rounded_rectangle(
//             &item,
//             &BorderRadii {
//                 bottom_left: radius,
//                 bottom_right: radius,
//                 top_left: radius,
//                 top_right: radius,
//             },
//             lyon::path::Winding::Positive,
//         );
//         builder.build().unwrap();
//     }

//     pub fn finish(self) -> VertexBuffers<UiVertex, u16> {
//         self.geometry
//     }
// }
