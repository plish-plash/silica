use std::{collections::HashMap, num::NonZeroU64, ops::Range};

use bytemuck::{Pod, Zeroable};
use glyphon::{FontSystem, PrepareError, RenderError, TextArea, TextRenderer};
use silica_color::Rgba;
use silica_wgpu::{
    draw::DrawQuad, wgpu, Context, ResizableBuffer, SurfaceSize, Texture, TextureConfig, UvRect,
};

use crate::{
    theme::{Theme, ThemeColor, ThemeLoader},
    Gui,
};

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Quad {
    pub rect: crate::Rect,
    pub uv: UvRect,
    pub color: Rgba,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    screen_resolution: SurfaceSize,
    _pad: [u32; 2],
}

struct Viewport {
    params: Params,
    params_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Viewport {
    fn new(device: &wgpu::Device, uniforms_layout: &wgpu::BindGroupLayout) -> Self {
        let params = Params {
            screen_resolution: SurfaceSize::zero(),
            _pad: [0, 0],
        };
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("silica uniforms"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("silica uniforms bind group"),
            layout: uniforms_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            }],
        });
        Viewport {
            params,
            params_buffer,
            bind_group,
        }
    }
    fn update(&mut self, queue: &wgpu::Queue, resolution: SurfaceSize) {
        if self.params.screen_resolution != resolution {
            self.params.screen_resolution = resolution;
            queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&self.params));
        }
    }
}

struct QuadRenderer {
    pipeline: wgpu::RenderPipeline,
    viewport: Viewport,
}

impl QuadRenderer {
    fn new(
        context: &Context,
        surface_format: wgpu::TextureFormat,
        texture_config: &TextureConfig,
    ) -> Self {
        use wgpu::*;

        let shader = context.device.create_shader_module(ShaderModuleDescriptor {
            label: Some("silica shader"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<Quad>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: &vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4],
        };
        let uniforms_layout = context
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("silica uniforms bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(std::mem::size_of::<Params>() as u64),
                    },
                    count: None,
                }],
            });
        let pipeline_layout = context
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&uniforms_layout, texture_config.bind_group_layout()],
                push_constant_ranges: &[],
            });

        let pipeline = context
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("silica pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[vertex_buffer_layout],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(ColorTargetState {
                        format: surface_format,
                        blend: Some(BlendState::ALPHA_BLENDING),
                        write_mask: ColorWrites::default(),
                    })],
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
                cache: None,
            });
        let viewport = Viewport::new(&context.device, &uniforms_layout);

        QuadRenderer { pipeline, viewport }
    }
    fn surface_resize(&mut self, context: &Context, size: SurfaceSize) {
        self.viewport.update(&context.queue, size);
    }
    fn bind(&self, pass: &mut wgpu::RenderPass, buffer: &ResizableBuffer<Quad>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.viewport.bind_group, &[]);
        pass.set_vertex_buffer(0, buffer.buffer().slice(..));
    }
    fn draw(&self, pass: &mut wgpu::RenderPass, texture: &Texture, range: Range<u32>) {
        pass.set_bind_group(1, texture.bind_group(), &[]);
        pass.draw(0..4, range.clone());
    }
}

struct TextResources {
    swash_cache: glyphon::SwashCache,
    atlas: glyphon::TextAtlas,
    viewport: glyphon::Viewport,
}

impl TextResources {
    fn new(context: &Context, surface_format: wgpu::TextureFormat) -> Self {
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(&context.device);
        let atlas = glyphon::TextAtlas::with_color_mode(
            &context.device,
            &context.queue,
            &cache,
            surface_format,
            glyphon::ColorMode::Web,
        );
        let viewport = glyphon::Viewport::new(&context.device, &cache);
        TextResources {
            swash_cache,
            atlas,
            viewport,
        }
    }
    fn surface_resize(&mut self, context: &Context, size: SurfaceSize) {
        self.viewport.update(
            &context.queue,
            glyphon::Resolution {
                width: size.width,
                height: size.height,
            },
        );
    }
}

enum DrawRange {
    ThemeQuads(Range<u32>),
    CustomQuads(Range<u32>, Texture),
    Text(usize),
}

pub struct GuiBatcher<'a> {
    layer: usize,
    quads: &'a mut Vec<Quad>,
    custom_quads: HashMap<Texture, Vec<Quad>>,
    text_renderer: &'a mut TextRenderer,
    text_areas: Vec<TextArea<'a>>,
    draw_start: u32,
}

impl<'a> GuiBatcher<'a> {
    fn new(layer: usize, quads: &'a mut Vec<Quad>, text_renderer: &'a mut TextRenderer) -> Self {
        let draw_start = quads.len() as u32;
        GuiBatcher {
            layer,
            quads,
            custom_quads: HashMap::new(),
            text_renderer,
            text_areas: Vec::new(),
            draw_start,
        }
    }
    pub fn queue_theme_quad(&mut self, quad: Quad) {
        self.quads.push(quad);
    }
    pub fn queue_custom_quad(&mut self, texture: Texture, quad: Quad) {
        self.custom_quads.entry(texture).or_default().push(quad);
    }
    pub fn queue_text(&mut self, text_area: TextArea<'a>) {
        self.text_areas.push(text_area);
    }
    fn commit(
        self,
        context: &Context,
        font_system: &mut FontSystem,
        text_resources: &mut TextResources,
        draw_ranges: &mut Vec<DrawRange>,
    ) {
        let mut draw_start = self.draw_start;
        let draw_end = self.quads.len() as u32;
        if draw_end > draw_start {
            draw_ranges.push(DrawRange::ThemeQuads(draw_start..draw_end));
            draw_start = draw_end;
        }
        for (texture, mut custom_quads) in self.custom_quads {
            self.quads.append(&mut custom_quads);
            let draw_end = self.quads.len() as u32;
            draw_ranges.push(DrawRange::CustomQuads(draw_start..draw_end, texture));
            draw_start = draw_end;
        }
        let has_text = !self.text_areas.is_empty();
        match self.text_renderer.prepare(
            &context.device,
            &context.queue,
            font_system,
            &mut text_resources.atlas,
            &text_resources.viewport,
            self.text_areas,
            &mut text_resources.swash_cache,
        ) {
            Ok(()) => (),
            Err(PrepareError::AtlasFull) => {
                log::warn!("failed to prepare text for rendering: atlas full")
            }
        }
        if has_text {
            draw_ranges.push(DrawRange::Text(self.layer));
        }
    }
}
impl DrawQuad for GuiBatcher<'_> {
    fn draw_quad(&mut self, rect: euclid::default::Box2D<f32>, uv: UvRect, color: Rgba) {
        self.queue_theme_quad(Quad {
            rect: rect.cast_unit(),
            uv,
            color,
        });
    }
}

pub struct GuiRenderer {
    quad_renderer: QuadRenderer,
    text_resources: TextResources,
    theme: Box<dyn Theme>,
    theme_texture: Texture,
    quads: Vec<Quad>,
    quads_buffer: ResizableBuffer<Quad>,
    text_renderers: Vec<TextRenderer>,
    draw_ranges: Vec<DrawRange>,
}

impl GuiRenderer {
    pub fn new(
        context: &Context,
        surface_format: wgpu::TextureFormat,
        texture_config: &TextureConfig,
        theme_loader: impl ThemeLoader,
    ) -> Self {
        let theme_texture = theme_loader.load_texture(context, texture_config);
        let theme = theme_loader.load_theme();
        Self::with_preloaded_theme(
            context,
            surface_format,
            texture_config,
            theme_texture,
            theme,
        )
    }
    pub fn with_preloaded_theme(
        context: &Context,
        surface_format: wgpu::TextureFormat,
        texture_config: &TextureConfig,
        theme_texture: Texture,
        theme: Box<dyn Theme>,
    ) -> Self {
        let quad_renderer = QuadRenderer::new(context, surface_format, texture_config);
        let text_resources = TextResources::new(context, surface_format);
        GuiRenderer {
            quad_renderer,
            text_resources,
            theme,
            theme_texture,
            quads: Vec::new(),
            quads_buffer: ResizableBuffer::new(context),
            text_renderers: Vec::new(),
            draw_ranges: Vec::new(),
        }
    }

    pub fn surface_resize(&mut self, context: &Context, size: SurfaceSize) {
        self.quad_renderer.surface_resize(context, size);
        self.text_resources.surface_resize(context, size);
    }
    pub fn background_color(&self) -> Rgba {
        self.theme.color(ThemeColor::Background)
    }

    pub fn render(&mut self, context: &Context, pass: &mut wgpu::RenderPass, gui: &mut Gui) {
        gui.layout();
        if gui.draw_dirty {
            self.quads.clear();
            self.draw_ranges.clear();
            for (index, layer) in gui.layouts.iter().enumerate() {
                if index >= self.text_renderers.len() {
                    self.text_renderers.push(TextRenderer::new(
                        &mut self.text_resources.atlas,
                        &context.device,
                        wgpu::MultisampleState::default(),
                        None,
                    ));
                }
                let mut batcher =
                    GuiBatcher::new(index, &mut self.quads, &mut self.text_renderers[index]);
                for (node, rect, padding) in layer.iter() {
                    if let Some(widget) = gui.tree.get_node_context(*node) {
                        widget.draw(&mut batcher, self.theme.as_ref(), *rect, *padding);
                    }
                }
                batcher.commit(
                    context,
                    &mut gui.font_system,
                    &mut self.text_resources,
                    &mut self.draw_ranges,
                );
            }
            self.quads_buffer.set_data(context, &self.quads);
            gui.draw_dirty = false;
        }

        let mut quads_pipeline_bound = false;
        for draw_range in self.draw_ranges.iter() {
            match draw_range {
                DrawRange::ThemeQuads(range) => {
                    if !quads_pipeline_bound {
                        self.quad_renderer.bind(pass, &self.quads_buffer);
                        quads_pipeline_bound = true;
                    }
                    self.quad_renderer
                        .draw(pass, &self.theme_texture, range.clone());
                }
                DrawRange::CustomQuads(range, texture) => {
                    if !quads_pipeline_bound {
                        self.quad_renderer.bind(pass, &self.quads_buffer);
                        quads_pipeline_bound = true;
                    }
                    self.quad_renderer.draw(pass, texture, range.clone());
                }
                DrawRange::Text(layer) => {
                    match self.text_renderers[*layer].render(
                        &self.text_resources.atlas,
                        &self.text_resources.viewport,
                        pass,
                    ) {
                        Ok(()) => (),
                        Err(RenderError::RemovedFromAtlas) => {
                            log::warn!("failed to render text: a glyph was removed from the atlas")
                        }
                        Err(RenderError::ScreenResolutionChanged) => {
                            log::warn!("failed to render text: screen resolution changed")
                        }
                    }
                    quads_pipeline_bound = false;
                }
            }
        }
    }
}
