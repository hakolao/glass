//! GPU textures created from image data or from nothing.

use image::DynamicImage;
use wgpu::{
    Device, Extent3d, Origin3d, Queue, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

use crate::GlassError;

/// A utility struct to ease Gpu texture creation from image data
#[derive(Debug)]
pub struct Texture {
    /// The underlying wgpu texture.
    pub texture: wgpu::Texture,
    /// One view per mip level, in order, so `views[0]` is the full-resolution view.
    pub views: Vec<TextureView>,
    /// Width and height in pixels, as floats because that is what shaders and cameras want.
    pub size: [f32; 2],
}

/// Shared parameters for the [`Texture`] constructors.
#[derive(Debug, Clone)]
pub struct TextureDesc<'a> {
    /// Debug label, shown in graphics debuggers and wgpu error messages.
    pub label: &'a str,
    /// Texel format. [`default_texture_format`](crate::utils::default_texture_format) is a
    /// reasonable choice for a texture you intend to blit to a window.
    pub format: TextureFormat,
    /// How the texture may be used. Must include every usage you actually perform.
    pub usage: TextureUsages,
    /// Number of mip levels, and therefore the number of views created. Use `1` for no mips.
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
        desc: &TextureDesc<'_>,
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

    /// Creates an empty 2D texture with one `D2` view per mip level. Nothing is written to it,
    /// so its contents are undefined until you render or copy into it.
    pub fn empty(device: &Device, size: Extent3d, desc: &TextureDesc<'_>) -> Self {
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

    /// Decodes `bytes` as an image and uploads it. Only PNG and JPEG are supported, because
    /// those are the only decoders `glass` enables in `image`.
    ///
    /// ```no_run
    /// use glass::prelude::*;
    /// use glass::{utils::default_texture_format, wgpu::TextureUsages};
    ///
    /// # fn load(context: &GlassContext) -> Result<Texture, GlassError> {
    /// let tree = Texture::from_bytes(
    ///     context.device(),
    ///     context.queue(),
    ///     include_bytes!("../examples/quad/tree.png"),
    ///     &TextureDesc {
    ///         label: "tree",
    ///         format: default_texture_format(),
    ///         usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
    ///         mip_count: 1,
    ///     },
    /// )?;
    /// # Ok(tree)
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Fails with [`GlassError::ImageError`] if `bytes` is not a supported image.
    pub fn from_bytes(
        device: &Device,
        queue: &Queue,
        bytes: &[u8],
        desc: &TextureDesc<'_>,
    ) -> Result<Self, GlassError> {
        let img = image::load_from_memory(bytes)?;
        Ok(Self::from_image(device, queue, &img, desc))
    }

    /// Uploads an already-decoded image, converting it to RGBA8 first.
    ///
    /// Only mip level 0 is written even when `desc.mip_count` is greater than one; generate the
    /// remaining levels yourself.
    pub fn from_image(
        device: &Device,
        queue: &Queue,
        img: &DynamicImage,
        desc: &TextureDesc<'_>,
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
