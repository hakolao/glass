use std::future::Future;

use wgpu::TextureFormat;

pub fn wait_async<F: Future>(fut: F) -> F::Output {
    pollster::block_on(fut)
}

/// Return default [`TextureFormat`](wgpu::TextureFormat)s
pub fn default_texture_format() -> TextureFormat {
    #[cfg(target_os = "linux")]
    {
        if std::env::var("WAYLAND_DISPLAY").is_ok()
            && std::env::var("WAYLAND_DISPLAY").unwrap() != ""
        {
            return TextureFormat::Rgba8Unorm;
        }
    }

    // Default to sRGB for all other cases (X11, Windows, macOS, etc.)
    TextureFormat::Rgba8UnormSrgb
}
