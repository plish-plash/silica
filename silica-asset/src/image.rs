use std::io::{BufRead, Error as IoError, ErrorKind, Seek};

use png::*;
use silica_wgpu::{Context, Texture, TextureConfig, TextureRect, Uv, UvRect, wgpu};

pub type ImagePoint = euclid::Point2D<u32, Image>;
pub type ImageSize = euclid::Size2D<u32, Image>;

pub struct Image {
    size: ImageSize,
    data: Vec<u8>,
}

impl Image {
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    pub fn new(width: usize, height: usize) -> Self {
        Image {
            size: ImageSize::new(width as u32, height as u32),
            data: vec![0; width * height],
        }
    }
    pub fn read<R: BufRead + Seek>(reader: R) -> Result<Self, DecodingError> {
        let mut decoder = Decoder::new(reader);
        decoder.set_transformations(Transformations::ALPHA);
        let mut image_reader = decoder.read_info()?;
        let mut data = vec![0; image_reader.output_buffer_size().unwrap()];
        let info = image_reader.next_frame(&mut data)?;
        data.truncate(info.buffer_size());
        assert_eq!(info.bit_depth, BitDepth::Eight);
        match info.color_type {
            ColorType::Rgba => {}
            ColorType::GrayscaleAlpha => {
                data = data.chunks_exact(2).flat_map(|x| [x[0], x[0], x[0], x[1]]).collect();
            }
            _ => {
                return Err(DecodingError::IoError(IoError::new(
                    ErrorKind::Unsupported,
                    format!("unsupported color type {:?}", info.color_type),
                )));
            }
        }
        Ok(Image {
            size: ImageSize::new(info.width, info.height),
            data,
        })
    }

    pub fn width(&self) -> usize {
        self.size.width as usize
    }
    pub fn height(&self) -> usize {
        self.size.height as usize
    }
    pub fn size(&self) -> ImageSize {
        self.size
    }

    pub fn create_texture(&self, context: &Context, config: &TextureConfig) -> Texture {
        Texture::new_with_data(context, config, self.size().cast_unit(), Self::FORMAT, &self.data)
    }
    pub fn write_to_texture(
        &self,
        context: &Context,
        source: ImagePoint,
        texture: &Texture,
        rect: Option<TextureRect>,
    ) -> UvRect {
        const BPP: u32 = 4;
        let rect = rect.unwrap_or(TextureRect::from_size(self.size().cast_unit()));
        let offset = (source.x + (source.y * self.size.width)) * BPP;
        texture.write_data(context, rect, &self.data, offset as u64, self.size.width * BPP);
        Uv::normalize(rect, texture.size())
    }
}
