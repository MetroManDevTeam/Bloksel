pub struct Model {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub normals: Vec<f32>,
    pub tex_coords: Vec<f32>,
}

impl Model {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            normals: Vec::new(),
            tex_coords: Vec::new(),
        }
    }
}
