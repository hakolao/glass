mod quad;
mod vertex;

use std::collections::HashMap;

pub use quad::QuadPipeline;
pub use vertex::*;
use wgpu::{ComputePipeline, Device, RenderPipeline, TextureFormat};

pub struct CommonPipelines {
    pub quad: QuadPipeline,
}

impl CommonPipelines {
    /// Creates common pipelines
    pub fn new(device: &Device, target_surface_format: TextureFormat) -> CommonPipelines {
        let quad = QuadPipeline::new(device, target_surface_format);
        CommonPipelines {
            quad,
        }
    }
}

/// A utility struct to help organize render and compute pipelines
#[derive(Default)]
pub struct Pipelines {
    draw_pipelines: HashMap<PipelineKey, RenderPipeline>,
    compute_pipelines: HashMap<PipelineKey, ComputePipeline>,
}

impl Pipelines {
    pub fn draw_pipeline(&self, key: &PipelineKey) -> Option<&RenderPipeline> {
        self.draw_pipelines.get(key)
    }

    pub fn compute_pipeline(&self, key: &PipelineKey) -> Option<&ComputePipeline> {
        self.compute_pipelines.get(key)
    }

    pub fn add_draw_pipeline(&mut self, pipeline_key: PipelineKey, pipeline: RenderPipeline) {
        self.draw_pipelines.insert(pipeline_key, pipeline);
    }

    pub fn add_compute_pipeline(&mut self, pipeline_key: PipelineKey, pipeline: ComputePipeline) {
        self.compute_pipelines.insert(pipeline_key, pipeline);
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct PipelineKey {
    pub name: &'static str,
}

impl PipelineKey {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
        }
    }
}
