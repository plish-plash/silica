use std::{fs::File, path::Path};

use etagere::BucketedAtlasAllocator;
use png::*;
use silica_wgpu::{Context, Texture, TextureConfig, TextureRect, TextureSize, Uv, UvRect, wgpu};

use crate::{GameError, error::io_data_error, load_asset};

pub type ImagePoint = euclid::Point2D<u32, Image>;
pub type ImageSize = euclid::Size2D<u32, Image>;

pub struct Image {
    pub size: ImageSize,
    pub data: Vec<u8>,
}

impl Image {
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, GameError> {
        load_asset(path, |path| {
            let file = File::open(path)?;
            let mut decoder = Decoder::new(file);
            decoder.set_transformations(Transformations::ALPHA);
            let mut image_reader = decoder.read_info()?;
            let mut data = vec![0; image_reader.output_buffer_size()];
            let info = image_reader.next_frame(&mut data)?;
            data.truncate(info.buffer_size());
            assert_eq!(info.bit_depth, BitDepth::Eight);
            match info.color_type {
                ColorType::Rgba => {}
                ColorType::GrayscaleAlpha => {
                    data = data
                        .chunks_exact(2)
                        .flat_map(|x| [x[0], x[0], x[0], x[1]])
                        .collect();
                }
                _ => {
                    return Err(DecodingError::IoError(io_data_error(
                        true,
                        format!("unsupported color type {:?}", info.color_type),
                    ))
                    .into());
                }
            }
            Ok(Image {
                size: ImageSize::new(info.width, info.height),
                data,
            })
        })
    }
    pub fn create_texture(&self, context: &Context, config: &TextureConfig) -> Texture {
        Texture::new_with_data(
            context,
            config,
            self.size.cast_unit(),
            Self::FORMAT,
            &self.data,
        )
    }
    pub fn load_texture<P: AsRef<Path>>(
        context: &Context,
        config: &TextureConfig,
        path: P,
    ) -> Result<Texture, GameError> {
        Ok(Self::load(path)?.create_texture(context, config))
    }
    pub fn write_to_texture(
        &self,
        context: &Context,
        source: ImagePoint,
        texture: &Texture,
        rect: Option<TextureRect>,
    ) -> UvRect {
        const BPP: u32 = 4;
        let rect = rect.unwrap_or(TextureRect::from_size(self.size.cast_unit()));
        let offset = (source.x + (source.y * self.size.width)) * BPP;
        texture.write_data(
            context,
            rect,
            &self.data,
            offset as u64,
            self.size.width * BPP,
        );
        Uv::normalize(rect, texture.size())
    }
}

pub struct TextureAtlas {
    texture: Texture,
    allocator: BucketedAtlasAllocator,
}

impl TextureAtlas {
    pub fn new(context: &Context, config: &TextureConfig, size: TextureSize) -> Self {
        TextureAtlas {
            texture: Texture::new(context, config, size, Image::FORMAT),
            allocator: BucketedAtlasAllocator::new(size.to_i32().cast_unit()),
        }
    }
    pub fn load(&mut self, context: &Context, image: &Image) -> UvRect {
        let alloc = self
            .allocator
            .allocate(image.size.to_i32().cast_unit())
            .expect("not enough space in atlas");
        let rect = TextureRect::from_origin_and_size(
            alloc.rectangle.min.to_u32().cast_unit(),
            image.size.cast_unit(),
        );
        image.write_to_texture(context, ImagePoint::zero(), &self.texture, Some(rect))
    }
    pub fn load_frames(
        &mut self,
        context: &Context,
        image: &Image,
        frame_size: TextureSize,
    ) -> Vec<UvRect> {
        let mut uvs = Vec::new();
        let mut x = 0;
        while x + frame_size.width <= image.size.width {
            let alloc = self
                .allocator
                .allocate(frame_size.to_i32().cast_unit())
                .expect("not enough space in atlas");
            let rect = TextureRect::from_origin_and_size(
                alloc.rectangle.min.to_u32().cast_unit(),
                frame_size,
            );
            uvs.push(image.write_to_texture(
                context,
                ImagePoint::new(x, 0),
                &self.texture,
                Some(rect),
            ));
            x += frame_size.width;
        }
        uvs
    }
    pub fn finish(self, name: &str) -> Texture {
        let fill_ratio =
            self.allocator.allocated_space() as f32 / self.allocator.size().area() as f32;
        log::debug!(
            "{} texture atlas {}% filled",
            name,
            (fill_ratio * 100.0) as i32
        );
        self.texture
    }
}
