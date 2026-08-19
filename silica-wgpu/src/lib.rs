mod batcher;
pub mod draw;
mod texture;

use std::{marker::PhantomData, sync::Arc};

use bytemuck::Pod;
use euclid::point2;
pub use wgpu;
use wgpu::wgt::WgpuHasDisplayHandle;

pub use crate::{batcher::*, texture::*};

pub struct Uv;
pub type UvRect = euclid::Box2D<f32, Uv>;

impl Uv {
    pub const ZERO: UvRect = UvRect::new(point2(0.0, 0.0), point2(0.0, 0.0));
    pub const FULL: UvRect = UvRect::new(point2(0.0, 0.0), point2(1.0, 1.0));
    pub fn normalize(rect: TextureRect, texture_size: TextureSize) -> UvRect {
        let size = texture_size.to_f32();
        rect.to_f32().scale(1.0 / size.width, 1.0 / size.height).cast_unit()
    }
    pub fn flip(mut rect: UvRect, flip_h: bool, flip_v: bool) -> UvRect {
        if flip_h {
            std::mem::swap(&mut rect.min.x, &mut rect.max.x);
        }
        if flip_v {
            std::mem::swap(&mut rect.min.y, &mut rect.max.y);
        }
        rect
    }
}

pub type Surface = wgpu::Surface<'static>;
pub type SurfaceSize = euclid::Size2D<u32, Surface>;

/// Context containing global wgpu resources.
pub struct Context {
    instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: Surface,
    config: wgpu::SurfaceConfiguration,
}

impl Context {
    pub async fn init_async(
        display: Box<dyn WgpuHasDisplayHandle>,
        window: Arc<impl wgpu::DisplayAndWindowHandle + 'static>,
        mut size: SurfaceSize,
    ) -> Self {
        log::info!("Initializing wgpu...");
        size.width = size.width.max(1);
        size.height = size.height.max(1);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(display));

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter,
                // so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device");

        let mut config = surface.get_default_config(&adapter, size.width, size.height).unwrap();
        if !config.format.is_srgb() {
            config.format = config.format.add_srgb_suffix();
            config.view_formats.push(config.format);
        }
        surface.configure(&device, &config);

        Self {
            instance,
            _adapter: adapter,
            device,
            queue,
            surface,
            config,
        }
    }
    pub fn init(
        display: Box<dyn WgpuHasDisplayHandle>,
        window: Arc<impl wgpu::DisplayAndWindowHandle + 'static>,
        size: SurfaceSize,
    ) -> Self {
        pollster::block_on(Self::init_async(display, window, size))
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn resize(&mut self, size: SurfaceSize) {
        self.config.width = size.width.max(1);
        self.config.height = size.height.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    pub fn get_surface_texture(
        &mut self,
        window: &Arc<impl wgpu::DisplayAndWindowHandle + 'static>,
    ) -> Option<wgpu::SurfaceTexture> {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => Some(frame),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => None,
            wgpu::CurrentSurfaceTexture::Suboptimal(_) | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                None
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!("No error scope registered, so validation errors will panic")
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(window.clone()).unwrap();
                self.surface.configure(&self.device, &self.config);
                None
            }
        }
    }
}

pub struct BufferWriter<T> {
    view: wgpu::QueueWriteBufferView,
    offset: usize,
    _type: PhantomData<T>,
}

impl<T: Pod> BufferWriter<T> {
    pub fn is_full(&self) -> bool {
        self.offset >= self.view.len()
    }
    pub fn push(&mut self, value: T) {
        let bytes = bytemuck::bytes_of(&value);
        self.view
            .slice(self.offset..(self.offset + bytes.len()))
            .copy_from_slice(bytes);
        self.offset += bytes.len();
    }
}
impl<T: Pod> Extend<T> for BufferWriter<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter.into_iter() {
            self.push(item);
        }
    }
}

pub struct Buffer<T> {
    buffer: wgpu::Buffer,
    _type: PhantomData<T>,
}

impl<T: Pod> Buffer<T> {
    pub fn new(context: &Context, capacity: usize) -> Self {
        assert!(capacity > 0);
        let buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (capacity * std::mem::size_of::<T>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Buffer {
            buffer,
            _type: PhantomData,
        }
    }
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
    pub fn capacity(&self) -> usize {
        self.buffer.size() as usize / std::mem::size_of::<T>()
    }
    pub fn set_data(&mut self, context: &Context, data: &[T]) {
        if data.is_empty() {
            return;
        }
        context.queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
    }
    pub fn write(&mut self, context: &Context) -> BufferWriter<T> {
        BufferWriter {
            view: context
                .queue
                .write_buffer_with(&self.buffer, 0, self.buffer.size().try_into().unwrap())
                .unwrap(),
            offset: 0,
            _type: PhantomData,
        }
    }
}

pub struct ResizableBuffer<T>(Buffer<T>, usize);

impl<T: Pod> ResizableBuffer<T> {
    const INITIAL_CAPACITY: usize = 64;
    pub fn new(context: &Context) -> Self {
        ResizableBuffer(Buffer::new(context, Self::INITIAL_CAPACITY), 0)
    }
    pub fn buffer(&self) -> &wgpu::Buffer {
        self.0.buffer()
    }
    pub fn len(&self) -> usize {
        self.1
    }
    pub fn is_empty(&self) -> bool {
        self.1 == 0
    }
    pub fn set_data(&mut self, context: &Context, data: &[T]) {
        if data.len() > self.0.capacity() {
            self.0 = Buffer::new(context, data.len().next_power_of_two());
        }
        self.0.set_data(context, data);
        self.1 = data.len();
    }
}
