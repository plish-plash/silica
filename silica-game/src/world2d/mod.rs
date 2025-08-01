use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use euclid::vec2;
use silica_gui::Rgba;
use silica_wgpu::{
    BatcherPipeline, Context, SurfaceSize, TextureConfig, UvRect,
    wgpu::{self, util::DeviceExt},
};

pub type Point = euclid::Point2D<f32, crate::WorldSpace>;
pub type Vector = euclid::Vector2D<f32, crate::WorldSpace>;
pub type Size = euclid::Size2D<f32, crate::WorldSpace>;
pub type Rect = euclid::Box2D<f32, crate::WorldSpace>;
pub type Transform = euclid::Transform2D<f32, crate::LocalSpace, crate::WorldSpace>;
pub type CameraTransform = euclid::Transform2D<f32, crate::WorldSpace, crate::ScreenSpace>;

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Quad {
    pub transform: Transform,
    pub uv: UvRect,
    pub color: Rgba,
}

impl Quad {
    pub fn rect_transform(rect: Rect) -> Transform {
        Transform::scale(rect.width(), rect.height()).then_translate(rect.min.to_vector())
    }
}

#[derive(Clone)]
pub struct Camera2D {
    pub center: Point,
    pub scale: f32,
}

impl Camera2D {
    pub fn transform(
        &self,
        size: SurfaceSize,
        viewport: Option<euclid::Rect<u32, crate::ScreenSpace>>,
    ) -> CameraTransform {
        let viewport_center = viewport
            .map(|viewport| viewport.center())
            .unwrap_or_else(|| (size / 2).to_vector().to_point().cast_unit())
            .to_f32();
        CameraTransform::translation(-self.center.x, -self.center.y)
            .then_scale(self.scale, self.scale)
            .then_translate(vec2(viewport_center.x, viewport_center.y))
    }
}
impl Default for Camera2D {
    fn default() -> Self {
        Camera2D {
            center: Point::zero(),
            scale: 1.0,
        }
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Uniforms {
    view_matrix: CameraTransform,
    screen_resolution: [f32; 2],
}

pub struct Pipeline2D {
    pipeline: wgpu::RenderPipeline,
    uniforms_buffer: wgpu::Buffer,
    uniforms_bind_group: wgpu::BindGroup,
}

impl Pipeline2D {
    pub fn new(context: &Context, texture_config: &TextureConfig) -> Self {
        use wgpu::*;
        let device = &context.device;
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("world2d shader"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let uniforms_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("world2d uniforms bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(size_of::<Uniforms>() as _),
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&uniforms_layout, texture_config.bind_group_layout()],
            push_constant_ranges: &[],
        });

        let uniforms = Uniforms {
            view_matrix: CameraTransform::identity(),
            screen_resolution: [0.0; 2],
        };
        let uniforms_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("world2d uniforms"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let uniforms_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("world2d uniforms bind group"),
            layout: &uniforms_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("world2d pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[
                    VertexBufferLayout {
                        array_stride: std::mem::size_of::<Quad>() as u64,
                        step_mode: VertexStepMode::Instance,
                        attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x4],
                    },
                ],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: context.surface_format.expect("surface not created"),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::default(),
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                ..PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Pipeline2D {
            pipeline,
            uniforms_buffer,
            uniforms_bind_group,
        }
    }

    pub fn set_camera(
        &mut self,
        context: &Context,
        camera_transform: CameraTransform,
        size: SurfaceSize,
    ) {
        let uniforms = Uniforms {
            view_matrix: camera_transform,
            screen_resolution: size.to_f32().to_array(),
        };
        context
            .queue
            .write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));
    }
}

impl BatcherPipeline for Pipeline2D {
    fn bind(&self, pass: &mut wgpu::RenderPass) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
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
