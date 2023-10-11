#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
    pub aspect_ratio: f32,
}

impl Camera {
    pub fn new(zoom: f32, aspect_ratio: f32) -> Self {
        Camera {
            x: 0.0,
            y: 0.0,
            zoom: zoom,
            aspect_ratio: aspect_ratio,
        }
    }

    pub fn to_slice(&self) -> [f32; 4] {
        [self.x, self.y, self.zoom, self.aspect_ratio]
    }
}


