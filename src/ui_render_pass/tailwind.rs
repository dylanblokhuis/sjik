use std::collections::HashMap;

use taffy::prelude::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct VisualStyle {
    pub background_color: [f32; 4],    
}

#[derive(Debug, Default, Clone)]
pub struct Tailwind {
    classes: String,
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
            vec![("100", [254, 242, 242, 255]), ("200", [254, 226, 226, 255])]
                .into_iter()
                .collect(),
        );

        tw.layout_style = Style {
            position: Position::Relative,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        };

        let classes: Vec<&str> = classes.split(' ').collect();

        for class in classes.iter() {
            if let Some(class) = class.strip_prefix("w-") {
                tw.layout_style.size.width = Dimension::Points(class.parse::<f32>().unwrap());
            }

            if let Some(class) = class.strip_prefix("h-") {
                tw.layout_style.size.height = Dimension::Points(class.parse::<f32>().unwrap());
            }

            if let Some(class) = class.strip_prefix("bg-") {
                tw.visual_style.background_color = tw.handle_color(class, &colors);
            }
        }

        tw
    }

    fn handle_color(&self, class: &str, colors: &Colors) -> [f32; 4] {
        // check check color then variant
        let color_and_variant: Vec<&str> = class.split('-').collect();
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

struct Color {
    variants: Vec<(String, String)>,
}
