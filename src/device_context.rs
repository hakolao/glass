//! Requesting and holding the wgpu instance, adapter, device and queue.

use std::{path::PathBuf, sync::Arc};

use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor,
    InstanceFlags, Limits, MemoryHints, PowerPreference, Queue, RequestAdapterOptions, Surface,
    Trace,
};

use crate::{
    error::{AdapterNotFoundError, DeviceCreationError, InsufficientDeviceError},
    utils::wait_async,
    GlassError,
};

/// What to ask wgpu for when creating the instance, adapter and device.
///
/// `features` and `limits` are requirements, not hints: if the adapter cannot meet them,
/// [`DeviceContext::new`] fails with [`GlassError::InsufficientDevice`] rather than quietly
/// giving you less than you asked for.
#[derive(Debug, Clone)]
pub struct DeviceConfig {
    /// Whether to prefer a discrete or an integrated GPU.
    pub power_preference: PowerPreference,
    /// Whether wgpu should tune its allocator for performance or for low memory use.
    pub memory_hints: MemoryHints,
    /// Features the adapter must support. Nothing is added on your behalf.
    pub features: wgpu::Features,
    /// Limits the adapter must meet.
    pub limits: Limits,
    /// Which graphics backends may be used. Narrowing this is the usual cause of
    /// [`GlassError::AdapterNotFound`].
    pub backends: Backends,
    /// Instance-level debugging and validation flags.
    pub instance_flags: InstanceFlags,
    /// Where to write an API trace. Currently unused; wgpu tracing is off.
    pub trace_path: Option<PathBuf>,
}

impl DeviceConfig {
    /// A configuration that asks for the high-performance adapter and all backends.
    pub fn performance() -> DeviceConfig {
        DeviceConfig {
            power_preference: PowerPreference::HighPerformance,
            memory_hints: MemoryHints::Performance,
            features: wgpu::Features::empty(),
            limits: Limits::default(),
            backends: Backends::all(),
            instance_flags: InstanceFlags::from_build_config(),
            trace_path: None,
        }
    }
}

impl Default for DeviceConfig {
    fn default() -> Self {
        DeviceConfig {
            power_preference: PowerPreference::default(),
            memory_hints: MemoryHints::Performance,
            features: wgpu::Features::empty(),
            limits: Limits::default(),
            backends: Backends::all(),
            instance_flags: InstanceFlags::from_build_config(),
            trace_path: None,
        }
    }
}

/// The wgpu instance, adapter, device and queue, shared by every window.
#[derive(Debug)]
pub struct DeviceContext {
    config: DeviceConfig,
    instance: Instance,
    adapter: Adapter,
    device: Arc<Device>,
    queue: Arc<Queue>,
}

impl DeviceContext {
    /// Requests an adapter matching `config` and a device from it.
    ///
    /// # Errors
    ///
    /// Fails with [`GlassError::AdapterNotFound`] if no adapter matches `config.backends`,
    /// [`GlassError::InsufficientDevice`] if the adapter lacks the requested features or
    /// limits, or [`GlassError::DeviceError`] if the adapter refuses to create a device.
    pub fn new(config: &DeviceConfig) -> Result<DeviceContext, GlassError> {
        let instance = Instance::new(InstanceDescriptor {
            backends: config.backends,
            flags: config.instance_flags,
            ..InstanceDescriptor::new_without_display_handle()
        });
        let (adapter, device, queue) =
            Self::create_adapter_device_and_queue(config, &instance, None)?;
        Ok(Self {
            config: config.clone(),
            instance,
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
        })
    }

    /// If adapter, device and queue has been created without a window (surface), recreate them
    /// once you have a surface to ensure compatibility of queue families.
    /// # Errors
    ///
    /// Fails the same way [`DeviceContext::new`] does.
    pub fn reconfigure_with_surface(&mut self, surface: &Surface<'_>) -> Result<(), GlassError> {
        let (adapter, device, queue) =
            Self::create_adapter_device_and_queue(&self.config, &self.instance, Some(surface))?;
        self.adapter = adapter;
        self.device = Arc::new(device);
        self.queue = Arc::new(queue);
        Ok(())
    }

    fn create_adapter_device_and_queue(
        config: &DeviceConfig,
        instance: &Instance,
        surface: Option<&Surface<'_>>,
    ) -> Result<(Adapter, Device, Queue), GlassError> {
        let adapter = match wait_async(instance.request_adapter(&RequestAdapterOptions {
            power_preference: config.power_preference,
            force_fallback_adapter: false,
            compatible_surface: surface,
            apply_limit_buckets: false,
        })) {
            Ok(a) => a,
            Err(e) => {
                // Fresh instance: `instance` was built with only `config.backends`, so it
                // can't see adapters on backends that were never initialized.
                let probe = Instance::new(InstanceDescriptor {
                    backends: Backends::all(),
                    flags: config.instance_flags,
                    ..InstanceDescriptor::new_without_display_handle()
                });
                let available: Vec<_> = wait_async(probe.enumerate_adapters(Backends::all()))
                    .iter()
                    .map(|a| a.get_info())
                    .collect();
                log_visible_adapters(&available);
                return Err(Box::new(AdapterNotFoundError {
                    source: e,
                    requested_backends: config.backends,
                    available,
                })
                .into());
            }
        };

        let info = adapter.get_info();
        log::info!(
            "using adapter '{}' | {:?} | {:?} | driver: {} {}",
            info.name,
            info.backend,
            info.device_type,
            info.driver,
            info.driver_info
        );

        // --- Capability check: error instead of downgrading ---
        let missing_features = config.features.difference(adapter.features());
        if !missing_features.is_empty() {
            return Err(Box::new(InsufficientDeviceError {
                adapter: adapter.get_info(),
                missing_features,
                violations: Vec::new(),
            })
            .into());
        }

        let mut violations = Vec::new();
        config.limits.check_limits_with_fail_fn(
            &adapter.limits(),
            false, // report ALL shortfalls, not just the first
            |name, required, allowed| {
                violations.push(format!("{name} (need {required}, allowed {allowed})"));
            },
        );
        if !violations.is_empty() {
            return Err(Box::new(InsufficientDeviceError {
                adapter: adapter.get_info(),
                missing_features: Features::empty(),
                violations,
            })
            .into());
        }
        // -------------------------------------------------------

        let _path = config.trace_path.as_deref();
        let (device, queue) = match wait_async(adapter.request_device(&DeviceDescriptor {
            label: None,
            required_features: config.features,
            required_limits: config.limits.clone(),
            experimental_features: Default::default(),
            memory_hints: config.memory_hints.clone(),
            trace: Trace::Off,
        })) {
            Ok(dq) => dq,
            Err(e) => {
                return Err(Box::new(DeviceCreationError {
                    source: e,
                    adapter: adapter.get_info(),
                })
                .into())
            }
        };

        Ok((adapter, device, queue))
    }

    /// The wgpu instance every surface is created from.
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    /// The adapter the device was requested from.
    pub fn adapter(&self) -> &Adapter {
        &self.adapter
    }

    /// The shared device.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// A cloned handle to the shared device.
    pub fn device_arc(&self) -> Arc<Device> {
        self.device.clone()
    }

    /// The shared queue.
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// A cloned handle to the shared queue.
    pub fn queue_arc(&self) -> Arc<Queue> {
        self.queue.clone()
    }
}

/// Logs every adapter the machine exposes on any backend.
///
/// This is the detail that used to be baked into the error message. It lives here so that the
/// error stays one line while the diagnostics remain available to anyone running with a logger.
fn log_visible_adapters(available: &[wgpu::AdapterInfo]) {
    if available.is_empty() {
        log::warn!("no adapters are visible on any backend");
        return;
    }
    log::warn!("adapters visible on other backends:");
    for info in available {
        log::warn!(
            "  {} | {:?} | {:?} | driver: {} {}",
            info.name,
            info.backend,
            info.device_type,
            info.driver,
            info.driver_info
        );
    }
}
