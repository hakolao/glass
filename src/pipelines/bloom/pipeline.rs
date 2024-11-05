use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use wgpu::{
    util::DeviceExt, AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent,
    BlendFactor, BlendOperation, BlendState, Buffer, Color, ColorTargetState, ColorWrites,
    CommandEncoder, Device, Extent3d, FilterMode, LoadOp, Operations, PushConstantRange,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, Sampler, SamplerBindingType,
    SamplerDescriptor, ShaderStages, StoreOp, TextureFormat, TextureSampleType, TextureUsages,
    TextureViewDimension,
};

use crate::{
    pipelines::{SimpleTexturedVertex, FULL_SCREEN_TRIANGLE_VERTICES},
    texture::Texture,
};

const BLOOM_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rg11b10Ufloat;
const FINAL_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
const MAX_MIP_DIMENSION: u32 = 512;

fn create_bloom_texture(device: &Device, width: u32, height: u32, mip_count: u32) -> Texture {
    Texture::empty(
        device,
        "bloom_texture",
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_count,
        BLOOM_TEXTURE_FORMAT,
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
    )
}

pub struct BloomPipeline {
    downsample_first_pipeline: RenderPipeline,
    downsample_pipeline: RenderPipeline,
    upsample_pipeline: RenderPipeline,
    final_pipeline: RenderPipeline,
    bloom_texture: Texture,
    bloom_sampler: Sampler,
    downsampling_bind_groups: Vec<BindGroup>,
    upsampling_bind_groups: Vec<BindGroup>,
    vertices: Buffer,
    mip_count: u32,
    width: u32,
    height: u32,
    settings: BloomSettings,
}

impl BloomPipeline {
    pub fn new(
        device: &Device,
        bloom_settings: BloomSettings,
        width: u32,
        height: u32,
    ) -> BloomPipeline {
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Bloom Vertex Buffer"),
            contents: bytemuck::cast_slice(FULL_SCREEN_TRIANGLE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mip_count = MAX_MIP_DIMENSION.ilog2().max(2) - 1;
        let mip_height_ratio = MAX_MIP_DIMENSION as f32 / height as f32;

        let bloom_texture = create_bloom_texture(
            device,
            ((width as f32 * mip_height_ratio).round() as u32).max(1),
            ((height as f32 * mip_height_ratio).round() as u32).max(1),
            mip_count,
        );

        // Bind group layout
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("bloom_bind_group_layout"),
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
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Bloom Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("bloom.wgsl"))),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Bloom Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::FRAGMENT,
                range: 0..std::mem::size_of::<BloomPushConstants>() as u32,
            }],
        });
        let downsample_first_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Bloom Downsample First Pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[SimpleTexturedVertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("downsample_first"),
                    compilation_options: Default::default(),
                    targets: &[Some(ColorTargetState {
                        format: BLOOM_TEXTURE_FORMAT,
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
        let downsample_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Bloom Downsample Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[SimpleTexturedVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("downsample"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: BLOOM_TEXTURE_FORMAT,
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

        let color_blend = match bloom_settings.composite_mode {
            BloomCompositeMode::EnergyConserving => BlendComponent {
                src_factor: BlendFactor::Constant,
                dst_factor: BlendFactor::OneMinusConstant,
                operation: BlendOperation::Add,
            },
            BloomCompositeMode::Additive => BlendComponent {
                src_factor: BlendFactor::Constant,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        };

        let upsample_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Bloom Upsample Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[SimpleTexturedVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("upsample"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: BLOOM_TEXTURE_FORMAT,
                    blend: Some(BlendState {
                        color: color_blend,
                        alpha: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
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

        let final_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Bloom Final Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[SimpleTexturedVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("upsample"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: FINAL_TEXTURE_FORMAT,
                    blend: Some(BlendState {
                        color: color_blend,
                        alpha: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
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
        let bloom_sampler = device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });

        let (downsampling_bind_groups, upsampling_bind_groups) = Self::create_bind_groups(
            device,
            &downsample_pipeline,
            &upsample_pipeline,
            &bloom_texture,
            &bloom_sampler,
            mip_count,
        );

        BloomPipeline {
            downsample_first_pipeline,
            downsample_pipeline,
            upsample_pipeline,
            final_pipeline,
            bloom_texture,
            bloom_sampler,
            downsampling_bind_groups,
            upsampling_bind_groups,
            vertices,
            mip_count,
            width,
            height,
            settings: bloom_settings,
        }
    }

    fn create_bind_groups(
        device: &Device,
        downsample_pipeline: &RenderPipeline,
        upsample_pipeline: &RenderPipeline,
        bloom_texture: &Texture,
        bloom_sampler: &Sampler,
        mip_count: u32,
    ) -> (Vec<BindGroup>, Vec<BindGroup>) {
        let bind_group_count = mip_count as usize - 1;
        let mut downsampling_bind_groups = Vec::with_capacity(bind_group_count);
        for mip in 1..mip_count as usize {
            downsampling_bind_groups.push(device.create_bind_group(&BindGroupDescriptor {
                label: Some("bloom_downsampling_bind_group"),
                layout: &downsample_pipeline.get_bind_group_layout(0),
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&bloom_texture.views[mip - 1]),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(bloom_sampler),
                    },
                ],
            }));
        }

        let mut upsampling_bind_groups = Vec::with_capacity(bind_group_count);
        for mip in (0..mip_count as usize).rev() {
            upsampling_bind_groups.push(device.create_bind_group(&BindGroupDescriptor {
                label: Some("bloom_upsampling_bind_group"),
                layout: &upsample_pipeline.get_bind_group_layout(0),
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&bloom_texture.views[mip]),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(bloom_sampler),
                    },
                ],
            }));
        }

        (downsampling_bind_groups, upsampling_bind_groups)
    }

    pub fn configure(&mut self, device: &Device, settings: BloomSettings, width: u32, height: u32) {
        // Changes to these requires recreation of the pipeline
        let recreate_pipeline = settings.composite_mode != self.settings.composite_mode
            || width != self.width
            || height != self.height;
        if recreate_pipeline {
            // Limit dimensions to prevent texture max width error...
            *self = BloomPipeline::new(device, settings, width.max(256), height.max(256));
        } else {
            self.settings = settings;
        }
    }

    pub fn bloom(
        &self,
        device: &Device,
        encoder: &mut CommandEncoder,
        bloom_target: &Texture,
        viewport_origin: [u32; 2],
        viewport_size: [u32; 2],
    ) {
        let size = bloom_target.size;
        let push_constants =
            BloomPushConstants::new(&self.settings, viewport_origin, viewport_size, [
                size[0] as u32,
                size[1] as u32,
            ]);
        // First downsample pass (main image)
        let downsampling_first_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("bloom_downsampling_first_bind_group"),
            layout: &self.downsample_first_pipeline.get_bind_group_layout(0),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    // Read from input texture
                    resource: BindingResource::TextureView(&bloom_target.views[0]),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.bloom_sampler),
                },
            ],
        });
        {
            let view = &self.bloom_texture.views[0];
            let mut first_downsample_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("bloom_downsampling_first_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    // Write to bloom texture
                    view,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            first_downsample_pass.set_pipeline(&self.downsample_first_pipeline);
            first_downsample_pass.set_bind_group(0, &downsampling_first_bind_group, &[]);
            first_downsample_pass.set_vertex_buffer(0, self.vertices.slice(..));
            first_downsample_pass.set_push_constants(
                ShaderStages::FRAGMENT,
                0,
                bytemuck::cast_slice(&[push_constants]),
            );
            first_downsample_pass.draw(0..3, 0..1);
        }

        // Other Downsamples
        for mip in 1..self.mip_count as usize {
            // Write to next bloom texture, 1, 2, 3, 4...
            let view = &self.bloom_texture.views[mip];
            let mut downsampling_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("bloom_downsampling_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                    // ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            downsampling_pass.set_pipeline(&self.downsample_pipeline);
            downsampling_pass.set_bind_group(
                0,
                // Read from bloom previous bloom texture 0, 1, 2, 3... and so on
                &self.downsampling_bind_groups[mip - 1],
                &[],
            );
            downsampling_pass.set_vertex_buffer(0, self.vertices.slice(..));
            downsampling_pass.set_push_constants(
                ShaderStages::FRAGMENT,
                0,
                bytemuck::cast_slice(&[push_constants]),
            );
            downsampling_pass.draw(0..3, 0..1);
        }

        // Upsample
        for mip in (1..self.mip_count as usize).rev() {
            // Write to next (larger) bloom texture, inverse order, 7, 6, 5...0
            let view = &self.bloom_texture.views[mip - 1];
            let mut upsampling_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("bloom_upsampling_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            upsampling_pass.set_pipeline(&self.upsample_pipeline);
            upsampling_pass.set_bind_group(
                0,
                // Read from bloom texture 0, 1, 2, 3... and so on
                &self.upsampling_bind_groups[self.mip_count as usize - mip - 1],
                &[],
            );
            upsampling_pass.set_vertex_buffer(0, self.vertices.slice(..));
            let blend =
                compute_blend_factor(&self.settings, mip as f32, (self.mip_count - 1) as f32);
            upsampling_pass.set_blend_constant(Color {
                r: blend as f64,
                g: blend as f64,
                b: blend as f64,
                a: 1.0,
            });
            upsampling_pass.set_push_constants(
                ShaderStages::FRAGMENT,
                0,
                bytemuck::cast_slice(&[push_constants]),
            );
            upsampling_pass.draw(0..3, 0..1);
        }

        // Final upsample pass
        {
            let mut upsampling_final_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("bloom_upsampling_final_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &bloom_target.views[0],
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            upsampling_final_pass.set_pipeline(&self.final_pipeline);
            upsampling_final_pass.set_bind_group(
                0,
                &self.upsampling_bind_groups[(self.mip_count - 1) as usize],
                &[],
            );
            upsampling_final_pass.set_vertex_buffer(0, self.vertices.slice(..));
            upsampling_final_pass.set_viewport(
                viewport_origin[0] as f32,
                viewport_origin[1] as f32,
                viewport_size[0] as f32,
                viewport_size[1] as f32,
                0.0,
                1.0,
            );
            let blend = compute_blend_factor(&self.settings, 0.0, (self.mip_count - 1) as f32);
            upsampling_final_pass.set_blend_constant(Color {
                r: blend as f64,
                g: blend as f64,
                b: blend as f64,
                a: 1.0,
            });
            upsampling_final_pass.set_push_constants(
                ShaderStages::FRAGMENT,
                0,
                bytemuck::cast_slice(&[push_constants]),
            );
            upsampling_final_pass.draw(0..3, 0..1);
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct BloomPushConstants {
    pub threshold_precomputations: [f32; 4],
    pub viewport: [f32; 4],
    pub aspect: f32,
    pub use_treshold: u32,
}

impl BloomPushConstants {
    pub fn new(
        settings: &BloomSettings,
        viewport_origin: [u32; 2],
        viewport_size: [u32; 2],
        target_image_size: [u32; 2],
    ) -> BloomPushConstants {
        let threshold = settings.prefilter_settings.threshold;
        let threshold_softness = settings.prefilter_settings.threshold_softness;
        let knee = threshold * threshold_softness.clamp(0.0, 1.0);
        BloomPushConstants {
            threshold_precomputations: [
                threshold,
                threshold - knee,
                2.0 * knee,
                0.25 / (knee + 0.00001),
            ],
            viewport: [
                viewport_origin[0] as f32 / target_image_size[0] as f32,
                viewport_origin[1] as f32 / target_image_size[1] as f32,
                viewport_size[0] as f32 / target_image_size[0] as f32,
                viewport_size[1] as f32 / target_image_size[1] as f32,
            ],
            aspect: viewport_size[0] as f32 / viewport_size[1] as f32,
            use_treshold: (threshold > 0.0) as u32,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BloomSettings {
    pub intensity: f32,
    pub low_frequency_boost: f32,
    pub low_frequency_boost_curvature: f32,
    pub high_pass_frequency: f32,
    pub prefilter_settings: BloomPrefilterSettings,
    pub composite_mode: BloomCompositeMode,
}

impl BloomSettings {
    /// The default bloom preset.
    pub const NATURAL: Self = Self {
        intensity: 0.15,
        low_frequency_boost: 0.7,
        low_frequency_boost_curvature: 0.95,
        high_pass_frequency: 1.0,
        prefilter_settings: BloomPrefilterSettings {
            threshold: 0.0,
            threshold_softness: 0.0,
        },
        composite_mode: BloomCompositeMode::EnergyConserving,
    };
    /// A preset that's similiar to how older games did bloom.
    pub const OLD_SCHOOL: Self = Self {
        intensity: 0.05,
        low_frequency_boost: 0.7,
        low_frequency_boost_curvature: 0.95,
        high_pass_frequency: 1.0,
        prefilter_settings: BloomPrefilterSettings {
            threshold: 0.6,
            threshold_softness: 0.2,
        },
        composite_mode: BloomCompositeMode::Additive,
    };
    /// A preset that applies a very strong bloom, and blurs the whole screen.
    pub const SCREEN_BLUR: Self = Self {
        intensity: 1.0,
        low_frequency_boost: 0.0,
        low_frequency_boost_curvature: 0.0,
        high_pass_frequency: 1.0 / 3.0,
        prefilter_settings: BloomPrefilterSettings {
            threshold: 0.0,
            threshold_softness: 0.0,
        },
        composite_mode: BloomCompositeMode::EnergyConserving,
    };
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self::NATURAL
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BloomPrefilterSettings {
    pub threshold: f32,
    pub threshold_softness: f32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BloomCompositeMode {
    EnergyConserving,
    Additive,
}

fn compute_blend_factor(bloom_settings: &BloomSettings, mip: f32, max_mip: f32) -> f32 {
    let mut lf_boost = (1.0
        - (1.0 - (mip / max_mip)).powf(1.0 / (1.0 - bloom_settings.low_frequency_boost_curvature)))
        * bloom_settings.low_frequency_boost;
    let high_pass_lq = 1.0
        - (((mip / max_mip) - bloom_settings.high_pass_frequency)
            / bloom_settings.high_pass_frequency)
            .clamp(0.0, 1.0);
    lf_boost *= match bloom_settings.composite_mode {
        BloomCompositeMode::EnergyConserving => 1.0 - bloom_settings.intensity,
        BloomCompositeMode::Additive => 1.0,
    };

    (bloom_settings.intensity + lf_boost) * high_pass_lq
}
