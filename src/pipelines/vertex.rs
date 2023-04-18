use bytemuck::{Pod, Zeroable};

/// A vertex with texture coordinates
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TexturedVertex {
    position: [f32; 4],
    color: [f32; 4],
    tex_coords: [f32; 2],
}

impl TexturedVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<TexturedVertex>() as wgpu::BufferAddress,
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
                wgpu::VertexAttribute {
                    offset: 2 * mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SimpleVertex {
    position: [f32; 4],
    tex_coords: [f32; 2],
}

impl SimpleVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SimpleVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 1 * mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
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

pub const FULL_SCREEN_TRIANGLE_VERTICES: &[SimpleVertex] = &[
    SimpleVertex {
        position: [-1.0, 1.0, 0.0, 1.0],
        tex_coords: [0.0, 1.0],
    },
    SimpleVertex {
        position: [-1.0, -3.0, 0.0, 1.0],
        tex_coords: [0.0, 0.0],
    },
    SimpleVertex {
        position: [3.0, 1.0, 0.0, 1.0],
        tex_coords: [1.0, 0.0],
    },
];
