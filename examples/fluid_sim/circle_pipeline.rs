use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use glass::wgpu::{
    util::DeviceExt, BindGroup, Buffer, Device, PushConstantRange, RenderPass, RenderPipeline,
    ShaderStages,
};

use crate::{
    color::Color,
    simple_vertex::{SimpleVertex, SIMPLE_QUAD_INDICES, SIMPLE_QUAD_VERTICES},
};

pub struct CirclePipeline {
    pipeline: RenderPipeline,
    vertices: Buffer,
    indices: Buffer,
    bind_group: BindGroup,
}

impl CirclePipeline {
    pub fn new(device: &Device, color_target_state: wgpu::ColorTargetState) -> CirclePipeline {
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(SIMPLE_QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(SIMPLE_QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let pipeline = Self::new_render_pipeline(device, color_target_state);
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        // Must match layout :), but no inputs so easy...
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[],
            label: Some("bind_group"),
        });
        Self {
            pipeline,
            vertices,
            indices,
            bind_group,
        }
    }

    pub fn new_render_pipeline(
        device: &Device,
        color_target_state: wgpu::ColorTargetState,
    ) -> RenderPipeline {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[],
            label: Some("circle_bind_group_layout"),
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("circle.wgsl"))),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Circle Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..std::mem::size_of::<CirclePushConstants>() as u32,
            }],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Circle Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[SimpleVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(color_target_state)],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
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
        });
        pipeline
    }

    pub fn draw<'r>(
        &'r self,
        rpass: &mut RenderPass<'r>,
        view_proj: [[f32; 4]; 4],
        pos: [f32; 2],
        color: Color,
        radius: f32,
        thickness: f32,
        smoothness: f32,
    ) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertices.slice(..));
        rpass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
        rpass.set_push_constants(
            ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::cast_slice(&[CirclePushConstants::new(
                view_proj,
                pos,
                color.color,
                radius,
                thickness,
                smoothness,
            )]),
        );
        rpass.draw_indexed(0..(SIMPLE_QUAD_INDICES.len() as u32), 0, 0..1);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CirclePushConstants {
    pub view_proj: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub pos: [f32; 2],
    pub radius: f32,
    pub thickness: f32,
    pub smoothness: f32,
}

impl CirclePushConstants {
    pub fn new(
        view_proj: [[f32; 4]; 4],
        pos: [f32; 2],
        color: [f32; 4],
        radius: f32,
        thickness: f32,
        smoothness: f32,
    ) -> CirclePushConstants {
        CirclePushConstants {
            view_proj,
            pos,
            color,
            radius,
            thickness,
            smoothness,
        }
    }
}
