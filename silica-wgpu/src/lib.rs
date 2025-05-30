pub mod draw;
mod texture;

use std::{marker::PhantomData, num::NonZero};

use euclid::point2;

pub use texture::*;
pub use wgpu;

pub struct Uv;
pub type UvRect = euclid::Box2D<f32, Uv>;

impl Uv {
    pub const ZERO: UvRect = UvRect::new(point2(0.0, 0.0), point2(0.0, 0.0));
    pub const FULL: UvRect = UvRect::new(point2(0.0, 0.0), point2(1.0, 1.0));
    pub fn normalize(rect: TextureRect, texture_size: TextureSize) -> UvRect {
        let size = texture_size.to_f32();
        rect.to_f32()
            .scale(1.0 / size.width, 1.0 / size.height)
            .cast_unit()
    }
}

pub struct AdapterFeatures {
    pub required_features: wgpu::Features,
    pub optional_features: wgpu::Features,
    pub required_downlevel_capabilities: wgpu::DownlevelCapabilities,
    pub required_limits: wgpu::Limits,
}

impl Default for AdapterFeatures {
    fn default() -> Self {
        Self {
            required_features: wgpu::Features::empty(),
            optional_features: wgpu::Features::empty(),
            required_downlevel_capabilities: wgpu::DownlevelCapabilities {
                flags: wgpu::DownlevelFlags::empty(),
                shader_model: wgpu::ShaderModel::Sm5,
                ..wgpu::DownlevelCapabilities::default()
            },
            required_limits: wgpu::Limits::downlevel_defaults(),
        }
    }
}

/// Context containing global wgpu resources.
#[derive(Clone)]
pub struct Context {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Context {
    async fn get_adapter_with_capabilities_or_from_env(
        instance: &wgpu::Instance,
        required_features: &wgpu::Features,
        required_downlevel_capabilities: &wgpu::DownlevelCapabilities,
        surface: &Option<&wgpu::Surface<'_>>,
    ) -> wgpu::Adapter {
        use wgpu::Backends;
        if std::env::var("WGPU_ADAPTER_NAME").is_ok() {
            let adapter = wgpu::util::initialize_adapter_from_env_or_default(instance, *surface)
                .await
                .expect("No suitable GPU adapters found on the system!");

            let adapter_info = adapter.get_info();
            log::info!("Using {} ({:?})", adapter_info.name, adapter_info.backend);

            let adapter_features = adapter.features();
            assert!(
                adapter_features.contains(*required_features),
                "Adapter does not support the required features: {:?}",
                *required_features - adapter_features
            );

            let downlevel_capabilities = adapter.get_downlevel_capabilities();
            assert!(
                downlevel_capabilities.shader_model >= required_downlevel_capabilities.shader_model,
                "Adapter does not support the required minimum shader model: {:?}",
                required_downlevel_capabilities.shader_model
            );
            assert!(
                downlevel_capabilities
                    .flags
                    .contains(required_downlevel_capabilities.flags),
                "Adapter does not support the required downlevel capabilities: {:?}",
                required_downlevel_capabilities.flags - downlevel_capabilities.flags
            );
            adapter
        } else {
            let adapters = instance.enumerate_adapters(Backends::all());

            let mut chosen_adapter = None;
            for adapter in adapters {
                if let Some(surface) = surface {
                    if !adapter.is_surface_supported(surface) {
                        continue;
                    }
                }

                let required_features = *required_features;
                let adapter_features = adapter.features();
                if !adapter_features.contains(required_features) {
                    continue;
                } else {
                    chosen_adapter = Some(adapter);
                    break;
                }
            }

            chosen_adapter.expect("No suitable GPU adapters found on the system!")
        }
    }

    /// Initializes the device with the given features.
    pub async fn init_async(features: AdapterFeatures) -> Self {
        log::info!("Initializing wgpu...");

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::from_env_or_default());

        let adapter = Self::get_adapter_with_capabilities_or_from_env(
            &instance,
            &features.required_features,
            &features.required_downlevel_capabilities,
            &None,
        )
        .await;
        // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the surface.
        let needed_limits = features.required_limits.using_resolution(adapter.limits());

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: (features.optional_features & adapter.features())
                    | features.required_features,
                required_limits: needed_limits,
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Unable to find a suitable GPU adapter!");

        Self {
            instance,
            adapter,
            device,
            queue,
        }
    }

    pub fn init(features: AdapterFeatures) -> Self {
        pollster::block_on(Self::init_async(features))
    }
}

#[derive(Default)]
pub struct Surface {
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
}

pub type SurfaceSize = euclid::Size2D<u32, Surface>;

impl Surface {
    /// Create a new surface wrapper with no surface or configuration.
    pub fn new() -> Self {
        Surface::default()
    }

    /// Called when an event which matches [`Self::start_condition`] is received.
    ///
    /// On all native platforms, this is where we create the surface.
    ///
    /// Additionally, we configure the surface based on the (now valid) window size.
    pub fn resume(
        &mut self,
        context: &Context,
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        size: SurfaceSize,
    ) {
        // Window size is only actually valid after we enter the event loop.
        let width = size.width.max(1);
        let height = size.height.max(1);

        log::debug!("Surface resume {size:?}");

        // We didn't create the surface in pre_adapter, so we need to do so now.
        self.surface = Some(context.instance.create_surface(window).unwrap());
        let surface = self.surface.as_ref().unwrap();

        // Get the default configuration,
        let mut config = surface
            .get_default_config(&context.adapter, width, height)
            .expect("Surface isn't supported by the adapter.");

        // All platforms support non-sRGB swapchains, so we can just use the format directly.
        let format = config.format.remove_srgb_suffix();
        config.format = format;
        config.view_formats.push(format);

        surface.configure(&context.device, &config);
        self.config = Some(config);
    }

    /// Resize the surface, making sure to not resize to zero.
    pub fn resize(&mut self, context: &Context, size: SurfaceSize) {
        log::debug!("Surface resize {size:?}");

        let config = self.config.as_mut().unwrap();
        config.width = size.width.max(1);
        config.height = size.height.max(1);
        let surface = self.surface.as_ref().unwrap();
        surface.configure(&context.device, config);
    }

    /// Acquire the next surface texture.
    pub fn acquire(&mut self, context: &Context) -> wgpu::SurfaceTexture {
        let surface = self.surface.as_ref().unwrap();

        match surface.get_current_texture() {
            Ok(frame) => frame,
            // If we timed out, just try again
            Err(wgpu::SurfaceError::Timeout) => surface
                .get_current_texture()
                .expect("Failed to acquire next surface texture!"),
            Err(
                // If the surface is outdated, or was lost, reconfigure it.
                wgpu::SurfaceError::Outdated
                | wgpu::SurfaceError::Lost
                | wgpu::SurfaceError::Other
                // If OutOfMemory happens, reconfiguring may not help, but we might as well try
                | wgpu::SurfaceError::OutOfMemory,
            ) => {
                surface.configure(&context.device, self.config());
                surface
                    .get_current_texture()
                    .expect("Failed to acquire next surface texture!")
            }
        }
    }

    /// On suspend on android, we drop the surface, as it's no longer valid.
    ///
    /// A suspend event is always followed by at least one resume event.
    pub fn suspend(&mut self) {
        log::debug!("Surface suspend");
        self.surface = None;
    }

    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        self.config.as_ref().unwrap()
    }
}

pub struct ResizableBuffer<T> {
    buffer: wgpu::Buffer,
    length: usize,
    capacity: usize,
    _type: PhantomData<T>,
}

impl<T: bytemuck::Pod> ResizableBuffer<T> {
    const MINIMUM_SIZE: usize = 512;
    fn create_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: capacity as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
    pub fn new(context: &Context) -> Self {
        let capacity = Self::MINIMUM_SIZE;
        ResizableBuffer {
            buffer: Self::create_buffer(&context.device, capacity),
            length: 0,
            capacity,
            _type: PhantomData,
        }
    }
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
    pub fn len(&self) -> usize {
        self.length
    }
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
    pub fn set_data(&mut self, context: &Context, data: &[T]) {
        self.length = data.len();
        if data.is_empty() {
            return;
        }
        let bytes = std::mem::size_of_val(data);
        if bytes > self.capacity {
            self.capacity = bytes.next_power_of_two();
            self.buffer = Self::create_buffer(&context.device, self.capacity);
        }
        let mut write_view = context
            .queue
            .write_buffer_with(&self.buffer, 0, NonZero::new(bytes as u64).unwrap())
            .unwrap();
        write_view.copy_from_slice(bytemuck::cast_slice(data));
    }
}
