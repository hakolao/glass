use egui::{Color32, Rgba};
use glam::Vec2;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum SandType {
    Empty,
    Sand,
}

impl SandType {
    pub fn is_empty(&self) -> bool {
        *self == SandType::Empty
    }
}

#[derive(Copy, Clone)]
pub struct Sand {
    pub sand: SandType,
    pub color: Rgba,
}

impl Sand {
    pub fn from_type(sand: SandType, pos: Vec2) -> Sand {
        match sand {
            SandType::Sand => Self::sand(pos),
            SandType::Empty => Self::empty(),
        }
    }

    pub fn sand(pos: Vec2) -> Sand {
        let rand_color = variate_color([0xc2, 0xb2, 0x80], 0.1, pos);
        let color = Color32::from_rgb(rand_color[0], rand_color[1], rand_color[2]);
        Sand {
            sand: SandType::Sand,
            color: Rgba::from(color),
        }
    }

    pub fn empty() -> Sand {
        Sand {
            sand: SandType::Empty,
            color: Rgba::BLACK,
        }
    }
}

pub fn variate_color(color: [u8; 3], range: f32, seed: Vec2) -> [u8; 3] {
    let p = sticky_rand(seed);
    let r = color[0] as f32 / 255.0;
    let g = color[1] as f32 / 255.0;
    let b = color[2] as f32 / 255.0;
    let variation = -(range / 2.0) + range * p;
    let r = ((r + variation).clamp(0.0, 1.0) * 255.0) as u8;
    let g = ((g + variation).clamp(0.0, 1.0) * 255.0) as u8;
    let b = ((b + variation).clamp(0.0, 1.0) * 255.0) as u8;
    [r, g, b]
}

fn sticky_rand(pos: Vec2) -> f32 {
    let seed = 123.0;
    ((pos * std::f32::consts::PHI).distance(pos) * seed * pos.x)
        .tan()
        .fract()
}
