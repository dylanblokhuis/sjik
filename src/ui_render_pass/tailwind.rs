use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Tailwind {
    classes: String,
    pub width: u32,
    pub height: u32,
    pub background_color: [f32; 4],
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

        let classes: Vec<&str> = classes.split(' ').collect();

        for class in classes.iter() {
            if let Some(class) = class.strip_prefix("w-") {
                tw.width = class.parse::<u32>().unwrap();
            }

            if let Some(class) = class.strip_prefix("h-") {
                tw.height = class.parse::<u32>().unwrap();
            }

            if let Some(class) = class.strip_prefix("bg-") {
                tw.background_color = tw.handle_color(class, &colors);
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
