//! Plain-old-data vertex types and the quad geometry the built-in pipelines draw.
//!
//! Each type is `#[repr(C)]` and `Pod`, so it can be uploaded to a vertex buffer with
//! `bytemuck::cast_slice`. Its `desc` function returns the matching
//! [`VertexBufferLayout`](wgpu::VertexBufferLayout); the two must stay in step, which the tests at
//! the bottom of this file enforce.

use bytemuck::{Pod, Zeroable};

/// A vertex with texture coordinates
#[repr(C)]
#[derive(Default, Copy, Clone, Debug, Pod, Zeroable)]
pub struct TexturedVertex {
    /// Homogeneous position.
    pub position: [f32; 4],
    /// Vertex colour, multiplied with the sampled texel by the quad shader.
    pub color: [f32; 4],
    /// Texture coordinates, with `[0.0, 0.0]` at the top left.
    pub tex_coords: [f32; 2],
}

impl TexturedVertex {
    /// The vertex buffer layout matching this type: position at location 0, colour at 1, texture
    /// coordinates at 2.
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

/// A vertex with texture coordinates but no colour, for passes that only sample a texture.
#[repr(C)]
#[derive(Default, Copy, Clone, Debug, Pod, Zeroable)]
pub struct SimpleTexturedVertex {
    /// Homogeneous position.
    pub position: [f32; 4],
    /// Texture coordinates, with `[0.0, 0.0]` at the top left.
    pub tex_coords: [f32; 2],
}

impl SimpleTexturedVertex {
    /// The vertex buffer layout matching this type: position at location 0, texture coordinates
    /// at 1.
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

/// A vertex with a colour and no texture, used by [`LinePipeline`](crate::pipelines::LinePipeline).
#[repr(C)]
#[derive(Default, Copy, Clone, Debug, Pod, Zeroable)]
pub struct ColoredVertex {
    /// Homogeneous position.
    pub position: [f32; 4],
    /// Linear RGBA colour.
    pub color: [f32; 4],
}

impl ColoredVertex {
    /// A vertex at `pos` in the z = 0 plane.
    pub fn new_2d(pos: [f32; 2], color: [f32; 4]) -> ColoredVertex {
        ColoredVertex {
            position: [pos[0], pos[1], 0.0, 1.0],
            color,
        }
    }

    /// The vertex buffer layout matching this type: position at location 0, colour at 1.
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

/// A unit quad centred on the origin, white, wound to be drawn with [`QUAD_INDICES`].
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

/// The two triangles that make up [`TEXTURED_QUAD_VERTICES`].
pub const QUAD_INDICES: &[u16] = &[0, 2, 1, 0, 3, 2];

/// An oversized triangle that covers the whole clip space, for fullscreen passes.
///
/// Drawing one triangle rather than two avoids the diagonal seam where a quad's triangles meet, so
/// it is the usual choice for post-processing.
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

#[cfg(test)]
mod tests {
    use wgpu::VertexFormat;

    use super::*;

    /// A layout that disagrees with its vertex type reads garbage on the GPU without failing
    /// anywhere visible, so check the stride and every attribute against the type it describes.
    fn assert_layout_matches<T>(layout: &wgpu::VertexBufferLayout<'_>) {
        assert_eq!(
            layout.array_stride,
            size_of::<T>() as wgpu::BufferAddress,
            "array_stride must equal the size of the vertex type"
        );
        assert_eq!(layout.step_mode, wgpu::VertexStepMode::Vertex);

        let mut expected_offset = 0;
        for attribute in layout.attributes {
            assert_eq!(
                attribute.offset, expected_offset,
                "attribute at location {} is not tightly packed after the previous one",
                attribute.shader_location
            );
            expected_offset += attribute.format.size();
        }
        assert_eq!(
            expected_offset, layout.array_stride,
            "attributes must cover the whole vertex, with no trailing padding"
        );
    }

    #[test]
    fn textured_vertex_layout_matches_its_type() {
        let layout = TexturedVertex::desc();
        assert_layout_matches::<TexturedVertex>(&layout);
        let formats: Vec<_> = layout.attributes.iter().map(|a| a.format).collect();
        assert_eq!(formats, vec![
            VertexFormat::Float32x4,
            VertexFormat::Float32x4,
            VertexFormat::Float32x2
        ]);
    }

    #[test]
    fn simple_textured_vertex_layout_matches_its_type() {
        assert_layout_matches::<SimpleTexturedVertex>(&SimpleTexturedVertex::desc());
    }

    #[test]
    fn colored_vertex_layout_matches_its_type() {
        assert_layout_matches::<ColoredVertex>(&ColoredVertex::desc());
    }

    #[test]
    fn shader_locations_are_sequential_from_zero() {
        for layout in [
            TexturedVertex::desc(),
            SimpleTexturedVertex::desc(),
            ColoredVertex::desc(),
        ] {
            for (i, attribute) in layout.attributes.iter().enumerate() {
                assert_eq!(attribute.shader_location, i as u32);
            }
        }
    }

    #[test]
    fn quad_indices_stay_within_the_quad() {
        let vertex_count = TEXTURED_QUAD_VERTICES.len() as u16;
        assert!(QUAD_INDICES.iter().all(|&i| i < vertex_count));
        assert_eq!(
            QUAD_INDICES.len() % 3,
            0,
            "indices must form whole triangles"
        );
    }

    #[test]
    fn the_fullscreen_triangle_covers_clip_space() {
        // Every clip-space corner must fall inside the triangle, or the pass leaves gaps.
        let vertices: Vec<[f32; 2]> = FULL_SCREEN_TRIANGLE_VERTICES
            .iter()
            .map(|v| [v.position[0], v.position[1]])
            .collect();
        for corner in [[-1.0, -1.0], [-1.0, 1.0], [1.0, -1.0], [1.0, 1.0]] {
            assert!(
                point_in_triangle(corner, vertices[0], vertices[1], vertices[2]),
                "clip-space corner {corner:?} is not covered"
            );
        }
    }

    fn point_in_triangle(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
        let sign = |p: [f32; 2], q: [f32; 2], r: [f32; 2]| {
            (p[0] - r[0]) * (q[1] - r[1]) - (q[0] - r[0]) * (p[1] - r[1])
        };
        let d1 = sign(p, a, b);
        let d2 = sign(p, b, c);
        let d3 = sign(p, c, a);
        let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
        let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
        !(has_neg && has_pos)
    }
}
