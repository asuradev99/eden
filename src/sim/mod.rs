
pub const DEFAULT_COMPUTE_SHADER: &str = include_str!("../shaders/experimental.wgsl");

pub mod params; 
pub use params::Params;

pub mod particle;
pub use particle::Particle;
