//! A pipeline that draws individual lines, or a whole buffer of them.

use std::{borrow::Cow, ops::Range};

use bytemuck::{Pod, Zeroable};
use wgpu::{util::DeviceExt, Buffer, Device, RenderPass, RenderPipeline};

use crate::pipelines::ColoredVertex;

/// Draws lines with [`LineList`](wgpu::PrimitiveTopology::LineList) topology.
///
/// Use [`LinePipeline::draw`] for one line at a time, or
/// [`LinePipeline::draw_line_buffer`] when you have many, which is far cheaper per line.
#[derive(Debug)]
pub struct LinePipeline {
    pipeline: RenderPipeline,
    vertices: Buffer,
}

impl LinePipeline {
    /// Builds the pipeline for a render target described by `color_target_state`, which must
    /// match the format of the attachment you draw into.
    pub fn new(device: &Device, color_target_state: wgpu::ColorTargetState) -> LinePipeline {
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&[ColoredVertex::new_2d([1.0, 1.0], [1.0; 4]); 2]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let pipeline = Self::new_render_pipeline(device, color_target_state);
        Self {
            pipeline,
            vertices,
        }
    }

    /// The underlying [`RenderPipeline`], for apps that manage their own vertex buffers.
    pub fn new_render_pipeline(
        device: &Device,
        color_target_state: wgpu::ColorTargetState,
    ) -> RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("line.wgsl"))),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Line Pipeline Layout"),
            bind_group_layouts: &[],
            immediate_size: size_of::<LinePushConstants>() as u32,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[ColoredVertex::desc().into()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(color_target_state)],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Line,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            cache: None,
            multiview_mask: None,
        });
        pipeline
    }

    /// Draws a single `line` through `view_proj`. The line is passed as immediate data, so no
    /// buffer upload is needed, but one draw call per line adds up quickly.
    pub fn draw<'r>(&'r self, rpass: &mut RenderPass<'r>, view_proj: [[f32; 4]; 4], line: Line) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, self.vertices.slice(..));
        rpass.set_immediates(
            0,
            bytemuck::cast_slice(&[LinePushConstants::new(view_proj, line)]),
        );
        rpass.draw(0..2, 0..1);
    }

    /// Draws `vertices` from `buffer` as a line list, two vertices per line.
    ///
    /// Buffer should contain [`ColoredVertex`]
    pub fn draw_line_buffer<'r>(
        &'r self,
        rpass: &mut RenderPass<'r>,
        view_proj: [[f32; 4]; 4],
        buffer: &'r Buffer,
        vertices: Range<u32>,
    ) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, buffer.slice(..));
        rpass.set_immediates(
            0,
            bytemuck::cast_slice(&[LinePushConstants::buffer(view_proj)]),
        );
        rpass.draw(vertices, 0..1);
    }
}

/// The immediate data `line.wgsl` reads.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LinePushConstants {
    /// Combined view and projection matrix.
    pub view_proj: [[f32; 4]; 4],
    /// Line start, in world space.
    pub start: [f32; 4],
    /// Line end, in world space.
    pub end: [f32; 4],
    /// Line colour.
    pub color: [f32; 4],
}

impl LinePushConstants {
    /// Immediate data for drawing `line` directly.
    pub fn new(view_proj: [[f32; 4]; 4], line: Line) -> LinePushConstants {
        LinePushConstants {
            view_proj,
            start: [line.start[0], line.start[1], line.start[2], 1.0],
            end: [line.end[0], line.end[1], line.end[2], 1.0],
            color: line.color,
        }
    }

    /// Immediate data for drawing from a vertex buffer, where the shader takes its positions
    /// and colours from the buffer and ignores the ones here.
    pub fn buffer(view_proj: [[f32; 4]; 4]) -> LinePushConstants {
        LinePushConstants {
            view_proj,
            start: [1.0, 1.0, 1.0, 1.0],
            end: [1.0, 1.0, 1.0, 1.0],
            color: [1.0; 4],
        }
    }
}

#[derive(Default, Copy, Clone, Debug)]
/// A single line segment in world space.
pub struct Line {
    /// Where the line starts.
    pub start: [f32; 3],
    /// Where the line ends.
    pub end: [f32; 3],
    /// Its colour.
    pub color: [f32; 4],
}

impl Line {
    /// A line from `start` to `end`, drawn in `color`.
    pub fn new(start: [f32; 3], end: [f32; 3], color: [f32; 4]) -> Line {
        Line {
            start,
            end,
            color,
        }
    }
}
