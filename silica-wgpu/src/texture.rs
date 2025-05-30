use wgpu::util::DeviceExt;

use crate::Context;

pub type TextureSize = euclid::Size2D<u32, Texture>;
pub type TextureRect = euclid::Box2D<u32, Texture>;

pub struct TextureConfig {
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl TextureConfig {
    pub fn new(context: &Context, filter: wgpu::FilterMode) -> Self {
        use wgpu::*;
        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("silica texture bind group layout"),
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                            ty: BindingType::Texture {
                                multisampled: false,
                                view_dimension: TextureViewDimension::D2,
                                sample_type: TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Sampler(SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });
        let sampler = context.device.create_sampler(&SamplerDescriptor {
            label: Some("silica texture sampler"),
            mag_filter: filter,
            min_filter: filter,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });
        TextureConfig {
            bind_group_layout,
            sampler,
        }
    }
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Texture {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

impl Texture {
    fn convert_size(size: TextureSize) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        }
    }
    fn create_texture(
        device: &wgpu::Device,
        size: TextureSize,
        format: wgpu::TextureFormat,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: Self::convert_size(size),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }
    fn create_texture_with_data(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: TextureSize,
        format: wgpu::TextureFormat,
        data: &[u8],
    ) -> wgpu::Texture {
        device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: None,
                size: Self::convert_size(size),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data,
        )
    }
    fn create_bind_group(
        context: &Context,
        config: &TextureConfig,
        texture: &wgpu::Texture,
    ) -> wgpu::BindGroup {
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &config.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&config.sampler),
                    },
                ],
            })
    }
    pub fn new(
        context: &Context,
        config: &TextureConfig,
        size: TextureSize,
        format: wgpu::TextureFormat,
    ) -> Self {
        let texture = Self::create_texture(&context.device, size, format);
        let bind_group = Self::create_bind_group(context, config, &texture);
        Texture {
            texture,
            bind_group,
        }
    }
    pub fn new_with_data(
        context: &Context,
        config: &TextureConfig,
        size: TextureSize,
        format: wgpu::TextureFormat,
        data: &[u8],
    ) -> Self {
        let texture =
            Self::create_texture_with_data(&context.device, &context.queue, size, format, data);
        let bind_group = Self::create_bind_group(context, config, &texture);
        Texture {
            texture,
            bind_group,
        }
    }
    pub fn width(&self) -> u32 {
        self.texture.width()
    }
    pub fn height(&self) -> u32 {
        self.texture.height()
    }
    pub fn size(&self) -> TextureSize {
        TextureSize::new(self.width(), self.height())
    }
    pub fn format(&self) -> wgpu::TextureFormat {
        self.texture.format()
    }
    pub fn write_data(
        &self,
        context: &Context,
        rect: TextureRect,
        data: &[u8],
        offset: u64,
        stride: u32,
    ) {
        let mut texture_copy_info = self.texture.as_image_copy();
        texture_copy_info.origin = wgpu::Origin3d {
            x: rect.min.x,
            y: rect.min.y,
            z: 0,
        };
        context.queue.write_texture(
            texture_copy_info,
            data,
            wgpu::TexelCopyBufferLayout {
                offset,
                bytes_per_row: Some(stride),
                rows_per_image: None,
            },
            Self::convert_size(rect.size()),
        );
    }
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
