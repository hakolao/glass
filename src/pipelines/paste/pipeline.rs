use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use wgpu::{
    util::DeviceExt, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, ColorTargetState, ColorWrites,
    CommandEncoder, Device, LoadOp, Operations, PushConstantRange, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, SamplerBindingType, ShaderStages, TextureFormat,
    TextureSampleType, TextureViewDimension,
};

use crate::{
    pipelines::{SimpleVertex, FULL_SCREEN_TRIANGLE_VERTICES},
    texture::Texture,
};

const PASTE_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

pub struct PastePipeline {
    paste_pipeline: RenderPipeline,
    vertices: Buffer,
}

impl PastePipeline {
    pub fn new(device: &Device) -> PastePipeline {
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Paste Vertex Buffer"),
            contents: bytemuck::cast_slice(FULL_SCREEN_TRIANGLE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        // Bind group layout
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("paste_bind_group_layout"),
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
            label: Some("Paste Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("paste.wgsl"))),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Paste Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..std::mem::size_of::<PastePushConstants>() as u32,
            }],
        });
        let paste_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Paste Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[SimpleVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fragment",
                targets: &[Some(ColorTargetState {
                    format: PASTE_TEXTURE_FORMAT,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::OVER,
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        PastePipeline {
            paste_pipeline,
            vertices,
        }
    }

    pub fn paste(
        &self,
        device: &Device,
        encoder: &mut CommandEncoder,
        input: &Texture,
        output: &Texture,
        size: Vec2,
        offset: Vec2,
        flip_x: bool,
        flip_y: bool,
    ) {
        let push_constants: PastePushConstants = PastePushConstants {
            scale: [
                size.x / output.size[0] * if flip_x { -1.0 } else { 1.0 },
                size.y / output.size[1] * if flip_y { -1.0 } else { 1.0 },
            ],
            offset: [offset.x / output.size[0], -offset.y / output.size[1]],
        };
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("paste_bind_group"),
            layout: &self.paste_pipeline.get_bind_group_layout(0),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&input.views[0]),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&input.sampler),
                },
            ],
        });
        {
            let mut r_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("paste_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &output.views[0],
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        ..Default::default()
                    },
                })],
                depth_stencil_attachment: None,
            });
            r_pass.set_pipeline(&self.paste_pipeline);
            r_pass.set_bind_group(0, &bind_group, &[]);
            r_pass.set_vertex_buffer(0, self.vertices.slice(..));
            r_pass.set_push_constants(
                ShaderStages::VERTEX,
                0,
                bytemuck::cast_slice(&[push_constants]),
            );
            r_pass.draw(0..3, 0..1);
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PastePushConstants {
    scale: [f32; 2],
    offset: [f32; 2],
}
