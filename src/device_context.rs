use std::{path::PathBuf, sync::Arc};

use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, Instance, InstanceDescriptor, InstanceFlags,
    Limits, MemoryHints, PowerPreference, Queue, RequestAdapterOptions, Surface,
};

use crate::{utils::wait_async, GlassError};

#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub power_preference: PowerPreference,
    pub memory_hints: MemoryHints,
    pub features: wgpu::Features,
    pub limits: Limits,
    pub backends: Backends,
    pub instance_flags: InstanceFlags,
    pub trace_path: Option<PathBuf>,
}

impl DeviceConfig {
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
    pub fn new(config: &DeviceConfig) -> Result<DeviceContext, GlassError> {
        let instance = Instance::new(InstanceDescriptor {
            backends: config.backends,
            flags: config.instance_flags,
            ..Default::default()
        });
        let (adapter, device, queue) =
            match Self::create_adapter_device_and_queue(config, &instance, None) {
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

        let path = config.trace_path.as_deref();
        // Create the logical device and command queue
        let (device, queue) = match wait_async(adapter.request_device(
            &DeviceDescriptor {
                label: None,
                required_features: config.features,
                required_limits: config.limits.clone(),
                memory_hints: config.memory_hints.clone(),
            },
            path,
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
