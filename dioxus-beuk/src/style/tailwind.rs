use std::collections::HashMap;

use dioxus_native_core::prelude::*;
use dioxus_native_core_macro::partial_derive_state;

use lightningcss::properties::border::{BorderColor, BorderSideWidth, BorderWidth};
use lightningcss::properties::border_radius::BorderRadius;
use lightningcss::values::color::CssColor;
use lightningcss::values::size::Size2D;
use shipyard::Component;
use taffy::prelude::*;
use taffy::style::Style;

type Colors = HashMap<&'static str, HashMap<&'static str, [u8; 4]>>;
#[derive(Clone, PartialEq, Debug, Component)]
pub(crate) struct Border {
    pub colors: BorderColor,
    pub width: BorderWidth,
    pub radius: BorderRadius,
}

#[derive(Clone, PartialEq, Debug, Component)]
pub(crate) struct Tailwind {
    pub color: CssColor,
    pub background_color: CssColor,
    pub border: Border,
    pub style: Style,
    pub node: Option<Node>,
}

#[partial_derive_state]
impl State for Tailwind {
    type ChildDependencies = (Self,);
    type ParentDependencies = ();
    type NodeDependencies = ();

    const NODE_MASK: NodeMaskBuilder<'static> =
        NodeMaskBuilder::new().with_attrs(AttributeMaskBuilder::Some(&["class"]));

    fn update<'a>(
        &mut self,
        node_view: NodeView,
        _: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        _: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        context: &SendAnyMap,
    ) -> bool {
        let taffy: &std::sync::Arc<std::sync::Mutex<Taffy>> = context.get().unwrap();
        // let text_context: &Arc<Mutex<TextContext>> = context.get().unwrap();
        let mut taffy = taffy.lock().unwrap();
        let mut changed = false;
        let mut classes = String::new();
        if let Some(class_attr) = node_view
            .attributes()
            .into_iter()
            .flatten()
            .find(|attr| attr.attribute.name == "class")
        {
            classes = class_attr.value.to_string();
        };
        let mut colors = Colors::new();
        insert_default_colors(&mut colors);

        let classes: Vec<&str> = classes.split(' ').collect();

        let mut style = Style::default();
        for class in classes.iter() {
            if class == &"flex-col" {
                style.display = Display::Flex;
                style.flex_direction = FlexDirection::Column;
            } else if class == &"flex-row" {
                style.display = Display::Flex;
                style.flex_direction = FlexDirection::Row;
            }

            if let Some(class) = class.strip_prefix("w-") {
                style.size.width = Self::handle_size(class);
            }

            if let Some(class) = class.strip_prefix("h-") {
                style.size.height = Self::handle_size(class);
            }

            if let Some(class) = class.strip_prefix("bg-") {
                if let Some(color) = Self::handle_color(class, &colors) {
                    self.background_color = color;
                }
            }

            if let Some(class) = class.strip_prefix("text-") {
                if let Some(color) = Self::handle_color(class, &colors) {
                    self.color = color;
                }
                // handle other text- classes here
            }

            if let Some(class) = class.strip_prefix("p-") {
                let padding = LengthPercentage::Points(class.parse::<f32>().unwrap());
                style.padding = Rect {
                    top: padding,
                    bottom: padding,
                    left: padding,
                    right: padding,
                }
            }

            if let Some(class) = class.strip_prefix("py-") {
                let padding = LengthPercentage::Points(class.parse::<f32>().unwrap());
                style.padding.top = padding;
                style.padding.bottom = padding;
            }

            if let Some(class) = class.strip_prefix("px-") {
                let padding = LengthPercentage::Points(class.parse::<f32>().unwrap());
                style.padding.left = padding;
                style.padding.right = padding;
            }

            if let Some(class) = class.strip_prefix("rounded-") {
                let value = class.parse::<f32>().unwrap();
                self.border.radius.bottom_left = Size2D(
                    lightningcss::values::percentage::DimensionPercentage::Dimension(
                        lightningcss::values::length::LengthValue::Px(value),
                    ),
                    lightningcss::values::percentage::DimensionPercentage::Dimension(
                        lightningcss::values::length::LengthValue::Px(value),
                    ),
                );
                self.border.radius.bottom_right = Size2D(
                    lightningcss::values::percentage::DimensionPercentage::Dimension(
                        lightningcss::values::length::LengthValue::Px(value),
                    ),
                    lightningcss::values::percentage::DimensionPercentage::Dimension(
                        lightningcss::values::length::LengthValue::Px(value),
                    ),
                );
                self.border.radius.top_left = Size2D(
                    lightningcss::values::percentage::DimensionPercentage::Dimension(
                        lightningcss::values::length::LengthValue::Px(value),
                    ),
                    lightningcss::values::percentage::DimensionPercentage::Dimension(
                        lightningcss::values::length::LengthValue::Px(value),
                    ),
                );
                self.border.radius.top_right = Size2D(
                    lightningcss::values::percentage::DimensionPercentage::Dimension(
                        lightningcss::values::length::LengthValue::Px(value),
                    ),
                    lightningcss::values::percentage::DimensionPercentage::Dimension(
                        lightningcss::values::length::LengthValue::Px(value),
                    ),
                );
            }

            if let Some(class) = class.strip_prefix("border-") {
                if let Some(color) = Self::handle_color(class, &colors) {
                    self.border.colors.top = color.clone();
                    self.border.colors.bottom = color.clone();
                    self.border.colors.left = color.clone();
                    self.border.colors.right = color;
                } else {
                    let value = class.parse::<f32>().unwrap();
                    self.border.width.top =
                        BorderSideWidth::Length(lightningcss::values::length::Length::Value(
                            lightningcss::values::length::LengthValue::Px(value),
                        ));

                    self.border.width.bottom =
                        BorderSideWidth::Length(lightningcss::values::length::Length::Value(
                            lightningcss::values::length::LengthValue::Px(value),
                        ));

                    self.border.width.left =
                        BorderSideWidth::Length(lightningcss::values::length::Length::Value(
                            lightningcss::values::length::LengthValue::Px(value),
                        ));

                    self.border.width.right =
                        BorderSideWidth::Length(lightningcss::values::length::Length::Value(
                            lightningcss::values::length::LengthValue::Px(value),
                        ));
                }
            }

            if let Some(class) = class.strip_prefix("justify-") {
                style.justify_content = Some(match class {
                    "start" => JustifyContent::Start,
                    "end" => JustifyContent::End,
                    "center" => JustifyContent::Center,
                    "between" => JustifyContent::SpaceBetween,
                    "around" => JustifyContent::SpaceAround,
                    "evenly" => JustifyContent::SpaceEvenly,
                    "stretch" => JustifyContent::Stretch,
                    _ => panic!("Unknown justify content {class}"),
                })
            }

            if let Some(class) = class.strip_prefix("items-") {
                style.align_items = Some(match class {
                    "start" => AlignItems::FlexStart,
                    "end" => AlignItems::FlexEnd,
                    "center" => AlignItems::Center,
                    "baseline" => AlignItems::Baseline,
                    "stretch" => AlignItems::Stretch,
                    _ => panic!("Unknown align items {class}"),
                })
            }
        }

        let mut child_layout = vec![];
        for (l,) in children {
            child_layout.push(l.node.unwrap());
        }

        let style_has_changed = self.style != style;
        if let Some(n) = self.node {
            if taffy.children(n).unwrap() != child_layout {
                taffy.set_children(n, &child_layout).unwrap();
                changed = true;
            }
            if style_has_changed {
                taffy.set_style(n, style.clone()).unwrap();
                changed = true;
            }
        } else {
            self.node = Some(
                taffy
                    .new_with_children(style.clone(), &child_layout)
                    .unwrap(),
            );
            changed = true;
        }

        if style_has_changed {
            self.style = style;
            changed = true;
        }

        changed
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

impl Default for Tailwind {
    fn default() -> Self {
        Self {
            border: Border {
                colors: BorderColor {
                    top: CssColor::default(),
                    right: CssColor::default(),
                    bottom: CssColor::default(),
                    left: CssColor::default(),
                },
                radius: BorderRadius::default(),
                width: BorderWidth {
                    top: BorderSideWidth::Length(lightningcss::values::length::Length::Value(
                        lightningcss::values::length::LengthValue::Px(0.0),
                    )),
                    right: BorderSideWidth::Length(lightningcss::values::length::Length::Value(
                        lightningcss::values::length::LengthValue::Px(0.0),
                    )),
                    bottom: BorderSideWidth::Length(lightningcss::values::length::Length::Value(
                        lightningcss::values::length::LengthValue::Px(0.0),
                    )),
                    left: BorderSideWidth::Length(lightningcss::values::length::Length::Value(
                        lightningcss::values::length::LengthValue::Px(0.0),
                    )),
                },
            },
            background_color: CssColor::default(),
            color: CssColor::default(),
            node: None,
            style: Style::default(),
        }
    }
}

impl Tailwind {
    fn handle_size(class: &str) -> Dimension {
        match class {
            "full" => Dimension::Percent(100.0),
            "auto" => Dimension::AUTO,
            class => {
                if class.ends_with('%') {
                    println!(
                        "{:?}",
                        class.strip_suffix('%').unwrap().parse::<f32>().unwrap()
                    );

                    Dimension::Percent(class.strip_suffix('%').unwrap().parse::<f32>().unwrap())
                } else {
                    Dimension::Points(class.parse::<f32>().unwrap())
                }
            }
        }
    }

    fn handle_color(class: &str, colors: &Colors) -> Option<CssColor> {
        // check check color then variant
        let color_and_variant: Vec<&str> = class.split('-').collect();
        if color_and_variant.len() != 2 {
            return match color_and_variant[0] {
                "transparent" => Some(CssColor::RGBA(cssparser::RGBA {
                    red: 0,
                    green: 0,
                    blue: 0,
                    alpha: 0,
                })),

                "white" => Some(CssColor::RGBA(cssparser::RGBA {
                    red: 255,
                    green: 255,
                    blue: 255,
                    alpha: 255,
                })),
                "black" => Some(CssColor::RGBA(cssparser::RGBA {
                    red: 0,
                    green: 0,
                    blue: 0,
                    alpha: 255,
                })),
                _ => colors.get(color_and_variant[0]).map(|c| {
                    let (_, variant) = c.iter().next().unwrap();
                    CssColor::RGBA(cssparser::RGBA {
                        red: variant[0],
                        green: variant[1],
                        blue: variant[2],
                        alpha: variant[3],
                    })
                }),
            };
        }
        let color = color_and_variant[0];
        let variant = color_and_variant[1];

        let Some(variants) = colors.get(color) else {
            return None;
        };

        let Some(variant_color) = variants.get(variant) else {
            return None;
        };

        let [r, g, b, a] = variant_color;

        Some(CssColor::RGBA(cssparser::RGBA {
            red: *r,
            green: *g,
            blue: *b,
            alpha: *a,
        }))
    }
}

pub fn insert_default_colors(colors: &mut Colors) {
    colors.insert(
        "slate",
        vec![
            ("50", [248, 250, 252, 255]),
            ("100", [241, 245, 249, 255]),
            ("200", [226, 232, 240, 255]),
            ("300", [203, 213, 225, 255]),
            ("400", [148, 163, 184, 255]),
            ("500", [100, 116, 139, 255]),
            ("600", [71, 85, 105, 255]),
            ("700", [51, 65, 85, 255]),
            ("800", [30, 41, 59, 255]),
            ("900", [15, 23, 42, 255]),
            ("950", [2, 6, 23, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "gray",
        vec![
            ("50", [249, 250, 251, 255]),
            ("100", [243, 244, 246, 255]),
            ("200", [229, 231, 235, 255]),
            ("300", [209, 213, 219, 255]),
            ("400", [156, 163, 175, 255]),
            ("500", [107, 114, 128, 255]),
            ("600", [75, 85, 99, 255]),
            ("700", [55, 65, 81, 255]),
            ("800", [31, 41, 55, 255]),
            ("900", [17, 24, 39, 255]),
            ("950", [3, 7, 18, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "zinc",
        vec![
            ("50", [250, 250, 250, 255]),
            ("100", [244, 244, 245, 255]),
            ("200", [228, 228, 231, 255]),
            ("300", [212, 212, 216, 255]),
            ("400", [161, 161, 170, 255]),
            ("500", [113, 113, 122, 255]),
            ("600", [82, 82, 91, 255]),
            ("700", [63, 63, 70, 255]),
            ("800", [39, 39, 42, 255]),
            ("900", [24, 24, 27, 255]),
            ("950", [9, 9, 11, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "neutral",
        vec![
            ("50", [250, 250, 250, 255]),
            ("100", [245, 245, 245, 255]),
            ("200", [229, 229, 229, 255]),
            ("300", [212, 212, 212, 255]),
            ("400", [163, 163, 163, 255]),
            ("500", [115, 115, 115, 255]),
            ("600", [82, 82, 82, 255]),
            ("700", [64, 64, 64, 255]),
            ("800", [38, 38, 38, 255]),
            ("900", [23, 23, 23, 255]),
            ("950", [10, 10, 10, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "stone",
        vec![
            ("50", [250, 250, 249, 255]),
            ("100", [245, 245, 244, 255]),
            ("200", [231, 229, 228, 255]),
            ("300", [214, 211, 209, 255]),
            ("400", [168, 162, 158, 255]),
            ("500", [120, 113, 108, 255]),
            ("600", [87, 83, 78, 255]),
            ("700", [68, 64, 60, 255]),
            ("800", [41, 37, 36, 255]),
            ("900", [28, 25, 23, 255]),
            ("950", [12, 10, 9, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "red",
        vec![
            ("50", [254, 242, 242, 255]),
            ("100", [254, 226, 226, 255]),
            ("200", [254, 202, 202, 255]),
            ("300", [252, 165, 165, 255]),
            ("400", [248, 113, 113, 255]),
            ("500", [239, 68, 68, 255]),
            ("600", [220, 38, 38, 255]),
            ("700", [185, 28, 28, 255]),
            ("800", [153, 27, 27, 255]),
            ("900", [127, 29, 29, 255]),
            ("950", [69, 10, 10, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "orange",
        vec![
            ("50", [255, 247, 237, 255]),
            ("100", [255, 237, 213, 255]),
            ("200", [254, 215, 170, 255]),
            ("300", [253, 186, 116, 255]),
            ("400", [251, 146, 60, 255]),
            ("500", [249, 115, 22, 255]),
            ("600", [234, 88, 12, 255]),
            ("700", [194, 65, 12, 255]),
            ("800", [154, 52, 18, 255]),
            ("900", [124, 45, 18, 255]),
            ("950", [67, 20, 7, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "amber",
        vec![
            ("50", [255, 251, 235, 255]),
            ("100", [254, 243, 199, 255]),
            ("200", [253, 230, 138, 255]),
            ("300", [252, 211, 77, 255]),
            ("400", [251, 191, 36, 255]),
            ("500", [245, 158, 11, 255]),
            ("600", [217, 119, 6, 255]),
            ("700", [180, 83, 9, 255]),
            ("800", [146, 64, 14, 255]),
            ("900", [120, 53, 15, 255]),
            ("950", [69, 26, 3, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "yellow",
        vec![
            ("50", [254, 252, 232, 255]),
            ("100", [254, 249, 195, 255]),
            ("200", [254, 240, 138, 255]),
            ("300", [253, 224, 71, 255]),
            ("400", [250, 204, 21, 255]),
            ("500", [234, 179, 8, 255]),
            ("600", [202, 138, 4, 255]),
            ("700", [161, 98, 7, 255]),
            ("800", [133, 77, 14, 255]),
            ("900", [113, 63, 18, 255]),
            ("950", [66, 32, 6, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "lime",
        vec![
            ("50", [247, 254, 231, 255]),
            ("100", [236, 252, 203, 255]),
            ("200", [217, 249, 157, 255]),
            ("300", [190, 242, 100, 255]),
            ("400", [163, 230, 53, 255]),
            ("500", [132, 204, 22, 255]),
            ("600", [101, 163, 13, 255]),
            ("700", [77, 124, 15, 255]),
            ("800", [63, 98, 18, 255]),
            ("900", [54, 83, 20, 255]),
            ("950", [26, 46, 5, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "green",
        vec![
            ("50", [240, 253, 244, 255]),
            ("100", [220, 252, 231, 255]),
            ("200", [187, 247, 208, 255]),
            ("300", [134, 239, 172, 255]),
            ("400", [74, 222, 128, 255]),
            ("500", [34, 197, 94, 255]),
            ("600", [22, 163, 74, 255]),
            ("700", [21, 128, 61, 255]),
            ("800", [22, 101, 52, 255]),
            ("900", [20, 83, 45, 255]),
            ("950", [5, 46, 22, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "emerald",
        vec![
            ("50", [236, 253, 245, 255]),
            ("100", [209, 250, 229, 255]),
            ("200", [167, 243, 208, 255]),
            ("300", [110, 231, 183, 255]),
            ("400", [52, 211, 153, 255]),
            ("500", [16, 185, 129, 255]),
            ("600", [5, 150, 105, 255]),
            ("700", [4, 120, 87, 255]),
            ("800", [6, 95, 70, 255]),
            ("900", [6, 78, 59, 255]),
            ("950", [2, 44, 34, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "teal",
        vec![
            ("50", [240, 253, 250, 255]),
            ("100", [204, 251, 241, 255]),
            ("200", [153, 246, 228, 255]),
            ("300", [94, 234, 212, 255]),
            ("400", [45, 212, 191, 255]),
            ("500", [20, 184, 166, 255]),
            ("600", [13, 148, 136, 255]),
            ("700", [15, 118, 110, 255]),
            ("800", [17, 94, 89, 255]),
            ("900", [19, 78, 74, 255]),
            ("950", [4, 47, 46, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "cyan",
        vec![
            ("50", [236, 254, 255, 255]),
            ("100", [207, 250, 254, 255]),
            ("200", [165, 243, 252, 255]),
            ("300", [103, 232, 249, 255]),
            ("400", [34, 211, 238, 255]),
            ("500", [6, 182, 212, 255]),
            ("600", [8, 145, 178, 255]),
            ("700", [14, 116, 144, 255]),
            ("800", [21, 94, 117, 255]),
            ("900", [22, 78, 99, 255]),
            ("950", [8, 51, 68, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "sky",
        vec![
            ("50", [240, 249, 255, 255]),
            ("100", [224, 242, 254, 255]),
            ("200", [186, 230, 253, 255]),
            ("300", [125, 211, 252, 255]),
            ("400", [56, 189, 248, 255]),
            ("500", [14, 165, 233, 255]),
            ("600", [2, 132, 199, 255]),
            ("700", [3, 105, 161, 255]),
            ("800", [7, 89, 133, 255]),
            ("900", [12, 74, 110, 255]),
            ("950", [8, 47, 73, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "blue",
        vec![
            ("50", [239, 246, 255, 255]),
            ("100", [219, 234, 254, 255]),
            ("200", [191, 219, 254, 255]),
            ("300", [147, 197, 253, 255]),
            ("400", [96, 165, 250, 255]),
            ("500", [59, 130, 246, 255]),
            ("600", [37, 99, 235, 255]),
            ("700", [29, 78, 216, 255]),
            ("800", [30, 64, 175, 255]),
            ("900", [30, 58, 138, 255]),
            ("950", [23, 37, 84, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "indigo",
        vec![
            ("50", [238, 242, 255, 255]),
            ("100", [224, 231, 255, 255]),
            ("200", [199, 210, 254, 255]),
            ("300", [165, 180, 252, 255]),
            ("400", [129, 140, 248, 255]),
            ("500", [99, 102, 241, 255]),
            ("600", [79, 70, 229, 255]),
            ("700", [67, 56, 202, 255]),
            ("800", [55, 48, 163, 255]),
            ("900", [49, 46, 129, 255]),
            ("950", [30, 27, 75, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "violet",
        vec![
            ("50", [245, 243, 255, 255]),
            ("100", [237, 233, 254, 255]),
            ("200", [221, 214, 254, 255]),
            ("300", [196, 181, 253, 255]),
            ("400", [167, 139, 250, 255]),
            ("500", [139, 92, 246, 255]),
            ("600", [124, 58, 237, 255]),
            ("700", [109, 40, 217, 255]),
            ("800", [91, 33, 182, 255]),
            ("900", [76, 29, 149, 255]),
            ("950", [46, 16, 101, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "purple",
        vec![
            ("50", [250, 245, 255, 255]),
            ("100", [243, 232, 255, 255]),
            ("200", [233, 213, 255, 255]),
            ("300", [216, 180, 254, 255]),
            ("400", [192, 132, 252, 255]),
            ("500", [168, 85, 247, 255]),
            ("600", [147, 51, 234, 255]),
            ("700", [126, 34, 206, 255]),
            ("800", [107, 33, 168, 255]),
            ("900", [88, 28, 135, 255]),
            ("950", [59, 7, 100, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "fuchsia",
        vec![
            ("50", [253, 244, 255, 255]),
            ("100", [250, 232, 255, 255]),
            ("200", [245, 208, 254, 255]),
            ("300", [240, 171, 252, 255]),
            ("400", [232, 121, 249, 255]),
            ("500", [217, 70, 239, 255]),
            ("600", [192, 38, 211, 255]),
            ("700", [162, 28, 175, 255]),
            ("800", [134, 25, 143, 255]),
            ("900", [112, 26, 117, 255]),
            ("950", [74, 4, 78, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "pink",
        vec![
            ("50", [253, 242, 248, 255]),
            ("100", [252, 231, 243, 255]),
            ("200", [251, 207, 232, 255]),
            ("300", [249, 168, 212, 255]),
            ("400", [244, 114, 182, 255]),
            ("500", [236, 72, 153, 255]),
            ("600", [219, 39, 119, 255]),
            ("700", [190, 24, 93, 255]),
            ("800", [157, 23, 77, 255]),
            ("900", [131, 24, 67, 255]),
            ("950", [80, 7, 36, 255]),
        ]
        .into_iter()
        .collect(),
    );
    colors.insert(
        "rose",
        vec![
            ("50", [255, 241, 242, 255]),
            ("100", [255, 228, 230, 255]),
            ("200", [254, 205, 211, 255]),
            ("300", [253, 164, 175, 255]),
            ("400", [251, 113, 133, 255]),
            ("500", [244, 63, 94, 255]),
            ("600", [225, 29, 72, 255]),
            ("700", [190, 18, 60, 255]),
            ("800", [159, 18, 57, 255]),
            ("900", [136, 19, 55, 255]),
            ("950", [76, 5, 25, 255]),
        ]
        .into_iter()
        .collect(),
    );
}