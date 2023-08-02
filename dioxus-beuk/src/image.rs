use epaint::textures::TextureOptions;
use image::io::Reader as ImageReader;

use dioxus_native_core::prelude::*;
use dioxus_native_core_macro::partial_derive_state;

use epaint::ColorImage;
use shipyard::Component;

use crate::application::RendererState;

#[derive(Clone, PartialEq, Debug, Component, Default)]
pub(crate) struct ImageExtractor {
    pub path: String,
    pub texture_id: epaint::TextureId,
}

/**
 * Extracts the image from the path and stores it in the renderer's texture manager.
 */
#[partial_derive_state]
impl State for ImageExtractor {
    type ChildDependencies = ();
    type ParentDependencies = ();
    type NodeDependencies = ();

    const NODE_MASK: NodeMaskBuilder<'static> =
        NodeMaskBuilder::new().with_attrs(AttributeMaskBuilder::Some(&["src"]));

    fn update<'a>(
        &mut self,
        node_view: NodeView,
        _: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        _: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        context: &SendAnyMap,
    ) -> bool {
        println!("{:?}", self);
        let Some(src_attr) = node_view
            .attributes()
            .into_iter()
            .flatten()
            .find(|attr| attr.attribute.name == "src")
        else {
            return false;
        };

        if src_attr.value.to_string() == self.path {
            return false;
        }

        let state: &RendererState = context.get().unwrap();
        if self.texture_id != epaint::TextureId::default() {
            println!("Freeing texture: {:?}", self.texture_id);
            let mut manager = state.tex_manager.write().unwrap();
            manager.free(self.texture_id);
        }

        let mut path = std::path::PathBuf::new();
        path.push("assets");
        path.push(src_attr.value.to_string());

        let Ok(reader) = ImageReader::open(path.clone()) else {
            log::error!("Failed to open image: {}", path.display());
            return false;
        };
        let Ok(img) = reader.decode() else {
            log::error!("Failed to decode image: {}", path.display());
            return false;
        };
        let size = [img.width() as usize, img.height() as usize];
        let rgba = img.to_rgba8();

        let mut manager = state.tex_manager.write().unwrap();
        let id = manager.alloc(
            src_attr.value.to_string(),
            epaint::ImageData::Color(ColorImage::from_rgba_unmultiplied(size, &rgba)),
            TextureOptions::LINEAR,
        );
        self.texture_id = id;
        self.path = src_attr.value.to_string();

        true
    }

    fn create<'a>(
        node_view: NodeView<()>,
        node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        context: &SendAnyMap,
    ) -> Self {
        let mut myself = Self::default();
        myself.update(node_view, node, parent, children, context);
        myself
    }
}
