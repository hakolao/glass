use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    Buffer, ColorTargetState, ColorWrites, CommandEncoder, Device, Operations, PushConstantRange,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, Sampler, SamplerBindingType,
    ShaderStages, TextureFormat, TextureSampleType, TextureView, TextureViewDimension,
};

use crate::{
    pipelines::{SimpleTexturedVertex, FULL_SCREEN_TRIANGLE_VERTICES},
    texture::Texture,
};

const TONEMAPPING_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

pub struct TonemappingPipeline {
    tonemapping_pipeline: RenderPipeline,
    vertices: Buffer,
}

impl TonemappingPipeline {
    pub fn new(device: &Device) -> TonemappingPipeline {
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tonemapping Vertex Buffer"),
            contents: bytemuck::cast_slice(FULL_SCREEN_TRIANGLE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        // Bind group layout
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("tonemapping_bind_group_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float {
                            filterable: false,
                        },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    visibility: ShaderStages::FRAGMENT,
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    visibility: ShaderStages::FRAGMENT,
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Tonemapping Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("tonemapping.wgsl"))),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Tonemapping Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::FRAGMENT,
                range: 0..std::mem::size_of::<ToneMappingPushConstants>() as u32,
            }],
        });
        let tonemapping_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Tonemapping Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[SimpleTexturedVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: TONEMAPPING_TEXTURE_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        TonemappingPipeline {
            tonemapping_pipeline,
            vertices,
        }
    }

    pub fn create_bind_group(
        &self,
        device: &Device,
        image: &TextureView,
        sampler: &Sampler,
    ) -> BindGroup {
        let bind_group_layout = self.tonemapping_pipeline.get_bind_group_layout(0);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(image),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
            label: Some("tonemap_bind_group"),
        });
        bind_group
    }

    pub fn tonemap(
        &self,
        encoder: &mut CommandEncoder,
        input_image_bind_group: &BindGroup,
        output: &Texture,
        color_grading: ColorGrading,
    ) {
        let push_constants: ToneMappingPushConstants = color_grading.into();
        {
            let mut r_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("tonemapping_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &output.views[0],
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            r_pass.set_pipeline(&self.tonemapping_pipeline);
            r_pass.set_bind_group(0, input_image_bind_group, &[]);
            r_pass.set_vertex_buffer(0, self.vertices.slice(..));
            r_pass.set_push_constants(
                ShaderStages::FRAGMENT,
                0,
                bytemuck::cast_slice(&[push_constants]),
            );
            r_pass.draw(0..3, 0..1);
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ToneMappingPushConstants {
    pub off: u32,
    pub exposure: f32,
    pub gamma: f32,
    pub pre_saturation: f32,
    pub post_saturation: f32,
}

impl From<ColorGrading> for ToneMappingPushConstants {
    fn from(val: ColorGrading) -> Self {
        ToneMappingPushConstants {
            off: val.off as u32,
            exposure: val.exposure,
            gamma: val.gamma,
            pre_saturation: val.pre_saturation,
            post_saturation: val.post_saturation,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ColorGrading {
    pub off: bool,
    pub exposure: f32,
    pub gamma: f32,
    pub pre_saturation: f32,
    pub post_saturation: f32,
}

impl Default for ColorGrading {
    fn default() -> Self {
        Self {
            off: false,
            exposure: 0.0,
            gamma: 1.0,
            pre_saturation: 1.0,
            post_saturation: 1.0,
        }
    }
}
