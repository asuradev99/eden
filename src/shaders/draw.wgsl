
struct Camera {
    x : f32, 
    y : f32,
    zoom : f32, 
    aspect_ratio : f32,
}
struct TypeColorsEntry {
  elem: f32,
  _pad1: f32,
  _pad2: f32,
  _pad3: f32,
}

@group(0) @binding(0) var<uniform> camera : Camera;
@group(0) @binding(1) var<storage, read> type_colors : array<TypeColorsEntry>;

@vertex
fn main_vs(
    @location(0) particle_pos: vec2<f32>,
    @location(1) particle_vel: vec2<f32>,
    @location(2) mass: f32,
    @location(3) kind: f32,
    @location(4) circle_coord: vec2<f32>,
) -> @builtin(position) vec4<f32> {
    let max_types = arrayLength(&type_colors) / 3u;

    let camera_pos_vec = vec2<f32>(camera.x, camera.y);
    var new_pos: vec2<f32> = vec2<f32>(particle_pos.x + (circle_coord.x * sqrt(mass)), particle_pos.y + (circle_coord.y * sqrt(mass)));

    if(camera.aspect_ratio > 1.0) {
        new_pos = vec2<f32>(new_pos.x / camera.aspect_ratio, new_pos.y);
    } else {
        new_pos = vec2<f32>(new_pos.x, new_pos.y / camera.aspect_ratio);

    }
    return vec4<f32>((new_pos - camera_pos_vec) * camera.zoom, kind / f32(max_types), 1.0);
}

@fragment
fn main_fs(@builtin(position) clip_position: vec4<f32>) -> @location(0) vec4<f32> {
    
    let max_types = f32(arrayLength(&type_colors) / 3u);
    let index = u32(clip_position.z * max_types);

    return vec4<f32>(type_colors[index*3u].elem, type_colors[(3u*index)+1u].elem, type_colors[(3u*index)+2u].elem, 0.1);
    //return vec4<f32>(0.0, 1.0, 1.0, 1.0);
}
