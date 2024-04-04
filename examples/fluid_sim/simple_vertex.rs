use bytemuck::{Pod, Zeroable};

/// A vertex with position
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SimpleVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
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
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub const SIMPLE_QUAD_VERTICES: &[SimpleVertex] = &[
    SimpleVertex {
        position: [-0.5, -0.5],
        tex_coords: [0.0, 1.0],
    },
    SimpleVertex {
        position: [-0.5, 0.5],
        tex_coords: [0.0, 0.0],
    },
    SimpleVertex {
        position: [0.5, 0.5],
        tex_coords: [1.0, 0.0],
    },
    SimpleVertex {
        position: [0.5, -0.5],
        tex_coords: [1.0, 1.0],
    },
];

pub const SIMPLE_QUAD_INDICES: &[u16] = &[0, 2, 1, 0, 3, 2];
