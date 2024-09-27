use glam::Vec2;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum SandType {
    Empty,
    Sand,
    Water,
}

impl SandType {
    pub fn is_empty(&self) -> bool {
        *self == SandType::Empty
    }

    pub fn is_water(&self) -> bool {
        *self == SandType::Water
    }
}

#[derive(Copy, Clone)]
pub struct Sand {
    pub sand: SandType,
    pub color: [u8; 3],
}

impl Sand {
    pub fn from_type(sand: SandType, pos: Vec2) -> Sand {
        match sand {
            SandType::Sand => Self::sand(pos),
            SandType::Empty => Self::empty(),
            SandType::Water => Self::water(),
        }
    }

    pub fn sand(pos: Vec2) -> Sand {
        let rand_color = variate_color([0xc2, 0xb2, 0x80], 0.1, pos);
        Sand {
            sand: SandType::Sand,
            color: rand_color,
        }
    }

    pub fn water() -> Sand {
        let color = [0x23, 0x89, 0xda];
        Sand {
            sand: SandType::Water,
            color,
        }
    }

    pub fn empty() -> Sand {
        Sand {
            sand: SandType::Empty,
            color: [0; 3],
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

const PHI: f32 = 1.61803;

fn sticky_rand(pos: Vec2) -> f32 {
    let seed = 123.0;
    ((pos * PHI).distance(pos) * seed * pos.x).tan().fract()
}
