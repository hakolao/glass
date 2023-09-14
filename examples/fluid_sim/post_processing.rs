use glam::UVec2;
use glass::{
    pipelines::{BloomPipeline, BloomSettings, ColorGrading, TonemappingPipeline},
    texture::Texture,
    wgpu::CommandEncoder,
    GlassContext,
};

use crate::app::{create_render_target, HEIGHT, WIDTH};

pub struct PostProcessing {
    bloom_pipeline: BloomPipeline,
    tonemap_pipeline: TonemappingPipeline,
    post_processed_image: Texture,
}

impl PostProcessing {
    pub fn new(context: &GlassContext) -> PostProcessing {
        let post_processed_image = create_render_target(context);
        PostProcessing {
            bloom_pipeline: BloomPipeline::new(
                context.device(),
                BloomSettings::default(),
                WIDTH,
                HEIGHT,
            ),
            tonemap_pipeline: TonemappingPipeline::new(context.device()),
            post_processed_image,
        }
    }

    pub fn output(&self) -> &Texture {
        &self.post_processed_image
    }

    pub fn run(&self, context: &GlassContext, encoder: &mut CommandEncoder, input_image: &Texture) {
        self.bloom_pipeline.bloom(
            context.device(),
            encoder,
            input_image,
            UVec2::new(0, 0),
            UVec2::new(WIDTH, HEIGHT),
        );

        self.tonemap_pipeline.tonemap(
            context.device(),
            encoder,
            input_image,
            &self.post_processed_image,
            ColorGrading {
                off: false,
                exposure: 1.0,
                gamma: 1.0,
                pre_saturation: 1.05,
                post_saturation: 1.05,
            },
        );
    }
}
