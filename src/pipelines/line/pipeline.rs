use std::{borrow::Cow, ops::Range};

use bytemuck::{Pod, Zeroable};
use wgpu::{
    util::DeviceExt, Buffer, Device, PushConstantRange, RenderPass, RenderPipeline, ShaderStages,
};

use crate::pipelines::ColoredVertex;

pub struct LinePipeline {
    pipeline: RenderPipeline,
    vertices: Buffer,
}

impl LinePipeline {
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
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..std::mem::size_of::<LinePushConstants>() as u32,
            }],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[ColoredVertex::desc()],
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
            multiview: None,
            cache: None,
        });
        pipeline
    }

    pub fn draw<'r>(&'r self, rpass: &mut RenderPass<'r>, view_proj: [[f32; 4]; 4], line: Line) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, self.vertices.slice(..));
        rpass.set_push_constants(
            ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::cast_slice(&[LinePushConstants::new(view_proj, line)]),
        );
        rpass.draw(0..2, 0..1);
    }

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
        rpass.set_push_constants(
            ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::cast_slice(&[LinePushConstants::buffer(view_proj)]),
        );
        rpass.draw(vertices, 0..1);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LinePushConstants {
    pub view_proj: [[f32; 4]; 4],
    pub start: [f32; 4],
    pub end: [f32; 4],
    pub color: [f32; 4],
}

impl LinePushConstants {
    pub fn new(view_proj: [[f32; 4]; 4], line: Line) -> LinePushConstants {
        LinePushConstants {
            view_proj,
            start: [line.start[0], line.start[1], line.start[2], 1.0],
            end: [line.end[0], line.end[1], line.end[2], 1.0],
            color: line.color,
        }
    }

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
pub struct Line {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub color: [f32; 4],
}

impl Line {
    pub fn new(start: [f32; 3], end: [f32; 3], color: [f32; 4]) -> Line {
        Line {
            start,
            end,
            color,
        }
    }
}
