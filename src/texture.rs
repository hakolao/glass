use image::DynamicImage;
use wgpu::{
    Device, Extent3d, Origin3d, Queue, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect,
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

/// Shared parameters for the [`Texture`] constructors.
pub struct TextureDesc<'a> {
    pub label: &'a str,
    pub format: TextureFormat,
    pub usage: TextureUsages,
    pub mip_count: u32,
}

impl Texture {
    /// Creates an empty 2D **array** texture. The view is always `D2Array`,
    /// even for a single layer, so it binds correctly to `texture_2d_array`.
    pub fn empty_array(
        device: &Device,
        width: u32,
        height: u32,
        layers: u32,
        desc: &TextureDesc,
    ) -> Self {
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: layers,
        };
        let texture = device.create_texture(&TextureDescriptor {
            label: Some(desc.label),
            size,
            mip_level_count: desc.mip_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            view_formats: &[],
            format: desc.format,
            usage: desc.usage,
        });
        let mut views = vec![];
        for mip_level in 0..desc.mip_count {
            let view = texture.create_view(&TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                base_mip_level: mip_level,
                mip_level_count: Some(1),
                ..Default::default()
            });
            views.push(view);
        }

        Self {
            texture,
            views,
            size: [width as f32, height as f32],
        }
    }

    pub fn empty(device: &Device, size: Extent3d, desc: &TextureDesc) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some(desc.label),
            size,
            mip_level_count: desc.mip_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            view_formats: &[],
            format: desc.format,
            usage: desc.usage,
        });
        let mut views = vec![];
        for mip_level in 0..desc.mip_count {
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
        desc: &TextureDesc,
    ) -> Result<Self, GlassError> {
        let img = image::load_from_memory(bytes)?;
        Ok(Self::from_image(device, queue, &img, desc))
    }

    pub fn from_image(
        device: &Device,
        queue: &Queue,
        img: &DynamicImage,
        desc: &TextureDesc,
    ) -> Self {
        let rgba = img.to_rgba8();
        let dimensions = rgba.dimensions();

        let size = Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&TextureDescriptor {
            label: Some(desc.label),
            size,
            mip_level_count: desc.mip_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            view_formats: &[],
            format: desc.format,
            usage: desc.usage,
        });

        queue.write_texture(
            TexelCopyTextureInfo {
                aspect: TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            &rgba,
            TexelCopyBufferLayout {
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
