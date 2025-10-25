use etagere::BucketedAtlasAllocator;
pub use silica_asset::image::Image;
use silica_asset::{AssetError, AssetSource};
use silica_wgpu::{Context, Texture, TextureConfig, TextureRect, TextureSize, Uv, UvRect, wgpu};

pub type ImagePoint = euclid::Point2D<u32, Image>;
pub type ImageSize = euclid::Size2D<u32, Image>;

pub trait ImageExt {
    const FORMAT: wgpu::TextureFormat;
    fn size(&self) -> ImageSize;
    fn create_texture(&self, context: &Context, config: &TextureConfig) -> Texture;
    fn load_texture<S: AssetSource>(
        context: &Context,
        config: &TextureConfig,
        asset_source: &mut S,
        path: &str,
    ) -> Result<Texture, AssetError>;
    fn write_to_texture(
        &self,
        context: &Context,
        source: ImagePoint,
        texture: &Texture,
        rect: Option<TextureRect>,
    ) -> UvRect;
}

impl ImageExt for Image {
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    fn size(&self) -> ImageSize {
        ImageSize::new(self.width, self.height)
    }
    fn create_texture(&self, context: &Context, config: &TextureConfig) -> Texture {
        Texture::new_with_data(context, config, self.size().cast_unit(), Self::FORMAT, &self.data)
    }
    fn load_texture<S: AssetSource>(
        context: &Context,
        config: &TextureConfig,
        asset_source: &mut S,
        path: &str,
    ) -> Result<Texture, AssetError> {
        Ok(silica_asset::load_image(asset_source, path)?.create_texture(context, config))
    }
    fn write_to_texture(
        &self,
        context: &Context,
        source: ImagePoint,
        texture: &Texture,
        rect: Option<TextureRect>,
    ) -> UvRect {
        const BPP: u32 = 4;
        let rect = rect.unwrap_or(TextureRect::from_size(self.size().cast_unit()));
        let offset = (source.x + (source.y * self.width)) * BPP;
        texture.write_data(context, rect, &self.data, offset as u64, self.width * BPP);
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
