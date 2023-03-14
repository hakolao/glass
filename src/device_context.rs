use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, Instance, InstanceDescriptor, Limits,
    PowerPreference, Queue, RequestAdapterOptions, Surface,
};
use winit::window::Window;

use crate::{utils::wait_async, window::WindowConfig};

#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub power_preference: PowerPreference,
    pub features: wgpu::Features,
    pub limits: Limits,
    pub backends: Backends,
}

impl DeviceConfig {
    pub fn performance() -> DeviceConfig {
        DeviceConfig {
            power_preference: PowerPreference::HighPerformance,
            features: wgpu::Features::empty(),
            limits: Limits::default(),
            backends: Backends::VULKAN,
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
        }
    }
}

#[derive(Debug)]
pub struct DeviceContext {
    config: DeviceConfig,
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
}

unsafe impl Send for DeviceContext {}

unsafe impl Sync for DeviceContext {}

impl DeviceContext {
    pub fn new(config: &DeviceConfig, initial_windows: &[(WindowConfig, Window)]) -> DeviceContext {
        let instance = Instance::new(InstanceDescriptor {
            backends: config.backends,
            dx12_shader_compiler: Default::default(),
        });
        // Ensure render context is compatible with our window...
        let surface_maybe = if let Some((_c, w)) = initial_windows.first() {
            Some(unsafe { instance.create_surface(&w).unwrap() })
        } else {
            None
        };
        let (adapter, device, queue) =
            Self::create_adapter_device_and_queue(config, &instance, surface_maybe.as_ref());
        Self {
            config: config.clone(),
            instance,
            adapter,
            device,
            queue,
        }
    }

    /// If adapter, device and queue has been created without a window (surface), recreate them
    /// once you have a surface to ensure compatibility of queue families.
    pub fn reconfigure_with_surface(&mut self, surface: &Surface) {
        let (adapter, device, queue) =
            Self::create_adapter_device_and_queue(&self.config, &self.instance, Some(surface));
        self.adapter = adapter;
        self.device = device;
        self.queue = queue;
    }

    fn create_adapter_device_and_queue(
        config: &DeviceConfig,
        instance: &Instance,
        surface: Option<&Surface>,
    ) -> (Adapter, Device, Queue) {
        let adapter = wait_async(instance.request_adapter(&RequestAdapterOptions {
            power_preference: config.power_preference,
            force_fallback_adapter: false,
            compatible_surface: surface,
        }))
        .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let (device, queue) = wait_async(adapter.request_device(
            &DeviceDescriptor {
                label: None,
                features: config.features,
                limits: config.limits.clone(),
            },
            None,
        ))
        .expect("Failed to create device");
        (adapter, device, queue)
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

    pub fn queue(&self) -> &Queue {
        &self.queue
    }
}
