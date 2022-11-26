
struct Camera {
    x : f32, 
    y : f32,
    zoom : f32, 
}

@group(0) @binding(0) var<uniform> camera : Camera;

@vertex
fn main_vs(
    @location(0) particle_pos: vec2<f32>,
    @location(1) particle_vel: vec2<f32>,
) -> @builtin(position) vec4<f32> {
    let camera_pos_vec = vec2<f32>(camera.x, camera.y);

    return vec4<f32>((particle_pos - camera_pos_vec) * camera.zoom, 0.0, 1.0);
}

@fragment
fn main_fs() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}