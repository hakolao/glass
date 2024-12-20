use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    Buffer, Color, ColorTargetState, ColorWrites, CommandEncoder, Device, Operations,
    PushConstantRange, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, Sampler,
    SamplerBindingType, ShaderStages, TextureFormat, TextureSampleType, TextureView,
    TextureViewDimension,
};

use crate::{
    pipelines::{TexturedVertex, QUAD_INDICES, TEXTURED_QUAD_VERTICES},
    texture::Texture,
};

pub struct PastePipeline {
    paste_pipeline: RenderPipeline,
    vertices: Buffer,
    indices: Buffer,
}

impl PastePipeline {
    pub fn new(
        device: &Device,
        target_texture_format: TextureFormat,
        is_nearest: bool,
    ) -> PastePipeline {
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Paste Vertex Buffer"),
            contents: bytemuck::cast_slice(
                &TEXTURED_QUAD_VERTICES
                    .iter()
                    .map(|v| TexturedVertex {
                        position: [
                            v.position[0] * 2.0,
                            v.position[1] * 2.0,
                            v.position[2],
                            v.position[3],
                        ],
                        ..*v
                    })
                    .collect::<Vec<TexturedVertex>>(),
            ),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Paste Index Buffer"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        // Bind group layout
        let bind_group_layout = if is_nearest {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
            })
        } else {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("paste_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float {
                                filterable: true,
                            },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        visibility: ShaderStages::FRAGMENT,
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        visibility: ShaderStages::FRAGMENT,
                        count: None,
                    },
                ],
            })
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Paste Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("paste.wgsl"))),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Paste Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..std::mem::size_of::<PastePushConstants>() as u32,
            }],
        });
        let paste_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Paste Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[TexturedVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: target_texture_format,
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
            cache: None,
        });

        PastePipeline {
            paste_pipeline,
            vertices,
            indices,
        }
    }

    pub fn create_input_bind_group(
        &self,
        device: &Device,
        image: &TextureView,
        sampler: &Sampler,
    ) -> BindGroup {
        let bind_group_layout = self.paste_pipeline.get_bind_group_layout(0);
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
            label: Some("paste_bind_group"),
        });
        bind_group
    }

    #[allow(clippy::too_many_arguments)]
    pub fn paste(
        &self,
        encoder: &mut CommandEncoder,
        ops: Operations<Color>,
        input_image_bind_group: &BindGroup,
        output: &Texture,
        tint: [f32; 4],
        size: [f32; 2],
        offset: [f32; 2],
        flip_x: bool,
        flip_y: bool,
    ) {
        let image_size = [size[0] / output.size[0], size[1] / output.size[1]];
        let push_constants: PastePushConstants = PastePushConstants {
            tint,
            scale: [
                image_size[0] * if flip_x { -1.0 } else { 1.0 },
                image_size[1] * if flip_y { -1.0 } else { 1.0 },
            ],
            offset: [
                (2.0 * offset[0] - output.size[0]) / output.size[0],
                -(2.0 * offset[1] - output.size[1]) / output.size[1],
            ],
        };
        {
            let mut r_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("paste_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &output.views[0],
                    resolve_target: None,
                    ops,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            r_pass.set_pipeline(&self.paste_pipeline);
            r_pass.set_bind_group(0, input_image_bind_group, &[]);
            r_pass.set_vertex_buffer(0, self.vertices.slice(..));
            r_pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
            r_pass.set_push_constants(
                ShaderStages::VERTEX_FRAGMENT,
                0,
                bytemuck::cast_slice(&[push_constants]),
            );
            r_pass.draw_indexed(0..(QUAD_INDICES.len() as u32), 0, 0..1);
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PastePushConstants {
    tint: [f32; 4],
    scale: [f32; 2],
    offset: [f32; 2],
}
