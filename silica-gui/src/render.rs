use std::{num::NonZeroU64, ops::Range};

use bytemuck::{Pod, Zeroable};
use glyphon::{FontSystem, PrepareError, RenderError, TextArea, TextRenderer};
use silica_color::Rgba;
use silica_wgpu::{
    draw::DrawQuad, wgpu, Context, ResizableBuffer, SurfaceSize, Texture, TextureConfig, UvRect,
};
use taffy::{Layout, NodeId, PrintTree, TaffyTree, TraversePartialTree};

use crate::{
    theme::{Theme, ThemeColor, ThemeLoader},
    Gui, LayoutExt, Vector, Widget,
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
    fn render(
        &self,
        pass: &mut wgpu::RenderPass,
        texture: &Texture,
        custom_textures: &[Texture],
        buffer: &ResizableBuffer<Quad>,
        ranges: &[(Option<usize>, Range<u32>)],
    ) {
        if buffer.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.viewport.bind_group, &[]);
        pass.set_vertex_buffer(0, buffer.buffer().slice(..));
        for (texture_index, range) in ranges {
            let texture = if let Some(index) = texture_index {
                &custom_textures[*index]
            } else {
                texture
            };
            pass.set_bind_group(1, texture.bind_group(), &[]);
            pass.draw(0..4, range.clone());
        }
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

struct GuiLayerRenderer {
    quads: Vec<Quad>,
    quads_ranges: Vec<(Option<usize>, Range<u32>)>,
    quads_buffer: ResizableBuffer<Quad>,
    text_renderer: TextRenderer,
}

impl GuiLayerRenderer {
    fn new(context: &Context, text_resources: &mut TextResources) -> Self {
        let text_renderer = TextRenderer::new(
            &mut text_resources.atlas,
            &context.device,
            wgpu::MultisampleState::default(),
            None,
        );
        GuiLayerRenderer {
            quads: Vec::new(),
            quads_ranges: Vec::new(),
            quads_buffer: ResizableBuffer::new(context),
            text_renderer,
        }
    }
}

pub struct GuiBatcher<'a> {
    layer: &'a mut GuiLayerRenderer,
    text_areas: Vec<TextArea<'a>>,
    current_texture: Option<usize>,
    last_index: u32,
}

impl<'a> GuiBatcher<'a> {
    fn new(layer: &'a mut GuiLayerRenderer) -> Self {
        layer.quads_ranges.clear();
        GuiBatcher {
            layer,
            text_areas: Vec::new(),
            current_texture: None,
            last_index: 0,
        }
    }
    fn set_texture(&mut self, texture: Option<usize>) {
        if self.current_texture != texture {
            let next_index = self.layer.quads.len() as u32;
            if next_index > self.last_index {
                self.layer
                    .quads_ranges
                    .push((self.current_texture, self.last_index..next_index));
            }
            self.current_texture = texture;
            self.last_index = next_index;
        }
    }
    pub fn queue_theme_quad(&mut self, quad: Quad) {
        self.set_texture(None);
        self.layer.quads.push(quad);
    }
    pub fn queue_custom_quad(&mut self, texture: impl Into<usize>, quad: Quad) {
        self.set_texture(Some(texture.into()));
        self.layer.quads.push(quad);
    }
    pub fn queue_text(&mut self, text_area: TextArea<'a>) {
        self.text_areas.push(text_area);
    }
    fn prepare(
        self,
        context: &Context,
        font_system: &mut FontSystem,
        text_resources: &mut TextResources,
    ) -> &'a mut GuiLayerRenderer {
        self.layer.quads_ranges.push((
            self.current_texture,
            self.last_index..(self.layer.quads.len() as u32),
        ));
        self.layer.quads_buffer.set_data(context, &self.layer.quads);
        self.layer.quads.clear();
        match self.layer.text_renderer.prepare(
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
        self.layer
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
    layers: Vec<GuiLayerRenderer>,
    theme: Box<dyn Theme>,
    theme_texture: Texture,
    custom_textures: Vec<Texture>,
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
            layers: Vec::new(),
            theme,
            theme_texture,
            custom_textures: Vec::new(),
        }
    }

    pub fn set_custom_textures(&mut self, textures: Vec<Texture>) {
        self.custom_textures = textures;
    }
    pub fn surface_resize(&mut self, context: &Context, size: SurfaceSize) {
        self.quad_renderer.surface_resize(context, size);
        self.text_resources.surface_resize(context, size);
    }
    pub fn background_color(&self) -> Rgba {
        self.theme.color(ThemeColor::Background)
    }

    fn visit_nodes<'a>(
        tree: &'a TaffyTree<Box<dyn Widget>>,
        node: NodeId,
        mut offset: Vector,
        f: &mut impl FnMut(Option<&'a dyn Widget>, Vector, &'a Layout),
    ) {
        let context = tree.get_node_context(node).map(|context| context.as_ref());
        if !context.map(|widget| widget.visible()).unwrap_or(true) {
            return;
        }
        let layout = tree.get_final_layout(node);
        f(context, offset, layout);
        offset.x += layout.location.x;
        offset.y += layout.location.y;
        for child in tree.child_ids(node) {
            Self::visit_nodes(tree, child, offset, f);
        }
    }
    fn render_layer_text(
        pass: &mut wgpu::RenderPass,
        layer: &GuiLayerRenderer,
        text_resources: &TextResources,
    ) {
        match layer
            .text_renderer
            .render(&text_resources.atlas, &text_resources.viewport, pass)
        {
            Ok(()) => (),
            Err(RenderError::RemovedFromAtlas) => {
                log::warn!("failed to render text: a glyph was removed from the atlas")
            }
            Err(RenderError::ScreenResolutionChanged) => {
                log::warn!("failed to render text: screen resolution changed")
            }
        }
    }
    pub fn render(&mut self, context: &Context, pass: &mut wgpu::RenderPass, gui: &mut Gui) {
        gui.layout();
        if gui.draw_dirty {
            gui.draw_dirty = false;
            let layer_count = 1; // TODO
            if layer_count > self.layers.len() {
                self.layers.resize_with(layer_count, || {
                    GuiLayerRenderer::new(context, &mut self.text_resources)
                });
            }
            let mut batchers: Vec<_> = self.layers.iter_mut().map(GuiBatcher::new).collect();
            Self::visit_nodes(
                &gui.tree,
                gui.root,
                Vector::zero(),
                &mut |widget, offset, layout| {
                    let layer = 0; // TODO
                    let batcher = &mut batchers[layer];
                    // TODO better border
                    if layout.border.left > 0.0
                        || layout.border.top > 0.0
                        || layout.border.right > 0.0
                        || layout.border.bottom > 0.0
                    {
                        self.theme
                            .draw_border(batcher, layout.border_box().translate(offset));
                    }
                    if let Some(widget) = widget {
                        let rect = layout.content_box().translate(offset);
                        widget.draw(batcher, self.theme.as_ref(), rect, layout.padding());
                    }
                },
            );
            for batcher in batchers {
                let layer =
                    batcher.prepare(context, &mut gui.font_system, &mut self.text_resources);
                self.quad_renderer.render(
                    pass,
                    &self.theme_texture,
                    &self.custom_textures,
                    &layer.quads_buffer,
                    &layer.quads_ranges,
                );
                Self::render_layer_text(pass, layer, &self.text_resources);
            }
        } else {
            for layer in self.layers.iter_mut() {
                self.quad_renderer.render(
                    pass,
                    &self.theme_texture,
                    &self.custom_textures,
                    &layer.quads_buffer,
                    &layer.quads_ranges,
                );
                Self::render_layer_text(pass, layer, &self.text_resources);
            }
        }
    }
}
