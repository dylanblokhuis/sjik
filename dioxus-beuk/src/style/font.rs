use dioxus_native_core::prelude::*;
use dioxus_native_core_macro::partial_derive_state;
use epaint::{FontFamily, FontId};
use shipyard::Component;

#[derive(Clone, PartialEq, Debug, Component)]
pub(crate) struct FontProperties(pub FontId);
const DEFAULT_FONT_SIZE: f32 = 16.0;

impl Default for FontProperties {
    fn default() -> Self {
        FontProperties(FontId {
            size: DEFAULT_FONT_SIZE,
            family: FontFamily::Proportional,
        })
    }
}

#[partial_derive_state]
impl State for FontProperties {
    type ChildDependencies = ();
    type ParentDependencies = (Self,);
    type NodeDependencies = ();

    const NODE_MASK: NodeMaskBuilder<'static> =
        NodeMaskBuilder::new().with_attrs(AttributeMaskBuilder::Some(&["class"]));

    fn update<'a>(
        &mut self,
        node_view: NodeView,
        _: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: &SendAnyMap,
    ) -> bool {
        let new = if let Some(class) = node_view.attributes().into_iter().flatten().next() {
            let classes = class.value.to_string();
            let classes: Vec<&str> = classes.split(' ').collect();

            let mut font = FontId {
                size: DEFAULT_FONT_SIZE,
                family: FontFamily::Proportional,
            };
            for class in classes {
                if let Some(class) = class.strip_prefix("text-") {
                    if let Ok(size) = class.parse::<f32>() {
                        font.size = size;
                    }
                }

                if let Some(class) = class.strip_prefix("font-") {
                    match class {
                        "sans" => {
                            font.family = FontFamily::Proportional;
                        }
                        "mono" => {
                            font.family = FontFamily::Monospace;
                        }
                        str => {
                            font.family = FontFamily::Name(str.into());
                        }
                    }
                }
            }

            font
        } else if let Some((parent_size,)) = parent {
            parent_size.0.clone()
        } else {
            return false;
        };

        if self.0 != new {
            *self = Self(new);
            true
        } else {
            false
        }
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
