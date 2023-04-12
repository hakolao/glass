use anyhow::*;
use image::DynamicImage;
use wgpu::{
    Device, Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d, Queue, Sampler,
    SamplerDescriptor, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor,
};

/// A utility struct to ease Gpu texture creation from image data
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: TextureView,
    pub sampler: Sampler,
    pub size: [f32; 2],
}

impl Texture {
    pub fn empty(
        device: &Device,
        label: &str,
        size: Extent3d,
        format: TextureFormat,
        sampler_descriptor: &SamplerDescriptor,
        usage: TextureUsages,
    ) -> Result<Self> {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            view_formats: &[],
            format,
            usage,
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(sampler_descriptor);

        Ok(Self {
            texture,
            view,
            sampler,
            size: [size.width as f32, size.height as f32],
        })
    }

    pub fn from_bytes(
        device: &Device,
        queue: &Queue,
        bytes: &[u8],
        label: &str,
        format: TextureFormat,
        sampler_descriptor: &SamplerDescriptor,
        usage: TextureUsages,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(
            device,
            queue,
            &img,
            label,
            format,
            sampler_descriptor,
            usage,
        )
    }

    pub fn from_image(
        device: &Device,
        queue: &Queue,
        img: &DynamicImage,
        label: &str,
        format: TextureFormat,
        sampler_descriptor: &SamplerDescriptor,
        usage: TextureUsages,
    ) -> Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = rgba.dimensions();

        let size = Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            view_formats: &[],
            format,
            usage,
        });

        queue.write_texture(
            ImageCopyTexture {
                aspect: TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            &rgba,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
                rows_per_image: None,
            },
            size,
        );

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(sampler_descriptor);

        Ok(Self {
            texture,
            view,
            sampler,
            size: [dimensions.0 as f32, dimensions.1 as f32],
        })
    }
}
