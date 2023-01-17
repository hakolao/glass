use std::{collections::HashMap, future::Future};

use wgpu::{ComputePipeline, RenderPipeline};

pub fn wait_async<F: Future>(fut: F) -> F::Output {
    pollster::block_on(fut)
}

/// A utility struct to help organize render and compute pipelines
#[derive(Default)]
pub struct Pipelines {
    draw_pipelines: HashMap<PipelineKey, DrawPipeline>,
    compute_pipelines: HashMap<PipelineKey, CalcPipeline>,
}

impl Pipelines {
    pub fn draw_pipeline(&self, key: &PipelineKey) -> Option<&DrawPipeline> {
        self.draw_pipelines.get(key)
    }

    pub fn compute_pipeline(&self, key: &PipelineKey) -> Option<&CalcPipeline> {
        self.compute_pipelines.get(key)
    }

    pub fn add_draw_pipeline(&mut self, pipeline_key: PipelineKey, pipeline: RenderPipeline) {
        self.draw_pipelines.insert(pipeline_key, DrawPipeline {
            pipeline,
        });
    }

    pub fn add_compute_pipeline(&mut self, pipeline_key: PipelineKey, pipeline: ComputePipeline) {
        self.compute_pipelines.insert(pipeline_key, CalcPipeline {
            pipeline,
        });
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

#[derive(Debug)]
pub struct DrawPipeline {
    pub pipeline: RenderPipeline,
}

#[derive(Debug)]
pub struct CalcPipeline {
    pub pipeline: ComputePipeline,
}
