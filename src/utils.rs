//! Small helpers that do not belong to any one type.

use std::future::Future;

use wgpu::TextureFormat;

/// Blocks the current thread until `fut` completes.
///
/// `glass` is a synchronous API over an asynchronous wgpu, and this is the one place that bridge
/// happens. Do not call it from inside an async runtime.
pub fn wait_async<F: Future>(fut: F) -> F::Output {
    pollster::block_on(fut)
}

/// A sensible default format for offscreen textures you render into and then blit to a window.
///
/// This is `Rgba8UnormSrgb`, except on a Wayland session where it is the non-sRGB `Rgba8Unorm`,
/// matching the sRGB-ness of [`default_surface_format`] so that a blit between the two does not
/// apply an unwanted conversion. Textures that never reach a surface can use any format you like.
pub fn default_texture_format() -> TextureFormat {
    if is_wayland_session() {
        TextureFormat::Rgba8Unorm
    } else {
        TextureFormat::Rgba8UnormSrgb
    }
}

/// The surface format that works on the current platform.
///
/// This is `Bgra8UnormSrgb`, except on a Wayland session where it is the non-sRGB `Bgra8Unorm`,
/// since Wayland compositors commonly do not advertise the sRGB variant.
pub fn default_surface_format() -> TextureFormat {
    if is_wayland_session() {
        TextureFormat::Bgra8Unorm
    } else {
        TextureFormat::Bgra8UnormSrgb
    }
}

/// Whether the process is running under a Wayland session.
///
/// Both default-format helpers go through here, so they can never disagree about what kind of
/// session this is. Always `false` off Linux.
fn is_wayland_session() -> bool {
    #[cfg(target_os = "linux")]
    {
        is_wayland(
            std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
            std::env::var("WAYLAND_DISPLAY").ok().as_deref(),
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// The session-detection rule itself, taking its inputs as arguments so it can be tested without
/// touching the process environment.
///
/// `XDG_SESSION_TYPE` is authoritative when set, because a Wayland compositor running Xwayland
/// still exports `WAYLAND_DISPLAY` to X11 clients. `WAYLAND_DISPLAY` is only the fallback.
#[cfg(any(target_os = "linux", test))]
fn is_wayland(xdg_session_type: Option<&str>, wayland_display: Option<&str>) -> bool {
    match xdg_session_type {
        Some(session_type) => session_type == "wayland",
        None => wayland_display.is_some_and(|display| !display.is_empty()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdg_session_type_decides_when_it_is_set() {
        assert!(is_wayland(Some("wayland"), None));
        assert!(!is_wayland(Some("x11"), None));
    }

    #[test]
    fn xdg_session_type_wins_over_wayland_display() {
        // Xwayland exports WAYLAND_DISPLAY to X11 clients, so trusting it here would misreport
        // an X11 client as a Wayland one.
        assert!(!is_wayland(Some("x11"), Some("wayland-0")));
    }

    #[test]
    fn falls_back_to_wayland_display_when_session_type_is_unset() {
        assert!(is_wayland(None, Some("wayland-0")));
        assert!(!is_wayland(None, None));
    }

    #[test]
    fn an_empty_wayland_display_does_not_count() {
        assert!(!is_wayland(None, Some("")));
    }

    #[test]
    fn the_two_default_formats_agree_on_srgb() {
        // A blit from an offscreen texture to the surface must not cross an sRGB boundary by
        // accident, so these two always share their sRGB-ness.
        let srgb_texture = default_texture_format().is_srgb();
        let srgb_surface = default_surface_format().is_srgb();
        assert_eq!(srgb_texture, srgb_surface);
    }
}
