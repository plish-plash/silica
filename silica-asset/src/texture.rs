use etagere::BucketedAtlasAllocator;
use silica_wgpu::{Context, Texture, TextureConfig, TextureRect, TextureSize, UvRect};

use crate::image::{Image, ImagePoint};

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
            .allocate(image.size().to_i32().cast_unit())
            .expect("not enough space in atlas");
        let rect =
            TextureRect::from_origin_and_size(alloc.rectangle.min.to_u32().cast_unit(), image.size().cast_unit());
        image.write_to_texture(context, ImagePoint::zero(), &self.texture, Some(rect))
    }
    pub fn load_frames(&mut self, context: &Context, image: &Image, frame_size: TextureSize) -> Vec<UvRect> {
        let mut uvs = Vec::new();
        let mut x = 0;
        while x + frame_size.width <= image.size().width {
            let alloc = self
                .allocator
                .allocate(frame_size.to_i32().cast_unit())
                .expect("not enough space in atlas");
            let rect = TextureRect::from_origin_and_size(alloc.rectangle.min.to_u32().cast_unit(), frame_size);
            uvs.push(image.write_to_texture(context, ImagePoint::new(x, 0), &self.texture, Some(rect)));
            x += frame_size.width;
        }
        uvs
    }
    pub fn finish(self, name: &str) -> Texture {
        let fill_ratio = self.allocator.allocated_space() as f32 / self.allocator.size().area() as f32;
        log::debug!("{} texture atlas {}% filled", name, (fill_ratio * 100.0) as i32);
        self.texture
    }
}
