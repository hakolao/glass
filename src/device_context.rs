use std::sync::Arc;

use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, Instance, InstanceDescriptor, InstanceFlags,
    Limits, PowerPreference, Queue, RequestAdapterOptions, Surface,
};
use winit::window::Window;

use crate::{utils::wait_async, window::WindowConfig, GlassError};

#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub power_preference: PowerPreference,
    pub features: wgpu::Features,
    pub limits: Limits,
    pub backends: Backends,
    pub instance_flags: InstanceFlags,
}

impl DeviceConfig {
    pub fn performance() -> DeviceConfig {
        DeviceConfig {
            power_preference: PowerPreference::HighPerformance,
            features: wgpu::Features::empty(),
            limits: Limits::default(),
            backends: Backends::all(),
            instance_flags: InstanceFlags::empty(),
        }
    }
}

impl Default for DeviceConfig {
    fn default() -> Self {
        DeviceConfig {
            power_preference: PowerPreference::default(),
            features: wgpu::Features::empty(),
            limits: Limits::default(),
            backends: Backends::all(),
            instance_flags: InstanceFlags::from_build_config(),
        }
    }
}

#[derive(Debug)]
pub struct DeviceContext {
    config: DeviceConfig,
    instance: Instance,
    adapter: Adapter,
    device: Arc<Device>,
    queue: Arc<Queue>,
}

unsafe impl Send for DeviceContext {}

unsafe impl Sync for DeviceContext {}

impl DeviceContext {
    pub fn new(
        config: &DeviceConfig,
        initial_windows: &[(WindowConfig, Window)],
    ) -> Result<DeviceContext, GlassError> {
        let instance = Instance::new(InstanceDescriptor {
            backends: config.backends,
            flags: config.instance_flags,
            ..Default::default()
        });
        // Ensure render context is compatible with our window...
        let surface_maybe = if let Some((_c, w)) = initial_windows.first() {
            Some(unsafe {
                match instance.create_surface(&w) {
                    Ok(s) => s,
                    Err(e) => return Err(GlassError::SurfaceError(e)),
                }
            })
        } else {
            None
        };
        let (adapter, device, queue) = match Self::create_adapter_device_and_queue(
            config,
            &instance,
            surface_maybe.as_ref(),
        ) {
            Ok(adq) => adq,
            Err(e) => return Err(e),
        };
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
    pub fn reconfigure_with_surface(&mut self, surface: &Surface) -> Result<(), GlassError> {
        let (adapter, device, queue) = match Self::create_adapter_device_and_queue(
            &self.config,
            &self.instance,
            Some(surface),
        ) {
            Ok(adq) => adq,
            Err(e) => return Err(e),
        };
        self.adapter = adapter;
        self.device = Arc::new(device);
        self.queue = Arc::new(queue);
        Ok(())
    }

    fn create_adapter_device_and_queue(
        config: &DeviceConfig,
        instance: &Instance,
        surface: Option<&Surface>,
    ) -> Result<(Adapter, Device, Queue), GlassError> {
        let adapter = match wait_async(instance.request_adapter(&RequestAdapterOptions {
            power_preference: config.power_preference,
            force_fallback_adapter: false,
            compatible_surface: surface,
        })) {
            Some(a) => a,
            None => return Err(GlassError::AdapterError),
        };

        let trace_env = std::env::var("WGPU_TRACE").ok();
        let path = trace_env.as_ref().map(std::path::Path::new);
        // Create the logical device and command queue
        let (device, queue) = match wait_async(adapter.request_device(
            &DeviceDescriptor {
                label: None,
                features: config.features,
                limits: config.limits.clone(),
            },
            if cfg!(feature = "trace") { path } else { None },
        )) {
            Ok(dq) => dq,
            Err(e) => return Err(GlassError::DeviceError(e)),
        };

        Ok((adapter, device, queue))
    }

    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    pub fn adapter(&self) -> &Adapter {
        &self.adapter
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn device_arc(&self) -> Arc<Device> {
        self.device.clone()
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn queue_arc(&self) -> Arc<Queue> {
        self.queue.clone()
    }
}
