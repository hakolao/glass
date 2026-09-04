//! A pipeline that draws a textured, optionally anti-aliased quad.

use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use wgpu::{
    util::DeviceExt, BindGroup, Buffer, Device, RenderPass, RenderPipeline, Sampler, TextureView,
};

use super::super::{vertex::TexturedVertex, QUAD_INDICES, TEXTURED_QUAD_VERTICES};

/// Draws a textured quad, sized and positioned per draw call through immediate data.
///
/// The quad geometry lives in this pipeline, so the only per-draw cost is the immediate
/// data and a bind group holding your texture and sampler.
///
/// Build the pipeline once, for the format of the attachment you draw into:
///
/// ```no_run
/// # use common::pipelines::QuadPipeline;
/// use glass::wgpu::{BindGroup, ColorTargetState, Device, RenderPass, Sampler, TextureView};
///
/// # fn setup(device: &Device, target: ColorTargetState) -> QuadPipeline {
/// QuadPipeline::new(device, target)
/// # }
/// ```
///
/// Then bind a texture once, and draw it as often as you like:
///
/// ```no_run
/// # use common::pipelines::QuadPipeline;
/// # use glass::wgpu::{BindGroup, Device, RenderPass, Sampler, TextureView};
/// # fn bind(pipeline: &QuadPipeline, device: &Device, view: &TextureView, sampler: &Sampler)
/// #     -> BindGroup {
/// pipeline.create_bind_group(device, view, sampler)
/// # }
///
/// # fn draw<'r>(
/// #     pipeline: &'r QuadPipeline,
/// #     rpass: &mut RenderPass<'r>,
/// #     bind_group: &'r BindGroup,
/// #     view_proj: [[f32; 4]; 4],
/// # ) {
/// pipeline.draw(
///     rpass,
///     bind_group,
///     [0.0, 0.0, 0.0, 1.0], // centred on the origin
///     view_proj,
///     [256.0, 256.0],       // 256x256 world units
///     0.0,                  // hard edges
/// );
/// # }
/// ```
#[derive(Debug)]
pub struct QuadPipeline {
    pipeline: RenderPipeline,
    vertices: Buffer,
    indices: Buffer,
}

impl QuadPipeline {
    /// Builds the pipeline and its quad geometry for a render target described by
    /// `color_target_state`, which must match the format of the attachment you draw into.
    pub fn new(device: &Device, color_target_state: wgpu::ColorTargetState) -> QuadPipeline {
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(TEXTURED_QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let pipeline = Self::new_render_pipeline(device, color_target_state);
        Self {
            pipeline,
            vertices,
            indices,
        }
    }

    /// The underlying [`RenderPipeline`], for apps that manage their own quad geometry.
    pub fn new_render_pipeline(
        device: &Device,
        color_target_state: wgpu::ColorTargetState,
    ) -> RenderPipeline {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("quad.wgsl"))),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Quad Pipeline Layout"),
            bind_group_layouts: &[Some(&texture_bind_group_layout)],
            immediate_size: size_of::<QuadPushConstants>() as u32,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Quad Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[TexturedVertex::desc().into()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(color_target_state)],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                // No cull mode to enable flipping of the quad
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
            cache: None,
            multiview_mask: None,
        });
        pipeline
    }

    /// Binds `image` and `sampler` for use with the draw functions. Create this once per
    /// texture rather than once per frame.
    pub fn create_bind_group(
        &self,
        device: &Device,
        image: &TextureView,
        sampler: &Sampler,
    ) -> BindGroup {
        let bind_group_layout = self.pipeline.get_bind_group_layout(0);
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
            label: Some("bind_group"),
        });
        bind_group
    }

    /// Draws the quad at `quad_pos` with `quad_size`, sampling the whole texture.
    ///
    /// `aa_strength` softens the quad edges in pixels; pass `0.0` for hard edges.
    pub fn draw<'r>(
        &'r self,
        rpass: &mut RenderPass<'r>,
        bind_group: &'r BindGroup,
        quad_pos: [f32; 4],
        view_proj: [[f32; 4]; 4],
        quad_size: [f32; 2],
        aa_strength: f32,
    ) {
        self.draw_inner(
            rpass,
            bind_group,
            quad_pos,
            view_proj,
            quad_size,
            [0.0; 2],
            [1.0, 1.0],
            aa_strength,
        );
    }

    /// [`QuadPipeline::draw`], but sampling only the sub-rectangle of the texture given by
    /// `uv_offset` and `uv_scale`. Use this to draw one sprite out of an atlas.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_with_uv<'r>(
        &'r self,
        rpass: &mut RenderPass<'r>,
        bind_group: &'r BindGroup,
        quad_pos: [f32; 4],
        view_proj: [[f32; 4]; 4],
        quad_size: [f32; 2],
        uv_offset: [f32; 2],
        uv_scale: [f32; 2],
        aa_strength: f32,
    ) {
        self.draw_inner(
            rpass,
            bind_group,
            quad_pos,
            view_proj,
            quad_size,
            uv_offset,
            uv_scale,
            aa_strength,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_inner<'r>(
        &'r self,
        rpass: &mut RenderPass<'r>,
        bind_group: &'r BindGroup,
        quad_pos: [f32; 4],
        view_proj: [[f32; 4]; 4],
        quad_size: [f32; 2],
        uv_offset: [f32; 2],
        uv_scale: [f32; 2],
        aa_strength: f32,
    ) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertices.slice(..));
        rpass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
        rpass.set_immediates(
            0,
            bytemuck::cast_slice(&[QuadPipeline::push_constants(
                quad_pos,
                view_proj,
                quad_size,
                uv_offset,
                uv_scale,
                aa_strength,
            )]),
        );
        rpass.draw_indexed(0..(QUAD_INDICES.len() as u32), 0, 0..1);
    }

    fn push_constants(
        quad_pos: [f32; 4],
        view_proj: [[f32; 4]; 4],
        quad_size: [f32; 2],
        uv_offset: [f32; 2],
        uv_scale: [f32; 2],
        aa_strength: f32,
    ) -> QuadPushConstants {
        QuadPushConstants {
            quad_pos,
            view_proj,
            dims: quad_size,
            uv_offset,
            uv_scale,
            aa_strength,
        }
    }
}

/// Quad instance specific values passed to the shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct QuadPushConstants {
    /// Centre of the quad, in world space.
    pub quad_pos: [f32; 4],
    /// Combined view and projection matrix.
    pub view_proj: [[f32; 4]; 4],
    /// Width and height of the quad, in world units.
    pub dims: [f32; 2],
    /// Offset into the texture, in UV space.
    pub uv_offset: [f32; 2],
    /// Portion of the texture to sample, in UV space.
    pub uv_scale: [f32; 2],
    /// Edge softening, in pixels. `0.0` gives hard edges.
    pub aa_strength: f32,
}
