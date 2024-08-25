use bytemuck::{Pod, Zeroable};

/// A vertex with texture coordinates
#[repr(C)]
#[derive(Default, Copy, Clone, Debug, Pod, Zeroable)]
pub struct TexturedVertex {
    pub position: [f32; 4],
    pub color: [f32; 4],
    pub tex_coords: [f32; 2],
}

impl TexturedVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<TexturedVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 2 * size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Default, Copy, Clone, Debug, Pod, Zeroable)]
pub struct SimpleTexturedVertex {
    pub position: [f32; 4],
    pub tex_coords: [f32; 2],
}

impl SimpleTexturedVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SimpleTexturedVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Default, Copy, Clone, Debug, Pod, Zeroable)]
pub struct ColoredVertex {
    pub position: [f32; 4],
    pub color: [f32; 4],
}

impl ColoredVertex {
    pub fn new_2d(pos: [f32; 2], color: [f32; 4]) -> ColoredVertex {
        ColoredVertex {
            position: [pos[0], pos[1], 0.0, 1.0],
            color,
        }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ColoredVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[allow(unused)]
pub fn colored_quad_vertices(color: [f32; 4], size: [f32; 2]) -> Vec<TexturedVertex> {
    let half_x = size[0] / 2.0;
    let half_y = size[1] / 2.0;
    vec![
        TexturedVertex {
            color,
            position: [-half_x, -half_y, 0.0, 1.0],
            tex_coords: [0.0, 1.0],
        },
        TexturedVertex {
            color,
            position: [-half_x, half_y, 0.0, 1.0],
            tex_coords: [0.0, 0.0],
        },
        TexturedVertex {
            color,
            position: [half_x, half_y, 0.0, 1.0],
            tex_coords: [1.0, 0.0],
        },
        TexturedVertex {
            color,
            position: [half_x, -half_y, 0.0, 1.0],
            tex_coords: [1.0, 1.0],
        },
    ]
}

pub const TEXTURED_QUAD_VERTICES: &[TexturedVertex] = &[
    TexturedVertex {
        position: [-0.5, -0.5, 0.0, 1.0],
        color: [1.0; 4],
        tex_coords: [0.0, 1.0],
    },
    TexturedVertex {
        position: [-0.5, 0.5, 0.0, 1.0],
        color: [1.0; 4],
        tex_coords: [0.0, 0.0],
    },
    TexturedVertex {
        position: [0.5, 0.5, 0.0, 1.0],
        color: [1.0; 4],
        tex_coords: [1.0, 0.0],
    },
    TexturedVertex {
        position: [0.5, -0.5, 0.0, 1.0],
        color: [1.0; 4],
        tex_coords: [1.0, 1.0],
    },
];

pub const QUAD_INDICES: &[u16] = &[0, 2, 1, 0, 3, 2];

pub const FULL_SCREEN_TRIANGLE_VERTICES: &[SimpleTexturedVertex] = &[
    SimpleTexturedVertex {
        position: [-1.0, 1.0, 0.0, 1.0],
        tex_coords: [0.0, 1.0],
    },
    SimpleTexturedVertex {
        position: [-1.0, -3.0, 0.0, 1.0],
        tex_coords: [0.0, 0.0],
    },
    SimpleTexturedVertex {
        position: [3.0, 1.0, 0.0, 1.0],
        tex_coords: [1.0, 0.0],
    },
];
