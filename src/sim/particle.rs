use super::params::Params;
use rand::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pos: (f32, f32),
    vel: (f32, f32),
    mass: f32,
    kind: f32,
}

impl Particle {
    pub fn to_slice(&self) -> [f32; 6] {
        [
            self.pos.0, self.pos.1, self.vel.0, self.vel.1, self.mass, self.kind,
        ]
    }
    pub fn new_random(params: &Params) -> Self {
        let mut rng = rand::thread_rng();
        let mut unif = || (rng.gen::<f32>() * 2f32 - 1f32) * params.world_size;
        let max_types: u32 = params.num_types;
        let mut rng = rand::thread_rng();

        Self {
            pos: (unif(), unif()),
            vel: (0.0, 0.0),
            mass: 1.0,
            kind: (rng.gen_range(0..max_types) as f32),
        }
    }
    pub fn new() -> Self {
        Self {
            pos: (0.0, 0.0),
            vel: (0.0, 0.0),
            mass: 100.0,
            kind: 0.01,
        }
    }

    pub fn bind_desc(binding: u32, size: u64, read_only: bool) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new((size * 24) as _), //CHANGE SIZE IF ISSUES
            },
            count: None,
        }
    }
}
