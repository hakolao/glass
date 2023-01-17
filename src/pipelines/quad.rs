use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use wgpu::{PushConstantRange, ShaderStages};

use crate::{pipelines::vertex::TexturedVertex, GlassContext};

pub struct QuadPipeline {}

impl QuadPipeline {
    pub fn new_render_pipeline(
        context: &GlassContext,
        color_target_state: wgpu::ColorTargetState,
    ) -> wgpu::RenderPipeline {
        let texture_bind_group_layout =
            context
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float {
                                    filterable: true,
                                },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("texture_bind_group_layout"),
                });
        let shader = context
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("quad.wgsl"))),
            });
        let layout = context
            .device()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Quad Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[PushConstantRange {
                    stages: ShaderStages::VERTEX,
                    range: 0..std::mem::size_of::<QuadPushConstants>() as u32,
                }],
            });
        let pipeline = context
            .device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Quad Render Pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[TexturedVertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(color_target_state)],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
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

    pub fn push_constants(
        view_position: [f32; 4],
        view_proj: [[f32; 4]; 4],
        quad_size: [f32; 2],
    ) -> QuadPushConstants {
        QuadPushConstants {
            view_position,
            view_proj,
            dims: quad_size,
        }
    }
}

/// Quad instance specific values passed to the shader.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct QuadPushConstants {
    pub view_position: [f32; 4],
    pub view_proj: [[f32; 4]; 4],
    pub dims: [f32; 2],
}
