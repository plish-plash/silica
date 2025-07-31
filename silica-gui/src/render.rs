use std::{num::NonZeroU64, ops::Range, rc::Rc};

use bytemuck::{Pod, Zeroable};
use euclid::Box2D;
use glyphon::TextRenderer;
use silica_wgpu::{
    BatcherPipeline, Context, ImmediateBatcher, SurfaceSize, Texture, TextureConfig, UvRect,
    draw::DrawQuad, wgpu,
};

use crate::{Color, FontSystem, Pixel, Rgba, theme::Theme};

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Quad {
    pub rect: Box2D<i32, Pixel>,
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

struct QuadPipeline {
    pipeline: wgpu::RenderPipeline,
    viewport: Viewport,
}

impl QuadPipeline {
    fn new(context: &Context, texture_config: &TextureConfig) -> Self {
        use wgpu::*;

        let shader = context.device.create_shader_module(ShaderModuleDescriptor {
            label: Some("silica shader"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<Quad>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: &vertex_attr_array![0 => Sint32x4, 1 => Float32x4, 2 => Float32x4],
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
                        format: context.surface_format.unwrap(),
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

        QuadPipeline { pipeline, viewport }
    }
    fn surface_resize(&mut self, context: &Context, size: SurfaceSize) {
        self.viewport.update(&context.queue, size);
    }
}
impl BatcherPipeline for QuadPipeline {
    fn bind(&self, pass: &mut wgpu::RenderPass) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.viewport.bind_group, &[]);
    }
    fn set_buffer(&self, pass: &mut wgpu::RenderPass, buffer: &wgpu::Buffer) {
        pass.set_vertex_buffer(0, buffer.slice(..));
    }
    fn set_texture(&self, pass: &mut wgpu::RenderPass, texture: &wgpu::BindGroup) {
        pass.set_bind_group(1, texture, &[]);
    }
    fn draw(&self, pass: &mut wgpu::RenderPass, range: Range<u32>) {
        pass.draw(0..4, range);
    }
}

pub struct TextResources {
    pub swash_cache: glyphon::SwashCache,
    pub atlas: glyphon::TextAtlas,
    pub viewport: glyphon::Viewport,
}

impl TextResources {
    fn new(context: &Context) -> Self {
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(&context.device);
        let atlas = glyphon::TextAtlas::with_color_mode(
            &context.device,
            &context.queue,
            &cache,
            context.surface_format.unwrap(),
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

pub struct GuiResources {
    quad_pipeline: QuadPipeline,
    text_resources: TextResources,
    theme: Rc<dyn Theme>,
}

impl GuiResources {
    pub fn new(context: &Context, texture_config: &TextureConfig, theme: Rc<dyn Theme>) -> Self {
        let quad_pipeline = QuadPipeline::new(context, texture_config);
        let text_resources = TextResources::new(context);
        GuiResources {
            quad_pipeline,
            text_resources,
            theme,
        }
    }

    pub fn surface_resize(&mut self, context: &Context, size: SurfaceSize) {
        self.quad_pipeline.surface_resize(context, size);
        self.text_resources.surface_resize(context, size);
    }
    pub fn background_color(&self) -> Rgba {
        self.theme.color(Color::Background)
    }

    pub fn text_resources(&mut self) -> &mut TextResources {
        &mut self.text_resources
    }
}

pub struct GuiRenderer<'a, 'b> {
    pub(crate) resources: &'a mut GuiResources,
    pub(crate) batcher: ImmediateBatcher<Quad>,
    pub(crate) context: &'a Context,
    pub(crate) pass: &'a mut wgpu::RenderPass<'b>,
}

impl GuiRenderer<'_, '_> {
    pub(crate) fn finish(&mut self) {
        self.batcher.draw(self.pass, &self.resources.quad_pipeline);
        self.batcher.finish(self.context);
    }
    pub fn theme(&self) -> Rc<dyn Theme> {
        self.resources.theme.clone()
    }
    pub fn draw_theme_quad(&mut self, quad: Quad) {
        self.batcher.set_texture(
            self.pass,
            &self.resources.quad_pipeline,
            self.resources.theme.texture(),
        );
        self.batcher
            .queue(self.context, self.pass, &self.resources.quad_pipeline, quad);
    }
    pub fn draw_quad(&mut self, texture: &Texture, quad: Quad) {
        self.batcher
            .set_texture(self.pass, &self.resources.quad_pipeline, texture);
        self.batcher
            .queue(self.context, self.pass, &self.resources.quad_pipeline, quad);
    }
    pub fn create_text_renderer(&mut self) -> TextRenderer {
        TextRenderer::new(
            &mut self.resources.text_resources.atlas,
            &self.context.device,
            wgpu::MultisampleState::default(),
            None,
        )
    }
    pub fn prepare_text<'a>(
        &mut self,
        font_system: &FontSystem,
        text_renderer: &mut TextRenderer,
        text_areas: impl IntoIterator<Item = glyphon::TextArea<'a>>,
    ) {
        text_renderer
            .prepare(
                &self.context.device,
                &self.context.queue,
                &mut font_system.borrow_mut(),
                &mut self.resources.text_resources.atlas,
                &self.resources.text_resources.viewport,
                text_areas,
                &mut self.resources.text_resources.swash_cache,
            )
            .unwrap();
    }
    pub fn draw_text(&mut self, text_renderer: &TextRenderer) {
        self.batcher.draw(self.pass, &self.resources.quad_pipeline);
        text_renderer
            .render(
                &self.resources.text_resources.atlas,
                &self.resources.text_resources.viewport,
                self.pass,
            )
            .unwrap();
    }
}
impl DrawQuad<i32, Pixel> for GuiRenderer<'_, '_> {
    fn draw_quad(&mut self, rect: Box2D<i32, Pixel>, uv: UvRect, color: Rgba) {
        self.draw_theme_quad(Quad { rect, uv, color });
    }
}
