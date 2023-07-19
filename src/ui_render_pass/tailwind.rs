use std::collections::HashMap;

use taffy::prelude::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct VisualStyle {
    pub background_color: [f32; 4],
    pub border_radius: [f32; 4],
}

#[derive(Debug, Default, Clone)]
pub struct Tailwind {
    pub layout_style: Style,
    pub visual_style: VisualStyle,
}

type Colors = HashMap<&'static str, HashMap<&'static str, [u32; 4]>>;

impl Tailwind {
    pub fn new(classes: &String) -> Self {
        let mut tw = Self::default();
        let mut colors = Colors::new();
        colors.insert(
            "red",
            vec![
                ("100", [254, 242, 242, 255]),
                ("200", [254, 226, 226, 255]),
                ("500", [244, 63, 94, 255]),
            ]
            .into_iter()
            .collect(),
        );
        colors.insert(
            "blue",
            vec![("500", [59, 130, 246, 255])].into_iter().collect(),
        );
        colors.insert(
            "green",
            vec![("500", [34, 197, 94, 255])].into_iter().collect(),
        );
        colors.insert(
            "green",
            vec![("500", [34, 197, 94, 255])].into_iter().collect(),
        );

        tw.layout_style = Style::default();
        let classes: Vec<&str> = classes.split(' ').collect();

        for class in classes.iter() {
            if class == &"flex-col" {
                tw.layout_style.display = Display::Flex;
                tw.layout_style.flex_direction = FlexDirection::Column;
            } else if class == &"flex-row" {
                tw.layout_style.display = Display::Flex;
                tw.layout_style.flex_direction = FlexDirection::Row;
            }

            if let Some(class) = class.strip_prefix("p-") {
                let padding = LengthPercentage::Points(class.parse::<f32>().unwrap());
                tw.layout_style.padding = Rect {
                    top: padding,
                    bottom: padding,
                    left: padding,
                    right: padding,
                }
            }

            if let Some(class) = class.strip_prefix("w-") {
                tw.layout_style.size.width = Self::handle_size(class);
            }

            if let Some(class) = class.strip_prefix("h-") {
                tw.layout_style.size.height = Self::handle_size(class);
            }

            if let Some(class) = class.strip_prefix("bg-") {
                tw.visual_style.background_color = tw.handle_color(class, &colors);
            }

            if let Some(class) = class.strip_prefix("justify-") {
                tw.layout_style.justify_content = Some(match class {
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
                tw.layout_style.align_items = Some(match class {
                    "start" => AlignItems::FlexStart,
                    "end" => AlignItems::FlexEnd,
                    "center" => AlignItems::Center,
                    "baseline" => AlignItems::Baseline,
                    "stretch" => AlignItems::Stretch,
                    _ => panic!("Unknown align items {class}"),
                })
            }

            if let Some(class) = class.strip_prefix("rounded-") {
                let radius = class.parse::<f32>().unwrap();
                tw.visual_style.border_radius = [radius; 4];
            }
        }

        tw
    }

    fn handle_size(class: &str) -> Dimension {
        match class {
            "full" => Dimension::Percent(100.0),
            "auto" => Dimension::AUTO,
            class => {
                if class.ends_with('%') {
                    Dimension::Percent(class.strip_suffix('%').unwrap().parse::<f32>().unwrap())
                } else {
                    Dimension::Points(class.parse::<f32>().unwrap())
                }
            }
        }
    }

    fn handle_color(&self, class: &str, colors: &Colors) -> [f32; 4] {
        // check check color then variant
        let color_and_variant: Vec<&str> = class.split('-').collect();
        if color_and_variant.len() != 2 && color_and_variant[0] == "transparent" {
            return [0.0, 0.0, 0.0, 0.0];
        }
        let color = color_and_variant[0];
        let variant = color_and_variant[1];

        let Some(variants) = colors.get(color) else {
            panic!("Color not found {color}");
        };

        let Some(variant_color) = variants.get(variant) else {
            panic!("Variant not found {variant} inside {color}");
        };

        let [r, g, b, a] = variant_color;

        [
            *r as f32 / 255.0,
            *g as f32 / 255.0,
            *b as f32 / 255.0,
            *a as f32 / 255.0,
        ]
    }
}
