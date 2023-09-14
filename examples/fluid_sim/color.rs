use egui::Color32;

#[derive(Default, Copy, Clone)]
pub struct Color {
    pub color: [f32; 4],
}

impl Color {
    #[allow(unused)]
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Color {
        let color = Color32::from_rgba_premultiplied(r, g, b, a).to_normalized_gamma_f32();
        Color {
            color,
        }
    }
}

impl From<Color32> for Color {
    fn from(value: Color32) -> Self {
        Color {
            color: value.to_normalized_gamma_f32(),
        }
    }
}
