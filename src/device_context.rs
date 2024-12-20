use std::{path::PathBuf, sync::Arc};

use wgpu::{
    Adapter, AddressMode, Backends, Device, DeviceDescriptor, FilterMode, Instance,
    InstanceDescriptor, InstanceFlags, Limits, MemoryHints, PowerPreference, Queue,
    RequestAdapterOptions, Sampler, SamplerDescriptor, Surface,
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
    sampler_nearest_repeat: Arc<Sampler>,
    sampler_linear_repeat: Arc<Sampler>,
    sampler_nearest_clamp_to_edge: Arc<Sampler>,
    sampler_linear_clamp_to_edge: Arc<Sampler>,
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
            Self::create_adapter_device_and_queue(config, &instance, None)?;
        let sampler_nearest_repeat = Arc::new(device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        }));
        let sampler_linear_repeat = Arc::new(device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        }));
        let sampler_nearest_clamp_to_edge = Arc::new(device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        }));
        let sampler_linear_clamp_to_edge = Arc::new(device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        }));

        Ok(Self {
            config: config.clone(),
            instance,
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
            sampler_nearest_repeat,
            sampler_linear_repeat,
            sampler_nearest_clamp_to_edge,
            sampler_linear_clamp_to_edge,
        })
    }

    /// If adapter, device and queue has been created without a window (surface), recreate them
    /// once you have a surface to ensure compatibility of queue families.
    pub fn reconfigure_with_surface(&mut self, surface: &Surface) -> Result<(), GlassError> {
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

    pub fn sampler_nearest_repeat(&self) -> &Arc<Sampler> {
        &self.sampler_nearest_repeat
    }

    pub fn sampler_linear_repeat(&self) -> &Arc<Sampler> {
        &self.sampler_linear_repeat
    }

    pub fn sampler_nearest_clamp_to_edge(&self) -> &Arc<Sampler> {
        &self.sampler_nearest_clamp_to_edge
    }

    pub fn sampler_linear_clamp_to_edge(&self) -> &Arc<Sampler> {
        &self.sampler_linear_clamp_to_edge
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
