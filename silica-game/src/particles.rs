use bytemuck::Pod;
use silica_wgpu::{Buffer, Context, Texture};

use crate::world2d::Quad;

pub trait ParticleSolver {
    type Particle;
    type Primitive: Pod;
    fn update(&self, particle: &mut Self::Particle, dt: f32) -> bool;
    fn draw(&self, particle: &Self::Particle) -> Self::Primitive;
}

pub struct ParticleSystem<S: ParticleSolver> {
    particles: Vec<S::Particle>,
    solver: S,
    texture: Texture,
    primitives: Option<Buffer<S::Primitive>>,
    changed: bool,
}

impl<S> ParticleSystem<S>
where
    S: ParticleSolver,
{
    pub fn new(solver: S, texture: Texture) -> Self {
        ParticleSystem {
            particles: Vec::new(),
            solver,
            texture,
            primitives: None,
            changed: false,
        }
    }
    pub fn with_particles(particles: Vec<S::Particle>, solver: S, texture: Texture) -> Self {
        ParticleSystem {
            particles,
            solver,
            texture,
            primitives: None,
            changed: true,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }
    pub fn spawn(&mut self, particle: S::Particle) {
        self.particles.push(particle);
        self.changed = true;
    }
    pub fn update(&mut self, dt: f32) {
        self.particles.retain_mut(|particle| self.solver.update(particle, dt));
        self.changed = true;
    }
    pub fn texture(&self) -> &Texture {
        &self.texture
    }
}
impl<S> ParticleSystem<S>
where
    S: ParticleSolver,
    S::Particle: Clone,
{
    pub fn spawn_many(&mut self, particle: S::Particle, count: usize) {
        self.particles.resize(self.particles.len() + count, particle);
        self.changed = true;
    }
}
impl<S> ParticleSystem<S>
where
    S: ParticleSolver<Primitive = Quad>,
{
    pub fn prepare(&mut self, context: &Context) -> Option<&Buffer<Quad>> {
        if self.changed {
            if self
                .primitives
                .as_ref()
                .map(|buffer| buffer.capacity() < self.particles.len())
                .unwrap_or(true)
            {
                self.primitives = Some(Buffer::new(context, self.particles.len().next_power_of_two()));
            }
            let mut writer = self.primitives.as_mut().unwrap().write(context);
            for particle in self.particles.iter() {
                writer.push(self.solver.draw(particle));
            }
            self.changed = false;
        }
        self.primitives.as_ref()
    }
}
