mod sim; 


pub const SAMPLE_COUNT: u32 = 4;
pub const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
pub const CIRCLE_RES: u32 = 32;
// const PARAMS: [f32; 2] = [
//     0.001, //dt
//     0.01//Gravitational constant
// ];

/// Example struct holds references to wgpu resources and frame persistent data
///
///
///

pub fn generate_circle(radius: f32) -> [f32; (CIRCLE_RES * 8)  as usize ] {
    use std::f64::consts::PI;
    use std::convert::TryInto;
    let mut coords = Vec::<f32>::new();

    for i in 0..(CIRCLE_RES) {
        coords.push(radius * ((2.0 * PI * i as f64 / CIRCLE_RES as f64) as f32).cos());
        coords.push(radius * ((2.0 * PI * i as f64 / CIRCLE_RES as f64) as f32).sin());


        coords.push(0.0);
        coords.push(0.0);

        coords.push(radius * ((2.0 * PI * (i+1) as f64 / CIRCLE_RES as f64) as f32).cos());
        coords.push(radius * ((2.0 * PI * (i+1) as f64 / CIRCLE_RES as f64) as f32).sin());

    }

     for _i in 0..CIRCLE_RES{
         coords.push(0.01);
         coords.push(0.0)
     }

    println!("Coords: {:?}", coords);
    coords.try_into().unwrap_or_else(|v: Vec<f32>| panic!("Expected a Vec of length {} but it was {}", CIRCLE_RES, v.len()))
}
