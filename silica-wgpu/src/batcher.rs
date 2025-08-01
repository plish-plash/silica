use std::ops::Range;

use bytemuck::Pod;

use crate::{Buffer, Context, ResizableBuffer, Texture};

struct DrawCall {
    buffer: Option<wgpu::Buffer>,
    texture: wgpu::BindGroup,
    range: Range<u32>,
}

pub trait BatcherPipeline {
    fn bind(&self, pass: &mut wgpu::RenderPass);
    fn set_buffer(&self, pass: &mut wgpu::RenderPass, buffer: &wgpu::Buffer);
    fn set_texture(&self, pass: &mut wgpu::RenderPass, texture: &wgpu::BindGroup);
    fn draw(&self, pass: &mut wgpu::RenderPass, range: Range<u32>);
}

pub struct Batcher<T> {
    buffer: ResizableBuffer<T>,
    buffer_data: Vec<T>,
    buffer_data_dirty: bool,
    draw_calls: Vec<DrawCall>,
    current_texture: Option<wgpu::BindGroup>,
    last_index: usize,
}

impl<T: Pod> Batcher<T> {
    pub fn new(context: &Context) -> Self {
        Batcher {
            buffer: ResizableBuffer::new(context),
            buffer_data: Vec::new(),
            buffer_data_dirty: false,
            draw_calls: Vec::new(),
            current_texture: None,
            last_index: 0,
        }
    }
    fn flush(&mut self) {
        if let Some(texture) = self.current_texture.clone() {
            if self.last_index < self.buffer_data.len() {
                self.draw_calls.push(DrawCall {
                    buffer: None,
                    texture,
                    range: (self.last_index as u32)..(self.buffer_data.len() as u32),
                });
                self.last_index = self.buffer_data.len();
            }
        }
    }
    pub fn clear(&mut self) {
        self.buffer_data.clear();
        self.buffer_data_dirty = true;
        self.draw_calls.clear();
        self.current_texture = None;
        self.last_index = 0;
    }
    pub fn set_texture(&mut self, texture: &Texture) {
        let texture = texture.bind_group();
        if self.current_texture.as_ref() != Some(texture) {
            self.flush();
            self.current_texture = Some(texture.clone());
        }
    }
    pub fn queue(&mut self, instance: T) {
        self.buffer_data.push(instance);
        self.buffer_data_dirty = true;
    }
    pub fn queue_buffer(&mut self, texture: &Texture, buffer: &wgpu::Buffer, range: Range<u32>) {
        self.flush();
        self.draw_calls.push(DrawCall {
            buffer: Some(buffer.clone()),
            texture: texture.bind_group().clone(),
            range,
        })
    }
    pub fn draw(
        &mut self,
        context: &Context,
        pass: &mut wgpu::RenderPass,
        pipeline: &impl BatcherPipeline,
    ) {
        self.flush();
        if self.buffer_data_dirty {
            self.buffer.set_data(context, &self.buffer_data);
            self.buffer_data_dirty = false;
        }
        pipeline.bind(pass);
        let mut reset_buffer = true;
        for DrawCall {
            buffer,
            texture,
            range,
        } in self.draw_calls.iter()
        {
            if let Some(buffer) = buffer {
                pipeline.set_buffer(pass, buffer);
                reset_buffer = true;
            } else if reset_buffer {
                pipeline.set_buffer(pass, self.buffer.buffer());
                reset_buffer = false;
            }
            pipeline.set_texture(pass, texture);
            pipeline.draw(pass, range.clone());
        }
    }
}

pub struct ImmediateBatcher<T> {
    buffer: Buffer<T>,
    buffer_data: Vec<T>,
    buffer_range: Range<u32>,
    current_texture: Option<wgpu::BindGroup>,
}

impl<T: Pod> ImmediateBatcher<T> {
    pub fn new(context: &Context) -> Self {
        ImmediateBatcher {
            buffer: Buffer::new(context, ResizableBuffer::<T>::INITIAL_CAPACITY),
            buffer_data: Vec::new(),
            buffer_range: 0..0,
            current_texture: None,
        }
    }
    pub fn set_texture(
        &mut self,
        pass: &mut wgpu::RenderPass,
        pipeline: &impl BatcherPipeline,
        texture: &Texture,
    ) {
        let texture = texture.bind_group();
        if self.current_texture.as_ref() != Some(texture) {
            self.draw(pass, pipeline);
            self.current_texture = Some(texture.clone());
        }
    }
    pub fn queue(
        &mut self,
        context: &Context,
        pass: &mut wgpu::RenderPass,
        pipeline: &impl BatcherPipeline,
        instance: T,
    ) {
        if self.buffer_data.len() >= self.buffer.capacity() {
            self.buffer.set_data(context, &self.buffer_data);
            self.draw(pass, pipeline);
            self.buffer = Buffer::new(context, self.buffer.capacity() * 2);
            self.buffer_data.clear();
            self.buffer_range = 0..0;
        }
        self.buffer_data.push(instance);
        self.buffer_range.end += 1;
    }
    pub fn draw(&mut self, pass: &mut wgpu::RenderPass, pipeline: &impl BatcherPipeline) {
        if let Some(texture) = self.current_texture.as_ref() {
            if !self.buffer_range.is_empty() {
                pipeline.bind(pass);
                pipeline.set_buffer(pass, self.buffer.buffer());
                pipeline.set_texture(pass, texture);
                pipeline.draw(pass, self.buffer_range.clone());
            }
        }
        self.buffer_range.start = self.buffer_range.end;
    }
    pub fn finish(&mut self, context: &Context) {
        self.buffer.set_data(context, &self.buffer_data);
        self.buffer_data.clear();
        self.buffer_range = 0..0;
    }
}
