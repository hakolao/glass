//! The error type returned throughout `glass`.

use image::ImageError;
use thiserror::Error;
use wgpu::{
    AdapterInfo, Backends, CreateSurfaceError, Features, RequestAdapterError, RequestDeviceError,
    TextureFormat,
};
use winit::{
    error::{EventLoopError, OsError},
    window::WindowId,
};

/// Every way `glass` can fail.
///
/// Variants that carry bulky diagnostics box their payload, so that returning
/// `Result<T, GlassError>` stays cheap. The full detail behind an adapter or device failure is
/// also emitted through the [`log`] facade, so enabling a logger gives you more than the
/// `Display` message.
#[derive(Debug, Error)]
pub enum GlassError {
    /// The operating system refused to create the window.
    #[error("failed to create a window: {0}")]
    WindowError(#[from] OsError),
    /// No window is registered under the given [`WindowId`], usually because it has already been
    /// closed.
    #[error("no window is registered under {window_id:?}")]
    WindowNotFoundError {
        /// The id that was looked up.
        window_id: WindowId,
    },
    /// Creating a wgpu surface for a window failed.
    #[error("failed to create a surface: {0}")]
    SurfaceError(#[from] CreateSurfaceError),
    /// The requested surface format is not among those the window's surface supports.
    ///
    /// Pick one of `supported`, or use [`GlassWindow::default_surface_format`] to get a format
    /// that works on the current platform.
    ///
    /// [`GlassWindow::default_surface_format`]: crate::GlassWindow::default_surface_format
    #[error("surface format {requested:?} is not supported (supported formats: {supported:?})")]
    UnsupportedSurfaceFormat {
        /// The format that was requested through [`WindowConfig::surface_config`].
        ///
        /// [`WindowConfig::surface_config`]: crate::WindowConfig::surface_config
        requested: TextureFormat,
        /// The formats the surface actually reports as usable.
        supported: Vec<TextureFormat>,
    },
    /// No adapter matched the requested backends. See [`AdapterNotFoundError`].
    #[error(transparent)]
    AdapterNotFound(#[from] Box<AdapterNotFoundError>),
    /// Requesting a device from the chosen adapter failed. See [`DeviceCreationError`].
    #[error(transparent)]
    DeviceError(#[from] Box<DeviceCreationError>),
    /// The chosen adapter does not meet the requested features or limits. See
    /// [`InsufficientDeviceError`].
    #[error(transparent)]
    InsufficientDevice(#[from] Box<InsufficientDeviceError>),
    /// Decoding image data for a texture failed.
    #[error("failed to decode image data: {0}")]
    ImageError(#[from] ImageError),
    /// Creating or running the winit event loop failed.
    #[error("event loop failure: {0}")]
    EventLoopError(#[from] EventLoopError),
}

/// Payload of [`GlassError::AdapterNotFound`].
///
/// `available` lists every adapter visible on *any* backend, which is what makes this actionable:
/// if it is non-empty, the requested backends were too narrow rather than the machine lacking a
/// GPU.
#[derive(Debug, Error)]
#[error(
    "no adapter matched the requested backends {requested_backends:?} ({} adapter(s) visible on \
     other backends): {source}",
    .available.len()
)]
pub struct AdapterNotFoundError {
    /// The underlying wgpu failure.
    #[source]
    pub source: RequestAdapterError,
    /// The backends that were asked for, from [`DeviceConfig::backends`].
    ///
    /// [`DeviceConfig::backends`]: crate::DeviceConfig::backends
    pub requested_backends: Backends,
    /// Every adapter visible on any backend, for diagnostics.
    pub available: Vec<AdapterInfo>,
}

/// Payload of [`GlassError::DeviceError`]: the adapter was found, but it would not hand out a
/// device.
#[derive(Debug, Error)]
#[error("failed to request a device from adapter '{}': {source}", .adapter.name)]
pub struct DeviceCreationError {
    /// The underlying wgpu failure.
    #[source]
    pub source: RequestDeviceError,
    /// The adapter that refused.
    pub adapter: AdapterInfo,
}

/// Payload of [`GlassError::InsufficientDevice`]: the adapter exists but lacks requested features
/// or does not meet requested limits.
///
/// `glass` reports this instead of silently downgrading, so that a missing capability surfaces at
/// startup rather than as a confusing failure deep in a pipeline.
#[derive(Debug)]
pub struct InsufficientDeviceError {
    /// The adapter that was checked.
    pub adapter: AdapterInfo,
    /// Requested features the adapter does not have. Empty when only limits were violated.
    pub missing_features: Features,
    /// Human-readable limit shortfalls, one per violated limit.
    pub violations: Vec<String>,
}

impl InsufficientDeviceError {
    /// Renders the missing features and violated limits as a single comma-separated list.
    fn requirements(&self) -> String {
        let mut parts = Vec::new();
        if !self.missing_features.is_empty() {
            parts.push(format!("missing features: {:?}", self.missing_features));
        }
        parts.extend(self.violations.iter().cloned());
        parts.join(", ")
    }
}

// Written by hand rather than derived, because the message needs to join two different kinds of
// shortfall (features and limits) into one list.
impl std::fmt::Display for InsufficientDeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "adapter '{}' does not meet requirements: {}",
            self.adapter.name,
            self.requirements()
        )
    }
}

impl std::error::Error for InsufficientDeviceError {}
