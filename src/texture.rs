use image::DynamicImage;
use wgpu::{
    Device, Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d, Queue, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

use crate::GlassError;

/// A utility struct to ease Gpu texture creation from image data
pub struct Texture {
    pub texture: wgpu::Texture,
    pub views: Vec<TextureView>,
    pub size: [f32; 2],
}

impl Texture {
    pub fn empty(
        device: &Device,
        label: &str,
        size: Extent3d,
        mip_count: u32,
        format: TextureFormat,
        usage: TextureUsages,
    ) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            view_formats: &[],
            format,
            usage,
        });
        let mut views = vec![];
        for mip_level in 0..mip_count {
            let view = texture.create_view(&TextureViewDescriptor {
                base_mip_level: mip_level,
                mip_level_count: Some(1),
                ..Default::default()
            });
            views.push(view);
        }

        Self {
            texture,
            views,
            size: [size.width as f32, size.height as f32],
        }
    }

    pub fn from_bytes(
        device: &Device,
        queue: &Queue,
        bytes: &[u8],
        label: &str,
        format: TextureFormat,
        usage: TextureUsages,
    ) -> Result<Self, GlassError> {
        let img = match image::load_from_memory(bytes) {
            Ok(im) => im,
            Err(e) => return Err(GlassError::ImageError(e)),
        };
        Ok(Self::from_image(
            device, queue, &img, label, format, usage, 1,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_image(
        device: &Device,
        queue: &Queue,
        img: &DynamicImage,
        label: &str,
        format: TextureFormat,
        usage: TextureUsages,
        mip_count: u32,
    ) -> Self {
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
            mip_level_count: mip_count,
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
                rows_per_image: None,
                bytes_per_row: Some(4 * dimensions.0),
            },
            size,
        );

        let view = texture.create_view(&TextureViewDescriptor::default());

        Self {
            texture,
            views: vec![view],
            size: [dimensions.0 as f32, dimensions.1 as f32],
        }
    }
}
